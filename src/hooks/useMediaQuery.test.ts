// The media-query seam: the current answer, and a re-render when it changes.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SHELL_WIDE_QUERY, useMediaQuery } from "./useMediaQuery";

function stubMatchMedia(initial: boolean) {
  let matches = initial;
  const listeners = new Set<() => void>();
  vi.stubGlobal(
    "matchMedia",
    vi.fn((query: string) => ({
      get matches() {
        return matches;
      },
      media: query,
      addEventListener: (_: string, listener: () => void) => listeners.add(listener),
      removeEventListener: (_: string, listener: () => void) => listeners.delete(listener),
    })),
  );
  return {
    set(next: boolean) {
      matches = next;
      for (const listener of listeners) listener();
    },
    listeners,
  };
}

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("useMediaQuery", () => {
  it("answers the query now and follows its changes", () => {
    const media = stubMatchMedia(true);
    const { result, unmount } = renderHook(() => useMediaQuery(SHELL_WIDE_QUERY));
    expect(result.current).toBe(true);
    act(() => media.set(false));
    expect(result.current).toBe(false);
    unmount();
    expect(media.listeners.size).toBe(0);
  });

  it("is the shell breakpoint's 1200 px (75rem)", () => {
    expect(SHELL_WIDE_QUERY).toBe("(min-width: 75rem)");
  });
});
