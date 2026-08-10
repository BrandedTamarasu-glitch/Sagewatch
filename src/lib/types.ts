export type ProviderId = "claude" | "codex";
export type ProviderSource =
  | "claude_statusline"
  | "codex_app_server"
  | "codex_rollout_cache"
  | "manual";
export type SourceConfidence =
  | "documented_local"
  | "experimental_local"
  | "sensitive_local_cache"
  | "manual";
export type Freshness = "live" | "recent" | "stale" | "unknown";
export type ProviderHealth =
  | "healthy"
  | "signed_out"
  | "unavailable"
  | "unsupported"
  | "source_changed"
  | "error";
export type WindowKind = "rolling" | "weekly" | "model_scoped" | "credits" | "unknown";

export interface AllowanceWindow {
  id: string;
  label: string;
  duration_minutes: number | null;
  used_percent: number | null;
  remaining_percent: number | null;
  reset_at: string | null;
  kind: WindowKind;
  is_active: boolean;
}

export interface ProviderStatus {
  schema_version: 1;
  provider: ProviderId;
  plan: string;
  observed_at: string;
  last_successful_at: string | null;
  source: ProviderSource;
  source_confidence: SourceConfidence;
  freshness: Freshness;
  health: ProviderHealth;
  headline_window_id: string | null;
  windows: AllowanceWindow[];
}

export type TimeFormat = "local_12_hour" | "local_24_hour";

export interface Preferences {
  refresh_interval_seconds: number;
  time_format: TimeFormat;
  alert_thresholds: number[];
  alerts_enabled: boolean;
  codex_rollout_fallback_enabled: boolean;
}

export interface ProviderDiagnostic {
  provider: ProviderId;
  health: ProviderHealth;
  summary: string;
  retryable: boolean;
}

export type Diagnostics = Partial<Record<ProviderId, ProviderDiagnostic | null>>;

export interface ProviderRuntimeState {
  status: ProviderStatus | null;
  diagnostics: ProviderDiagnostic | null;
  consecutive_failures: number;
  next_retry_at: string | null;
  refreshing: boolean;
}

export type RuntimeStatus = Partial<Record<ProviderId, ProviderRuntimeState>>;

export interface StatusSnapshot {
  providers: RuntimeStatus;
  preferences: Preferences;
}
