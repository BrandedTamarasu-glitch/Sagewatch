# Sagewatch

Sagewatch is a local-first Linux desktop widget for the remaining allowance, reset time, freshness, and health of one Claude Teams seat and one Codex subscription.

The current Waves 1–2 implementation includes a Tauri 2/Rust backend scaffold, a Vite/React interface, private atomic JSON storage, fake provider adapters, and a sanitized bridge for Claude Code's documented status-line `rate_limits` input. Live provider adapters remain intentionally disabled until their contracts are verified.

## Development

Prerequisites are Node.js 20+, Rust stable, and the Linux system libraries required by Tauri 2.

```sh
npm install
npm run build
npm run test:bridge
npm run tauri dev
```

See [setup](docs/setup.md), [privacy](docs/privacy.md), and [troubleshooting](docs/troubleshooting.md) for provider integration details.

## Product principles

- Local-first, read-only provider access
- No cloud aggregation or telemetry
- No credential, cookie, token, transcript, or raw provider-response retention
- Stale or estimated data is never presented as live and exact
- One provider failure never hides the other
- Status remains understandable without relying on color

