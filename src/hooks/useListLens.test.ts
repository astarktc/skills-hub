// Tests for the list lens: query and sort through the pure fold, Grouped vs
// Flat with the optional singleton fold, view mode, persistence through each
// surface's own preference (the literal storage keys are a compat contract),
// and the selection's visible subset.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useListLens, type LensItem, type ListLensOptions } from "./useListLens";
import {
  groupByRepoPreference,
  projectsGroupByRepoPreference,
  viewModePreference,
} from "../lib/preferences";
import { INDIVIDUAL_GROUP_KEY, LOCAL_GROUP_KEY } from "../lib/skillPresentation";
import { focusAfter } from "../lib/actionLifecycle";

const item = (id: string, name: string, source_ref: string | null, updated_at = 0): LensItem => ({
  id,
  name,
  source_type: source_ref ? "git" : "local",
  source_ref,
  central_path: `/hub/${name}`,
  created_at: 0,
  updated_at,
});

const ITEMS = [
  item("1", "gamma", "https://github.com/acme/tools", 3),
  item("2", "alpha", "https://github.com/acme/tools", 1),
  item("3", "beta", "https://github.com/solo/one", 2),
  item("4", "delta", null, 4),
];
const labels = { local: "Local", ungrouped: "Ungrouped", individual: "Individual sources" };

function render(overrides: Partial<ListLensOptions<LensItem>> = {}) {
  return renderHook((props: Partial<ListLensOptions<LensItem>>) =>
    useListLens({ items: ITEMS, labels, groupByRepoPreference, viewModePreference, ...props }),
    { initialProps: overrides },
  );
}

// jsdom here provides no localStorage; stub the Web Storage interface in
// memory (as persistedPreference.test.ts does).
beforeEach(() => {
  const memory = new Map<string, string>();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => memory.get(key) ?? null,
      setItem: (key: string, value: string) => void memory.set(key, value),
      removeItem: (key: string) => void memory.delete(key),
      clear: () => memory.clear(),
    },
  });
});
afterEach(cleanup);

describe("useListLens", () => {
  it("filters by query and sorts, flat by default", () => {
    const { result } = render();
    expect(result.current.groups).toBeNull();
    expect(result.current.visible.map((s) => s.name)).toEqual(["alpha", "beta", "delta", "gamma"]);

    act(() => result.current.setSort("updated"));
    expect(result.current.visible.map((s) => s.name)).toEqual(["delta", "gamma", "beta", "alpha"]);

    act(() => result.current.setQuery("solo"));
    expect(result.current.visible.map((s) => s.name)).toEqual(["beta"]);
  });

  it("groups by repo, optionally folding single-skill sources", () => {
    const { result, rerender } = render();
    act(() => result.current.setGroupByRepo(true));
    expect(result.current.groups?.map((g) => g.label)).toEqual(["acme/tools", "solo/one", "Local"]);

    rerender({ foldSingletons: true });
    expect(result.current.groups?.map((g) => g.key)).toEqual([
      "acme/tools",
      INDIVIDUAL_GROUP_KEY,
      LOCAL_GROUP_KEY,
    ]);
    expect(result.current.groups?.[1].skills.map((s) => s.name)).toEqual(["beta"]);
  });

  it("persists Grouped/Flat and view mode under the surface's own keys", () => {
    const { result } = render();
    act(() => result.current.setGroupByRepo(true));
    act(() => result.current.setViewMode("dense-grid"));
    expect(window.localStorage.getItem("skills-groupByRepo")).toBe("true");
    expect(window.localStorage.getItem("skills-viewMode")).toBe("dense-grid");

    // A second surface (the matrix) keeps its own grouping and has no view mode.
    const matrix = renderHook(() =>
      useListLens({ items: ITEMS, labels, groupByRepoPreference: projectsGroupByRepoPreference }),
    );
    expect(matrix.result.current.groupByRepo).toBe(false);
    expect(matrix.result.current.viewMode).toBe("list");

    // A fresh instance reads the stored choice back.
    const again = render();
    expect(again.result.current.groupByRepo).toBe(true);
    expect(again.result.current.viewMode).toBe("dense-grid");
  });

  it("walks a grouped list in its display order, not the flat sort's (Delete's next row)", () => {
    const { result } = render({ selection: new Set(["1", "3"]) });
    // Flat: the sort's order.
    expect(result.current.displayed.map((s) => s.name)).toEqual(["alpha", "beta", "delta", "gamma"]);
    act(() => result.current.setGroupByRepo(true));
    // Grouped: acme/tools interleaves with solo/one — gamma now follows alpha.
    expect(result.current.displayed.map((s) => s.name)).toEqual(["alpha", "gamma", "beta", "delta"]);
    expect(result.current.visibleSelection).toEqual(["1", "3"]);
    // The focus rule reads that sequence: deleting alpha lands on gamma.
    const order = result.current.displayed.map((s) => s.id);
    expect(focusAfter({ verb: "delete", removed: true }, { skillId: "2", order, surface: "skills" })).toEqual({
      kind: "row",
      skillId: "1",
    });
  });

  it("answers the selection's visible subset in sort order", () => {
    const { result, rerender } = render({ selection: new Set(["1", "3", "gone"]) });
    expect(result.current.visibleSelection).toEqual(["3", "1"]);

    act(() => result.current.setQuery("gam"));
    expect(result.current.visibleSelection).toEqual(["1"]);

    rerender({ selection: undefined });
    expect(result.current.visibleSelection).toEqual([]);
  });

  it("applies a named filter (a chip) with the query, and counts each over the whole list", () => {
    const FILTERS = { local: (item: LensItem) => item.source_ref === null, acme: (item: LensItem) => Boolean(item.source_ref?.includes("acme")) };
    const { result } = render({ filters: FILTERS });
    expect(result.current.filter).toBe("all");
    expect(result.current.filterCounts).toEqual({ all: 4, local: 1, acme: 2 });

    act(() => result.current.setFilter("acme"));
    expect(result.current.visible.map((s) => s.name)).toEqual(["alpha", "gamma"]);
    act(() => result.current.setQuery("gam"));
    expect(result.current.visible.map((s) => s.name)).toEqual(["gamma"]);
    // The counts are the list's, not the query's.
    expect(result.current.filterCounts.acme).toBe(2);

    act(() => result.current.setFilter("all"));
    expect(result.current.visible.map((s) => s.name)).toEqual(["gamma"]);
  });
});
