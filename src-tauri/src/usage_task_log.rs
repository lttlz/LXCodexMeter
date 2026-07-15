use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn ReplaceFileW(
        replaced_file_name: *const u16,
        replacement_file_name: *const u16,
        backup_file_name: *const u16,
        replace_flags: u32,
        exclude: *mut std::ffi::c_void,
        reserved: *mut std::ffi::c_void,
    ) -> i32;
}

const SCHEMA_VERSION: u32 = 3;
const MAX_TASKS: usize = 10_000;
const MIN_CONSUMPTION_PERCENT: f64 = 0.01;
const RESET_INCREASE_PERCENT: f64 = 10.0;
const IDLE_TIMEOUT_MS: u64 = 10 * 60 * 1_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub captured_at_ms: u64,
    pub weekly_remaining_percent: Option<f64>,
    pub five_hour_remaining_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageTask {
    pub id: String,
    pub started_at_ms: u64,
    pub ended_at_ms: u64,
    pub duration_seconds: u64,
    pub weekly_consumed_percent: Option<f64>,
    pub five_hour_consumed_percent: Option<f64>,
    #[serde(default)]
    pub end_weekly_remaining_percent: Option<f64>,
    #[serde(default)]
    pub end_five_hour_remaining_percent: Option<f64>,
    pub record_mode: String,
    pub is_complete: bool,
    pub is_estimated: bool,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct ActiveUsageTask {
    id: String,
    started_at_ms: u64,
    last_activity_at_ms: u64,
    #[serde(default)]
    last_observed_at_ms: u64,
    weekly_consumed_percent: Option<f64>,
    five_hour_consumed_percent: Option<f64>,
    #[serde(default)]
    end_weekly_remaining_percent: Option<f64>,
    #[serde(default)]
    end_five_hour_remaining_percent: Option<f64>,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl ActiveUsageTask {
    fn as_view(&self, now_ms: u64) -> UsageTaskView {
        let ended_at_ms = now_ms.max(self.last_activity_at_ms).max(self.started_at_ms);
        UsageTaskView {
            task: UsageTask {
                id: self.id.clone(),
                started_at_ms: self.started_at_ms,
                ended_at_ms,
                duration_seconds: ended_at_ms.saturating_sub(self.started_at_ms) / 1_000,
                weekly_consumed_percent: self.weekly_consumed_percent,
                five_hour_consumed_percent: self.five_hour_consumed_percent,
                end_weekly_remaining_percent: self.end_weekly_remaining_percent,
                end_five_hour_remaining_percent: self.end_five_hour_remaining_percent,
                record_mode: "automatic".to_string(),
                is_complete: false,
                is_estimated: false,
                created_at_ms: self.created_at_ms,
                updated_at_ms: self.updated_at_ms,
            },
            is_active: true,
        }
    }

    fn finish(self, ended_at_ms: u64, is_complete: bool, is_estimated: bool) -> UsageTask {
        let ended_at_ms = ended_at_ms.max(self.started_at_ms);
        UsageTask {
            id: self.id,
            started_at_ms: self.started_at_ms,
            ended_at_ms,
            duration_seconds: ended_at_ms.saturating_sub(self.started_at_ms) / 1_000,
            weekly_consumed_percent: self.weekly_consumed_percent,
            five_hour_consumed_percent: self.five_hour_consumed_percent,
            end_weekly_remaining_percent: self.end_weekly_remaining_percent,
            end_five_hour_remaining_percent: self.end_five_hour_remaining_percent,
            record_mode: "automatic".to_string(),
            is_complete,
            is_estimated,
            created_at_ms: self.created_at_ms,
            updated_at_ms: ended_at_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogPreferences {
    pub weekly_filter: String,
    pub custom_threshold: f64,
    pub time_filter: String,
    pub sort_mode: String,
}

impl Default for UsageLogPreferences {
    fn default() -> Self {
        Self {
            weekly_filter: "gte3".to_string(),
            custom_threshold: 3.0,
            time_filter: "30d".to_string(),
            sort_mode: "latest".to_string(),
        }
    }
}

impl UsageLogPreferences {
    fn normalized(mut self) -> Self {
        if !matches!(
            self.weekly_filter.as_str(),
            "all" | "gte1" | "gte3" | "gte5" | "custom"
        ) {
            self.weekly_filter = "gte3".to_string();
        }
        if !self.custom_threshold.is_finite() || !(0.0..=100.0).contains(&self.custom_threshold) {
            self.custom_threshold = 3.0;
        }
        if !matches!(self.time_filter.as_str(), "today" | "7d" | "30d" | "all") {
            self.time_filter = "30d".to_string();
        }
        if !matches!(self.sort_mode.as_str(), "latest" | "weekly" | "duration") {
            self.sort_mode = "latest".to_string();
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageLogData {
    #[serde(default = "legacy_schema_version")]
    schema_version: u32,
    #[serde(default)]
    last_snapshot: Option<UsageSnapshot>,
    #[serde(default)]
    active_task: Option<ActiveUsageTask>,
    #[serde(default)]
    tasks: Vec<UsageTask>,
    #[serde(default)]
    preferences: UsageLogPreferences,
}

fn legacy_schema_version() -> u32 {
    1
}

impl Default for UsageLogData {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            last_snapshot: None,
            active_task: None,
            tasks: Vec::new(),
            preferences: UsageLogPreferences::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageTaskView {
    #[serde(flatten)]
    pub task: UsageTask,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLogView {
    pub tasks: Vec<UsageTaskView>,
    pub preferences: UsageLogPreferences,
    pub warning: Option<String>,
}

pub struct UsageTaskStore {
    path: PathBuf,
    data: UsageLogData,
    warning: Option<String>,
}

impl UsageTaskStore {
    pub fn load(path: PathBuf, now_ms: u64) -> Self {
        let mut warning = None;
        let mut data = if path.exists() {
            match fs::read_to_string(&path)
                .map_err(|_| "无法读取消耗日志".to_string())
                .and_then(|text| {
                    serde_json::from_str::<UsageLogData>(&text)
                        .map_err(|_| "消耗日志已损坏，已创建新的空日志".to_string())
                }) {
                Ok(mut parsed) => {
                    parsed.preferences = parsed.preferences.normalized();
                    parsed
                }
                Err(message) => {
                    warning = Some(message);
                    preserve_corrupt_file(&path, now_ms);
                    UsageLogData::default()
                }
            }
        } else {
            UsageLogData::default()
        };

        if data.schema_version < 2 {
            data.schema_version = SCHEMA_VERSION;
            data.last_snapshot = None;
            warning = Some("消耗日志已升级，旧额度基准已安全失效".to_string());
        } else if data.schema_version < SCHEMA_VERSION {
            data.schema_version = SCHEMA_VERSION;
        }

        if let Some(active) = data.active_task.take() {
            let ended_at_ms = active
                .last_observed_at_ms
                .max(active.last_activity_at_ms)
                .max(active.started_at_ms);
            data.tasks.push(active.finish(ended_at_ms, false, true));
            trim_oldest(&mut data.tasks);
            warning.get_or_insert_with(|| "上次未结束的任务已标记为不完整".to_string());
        }

        let mut store = Self {
            path,
            data,
            warning,
        };
        if let Err(error) = store.persist() {
            store.warning = Some(error);
        }
        store
    }

    pub fn view(&self, now_ms: u64) -> UsageLogView {
        let mut tasks: Vec<UsageTaskView> = self
            .data
            .tasks
            .iter()
            .cloned()
            .map(|task| UsageTaskView {
                task,
                is_active: false,
            })
            .collect();
        if let Some(active) = &self.data.active_task {
            tasks.push(active.as_view(now_ms));
        }
        UsageLogView {
            tasks,
            preferences: self.data.preferences.clone(),
            warning: self.warning.clone(),
        }
    }

    pub fn record_snapshot(
        &mut self,
        mut current: UsageSnapshot,
        allow_consumption: bool,
    ) -> Result<(), String> {
        current.weekly_remaining_percent = valid_percent(current.weekly_remaining_percent);
        current.five_hour_remaining_percent = valid_percent(current.five_hour_remaining_percent);

        let Some(previous) = self.data.last_snapshot.clone() else {
            self.data.last_snapshot = Some(current);
            return self.persist();
        };

        if !allow_consumption {
            if let Some(active) = self.data.active_task.as_mut() {
                active.last_observed_at_ms =
                    current.captured_at_ms.max(active.last_observed_at_ms);
                active.end_weekly_remaining_percent = current.weekly_remaining_percent;
                active.end_five_hour_remaining_percent = current.five_hour_remaining_percent;
            }
            self.data.last_snapshot = Some(replace_baselines(current));
            return self.persist();
        }

        let weekly = quota_change(
            previous.weekly_remaining_percent,
            current.weekly_remaining_percent,
        );
        let five_hour = quota_change(
            previous.five_hour_remaining_percent,
            current.five_hour_remaining_percent,
        );
        let end_weekly_remaining_percent = current.weekly_remaining_percent;
        let end_five_hour_remaining_percent = current.five_hour_remaining_percent;

        if weekly.reset || five_hour.reset {
            self.finish_active(current.captured_at_ms, true, false);
        }

        if weekly.consumed > 0.0 || five_hour.consumed > 0.0 {
            self.add_consumption(
                current.captured_at_ms,
                weekly,
                five_hour,
                end_weekly_remaining_percent,
                end_five_hour_remaining_percent,
            );
        }
        if let Some(active) = self.data.active_task.as_mut() {
            active.last_observed_at_ms = current.captured_at_ms.max(active.last_observed_at_ms);
            active.end_weekly_remaining_percent = end_weekly_remaining_percent;
            active.end_five_hour_remaining_percent = end_five_hour_remaining_percent;
        }

        self.data.last_snapshot = Some(UsageSnapshot {
            captured_at_ms: current.captured_at_ms,
            weekly_remaining_percent: weekly.next_baseline,
            five_hour_remaining_percent: five_hour.next_baseline,
        });
        self.persist()
    }

    pub fn close_idle_task(&mut self, now_ms: u64) -> Result<bool, String> {
        let should_close =
            self.data.active_task.as_ref().is_some_and(|task| {
                now_ms.saturating_sub(task.last_activity_at_ms) >= IDLE_TIMEOUT_MS
            });
        if !should_close {
            return Ok(false);
        }
        let ended_at_ms = self
            .data
            .active_task
            .as_ref()
            .map(|task| task.last_activity_at_ms)
            .unwrap_or(now_ms);
        self.finish_active(ended_at_ms, true, false);
        self.persist()?;
        Ok(true)
    }

    pub fn finish_for_process_exit(
        &mut self,
        now_ms: u64,
        final_refresh_ok: bool,
    ) -> Result<(), String> {
        if self.data.active_task.is_none() {
            return Ok(());
        }
        let ended_at_ms = self
            .data
            .active_task
            .as_ref()
            .map(|task| task.last_observed_at_ms.max(task.last_activity_at_ms))
            .unwrap_or(now_ms);
        self.finish_active(ended_at_ms, true, !final_refresh_ok);
        self.persist()
    }

    pub fn finish_for_app_exit(&mut self, now_ms: u64) -> Result<(), String> {
        if self.data.active_task.is_none() {
            return Ok(());
        }
        let ended_at_ms = self
            .data
            .active_task
            .as_ref()
            .map(|task| task.last_observed_at_ms.max(task.last_activity_at_ms))
            .unwrap_or(now_ms);
        self.finish_active(ended_at_ms, false, true);
        self.persist()
    }

    pub fn delete_task(&mut self, id: &str) -> Result<bool, String> {
        let before = self.data.tasks.len();
        self.data.tasks.retain(|task| task.id != id);
        if self.data.tasks.len() == before {
            return Ok(false);
        }
        self.persist()?;
        Ok(true)
    }

    pub fn clear_history(&mut self) -> Result<(), String> {
        self.data.tasks.clear();
        self.persist()
    }

    pub fn save_preferences(&mut self, preferences: UsageLogPreferences) -> Result<(), String> {
        self.data.preferences = preferences.normalized();
        self.persist()
    }

    fn add_consumption(
        &mut self,
        captured_at_ms: u64,
        weekly: QuotaChange,
        five_hour: QuotaChange,
        end_weekly_remaining_percent: Option<f64>,
        end_five_hour_remaining_percent: Option<f64>,
    ) {
        let task = self
            .data
            .active_task
            .get_or_insert_with(|| ActiveUsageTask {
                id: unique_task_id(captured_at_ms, &self.data.tasks),
                started_at_ms: captured_at_ms,
                last_activity_at_ms: captured_at_ms,
                last_observed_at_ms: captured_at_ms,
                weekly_consumed_percent: weekly.available.then_some(0.0),
                five_hour_consumed_percent: five_hour.available.then_some(0.0),
                end_weekly_remaining_percent,
                end_five_hour_remaining_percent,
                created_at_ms: captured_at_ms,
                updated_at_ms: captured_at_ms,
            });
        if weekly.available {
            *task.weekly_consumed_percent.get_or_insert(0.0) += weekly.consumed;
        }
        if five_hour.available {
            *task.five_hour_consumed_percent.get_or_insert(0.0) += five_hour.consumed;
        }
        task.last_activity_at_ms = captured_at_ms.max(task.last_activity_at_ms);
        task.updated_at_ms = captured_at_ms.max(task.updated_at_ms);
    }

    fn finish_active(&mut self, ended_at_ms: u64, is_complete: bool, is_estimated: bool) {
        if let Some(active) = self.data.active_task.take() {
            self.data
                .tasks
                .push(active.finish(ended_at_ms, is_complete, is_estimated));
            trim_oldest(&mut self.data.tasks);
        }
    }

    fn persist(&mut self) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "消耗日志目录无效".to_string())?;
        fs::create_dir_all(parent).map_err(|_| "无法创建消耗日志目录".to_string())?;
        let bytes =
            serde_json::to_vec_pretty(&self.data).map_err(|_| "无法序列化消耗日志".to_string())?;
        safe_replace(&self.path, &bytes).inspect_err(|message| {
            self.warning = Some(message.clone());
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct QuotaChange {
    consumed: f64,
    reset: bool,
    next_baseline: Option<f64>,
    available: bool,
}

fn quota_change(previous: Option<f64>, current: Option<f64>) -> QuotaChange {
    let previous = valid_percent(previous);
    let current = valid_percent(current);
    let Some(current) = current else {
        return QuotaChange {
            consumed: 0.0,
            reset: false,
            next_baseline: None,
            available: false,
        };
    };
    let Some(previous) = previous else {
        return QuotaChange {
            consumed: 0.0,
            reset: false,
            next_baseline: Some(current),
            available: true,
        };
    };
    let decrease = previous - current;
    if decrease + f64::EPSILON >= MIN_CONSUMPTION_PERCENT {
        return QuotaChange {
            consumed: decrease.max(0.0),
            reset: false,
            next_baseline: Some(current),
            available: true,
        };
    }
    let increase = current - previous;
    let reset = increase >= RESET_INCREASE_PERCENT && current >= 90.0;
    QuotaChange {
        consumed: 0.0,
        reset,
        next_baseline: Some(if reset { current } else { previous }),
        available: true,
    }
}

fn valid_percent(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
}

fn replace_baselines(current: UsageSnapshot) -> UsageSnapshot {
    UsageSnapshot {
        captured_at_ms: current.captured_at_ms,
        weekly_remaining_percent: current.weekly_remaining_percent,
        five_hour_remaining_percent: current.five_hour_remaining_percent,
    }
}

fn unique_task_id(started_at_ms: u64, tasks: &[UsageTask]) -> String {
    let mut suffix = tasks.len() + 1;
    loop {
        let candidate = format!("task-{started_at_ms}-{suffix}");
        if !tasks.iter().any(|task| task.id == candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn trim_oldest(tasks: &mut Vec<UsageTask>) {
    if tasks.len() > MAX_TASKS {
        let remove = tasks.len() - MAX_TASKS;
        tasks.drain(0..remove);
    }
}

fn preserve_corrupt_file(path: &Path, now_ms: u64) {
    let backup = path.with_file_name(format!("usage-task-log.corrupt-{now_ms}.json"));
    let _ = fs::rename(path, backup);
}

fn safe_replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension("json.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp)
        .map_err(|_| "无法创建消耗日志临时文件".to_string())?;
    file.write_all(bytes)
        .map_err(|_| "无法写入消耗日志临时文件".to_string())?;
    file.sync_all()
        .map_err(|_| "无法同步消耗日志临时文件".to_string())?;
    drop(file);

    if !path.exists() {
        return fs::rename(&temp, path).map_err(|_| "无法保存消耗日志".to_string());
    }

    replace_existing(path, &temp)
}

#[cfg(windows)]
fn replace_existing(path: &Path, temp: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    let replaced: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let replacement: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
    let result = unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if result != 0 {
        return Ok(());
    }
    let _ = fs::remove_file(temp);
    Err("无法原子替换消耗日志，原日志已保留".to_string())
}

#[cfg(not(windows))]
fn replace_existing(path: &Path, temp: &Path) -> Result<(), String> {
    let backup = path.with_extension("json.previous");

    let _ = fs::remove_file(&backup);
    fs::rename(path, &backup).map_err(|_| "无法准备消耗日志安全替换".to_string())?;
    if fs::rename(temp, path).is_err() {
        let _ = fs::rename(&backup, path);
        return Err("无法替换消耗日志，原日志已保留".to_string());
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

#[cfg(test)]
include!("usage_task_log_tests.rs");
