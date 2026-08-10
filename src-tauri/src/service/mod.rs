use crate::{
    domain::{Freshness, Preferences, Provider, ProviderStatus},
    providers::{
        claude::ClaudeAdapter, codex::CodexAdapter, AdapterError, AdapterErrorKind,
        ProviderAdapter, ProviderDiagnostics,
    },
    store::{JsonStore, StoreError},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProviderRuntimeState {
    pub status: Option<ProviderStatus>,
    pub diagnostics: Option<ProviderDiagnostics>,
    pub consecutive_failures: u32,
    pub next_retry_at: Option<chrono::DateTime<Utc>>,
    pub refreshing: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub providers: BTreeMap<Provider, ProviderRuntimeState>,
    pub preferences: Preferences,
}
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("provider adapter is not registered")]
    UnknownProvider,
    #[error("provider retry is paused until {0}")]
    RetryLater(chrono::DateTime<Utc>),
    #[error("provider refresh failed: {0}")]
    Adapter(#[from] AdapterError),
    #[error("state persistence failed: {0}")]
    Store(#[from] StoreError),
}

pub trait BackoffPolicy: Send + Sync {
    fn delay(&self, failures: u32) -> Duration;
}
#[derive(Clone, Debug)]
pub struct ExponentialBackoff {
    pub base: Duration,
    pub maximum: Duration,
}
impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self {
            base: Duration::from_secs(5),
            maximum: Duration::from_secs(15 * 60),
        }
    }
}
impl BackoffPolicy for ExponentialBackoff {
    fn delay(&self, failures: u32) -> Duration {
        let factor = 1u32
            .checked_shl(failures.saturating_sub(1).min(16))
            .unwrap_or(u32::MAX);
        self.base.saturating_mul(factor).min(self.maximum)
    }
}

pub struct RefreshService {
    adapters: HashMap<Provider, Arc<dyn ProviderAdapter>>,
    states: RwLock<BTreeMap<Provider, ProviderRuntimeState>>,
    preferences: RwLock<Preferences>,
    store: JsonStore,
    timeout: Duration,
    backoff: Arc<dyn BackoffPolicy>,
}

