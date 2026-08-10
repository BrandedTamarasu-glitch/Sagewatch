import type { AllowanceWindow, Diagnostics, Preferences, ProviderStatus } from "../lib/types";
import type { AllowanceAlert } from "../lib/alerts";

const escape = (value: string) => value.replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character]!);
const display = (value: string) => escape(value.replaceAll("_", " ").replace(/^./, (letter) => letter.toUpperCase()));
const name = (provider: ProviderStatus["provider"]) => provider === "claude" ? "Claude" : "Codex";
const percentage = (value: number | null | undefined) => value == null ? "Unknown" : `${Math.round(value)}%`;
const time = (value: string | null, format: Preferences["time_format"]) => {
  if (!value) return "Not available";
  const parsed = new Date(value);
  return Number.isNaN(parsed.valueOf()) ? "Not available" : new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short", hour12: format === "local_12_hour" }).format(parsed);
};

export const badge = (value: string, kind: string) => `<span class="status-badge status-badge--${kind} status-badge--${value}">${display(value)}</span>`;

export function alertList(alerts: AllowanceAlert[]): string {
  if (!alerts.length) return "";
  return `<section class="alerts" aria-label="Allowance alerts">${alerts.map((alert) => `<div class="alert"><p>${escape(alert.message)}</p><button type="button" class="icon-button" data-dismiss-alert="${escape(alert.id)}" aria-label="Dismiss alert: ${escape(alert.message)}">×</button></div>`).join("")}</section>`;
}

export function providerCard(status: ProviderStatus, preferences: Preferences, refreshing: boolean): string {
  const headline = status.windows.find((window) => window.id === status.headline_window_id);
  const providerName = name(status.provider);
  return `<article class="provider-card" aria-labelledby="${status.provider}-title">
    <header class="provider-card__header"><div><h2 id="${status.provider}-title">${providerName}</h2><p>${status.plan === "unknown" ? "Plan unknown" : escape(status.plan)}</p></div>${badge(status.health, "health")}</header>
    <div class="allowance" aria-label="${providerName} remaining allowance ${percentage(headline?.remaining_percent)}"><strong>${percentage(headline?.remaining_percent)}</strong><span>remaining${headline ? ` · ${escape(headline.label)}` : ""}</span></div>
    ${headline?.remaining_percent != null ? `<progress max="100" value="${headline.remaining_percent}" aria-label="${providerName} remaining percentage"></progress>` : ""}
    <dl class="provider-card__facts"><div><dt>Resets</dt><dd>${time(headline?.reset_at ?? null, preferences.time_format)}</dd></div><div><dt>Freshness</dt><dd>${badge(status.freshness, "freshness")}</dd></div><div><dt>Confidence</dt><dd>${badge(status.source_confidence, "confidence")}</dd></div></dl>
    <div class="provider-card__actions"><button type="button" class="button button--quiet" data-refresh="${status.provider}" ${refreshing ? "disabled" : ""} aria-label="Refresh ${providerName}">${refreshing ? "Refreshing…" : "Refresh"}</button><button type="button" class="button" data-details="${status.provider}" aria-haspopup="dialog">View details</button></div>
  </article>`;
}

function allowanceRow(window: AllowanceWindow, preferences: Preferences): string {
  return `<article class="window-row"><div><h4>${escape(window.label)}</h4><p>${display(window.kind)} · ${window.is_active ? "Active" : "Inactive"}</p></div><div class="window-row__amount"><strong>${percentage(window.remaining_percent)}</strong><span>remaining</span></div><p class="window-row__reset">Reset: ${time(window.reset_at, preferences.time_format)}</p></article>`;
}

