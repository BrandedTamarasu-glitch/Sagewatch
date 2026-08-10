# Current Plan — Sagewatch

Status: implementation and release validation complete; one live Claude session verification remains a documented residual check.

Canonical phase plan: [../../docs/implementation-phases.md](../../docs/implementation-phases.md)

## Phase Sequence

1. **Source discovery and contracts** — completed. Claude status-line ingestion and Codex app-server access were narrowed to read-only local sources.
2. **Project foundation and widget shell** — completed. The Tauri/Vite shell, fixture-first UI, and compact/expanded rendering are in place.
3. **Provider framework and local safety** — completed. Rust owns the provider boundary, normalization, safe storage, and refresh orchestration.
4. **Claude Teams and Codex adapters** — completed. Claude uses the session-attached status-line bridge; Codex uses the local `app-server` path with the live probe captured in the adapter tests.
5. **Product polish, preferences, and alerts** — completed for the shipped UI/settings surface. The rollout-cache fallback remains deferred and disabled; preference restoration, scheduled refresh, backend backoff, and accessible threshold alerts are wired.
6. **Verification, documentation, and release readiness** — completed for the implemented surface. Build, frontend tests, bridge tests, and the default Rust suite are green; Claude live-session verification is still pending.

## Hard Gate

The original gate is satisfied at the contract level: both providers have read-only local sources or honest degraded modes, and Codex remains version-gated to the local CLI/app-server surface.

## Scope Guardrail

Version 1 is a single-user, local, read-only allowance monitor. Team-wide administrative analytics, billing actions, multi-user reporting, cloud sync, and automated purchases remain deferred.

## Validation Summary

- `npm run build` passes.
- `npm run test:frontend` passes.
- `npm run test:bridge` passes.
- `cargo test --offline` passes with 12 Rust tests and 1 intentionally ignored live Codex handshake probe.
- The ignored live Codex app-server test was run explicitly against the authenticated local CLI and passed.
- `cargo fmt --check`, strict Clippy, and the release build pass.
- The remaining live probe is Claude status-line verification in an authenticated active Claude Code session.
- Stale or estimated data is labeled explicitly.
- Logs, caches, and fixtures contain no reusable secrets.
- Keyboard, contrast, reduced motion, screen-reader labeling, responsive 200% layout, and honest unavailable states passed Forgeflow review and visual inspection.
