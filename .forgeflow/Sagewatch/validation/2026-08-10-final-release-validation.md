# Final release validation — 2026-08-10

Verdict: PASS

## Automated evidence

- `npm run build`: pass
- `npm run test:frontend`: pass
- `npm run test:bridge`: pass
- `cargo fmt --check`: pass
- `cargo test --offline`: 12 passed; 1 intentional live test ignored by default
- `cargo clippy --offline --all-targets -- -D warnings`: pass
- `cargo build --release --offline`: pass
- Explicit authenticated Codex app-server handshake test: pass
- Explicit authenticated Claude status-line bridge and Rust adapter contract test: pass

## Review evidence

- Compass revalidation: pass after preference, scheduling, backoff, and alert remediation
- Arbiter integration: pass after shared Claude model-window contract fixture remediation
- Visual inspection: compact 420×640 layout passes; unavailable providers display no fabricated percentage

## Live-provider evidence

- Claude Code emitted a sanitized snapshot with five-hour and seven-day windows at the documented XDG path using `0600` file permissions.
- The Rust Claude adapter loaded and normalized that exact snapshot successfully.
- The Codex adapter completed an authenticated local app-server handshake and normalized the returned windows.
