# Sagewatch implementation phases

## Goal

Build an accessible, local Linux widget that reports personal Claude Teams and Codex subscription allowance status, including remaining usage, reset times, source freshness, and provider health.

## Phase 0 — Source discovery and contracts

Prove what each locally authenticated product exposes before committing to a UI or authentication design.

Deliverables:

- Claude and Codex capability matrix covering supported interfaces, local commands, app services, cached state, and safe fallbacks
- Authentication and privacy threat model
- Normalized status contract with `provider`, `plan`, `windows`, `remaining_percent`, `reset_at`, `observed_at`, `freshness`, `health`, and `source_confidence`
- Fixture examples for healthy, stale, signed-out, unavailable, exhausted, and changed-source states
- Decision record selecting the first supported Linux surface

Exit gate:

- Both providers have a proven read-only source or an explicitly approved fallback.
- Reset and percentage semantics are documented without pretending estimates are exact.
- No implementation depends on copying or storing raw account credentials.

## Phase 1 — Project foundation and widget shell

Create an isolated application and demonstrate the daily-use interaction with fixture data.

Deliverables:

- Application scaffold, initially targeting Tauri 2 and Vite unless Phase 0 finds a materially simpler Linux-native option
- Compact two-provider widget and expanded details/settings view
- Configuration bootstrap, test commands, formatting, and CI skeleton
- Placeholder provider states driven by Phase 0 fixtures

Exit gate:

- The application launches locally and renders compact and expanded views.
- Keyboard navigation works end to end.
- The interface remains readable at 200% scaling and in light and dark themes.

## Phase 2 — Provider framework and local safety

Build the provider-independent machinery before adding live collectors.

Deliverables:

- Common provider adapter interface
- Independent refresh scheduling, timeout, backoff, and manual refresh
- Local cache with explicit freshness and stale-data rules
- Error taxonomy for signed-out, throttled, unavailable, unsupported, changed-source, and unknown states
- Keyring-backed secret handling only if Phase 0 proves additional secrets are unavoidable
- Redacted structured logging and fixture-driven contract tests

Exit gate:

- A broken adapter cannot block the other provider.
- No secret or raw sensitive response appears in logs, caches, or fixtures.
- Offline and expired-session states remain useful and understandable.

## Phase 3 — Claude Teams and Codex adapters

Connect the widget to proven data sources, using supported/local interfaces ahead of brittle web automation.

Deliverables:

- Claude Teams personal-seat adapter
- Codex subscription adapter
- Reset-time normalization in the user's local timezone
- Remaining-percentage normalization for multiple concurrent allowance windows
- Provider diagnostics and reauthentication guidance
- Contract snapshots that detect upstream schema or presentation drift

Preferred fallback order:

1. Documented provider interface
2. Stable local application or CLI interface already authenticated by the user
3. Explicit manual refresh/import mode

Exit gate:

- Values match known samples for each provider.
- Each adapter refreshes independently.
- Source confidence and last successful refresh are always visible.
- Provider changes fail closed into an honest unavailable state, not fabricated data.

## Phase 4 — Product polish, preferences, and alerts

Turn the working collectors into a calm, glanceable daily tool.

Deliverables:

- Polished compact layout with provider name, remaining allowance, reset time, and freshness
- Expanded details and diagnostics
- Persisted non-secret preferences for refresh cadence, time format, and warning thresholds
- Optional local notifications at configurable thresholds
- Clear loading, stale, exhausted, signed-out, and unavailable states

Exit gate:

- Normal status can be understood at a glance.
- Detailed status requires no more than one interaction.
- Alerts are optional, rate-limited, and never depend on color alone.

## Phase 5 — Verification, documentation, and release readiness

Validate realistic conditions and prepare a reproducible first release.

Deliverables:

- Unit tests for normalization, reset calculations, cache behavior, and health transitions
- Adapter contract tests for all saved fixtures
- Integration tests for one-provider failure, offline use, expired authentication, and upstream drift
- Keyboard, screen-reader, contrast, reduced-motion, small-window, and 200% scaling checks
- Setup, privacy, troubleshooting, provider-limitations, and release documentation
- Linux packaging and release checklist

