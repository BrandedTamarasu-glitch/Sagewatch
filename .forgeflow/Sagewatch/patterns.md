# Patterns

Project-specific good patterns and anti-patterns.

## Good Patterns

- Normalize provider data in Rust before it ever reaches the UI or disk.
- Preserve every allowance window, then derive the compact headline from the most constrained active window.
- Prefer explicit health/freshness/source-confidence states over inferred certainty.
- Use atomic private writes with restrictive permissions for local snapshot and preference files.
- Keep provider failures isolated so one provider never erases the other provider's last known snapshot.
- Render from DTOs and fixtures in the frontend; do not re-interpret provider payloads there.
- Show accessibility and trust state in text, not just color or iconography.
- Keep the Claude status-line bridge as a sanitizing writer, not a general-purpose scraper.

## Anti-Patterns

- Do not persist raw provider payloads, transcripts, cookies, tokens, or secrets.
- Do not flatten multiple allowance windows into a single percentage or guessed reset time.
- Do not treat the Codex rollout JSONL path as a default code path; it remains deferred and opt-in.
- Do not let provider drift masquerade as healthy data.
- Do not move parsing or policy logic into `src/App.ts` or the render helpers.
- Do not couple the refresh lifecycle of one provider to the availability of the other.

## Validation Notes

- `npm run build` passes.
- `npm run test:frontend` passes.
- `npm run test:bridge` passes.
- `cargo test` passes with 9 tests and 1 ignored live Codex handshake probe.
- The remaining open live probe is Claude session-attached status-line verification in the user's authenticated local Claude Code context.
