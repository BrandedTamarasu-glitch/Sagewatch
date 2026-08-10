# Sagewatch revalidation report

Date: 2026-08-10
Validator: Compass
Scope: remediation revalidation against `.forgeflow/Sagewatch/current-brief.md` and `current-research.md`

## Prior findings status

### 1. Persisted settings hydration

Resolved.

- Backend `get_status` now returns an `AppSnapshot` containing both provider runtime state and persisted preferences ([src-tauri/src/service/mod.rs](../../src-tauri/src/service/mod.rs), [src-tauri/src/commands/status.rs](../../src-tauri/src/commands/status.rs)).
- Frontend startup now hydrates `this.preferences` from the snapshot instead of staying on fixture defaults ([src/App.ts](../../src/App.ts)).
- Rust coverage includes `snapshot_restores_saved_preferences` to verify restored preferences survive service bootstrap ([src-tauri/src/service/mod.rs](../../src-tauri/src/service/mod.rs)).

### 2. Refresh cadence and retry gating

Resolved.

- The frontend now installs a lifecycle-managed interval in `start()`, rebuilds it when settings change, pauses it when the document is hidden, and tears it down on `pagehide` ([src/App.ts](../../src/App.ts)).
- The interval refreshes Claude and Codex independently via separate `refresh(provider)` calls under `Promise.allSettled`, so one provider failure does not block the other ([src/App.ts](../../src/App.ts)).
- The backend now enforces a retry gate with `ServiceError::RetryLater` when `next_retry_at` has not elapsed ([src-tauri/src/service/mod.rs](../../src-tauri/src/service/mod.rs)).
- Rust coverage includes `retry_deadline_gates_repeated_refreshes` to verify repeated refreshes are blocked during backoff ([src-tauri/src/service/mod.rs](../../src-tauri/src/service/mod.rs)).

### 3. Alerts

Resolved for the requested behavior.

- Alerts only trigger on downward threshold crossings or exhaustion because `ThresholdAlertTracker` compares previous remaining percentage to current remaining percentage and checks `previous > threshold && current <= threshold` ([src/lib/alerts.ts](../../src/lib/alerts.ts)).
- Alerts are deduped until recovery because each `(provider, window, threshold)` key is added to `announced` and only rearmed once the remaining percentage rises back above the threshold ([src/lib/alerts.ts](../../src/lib/alerts.ts)).
- Alerts are dismissible in the UI through `data-dismiss-alert`, and dismissed alerts do not immediately respawn because the tracker still holds the announcement key until recovery ([src/App.ts](../../src/App.ts), [src/components/render.ts](../../src/components/render.ts)).
- Frontend coverage includes crossing/dedupe/rearm tests and exhaustion-dismissible rendering checks ([tests/frontend/render.test.mjs](../../tests/frontend/render.test.mjs)).

## Command results

- `npm run build`: pass
- `npm run test:frontend`: pass
- `npm run test:bridge`: pass
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`: pass
- `cargo test --manifest-path src-tauri/Cargo.toml --offline`: pass
  - 11 passed
  - 1 ignored in this run: `providers::codex::tests::live_authenticated_app_server_handshake`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --offline --all-targets -- -D warnings`: pass

## Live provider evidence

- Root reported that the live authenticated Codex handshake test was executed successfully before remediation: `1 passed`.
- This revalidation run kept the live Codex test in its ignored offline state, which is consistent with the local-authenticated requirement in the test itself.

## Security, privacy, and isolation review

- Pass: sanitized Claude bridge still strips unsupported fields and secrets before persistence.
- Pass: atomic local storage and restrictive file permissions remain covered by tests.
- Pass: provider isolation still holds; one provider failure does not erase the other provider snapshot.
- Pass: backend retry gating improves safety by reducing repeated failing refresh attempts against unstable or signed-out providers.
- Pass: no new credential, cookie, transcript, or raw response retention paths were introduced in the remediation.

## Accessibility evidence

Accessibility evidence is acceptable for this implementation pass.

- Keyboard flow: usage tabs, refresh buttons, details button, dialog close button, settings controls, and dismiss-alert controls are all native interactive elements; dialog open/close focus handling is explicit ([src/App.ts](../../src/App.ts)).
- Screen-reader labels: refresh, close-details, and dismiss-alert buttons have explicit accessible names; status and alert announcements use live regions ([src/App.ts](../../src/App.ts), [src/components/render.ts](../../src/components/render.ts)).
- Focus visibility: controls use a visible `:focus-visible` outline ([src/styles.css](../../src/styles.css)).
- Non-color cues: health/freshness/confidence badges include visible text plus symbol hooks, not color alone ([src/components/render.ts](../../src/components/render.ts), [src/styles.css](../../src/styles.css)).
- Reduced motion: explicit `prefers-reduced-motion` handling remains present ([src/styles.css](../../src/styles.css)).
- 200% scaling and small-window behavior: layout uses fluid widths, wrapped actions, and a single-column mobile fallback without fixed-height clipping ([src/styles.css](../../src/styles.css)).
- Automated accessibility-adjacent coverage: frontend tests cover trust text, dialog labels, visible status text, alert dismissal text, and absolute reset-time wording ([tests/frontend/render.test.mjs](../../tests/frontend/render.test.mjs)).

Residual note:

- I did not run a native screen reader or visual browser-based zoom session in this turn, so that evidence remains inspection-based rather than instrumented.

## Findings

No blocking findings in this revalidation pass.

## Gate summary

- `build`: pass
- `frontend tests`: pass
- `bridge tests`: pass
- `rust fmt/test/clippy`: pass
- `security/privacy boundaries`: pass
- `provider isolation`: pass
- `schema behavior`: pass
- `accessibility evidence`: pass with noted manual-evidence limitation
- `live Codex handshake evidence`: pass, based on root's authenticated run
- `overall release gate`: pass