Exit gate:

- A clean installation can reach a useful status following the documented path.
- Critical status, failure, privacy, and accessibility paths pass validation.
- Release notes state which data sources are supported and which may change upstream.

## Daily-use desktop integration increment

Deliver the next bounded Linux release increment without expanding into team-wide analytics:

- Add a system-tray icon with Show, Hide, Refresh, and Quit actions.
- After a tray interaction confirms a recovery path, closing hides the window to the tray. Before that confirmation, closing exits rather than risking an inaccessible background process; explicit Quit always terminates.
- Add an opt-in `Start Sagewatch at login` preference backed by the desktop autostart facility. It must default off and surface failures honestly.
- Emit local desktop notifications for newly crossed allowance thresholds only when alerts are enabled; preserve the existing in-app accessible alert path and deduplication semantics.
- Produce a release AppImage using the existing Tauri bundle configuration and document installation, desktop integration, troubleshooting, and removal.
- Add a practical soak-test checklist covering startup, tray lifecycle, provider isolation, refresh cadence, threshold deduplication, suspend/resume, offline behavior, and restart persistence.

Exit gate:

- No hidden network listener, credential read, or provider-source expansion.
- Tray actions remain usable when the main window is hidden, and window close never strands an unquittable background process.
- Autostart is user-controlled, persisted, idempotent, and testable without silently changing login state during automated tests.
- Desktop-notification permission or delivery failure never blocks refresh, erases status, or duplicates threshold alerts.
- Keyboard and screen-reader behavior of existing in-app alerts remains intact.
- Frontend tests, bridge tests, Rust tests, Clippy, production build, and AppImage generation pass, or an environmental packaging blocker is documented with exact evidence.

## Scope boundaries

In scope for version 1:

- One user's Claude Teams seat and one Codex subscription
- Personal allowance windows, remaining percentage, reset time, freshness, and health
- Local Linux compact and expanded views
- Read-only collection, local caching, diagnostics, and optional local alerts

Out of scope:

- Account, billing, seat, or membership changes
- Cloud synchronization or remote telemetry
- Sharing credentials or subscription sessions
- Scraping that evades provider controls

Deferred:

- Team-wide Claude administrative analytics
- Multi-user and multi-account reporting
- Historical trend charts
- Native Waybar JSON module and additional providers
- Automated overage purchases or seat upgrades

## Dependencies and risks

- **Provider observability:** Codex may not expose a supported external allowance interface. Phase 0 is therefore a hard gate.
- **Different allowance semantics:** providers may expose several rolling windows or model-specific limits; normalization must preserve those distinctions.
- **Upstream drift:** adapters require contract fixtures, isolated failure, and explicit source confidence.
- **Authentication sensitivity:** reuse existing local authenticated surfaces where permitted; avoid collecting credentials directly.
- **Refresh pressure:** use conservative polling, backoff, jitter, and manual refresh.
- **Workspace isolation:** all project work stays beneath `/home/corye/openai-cli/Sagewatch`.

## Accessibility checklist

- Status is conveyed with text and shape as well as color.
- All controls work by keyboard and show visible focus.
- Compact content remains readable at 200% text scaling.
- Light, dark, and high-contrast themes retain sufficient contrast.
- Stale and error states do not require hover to understand.
- Reset times include an absolute local time; relative countdowns are supplementary.
- Motion is minimal and respects reduced-motion preferences.
- Live announcements are limited to meaningful status changes.

## Version 1 success criteria

- Both providers render independently from a shared normalized model.
- Remaining allowance and reset values match verified source samples.
- Stale, estimated, and unavailable data are unmistakable.
- The application stores no raw account password or reusable session secret in plaintext.
- The compact view is usable by keyboard and at 200% scaling.
- Setup and troubleshooting can be followed from a clean installation.

## Next release intent

The tray, autostart, local notification, and AppImage work above is the next bounded release increment. It is a desktop-integration follow-on, not a scope expansion into provider features or collaboration features.
