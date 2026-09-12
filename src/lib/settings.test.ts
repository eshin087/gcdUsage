import { describe, expect, it } from "vitest";
import type { AppSettings } from "./types";
import { mergeDockSettings } from "./settings";
const saved = {
  dockHidden: false,
  dockScale: 100,
  dockMinutes: 60,
  stripX: 10,
  stripY: 20,
  theme: "black",
} as AppSettings;
describe("native dock settings with unsaved dashboard edits", () => {
  it("retains tray hiding and corner sizing while editing another preference", () => {
    const next = mergeDockSettings({ ...saved, theme: "light" }, saved, {
      ...saved,
      dockHidden: true,
      dockScale: 126,
      stripX: 500,
    });
    expect(next).toMatchObject({
      theme: "light",
      dockHidden: true,
      dockScale: 126,
      stripX: 500,
    });
  });
  it("preserves explicit dashboard edits to dock controls", () => {
    const next = mergeDockSettings(
      { ...saved, dockScale: 160, dockMinutes: 1440 },
      saved,
      { ...saved, dockScale: 126, dockMinutes: 30, dockHidden: true },
    );
    expect(next).toMatchObject({
      dockScale: 160,
      dockMinutes: 1440,
      dockHidden: true,
    });
  });
});
