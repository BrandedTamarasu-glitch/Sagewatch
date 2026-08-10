use crate::{
    domain::{Provider, ProviderStatus},
    service::AppSnapshot,
    AppState,
};
use tauri::State;

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> Result<AppSnapshot, String> {
    Ok(state.service.get_status().await)
}

#[tauri::command]
pub async fn refresh_provider(
    provider: Provider,
    state: State<'_, AppState>,
) -> Result<ProviderStatus, String> {
    state
        .service
        .refresh_provider(provider)
        .await
        .map_err(|error| error.to_string())
}
