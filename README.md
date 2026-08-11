# Sagewatch

**A private, local-first Linux dashboard for Claude and Codex usage allowances.**

Sagewatch keeps your remaining allowance, reset time, freshness, and provider health visible in one compact desktop widget. It uses a polished, provider-aware interface and stays honest about stale or unavailable data.

> Sagewatch runs locally. It has no account system, cloud service, analytics, or telemetry.

## What it shows

- Claude and Codex remaining allowance side by side
- Absolute reset dates and times
- Live, recent, stale, and unavailable freshness states
- Provider health and source-confidence details
- Local threshold notifications
- A tray icon and optional launch at login
- Responsive light and dark themes

## Privacy by design

Sagewatch is built around a narrow, read-only data model:

- Provider credentials, cookies, tokens, prompts, and transcripts are never stored.
- Provider snapshots are saved locally in private, atomic JSON files.
- Claude's optional `/usage` probe runs in a private working directory and retains only its sanitized status-line snapshot.
- The sensitive Codex rollout-cache fallback is disabled unless explicitly enabled.
- One provider failing never hides or invalidates the other.
- Stale or estimated information is never presented as live and exact.

Read the complete [privacy model](docs/privacy.md) before enabling either optional fallback.

## Install on Linux

Download the AppImage from the [latest release](https://github.com/BrandedTamarasu-glitch/Sagewatch/releases/latest), make it executable, and launch it:

```sh
chmod +x Sagewatch_0.1.4_amd64.AppImage
./Sagewatch_0.1.4_amd64.AppImage
```

> **Your accounts stay yours.** Sagewatch contains no bundled account, credential, or usage data. It reads allowance information from the Claude Code and Codex CLIs installed and authenticated for the current Linux user. If a CLI is missing or signed out, that provider appears unavailable until the user installs or signs in to it.

Sagewatch currently targets 64-bit Linux desktops. See the [setup guide](docs/setup.md) for provider configuration, desktop integration, and removal.

### KDE Plasma 6 widget

Download `Sagewatch_Plasma_0.1.4_amd64.tar.gz` from the latest release, extract it, and run its installer:

```sh
tar -xzf Sagewatch_Plasma_0.1.4_amd64.tar.gz
cd Sagewatch_Plasma_0.1.4_amd64
./install.sh
```

From a source checkout, install it for the current user with:

```sh
./scripts/install-plasma-widget.sh
```

Then right-click the desktop, choose **Enter Edit Mode**, select **Add Widgets**, and add **Sagewatch**. The installer builds a read-only local data helper in `~/.local/libexec/` and installs the widget in `~/.local/share/plasma/plasmoids/`. It does not require root access.

## Provider support

| Provider | Integration | Notes |
| --- | --- | --- |
| Codex | Local app-server adapter | Authenticated handshake and allowance feed verified locally |
| Claude | Documented status-line input | Updates follow Claude Code sessions; the optional zero-token `/usage` probe can refresh on demand |

Both integrations are deliberately version-aware and surface an explicit diagnostic instead of silently guessing when an upstream format changes.

## Development

Prerequisites:

- Node.js 20 or newer
- Rust stable
- The [Tauri 2 Linux system dependencies](https://v2.tauri.app/start/prerequisites/)

```sh
npm install
npm run test:frontend
npm run test:bridge
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
NO_STRIP=1 npm run tauri -- build
./scripts/install-plasma-widget.sh
```

`NO_STRIP=1` avoids an incompatibility between rolling-release Linux libraries and the older `strip` bundled with `linuxdeploy`.

## Documentation

- [Setup](docs/setup.md)
- [Privacy](docs/privacy.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Release checklist](docs/release.md)

## Status

Sagewatch is an early release. Provider interfaces can change, so diagnostics and explicit freshness labels are part of the product contract—not incidental UI.
