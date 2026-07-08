use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env, fs,
    path::PathBuf,
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn default_refresh_interval_secs() -> u64 { 300 }
fn default_show_floating_window() -> bool { true }
fn default_opacity() -> f64 { 0.92 }
fn default_always_on_top() -> bool { true }
fn default_source_mode() -> String { "app_server".to_string() }
fn default_ui_scale() -> f64 { 1.0 }
fn default_taskbar_strip() -> bool { false }
fn default_show_reset_time() -> bool { true }
fn default_auto_update() -> bool { false }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeterConfig {
    #[serde(default = "default_refresh_interval_secs")]
    pub refresh_interval_secs: u64,
    #[serde(default = "default_show_floating_window")]
    pub show_floating_window: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    #[serde(default = "default_always_on_top")]
    pub always_on_top: bool,
    #[serde(default)]
    pub compact: bool,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f64,
    #[serde(default = "default_taskbar_strip")]
    pub taskbar_strip: bool,
    #[serde(default = "default_show_reset_time")]
    pub show_reset_time: bool,
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
    /// app_server | auto | client_window | client_watch
    #[serde(default = "default_source_mode")]
    pub source_mode: String,
}

impl Default for MeterConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: default_refresh_interval_secs(),
            show_floating_window: default_show_floating_window(),
            opacity: default_opacity(),
            always_on_top: default_always_on_top(),
            compact: false,
            ui_scale: default_ui_scale(),
            taskbar_strip: default_taskbar_strip(),
            show_reset_time: default_show_reset_time(),
            auto_update: default_auto_update(),
            source_mode: default_source_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitWindow {
    pub label: String,
    pub used_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<i64>,
    pub reset_text: Option<String>,
    pub reached_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexMeterStatus {
    pub ok: bool,
    pub message: String,
    pub source_mode: String,
    pub auth_mode: Option<String>,
    pub plan_type: Option<String>,
    pub primary: Option<LimitWindow>,
    pub secondary: Option<LimitWindow>,
    pub credit_balance: Option<f64>,
    pub credit_limit: Option<f64>,
    pub reset_credits_available: Option<i64>,
    pub updated_at_ms: u128,
}

struct AppServerClient {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    next_id: u64,
    launched_from: String,
}

impl Drop for AppServerClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

impl AppServerClient {
    async fn spawn() -> Result<Self, String> {
        let candidates = codex_candidate_paths();
        let mut errors: Vec<String> = Vec::new();

        for candidate in candidates {
            let label = candidate.display().to_string();
            let mut cmd = Command::new(&candidate);
            cmd.args(["app-server", "--stdio"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());

            #[cfg(windows)]
            {
                // CREATE_NO_WINDOW: avoid flashing a console window on Windows.
                cmd.creation_flags(0x08000000);
            }

            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(e) => {
                    errors.push(format!("{} => {}", redact_home(&label), e));
                    continue;
                }
            };

            let stdin = match child.stdin.take() {
                Some(stdin) => stdin,
                None => {
                    let _ = child.start_kill();
                    errors.push(format!("{} => 无法连接 stdin", redact_home(&label)));
                    continue;
                }
            };
            let stdout = match child.stdout.take() {
                Some(stdout) => stdout,
                None => {
                    let _ = child.start_kill();
                    errors.push(format!("{} => 无法连接 stdout", redact_home(&label)));
                    continue;
                }
            };

            return Ok(Self {
                child,
                stdin,
                stdout: BufReader::new(stdout).lines(),
                next_id: 1,
                launched_from: redact_home(&label),
            });
        }

        let tried = if errors.is_empty() {
            "没有找到候选 codex.exe。".to_string()
        } else {
            errors.join("；")
        };
        Err(format!(
            "无法启动 Codex app-server。已尝试 PATH、AppData\\Local\\OpenAI\\Codex\\bin、.codex\\plugins\\.plugin-appserver。详情：{tried}"
        ))
    }

    async fn initialize(&mut self) -> Result<(), String> {
        let _ = self.request("initialize", json!({
            "clientInfo": {
                "name": "lx_codex_meter",
                "title": "LX Codex Meter",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "optOutNotificationMethods": [
                    "thread/tokenUsage/updated",
                    "item/agentMessage/delta",
                    "turn/started",
                    "turn/completed"
                ]
            }
        })).await?;
        self.notify("initialized", json!({})).await?;
        Ok(())
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let msg = json!({ "method": method, "params": params });
        let line = format!("{}\n", msg);
        self.stdin.write_all(line.as_bytes()).await.map_err(|e| format!("写入 Codex app-server 失败：{e}"))?;
        self.stdin.flush().await.map_err(|e| format!("刷新 Codex app-server stdin 失败：{e}"))?;
        Ok(())
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;

        let mut msg = json!({ "id": id, "method": method });
        if !params.is_null() {
            msg["params"] = params;
        }

        let line = format!("{}\n", msg);
        self.stdin.write_all(line.as_bytes()).await.map_err(|e| format!("写入 Codex app-server 失败：{e}"))?;
        self.stdin.flush().await.map_err(|e| format!("刷新 Codex app-server stdin 失败：{e}"))?;

        let deadline = Duration::from_secs(25);
        loop {
            let line = timeout(deadline, self.stdout.next_line())
                .await
                .map_err(|_| format!("等待 Codex app-server `{method}` 响应超时"))?
                .map_err(|e| format!("读取 Codex app-server 输出失败：{e}"))?
                .ok_or_else(|| "Codex app-server 已退出".to_string())?;

            let value: Value = serde_json::from_str(&line)
                .map_err(|e| format!("Codex app-server 返回了无法解析的 JSON：{e}"))?;

            if value.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }

            if let Some(error) = value.get("error") {
                return Err(format!("Codex app-server `{method}` 返回错误：{}", compact_json(error)));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

pub async fn read_status(mode: Option<String>, client_text: Option<String>) -> Result<CodexMeterStatus, String> {
    let mode = mode.unwrap_or_else(default_source_mode);
    match mode.as_str() {
        "app_server" => read_codex_status_from_app_server().await,
        "client_window" => read_codex_status_from_client_window().await,
        "client_watch" => Ok(parse_client_usage_text(client_text.unwrap_or_default())),
        "auto" => match read_codex_status_from_app_server().await {
            Ok(status) if status.ok => Ok(status),
            _ => match read_codex_status_from_client_window().await {
                Ok(status) if status.ok => Ok(status),
                _ => Ok(parse_client_usage_text(client_text.unwrap_or_default())),
            },
        },
        _ => read_codex_status_from_app_server().await,
    }
}

async fn read_codex_status_from_client_window() -> Result<CodexMeterStatus, String> {
    let updated_at_ms = now_ms();
    let text = read_visible_codex_client_text().await?;
    let mut status = parse_client_usage_text(text);
    status.source_mode = "client_window".to_string();
    status.auth_mode = Some("windows-ui-automation-visible-text".to_string());
    status.updated_at_ms = updated_at_ms;
    if !status.ok {
        status.message = "已尝试自动读取 Codex 客户端窗口，但没有解析出额度。请把 Codex 客户端打开到 Settings / Usage / Credits 页面；如果仍失败，说明客户端未向 Windows 辅助功能暴露页面文字。".to_string();
    }
    Ok(status)
}

#[cfg(windows)]
async fn read_visible_codex_client_text() -> Result<String, String> {
    let script = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
$keywords = @('Codex', 'ChatGPT', 'OpenAI')
$windows = Get-Process | Where-Object {
  if ($_.MainWindowHandle -eq 0 -or [string]::IsNullOrWhiteSpace($_.MainWindowTitle)) { return $false }
  $title = $_.MainWindowTitle
  $proc = $_.ProcessName
  foreach ($k in $keywords) {
    if ($title -like "*$k*" -or $proc -like "*$k*") { return $true }
  }
  return $false
} | Select-Object -First 8
$items = New-Object System.Collections.Generic.List[string]
foreach ($p in $windows) {
  try {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
    if ($null -eq $root) { continue }
    $items.Add("WINDOW_TITLE: " + $p.MainWindowTitle)
    $nodes = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($node in $nodes) {
      try {
        $name = $node.Current.Name
        if (![string]::IsNullOrWhiteSpace($name)) { $items.Add($name) }
        $vp = $node.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        if ($vp -and ![string]::IsNullOrWhiteSpace($vp.Current.Value)) { $items.Add($vp.Current.Value) }
      } catch {}
    }
  } catch {}
}
if ($items.Count -eq 0) { exit 2 }
$items | Select-Object -Unique | Select-Object -First 2000
"#;

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000);
    }

    let output = timeout(Duration::from_secs(8), cmd.output())
        .await
        .map_err(|_| "自动读取 Codex 客户端窗口超时".to_string())?
        .map_err(|e| format!("无法执行 Windows UI Automation 读取：{e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() && !stdout.trim().is_empty() {
        return Ok(stdout);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err("没有找到可读取的 Codex / ChatGPT / OpenAI 客户端窗口。请打开 Codex 客户端并进入 Usage / Credits 页面。".to_string())
    } else {
        Err(format!("自动读取客户端窗口失败：{stderr}"))
    }
}

#[cfg(not(windows))]
async fn read_visible_codex_client_text() -> Result<String, String> {
    Err("Client Window 自动读取只支持 Windows。".to_string())
}

async fn read_codex_status_from_app_server() -> Result<CodexMeterStatus, String> {
    let updated_at_ms = now_ms();
    let mut client = AppServerClient::spawn().await?;
    client.initialize().await?;

    let account = client.request("account/read", json!({ "refreshToken": false })).await?;
    let account_obj = account.get("account").cloned().unwrap_or(Value::Null);
    if account_obj.is_null() {
        return Ok(CodexMeterStatus {
            ok: false,
            message: format!("已启动 Codex app-server（{}），但未返回账号。请确认 Codex 客户端已登录。", client.launched_from),
            source_mode: "app_server".to_string(),
            auth_mode: None,
            plan_type: None,
            primary: None,
            secondary: None,
            credit_balance: None,
            credit_limit: None,
            reset_credits_available: None,
            updated_at_ms,
        });
    }

    let auth_mode = account_obj.get("type").and_then(Value::as_str).map(str::to_string);
    let mut plan_type = account_obj.get("planType").and_then(Value::as_str).map(str::to_string);

    let limits = client.request("account/rateLimits/read", Value::Null).await?;
    let usage = client.request("account/usage/read", Value::Null).await.ok();

    let selected_limits = select_codex_rate_limits(&limits);
    if plan_type.is_none() {
        plan_type = selected_limits.get("planType").and_then(Value::as_str).map(str::to_string);
    }

    let primary = parse_limit("主额度", selected_limits.get("primary"));
    let secondary = parse_limit("副额度", selected_limits.get("secondary"));

    let reset_credits_available = limits
        .pointer("/rateLimitResetCredits/availableCount")
        .or_else(|| selected_limits.pointer("/rateLimitResetCredits/availableCount"))
        .and_then(Value::as_i64);

    let credit_balance = find_credit_number(Some(&limits), &["credit", "remaining"])
        .or_else(|| find_credit_number(Some(&limits), &["credit", "balance"]))
        .or_else(|| find_credit_number(Some(&limits), &["credit", "available"]))
        .or_else(|| find_credit_number(usage.as_ref(), &["credit", "balance"]));
    let credit_limit = find_credit_number(Some(&limits), &["credit", "limit"])
        .or_else(|| find_credit_number(Some(&limits), &["individual", "limit"]))
        .or_else(|| find_credit_number(usage.as_ref(), &["credit", "limit"]));

    let ok = primary.is_some() || secondary.is_some() || credit_balance.is_some() || reset_credits_available.is_some();

    Ok(CodexMeterStatus {
        ok,
        message: if ok { "OK".to_string() } else { format!("已连接 Codex app-server（{}），但没有返回可显示的额度字段。", client.launched_from) },
        source_mode: "app_server".to_string(),
        auth_mode,
        plan_type,
        primary,
        secondary,
        credit_balance,
        credit_limit,
        reset_credits_available,
        updated_at_ms,
    })
}

pub fn parse_client_usage_text(text: String) -> CodexMeterStatus {
    let updated_at_ms = now_ms();
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return CodexMeterStatus {
            ok: false,
            message: "Client Watch 模式未获得 Usage 页面文本。请打开 Codex 客户端的 Usage / Credits 页面，复制页面可见文字后点“从剪贴板读取”或粘贴到文本框。".to_string(),
            source_mode: "client_watch".to_string(),
            auth_mode: Some("client-visible-text".to_string()),
            plan_type: None,
            primary: None,
            secondary: None,
            credit_balance: None,
            credit_limit: None,
            reset_credits_available: None,
            updated_at_ms,
        };
    }

    let lines = normalized_lines(trimmed);
    let primary = parse_percent_window(
        &lines,
        "5小时",
        &["5-hour", "5 hour", "5h", "5 小时", "5小时", "five hour", "session", "primary", "short"],
        Some(300),
    );
    let secondary = parse_percent_window(
        &lines,
        "周额度",
        &["weekly", "week", "7-day", "7 day", "周", "secondary", "long"],
        Some(10080),
    );
    let (credit_balance, credit_limit) = parse_credits(&lines);
    let reset_credits_available = parse_reset_credit_count(&lines);
    let plan_type = parse_plan_type(&lines);

    let any = primary.is_some() || secondary.is_some() || credit_balance.is_some() || credit_limit.is_some() || reset_credits_available.is_some();

    CodexMeterStatus {
        ok: any,
        message: if any {
            "OK".to_string()
        } else {
            "已读取文本，但没有解析出额度。请尽量复制 Codex 客户端 Settings / Usage / Credits 页面完整可见文字。".to_string()
        },
        source_mode: "client_watch".to_string(),
        auth_mode: Some("client-visible-text".to_string()),
        plan_type,
        primary,
        secondary,
        credit_balance,
        credit_limit,
        reset_credits_available,
        updated_at_ms,
    }
}

fn parse_limit(label: &str, value: Option<&Value>) -> Option<LimitWindow> {
    let value = value?;
    if value.is_null() {
        return None;
    }
    let used = value.get("usedPercent").and_then(Value::as_f64);
    let remaining = used.map(|v| 100.0 - v);
    Some(LimitWindow {
        label: label.to_string(),
        used_percent: used,
        remaining_percent: remaining,
        window_duration_mins: value.get("windowDurationMins").and_then(Value::as_i64),
        resets_at: value.get("resetsAt").and_then(Value::as_i64),
        reset_text: None,
        reached_type: value.get("rateLimitReachedType").and_then(Value::as_str).map(str::to_string),
    })
}

fn select_codex_rate_limits(limits: &Value) -> &Value {
    if let Some(v) = limits.pointer("/rateLimitsByLimitId/codex") {
        return v;
    }
    if let Some(map) = limits.get("rateLimitsByLimitId").and_then(Value::as_object) {
        if let Some((_, value)) = map.iter().find(|(key, _)| key.to_ascii_lowercase().contains("codex")) {
            return value;
        }
    }
    limits.get("rateLimits").unwrap_or(limits)
}

fn normalized_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.replace('\u{00a0}', " ").trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn parse_percent_window(lines: &[String], label: &str, keywords: &[&str], duration: Option<i64>) -> Option<LimitWindow> {
    let lower: Vec<String> = lines.iter().map(|l| l.to_ascii_lowercase()).collect();
    for (idx, line_l) in lower.iter().enumerate() {
        if !keywords.iter().any(|k| line_l.contains(&k.to_ascii_lowercase())) {
            continue;
        }
        let mut block = lines[idx].clone();
        for off in 1..=3 {
            if let Some(next) = lines.get(idx + off) {
                block.push(' ');
                block.push_str(next);
            }
        }
        if let Some((used, remaining)) = parse_used_remaining_from_block(&block) {
            return Some(LimitWindow {
                label: label.to_string(),
                used_percent: used,
                remaining_percent: remaining,
                window_duration_mins: duration,
                resets_at: None,
                reset_text: extract_reset_text(&block),
                reached_type: None,
            });
        }
    }
    None
}

fn parse_used_remaining_from_block(block: &str) -> Option<(Option<f64>, Option<f64>)> {
    let lower = block.to_ascii_lowercase();
    let percents = extract_percent_numbers(block);
    if percents.is_empty() {
        return None;
    }
    let p = clamp_percent(percents[0]);
    let remaining_markers = ["remaining", "left", "available", "remain", "剩余", "可用", "剩下", "还剩"];
    let used_markers = ["used", "usage", "consumed", "已用", "使用", "消耗"];

    if remaining_markers.iter().any(|m| lower.contains(&m.to_ascii_lowercase())) {
        return Some((Some(100.0 - p), Some(p)));
    }
    if used_markers.iter().any(|m| lower.contains(&m.to_ascii_lowercase())) {
        return Some((Some(p), Some(100.0 - p)));
    }

    // Fallback: if the page shows a single percentage near a quota label, treat it as remaining.
    Some((Some(100.0 - p), Some(p)))
}

fn extract_percent_numbers(text: &str) -> Vec<f64> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let number: String = chars[start..i].iter().collect();
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && chars[j] == '%' {
                if let Ok(v) = number.parse::<f64>() {
                    out.push(v);
                }
            }
        } else {
            i += 1;
        }
    }
    out
}

