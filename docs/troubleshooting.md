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

## Reset time shows 1970

An epoch-like reset time such as `Jan 21, 1970` means the source did not provide a usable reset timestamp and the app is showing a malformed or placeholder value. Treat it as stale or invalid data, not as a real live reset. Re-run a verified refresh from the underlying source and do not ship the value until the adapter or ingest path is fixed.

## Permission or write failures

The bridge needs permission to create its data directory and replace its snapshot. Check ownership of the selected path. Expected Unix permissions are `0700` for the `ingest` directory and `0600` for the JSON file.

## Stale data

Staleness is expected after Claude Code stops sending status-line observations. Sagewatch retains the last normalized snapshot but must label it stale. Verify current allowance manually with `/usage` or Settings > Usage.

## Tray icon missing

- Confirm the desktop session supports a system tray or StatusNotifierItem/AppIndicator-style tray surface.
- Confirm the desktop environment has the libraries it needs for tray support on Linux.
- If the tray works in one desktop session but not another, treat it as an environment limitation rather than a Sagewatch data failure.
- Sagewatch only enables close-to-tray after a tray interaction proves that the current session exposes a recovery path. Until then, Close exits safely.

## Start at login did not stick

- Confirm the desktop session honors XDG autostart entries.
- Check whether the session is sandboxed or managed by a launcher that ignores `~/.config/autostart/`.
- If the preference fails, Sagewatch should leave the app usable and report the failure honestly instead of pretending the login entry exists.

## AppImage will not build

- Confirm the host has the Tauri Linux prerequisites listed in the official [prerequisites guide](https://v2.tauri.app/start/prerequisites/).
- Confirm AppImage bundling is enabled and the bundle icon resolves to `src-tauri/icons/icon.png`.
- If linuxdeploy reports `unknown type [0x13] section '.relr.dyn'`, rerun with `NO_STRIP=1 npm run tauri -- build`. This is a compatibility issue between rolling-release host libraries and linuxdeploy's bundled `strip`.
- If you build on a newer Linux base than the runtime machines you need to support, watch for glibc compatibility failures. Tauri recommends building on an old-enough baseline, such as Ubuntu 22.04 or Debian 12, for broad Linux compatibility.

## Development checks

```sh
npm run test:bridge
npm run build
cd src-tauri && cargo test
```
