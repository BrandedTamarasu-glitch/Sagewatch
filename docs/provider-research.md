# Provider research

The current Forgeflow research and Phase 0 decision are maintained in [current-research.md](../.forgeflow/Sagewatch/current-research.md).

Decision summary:

- Claude Teams is fed by Claude Code's documented status-line `rate_limits` data and is explicitly session-attached.
- Codex uses the local experimental app-server `account/rateLimits/read` method.
- A privacy-sensitive Codex rollout-cache fallback is optional and disabled by default.
- Browser scraping, hidden OAuth endpoint calls, and direct credential collection are excluded.
