use super::*;
use crate::domain::{
    AllowanceWindow, Freshness, Health, Provider, ProviderStatus, Source, SourceConfidence,
    WindowKind,
};
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

const MAX_LINE_BYTES: usize = 64 * 1024;
const MAX_TOTAL_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug)]
pub struct CodexAdapter {
    executable: PathBuf,
    timeout: Duration,
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self {
            executable: "codex".into(),
            timeout: Duration::from_secs(10),
        }
    }
}
impl CodexAdapter {
    pub fn new(executable: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            executable: executable.into(),
            timeout,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Limits {
    primary: Option<Window>,
    secondary: Option<Window>,
    #[serde(default)]
    plan_type: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Window {
    used_percent: f64,
    window_duration_mins: Option<u32>,
    resets_at: Option<i64>,
}

impl CodexAdapter {
    async fn version(&self) -> Result<String, AdapterError> {
        let output = tokio::time::timeout(
            self.timeout,
            Command::new(&self.executable)
                .arg("--version")
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .output(),
        )
        .await
        .map_err(|_| err(AdapterErrorKind::Timeout, true))?
        .map_err(|_| err(AdapterErrorKind::Unavailable, true))?;
        if !output.status.success() || output.stdout.len() > 1024 {
            return Err(err(AdapterErrorKind::Unavailable, true));
        }
        String::from_utf8(output.stdout)
            .map(|s| s.trim().to_owned())
            .map_err(|_| err(AdapterErrorKind::SourceChanged, false))
    }
    fn compatible(version: &str) -> bool {
        version
            .split_whitespace()
            .any(|part| part.starts_with("0.147."))
    }
    async fn request(&self) -> Result<Value, AdapterError> {
        let mut child = Command::new(&self.executable)
            .args(["app-server", "--stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| err(AdapterErrorKind::Unavailable, true))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| err(AdapterErrorKind::Internal, false))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| err(AdapterErrorKind::Internal, false))?;
        let mut lines = BufReader::new(stdout).lines();
        stdin.write_all(format!("{}\n", json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"sagewatch","title":"Sagewatch","version":env!("CARGO_PKG_VERSION")}}})).as_bytes()).await.map_err(|_| err(AdapterErrorKind::Unavailable, true))?;
        let _ = read_response(&mut lines, 1, self.timeout).await?;
        stdin
            .write_all(
                format!(
                    "{}\n{}\n",
                    json!({"method":"initialized","params":{}}),
                    json!({"id":2,"method":"account/rateLimits/read","params":{}})
                )
                .as_bytes(),
            )
            .await
            .map_err(|_| err(AdapterErrorKind::Unavailable, true))?;
        let response = read_response(&mut lines, 2, self.timeout).await?;
        let _ = child.kill().await;
        response
            .get("result")
            .cloned()
            .ok_or_else(|| err(AdapterErrorKind::SourceChanged, false))
    }
    fn normalize(value: Value) -> Result<ProviderStatus, AdapterError> {
        let limits_value = value
            .get("rateLimits")
            .cloned()
            .ok_or_else(|| err(AdapterErrorKind::SourceChanged, false))?;
        let limits: Limits = serde_json::from_value(limits_value)
            .map_err(|_| err(AdapterErrorKind::SourceChanged, false))?;
        let mut windows = Vec::new();
        if let Some(window) = limits.primary {
            windows.push(to_window(
                "primary",
                "Primary",
                WindowKind::Rolling,
                window,
            )?);
        }
        if let Some(window) = limits.secondary {
            windows.push(to_window(
                "secondary",
                "Secondary",
                WindowKind::Unknown,
                window,
            )?);
        }
        if windows.is_empty() {
            return Err(err(AdapterErrorKind::SourceChanged, false));
        }
        if let Some(by_id) = value.get("rateLimitsByLimitId").and_then(Value::as_object) {
            for (limit_id, raw) in by_id {
                if limit_id.is_empty() || limit_id.len() > 128 {
                    return Err(err(AdapterErrorKind::SourceChanged, false));
                }
                let grouped: Limits = serde_json::from_value(raw.clone())
                    .map_err(|_| err(AdapterErrorKind::SourceChanged, false))?;
                if let Some(window) = grouped.primary {
                    windows.push(to_window(
                        &format!("{limit_id}:primary"),
                        limit_id,
                        WindowKind::Unknown,
                        window,
                    )?);
                }
                if let Some(window) = grouped.secondary {
                    windows.push(to_window(
                        &format!("{limit_id}:secondary"),
                        limit_id,
                        WindowKind::Unknown,
                        window,
                    )?);
                }
            }
        }
        Ok(ProviderStatus {
            schema_version: 1,
            provider: Provider::Codex,
            plan: limits.plan_type.unwrap_or_else(|| "unknown".into()),
            observed_at: Utc::now(),
            last_successful_at: None,
            source: Source::CodexAppServer,
            source_confidence: SourceConfidence::ExperimentalLocal,
            freshness: Freshness::Live,
            health: Health::Healthy,
            headline_window_id: None,
            windows,
        })
    }
}

async fn read_response<R: tokio::io::AsyncBufRead + Unpin>(
    lines: &mut tokio::io::Lines<R>,
    id: i64,
    timeout: Duration,
) -> Result<Value, AdapterError> {
    let future = async {
        let mut total = 0;
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|_| err(AdapterErrorKind::Unavailable, true))?
        {
            total += line.len();
            if line.len() > MAX_LINE_BYTES || total > MAX_TOTAL_BYTES {
                return Err(err(AdapterErrorKind::SourceChanged, false));
            }
            let value: Value = serde_json::from_str(&line)
                .map_err(|_| err(AdapterErrorKind::SourceChanged, false))?;
            if value.get("id").and_then(Value::as_i64) == Some(id) {
                if value.get("error").is_some() {
                    return Err(err(AdapterErrorKind::SignedOut, false));
                }
                return Ok(value);
            }
        }
        Err(err(AdapterErrorKind::Unavailable, true))
    };
    tokio::time::timeout(timeout, future)
        .await
        .map_err(|_| err(AdapterErrorKind::Timeout, true))?
}
fn to_window(
    id: &str,
    label: &str,
    kind: WindowKind,
    window: Window,
) -> Result<AllowanceWindow, AdapterError> {
    if !window.used_percent.is_finite() {
        return Err(err(AdapterErrorKind::SourceChanged, false));
    }
    let reset_at = window
        .resets_at
        .map(|seconds| {
            Utc.timestamp_opt(seconds, 0)
                .single()
                .ok_or_else(|| err(AdapterErrorKind::SourceChanged, false))
        })
        .transpose()?;
    Ok(AllowanceWindow {
        id: id.into(),
        label: label.into(),
        duration_minutes: window.window_duration_mins,
        used_percent: Some(window.used_percent),
        remaining_percent: None,
        reset_at,
        kind,
        is_active: true,
    })
}
fn err(kind: AdapterErrorKind, retryable: bool) -> AdapterError {
    AdapterError {
        kind,
        message: safe_summary(kind).into(),
        retryable,
    }
}

