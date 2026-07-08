mod meter;

use meter::{parse_client_usage_text, read_status, CodexMeterStatus, MeterConfig};
use std::{fs, path::PathBuf, process::Command};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_updater::UpdaterExt;

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("failed to locate config dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create config dir: {e}"))?;
    Ok(dir.join("config.json"))
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
fn load_config(app: tauri::AppHandle) -> Result<MeterConfig, String> {
    let path = config_path(&app)?;
    if !path.exists() {
        return Ok(MeterConfig::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("failed to read config: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid config json: {e}"))
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: MeterConfig) -> Result<(), String> {
    let path = config_path(&app)?;
    let text = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("failed to serialize config: {e}"))?;
    fs::write(path, text).map_err(|e| format!("failed to save config: {e}"))
}

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn open_project_url() -> Result<(), String> {
    const URL: &str = "https://github.com/lttlz/LXCodexMeter";

    #[cfg(target_os = "windows")]
    {
        Command::new("rundll32")
            .arg("url.dll,FileProtocolHandler")
            .arg(URL)
            .spawn()
            .map_err(|e| format!("failed to open url: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(URL)
            .spawn()
            .map_err(|e| format!("failed to open url: {e}"))?;
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(URL)
            .spawn()
            .map_err(|e| format!("failed to open url: {e}"))?;
        Ok(())
    }
}

/// Check for an available update via the Tauri updater plugin.
/// Returns `Some(version)` when a newer release is available, otherwise `None`.
/// Only contacts the official endpoints configured in `tauri.conf.json`.
#[tauri::command]
async fn check_for_updates(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let updater = app
        .updater()
        .map_err(|e| format!("failed to access updater: {e}"))?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(update.version)),
        Ok(None) => Ok(None),
        Err(e) => Err(format!("update check failed: {e}")),
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let strip = MenuItem::with_id(app, "toggle_strip", "Toggle Strip Mode", true, None::<&str>)?;
            let toggle = MenuItem::with_id(app, "toggle", "Show / Hide", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&refresh, &settings, &strip, &toggle, &quit])?;

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

            // Background update check shortly after startup (non-blocking, silent).
            // Only contacts the official GitHub/Gitee endpoints configured in tauri.conf.json.
            // Emits `updater-available` with the new version string when an update exists.
            let updater_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(6)).await;
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
            check_for_updates
        ])
        .run(tauri::generate_context!())
        .expect("LX Codex Meter failed to start");
}