fn extract_plain_numbers(text: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let mut token = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            token.push(ch);
        } else if !token.is_empty() {
            if let Ok(v) = token.replace(',', "").parse::<f64>() {
                out.push(v);
            }
            token.clear();
        }
    }
    if !token.is_empty() {
        if let Ok(v) = token.replace(',', "").parse::<f64>() {
            out.push(v);
        }
    }
    out
}

fn parse_credits(lines: &[String]) -> (Option<f64>, Option<f64>) {
    let mut balance = None;
    let mut limit = None;
    for (idx, line) in lines.iter().enumerate() {
        let lower = line.to_ascii_lowercase();
        if !(lower.contains("credit") || lower.contains("credits") || lower.contains("点数") || lower.contains("额度券")) {
            continue;
        }
        let mut block = line.clone();
        for off in 1..=2 {
            if let Some(next) = lines.get(idx + off) {
                block.push(' ');
                block.push_str(next);
            }
        }
        let block_l = block.to_ascii_lowercase();
        let numbers = extract_plain_numbers(&block);
        if numbers.is_empty() {
            continue;
        }
        if block_l.contains("balance") || block_l.contains("available") || block_l.contains("remaining") || block_l.contains("剩余") || block_l.contains("可用") {
            balance.get_or_insert(numbers[0]);
        }
        if block_l.contains("limit") || block_l.contains("total") || block_l.contains("monthly") || block_l.contains("上限") || block_l.contains("总") {
            limit.get_or_insert(*numbers.last().unwrap_or(&numbers[0]));
        }
        if numbers.len() >= 2 && (block_l.contains("/") || block_l.contains("of")) {
            balance.get_or_insert(numbers[0]);
            limit.get_or_insert(numbers[1]);
        } else if balance.is_none() {
            balance = Some(numbers[0]);
        }
    }
    (balance, limit)
}

