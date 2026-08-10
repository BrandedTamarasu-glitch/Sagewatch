import { invoke } from "@tauri-apps/api/core";
import type { Diagnostics, Preferences, ProviderId, ProviderStatus, StatusSnapshot } from "./types";

export const getStatus = (): Promise<StatusSnapshot> => invoke("get_status");

export const refreshProvider = (provider: ProviderId): Promise<ProviderStatus> =>
  invoke("refresh_provider", { provider });

export const setPreferences = (preferences: Preferences): Promise<Preferences> =>
  invoke("set_preferences", { preferences });

export const setClaudeUsageProbeEnabled = (enabled: boolean): Promise<Preferences> =>
  invoke("set_claude_usage_probe_enabled", { enabled });

export const getDiagnostics = (): Promise<Diagnostics> => invoke("get_diagnostics");

export const getAutostartEnabled = (): Promise<boolean> => invoke("get_autostart_enabled");

export const setAutostartEnabled = (enabled: boolean): Promise<boolean> =>
  invoke("set_autostart_enabled", { enabled });

export const showDesktopNotification = (title: string, body: string): Promise<void> =>
  invoke("show_desktop_notification", { title, body });
