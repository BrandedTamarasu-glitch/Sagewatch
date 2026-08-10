import type { Diagnostics, Preferences, ProviderStatus } from "../lib/types";

export const fixturePreferences: Preferences = {
  refresh_interval_seconds: 300,
  time_format: "local_12_hour",
  alert_thresholds: [20, 10],
  alerts_enabled: false,
  codex_rollout_fallback_enabled: false,
};

export const fixtureProviders: ProviderStatus[] = [
  {
    schema_version: 1,
    provider: "claude",
    plan: "Teams",
    observed_at: "2026-08-07T18:18:00Z",
    last_successful_at: "2026-08-07T18:18:00Z",
    source: "claude_statusline",
    source_confidence: "documented_local",
    freshness: "live",
    health: "healthy",
    headline_window_id: "claude-session",
    windows: [
      { id: "claude-session", label: "Current session", duration_minutes: 300, used_percent: 68, remaining_percent: 32, reset_at: "2026-08-07T20:00:00Z", kind: "rolling", is_active: true },
      { id: "claude-weekly", label: "Weekly", duration_minutes: 10080, used_percent: 41, remaining_percent: 59, reset_at: "2026-08-11T07:00:00Z", kind: "weekly", is_active: true },
    ],
  },
  {
    schema_version: 1,
    provider: "codex",
    plan: "Plus",
    observed_at: "2026-08-07T18:14:00Z",
    last_successful_at: "2026-08-07T18:14:00Z",
    source: "codex_app_server",
    source_confidence: "experimental_local",
    freshness: "recent",
    health: "healthy",
    headline_window_id: "codex-weekly",
    windows: [
      { id: "codex-five-hour", label: "5-hour limit", duration_minutes: 300, used_percent: 12, remaining_percent: 88, reset_at: "2026-08-07T22:15:00Z", kind: "rolling", is_active: true },
      { id: "codex-weekly", label: "Weekly limit", duration_minutes: 10080, used_percent: 77, remaining_percent: 23, reset_at: "2026-08-10T16:00:00Z", kind: "weekly", is_active: true },
    ],
  },
];

export const unavailableProviders: ProviderStatus[] = (["claude", "codex"] as const).map((provider) => ({
  schema_version: 1,
  provider,
  plan: "unknown",
  observed_at: new Date(0).toISOString(),
  last_successful_at: null,
  source: "manual",
  source_confidence: "manual",
  freshness: "unknown",
  health: "unavailable",
  headline_window_id: null,
  windows: [],
}));

export const fixtureDiagnostics: Diagnostics = {
  claude: { provider: "claude", health: "healthy", summary: "Local source is responding", retryable: false },
  codex: { provider: "codex", health: "healthy", summary: "Local source is responding", retryable: false },
};
