## Summary

Sagewatch v0.1.0

Initial local release of a read-only Linux allowance monitor for Claude Teams and Codex subscriptions.

### Included

- Tauri 2/Rust provider, normalization, cache, diagnostics, and backoff layer
- Version-gated local Codex app-server integration with a verified authenticated handshake
- Sanitized Claude Code status-line bridge with session-attached freshness
- Accessible compact cards, expanded details, preferences, scheduled refresh, and threshold alerts
- Local-only atomic storage with no credential, transcript, or raw-response retention

### Validation

- Frontend production build and release Rust build
- Frontend and Claude bridge tests
- 12 default Rust tests plus a separately executed live Codex handshake test
- Rust formatting and strict Clippy
- Compass release validation and Arbiter integration approval
- 420×640 visual inspection, including honest unavailable states

## Review Gate

- Status: APPROVE
- Note: Compass revalidation and Arbiter integration review both passed after remediation.

## Generated Artifacts

- /home/corye/openai-cli/Sagewatch/.forgeflow/Sagewatch/ship/ship-summary.json
- /home/corye/openai-cli/Sagewatch/.forgeflow/Sagewatch/ship/ship-presentation.html

## Residual limitation

Claude live-session ingestion still requires verification from an active authenticated Claude Code session on the target machine. When no session observation exists, Sagewatch reports unavailable or stale data rather than fabricating a value.
