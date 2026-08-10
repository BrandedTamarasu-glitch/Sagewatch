# Release checklist

## Automated gates

```sh
npm ci
npm run build
npm run test:frontend
npm run test:bridge
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --offline
cargo clippy --manifest-path src-tauri/Cargo.toml --offline --all-targets -- -D warnings
cargo build --manifest-path src-tauri/Cargo.toml --release --offline
```

Tauri bundling is enabled for the AppImage target. A validated local build produces
`src-tauri/target/release/bundle/appimage/Sagewatch_0.1.0_amd64.AppImage`.
On rolling-release distributions with `.relr.dyn` library sections, run
`NO_STRIP=1 npm run tauri -- build` to avoid linuxdeploy's older bundled `strip`.

The authenticated Codex check is intentionally ignored by the default suite. Run it only on a signed-in developer machine:

```sh
cargo test --manifest-path src-tauri/Cargo.toml live_authenticated_app_server_handshake -- --ignored
```

## Manual gates

- Verify Claude ingestion from an active authenticated Claude Code session.
- Confirm Claude becomes stale rather than live after status-line observations stop.
- Inspect the compact view at 420×640 and 200% scaling.
- Check keyboard navigation, dialog focus return, visible focus, dark mode, high contrast, and reduced motion.
- Confirm unavailable providers show no example or fabricated percentage.
- Confirm no raw provider response, transcript, credential, cookie, or token is present in app storage or logs.
- Confirm the Codex CLI compatibility gate matches the intended release environment.
- Confirm the tray icon, tray menu, autostart preference, and notifications are usable in the target desktop environment.
- Confirm AppImage packaging is either produced or blocked with exact evidence.

The 2026-08-10 v0.1.0 release candidate passed both authenticated provider checks on the development machine.

## Linux packaging

When AppImage bundling is enabled for a release run, Tauri documents the output as an executable AppImage that can be launched directly after `chmod +x`. Build on an old-enough Linux base to avoid glibc compatibility problems on older user systems, and use the Tauri Linux prerequisites for the host machine. See the official Tauri prerequisites and AppImage docs for the current package list and limitations:

- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri AppImage guide](https://v2.tauri.app/distribute/appimage/)

Install flow for an AppImage release:

1. Download or copy the `*.AppImage` artifact from the release.
2. Make it executable: `chmod +x Sagewatch-*.AppImage`.
3. Launch it directly: `./Sagewatch-*.AppImage`.
4. If you want a persistent menu entry, place the file wherever you keep local apps and let your desktop launcher index it.

Removal flow:

1. Delete the AppImage file.
2. Remove any desktop launcher or autostart entry you created for it.
3. If you installed a release-specific autostart file, remove the corresponding file from `~/.config/autostart/`.

If the build host is newer than the oldest runtime you need to support, record that as a packaging risk. Tauri warns that newer build hosts can raise the glibc floor and produce runtime errors on older machines.

## Daily-use soak test

Use this checklist for a practical one-day soak on the target desktop:

- Startup: cold-launch after reboot and confirm the app restores the last visible state.
- Tray lifecycle: first use a tray action to confirm the desktop exposes the tray, then close the main window, confirm it hides, and confirm tray actions still work while hidden. In a session that never exposes the tray, confirm Close exits instead of leaving an inaccessible background process.
- Provider isolation: break one provider path and verify the other still refreshes and remains visible.
- Refresh cadence: wait through at least two scheduled refresh intervals and confirm the cadence does not drift or double-fire.
- Threshold dedupe: cross a warning threshold once, then verify the resulting notification and in-app alert fire once until the state genuinely changes again.
- Suspend/resume: suspend the machine, resume it, and confirm freshness, timers, and stale state behave honestly.
- Offline: disconnect from the network or block upstream access and confirm local state remains readable and does not collapse into fabricated live data.
- Persistence: restart the app and verify preferences, last snapshot, tray behavior, and login preference survive the restart.

## Shipping boundaries

- The Codex rollout transcript fallback is not shipped.
- Team-wide administrative analytics, multi-account support, and cloud sync are deferred.
- Do not publish or push until a remote and release destination are explicitly approved.
