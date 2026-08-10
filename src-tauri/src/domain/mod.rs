use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    ClaudeStatusline,
    CodexAppServer,
    CodexRolloutCache,
    Manual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceConfidence {
    DocumentedLocal,
    ExperimentalLocal,
    SensitiveLocalCache,
    Manual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Live,
    Recent,
    Stale,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Health {
    Healthy,
    SignedOut,
    Unavailable,
    Unsupported,
    SourceChanged,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowKind {
    Rolling,
    Weekly,
    ModelScoped,
    Credits,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AllowanceWindow {
    pub id: String,
    pub label: String,
    pub duration_minutes: Option<u32>,
    pub used_percent: Option<f64>,
    pub remaining_percent: Option<f64>,
    pub reset_at: Option<DateTime<Utc>>,
    pub kind: WindowKind,
    pub is_active: bool,
}

impl AllowanceWindow {
    pub fn normalize(mut self) -> Self {
        self.id = self.id.trim().to_owned();
        self.label = self.label.trim().to_owned();
        self.used_percent = finite_clamped(self.used_percent);
        self.remaining_percent = finite_clamped(self.remaining_percent)
            .or_else(|| self.used_percent.map(|used| 100.0 - used));
        self.used_percent = self
            .used_percent
            .or_else(|| self.remaining_percent.map(|remaining| 100.0 - remaining));
        self
    }
}

fn finite_clamped(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite()).map(|v| v.clamp(0.0, 100.0))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub schema_version: u32,
    pub provider: Provider,
    pub plan: String,
    pub observed_at: DateTime<Utc>,
    pub last_successful_at: Option<DateTime<Utc>>,
    pub source: Source,
    pub source_confidence: SourceConfidence,
    pub freshness: Freshness,
    pub health: Health,
    pub headline_window_id: Option<String>,
    pub windows: Vec<AllowanceWindow>,
}

impl ProviderStatus {
    pub fn normalize(mut self) -> Result<Self, ValidationError> {
        self.schema_version = SCHEMA_VERSION;
        self.plan = match self.plan.trim() {
            "" => "unknown".into(),
            value => value.into(),
        };
        self.windows = self
            .windows
            .into_iter()
            .map(AllowanceWindow::normalize)
            .collect();
        self.validate_windows()?;
        self.headline_window_id = select_headline(&self.windows).map(|window| window.id.clone());
        if self.health == Health::Healthy {
            self.last_successful_at = Some(self.observed_at);
        }
        Ok(self)
    }

    pub fn headline_window(&self) -> Option<&AllowanceWindow> {
        let id = self.headline_window_id.as_deref()?;
        self.windows.iter().find(|window| window.id == id)
    }

    fn validate_windows(&self) -> Result<(), ValidationError> {
        let mut ids = BTreeSet::new();
        for window in &self.windows {
            if window.id.is_empty() {
                return Err(ValidationError::EmptyWindowId);
            }
            if !ids.insert(&window.id) {
                return Err(ValidationError::DuplicateWindowId(window.id.clone()));
            }
        }
        Ok(())
    }
}

pub fn select_headline(windows: &[AllowanceWindow]) -> Option<&AllowanceWindow> {
    windows
        .iter()
        .filter(|window| window.is_active)
        .min_by(|left, right| {
            let l = left.remaining_percent.unwrap_or(f64::INFINITY);
            let r = right.remaining_percent.unwrap_or(f64::INFINITY);
            l.total_cmp(&r)
                .then_with(|| left.reset_at.cmp(&right.reset_at))
                .then_with(|| left.id.cmp(&right.id))
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeFormat {
    Local12Hour,
    Local24Hour,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Preferences {
    pub refresh_interval_seconds: u64,
    pub time_format: TimeFormat,
    pub alert_thresholds: Vec<u8>,
    pub alerts_enabled: bool,
    pub codex_rollout_fallback_enabled: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            refresh_interval_seconds: 300,
            time_format: TimeFormat::Local12Hour,
            alert_thresholds: vec![20, 10],
            alerts_enabled: false,
            codex_rollout_fallback_enabled: false,
        }
    }
}

impl Preferences {
    pub fn normalize(mut self) -> Result<Self, ValidationError> {
        self.refresh_interval_seconds = self.refresh_interval_seconds.clamp(30, 86_400);
        if self.alert_thresholds.iter().any(|value| *value > 100) {
            return Err(ValidationError::InvalidAlertThreshold);
        }
        self.alert_thresholds.sort_unstable_by(|a, b| b.cmp(a));
        self.alert_thresholds.dedup();
        Ok(self)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    #[error("allowance window id must not be empty")]
    EmptyWindowId,
    #[error("duplicate allowance window id: {0}")]
    DuplicateWindowId(String),
    #[error("alert thresholds must be between 0 and 100")]
    InvalidAlertThreshold,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn window(id: &str, remaining: Option<f64>, active: bool) -> AllowanceWindow {
        AllowanceWindow {
            id: id.into(),
            label: id.into(),
            duration_minutes: None,
            used_percent: None,
            remaining_percent: remaining,
            reset_at: None,
            kind: WindowKind::Rolling,
            is_active: active,
        }
    }

    #[test]
    fn clamps_and_complements_percentages() {
        let value = window("a", Some(140.0), true).normalize();
        assert_eq!(value.remaining_percent, Some(100.0));
        assert_eq!(value.used_percent, Some(0.0));
    }
    #[test]
    fn headline_is_most_constrained_active_window() {
        let values = vec![
            window("inactive", Some(1.0), false),
            window("week", Some(40.0), true),
            window("roll", Some(10.0), true),
        ];
        assert_eq!(select_headline(&values).unwrap().id, "roll");
    }
    #[test]
    fn headline_handles_unknown_remaining_deterministically() {
        let values = vec![window("b", None, true), window("a", None, true)];
        assert_eq!(select_headline(&values).unwrap().id, "a");
    }
}
