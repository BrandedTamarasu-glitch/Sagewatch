import { invoke } from "@tauri-apps/api/core";
import type { Diagnostics, Preferences, ProviderId, ProviderStatus, StatusSnapshot } from "./types";

export const getStatus = (): Promise<StatusSnapshot> => invoke("get_status");

export const refreshProvider = (provider: ProviderId): Promise<ProviderStatus> =>
  invoke("refresh_provider", { provider });

export const setPreferences = (preferences: Preferences): Promise<Preferences> =>
  invoke("set_preferences", { preferences });

export const getDiagnostics = (): Promise<Diagnostics> => invoke("get_diagnostics");
