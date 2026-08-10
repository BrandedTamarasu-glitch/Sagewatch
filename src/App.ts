import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as api from "./lib/api";
import { fixtureDiagnostics, fixturePreferences, unavailableProviders } from "./fixtures/status";
import type { Diagnostics, Preferences, ProviderId, ProviderStatus, StatusSnapshot } from "./lib/types";
import { alertList, detailsDialog, diagnosticsPanel, providerCard, settingsPanel } from "./components/render";
import { deliverDesktopAlerts, reconcileProviderStatuses, reconcileStatusSnapshot, refreshMilliseconds, ThresholdAlertTracker, type AllowanceAlert } from "./lib/alerts";

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
  private autostartEnabled = false;
  private autostartError = "";
  private interval: number | null = null;
  private inFlight = new Set<ProviderId>();
  private unlistenStatusUpdated: UnlistenFn | null = null;
  private destroyed = false;

  constructor(private readonly root: HTMLElement) {
    this.root.addEventListener("click", this.onClick);
    this.root.addEventListener("submit", this.onSubmit);
    document.addEventListener("visibilitychange", this.onVisibilityChange);
    window.addEventListener("pagehide", this.destroy, { once: true });
  }

  async start() {
    this.render();
    try {
      const unlisten = await listen<StatusSnapshot>("sagewatch://status-updated", (event) => {
        this.applySnapshot(event.payload);
      });
      if (this.destroyed) unlisten();
      else this.unlistenStatusUpdated = unlisten;
    }
    catch {
      this.notice = "Tray refresh updates are unavailable. In-app refresh remains available.";
    }
    const [statusResult, autostartResult] = await Promise.allSettled([
      api.getStatus(),
      api.getAutostartEnabled(),
    ]);
    if (statusResult.status === "fulfilled") {
      const snapshot = statusResult.value;
      const live = Object.values(snapshot.providers).flatMap((state) => state?.status ? [state.status] : []);
      this.providers = this.providers.map((placeholder) => live.find((status) => status.provider === placeholder.provider) ?? placeholder);
      this.preferences = snapshot.preferences;
      this.autostartEnabled = snapshot.preferences.start_at_login;
      this.notice = live.length ? "Usage status loaded." : "No saved usage is available yet.";
    }
    else { this.notice = "Live status is unavailable."; }
    if (autostartResult.status === "fulfilled") {
      this.autostartEnabled = autostartResult.value;
      this.autostartError = "";
    }
    else {
      this.autostartError = "Start-at-login status is unavailable. No login setting was changed.";
    }
    this.render();
    this.scheduleRefresh();
    void this.refreshAll();
  }

  private render() {
    const content = this.tab === "usage" ? `<section class="provider-grid" aria-label="Provider allowance status">${this.providers.map((status) => providerCard(status, this.preferences, this.refreshing === status.provider)).join("")}</section>` : this.tab === "diagnostics" ? diagnosticsPanel(this.diagnostics) : settingsPanel(this.preferences, this.saving, this.autostartEnabled, this.autostartError);
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
      const status = await api.refreshProvider(provider);
      this.applyProviderStatuses([status], this.preferences, false);
      this.notice = `${provider === "claude" ? "Claude" : "Codex"} refreshed.`;
    }
    catch { this.notice = `${provider === "claude" ? "Claude" : "Codex"} could not refresh. Last known status is still shown.`; }
    this.inFlight.delete(provider); this.refreshing = this.inFlight.values().next().value ?? null; this.render();
  }

  private onSubmit = async (event: Event) => {
    const form = (event.target as Element).closest<HTMLFormElement>("[data-settings]"); if (!form) return; event.preventDefault();
    const data = new FormData(form); const thresholds = String(data.get("alert_thresholds") ?? "").split(",").map(Number).filter((value) => Number.isFinite(value) && value >= 0 && value <= 100);
    const next: Preferences = { ...this.preferences, refresh_interval_seconds: Number(data.get("refresh_interval_seconds")), time_format: data.get("time_format") as Preferences["time_format"], alert_thresholds: thresholds, alerts_enabled: data.get("alerts_enabled") === "on", start_at_login: this.autostartEnabled, codex_rollout_fallback_enabled: data.get("codex_rollout_fallback_enabled") === "on" };
    const requestedAutostart = data.get("autostart_enabled") === "on";
    this.saving = true; this.render();
    const preferencesResult = await Promise.allSettled([api.setPreferences(next)]).then(([result]) => result);
    if (preferencesResult.status === "fulfilled") { this.preferences = preferencesResult.value; this.scheduleRefresh(); }
    const autostartResult = await Promise.allSettled([
      requestedAutostart === this.autostartEnabled ? Promise.resolve(this.autostartEnabled) : api.setAutostartEnabled(requestedAutostart),
    ]).then(([result]) => result);
    if (autostartResult.status === "fulfilled") {
      this.autostartEnabled = autostartResult.value;
      this.preferences = { ...this.preferences, start_at_login: autostartResult.value };
      this.autostartError = "";
    }
    else { this.autostartError = "Start at login could not be changed. The previous setting is still in effect."; }
    this.notice = preferencesResult.status === "fulfilled" && autostartResult.status === "fulfilled" ? "Settings saved." : "Some settings could not be saved. Review the message in Settings.";
    this.saving = false; this.render();
  };

  private async deliverDesktopAlerts(alerts: AllowanceAlert[]) {
    const delivered = await deliverDesktopAlerts(alerts, api.showDesktopNotification);
    if (!delivered) {
      this.notice = "A desktop notification could not be delivered. The in-app alert remains available.";
      this.render();
    }
  }

  private applySnapshot(snapshot: StatusSnapshot) {
    const incoming = Object.values(snapshot.providers).flatMap((state) => state?.status ? [state.status] : []);
    this.preferences = snapshot.preferences;
    this.autostartEnabled = snapshot.preferences.start_at_login;
    const reconciliation = reconcileStatusSnapshot(this.providers, snapshot, this.alertTracker);
    this.applyReconciliation(reconciliation.providers, reconciliation.alerts, false);
    this.notice = incoming.length ? "Usage status updated from the tray." : "Tray refresh completed with no provider status.";
    this.render();
  }

  private applyProviderStatuses(incoming: ProviderStatus[], preferences: Preferences, render = true) {
    const reconciliation = reconcileProviderStatuses(this.providers, incoming, preferences, this.alertTracker);
    this.applyReconciliation(reconciliation.providers, reconciliation.alerts, render);
  }

  private applyReconciliation(providers: ProviderStatus[], alerts: AllowanceAlert[], render: boolean) {
    this.providers = providers;
    const newAlerts = alerts.filter((alert) => !this.alerts.some((current) => current.id === alert.id));
    this.alerts.push(...newAlerts);
    if (newAlerts.length) {
      this.alertAnnouncement = newAlerts.map((alert) => alert.message).join(" ");
      void this.deliverDesktopAlerts(newAlerts);
    }
    if (render) this.render();
  }

  private scheduleRefresh() {
    if (this.interval != null) window.clearInterval(this.interval);
    this.interval = null;
    if (document.visibilityState === "hidden") return;
    this.interval = window.setInterval(() => { void this.refreshAll(); }, refreshMilliseconds(this.preferences.refresh_interval_seconds));
  }

  private refreshAll() {
    return Promise.allSettled((["claude", "codex"] as ProviderId[]).map((provider) => this.refresh(provider)));
  }

  private onVisibilityChange = () => {
    this.scheduleRefresh();
    if (!document.hidden) void this.refreshAll();
  };

  destroy = () => {
    this.destroyed = true;
    this.unlistenStatusUpdated?.();
    this.unlistenStatusUpdated = null;
    if (this.interval != null) window.clearInterval(this.interval);
    this.interval = null;
    document.removeEventListener("visibilitychange", this.onVisibilityChange);
    this.root.removeEventListener("click", this.onClick);
    this.root.removeEventListener("submit", this.onSubmit);
  };
}
