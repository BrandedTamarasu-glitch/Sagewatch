import type { Preferences, ProviderStatus, StatusSnapshot } from "./types";

export interface AllowanceAlert {
  id: string;
  message: string;
}

export interface StatusReconciliation {
  providers: ProviderStatus[];
  alerts: AllowanceAlert[];
}

export const refreshMilliseconds = (seconds: number): number => Math.max(30, seconds) * 1_000;

export async function deliverDesktopAlerts(
  alerts: AllowanceAlert[],
  notify: (title: string, body: string) => Promise<void>,
): Promise<boolean> {
  const results = await Promise.allSettled(
    alerts.map((alert) => notify("Sagewatch allowance alert", alert.message)),
  );
  return results.every((result) => result.status === "fulfilled");
}

export function reconcileProviderStatuses(
  current: ProviderStatus[],
  incoming: ProviderStatus[],
  preferences: Preferences,
  tracker: ThresholdAlertTracker,
): StatusReconciliation {
  const providers = [...current];
  const alerts: AllowanceAlert[] = [];
  for (const status of incoming) {
    const before = providers.find((provider) => provider.provider === status.provider);
    alerts.push(...tracker.evaluate(before, status, preferences));
    const index = providers.findIndex((provider) => provider.provider === status.provider);
    if (index === -1) providers.push(status);
    else providers[index] = status;
  }
  return { providers, alerts };
}

export function reconcileStatusSnapshot(
  current: ProviderStatus[],
  snapshot: StatusSnapshot,
  tracker: ThresholdAlertTracker,
): StatusReconciliation {
  const incoming = Object.values(snapshot.providers).flatMap((state) =>
    state?.status ? [state.status] : [],
  );
  return reconcileProviderStatuses(current, incoming, snapshot.preferences, tracker);
}

export class ThresholdAlertTracker {
  private previous = new Map<string, number>();
  private announced = new Set<string>();

  evaluate(before: ProviderStatus | undefined, after: ProviderStatus, preferences: Preferences): AllowanceAlert[] {
    if (!preferences.alerts_enabled) {
      this.remember(after);
      return [];
    }
    if (before) this.remember(before);
    const alerts: AllowanceAlert[] = [];
    for (const window of after.windows.filter((item) => item.is_active && item.remaining_percent != null)) {
      const windowKey = `${after.provider}:${window.id}`;
      const current = window.remaining_percent!;
      const previous = this.previous.get(windowKey);
      const thresholds = [...new Set([...preferences.alert_thresholds, 0])].sort((left, right) => right - left);
      for (const threshold of thresholds) {
        const key = `${windowKey}:${threshold}`;
        if (current > threshold) this.announced.delete(key);
        const crossed = previous != null && previous > threshold && current <= threshold;
        if (crossed && !this.announced.has(key)) {
          const providerName = after.provider === "claude" ? "Claude" : "Codex";
          const state = threshold === 0 ? "is exhausted" : `crossed ${threshold}% remaining`;
          alerts.push({ id: key, message: `${providerName} ${window.label} ${state}. ${Math.round(current)}% remains.` });
          this.announced.add(key);
        }
      }
      this.previous.set(windowKey, current);
    }
    return alerts;
  }

  private remember(status: ProviderStatus) {
    for (const window of status.windows) if (window.remaining_percent != null) this.previous.set(`${status.provider}:${window.id}`, window.remaining_percent);
  }
}
