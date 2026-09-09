import { describe, expect, it } from "vitest";
import {
  buildHistoryFilter,
  countdown,
  emptyTokens,
  isStale,
  percent,
  quotaWindow,
  quotaPercent,
  totalTokens,
  uniqueModelStats,
} from "./format";
import type { ModelStat, QuotaSnapshot } from "./types";

describe("measured usage display", () => {
  it("keeps absent measurements distinct from measured zero", () => {
    expect(percent(null)).toBe("—");
    expect(percent(0)).toBe("0%");
    expect(totalTokens(emptyTokens())).toBeNull();
    expect(totalTokens({ ...emptyTokens(), input: 0 })).toBe(0);
  });
  it("defaults to remaining allowance without turning missing readings into full allowance", () => {
    expect(quotaPercent(0)).toBe(100);
    expect(quotaPercent(100)).toBe(0);
    expect(quotaPercent(62)).toBe(38);
    expect(quotaPercent(62, "used")).toBe(62);
    expect(quotaPercent(null)).toBeNull();
    expect(quotaPercent(Number.NaN)).toBeNull();
  });
  it("does not add reasoning tokens twice", () => {
    expect(
      totalTokens({
        input: 100,
        cacheRead: 200,
        cacheWrite: 300,
        output: 400,
        reasoning: 250,
      }),
    ).toBe(1000);
  });
  it("identifies weekly windows by duration, regardless of slot", () => {
    const snapshot = {
      windows: [
        { id: "primary", durationMinutes: 10080 },
        { id: "secondary", durationMinutes: 300 },
      ],
    } as QuotaSnapshot;
    expect(quotaWindow(snapshot, "weekly")?.id).toBe("primary");
    expect(quotaWindow(snapshot, "five_hour")?.id).toBe("secondary");
  });
  it("marks old and reset readings stale", () => {
    const window = {
      id: "week",
      label: "Weekly",
      durationMinutes: 10080,
      usedPercent: 0,
      resetsAt: 1000,
    };
    const snapshot = { status: "connected", fetchedAt: 900 } as QuotaSnapshot;
    expect(isStale(snapshot, window, 999)).toBe(false);
    expect(isStale(snapshot, window, 1000)).toBe(true);
    expect(isStale({ ...snapshot, fetchedAt: 100 }, window, 999)).toBe(true);
  });
  it("uses explicit labels for absent and elapsed resets", () => {
    expect(countdown(null, 100)).toBe("Reset N/A");
    expect(countdown(99, 100)).toBe("Awaiting reset");
    expect(countdown(161, 100)).toBe("Resets in 2m");
  });
});

describe("history boundaries", () => {
  it("includes the entire end date in local time and paginates in groups of 50", () => {
    const filter = buildHistoryFilter(
      {
        query: " example ",
        provider: "claude",
        model: "",
        effort: "",
        deviceId: "",
        from: "2026-09-01",
        to: "2026-09-02",
      },
      2,
    );
    expect(filter.query).toBe("example");
    expect(filter.from).toBe(new Date("2026-09-01T00:00:00").getTime() / 1000);
    expect(filter.to).toBe(
      new Date("2026-09-03T00:00:00").getTime() / 1000 - 1,
    );
    expect(filter.offset).toBe(100);
    expect(filter.model).toBeNull();
  });
});

describe("model accounting", () => {
  it("deduplicates repeated quota windows without combining different accounts", () => {
    const row = {
      provider: "claude",
      accountId: "account-one",
      model: "sonnet",
      effort: "high",
      totalTokens: 100,
    } as ModelStat;
    const rows = uniqueModelStats([
      { ...row, quotaWindowId: "five_hour" },
      { ...row, quotaWindowId: "weekly" },
      { ...row, accountId: "account-two", quotaWindowId: "weekly" },
    ]);
    expect(rows).toHaveLength(2);
    expect(rows.reduce((sum, item) => sum + item.totalTokens, 0)).toBe(200);
  });
});
