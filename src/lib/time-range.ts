import type { MetricsRange } from "./types";
export const timePresets = [
  ["30m", "Last 30 minutes"],
  ["1h", "Last hour"],
  ["6h", "Last 6 hours"],
  ["24h", "Last 24 hours"],
  ["7d", "Last 7 days"],
  ["week", "This week"],
  ["30d", "Last 30 days"],
  ["month", "This month"],
  ["all", "All time"],
  ["custom", "Custom interval"],
] as const;
export type TimePreset = (typeof timePresets)[number][0];
export function localDateTime(timestamp: number): string {
  const date = new Date(timestamp * 1000);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}
export function resolveRange(
  preset: TimePreset,
  now: number,
  from = "",
  to = "",
): MetricsRange {
  const end = Math.floor(now);
  const seconds: Record<string, number> = {
    "30m": 1800,
    "1h": 3600,
    "6h": 21600,
    "24h": 86400,
    "7d": 604800,
    "30d": 2592000,
  };
  if (seconds[preset])
    return { from: Math.max(0, end - seconds[preset]), to: end };
  if (preset === "all") return { from: null, to: end };
  if (preset === "custom") {
    if (!from || !to) throw Error("Choose both a start and an end time.");
    const start = new Date(from).getTime() / 1000;
    const finish = new Date(to).getTime() / 1000;
    // Reject invalid and nonexistent local times (for example a spring DST gap).
    const matches = (text: string, value: number) =>
      Number.isFinite(value) && localDateTime(value).startsWith(text);
    if (!matches(from, start) || !matches(to, finish))
      throw Error("Choose valid local dates and times.");
    if (start < 0 || start >= finish)
      throw Error("The end time must be after the start time.");
    return { from: Math.floor(start), to: Math.floor(finish) };
  }
  const start = new Date(end * 1000);
  start.setHours(0, 0, 0, 0);
  if (preset === "week")
    start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
  else start.setDate(1);
  return {
    from: Math.floor(start.getTime() / 1000),
    to: Math.max(end, start.getTime() / 1000 + 1),
  };
}
export function rangeDescription(range: MetricsRange): string {
  const format = (value: number) =>
    new Date(value * 1000).toLocaleString(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    });
  return `${range.from == null ? "Beginning of history" : format(range.from)} → ${format(range.to)}`;
}
