mod meter;

use meter::{parse_client_usage_text, read_status, CodexMeterStatus, MeterConfig};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::{atomic::{AtomicBool, Ordering}, Arc},
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_updater::UpdaterExt;

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
    mode: Option<String>,
    client_text: Option<String>,
) -> Result<CodexMeterStatus, String> {
    read_status(mode, client_text).await
}

#[tauri::command]
fn parse_usage_text(text: String) -> Result<CodexMeterStatus, String> {
    Ok(parse_client_usage_text(text))
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
fn exit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn hide_to_tray(
    app: AppHandle,
    manual_hidden: tauri::State<'_, ManualHidden>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    manual_hidden.set(true);

    if let Err(error) = window.hide() {
        manual_hidden.set(false);
        return Err(format!("failed to hide main window: {error}"));
    }

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
// Detects whether a *user* Codex/ChatGPT is running by enumerating processes
// through sysinfo. Rules:
//   1. ChatGPT.exe exists -> user is running ChatGPT/Codex entry -> running
//   2. codex.exe / Codex.exe exists AND CommandLine does NOT contain
//      `app-server` or `--stdio` -> real user Codex -> running
//   3. codex.exe with `app-server` / `--stdio` in CommandLine is LXCodexMeter's
//      own data-source child -> EXCLUDED, does not count.
// Never reads Codex files, sessions, tokens, or credentials. Command lines are
// used only for this in-memory classification and are never logged or emitted.
#[cfg(windows)]
fn codex_running(system: &mut sysinfo::System) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};

    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_cmd(UpdateKind::Always),
    );

    for process in system.processes().values() {
        let name = process.name().to_string_lossy().to_ascii_lowercase();

        if name == "chatgpt.exe" {
            return true;
        }

        if name == "codex.exe" {
            let command_line = process.cmd();
            if command_line.is_empty() {
                return true;
            }

            let is_meter_data_source = command_line.iter().any(|argument| {
                let argument = argument.to_string_lossy().to_ascii_lowercase();
                argument.contains("app-server") || argument.contains("--stdio")
            });

            if !is_meter_data_source {
                return true;
            }
        }
    }

    false
}

#[cfg(not(windows))]
fn codex_running(_system: &mut sysinfo::System) -> bool {
    false
}

// Show the window WITHOUT stealing focus (Windows SW_SHOWNOACTIVATE). Used by
// the Codex auto-show so it never interrupts the user's current input. Manual
// shows (tray/menu) still use the normal show()+set_focus().
#[cfg(windows)]
fn show_window_no_activate(app: &AppHandle) {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(hwnd) = w.hwnd() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
    }
}

#[cfg(not(windows))]
fn show_window_no_activate(_app: &AppHandle) {}

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
fn tray_labels(lang: &str) -> (&'static str, &'static str, &'static str, &'static str, &'static str) {
    match lang {
        "en" => (
            "Refresh",
            "Settings",
            "Toggle Strip Mode",
            "Show / Hide",
            "Quit",
        ),
        _ => (
            "刷新",
            "设置",
            "切换任务栏条模式",
            "显示 / 隐藏",
            "退出",
        ),
    }
}

fn make_menu<R: tauri::Runtime>(manager: &impl tauri::Manager<R>, lang: &str) -> tauri::Result<Menu<R>> {
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
        .setup(move |app| {
            let cfg = load_config_inner(&app.handle());
            let lang = cfg.language.clone();
            let autostart_launch = std::env::args().any(|a| a == "--autostart");

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

            TrayIconBuilder::with_id("main")
                .tooltip("LX Codex Meter")
                .icon(tray_icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "refresh" => {
                        let _ = app.emit("meter-refresh-requested", ());
                    }
                    "settings" => {
                        // user-initiated show -> mark manual
                        ms_menu.set(true);
                        mh_menu.set(false);
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("meter-settings-requested", ());
                    }
                    "toggle_strip" => {
                        ms_menu.set(true);
                        mh_menu.set(false);
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("meter-toggle-strip-requested", ());
                    }
                    "toggle" => {
                        if let Some(window) = app.get_webview_window("main") {
                            match window.is_visible() {
                                Ok(true) => {
                                    mh_menu.set(true);
                                    let _ = window.hide();
                                }
                                Ok(false) => {
                                    // user chose to show -> manual
                                    ms_menu.set(true);
                                    mh_menu.set(false);
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                                Err(_) => {}
                            }
                        }
                    }
                    "quit" => {
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
                                    mh_tray.set(true);
                                    let _ = window.hide();
                                }
                                Ok(false) => {
                                    // user left-click to show -> manual
                                    ms_tray.set(true);
                                    mh_tray.set(false);
                                    let _ = window.show();
                                    let _ = window.set_focus();
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
            // - auto-show uses SW_SHOWNOACTIVATE (no focus steal).
            // - auto-hide fires at most ONCE per true->false transition, and
            //   is not blocked by manual tray/settings interactions.
            // - Closing requires 2 consecutive false readings (debounce) so a
            //   short-lived app-server child doesn't trigger a hide/show flicker.
            let codex_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut prev = false;
                let mut close_confirm = 0u32;
                let mut auto_hidden_this_cycle = false;
                let mut sleep_next = Duration::from_secs(4);
                let mut process_system = sysinfo::System::new();

                loop {
                    std::thread::sleep(sleep_next);
                    let c = load_config_inner(&codex_handle);
                    if !c.auto_show_on_codex && !c.auto_hide_on_codex_close {
                        prev = false;
                        close_confirm = 0;
                        auto_hidden_this_cycle = false;
                        sleep_next = Duration::from_secs(4);
                        continue;
                    }

                    let cur = codex_running(&mut process_system);
                    if cur {
                        close_confirm = 0;
                        sleep_next = Duration::from_secs(1);

                        if !prev {
                            // false -> true: new cycle, reset manual flag + allow one hide
                            ms_watcher.set(false);
                            auto_hidden_this_cycle = false;
                            if c.auto_show_on_codex && !mh_watcher.get() {
                                show_window_no_activate(&codex_handle);
                            }
                        }
                        prev = true;
                    } else if prev {
                        close_confirm = close_confirm.saturating_add(1);
                        sleep_next = Duration::from_millis(500);

                        if close_confirm >= 2 {
                            // confirmed true -> false
                            mh_watcher.set(false);
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
                        close_confirm = 0;
                        sleep_next = Duration::from_secs(4);
                    }
                }
            });

            // Background update check shortly after startup (non-blocking, silent).
            let updater_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_secs(6)).await;
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
