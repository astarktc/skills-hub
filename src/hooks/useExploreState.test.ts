// Tests for the Explore preview's intent (S8): a preview is a read that waits
// for a running action; when its turn comes it opens only if it is still the
// latest request and Explore is still showing. The backend is mocked at the
// invokeTauri seam; the reporter is the real one.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(async (command: string, _url: string, name: string) =>
    command === "cloneExploreSkill" ? `/cache/${name}` : null,
  ),
}));

import { invokeTauri } from "../lib/tauri";
import { useExploreState } from "./useExploreState";
import { useStatusReporter } from "./useStatusReporter";

const t = (key: string) => key;

function setup() {
  let area = "explore";
  const onOpenPreview = vi.fn();
  const rendered = renderHook(() => {
    const reporter = useStatusReporter(t);
    const explore = useExploreState({ t, reporter, onOpenPreview, isActive: () => area === "explore" });
    return { reporter, explore };
  });
  let release: () => void = () => {};
  let running: Promise<unknown> = Promise.resolve();
  act(() => {
    running = rendered.result.current.reporter.runAction({ label: "Refresh" }, () => new Promise<void>((resolve) => (release = resolve)));
  });
  return {
    ...rendered,
    onOpenPreview,
    navigate: (next: string) => {
      area = next;
    },
    settleMutation: async () => {
      release();
      await running;
    },
  };
}

beforeEach(() => vi.clearAllMocks());
afterEach(cleanup);

describe("the Explore preview's intent", () => {
  it("a preview queued behind a mutation opens once the slot frees", async () => {
    const s = setup();
    let preview: Promise<void> = Promise.resolve();
    act(() => {
      preview = s.result.current.explore.handleOpenExploreDetail("https://github.com/o/r", "alpha");
    });
    await act(async () => {
      await s.settleMutation();
      await preview;
    });
    expect(s.onOpenPreview).toHaveBeenCalledWith(expect.objectContaining({ name: "alpha", cachePath: "/cache/alpha" }));
  });

  it("navigating away while it waits drops it: no clone, no surface", async () => {
    const s = setup();
    let preview: Promise<void> = Promise.resolve();
    act(() => {
      preview = s.result.current.explore.handleOpenExploreDetail("https://github.com/o/r", "alpha");
    });
    s.navigate("skills");
    await act(async () => {
      await s.settleMutation();
      await preview;
    });
    expect(invokeTauri).not.toHaveBeenCalledWith("cloneExploreSkill", expect.anything(), expect.anything());
    expect(s.onOpenPreview).not.toHaveBeenCalled();
  });

  it("a newer preview request supersedes a waiting one", async () => {
    const s = setup();
    let first: Promise<void> = Promise.resolve();
    let second: Promise<void> = Promise.resolve();
    act(() => {
      first = s.result.current.explore.handleOpenExploreDetail("https://github.com/o/a", "alpha");
      second = s.result.current.explore.handleOpenExploreDetail("https://github.com/o/b", "beta");
    });
    await act(async () => {
      await s.settleMutation();
      await Promise.all([first, second]);
    });
    expect(s.onOpenPreview).toHaveBeenCalledTimes(1);
    expect(s.onOpenPreview).toHaveBeenCalledWith(expect.objectContaining({ name: "beta" }));
  });
});
