import type { AppSettings } from "./types";

// Native dock controls can change saved settings while the dashboard has an
// unrelated unsaved edit. Preserve only fields the user actually changed here.
export function mergeDockSettings(
  draft: AppSettings,
  baseline: AppSettings,
  incoming: AppSettings,
): AppSettings {
  return {
    ...draft,
    dockHidden:
      draft.dockHidden === baseline.dockHidden
        ? incoming.dockHidden
        : draft.dockHidden,
    dockScale:
      draft.dockScale === baseline.dockScale
        ? incoming.dockScale
        : draft.dockScale,
    dockMinutes:
      draft.dockMinutes === baseline.dockMinutes
        ? incoming.dockMinutes
        : draft.dockMinutes,
    stripX: incoming.stripX,
    stripY: incoming.stripY,
  };
}
