# Sagewatch

Sagewatch is a local-first Linux desktop widget for the remaining allowance, reset time, freshness, and health of one Claude Teams seat and one Codex subscription.

Version 0.1 includes a Tauri 2/Rust backend, a Vite/TypeScript interface, private atomic JSON storage, a sanitized bridge for Claude Code's documented status-line `rate_limits` input, and a version-gated local Codex app-server adapter. The authenticated Codex handshake has been verified live; Claude updates remain session-attached and become explicitly stale when Claude Code stops emitting status-line data.

## Development

Prerequisites are Node.js 20+, Rust stable, and the Linux system libraries required by Tauri 2.

```sh
npm install
npm run build
npm run test:frontend
npm run test:bridge
npm run tauri build
npm run tauri dev
```

See [setup](docs/setup.md), [privacy](docs/privacy.md), [troubleshooting](docs/troubleshooting.md), and the [release checklist](docs/release.md) for provider and release details.

## Product principles

- Local-first, read-only provider access
- No cloud aggregation or telemetry
- No credential, cookie, token, transcript, or raw provider-response retention
- Stale or estimated data is never presented as live and exact
- One provider failure never hides the other
- Status remains understandable without relying on color
