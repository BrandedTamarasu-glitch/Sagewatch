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

## Shipping boundaries

- The Codex rollout transcript fallback is not shipped.
- Team-wide administrative analytics, multi-account support, and cloud sync are deferred.
- Do not publish or push until a remote and release destination are explicitly approved.

