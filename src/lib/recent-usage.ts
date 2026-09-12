import type { RecentAllowanceWindow } from "./types";
export function durationLabel(minutes: number): string {
  if (minutes === 0) return "0m";
  if (minutes % 1440 === 0) return minutes / 1440 + "d";
  if (minutes % 60 === 0) return minutes / 60 + "h";
  return minutes + "m";
}
export function recentPercent(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value) || value < 0) return "—";
  if (value > 0 && value < 0.01) return "<0.01%";
  return (
    new Intl.NumberFormat(undefined, { maximumFractionDigits: 2 }).format(
      value,
    ) + "%"
  );
}
export function coverageText(
  row: RecentAllowanceWindow,
  minutes: number,
): string {
  const coverage =
    durationLabel(Math.max(0, Math.round(row.observedSeconds / 60))) +
    " observed of " +
    durationLabel(minutes);
  const interruptions = [
    row.resetCount
      ? row.resetCount +
        " reset/correction " +
        (row.resetCount === 1 ? "boundary" : "boundaries")
      : "",
    row.gapCount
      ? row.gapCount + " " + (row.gapCount === 1 ? "gap" : "gaps")
      : "",
  ]
    .filter(Boolean)
    .join(" · ");
  return coverage + (interruptions ? " · " + interruptions : "");
}
