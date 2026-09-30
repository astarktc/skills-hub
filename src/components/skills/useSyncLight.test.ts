// The strip's sync-completion light (ticket 15): a mark lights only when it
// becomes synced on screen, never when it mounts synced.

import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { StripMarkState } from "../../lib/skillPresentation";
import { useSyncLight } from "./useSyncLight";

const mount = (state: StripMarkState) =>
  renderHook(({ state: current }: { state: StripMarkState }) => useSyncLight(current), {
    initialProps: { state },
  });

describe("useSyncLight", () => {
  it("does not light a mark that mounts synced (first paint, a filter, a scroll)", () => {
    const { result, rerender } = mount("synced");
    expect(result.current).toBe(false);
    rerender({ state: "synced" });
    expect(result.current).toBe(false);
  });

  it.each<StripMarkState>(["not_synced", "failed", "missing", "broken", "stale", "disabled"])(
    "lights a mark that becomes synced from %s",
    (from) => {
      const { result, rerender } = mount(from);
      expect(result.current).toBe(false);
      rerender({ state: "synced" });
      expect(result.current).toBe(true);
      // Re-renders of the same state (progress ticks, a reload) keep it; its
      // starting style is inert after the glyph's first frame.
      rerender({ state: "synced" });
      expect(result.current).toBe(true);
    },
  );

  it("goes dark when the mark leaves synced, and lights again on the next sync", () => {
    const { result, rerender } = mount("not_synced");
    rerender({ state: "synced" });
    rerender({ state: "disabled" });
    expect(result.current).toBe(false);
    rerender({ state: "synced" });
    expect(result.current).toBe(true);
  });

  it("never lights a change that does not end synced", () => {
    const { result, rerender } = mount("synced");
    rerender({ state: "stale" });
    expect(result.current).toBe(false);
    rerender({ state: "failed" });
    expect(result.current).toBe(false);
  });
});