fn parse_reset_credit_count(lines: &[String]) -> Option<i64> {
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if (lower.contains("reset") || lower.contains("重置")) && (lower.contains("credit") || lower.contains("credits") || lower.contains("券")) {
            for n in extract_plain_numbers(line) {
                return Some(n.round() as i64);
            }
        }
    }
    None
}

fn parse_plan_type(lines: &[String]) -> Option<String> {
    let all = lines.join(" ").to_ascii_lowercase();
    for plan in ["enterprise", "team", "business", "pro", "plus", "go", "free"] {
        if all.contains(plan) {
            return Some(plan.to_string());
        }
    }
    None
}

fn extract_reset_text(block: &str) -> Option<String> {
    let lower = block.to_ascii_lowercase();
    let markers = ["reset", "resets", "renews", "重置", "恢复", "刷新"];
    for marker in markers {
        if let Some(pos) = lower.find(marker) {
            let tail: String = block.chars().skip(pos).take(48).collect();
            return Some(tail.trim().to_string());
        }
    }
    None
}

fn clamp_percent(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

fn find_credit_number(value: Option<&Value>, keywords: &[&str]) -> Option<f64> {
    let value = value?;
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let key_l = key.to_ascii_lowercase();
                let matches = keywords.iter().all(|k| key_l.contains(&k.to_ascii_lowercase()));
                if matches {
                    if let Some(n) = child.as_f64() {
                        return Some(n);
                    }
                    if let Some(n) = first_number(child) {
                        return Some(n);
                    }
                }
                if let Some(n) = find_credit_number(Some(child), keywords) {
                    return Some(n);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|v| find_credit_number(Some(v), keywords)),
        _ => None,
    }
}

