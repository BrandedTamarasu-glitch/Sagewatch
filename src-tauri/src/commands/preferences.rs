use crate::{domain::Preferences, AppState};
use tauri::State;

#[tauri::command]
pub async fn set_preferences(
    preferences: Preferences,
    state: State<'_, AppState>,
) -> Result<Preferences, String> {
    state
        .service
        .set_preferences(preferences)
        .await
        .map_err(|error| error.to_string())
}
