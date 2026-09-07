import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  AppSettings,
  HistoryFilter,
  HistoryPage,
  Overview,
  Provider,
  RecommendationSet,
  TaskClass,
} from "./types";

export const native = isTauri();
export const preview =
  import.meta.env.DEV &&
  !native &&
  new URLSearchParams(location.search).get("preview") === "1";
async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (native) return invoke<T>(command, args);
  if (preview)
    return (await import("./preview")).previewInvoke<T>(command, args);
  throw new Error(
    "Open GCD Usage as a desktop app to connect your coding tools.",
  );
}
export const api = {
  overview: () => call<Overview>("get_overview"),
  history: (filter: HistoryFilter) =>
    call<HistoryPage>("get_history", { filter }),
  recommendations: (task: TaskClass) =>
    call<RecommendationSet>("get_recommendations", { task }),
  refresh: () => call<void>("refresh_usage"),
  importHistory: () => call<void>("import_history"),
  saveSettings: (settings: AppSettings) =>
    call<AppSettings>("save_settings", { settings }),
  selectSyncFolder: () => call<string | null>("select_sync_folder"),
  exportHistory: (filter: HistoryFilter) =>
    call<string | null>("export_history", { filter }),
  reconnect: (provider: Provider) =>
    call<void>("reconnect_provider", { provider }),
};
