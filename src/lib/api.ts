import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  AppSettings,
  MetricsRange,
  UsageMetrics,
  HistoryFilter,
  HistoryPage,
  Overview,
  Provider,
  RecommendationSet,
  TaskClass,
  SignInProgress,
  BrowserFilter,
  BrowserPage,
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
  promptDetail: (id:string) => call<import('./PromptDetails.svelte').PromptDetail>('get_prompt_detail',{id}),
  openOriginal: (id:string) => call<boolean>('open_original_prompt',{id}),
  pendingPrompt: () => call<string|null>('take_pending_prompt'),
  overview: () => call<Overview>("get_overview"),
  metrics: (range: MetricsRange) =>
    call<UsageMetrics>("get_usage_metrics", { range }),
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
  reconnect: (provider: Provider, sso = false) =>
    call<void>("reconnect_provider", { provider, sso }),
  signIns: () => call<SignInProgress[]>("get_signin_status"),
  signInInput: (provider: Provider, code: string | null) =>
    call<void>("signin_input", { provider, code }),
  browserHistory: (filter: BrowserFilter) =>
    call<BrowserPage>("get_browser_history", { filter }),
  importBrowser: (accountLabel: string) =>
    call<{ added: number; duplicates: number; skipped: number } | null>(
      "import_browser_history",
      { accountLabel },
    ),
};
