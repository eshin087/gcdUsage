import { describe, expect, it } from "vitest";
import { localDateTime, resolveRange } from "./time-range";
import { buildHistoryFilter } from "./format";

describe("time-range accounting boundaries", () => {
  it("uses exact rolling durations, without rounding to calendar days", () => {
    const now = new Date("2026-09-09T14:23:45").getTime() / 1000;
    for (const [preset, seconds] of [
      ["30m", 1800],
      ["1h", 3600],
      ["6h", 21600],
      ["24h", 86400],
      ["7d", 604800],
      ["30d", 2592000],
    ] as const) {
      expect(resolveRange(preset, now)).toEqual({
        from: now - seconds,
        to: now,
      });
    }
    expect(resolveRange("all", now)).toEqual({ from: null, to: now });
  });
  it("keeps calendar weeks and months distinct from rolling intervals", () => {
    const now = new Date("2026-09-09T14:23:45").getTime() / 1000;
    expect(resolveRange("week", now).from).toBe(
      new Date("2026-09-07T00:00:00").getTime() / 1000,
    );
    expect(resolveRange("month", now).from).toBe(
      new Date("2026-09-01T00:00:00").getTime() / 1000,
    );
    const sunday = new Date("2026-09-13T23:59:59").getTime() / 1000;
    expect(resolveRange("week", sunday).from).toBe(
      new Date("2026-09-07T00:00:00").getTime() / 1000,
    );
  });
  it("preserves custom seconds and rejects missing, invalid, and reversed ranges", () => {
    const from = "2026-09-01T14:12:34",
      to = "2026-09-01T14:42:35";
    const range = resolveRange("custom", 0, from, to);
    expect(localDateTime(range.from!)).toBe(from);
    expect(range.to - range.from!).toBe(1801);
    for (const [a, b] of [
      ["", to],
      ["bad", to],
      [to, from],
      [from, from],
      ["2026-02-30T12:00", "2026-03-05T12:00"],
    ])
      expect(() => resolveRange("custom", 0, a, b)).toThrow();
  });
  it("opens history at the same second boundaries without including the next interval", () => {
    const range = resolveRange(
      "custom",
      0,
      "2026-09-01T14:12:34",
      "2026-09-01T14:42:35",
    );
    const filter = buildHistoryFilter({
      query: "",
      provider: "",
      model: "",
      effort: "",
      deviceId: "",
      from: localDateTime(range.from!),
      to: localDateTime(range.to - 1),
    });
    expect(filter.from).toBe(range.from);
    expect(filter.to).toBe(range.to - 1);
  });
});
