mod meter;

use meter::{parse_client_usage_text, read_status, CodexMeterStatus, MeterConfig};
use std::{fs, path::PathBuf, process::Command, time::Duration};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_updater::UpdaterExt;

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

// --- Codex process detection (name only; never reads Codex files/sessions/tokens) ---
#[cfg(windows)]
fn codex_running() -> bool {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new("tasklist");
    cmd.args(["/FI", "IMAGENAME eq codex.exe", "/NH"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    // CREATE_NO_WINDOW: never flash a console.
    cmd.creation_flags(0x08000000);
    let out = match cmd.output() {
        Ok(o) => o,
        Err(_) => return false, // silent degrade
    };
    let text = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    text.contains("codex.exe")
}

#[cfg(not(windows))]
fn codex_running() -> bool {
    false
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
/// Check for an available update. Returns `Some(version)` when a newer release
/// is available, otherwise `None`. Does not cache the Update object; the
/// download-and-install command re-checks to keep state simple and stable.
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

/// Download and install the latest update. Re-checks the manifest (does not
/// rely on a cached Update), then downloads and installs. Emits
/// `updater-progress` (downloaded, total) while downloading and
/// `updater-installed` when the installer has finished.
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
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let cfg = load_config_inner(&app.handle());
            let lang = cfg.language.clone();

            let menu = make_menu(app, &lang)?;

            let tray_icon = app
                .default_window_icon()
                .cloned()
                .ok_or("failed to load default tray icon")?;

            TrayIconBuilder::with_id("main")
                .tooltip("LX Codex Meter")
                .icon(tray_icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "refresh" => {
                        let _ = app.emit("meter-refresh-requested", ());
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                        let _ = app.emit("meter-settings-requested", ());
                    }
                    "toggle_strip" => {
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
                                    let _ = window.hide();
                                }
                                Ok(false) => {
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
                .on_tray_icon_event(|tray, event| {
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
                                    let _ = window.hide();
                                }
                                Ok(false) => {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                                Err(_) => {}
                            }
                        }
                    }
                })
                .build(app)?;

            // Start hidden to tray: hide the main window on launch when enabled.
            // The frontend show/hide effect has a one-shot guard so it does not
            // re-show the window on first mount.
            if cfg.start_hidden {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            // Codex process watcher: only detects whether codex.exe is running.
            // Never reads Codex files, sessions, or credentials. tasklist
            // failures degrade silently (no error, no crash).
            let codex_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut prev = codex_running();
                loop {
                    std::thread::sleep(Duration::from_secs(4));
                    let c = load_config_inner(&codex_handle);
                    if !c.auto_show_on_codex && !c.auto_hide_on_codex_close {
                        prev = false;
                        continue;
                    }
                    let cur = codex_running();
                    if cur && !prev && c.auto_show_on_codex {
                        if let Some(window) = codex_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    } else if !cur && prev && c.auto_hide_on_codex_close {
                        if let Some(window) = codex_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    prev = cur;
                }
            });

            // Background update check shortly after startup (non-blocking, silent).
            // Only contacts the official GitHub/Gitee endpoints configured in tauri.conf.json.
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
            open_project_url,
            check_for_updates,
            download_and_install_update,
            restart_app,
            set_autostart,
            get_autostart_enabled,
            rebuild_tray_menu
        ])
        .run(tauri::generate_context!())
        .expect("LX Codex Meter failed to start");
}
