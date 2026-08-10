use crate::{domain::Provider, providers::ProviderDiagnostics, AppState};
use std::collections::BTreeMap;
use tauri::State;

#[tauri::command]
pub async fn get_diagnostics(
    state: State<'_, AppState>,
) -> Result<BTreeMap<Provider, Option<ProviderDiagnostics>>, String> {
    Ok(state.service.get_diagnostics().await)
}
