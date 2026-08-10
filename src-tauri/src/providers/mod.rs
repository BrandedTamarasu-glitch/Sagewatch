use crate::domain::{Health, Preferences, Provider, ProviderStatus};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub mod claude;
pub mod codex;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub provider: Provider,
    pub supports_refresh: bool,
    pub session_attached: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub available: bool,
    pub compatible: bool,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterErrorKind {
    SignedOut,
    Unavailable,
    Unsupported,
    SourceChanged,
    Timeout,
    InvalidData,
    Internal,
}

#[derive(Clone, Debug, Error, Eq, PartialEq, Serialize, Deserialize)]
#[error("{kind:?}: {message}")]
pub struct AdapterError {
    pub kind: AdapterErrorKind,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderDiagnostics {
    pub provider: Provider,
    pub health: Health,
    pub summary: String,
    pub retryable: bool,
}

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn set_preferences(&self, _preferences: &Preferences) {}
    async fn probe(&self) -> CapabilityReport;
    async fn refresh(&self) -> Result<ProviderStatus, AdapterError>;
    fn diagnose(&self, error: &AdapterError) -> ProviderDiagnostics;
    fn capabilities(&self) -> ProviderCapabilities;
}

#[derive(Clone)]
pub struct FakeProviderAdapter {
    capabilities: ProviderCapabilities,
    result: Arc<Mutex<Result<ProviderStatus, AdapterError>>>,
}

impl FakeProviderAdapter {
    pub fn new(provider: Provider, result: Result<ProviderStatus, AdapterError>) -> Self {
        Self {
            capabilities: ProviderCapabilities {
                provider,
                supports_refresh: true,
                session_attached: provider == Provider::Claude,
            },
            result: Arc::new(Mutex::new(result)),
        }
    }
    pub fn set_result(&self, result: Result<ProviderStatus, AdapterError>) {
        *self.result.lock().expect("fake provider mutex poisoned") = result;
    }
}

#[async_trait]
impl ProviderAdapter for FakeProviderAdapter {
    async fn probe(&self) -> CapabilityReport {
        CapabilityReport {
            available: true,
            compatible: true,
            detail: "safe fake provider".into(),
        }
    }
    async fn refresh(&self) -> Result<ProviderStatus, AdapterError> {
        self.result
            .lock()
            .expect("fake provider mutex poisoned")
            .clone()
    }
    fn diagnose(&self, error: &AdapterError) -> ProviderDiagnostics {
        ProviderDiagnostics {
            provider: self.capabilities.provider,
            health: health_for_error(error.kind),
            summary: safe_summary(error.kind).into(),
            retryable: error.retryable,
        }
    }
    fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities.clone()
    }
}

pub fn health_for_error(kind: AdapterErrorKind) -> Health {
    match kind {
        AdapterErrorKind::SignedOut => Health::SignedOut,
        AdapterErrorKind::Unavailable | AdapterErrorKind::Timeout => Health::Unavailable,
        AdapterErrorKind::Unsupported => Health::Unsupported,
        AdapterErrorKind::SourceChanged | AdapterErrorKind::InvalidData => Health::SourceChanged,
        AdapterErrorKind::Internal => Health::Error,
    }
}

pub fn safe_summary(kind: AdapterErrorKind) -> &'static str {
    match kind {
        AdapterErrorKind::SignedOut => "provider is signed out",
        AdapterErrorKind::Unavailable => "provider is unavailable",
        AdapterErrorKind::Unsupported => "provider version is unsupported",
        AdapterErrorKind::SourceChanged | AdapterErrorKind::InvalidData => {
            "provider source contract changed"
        }
        AdapterErrorKind::Timeout => "provider refresh timed out",
        AdapterErrorKind::Internal => "provider refresh failed",
    }
}