fn first_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::Object(map) => map.values().find_map(first_number),
        Value::Array(items) => items.iter().find_map(first_number),
        _ => None,
    }
}

fn codex_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(path) = env::var_os("LX_CODEX_EXE") {
        paths.push(PathBuf::from(path));
    }

    if let Some(path) = find_on_path("codex.exe") {
        paths.push(path);
    }
    if let Some(path) = find_on_path("codex") {
        paths.push(path);
    }

    #[cfg(windows)]
    {
        if let Some(local) = env::var_os("LOCALAPPDATA") {
            let bin_root = PathBuf::from(local).join("OpenAI").join("Codex").join("bin");
            let mut found = Vec::new();
            if let Ok(entries) = fs::read_dir(&bin_root) {
                for entry in entries.flatten() {
                    let path = entry.path().join("codex.exe");
                    if path.is_file() {
                        let modified = fs::metadata(&path)
                            .and_then(|m| m.modified())
                            .unwrap_or(SystemTime::UNIX_EPOCH);
                        found.push((modified, path));
                    }
                }
            }
            found.sort_by(|a, b| b.0.cmp(&a.0));
            for (_, path) in found {
                paths.push(path);
            }
        }

        if let Some(home) = env::var_os("USERPROFILE") {
            paths.push(PathBuf::from(&home).join(".codex").join("plugins").join(".plugin-appserver").join("codex.exe"));
        }
    }

    dedup_paths(paths)
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = Vec::<String>::new();
    let mut out = Vec::new();
    for path in paths {
        let key = path.to_string_lossy().to_ascii_lowercase();
        if seen.iter().any(|s| s == &key) {
            continue;
        }
        if path.file_name().is_some() || path.to_string_lossy() == "codex" {
            seen.push(key);
            out.push(path);
        }
    }
    out
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn redact_home(path: &str) -> String {
    for key in ["USERPROFILE", "HOME"] {
        if let Some(home) = env::var_os(key) {
            let home = home.to_string_lossy().to_string();
            if !home.is_empty() && path.starts_with(&home) {
                return path.replacen(&home, "%USERPROFILE%", 1);
            }
        }
    }
    path.to_string()
}

