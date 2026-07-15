mod meter;
mod quota;
mod usage_csv;
mod usage_runtime;
mod usage_task_log;

use meter::{parse_client_usage_text, CodexMeterStatus, MeterConfig};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_updater::UpdaterExt;
use usage_runtime::UsageRuntime;
use usage_csv::UsageCsvRow;
use usage_task_log::{UsageLogPreferences, UsageLogView};

/// Shared flag: set when the user manually shows/maintains the window after
/// Codex has closed. While true, the Codex watcher must not auto-hide. Reset to
/// false on the next Codex false->true transition (new cycle).
#[derive(Clone)]
struct ManualShow(Arc<AtomicBool>);

impl ManualShow {
    fn set(&self, value: bool) {
        self.0.store(value, Ordering::SeqCst);
    }
}

#[derive(Clone)]
struct ManualHidden(Arc<AtomicBool>);

impl ManualHidden {
    fn set(&self, value: bool) {
        self.0.store(value, Ordering::SeqCst);
    }

    fn get(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// Whether the watcher currently considers a user Codex/ChatGPT cycle active.
/// ManualHidden is meaningful only while this flag is true.
#[derive(Clone)]
struct CodexKnownRunning(Arc<AtomicBool>);

impl CodexKnownRunning {
    fn set(&self, value: bool) {
        self.0.store(value, Ordering::SeqCst);
    }

    fn get(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

fn hide_window_for_user<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
    manual_hidden: &ManualHidden,
    codex_known_running: &CodexKnownRunning,
) -> tauri::Result<()> {
    let previous = manual_hidden.get();
    manual_hidden.set(codex_known_running.get());

    if let Err(error) = window.hide() {
        manual_hidden.set(previous);
        return Err(error);
    }

    Ok(())
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("failed to locate config dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create config dir: {e}"))?;
    Ok(dir.join("config.json"))
}

/// Read config from disk without going through a command (used by setup + background tasks).
fn load_config_inner(app: &AppHandle) -> MeterConfig {
    let path = match config_path(app) {
        Ok(p) => p,
        Err(_) => return MeterConfig::default(),
    };
    if !path.exists() {
        return MeterConfig::default();
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return MeterConfig::default(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

#[tauri::command]
async fn get_status(
    app: AppHandle,
    usage_runtime: tauri::State<'_, UsageRuntime>,
    mode: Option<String>,
    client_text: Option<String>,
) -> Result<Option<CodexMeterStatus>, String> {
    if !should_collect_usage(is_main_window_visible(&app), usage_runtime.target_running())
        && !usage_runtime.final_refresh_pending()
    {
        #[cfg(debug_assertions)]
        eprintln!("status fetch skipped: window hidden and no target process");
        return Ok(None);
    }

    #[cfg(debug_assertions)]
    eprintln!("status fetch started through coordinated usage channel");
    usage_runtime
        .fetch_status(mode, client_text, false, true)
        .await
        .map(Some)
}

fn is_main_window_visible(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

fn should_collect_usage(window_visible: bool, target_process_running: bool) -> bool {
    window_visible || target_process_running
}

#[tauri::command]
fn parse_usage_text(text: String) -> Result<CodexMeterStatus, String> {
    Ok(parse_client_usage_text(text))
}

#[tauri::command]
fn get_usage_log(usage_runtime: tauri::State<'_, UsageRuntime>) -> Result<UsageLogView, String> {
    usage_runtime.log_view()
}

#[tauri::command]
fn delete_usage_task(
    usage_runtime: tauri::State<'_, UsageRuntime>,
    id: String,
) -> Result<bool, String> {
    usage_runtime.delete_task(&id)
}

#[tauri::command]
fn clear_usage_tasks(usage_runtime: tauri::State<'_, UsageRuntime>) -> Result<(), String> {
    usage_runtime.clear_history()
}

#[tauri::command]
async fn export_usage_csv(
    rows: Vec<UsageCsvRow>,
    language: String,
    file_name: String,
) -> Result<bool, String> {
    usage_csv::export_usage_csv(rows, language, file_name).await
}

#[tauri::command]
fn save_usage_log_preferences(
    usage_runtime: tauri::State<'_, UsageRuntime>,
    preferences: UsageLogPreferences,
) -> Result<(), String> {
    usage_runtime.save_preferences(preferences)
}

#[tauri::command]
fn load_config(app: AppHandle) -> Result<MeterConfig, String> {
    Ok(load_config_inner(&app))
}

#[tauri::command]
fn save_config(app: AppHandle, config: MeterConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    let text = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("failed to serialize config: {e}"))?;
    fs::write(path, text).map_err(|e| format!("failed to save config: {e}"))
}

#[tauri::command]
fn exit_app(app: AppHandle, usage_runtime: tauri::State<'_, UsageRuntime>) {
    usage_runtime.finish_for_app_exit();
    app.exit(0);
}

#[tauri::command]
fn hide_to_tray(
    app: AppHandle,
    manual_hidden: tauri::State<'_, ManualHidden>,
    codex_known_running: tauri::State<'_, CodexKnownRunning>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    hide_window_for_user(&window, &manual_hidden, &codex_known_running)
        .map_err(|error| format!("failed to hide main window: {error}"))?;

    Ok(())
}

// --- Strict URL whitelist (no query, no fragment, no arbitrary tag) ---
const GITHUB_URL: &str = "https://github.com/lttlz/LXCodexMeter";
const GITEE_URL: &str = "https://gitee.com/lttlz/LXCodexMeter";
const GITHUB_RELEASES: &str = "https://github.com/lttlz/LXCodexMeter/releases";
const GITEE_RELEASES: &str = "https://gitee.com/lttlz/LXCodexMeter/releases";

fn is_allowed_url(url: &str) -> bool {
    if url.contains('?') || url.contains('#') {
        return false;
    }
    let ver = env!("CARGO_PKG_VERSION");
    let github_tag = format!("{GITHUB_URL}/releases/tag/v{ver}");
    let gitee_tag = format!("{GITEE_URL}/releases/tag/v{ver}");
    url == GITHUB_URL
        || url == GITEE_URL
        || url == GITHUB_RELEASES
        || url == GITEE_RELEASES
        || url == github_tag
        || url == gitee_tag
}

#[tauri::command]
fn open_project_url(url: Option<String>) -> Result<(), String> {
    let target = match url.as_deref() {
        None => GITHUB_URL,
        Some(u) => {
            if !is_allowed_url(u) {
                return Err("unsupported project url".to_string());
            }
            u
        }
    };

    #[cfg(target_os = "windows")]
    {
        Command::new("rundll32")
            .arg("url.dll,FileProtocolHandler")
            .arg(target)
            .spawn()
            .map_err(|e| format!("failed to open url: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("failed to open url: {e}"))?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("failed to open url: {e}"))?;
        Ok(())
    }
}

// --- Autostart (Tauri v2 official plugin; writes HKCU Run key, no admin) ---
#[tauri::command]
fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launch = app.autolaunch();
    if enabled {
        launch
            .enable()
            .map_err(|e| format!("启用开机启动失败：{e}"))
    } else {
        launch
            .disable()
            .map_err(|e| format!("禁用开机启动失败：{e}"))
    }
}

#[tauri::command]
fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|e| format!("读取开机启动状态失败：{e}"))
}

// --- Codex / ChatGPT process detection ---
// Detects whether a *user* Codex/ChatGPT is running by enumerating process names
// and parent PIDs through sysinfo. Only a codex.exe whose direct parent is this
// LXCodexMeter process is excluded. No command lines or executable paths are read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CodexDetectionResult {
    running: bool,
    matched_chatgpt: usize,
    matched_codex: usize,
    excluded_meter_children: usize,
}

fn classify_codex_process(
    result: &mut CodexDetectionResult,
    process_name: &str,
    is_meter_child: bool,
) {
    match process_name.to_ascii_lowercase().as_str() {
        "chatgpt.exe" => {
            result.matched_chatgpt += 1;
            result.running = true;
        }
        "codex.exe" if is_meter_child => {
            result.excluded_meter_children += 1;
        }
        "codex.exe" => {
            result.matched_codex += 1;
            result.running = true;
        }
        _ => {}
    }
}

#[cfg(windows)]
fn detect_codex_processes(system: &mut sysinfo::System) -> CodexDetectionResult {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate};

    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().without_tasks(),
    );

    let meter_pid = Pid::from_u32(std::process::id());
    let mut result = CodexDetectionResult::default();

    for process in system.processes().values() {
        classify_codex_process(
            &mut result,
            &process.name().to_string_lossy(),
            process.parent() == Some(meter_pid),
        );
    }

    result
}

#[cfg(not(windows))]
fn detect_codex_processes(_system: &mut sysinfo::System) -> CodexDetectionResult {
    CodexDetectionResult::default()
}

// Show through Tauri so its managed visibility state stays synchronized. A
// successful show requests one immediate frontend refresh; automatic shows do
// not focus the window, while tray/menu shows retain their focus behavior.
fn show_window_and_request_refresh(app: &AppHandle, focus: bool) {
    if let Some(window) = app.get_webview_window("main") {
        if window.show().is_ok() {
            if focus {
                let _ = window.set_focus();
            }
            let _ = app.emit("meter-refresh-requested", ());
        }
    }
}

/// Called from the frontend whenever the user manually opens/focuses the window
/// (settings button, context menu, tray-driven open). The watcher resets this
/// marker on the next Codex cycle; it does not suppress a confirmed close.
#[tauri::command]
fn mark_manual_show(manual_show: tauri::State<'_, ManualShow>) {
    manual_show.set(true);
}

/// Returns true only when this process was launched by the OS autostart entry
/// (the `--autostart` arg is present) AND the user enabled start_hidden. A
/// manual launch (desktop shortcut, start menu, exe double-click) has no
/// `--autostart` arg, so this returns false and the window shows normally.
#[tauri::command]
fn should_start_hidden(app: AppHandle) -> bool {
    let cfg = load_config_inner(&app);
    let autostart_launch = std::env::args().any(|a| a == "--autostart");
    cfg.start_hidden && autostart_launch
}

// --- Tray menu localization ---
fn tray_labels(
    lang: &str,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match lang {
        "en" => (
            "Refresh",
            "Settings",
            "Toggle Strip Mode",
            "Show / Hide",
            "Quit",
        ),
        _ => ("刷新", "设置", "切换任务栏条模式", "显示 / 隐藏", "退出"),
    }
}

fn make_menu<R: tauri::Runtime>(
    manager: &impl tauri::Manager<R>,
    lang: &str,
) -> tauri::Result<Menu<R>> {
    let (s_refresh, s_settings, s_strip, s_toggle, s_quit) = tray_labels(lang);
    let refresh = MenuItem::with_id(manager, "refresh", s_refresh, true, None::<&str>)?;
    let settings = MenuItem::with_id(manager, "settings", s_settings, true, None::<&str>)?;
    let strip = MenuItem::with_id(manager, "toggle_strip", s_strip, true, None::<&str>)?;
    let toggle = MenuItem::with_id(manager, "toggle", s_toggle, true, None::<&str>)?;
    let quit = MenuItem::with_id(manager, "quit", s_quit, true, None::<&str>)?;
    Menu::with_items(manager, &[&refresh, &settings, &strip, &toggle, &quit])
}

#[tauri::command]
fn rebuild_tray_menu(app: AppHandle, lang: String) -> Result<(), String> {
    let menu = make_menu(&app, &lang).map_err(|e| format!("rebuild tray menu: {e}"))?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))
            .map_err(|e| format!("set tray menu: {e}"))?;
    }
    Ok(())
}

