# Privacy

Sagewatch is designed to keep provider data local and minimize what it retains.

The Claude bridge accepts the status-line JSON through standard input, builds a new object, and discards the original. The stored object contains only:

- schema version and bridge-generated observation time;
- an optional short, printable plan label;
- recognized rate-limit windows and their percentage/reset fields.

Unknown windows, unknown fields, credentials, tokens, paths, transcripts, and raw payloads are discarded. Percentages are clamped to 0–100 and reset values must parse as timestamps. The bridge emits no telemetry and does not access the network.

The snapshot directory is created with Unix mode `0700`; its atomically replaced JSON file uses `0600`. The default location is `${XDG_DATA_HOME:-$HOME/.local/share}/sagewatch/ingest/claude-statusline.json`. Anyone with access to your user account may still be able to read it.

The experimental Claude `/usage` refresh is opt-in and disabled by default. It launches the already authenticated local Claude Code client in a PTY and sends `/usage`, Escape, and `/exit`. A bounded in-memory scanner may press Enter only when it sees both exact phrases from Claude's one-time trust screen for Sagewatch's controlled empty private directory. Sagewatch otherwise drains and discards terminal output so the child cannot block; it does not parse, persist, or log that output. The only accepted result is a newer sanitized status-line snapshot written by the existing bridge. Authentication remains owned by Claude Code.

Optional `--passthrough` chaining is outside Sagewatch's privacy boundary. A configured status-line executable receives the original input and controls its own output, storage, and network behavior.

Sagewatch does not collect credentials or call hidden provider endpoints. The opt-in Codex rollout-cache fallback described in planning is disabled and unimplemented.
