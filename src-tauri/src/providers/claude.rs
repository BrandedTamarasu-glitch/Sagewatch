use super::*;
use crate::domain::{
    AllowanceWindow, Freshness, Health, Provider, ProviderStatus, Source, SourceConfidence,
    WindowKind,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct ClaudeAdapter {
    path: PathBuf,
    stale_after: Duration,
}
impl ClaudeAdapter {
    pub fn new(path: impl Into<PathBuf>, stale_after: Duration) -> Self {
        Self {
            path: path.into(),
            stale_after,
        }
    }
}
#[derive(Deserialize)]
struct Snapshot {
    observed_at: DateTime<Utc>,
    #[serde(default)]
    plan: Option<String>,
    rate_limits: RateLimits,
}
#[derive(Deserialize)]
struct RateLimits {
    five_hour: Option<Window>,
    seven_day: Option<Window>,
    seven_day_sonnet: Option<Window>,
    seven_day_opus: Option<Window>,
    extra_usage: Option<Window>,
    #[serde(default)]
    models: BTreeMap<String, Window>,
}
#[derive(Deserialize)]
struct Window {
    #[serde(default, alias = "used_percentage")]
    utilization: Option<f64>,
    #[serde(default)]
    remaining_percentage: Option<f64>,
    #[serde(alias = "reset_at")]
    resets_at: Option<DateTime<Utc>>,
}
impl ClaudeAdapter {
    fn load(&self) -> Result<ProviderStatus, AdapterError> {
        let metadata = fs::symlink_metadata(&self.path)
            .map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 64 * 1024 {
            return Err(ae(AdapterErrorKind::InvalidData, false));
        }
        let bytes = fs::read(&self.path).map_err(|_| ae(AdapterErrorKind::Unavailable, true))?;
        let snapshot: Snapshot = serde_json::from_slice(&bytes)
            .map_err(|_| ae(AdapterErrorKind::SourceChanged, false))?;
        let mut windows = Vec::new();
        if let Some(w) = snapshot.rate_limits.five_hour {
            windows.push(cw(
                "five_hour",
                "Five hour",
                WindowKind::Rolling,
                Some(300),
                w,
            )?);
        }
        if let Some(w) = snapshot.rate_limits.seven_day {
            windows.push(cw(
                "seven_day",
                "Seven day",
                WindowKind::Weekly,
                Some(10080),
                w,
            )?);
        }
        if let Some(w) = snapshot.rate_limits.seven_day_sonnet {
            windows.push(cw(
                "seven_day_sonnet",
                "Sonnet seven day",
                WindowKind::ModelScoped,
                Some(10080),
                w,
            )?);
        }
        if let Some(w) = snapshot.rate_limits.seven_day_opus {
            windows.push(cw(
                "seven_day_opus",
                "Opus seven day",
                WindowKind::ModelScoped,
                Some(10080),
                w,
            )?);
        }
        if let Some(w) = snapshot.rate_limits.extra_usage {
            windows.push(cw(
                "extra_usage",
                "Extra usage",
                WindowKind::Credits,
                None,
                w,
            )?);
        }
        for (model, w) in snapshot.rate_limits.models {
            if model.is_empty() || model.len() > 128 {
                return Err(ae(AdapterErrorKind::InvalidData, false));
            }
            windows.push(cw(
                &format!("model:{model}"),
                &model,
                WindowKind::ModelScoped,
                None,
                w,
            )?);
        }
        if windows.is_empty() {
            return Err(ae(AdapterErrorKind::SourceChanged, false));
        }
        let age = Utc::now()
            .signed_duration_since(snapshot.observed_at)
            .to_std()
            .unwrap_or_default();
        Ok(ProviderStatus {
            schema_version: 1,
            provider: Provider::Claude,
            plan: snapshot.plan.unwrap_or_else(|| "unknown".into()),
            observed_at: snapshot.observed_at,
            last_successful_at: Some(snapshot.observed_at),
            source: Source::ClaudeStatusline,
            source_confidence: SourceConfidence::DocumentedLocal,
            freshness: if age > self.stale_after {
                Freshness::Stale
            } else {
                Freshness::Live
            },
            health: Health::Healthy,
            headline_window_id: None,
            windows,
        })
    }
}
fn cw(
    id: &str,
    label: &str,
    kind: WindowKind,
    duration: Option<u32>,
    w: Window,
) -> Result<AllowanceWindow, AdapterError> {
    if w.utilization.is_none() && w.remaining_percentage.is_none() {
        return Err(ae(AdapterErrorKind::SourceChanged, false));
    }
    if w.utilization
        .iter()
        .chain(w.remaining_percentage.iter())
        .any(|value| !value.is_finite())
    {
        return Err(ae(AdapterErrorKind::SourceChanged, false));
    }
    Ok(AllowanceWindow {
        id: id.into(),
        label: label.into(),
        duration_minutes: duration,
        used_percent: w.utilization,
        remaining_percent: w.remaining_percentage,
        reset_at: w.resets_at,
        kind,
        is_active: true,
    })
}
fn ae(kind: AdapterErrorKind, retryable: bool) -> AdapterError {
    AdapterError {
        kind,
        message: safe_summary(kind).into(),
        retryable,
    }
}
#[async_trait]
impl ProviderAdapter for ClaudeAdapter {
    async fn probe(&self) -> CapabilityReport {
        CapabilityReport {
            available: self.path.is_file(),
            compatible: true,
            detail: if self.path.is_file() {
                "Claude statusline snapshot available".into()
            } else {
                "Claude statusline snapshot unavailable".into()
            },
        }
    }
    async fn refresh(&self) -> Result<ProviderStatus, AdapterError> {
        self.load()
    }
    fn diagnose(&self, error: &AdapterError) -> ProviderDiagnostics {
        ProviderDiagnostics {
            provider: Provider::Claude,
            health: health_for_error(error.kind),
            summary: safe_summary(error.kind).into(),
            retryable: error.retryable,
        }
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: Provider::Claude,
            supports_refresh: true,
            session_attached: true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_sanitized_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("status.json");
        fs::write(&path,format!(r#"{{"observed_at":"{}","plan":"team","rate_limits":{{"five_hour":{{"utilization":20,"resets_at":null}},"seven_day":{{"utilization":80,"resets_at":null}},"models":{{"sonnet":{{"utilization":50,"resets_at":null}}}}}}}}"#,Utc::now().to_rfc3339())).unwrap();
        let status = ClaudeAdapter::new(path, Duration::from_secs(60))
            .load()
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(status.windows.len(), 3);
        assert_eq!(status.headline_window_id.as_deref(), Some("seven_day"));
    }

    #[test]
    fn reads_shared_bridge_model_contract_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/claude-model-statusline.json");
        let status = ClaudeAdapter::new(path, Duration::from_secs(60))
            .load()
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(status.plan, "Claude Team");
        assert_eq!(status.windows.len(), 1);
        assert_eq!(status.windows[0].id, "model:sonnet-5");
        assert_eq!(status.windows[0].remaining_percent, Some(42.0));
        assert_eq!(status.windows[0].kind, WindowKind::ModelScoped);
    }
}
