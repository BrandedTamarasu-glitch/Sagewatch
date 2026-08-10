use crate::AppState;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_notification::NotificationExt;

const MAX_NOTIFICATION_TITLE_CHARS: usize = 120;
const MAX_NOTIFICATION_BODY_CHARS: usize = 500;

fn validate_notification(title: &str, body: &str) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("notification title must not be empty".into());
    }
    if title.chars().count() > MAX_NOTIFICATION_TITLE_CHARS
        || body.chars().count() > MAX_NOTIFICATION_BODY_CHARS
    {
        return Err("notification content is too long".into());
    }
    Ok(())
}

fn verified_autostart_result(requested: bool, verified: bool) -> Result<bool, String> {
    if verified != requested {
        return Err(format!(
            "desktop autostart verified as {verified}, not the requested {requested} state"
        ));
    }
    Ok(verified)
}

#[tauri::command]
pub fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| format!("could not read the desktop autostart setting: {error}"))
}

#[tauri::command]
pub async fn set_autostart_enabled(
    enabled: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let autostart = app.autolaunch();
    let was_enabled = autostart
        .is_enabled()
        .map_err(|error| format!("could not read the desktop autostart setting: {error}"))?;

    if enabled != was_enabled {
        let result = if enabled {
            autostart.enable()
        } else {
            autostart.disable()
        };
        result
            .map_err(|error| format!("could not update the desktop autostart setting: {error}"))?;
    }

    let verified_enabled = autostart.is_enabled().map_err(|error| {
        format!(
            "autostart may have changed, but its resulting state could not be verified: {error}"
        )
    })?;
    let mut preferences = state.service.preferences().await;
    preferences.start_at_login = verified_enabled;
    if let Err(error) = state.service.set_preferences(preferences).await {
        if verified_enabled != was_enabled {
            let _ = if was_enabled {
                autostart.enable()
            } else {
                autostart.disable()
            };
        }
        return Err(format!(
            "could not persist the autostart preference: {error}"
        ));
    }

    verified_autostart_result(enabled, verified_enabled)
}

#[tauri::command]
pub fn show_desktop_notification(
    title: String,
    body: String,
    app: AppHandle,
) -> Result<(), String> {
    validate_notification(&title, &body)?;
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| format!("desktop notification could not be delivered: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_rejects_empty_title() {
        assert!(validate_notification("  ", "body").is_err());
    }

    #[test]
    fn notification_rejects_oversized_content() {
        let title = "x".repeat(MAX_NOTIFICATION_TITLE_CHARS + 1);
        assert!(validate_notification(&title, "body").is_err());
        let body = "x".repeat(MAX_NOTIFICATION_BODY_CHARS + 1);
        assert!(validate_notification("title", &body).is_err());
    }

    #[test]
    fn notification_accepts_a_bounded_message() {
        assert_eq!(
            validate_notification("Allowance low", "Claude has 10% remaining"),
            Ok(())
        );
    }

    #[test]
    fn autostart_returns_only_the_requested_verified_state() {
        assert_eq!(verified_autostart_result(true, true), Ok(true));
        assert_eq!(verified_autostart_result(false, false), Ok(false));
        assert!(verified_autostart_result(true, false).is_err());
        assert!(verified_autostart_result(false, true).is_err());
    }
}
