// Settings → Updates' static-pane rule (ticket 20 audit F4): with no control
// to offer, the frame makes the pane itself the Tab stop.

import { describe, expect, it } from "vitest";
import type { AppUpdateStatus } from "../hooks/useAppUpdate";
import { updatesPaneHasControls } from "./settingsPanes";

describe("updatesPaneHasControls", () => {
  it.each<[AppUpdateStatus, boolean]>([
    ["idle", true],
    ["up-to-date", true],
    ["error", true],
    ["available", true],
    ["checking", false],
    ["installing", false],
    ["done", false],
  ])("in the app, %s → %s", (status, expected) => {
    expect(updatesPaneHasControls(status, true)).toBe(expected);
  });

  it("outside the app only a found release has controls", () => {
    expect(updatesPaneHasControls("idle", false)).toBe(false);
    expect(updatesPaneHasControls("available", false)).toBe(true);
  });
});
