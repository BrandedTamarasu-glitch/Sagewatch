# Sagewatch desktop integration revalidation

Date: 2026-08-10
Validator: Compass
Scope: final revalidation for tray close safety, verified autostart persistence, tray refresh event consumption, and desktop-integration docs

## Verdict

Pass.

The prior backend blockers remain resolved, and the frontend now consumes tray refresh events through the same reconciliation, alert, notification, and dedupe path as manual refresh. The increment still passes the brief because the integrated frontend/backend gates are green and the latest AppImage rerun is blocked by a documented environment issue with exact evidence rather than a product regression.

## Prior blocker status

### 1. Tray close safety

Resolved.

- Close-to-tray is now gated by a proven tray recovery path in [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L24).
- The app records tray interaction before enabling hide-on-close in [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L42).
- Until that proof exists for the current desktop session, closing the main window exits cleanly instead of hiding an unreachable process in [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L109).
- Regression coverage exists in [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L141).

This satisfies the brief's gate that close must not leave an unquittable background process behind.

### 2. Verified autostart persistence

Resolved.

- `set_autostart_enabled` now verifies the OS state before persisting it in [src-tauri/src/commands/desktop.rs](../../src-tauri/src/commands/desktop.rs#L58).
- The stored `start_at_login` preference is now set from the verified desktop state, not the requested state, in [src-tauri/src/commands/desktop.rs](../../src-tauri/src/commands/desktop.rs#L63).
- The command returns an error if the verified state differs from the requested state via `verified_autostart_result` in [src-tauri/src/commands/desktop.rs](../../src-tauri/src/commands/desktop.rs#L21).
- Generic preference writes still cannot mutate login state because [src-tauri/src/commands/preferences.rs](../../src-tauri/src/commands/preferences.rs#L4) preserves the service-owned `start_at_login` value.
- Regression coverage exists in [src-tauri/src/commands/desktop.rs](../../src-tauri/src/commands/desktop.rs#L121).

This satisfies the brief's requirement that autostart remain user-controlled, persisted, idempotent, and honest about failure.

### 3. Packaging and troubleshooting docs

Resolved.

- Release documentation now states that AppImage bundling is enabled and records the `NO_STRIP=1` workaround in [docs/release.md](../../docs/release.md#L16).
- Setup documentation now describes the enabled AppImage path and the rolling-release workaround in [docs/setup.md](../../docs/setup.md#L54).
- Troubleshooting now explains the close-to-tray safety behavior and the `.relr.dyn` packaging workaround in [docs/troubleshooting.md](../../docs/troubleshooting.md#L31).

### 4. Tray refresh event consumption and alert dedupe

Resolved.

- The frontend now subscribes to `sagewatch://status-updated` in [src/App.ts](../../src/App.ts#L37) and applies the snapshot through `applySnapshot`.
- Tray-emitted snapshots are reconciled through the shared provider/alert pipeline in [src/App.ts](../../src/App.ts#L137) and [src/lib/alerts.ts](../../src/lib/alerts.ts#L23), so tray refreshes and manual refreshes share the same threshold-crossing and dedupe semantics.
- Listener cleanup is explicit and safe in [src/App.ts](../../src/App.ts#L179), including the destroyed-before-listen-resolves case in [src/App.ts](../../src/App.ts#L43).
- Regression coverage now includes one-notification-only behavior and nonblocking failure/dedupe for tray snapshots in [tests/frontend/render.test.mjs](../../tests/frontend/render.test.mjs#L91).

## Command results

- `npm run test:frontend`: pass
- `npm run test:bridge`: pass
- `npm run build`: pass
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`: pass
- `cargo test --manifest-path src-tauri/Cargo.toml --offline`: pass
  - 18 passed
  - 2 ignored: live Claude and authenticated Codex checks
- `cargo clippy --manifest-path src-tauri/Cargo.toml --offline --all-targets -- -D warnings`: pass
- `NO_STRIP=1 npm run tauri -- build -v`: environment blocker with exact evidence
  - linuxdeploy reached the GTK plugin stage and then failed with:

```text
[gtk/stderr] env: ‘/tmp/appimage_extracted_5a27dadc81a3abf452534fdbee9c58ee/usr/bin/linuxdeploy (deleted)’: No such file or directory
[gtk/stderr] env: use -[v]S to pass options in shebang lines
ERROR: Failed to run plugin: gtk (exit code: 127)
failed to bundle project: `failed to run /home/corye/.cache/tauri/linuxdeploy-x86_64.AppImage`
```

  - The frontend remediation in this pass did not touch bundling config or Rust packaging code.

## Acceptance audit

### Pass

- Tray close behavior is now safe by construction: hidden-window mode is only enabled after tray interaction proves recovery for the current session.
- Autostart persistence now reflects only verified desktop state, and mismatch is surfaced as an error instead of silently drifting.
- Notification behavior remains non-blocking and bounded, and tray-triggered refreshes now use the same deduped notification path as manual refresh.
- Capability exposure remains narrow and unchanged from the prior validation pass.
- Packaging documentation matches the current tree and the observed linuxdeploy failure mode.

### Residual manual checks

- Native tray rendering, live notifications, and XDG autostart behavior in the target desktop session remain manual release checks, as already documented in [docs/release.md](../../docs/release.md#L27).
- AppImage reruns are currently subject to an environment-level linuxdeploy extraction/plugin failure in this tool context. The brief allows a documented environment blocker with exact evidence.
- These are not blocking findings for this code revalidation because the previously failing product behaviors are now enforced and tested, and the current packaging failure is not attributable to the frontend remediation.

## Gate summary

- Frontend checks: pass
- Bridge checks: pass
- Rust fmt/test/clippy: pass
- Tray close/quit lifecycle: pass
- Autostart persistence/idempotency: pass
- Tray refresh event reconciliation/dedupe: pass
- Notification non-blocking semantics: pass
- AppImage packaging: blocked by documented environment issue with exact evidence
- Overall desktop increment gate: pass
