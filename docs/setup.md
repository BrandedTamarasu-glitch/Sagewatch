# Setup

## Development app

Install Node.js 20+, Rust stable, and the [Tauri 2 Linux prerequisites](https://v2.tauri.app/start/prerequisites/), then run:

```sh
npm install
npm run tauri dev
```

Sagewatch includes live local adapters for Claude and Codex. Until a provider produces a verified observation, the interface displays an honest unavailable state rather than example percentages.

## Account and authentication boundary

Sagewatch does not ship with a Claude or Codex account and does not ask users to enter provider credentials. It uses the Claude Code and Codex CLIs installed and authenticated for the current Linux user:

- Each installation displays allowance data only for that user's locally authenticated CLI sessions.
- A missing or signed-out CLI produces an unavailable or signed-out provider state.
- Sagewatch does not copy, bundle, upload, or share account credentials or usage data.
- Installing a release built by another person does not give access to the builder's accounts.

## Claude Code status-line bridge

Claude Code sends its status-line command one JSON object on standard input. Configure the absolute bridge path as the status-line command in Claude Code:

```json
{
  "statusLine": {
    "type": "command",
    "command": "SAGEWATCH_CLAUDE_PLAN='Claude Team' node /absolute/path/to/Sagewatch/scripts/claude-statusline-bridge.mjs"
  }
}
```

`SAGEWATCH_CLAUDE_PLAN` is optional. Claude's status-line payload does not currently
include subscription-plan metadata, so set this only when you want Sagewatch to
display a manually confirmed plan name.

The bridge writes the sanitized observation to:

```text
${XDG_DATA_HOME:-$HOME/.local/share}/sagewatch/ingest/claude-statusline.json
```

Set `SAGEWATCH_CLAUDE_SNAPSHOT_PATH` to a custom destination if necessary. The live backend adapter and bridge must use the same override.

The bridge intentionally prints nothing by default. To retain an existing status-line program, append `--passthrough` and an absolute executable path plus its arguments. The executable is launched directly without a shell:

```json
{
  "statusLine": {
    "type": "command",
    "command": "node /absolute/path/to/Sagewatch/scripts/claude-statusline-bridge.mjs --passthrough /absolute/path/to/my-statusline"
  }
}
```

Only chain a program you trust: it receives Claude Code's original status-line JSON because it must render the existing output. Sagewatch never logs or persists that original input.

The feed is session-attached. It updates while Claude Code invokes the status line and becomes stale when those observations stop. Use Claude Code's `/usage` or Settings > Usage for manual verification.

## Desktop integration

The tray icon, notifications, and start-at-login preference are part of the desktop-integration release increment. They are local-only features and depend on the current Linux desktop session honoring the standard tray, notification, and XDG autostart facilities.

## Experimental background Claude refresh

Sagewatch can optionally refresh account-wide Claude usage even when no regular Claude session is open. Enable **Experimental Claude /usage refresh** in Settings. It is disabled by default.

When enabled, each Claude refresh starts the fixed Claude executable discovered and canonicalized at Sagewatch startup in a real local PTY, from a private Sagewatch working directory. Sagewatch checks for a compatible Claude Code 2.1.x version, waits for the interactive prompt, sends `/usage`, Escape, and `/exit`, and allows up to 17 seconds for the existing status-line bridge to advance its sanitized snapshot within a strict 22-second probe lifetime. It leaves `/usage` open for up to 12 seconds before returning to the prompt, allowing for the bridge's measured latency. On Claude's exact one-time safety screen it may also press Enter to trust that controlled empty directory. It never parses terminal percentages and does not persist or log PTY output, prompts, transcripts, or credentials. A timeout, concurrent probe, incompatible version, or unchanged snapshot fails safely and keeps the last valid status visible.

If you are testing an AppImage build, make the artifact executable and launch it directly:

```sh
chmod +x Sagewatch-*.AppImage
./Sagewatch-*.AppImage
```

Removal is the reverse:

1. Delete the `*.AppImage` file.
2. Remove any desktop launcher you created.
3. Remove any autostart entry created for Sagewatch from `~/.config/autostart/`.

The repository now enables the AppImage target. Build it with `npm run tauri -- build`.
On rolling-release distributions whose libraries contain `.relr.dyn` sections, use
`NO_STRIP=1 npm run tauri -- build` because linuxdeploy's bundled `strip` may be older
than the host libraries.
