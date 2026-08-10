pub mod commands;
pub mod domain;
pub mod providers;
pub mod service;
pub mod store;

use service::RefreshService;
use std::sync::Arc;
use tauri::Manager;

pub struct AppState {
    pub service: Arc<RefreshService>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            let service = RefreshService::bootstrap(app_data_dir)?;
            app.manage(AppState {
                service: Arc::new(service),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::get_status,
            commands::status::refresh_provider,
            commands::preferences::set_preferences,
            commands::diagnostics::get_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sagewatch");
}
