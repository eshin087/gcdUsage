import type {
  HistoryFilter,
  MeterDisplay,
  ModelStat,
  Provider,
  QuotaSnapshot,
  QuotaWindow,
  TokenUsage,
} from "./types";

const compact = new Intl.NumberFormat(undefined, {
  notation: "compact",
  maximumFractionDigits: 1,
});
const full = new Intl.NumberFormat();
export const count = (value: number | null | undefined): string =>
  value == null || !Number.isFinite(value) ? "—" : compact.format(value);
export const exactCount = (value: number | null | undefined): string =>
  value == null || !Number.isFinite(value) ? "—" : full.format(value);
export const percent = (value: number | null | undefined): string =>
  value == null || !Number.isFinite(value) ? "—" : `${Math.round(value)}%`;
export function quotaPercent(
  used: number | null | undefined,
  display: MeterDisplay = "remaining",
): number | null {
  if (used == null || !Number.isFinite(used)) return null;
  const clamped = Math.min(100, Math.max(0, used));
  return display === "remaining" ? 100 - clamped : clamped;
}
export const providerName = (provider: Provider): string =>
  provider === "claude" ? "Claude" : "Codex";
export function totalTokens(tokens: TokenUsage): number | null {
  const values = [
    tokens.input,
    tokens.cacheRead,
    tokens.cacheWrite,
    tokens.output,
  ];
  return values.every((value) => value == null)
    ? null
    : values.reduce<number>((sum, value) => sum + (value ?? 0), 0);
}
export function countdown(
  resetsAt: number | null | undefined,
  now: number,
): string {
  if (resetsAt == null || !Number.isFinite(resetsAt)) return "Reset N/A";
  const seconds = Math.max(0, Math.ceil(resetsAt - now));
  if (!seconds) return "Awaiting reset";
  const minutes = Math.ceil(seconds / 60);
  const days = Math.floor(minutes / 1440);
  const hours = Math.floor((minutes % 1440) / 60);
  return days
    ? `Resets in ${days}d ${hours}h`
    : hours
      ? `Resets in ${hours}h ${minutes % 60}m`
      : `Resets in ${minutes}m`;
}
export function relativeTime(
  timestamp: number | null | undefined,
  now: number,
): string {
  if (timestamp == null || timestamp <= 0) return "Not yet";
  const delta = Math.max(0, now - timestamp);
  if (delta < 60) return "Just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
  return `${Math.floor(delta / 86400)}d ago`;
}
export const dateTime = (timestamp: number): string =>
  new Date(timestamp * 1000).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
export const shortDate = (date: string): string =>
  new Date(`${date}T12:00:00`).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
export function quotaWindow(
  snapshot: QuotaSnapshot | undefined,
  period: "five_hour" | "weekly",
): QuotaWindow | undefined {
  return snapshot?.windows.find((window) =>
    period === "five_hour"
      ? window.durationMinutes === 300
      : window.durationMinutes >= 8640 && window.durationMinutes <= 11520,
  );
}
export function isStale(
  snapshot: QuotaSnapshot | undefined,
  window: QuotaWindow | undefined,
  now: number,
): boolean {
  return (
    !!window &&
    (!snapshot ||
      snapshot.status !== "connected" ||
      now - snapshot.fetchedAt > 300 ||
      (window.resetsAt != null && window.resetsAt <= now))
  );
}
export interface FilterInputs {
  query: string;
  provider: "" | Provider;
  model: string;
  effort: string;
  deviceId: string;
  from: string;
  to: string;
}
export function buildHistoryFilter(
  inputs: FilterInputs,
  page = 0,
): HistoryFilter {
  const dateEpoch = (value: string, end: boolean): number | null => {
    if (!value) return null;
    if (value.includes("T")) {
      const exact = new Date(value).getTime() / 1000;
      return Number.isFinite(exact) ? Math.floor(exact) : null;
    }
    const date = new Date(`${value}T00:00:00`);
    if (!Number.isFinite(date.getTime())) return null;
    if (end) date.setDate(date.getDate() + 1);
    return Math.floor(date.getTime() / 1000) - (end ? 1 : 0);
  };
  return {
    query: inputs.query.trim() || null,
    provider: inputs.provider || null,
    model: inputs.model || null,
    effort: inputs.effort || null,
    deviceId: inputs.deviceId || null,
    from: dateEpoch(inputs.from, false),
    to: dateEpoch(inputs.to, true),
    limit: 50,
    offset: Math.max(0, page) * 50,
  };
}
export const emptyTokens = (): TokenUsage => ({
  input: null,
  cacheRead: null,
  cacheWrite: null,
  output: null,
  reasoning: null,
});

// Quota calibration can repeat the same measured totals for several quota windows.
export function uniqueModelStats(stats: ModelStat[]): ModelStat[] {
  const seen = new Set<string>();
  return stats.filter((stat) => {
    const key = JSON.stringify([
      stat.provider,
      stat.accountId ?? null,
      stat.model,
      stat.effort,
    ]);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