// --- Updater ---
#[tauri::command]
async fn check_for_updates(app: AppHandle) -> Result<Option<String>, String> {
    let updater = app
        .updater()
        .map_err(|e| format!("failed to access updater: {e}"))?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(update.version)),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("update check failed: {e}")),
    }
}

#[tauri::command]
async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    let updater = app
        .updater()
        .map_err(|e| format!("failed to access updater: {e}"))?;
    let update = updater
        .check()
        .await
        .map_err(|e| format!("update check failed: {e}"))?
        .ok_or_else(|| "no update available".to_string())?;

    let app_for_chunk = app.clone();
    let app_for_finish = app.clone();
    update
        .download_and_install(
            move |downloaded, total| {
                let _ = app_for_chunk.emit("updater-progress", (downloaded as u64, total));
            },
            move || {
                let _ = app_for_finish.emit("updater-installed", ());
            },
        )
        .await
        .map_err(|e| format!("download and install failed: {e}"))?;
    Ok(())
}

#[tauri::command]
fn restart_app(app: AppHandle) {
    app.restart();
}

pub fn run() {
    let manual_show = ManualShow(Arc::new(AtomicBool::new(false)));
    let manual_show_for_setup = manual_show.clone();
    let manual_hidden = ManualHidden(Arc::new(AtomicBool::new(false)));
    let manual_hidden_for_setup = manual_hidden.clone();
    let codex_known_running = CodexKnownRunning(Arc::new(AtomicBool::new(false)));
    let codex_known_running_for_setup = codex_known_running.clone();
    let usage_runtime = UsageRuntime::new();
    let usage_runtime_for_setup = usage_runtime.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            // Pass --autostart so the launched process can tell it was started
            // by the OS autostart entry (vs. a manual user launch). This is how
            // `should_start_hidden` distinguishes the two.
            Some(vec!["--autostart"]),
        ))
        .manage(manual_show)
        .manage(manual_hidden)
        .manage(codex_known_running)
        .manage(usage_runtime)
        .setup(move |app| {
            let cfg = load_config_inner(app.handle());
            let lang = cfg.language.clone();
            let autostart_launch = std::env::args().any(|a| a == "--autostart");

            if let Ok(data_dir) = app.path().app_data_dir() {
                usage_runtime_for_setup.initialize_store(data_dir.join("usage-task-log.json"));
            }

            if cfg.start_hidden && autostart_launch {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            let menu = make_menu(app, &lang)?;

            let tray_icon = app
                .default_window_icon()
                .cloned()
                .ok_or("failed to load default tray icon")?;

            let ms_menu = manual_show_for_setup.clone();
            let ms_tray = manual_show_for_setup.clone();
            let ms_watcher = manual_show_for_setup.clone();
            let mh_menu = manual_hidden_for_setup.clone();
            let mh_tray = manual_hidden_for_setup.clone();
            let mh_watcher = manual_hidden_for_setup.clone();
            let ckr_menu = codex_known_running_for_setup.clone();
            let ckr_tray = codex_known_running_for_setup.clone();
            let ckr_watcher = codex_known_running_for_setup.clone();
            let usage_quit = usage_runtime_for_setup.clone();
            let usage_watcher = usage_runtime_for_setup.clone();
            let usage_collector = usage_runtime_for_setup.clone();

            TrayIconBuilder::with_id("main")
                .tooltip("LX Codex Meter")
                .icon(tray_icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "refresh" => {
                        if is_main_window_visible(app) {
                            let _ = app.emit("meter-refresh-requested", ());
                        }
                    }
                    "settings" => {
                        // user-initiated show -> mark manual
                        ms_menu.set(true);
                        mh_menu.set(false);
                        show_window_and_request_refresh(app, true);
                        let _ = app.emit("meter-settings-requested", ());
                    }
                    "toggle_strip" => {
                        ms_menu.set(true);
                        mh_menu.set(false);
                        show_window_and_request_refresh(app, true);
                        let _ = app.emit("meter-toggle-strip-requested", ());
                    }
                    "toggle" => {
                        if let Some(window) = app.get_webview_window("main") {
                            match window.is_visible() {
                                Ok(true) => {
                                    let _ = hide_window_for_user(&window, &mh_menu, &ckr_menu);
                                }
                                Ok(false) => {
                                    // user chose to show -> manual
                                    ms_menu.set(true);
                                    mh_menu.set(false);
                                    show_window_and_request_refresh(app, true);
                                }
                                Err(_) => {}
                            }
                        }
                    }
                    "quit" => {
                        usage_quit.finish_for_app_exit();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            match window.is_visible() {
                                Ok(true) => {
                                    let _ = hide_window_for_user(&window, &mh_tray, &ckr_tray);
                                }
                                Ok(false) => {
                                    // user left-click to show -> manual
                                    ms_tray.set(true);
                                    mh_tray.set(false);
                                    show_window_and_request_refresh(app, true);
                                }
                                Err(_) => {}
                            }
                        }
                    }
                })
                .build(app)?;

            // start_hidden is handled early in backend setup, with the frontend
            // `should_start_hidden` command kept as a fallback. Both paths only
            // hide when launched by the OS autostart entry (--autostart arg) AND
            // start_hidden is on. A manual launch never hides.

            // Codex/ChatGPT process watcher.
            // - Detects real user Codex/ChatGPT (excludes our own app-server child).
            // - auto-show uses Tauri show() without set_focus(), preserving
            //   managed visibility synchronization; runtime acceptance confirms no focus steal.
            // - auto-hide fires at most ONCE per true->false transition, and
            //   is not blocked by manual tray/settings interactions.
            // - Closing requires 2 consecutive false readings (debounce) so a
            //   short-lived app-server child doesn't trigger a hide/show flicker.
            let codex_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut prev = false;
                let mut close_confirm = 0u32;
                let mut auto_hidden_this_cycle = false;
                let mut process_system = sysinfo::System::new();
                #[cfg(debug_assertions)]
                let mut last_detection = None;

                loop {
                    let c = load_config_inner(&codex_handle);
                    let detection = detect_codex_processes(&mut process_system);
                    #[cfg(debug_assertions)]
                    if last_detection != Some(detection) {
                        eprintln!(
                            "[codex-detection] running={} matched_chatgpt={} matched_codex={} excluded_meter_children={}",
                            detection.running,
                            detection.matched_chatgpt,
                            detection.matched_codex,
                            detection.excluded_meter_children
                        );
                        last_detection = Some(detection);
                    }

                    let cur = detection.running;
                    let mut sleep_next;
                    if cur {
                        ckr_watcher.set(true);
                        usage_watcher.set_target_running(true);
                        close_confirm = 0;
                        sleep_next = Duration::from_secs(1);

                        if !prev {
                            // false -> true: new cycle, reset manual show + allow one hide
                            ms_watcher.set(false);
                            auto_hidden_this_cycle = false;
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "[codex-lifecycle] false -> true matched_chatgpt={} matched_codex={} excluded_meter_children={}",
                                detection.matched_chatgpt,
                                detection.matched_codex,
                                detection.excluded_meter_children
                            );
                            if c.auto_show_on_codex {
                                if mh_watcher.get() {
                                    #[cfg(debug_assertions)]
                                    eprintln!(
                                        "[codex-lifecycle] auto-show suppressed by manual hide in current cycle"
                                    );
                                } else {
                                    show_window_and_request_refresh(&codex_handle, false);
                                }
                            }
                        }
                        prev = true;
                    } else if prev {
                        close_confirm = close_confirm.saturating_add(1);
                        sleep_next = Duration::from_millis(500);

                        if close_confirm >= 2 {
                            // confirmed true -> false
                            ckr_watcher.set(false);
                            usage_watcher.set_target_running(false);
                            mh_watcher.set(false);
                            #[cfg(debug_assertions)]
                            eprintln!(
                                "[codex-lifecycle] true -> false matched_chatgpt={} matched_codex={} excluded_meter_children={}",
                                detection.matched_chatgpt,
                                detection.matched_codex,
                                detection.excluded_meter_children
                            );
                            if c.auto_hide_on_codex_close && !auto_hidden_this_cycle {
                                if let Some(window) = codex_handle.get_webview_window("main") {
                                    let _ = window.hide();
                                }
                                auto_hidden_this_cycle = true;
                            }
                            prev = false;
                            close_confirm = 0;
                            sleep_next = Duration::from_secs(4);
                        }
                    } else {
                        ckr_watcher.set(false);
                        usage_watcher.set_target_running(false);
                        mh_watcher.set(false);
                        close_confirm = 0;
                        sleep_next = Duration::from_secs(4);
                    }

                    std::thread::sleep(sleep_next);
                }
            });

            // One coordinated backend collector drives both visible and hidden
            // refreshes. The frontend never owns the recurring timer, and every
            // request passes through UsageRuntime's single-flight lock.
            let usage_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut last_fetch = tokio::time::Instant::now();
                let mut last_idle_check = tokio::time::Instant::now();
                let mut last_config_check = tokio::time::Instant::now();
                let mut refresh_interval = Duration::from_secs(
                    load_config_inner(&usage_handle).refresh_interval_secs.max(60),
                );
                let mut was_collecting = false;

                // Let the initial frontend request establish the first snapshot.
                // If it did not, the recent-result cache below still makes this
                // an immediate fallback without starting a concurrent app-server.
                tokio::time::sleep(Duration::from_secs(2)).await;

                loop {
                    if last_config_check.elapsed() >= Duration::from_secs(10) {
                        refresh_interval = Duration::from_secs(
                            load_config_inner(&usage_handle).refresh_interval_secs.max(60),
                        );
                        last_config_check = tokio::time::Instant::now();
                    }
                    if last_idle_check.elapsed() >= Duration::from_secs(5) {
                        usage_collector.close_idle_task();
                        last_idle_check = tokio::time::Instant::now();
                    }

                    let mut fetched_this_cycle = false;
                    if usage_collector.take_final_refresh_request() {
                        let final_result = usage_collector
                            .fetch_status(Some("app_server".to_string()), Some(String::new()), true, true)
                            .await;
                        let final_ok = final_result.as_ref().is_ok_and(|status| status.ok);
                        if let Ok(status) = final_result {
                            let _ = usage_handle.emit("meter-status-updated", status);
                        }
                        usage_collector.finish_for_process_exit(final_ok);
                        last_fetch = tokio::time::Instant::now();
                        fetched_this_cycle = true;
                    }

                    let should_collect = should_collect_usage(
                        is_main_window_visible(&usage_handle),
                        usage_collector.target_running(),
                    );
                    if should_collect
                        && !fetched_this_cycle
                        && (!was_collecting || last_fetch.elapsed() >= refresh_interval)
                    {
                        if let Ok(status) = usage_collector
                            .fetch_status(Some("app_server".to_string()), Some(String::new()), false, false)
                            .await
                        {
                            let _ = usage_handle.emit("meter-status-updated", status);
                        }
                        last_fetch = tokio::time::Instant::now();
                    }
                    was_collecting = should_collect;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            });

            // Background update check shortly after startup. Do not contact the
            // update endpoint until the main window has actually become visible.
            let updater_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(6)).await;
                loop {
                    if is_main_window_visible(&updater_handle)
                        && is_main_window_visible(&updater_handle)
                    {
                        break;
                    }
                    #[cfg(debug_assertions)]
                    eprintln!("auto update deferred: window hidden");
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }

                #[cfg(debug_assertions)]
                eprintln!("auto update started: window visible");
                match updater_handle.updater() {
                    Ok(updater) => match updater.check().await {
                        Ok(Some(update)) => {
                            let _ = updater_handle.emit("updater-available", update.version);
                        }
                        Ok(None) => {}
                        Err(e) => eprintln!("update check failed: {e}"),
                    },
                    Err(e) => eprintln!("updater plugin unavailable: {e}"),
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            parse_usage_text,
            get_usage_log,
            delete_usage_task,
            clear_usage_tasks,
            export_usage_csv,
            save_usage_log_preferences,
            load_config,
            save_config,
            exit_app,
            hide_to_tray,
            open_project_url,
            check_for_updates,
            download_and_install_update,
            restart_app,
            set_autostart,
            get_autostart_enabled,
            rebuild_tray_menu,
            mark_manual_show,
            should_start_hidden
        ])
        .run(tauri::generate_context!())
        .expect("LX Codex Meter failed to start");
}