fn compact_json(value: &Value) -> String {
    let text = serde_json::to_string(value).unwrap_or_else(|_| "<json error>".to_string());
    if text.len() > 800 {
        format!("{}…", &text[..800])
    } else {
        text
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_watch_text() {
        let text = "ChatGPT Plus\n5-hour limit remaining 72% resets at 23:18\nWeekly limit used 59%\nCredits balance 12 of 30\nReset credits 2";
        let status = parse_client_usage_text(text.to_string());
        assert!(status.ok);
        assert_eq!(status.plan_type.as_deref(), Some("plus"));
        assert_eq!(status.primary.unwrap().remaining_percent.unwrap().round() as i32, 72);
        assert_eq!(status.secondary.unwrap().remaining_percent.unwrap().round() as i32, 41);
        assert_eq!(status.credit_balance.unwrap().round() as i32, 12);
        assert_eq!(status.credit_limit.unwrap().round() as i32, 30);
        assert_eq!(status.reset_credits_available, Some(2));
    }

    #[test]
    fn selects_codex_bucket() {
        let value = json!({
            "rateLimitsByLimitId": {
                "other": { "primary": { "usedPercent": 90 } },
                "codex": { "primary": { "usedPercent": 35, "windowDurationMins": 300 } }
            }
        });
        let selected = select_codex_rate_limits(&value);
        assert_eq!(selected.pointer("/primary/usedPercent").and_then(Value::as_i64), Some(35));
    }
}
