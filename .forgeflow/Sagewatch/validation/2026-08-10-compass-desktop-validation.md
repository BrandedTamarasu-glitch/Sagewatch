# Sagewatch desktop integration validation

Date: 2026-08-10
Validator: Compass
Scope: daily-use desktop integration increment validation against `.forgeflow/Sagewatch/current-brief.md`

## Verdict

Fail.

The automated frontend and Rust gates are green, and an AppImage can now be produced in this environment with a documented `NO_STRIP=1` workaround. The increment still fails the desktop-integration acceptance gates because tray lifecycle and autostart persistence are not yet safe enough to sign off.

## Findings

### 1. Close-to-tray can strand the app when the desktop session does not surface the tray icon

Severity: high

- The main window always intercepts close and hides instead of exiting in [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L94).
- The only built-in recovery path after hiding is the tray menu created in [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs#L28).
- There is no fallback path when the Linux shell does not render the tray icon or renders it inconsistently.
- The project documentation already acknowledges that tray availability varies by desktop session in [docs/troubleshooting.md](../../docs/troubleshooting.md#L31).

This violates the brief's gate that window close must not leave an unquittable background process behind. In sessions where the tray is unavailable or invisible, closing the window removes the only visible control surface.

### 2. The persisted start-at-login preference can drift from the verified desktop autostart state

Severity: medium

- `set_autostart_enabled` writes `preferences.start_at_login = enabled` before it performs the final `autostart.is_enabled()` verification in [src-tauri/src/commands/desktop.rs](../../src-tauri/src/commands/desktop.rs#L49).
- If the autostart backend accepts the enable/disable call but the final state verifies differently, the command returns the verified boolean to the frontend but leaves the stored preference on the requested value.
- That creates a restart-time mismatch: `preferences.json` can claim start-at-login is enabled while the desktop facility reports that it is not.

This does not meet the brief's requirement that autostart remain user-controlled, persisted, idempotent, and honest about failure.

### 3. The release/setup/troubleshooting docs still describe AppImage bundling as disabled and omit the `NO_STRIP=1` Linux packaging workaround

Severity: low

- [docs/release.md](../../docs/release.md#L16), [docs/setup.md](../../docs/setup.md#L71), and [docs/troubleshooting.md](../../docs/troubleshooting.md#L43) say the repo keeps `bundle.active` disabled.
- The current config in [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json#L28) has `"active": true`.
- A successful packaging run in this environment required `NO_STRIP=1 npm run tauri -- build`, but that workaround is not documented.

This does not block execution by itself, but it makes the installation and troubleshooting guidance inaccurate for the current tree.

## Automated command results

- `npm run test:frontend`: pass
- `npm run test:bridge`: pass
- `npm run build`: pass
- `cargo fmt --manifest-path src-tauri/Cargo.toml --check`: pass
- `cargo test --manifest-path src-tauri/Cargo.toml --offline`: pass
  - 15 passed
  - 2 ignored: live Claude and authenticated Codex checks
- `cargo clippy --manifest-path src-tauri/Cargo.toml --offline --all-targets -- -D warnings`: pass
- `cargo build --manifest-path src-tauri/Cargo.toml --release --offline`: pass
- `npm run tauri build -- --bundles appimage`: initial fail with AppImage icon panic
- `NO_STRIP=1 npm run tauri -- build`: pass in the user's CachyOS environment
  - produced `/home/corye/openai-cli/Sagewatch/src-tauri/target/release/bundle/appimage/Sagewatch_0.1.0_amd64.AppImage`

## Acceptance audit

### Pass

- Capability exposure is still narrow. The default capability only adds `autostart:allow-enable`, `autostart:allow-disable`, `autostart:allow-is-enabled`, and `notification:allow-notify` in [src-tauri/capabilities/default.json](../../src-tauri/capabilities/default.json#L1).
- Notification failure is non-blocking by inspection and test coverage. The frontend uses `Promise.allSettled` in [src/lib/alerts.ts](../../src/lib/alerts.ts#L9) and keeps the in-app alert path alive in [src/App.ts](../../src/App.ts#L121). Coverage exists in [tests/frontend/render.test.mjs](../../tests/frontend/render.test.mjs#L84).
- Accessible error surfacing for the autostart checkbox is present in [src/components/render.ts](../../src/components/render.ts#L47) and covered in [tests/frontend/render.test.mjs](../../tests/frontend/render.test.mjs#L42).
- Generic preference writes cannot silently change desktop login state because `set_preferences` preserves the stored autostart value in [src-tauri/src/commands/preferences.rs](../../src-tauri/src/commands/preferences.rs#L5).
- AppImage packaging is now green with environment-specific evidence. The artifact exists at `/home/corye/openai-cli/Sagewatch/src-tauri/target/release/bundle/appimage/Sagewatch_0.1.0_amd64.AppImage`, and the user's successful build used `NO_STRIP=1 npm run tauri -- build` after adding an explicit bundle icon in [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json#L28).

### Blocked or not fully proven in this environment

- Native tray, hidden-window recovery, live notifications, and real autostart file creation were not fully exercised interactively from this validation environment.
- Runtime smoke-launch evidence:

```text
thread 'main' (10) panicked at .../tao-0.35.3/src/platform_impl/linux/event_loop.rs:217:53:
Failed to initialize gtk backend!: BoolError { message: "Failed to initialize GTK", ... }
```

- Command used: `timeout 5s /home/corye/openai-cli/Sagewatch/src-tauri/target/release/sagewatch --hidden`

That GTK initialization failure blocks live desktop-session verification from this tool context, so tray/menu/quit behavior in an actual user session remains inspection-based here.

## Gate summary

- Frontend checks: pass
- Bridge checks: pass
- Rust fmt/test/clippy/release build: pass
- Capability exposure: pass
- Notification non-blocking semantics: pass
- Accessibility evidence: pass by inspection and test coverage, with live assistive-tech checks still manual
- Tray close/quit lifecycle: fail due missing no-tray fallback
- Autostart persistence/idempotency: fail due verified-state drift risk
- AppImage packaging: pass with documented CachyOS `NO_STRIP=1` workaround
- Overall desktop increment gate: fail
