# Codebase Map

Living architecture notes for Sagewatch.

## Current Shape

- Tauri 2 backend with a Vite/TypeScript frontend.
- Rust owns domain normalization, provider ingestion, persistence, refresh orchestration, and Tauri commands.
- The frontend is presentation-first and consumes typed IPC DTOs plus fixture-backed samples.
- The only local provider bridge outside the app tree is the Claude status-line sanitizer/writer script.

## Ownership Map

### Warden

Owns the Rust core and provider boundary:

- `src-tauri/src/domain/mod.rs`
- `src-tauri/src/providers/mod.rs`
- `src-tauri/src/providers/claude.rs`
- `src-tauri/src/providers/codex.rs`
- `src-tauri/src/store/mod.rs`
- `src-tauri/src/service/mod.rs`
- `scripts/claude-statusline-bridge.mjs`
- `scripts/__tests__/claude-statusline-bridge.test.mjs`

Responsibilities:

- Normalize provider payloads into the locked allowance contract.
- Keep raw provider payloads out of persistence and UI surfaces.
- Enforce retry, health, freshness, and corruption-recovery behavior.
- Gate Codex to the local `app-server` contract and keep the rollout fallback disabled by default.

### Smith

Owns the Tauri command surface and the runtime/frontend glue:

- `src-tauri/src/lib.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/status.rs`
- `src-tauri/src/commands/preferences.rs`
- `src-tauri/src/commands/diagnostics.rs`
- `src/lib/api.ts`
- `src/lib/types.ts`
- `src/App.ts`
- `src/main.ts`
- `src/fixtures/status.ts`
- `tests/frontend/render.test.mjs`

Responsibilities:

- Expose only the approved IPC commands.
- Keep frontend state fixture-first, then swap in live runtime snapshots when available.
- Maintain typed DTO parity with the Rust contract.
- Keep the UI shell responsive to refresh, details, diagnostics, and settings actions.

### Lumen

Owns the visual presentation layer:

- `src/components/render.ts`
- `src/styles.css`

Responsibilities:

- Render the compact cards, details dialog, diagnostics, and settings markup.
- Preserve accessibility semantics, keyboard flow, and visible trust signals.
- Keep styling expressive without moving parsing or business logic into the view layer.

## Validation Surfaces

- Frontend build output is `dist/` from `npm run build`.
- Frontend rendering tests live in `tests/frontend/render.test.mjs`.
- Bridge sanitation tests live in `scripts/__tests__/claude-statusline-bridge.test.mjs`.
- Rust unit coverage is centered in `src-tauri/src/{domain,providers,service,store}.rs`.
- The live Codex handshake probe is the ignored `live_authenticated_app_server_handshake` test in `src-tauri/src/providers/codex.rs`.

## Deferred Surfaces

- Codex rollout JSONL fallback remains off by default and intentionally unimplemented beyond the toggle and privacy note.
- Team/admin analytics, cloud sync, and multi-user reporting remain out of scope for v1.