impl RefreshService {
    pub fn bootstrap(app_data_dir: impl Into<PathBuf>) -> Result<Self, ServiceError> {
        let app_data_dir = app_data_dir.into();
        let claude_path = std::env::var_os("SAGEWATCH_CLAUDE_SNAPSHOT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let data_home = std::env::var_os("XDG_DATA_HOME")
                    .map(PathBuf::from)
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(|home| PathBuf::from(home).join(".local/share"))
                    })
                    .unwrap_or_else(|| app_data_dir.clone());
                data_home.join("sagewatch/ingest/claude-statusline.json")
            });
        let adapters: Vec<Arc<dyn ProviderAdapter>> = vec![
            Arc::new(ClaudeAdapter::new(
                claude_path,
                Duration::from_secs(10 * 60),
            )),
            Arc::new(CodexAdapter::default()),
        ];
        Self::new(adapters, JsonStore::new(app_data_dir)?)
    }
    pub fn new(
        adapters: impl IntoIterator<Item = Arc<dyn ProviderAdapter>>,
        store: JsonStore,
    ) -> Result<Self, ServiceError> {
        Self::with_policy(
            adapters,
            store,
            Duration::from_secs(23),
            Arc::new(ExponentialBackoff::default()),
        )
    }
    pub fn with_policy(
        adapters: impl IntoIterator<Item = Arc<dyn ProviderAdapter>>,
        store: JsonStore,
        timeout: Duration,
        backoff: Arc<dyn BackoffPolicy>,
    ) -> Result<Self, ServiceError> {
        let preferences = store.load_preferences()?.normalize().unwrap_or_default();
        let adapters: HashMap<_, _> = adapters
            .into_iter()
            .map(|adapter| {
                adapter.set_preferences(&preferences);
                (adapter.capabilities().provider, adapter)
            })
            .collect();
        let saved = store.load_snapshots()?;
        let states = saved
            .into_iter()
            .map(|(provider, status)| {
                (
                    provider,
                    ProviderRuntimeState {
                        status: Some(status),
                        ..Default::default()
                    },
                )
            })
            .collect();
        Ok(Self {
            adapters,
            states: RwLock::new(states),
            preferences: RwLock::new(preferences),
            store,
            timeout,
            backoff,
        })
    }
    pub async fn get_status(&self) -> AppSnapshot {
        self.snapshot().await
    }
    pub async fn snapshot(&self) -> AppSnapshot {
        AppSnapshot {
            providers: self.states.read().await.clone(),
            preferences: self.preferences.read().await.clone(),
        }
    }
    pub async fn get_diagnostics(&self) -> BTreeMap<Provider, Option<ProviderDiagnostics>> {
        self.states
            .read()
            .await
            .iter()
            .map(|(provider, state)| (*provider, state.diagnostics.clone()))
            .collect()
    }
    pub async fn preferences(&self) -> Preferences {
        self.preferences.read().await.clone()
    }
    pub async fn set_preferences(
        &self,
        preferences: Preferences,
    ) -> Result<Preferences, ServiceError> {
        let preferences = preferences.normalize().map_err(|error| AdapterError {
            kind: AdapterErrorKind::InvalidData,
            message: error.to_string(),
            retryable: false,
        })?;
        self.store.save_preferences(&preferences)?;
        for adapter in self.adapters.values() {
            adapter.set_preferences(&preferences);
        }
        *self.preferences.write().await = preferences.clone();
        Ok(preferences)
    }
    pub async fn refresh_provider(
        &self,
        provider: Provider,
    ) -> Result<ProviderStatus, ServiceError> {
        let adapter = self
            .adapters
            .get(&provider)
            .cloned()
            .ok_or(ServiceError::UnknownProvider)?;
        {
            let mut states = self.states.write().await;
            let state = states.entry(provider).or_default();
            if let Some(next_retry_at) = state.next_retry_at {
                if next_retry_at > Utc::now() {
                    return Err(ServiceError::RetryLater(next_retry_at));
                }
            }
            state.refreshing = true;
        }
        let result = match tokio::time::timeout(self.timeout, adapter.refresh()).await {
            Ok(result) => result,
            Err(_) => Err(AdapterError {
                kind: AdapterErrorKind::Timeout,
                message: "provider refresh timed out".into(),
                retryable: true,
            }),
        }
        .and_then(|status| {
            if status.provider != provider {
                return Err(AdapterError {
                    kind: AdapterErrorKind::InvalidData,
                    message: "provider identity mismatch".into(),
                    retryable: false,
                });
            }
            status.normalize().map_err(|error| AdapterError {
                kind: AdapterErrorKind::InvalidData,
                message: error.to_string(),
                retryable: false,
            })
        });
        match result {
            Ok(status) => {
                let mut states = self.states.write().await;
                states.insert(
                    provider,
                    ProviderRuntimeState {
                        status: Some(status.clone()),
                        diagnostics: None,
                        consecutive_failures: 0,
                        next_retry_at: None,
                        refreshing: false,
                    },
                );
                let snapshots = states
                    .iter()
                    .filter_map(|(provider, state)| {
                        state.status.clone().map(|status| (*provider, status))
                    })
                    .collect();
                self.store.save_snapshots(&snapshots)?;
                Ok(status)
            }
            Err(error) => {
                let diagnostic = adapter.diagnose(&error);
                let mut states = self.states.write().await;
                let state = states.entry(provider).or_default();
                state.refreshing = false;
                state.consecutive_failures = state.consecutive_failures.saturating_add(1);
                state.next_retry_at = error.retryable.then(|| {
                    Utc::now()
                        + chrono::Duration::from_std(self.backoff.delay(state.consecutive_failures))
                            .unwrap_or_default()
                });
                state.diagnostics = Some(diagnostic);
                if let Some(status) = state.status.as_mut() {
                    status.health = crate::providers::health_for_error(error.kind);
                    status.freshness = Freshness::Stale;
                }
                Err(error.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{Health, Source, SourceConfidence},
        providers::FakeProviderAdapter,
    };
    fn healthy(provider: Provider) -> ProviderStatus {
        ProviderStatus {
            schema_version: 1,
            provider,
            plan: "test".into(),
            observed_at: Utc::now(),
            last_successful_at: None,
            source: Source::Manual,
            source_confidence: SourceConfidence::Manual,
            freshness: Freshness::Live,
            health: Health::Healthy,
            headline_window_id: None,
            windows: vec![],
        }
    }
    #[tokio::test]
    async fn provider_failure_does_not_erase_other_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let claude = Arc::new(FakeProviderAdapter::new(
            Provider::Claude,
            Ok(healthy(Provider::Claude)),
        ));
        let codex = Arc::new(FakeProviderAdapter::new(
            Provider::Codex,
            Ok(healthy(Provider::Codex)),
        ));
        let service = RefreshService::new(
            vec![
                claude.clone() as Arc<dyn ProviderAdapter>,
                codex.clone() as Arc<dyn ProviderAdapter>,
            ],
            JsonStore::new(dir.path()).unwrap(),
        )
        .unwrap();
        service.refresh_provider(Provider::Claude).await.unwrap();
        service.refresh_provider(Provider::Codex).await.unwrap();
        codex.set_result(Err(AdapterError {
            kind: AdapterErrorKind::Unavailable,
            message: "offline".into(),
            retryable: true,
        }));
        assert!(service.refresh_provider(Provider::Codex).await.is_err());
        let states = service.get_status().await.providers;
        assert_eq!(
            states[&Provider::Claude].status.as_ref().unwrap().health,
            Health::Healthy
        );
        assert!(states[&Provider::Codex].status.is_some());
    }

    #[tokio::test]
    async fn snapshot_restores_saved_preferences() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonStore::new(dir.path()).unwrap();
        let preferences = Preferences {
            refresh_interval_seconds: 900,
            alerts_enabled: true,
            ..Preferences::default()
        };
        store.save_preferences(&preferences).unwrap();
        let service =
            RefreshService::new(std::iter::empty::<Arc<dyn ProviderAdapter>>(), store).unwrap();
        assert_eq!(service.snapshot().await.preferences, preferences);
    }

    #[tokio::test]
    async fn retry_deadline_gates_repeated_refreshes() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = Arc::new(FakeProviderAdapter::new(
            Provider::Codex,
            Err(AdapterError {
                kind: AdapterErrorKind::Unavailable,
                message: "offline".into(),
                retryable: true,
            }),
        ));
        let service = RefreshService::new(
            vec![adapter as Arc<dyn ProviderAdapter>],
            JsonStore::new(dir.path()).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            service.refresh_provider(Provider::Codex).await,
            Err(ServiceError::Adapter(_))
        ));
        assert!(matches!(
            service.refresh_provider(Provider::Codex).await,
            Err(ServiceError::RetryLater(_))
        ));
        assert_eq!(
            service.snapshot().await.providers[&Provider::Codex].consecutive_failures,
            1
        );
    }
}
