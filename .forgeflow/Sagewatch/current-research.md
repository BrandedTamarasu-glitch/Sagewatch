# Current Research — Provider observability

Date: 2026-08-07

## Recommendation

Clear Phase 0 with a constrained source strategy:

- **Claude Teams:** consume the officially documented Claude Code status-line `rate_limits` payload. This is session-attached rather than a general background API, so the last observation becomes `stale` when Claude Code is not running. Manual verification through `/usage` or Settings > Usage remains the fallback.
- **Codex subscription:** use the local Codex app-server request `account/rateLimits/read`. The installed CLI's generated protocol includes primary and secondary windows, used percentage, and reset timestamps. Treat this integration as version-coupled because app-server is currently marked experimental.
- **Codex fallback:** optionally extract only `rate_limits` events from local rollout JSONL files. This must be explicit opt-in because those files also contain conversation data; the parser must discard every unrelated event without retaining raw records.
- **Reject:** browser scraping, direct calls to undocumented Claude OAuth endpoints, copying tokens, or presenting inferred limits as exact.

This clears the gate because each provider has a read-only local source plus an honest degraded mode. It does not promise fully autonomous Claude refresh when no Claude Code session is active.

## Options considered

### Claude Teams

1. **Claude Code status-line payload — selected.** Anthropic documents `rate_limits` fields for watching remaining allowance. It reuses the user's existing authenticated client and avoids credential handling. Limitation: observations arrive with active Claude Code status updates.
2. **Interactive `/usage` or Settings > Usage — fallback.** Documented and authoritative, but not autonomous or machine-oriented.
3. **Direct internal `/api/oauth/usage` — rejected.** Present in the installed client but not a supported headless integration contract.
4. **Authenticated browser scraping — rejected.** Brittle, auth-sensitive, and unnecessary.

Expected Claude windows include `five_hour`, `seven_day`, model-scoped weekly limits, and possible overage/credit windows. The internal model must preserve all windows rather than flattening them.

### Codex subscription

1. **Local app server — selected.** Codex CLI 0.147.0 generates a JSON schema containing `account/rateLimits/read`, `account/rateLimits/updated`, `RateLimitWindow.usedPercent`, `RateLimitWindow.resetsAt`, and primary/secondary windows.
2. **Rate-limit events in local rollout JSONL — opt-in fallback.** Technically sufficient but privacy-sensitive because the surrounding files contain transcripts.
3. **Codex Settings > Usage — manual verification fallback.** Authoritative for the user but not suitable for routine widget refresh.
4. **Browser scraping — rejected.** Highest drift and authentication risk.

The sandboxed live app-server probe could not complete its writable sqlite initialization under `~/.codex`; implementation must run the adapter in the user's normal local context and test its handshake before depending on live values.

## Normalized contract

```text
ProviderStatus
  provider: claude | codex
  plan: string | unknown
  observed_at: timestamp
  source: claude_statusline | codex_app_server | codex_rollout_cache | manual
  source_confidence: documented_local | experimental_local | sensitive_local_cache | manual
  freshness: live | recent | stale | unknown
  health: healthy | signed_out | unavailable | unsupported | source_changed | error
  windows: AllowanceWindow[]

AllowanceWindow
  id: provider-stable identifier when available
  label: human-readable window name
  duration_minutes: number | null
  used_percent: number | null
  remaining_percent: number | null
  reset_at: timestamp | null
  kind: rolling | weekly | model_scoped | credits | unknown
```

Normalization rules:

- Preserve multiple windows.
- Derive `remaining_percent = 100 - used_percent` only after clamping verified numeric input to 0–100.
- Never derive a reset time that the source did not provide.
- The compact headline may use the lowest remaining active window; details list every window.
- Show reset time as absolute local time, with a relative countdown only as secondary text.
- Track `freshness` separately from `health`.

## Privacy and security posture

- Never request or persist account passwords, OAuth tokens, cookies, or raw authenticated responses.
- Prefer data delivered by already authenticated local clients.
- Store normalized snapshots only, using atomic local writes and restrictive permissions.
- Redact logs by construction; log adapter state and schema errors, not payloads.
- Keep transcript fallback disabled by default and explain its privacy implications before opt-in.
- Poll conservatively. Prefer events/status updates and user-triggered refresh over sub-minute loops.
- One provider's failure must not affect the other provider's last known state.

## Codebase patterns

- Reuse the parent workspace's Tauri 2/Vite split: thin web UI, Rust-owned provider and persistence boundary.
- Follow the existing Rust validation, mutex-protected state, atomic-write, and rollback patterns.
- Reuse CI patterns of `cargo check` plus frontend build.
- Run smoke tests against an isolated temporary home rather than real subscription state.
- Treat unstable provider surfaces like PenguinSlide treats unstable platform integration: capability-gate them, isolate them, and fail closed.

## Risks and tradeoffs

- Claude data is session-attached, so background freshness is limited. UI must make staleness obvious.
- Codex app-server is experimental and may change across CLI updates. Pin compatible versions and use contract tests.
- Allowance semantics differ by provider and plan. Do not label all secondary windows simply as “weekly” without source evidence.
- Rollout fallback touches files that contain sensitive conversations. It must be opt-in, streaming, event-filtered, and non-retaining.
- Provider refreshes can fail due to signed-out clients, sandboxes, or state-directory permissions. Diagnostics must distinguish these states.

## Accessibility implications

- Every freshness and health state needs visible text, not only color or iconography.
- Compact view uses the most constrained window but exposes the selection rule accessibly.
- Detailed view lists all windows with explicit provider, label, percentage, and absolute reset time.
- Status changes should not create noisy live-region announcements; reserve announcements for user-initiated refresh and threshold crossings.
- Keyboard, 200% scaling, high contrast, and reduced motion remain release gates.

## Phase 0 decision

Phase 0 is **conditionally passed**. Proceed to consultation and fixture-first implementation with these constraints:

1. Prototype and contract-test the Codex app-server handshake before UI integration.
2. Implement Claude as status-line-fed and explicitly session-attached.
3. Keep all manual and cache fallbacks honest about freshness and confidence.
4. Do not add direct provider credential handling or browser scraping.

## Evidence

- Anthropic documents `/usage`, subscription usage bars, rolling limits, and custom status-line rate-limit fields in the Claude Code documentation.
- The locally installed Claude Code 2.1.220 binary models five-hour, weekly, model-scoped, utilization, and reset fields.
- The locally installed Codex CLI 0.147.0 generated an experimental app-server schema containing `account/rateLimits/read` and structured rate-limit windows.
- Existing workspace Tauri/Vite, CI, smoke-test, and platform capability-gating patterns provide an implementation baseline.