#[cfg(test)]
mod tests {
    use super::{classify_codex_process, should_collect_usage, CodexDetectionResult};

    #[test]
    fn collection_policy_matches_visibility_and_process_matrix() {
        assert!(should_collect_usage(true, false));
        assert!(should_collect_usage(true, true));
        assert!(should_collect_usage(false, true));
        assert!(!should_collect_usage(false, false));
    }

    #[test]
    fn detection_excludes_only_meter_codex_children() {
        let mut result = CodexDetectionResult::default();

        classify_codex_process(&mut result, "codex.exe", true);
        classify_codex_process(&mut result, "Codex.exe", false);
        classify_codex_process(&mut result, "ChatGPT.exe", false);

        assert!(result.running);
        assert_eq!(result.matched_chatgpt, 1);
        assert_eq!(result.matched_codex, 1);
        assert_eq!(result.excluded_meter_children, 1);
    }

    #[test]
    fn detection_ignores_generic_and_helper_processes() {
        let mut result = CodexDetectionResult::default();

        for name in [
            "node.exe",
            "Code.exe",
            "Cursor.exe",
            "codex-code-mode-host.exe",
            "codex-windows-sandbox-setup.exe",
        ] {
            classify_codex_process(&mut result, name, false);
        }

        assert_eq!(result, CodexDetectionResult::default());
    }

    #[test]
    fn meter_child_alone_does_not_mark_codex_running() {
        let mut result = CodexDetectionResult::default();

        classify_codex_process(&mut result, "codex.exe", true);

        assert!(!result.running);
        assert_eq!(result.excluded_meter_children, 1);
    }
}
