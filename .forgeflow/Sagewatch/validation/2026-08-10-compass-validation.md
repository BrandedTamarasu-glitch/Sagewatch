# Sagewatch validation report

Date: 2026-08-10
Validator: Compass
Scope: `/home/corye/openai-cli/Sagewatch` against `.forgeflow/Sagewatch/current-brief.md` and `current-research.md`

## Command results

- `npm run build`: pass
- `npm run test:frontend`: pass
- `npm run test:bridge`: pass
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`: pass
- `cargo test --manifest-path src-tauri/Cargo.toml --offline`: pass
  - 9 passed
  - 1 ignored: `providers::codex::tests::live_authenticated_app_server_handshake`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --offline --all-targets -- -D warnings`: pass

## Findings

### 1. Persisted settings are not restored into the UI

Severity: high

The backend saves preferences, but the frontend always boots from fixtures and never asks the backend for the saved values. `SagewatchApp` initializes `preferences` from `fixturePreferences` and `start()` only calls `get_status` ([src/App.ts](../../src/App.ts)). The IPC layer exposes `get_status`, `refresh_provider`, `set_preferences`, and `get_diagnostics`, but there is no way for the UI to load persisted preferences on startup ([src/lib/api.ts](../../src/lib/api.ts), [src-tauri/src/commands/preferences.rs](../../src-tauri/src/commands/preferences.rs), [src-tauri/src/service/mod.rs](../../src-tauri/src/service/mod.rs)). User-visible impact: refresh cadence, time format, alert toggles, and rollout-fallback toggle revert to fixture defaults after relaunch, so the v1 preferences requirement is not met end to end.

### 2. Refresh cadence is stored but never drives any scheduling

Severity: high

The brief requires refresh orchestration and a refresh-cadence preference. The service records `refresh_interval_seconds` and computes `next_retry_at`, but there is no scheduler task, timer loop, or startup refresh orchestration that consumes those values ([src-tauri/src/service/mod.rs](../../src-tauri/src/service/mod.rs)). The frontend only triggers refreshes from the manual button path ([src/App.ts](../../src/App.ts)). User-visible impact: changing the refresh interval in Settings has no behavior change; the app remains manual-refresh only.

### 3. Optional alerts are not implemented beyond storage and copy

Severity: medium

The brief locks optional rate-limited local alerts into scope, but the current implementation only renders and persists `alerts_enabled` and `alert_thresholds` ([src/components/render.ts](../../src/components/render.ts), [src/App.ts](../../src/App.ts), [src-tauri/src/domain/mod.rs](../../src-tauri/src/domain/mod.rs)). No alert emission, throttling, or accessible announcement path exists anywhere in `src/` or `src-tauri/`. User-visible impact: the settings UI advertises alerts that do nothing.

## Security, privacy, and isolation review

- Pass: Claude ingestion is sanitized and secrets are stripped before persistence by `scripts/claude-statusline-bridge.mjs`; bridge tests cover redaction and private-file permissions.
- Pass: store writes are atomic and permission-restricted; Rust tests cover corrupt-file recovery and Unix `0700`/`0600`.
- Pass: Claude adapter rejects symlinks, non-files, oversized snapshots, and invalid/drifted payloads before normalization.
- Pass: Codex adapter uses fixed `codex app-server --stdio` arguments, bounded IO limits, a version gate, and no rollout-cache fallback implementation.
- Pass: one provider failure does not erase the other provider snapshot; Rust service test covers this.
- Note: live Codex evidence was not visible in this thread. The only live check present in repo is the ignored test `providers::codex::tests::live_authenticated_app_server_handshake`, which was not executed in this validation run.

## Accessibility acceptance review

- Pass by inspection: visible focus styles, non-color status cues, reduced-motion handling, dialog close labelling, refresh button labelling, and absolute reset-time rendering are present.
- Partial evidence only: automated tests cover some accessible strings and dialog labelling, but there is no executable coverage for keyboard flow, 200% scaling, or screen-reader interaction gates called out in the brief.

## Gate summary

- `build`: pass
- `frontend tests`: pass
- `bridge tests`: pass
- `rust fmt/test/clippy`: pass
- `security/privacy boundaries`: pass with no blocking defect found
- `provider isolation`: pass
- `schema behavior`: pass for covered normalization/store cases
- `accessibility release gate evidence`: partial
- `overall release gate`: fail

Release is blocked by findings 1 and 2. Finding 3 is also out of scope relative to the claimed completed implementation and should be resolved before calling the product release-ready.
