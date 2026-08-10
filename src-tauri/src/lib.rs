pub mod commands;
pub mod domain;
pub mod providers;
pub mod service;
pub mod store;

use service::RefreshService;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;

pub struct AppState {
    pub service: Arc<RefreshService>,
    tray_interaction_confirmed: AtomicBool,
}

fn should_hide_on_close(tray_interaction_confirmed: bool) -> bool {
    tray_interaction_confirmed
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &refresh, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Sagewatch")
        .menu(&menu)
        .on_tray_icon_event(|tray, _event| {
            tray.app_handle()
                .state::<AppState>()
                .tray_interaction_confirmed
                .store(true, Ordering::Release);
        })
        .on_menu_event(|app, event| {
            app.state::<AppState>()
                .tray_interaction_confirmed
                .store(true, Ordering::Release);
            match event.id.as_ref() {
                "show" => show_main_window(app),
                "hide" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.hide();
                    }
                }
                "refresh" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<AppState>();
                        let service = state.service.clone();
                        let (claude, codex) = tokio::join!(
                            service.refresh_provider(crate::domain::Provider::Claude),
                            service.refresh_provider(crate::domain::Provider::Codex)
                        );
                        let snapshot = service.snapshot().await;
                        // Refresh failures remain isolated in the snapshot diagnostics. Emission
                        // is best-effort because a closing webview must not block tray use.
                        let _ = app.emit("sagewatch://status-updated", &snapshot);
                        if claude.is_err() && codex.is_err() {
                            let _ = app.emit("sagewatch://refresh-failed", ());
                        }
                    });
                }
                "quit" => app.exit(0),
                _ => {}
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let service = RefreshService::bootstrap(app_data_dir)?;
            app.manage(AppState {
                service: Arc::new(service),
                tray_interaction_confirmed: AtomicBool::new(false),
            });
            create_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let tray_confirmed = window
                        .state::<AppState>()
                        .tray_interaction_confirmed
                        .load(Ordering::Acquire);
                    if should_hide_on_close(tray_confirmed) {
                        api.prevent_close();
                        let _ = window.hide();
                    } else {
                        // Until the user has interacted with the tray, its visibility has not
                        // been proven for this desktop session. Exit instead of risking a hidden,
                        // unreachable process on shells without tray support.
                        window.app_handle().exit(0);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::get_status,
            commands::status::refresh_provider,
            commands::preferences::set_preferences,
            commands::diagnostics::get_diagnostics,
            commands::desktop::get_autostart_enabled,
            commands::desktop::set_autostart_enabled,
            commands::desktop::show_desktop_notification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sagewatch");
}

#[cfg(test)]
mod tests {
    use super::should_hide_on_close;

    #[test]
    fn close_exits_until_tray_recovery_path_is_confirmed() {
        assert!(!should_hide_on_close(false));
    }

    #[test]
    fn close_hides_after_a_tray_interaction_confirms_recovery() {
        assert!(should_hide_on_close(true));
    }
}
