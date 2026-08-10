# Sagewatch binding implementation brief

Status: binding for implementation

## Authority and resolved decisions

- `.forgeflow/Sagewatch/current-research.md` is authoritative where planning artifacts differ.
- Use Tauri 2 and Vite unless live provider probes prove the platform cannot access the required local surfaces.
- Rust owns provider access, normalization, cache, refresh scheduling, diagnostics, and error mapping.
- The frontend is presentation-only and never receives raw provider payloads.
- Use restrictive atomic JSON files for normalized snapshots and non-secret preferences in v1. Defer SQLite until history or trends justify it.
- Do not build the Codex rollout JSONL fallback until the primary app-server adapter works and is contract-tested. The fallback remains opt-in and disabled by default.

## Locked v1 scope

In scope:

- One Claude Teams seat and one Codex subscription
- Remaining allowance, reset time, freshness, health, and source confidence
- Compact widget plus expanded details, diagnostics, and settings
- Local cache, manual refresh, and optional rate-limited alerts

Out of scope:

- Credential, cookie, or token collection
- Raw provider-response persistence
- Browser scraping or direct hidden provider endpoints
- Cloud sync, multi-account support, admin analytics, and historical charts

## Implementation waves and ownership

### Wave 1 — Scaffold and frozen contracts

Smith owns the scaffold and IPC surface, Warden owns the Rust domain contract, Lumen owns the UI/accessibility contract, and Atlas tracks the gate.

- Create the Tauri/Vite app.
- Register `get_status`, `refresh_provider`, `set_preferences`, and `get_diagnostics` only.
- Freeze domain/DTO schemas, fixtures, and error taxonomy.
- Build compact and expanded views from fixtures.

### Wave 2 — Store, scheduler, and fake adapters

Warden owns backend systems and Smith wires commands/UI.

- Atomic snapshot/preferences storage beneath the Tauri app-data directory with `0700` directories and `0600` files.
- Refresh orchestration, timeouts, jittered backoff, independent provider states, staleness rules, and diagnostics.
- Fake Claude and Codex adapters for integration and UI tests.

### Wave 3 — Codex primary adapter

Warden owns the provider client and Smith owns integration wiring.

- Version-gated local app-server client using fixed arguments, bounded execution, and no network listener.
- Normalize `account/rateLimits/read` into the shared contract.
- Fail closed on protocol/schema drift.
- Keep rollout fallback disabled and unimplemented until the primary adapter passes.

### Wave 4 — Claude primary adapter

Warden owns ingestion, Smith owns wiring, and Lumen validates stale-state UX.

- Ingest documented Claude Code status-line `rate_limits` input from the already authenticated local client.
- Retain the last normalized snapshot and mark it stale when session-fed observations stop.
- Provide manual `/usage` or Settings > Usage guidance only; no automated browser/API fallback.

### Wave 5 — Product polish and optional alerts

Lumen owns UX and Smith owns implementation integration.

- Complete the compact trust-first two-card layout and expanded diagnostics.
- Add preferences for refresh cadence, time format, and thresholds.
- Add optional rate-limited local alerts with accessible behavior.

### Wave 6 — Verification and release readiness

Compass validates and Atlas enforces the release gates.

- Run domain, store, adapter, integration, accessibility, and packaging checks.
- Publish setup, privacy, troubleshooting, and source-limitations documentation.

## Exact module boundaries

Rust:

- `src-tauri/src/domain/`: canonical types, validation, error taxonomy
- `src-tauri/src/providers/`: adapter trait, `claude.rs`, `codex.rs`, and later `codex_rollout.rs`
- `src-tauri/src/store/`: snapshot/preference stores and atomic filesystem helpers
- `src-tauri/src/service/`: refresh orchestration, scheduler, diagnostics, and in-memory state
- `src-tauri/src/commands/`: Tauri IPC handlers only
- `src-tauri/src/main.rs`: bootstrap and command registration only

Frontend:

- `src/lib/api.ts`: typed IPC wrappers only
- `src/lib/types.ts`: DTO mirrors only
- `src/components/`: compact cards, detail panels, status badges, diagnostics
- `src/views/`: widget shell and settings/details composition
- No provider parsing or business rules in the frontend

## Locked adapter interface

```text
probe() -> CapabilityReport
refresh() -> Result<ProviderStatus, AdapterError>
diagnose(error) -> ProviderDiagnostics
capabilities() -> ProviderCapabilities
```

## Locked domain contract

```text
ProviderStatus {
  schema_version: 1
  provider: "claude" | "codex"
  plan: string | "unknown"
  observed_at: ISO timestamp
  last_successful_at: ISO timestamp | null
  source: "claude_statusline" | "codex_app_server" | "codex_rollout_cache" | "manual"
  source_confidence: "documented_local" | "experimental_local" | "sensitive_local_cache" | "manual"
  freshness: "live" | "recent" | "stale" | "unknown"
  health: "healthy" | "signed_out" | "unavailable" | "unsupported" | "source_changed" | "error"
  headline_window_id: string | null
  windows: AllowanceWindow[]
}

AllowanceWindow {
  id: string
  label: string
  duration_minutes: number | null
  used_percent: number | null
  remaining_percent: number | null
  reset_at: ISO timestamp | null
  kind: "rolling" | "weekly" | "model_scoped" | "credits" | "unknown"
  is_active: boolean
}
```

Internal and persisted state do not duplicate top-level remaining percentage or reset time. UI DTOs derive those values from `headline_window_id` while preserving every window.

Preferences:

- `refresh_interval_seconds`
- `time_format`
- `alert_thresholds`
- `alerts_enabled`
- `codex_rollout_fallback_enabled`

## Source ingestion rules

- Claude: documented status-line `rate_limits` ingestion only, session-attached by design.
- Codex: local app-server `account/rateLimits/read`, gated to known compatible CLI/protocol versions, fixed arguments, bounded process, no network listener.
- Codex fallback: later opt-in streaming JSONL parser that extracts only rate-limit events, discards all other records, canonicalizes allowed paths, and rejects traversal/symlink escape.

## Acceptance gates

- Schemas and fixtures land before live adapter UI wiring.
- One provider failure never blocks the other or erases its last known snapshot.
- No raw response, transcript, token, cookie, or credential is logged, cached, or included in fixtures.
- Drift maps to `source_changed` or `unsupported`, never fabricated values.
- Compact view shows the most constrained active window; expanded view lists all windows.
- Freshness, health, source confidence, and absolute local reset time remain visible.
- Keyboard navigation, screen-reader labels, focus visibility, contrast, non-color cues, reduced motion, and 200% scaling are release gates.

## Required tests

- Domain normalization: clamping, headline selection, and reset handling
- Store: atomic writes, restrictive permissions, corrupt-file recovery
- Adapters: healthy, stale, signed-out, unavailable, exhausted, and drift fixtures
- Security: redaction, provider isolation, protocol mismatch, path traversal, symlink defense, and fallback non-retention
- Frontend: compact/detail rendering, stale/error semantics, keyboard flow, and accessible labels

## Live-probe gates

- Confirm the Codex app-server handshake, version gate, and bounded invocation in the user's normal local context.
- Confirm Claude status-line ingestion and that its payload satisfies the locked contract.
- Waves 1 and 2 may proceed before these probes complete. If a probe fails, revise only that adapter strategy rather than reopening the shared contract or UI foundation.