#[async_trait]
impl ProviderAdapter for CodexAdapter {
    async fn probe(&self) -> CapabilityReport {
        match self.version().await {
            Ok(v) => CapabilityReport {
                available: true,
                compatible: Self::compatible(&v),
                detail: if Self::compatible(&v) {
                    "compatible Codex CLI".into()
                } else {
                    "unsupported Codex CLI version".into()
                },
            },
            Err(_) => CapabilityReport {
                available: false,
                compatible: false,
                detail: "Codex CLI unavailable".into(),
            },
        }
    }
    async fn refresh(&self) -> Result<ProviderStatus, AdapterError> {
        let version = self.version().await?;
        if !Self::compatible(&version) {
            return Err(err(AdapterErrorKind::Unsupported, false));
        }
        Self::normalize(self.request().await?)
    }
    fn diagnose(&self, error: &AdapterError) -> ProviderDiagnostics {
        ProviderDiagnostics {
            provider: Provider::Codex,
            health: health_for_error(error.kind),
            summary: safe_summary(error.kind).into(),
            retryable: error.retryable,
        }
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            provider: Provider::Codex,
            supports_refresh: true,
            session_attached: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_primary_secondary() {
        let status=CodexAdapter::normalize(json!({"rateLimits":{"primary":{"usedPercent":25.0,"windowDurationMins":300,"resetsAt":1893456000},"secondary":{"usedPercent":90.0,"windowDurationMins":10080,"resetsAt":1893456000},"planType":"plus"}})).unwrap().normalize().unwrap();
        assert_eq!(status.windows.len(), 2);
        assert_eq!(status.headline_window_id.as_deref(), Some("secondary"));
        assert_eq!(status.windows[0].remaining_percent, Some(75.0));
    }
    #[test]
    fn rejects_drift() {
        assert!(CodexAdapter::normalize(json!({"unexpected":true})).is_err());
    }

    #[tokio::test]
    #[ignore = "requires the user's locally authenticated Codex CLI"]
    async fn live_authenticated_app_server_handshake() {
        let status = CodexAdapter::default()
            .refresh()
            .await
            .expect("authenticated Codex app-server rate-limit read should succeed")
            .normalize()
            .expect("live Codex rate limits should satisfy the domain contract");
        assert_eq!(status.provider, Provider::Codex);
        assert!(!status.windows.is_empty());
    }
}
