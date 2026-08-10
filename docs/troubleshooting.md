# Troubleshooting

## No Claude observation appears

- Confirm Claude Code is actively invoking the configured status line; this source is session-attached.
- Use an absolute path to `claude-statusline-bridge.mjs` in Claude Code settings.
- Confirm Node.js 20 or newer is available to the status-line process.
- Check the default snapshot path: `${XDG_DATA_HOME:-$HOME/.local/share}/sagewatch/ingest/claude-statusline.json`.
- If `SAGEWATCH_CLAUDE_SNAPSHOT_PATH` is set, ensure both the bridge and Sagewatch are launched with the same value.

## “rate_limits contains no supported windows”

The bridge fails closed when the upstream shape is absent or has drifted. It supports `five_hour`, `seven_day`, `seven_day_sonnet`, `seven_day_opus`, and `extra_usage`, with percentage and reset fields only. Do not work around this by saving the raw status-line payload. Report the sanitized field names and update the bridge contract deliberately.

## Existing status line disappeared

The bridge emits no visual status by itself. Configure `--passthrough /absolute/path/to/executable` as shown in [setup](setup.md). Shell expressions, aliases, pipelines, and relative executable paths are deliberately unsupported; place complex rendering in a trusted executable script and pass its absolute path.

## Permission or write failures

The bridge needs permission to create its data directory and replace its snapshot. Check ownership of the selected path. Expected Unix permissions are `0700` for the `ingest` directory and `0600` for the JSON file.

## Stale data

Staleness is expected after Claude Code stops sending status-line observations. Sagewatch retains the last normalized snapshot but must label it stale. Verify current allowance manually with `/usage` or Settings > Usage.

## Development checks

```sh
npm run test:bridge
npm run build
cd src-tauri && cargo test
```

