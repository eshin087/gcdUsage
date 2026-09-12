import { describe, it, expect } from "vitest";
import { coverageText, durationLabel, recentPercent } from "./recent-usage";
import type { RecentAllowanceWindow } from "./types";
describe("recent allowance presentation", () => {
  it("keeps unknown, zero, tiny changes and repeated windows distinct", () => {
    expect(recentPercent(null)).toBe("—");
    expect(recentPercent(NaN)).toBe("—");
    expect(recentPercent(0)).toBe("0%");
    expect(recentPercent(0.004)).toBe("<0.01%");
    expect(recentPercent(240)).toBe("240%");
  });
  it("explains coverage and interruptions independently of the observed total", () => {
    const row = {
      observedSeconds: 900,
      resetCount: 1,
      gapCount: 2,
    } as RecentAllowanceWindow;
    expect(coverageText(row, 60)).toContain("15m observed of 1h");
    expect(coverageText(row, 60)).toContain("1 reset/correction boundary");
    expect(coverageText(row, 60)).toContain("2 gaps");
    expect(durationLabel(90)).toBe("90m");
    expect(durationLabel(1440)).toBe("1d");
  });
});
