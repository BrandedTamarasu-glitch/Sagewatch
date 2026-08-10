use crate::{domain::Preferences, AppState};
use tauri::State;

#[tauri::command]
pub async fn set_preferences(
    mut preferences: Preferences,
    state: State<'_, AppState>,
) -> Result<Preferences, String> {
    // Login integration is deliberately controlled only by set_autostart_enabled so a
    // generic preference write can never change desktop login state or claim that it did.
    preferences.start_at_login = state.service.preferences().await.start_at_login;
    state
        .service
        .set_preferences(preferences)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn set_claude_usage_probe_enabled(
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<Preferences, String> {
    let mut preferences = state.service.preferences().await;
    preferences.claude_usage_probe_enabled = enabled;
    state
        .service
        .set_preferences(preferences)
        .await
        .map_err(|error| error.to_string())
}
