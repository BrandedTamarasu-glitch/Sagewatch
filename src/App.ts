import * as api from "./lib/api";
import { fixtureDiagnostics, fixturePreferences, unavailableProviders } from "./fixtures/status";
import type { Diagnostics, Preferences, ProviderId, ProviderStatus } from "./lib/types";
import { alertList, detailsDialog, diagnosticsPanel, providerCard, settingsPanel } from "./components/render";
import { refreshMilliseconds, ThresholdAlertTracker, type AllowanceAlert } from "./lib/alerts";

type Tab = "usage" | "diagnostics" | "settings";

export class SagewatchApp {
  private providers: ProviderStatus[] = unavailableProviders;
  private preferences: Preferences = fixturePreferences;
  private diagnostics: Diagnostics | null = fixtureDiagnostics;
  private tab: Tab = "usage";
  private selected: ProviderId | null = null;
  private refreshing: ProviderId | null = null;
  private saving = false;
  private focusAfterRender: string | null = null;
  private notice = "Connecting to local provider sources.";
  private alertAnnouncement = "";
  private alerts: AllowanceAlert[] = [];
  private alertTracker = new ThresholdAlertTracker();
  private interval: number | null = null;
  private inFlight = new Set<ProviderId>();

  constructor(private readonly root: HTMLElement) {
    this.root.addEventListener("click", this.onClick);
    this.root.addEventListener("submit", this.onSubmit);
    document.addEventListener("visibilitychange", this.onVisibilityChange);
    window.addEventListener("pagehide", this.destroy, { once: true });
  }

  async start() {
    this.render();
    try {
      const snapshot = await api.getStatus();
      const live = Object.values(snapshot.providers).flatMap((state) => state?.status ? [state.status] : []);
      this.providers = this.providers.map((placeholder) => live.find((status) => status.provider === placeholder.provider) ?? placeholder);
      this.preferences = snapshot.preferences;
      this.notice = live.length ? "Usage status loaded." : "No saved usage is available yet.";
    }
    catch { this.notice = "Live status is unavailable."; }
    this.render();
    this.scheduleRefresh();
  }

  private render() {
    const content = this.tab === "usage" ? `<section class="provider-grid" aria-label="Provider allowance status">${this.providers.map((status) => providerCard(status, this.preferences, this.refreshing === status.provider)).join("")}</section>` : this.tab === "diagnostics" ? diagnosticsPanel(this.diagnostics) : settingsPanel(this.preferences, this.saving);
    const selectedStatus = this.providers.find((provider) => provider.provider === this.selected);
    this.root.innerHTML = `<main class="shell"><header class="app-header"><div><p class="eyebrow">Local allowance monitor</p><h1>Sagewatch</h1></div><span class="privacy-mark">Local only</span></header><nav class="tabs" aria-label="Sagewatch sections">${(["usage", "diagnostics", "settings"] as Tab[]).map((tab) => `<button type="button" data-tab="${tab}" ${this.tab === tab ? 'aria-current="page"' : ""}>${tab[0].toUpperCase()}${tab.slice(1)}</button>`).join("")}</nav><p class="sr-only" role="status" aria-live="polite">${this.notice}</p><p class="sr-only" role="status" aria-live="assertive">${this.alertAnnouncement}</p>${alertList(this.alerts)}${content}${selectedStatus ? detailsDialog(selectedStatus, this.preferences) : ""}</main>`;
    this.alertAnnouncement = "";
    const dialog = this.root.querySelector<HTMLDialogElement>("dialog");
    if (dialog) { dialog.addEventListener("cancel", (event) => { event.preventDefault(); this.closeDialog(); }); dialog.showModal(); dialog.querySelector<HTMLButtonElement>("[data-close]")?.focus(); }
    else if (this.focusAfterRender) { this.root.querySelector<HTMLElement>(this.focusAfterRender)?.focus(); this.focusAfterRender = null; }
  }

  private onClick = async (event: Event) => {
    const button = (event.target as Element).closest<HTMLButtonElement>("button"); if (!button) return;
    const tab = button.dataset.tab as Tab | undefined;
    if (tab) { this.tab = tab; this.selected = null; if (tab === "diagnostics") { try { this.diagnostics = await api.getDiagnostics(); } catch { this.notice = "Diagnostics could not be loaded."; } } this.render(); return; }
    if (button.dataset.close !== undefined) { this.closeDialog(); return; }
    const alertId = button.dataset.dismissAlert; if (alertId) { this.alerts = this.alerts.filter((alert) => alert.id !== alertId); this.render(); return; }
    const details = button.dataset.details as ProviderId | undefined; if (details) { this.selected = details; this.render(); return; }
    const provider = button.dataset.refresh as ProviderId | undefined; if (provider) await this.refresh(provider);
  };

  private closeDialog() { const provider = this.selected; this.selected = null; this.focusAfterRender = provider ? `[data-details="${provider}"]` : null; this.render(); }

  private async refresh(provider: ProviderId) {
    if (this.inFlight.has(provider)) return;
    this.inFlight.add(provider); this.refreshing = provider; this.notice = `Refreshing ${provider}…`; this.render();
    try {
      const before = this.providers.find((item) => item.provider === provider);
      const status = await api.refreshProvider(provider);
      const nextAlerts = this.alertTracker.evaluate(before, status, this.preferences);
      this.providers = this.providers.map((item) => item.provider === provider ? status : item);
      this.alerts.push(...nextAlerts.filter((alert) => !this.alerts.some((current) => current.id === alert.id)));
      if (nextAlerts.length) this.alertAnnouncement = nextAlerts.map((alert) => alert.message).join(" ");
      this.notice = `${provider === "claude" ? "Claude" : "Codex"} refreshed.`;
    }
    catch { this.notice = `${provider === "claude" ? "Claude" : "Codex"} could not refresh. Last known status is still shown.`; }
    this.inFlight.delete(provider); this.refreshing = this.inFlight.values().next().value ?? null; this.render();
  }

  private onSubmit = async (event: Event) => {
    const form = (event.target as Element).closest<HTMLFormElement>("[data-settings]"); if (!form) return; event.preventDefault();
    const data = new FormData(form); const thresholds = String(data.get("alert_thresholds") ?? "").split(",").map(Number).filter((value) => Number.isFinite(value) && value >= 0 && value <= 100);
    const next: Preferences = { ...this.preferences, refresh_interval_seconds: Number(data.get("refresh_interval_seconds")), time_format: data.get("time_format") as Preferences["time_format"], alert_thresholds: thresholds, alerts_enabled: data.get("alerts_enabled") === "on", codex_rollout_fallback_enabled: data.get("codex_rollout_fallback_enabled") === "on" };
    this.saving = true; this.render(); try { this.preferences = await api.setPreferences(next); this.notice = "Settings saved."; this.scheduleRefresh(); } catch { this.notice = "Settings could not be saved."; } this.saving = false; this.render();
  };

  private scheduleRefresh() {
    if (this.interval != null) window.clearInterval(this.interval);
    this.interval = null;
    if (document.visibilityState === "hidden") return;
    this.interval = window.setInterval(() => { void Promise.allSettled((["claude", "codex"] as ProviderId[]).map((provider) => this.refresh(provider))); }, refreshMilliseconds(this.preferences.refresh_interval_seconds));
  }

  private onVisibilityChange = () => this.scheduleRefresh();

  destroy = () => {
    if (this.interval != null) window.clearInterval(this.interval);
    this.interval = null;
    document.removeEventListener("visibilitychange", this.onVisibilityChange);
    this.root.removeEventListener("click", this.onClick);
    this.root.removeEventListener("submit", this.onSubmit);
  };
}