export function detailsDialog(status: ProviderStatus, preferences: Preferences): string {
  const providerName = name(status.provider);
  return `<dialog class="detail-dialog" aria-labelledby="detail-title"><header class="dialog-header"><div><p class="eyebrow">Provider details</p><h2 id="detail-title">${providerName}</h2></div><button type="button" class="icon-button" data-close aria-label="Close ${providerName} details">×</button></header><div class="badge-row">${badge(status.health, "health")}${badge(status.freshness, "freshness")}${badge(status.source_confidence, "confidence")}</div><p class="source-note">Source: ${display(status.source)}. Observed ${time(status.observed_at, preferences.time_format)}.</p><section aria-labelledby="windows-title"><h3 id="windows-title">All allowance windows</h3><div class="window-list">${status.windows.length ? status.windows.map((window) => allowanceRow(window, preferences)).join("") : "<p>No allowance windows reported.</p>"}</div></section></dialog>`;
}

export function diagnosticsPanel(diagnostics: Diagnostics | null): string {
  const items = diagnostics ? Object.values(diagnostics).filter((item) => item != null) : [];
  return `<section class="panel" aria-labelledby="diagnostics-title"><h2 id="diagnostics-title">Diagnostics</h2>${items.length ? items.map((item) => `<article class="diagnostic"><h3>${item!.provider === "claude" ? "Claude" : "Codex"} · ${display(item!.health)}</h3><p>${escape(item!.summary)}</p><p>${item!.retryable ? "Retry is available." : "No automatic retry is recommended."}</p></article>`).join("") : "<p>No diagnostics are currently reported.</p>"}</section>`;
}

export function settingsPanel(preferences: Preferences, saving: boolean, autostartEnabled = false, autostartError = ""): string {
  const describedBy = autostartError ? ' aria-describedby="autostart-error"' : "";
  return `<section class="panel" aria-labelledby="settings-title"><h2 id="settings-title">Settings</h2><form class="settings" data-settings><label>Refresh interval<select name="refresh_interval_seconds"><option value="60" ${preferences.refresh_interval_seconds === 60 ? "selected" : ""}>Every minute</option><option value="300" ${preferences.refresh_interval_seconds === 300 ? "selected" : ""}>Every 5 minutes</option><option value="900" ${preferences.refresh_interval_seconds === 900 ? "selected" : ""}>Every 15 minutes</option></select></label><label>Time format<select name="time_format"><option value="local_12_hour" ${preferences.time_format === "local_12_hour" ? "selected" : ""}>12-hour</option><option value="local_24_hour" ${preferences.time_format === "local_24_hour" ? "selected" : ""}>24-hour</option></select></label><label>Alert thresholds (%)<input name="alert_thresholds" inputmode="numeric" aria-describedby="threshold-help" value="${preferences.alert_thresholds.join(", ")}"></label><p id="threshold-help" class="field-help">Comma-separated remaining percentages from 0 to 100, such as 20, 10.</p><label class="check-row"><input name="alerts_enabled" type="checkbox" ${preferences.alerts_enabled ? "checked" : ""}> Enable local allowance alerts</label><label class="check-row"><input name="autostart_enabled" type="checkbox" ${autostartEnabled ? "checked" : ""}${describedBy}> Start Sagewatch at login</label>${autostartError ? `<p id="autostart-error" class="field-help" role="status">${escape(autostartError)}</p>` : ""}<label class="check-row"><input name="claude_usage_probe_enabled" type="checkbox" ${preferences.claude_usage_probe_enabled ? "checked" : ""}> Enable experimental Claude /usage refresh</label><p class="field-help"><strong>Experimental privacy note:</strong> when enabled, Sagewatch launches your validated local Claude Code executable in a private working directory, sends /usage, Escape, and /exit, and trusts only the sanitized status-line snapshot. On the exact one-time Claude safety screen it may also press Enter to trust that empty private directory. Terminal output, prompts, transcripts, and credentials are not stored or logged. It is off by default.</p><label class="check-row"><input name="codex_rollout_fallback_enabled" type="checkbox" ${preferences.codex_rollout_fallback_enabled ? "checked" : ""}> Enable sensitive Codex rollout-cache fallback</label><p class="field-help"><strong>Privacy note:</strong> the rollout-cache fallback is opt-in and may read sensitive local cache data. It is off by default.</p><button class="button" type="submit" ${saving ? "disabled" : ""}>${saving ? "Saving…" : "Save settings"}</button></form></section>`;
}
