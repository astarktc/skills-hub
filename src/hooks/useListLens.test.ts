// Tests for the list lens: query and sort through the pure fold, Grouped vs
// Flat with the optional singleton fold, view mode, persistence through each
// surface's own preference (the literal storage keys are a compat contract),
// and the selection's visible subset.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useListLens, type LensItem, type ListLensOptions } from "./useListLens";
import {
  collapsedSourcesPreference,
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
const labels = { local: "Local", managed: "Managed here", ungrouped: "Ungrouped", individual: "Individual sources" };

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

  // BACKLOG #72: Problems first reads the surface's rank, flat and grouped.
  it("sorts Problems first by the surface's rank, groups by their loudest row", () => {
    const rank: Record<string, number> = { beta: 2, delta: 1 };
    const { result } = render({ problemRank: (row) => rank[row.name] ?? 0 });
    act(() => result.current.setSort("issues"));
    expect(result.current.visible.map((s) => s.name)).toEqual(["beta", "delta", "alpha", "gamma"]);
    act(() => result.current.setGroupByRepo(true));
    expect(result.current.groups?.map((g) => g.label)).toEqual(["solo/one", "Local", "acme/tools"]);
    // Another sort keeps the groups' own order.
    act(() => result.current.setSort("name"));
    expect(result.current.groups?.map((g) => g.label)).toEqual(["acme/tools", "solo/one", "Local"]);
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

// Ticket 31 (D42): grouping is decided on the unfiltered catalog, and groups
// collapse — persisted per source key, expanded for the duration of a query
// or chip, restored when it clears.
describe("useListLens groups (ticket 31)", () => {
  it("keeps a filtered singleton of a multi-skill repo under its repo header", () => {
    const { result } = render({ foldSingletons: true });
    act(() => result.current.setGroupByRepo(true));
    act(() => result.current.setQuery("gam"));
    expect(result.current.groups?.map((g) => [g.key, g.skills.map((s) => s.name)])).toEqual([
      ["acme/tools", ["gamma"]],
    ]);
  });

  it("collapses a group, persists it per source key, and reads it back", () => {
    const { result } = render({ collapsePreference: collapsedSourcesPreference });
    act(() => result.current.setGroupByRepo(true));
    expect(result.current.collapsed.size).toBe(0);
    act(() => result.current.toggleCollapsed("acme/tools"));
    expect([...result.current.collapsed]).toEqual(["acme/tools"]);
    expect(window.localStorage.getItem("skills-collapsedSources")).toBe('["acme/tools"]');
    // A collapsed group's rows stay in the lens (selected and counted), but
    // are not on screen (keyboard order, the focus rule, the name column).
    expect(result.current.displayed.map((s) => s.name)).toContain("gamma");
    expect(result.current.onScreen.map((s) => s.name)).toEqual(["beta", "delta"]);

    const again = render({ collapsePreference: collapsedSourcesPreference });
    expect([...again.result.current.collapsed]).toEqual(["acme/tools"]);
    act(() => again.result.current.toggleCollapsed("acme/tools"));
    expect(again.result.current.collapsed.size).toBe(0);
    expect(window.localStorage.getItem("skills-collapsedSources")).toBe("[]");
  });

  it("Collapse all / Expand all act on every group shown", () => {
    const { result } = render({ collapsePreference: collapsedSourcesPreference });
    act(() => result.current.setGroupByRepo(true));
    act(() => result.current.collapseAll());
    expect([...result.current.collapsed].sort()).toEqual(["__local__", "acme/tools", "solo/one"]);
    act(() => result.current.expandAll());
    expect(result.current.collapsed.size).toBe(0);
  });

  it("a query or chip expands every group for its duration, then restores the stored state", () => {
    const FILTERS = { acme: (item: LensItem) => Boolean(item.source_ref?.includes("acme")) };
    const { result } = render({ collapsePreference: collapsedSourcesPreference, filters: FILTERS });
    act(() => result.current.setGroupByRepo(true));
    act(() => result.current.toggleCollapsed("acme/tools"));

    act(() => result.current.setQuery("gam"));
    expect(result.current.collapsed.size).toBe(0);
    // A collapse during the query is the query's own; the stored state is untouched.
    act(() => result.current.toggleCollapsed("acme/tools"));
    expect([...result.current.collapsed]).toEqual(["acme/tools"]);
    act(() => result.current.toggleCollapsed("acme/tools"));
    act(() => result.current.setQuery(""));
    expect([...result.current.collapsed]).toEqual(["acme/tools"]);

    act(() => result.current.setFilter("acme"));
    expect(result.current.collapsed.size).toBe(0);
    act(() => result.current.setFilter("all"));
    expect([...result.current.collapsed]).toEqual(["acme/tools"]);
    expect(window.localStorage.getItem("skills-collapsedSources")).toBe('["acme/tools"]');
  });

  it("a surface without a collapse preference never collapses", () => {
    const { result } = render();
    act(() => result.current.setGroupByRepo(true));
    act(() => result.current.toggleCollapsed("acme/tools"));
    expect(result.current.collapsed.size).toBe(0);
  });
});

// A surface whose rows are not skill-shaped (Explore, ticket 22) brings its
// own query fold, may own the query (it drives a fetch), opens on a chip and
// has no Grouped/Flat choice.
describe("useListLens with a surface fold", () => {
  type Row = { id: string; name: string; source_ref: string | null; online: boolean };
  const ROWS: Row[] = [
    { id: "a", name: "zeta", source_ref: null, online: false },
    { id: "b", name: "alpha", source_ref: null, online: true },
    { id: "c", name: "beta", source_ref: null, online: false },
  ];
  const keepOnlineOrMatch = (items: readonly Row[], query: string) =>
    items.filter((row) => row.online || row.name.includes(query));

  it("runs the surface fold in the rows' own order, never sorting, and renders Flat", () => {
    const { result } = renderHook(() => useListLens({ items: ROWS, labels, fold: keepOnlineOrMatch }));
    expect(result.current.displayed.map((row) => row.id)).toEqual(["a", "b", "c"]);
    expect(result.current.groups).toBeNull();
    act(() => result.current.setQuery("bet"));
    expect(result.current.displayed.map((row) => row.id)).toEqual(["b", "c"]);
  });

  it("reads and writes a controlled query instead of its own", () => {
    let value = "zet";
    const set = (next: string) => {
      value = next;
    };
    const { result, rerender } = renderHook(() =>
      useListLens({ items: ROWS, labels, fold: keepOnlineOrMatch, controlledQuery: { value, set } }),
    );
    expect(result.current.query).toBe("zet");
    expect(result.current.displayed.map((row) => row.id)).toEqual(["a", "b"]);
    act(() => result.current.setQuery("beta"));
    expect(value).toBe("beta");
    rerender();
    expect(result.current.displayed.map((row) => row.id)).toEqual(["b", "c"]);
  });

  it("opens on its default chip", () => {
    const FILTERS = { online: (row: Row) => row.online, featured: (row: Row) => !row.online };
    const { result } = renderHook(() =>
      useListLens({ items: ROWS, labels, fold: keepOnlineOrMatch, filters: FILTERS, defaultFilter: "featured" }),
    );
    expect(result.current.filter).toBe("featured");
    expect(result.current.displayed.map((row) => row.id)).toEqual(["a", "c"]);
    expect(result.current.filterCounts).toEqual({ all: 3, online: 1, featured: 2 });
  });
});

// Ticket 24: the grid is a view mode of the same lens. Both densities persist
// under the one compat key, and no view mode reorders anything — a grid's
// cards fill row-major in the DOM's order, so its reading order is the lens's
// `onScreen` sequence (j / k, a shift-click range, Delete's next card).
describe("useListLens view modes (ticket 24)", () => {
  it("persists list, auto-grid and dense-grid under skills-viewMode, and drops an unknown value", () => {
    const { result } = render();
    expect(result.current.viewMode).toBe("list");
    for (const mode of ["auto-grid", "dense-grid", "list"] as const) {
      act(() => result.current.setViewMode(mode));
      expect(window.localStorage.getItem("skills-viewMode")).toBe(mode);
      expect(render().result.current.viewMode).toBe(mode);
    }
    window.localStorage.setItem("skills-viewMode", "masonry");
    expect(render().result.current.viewMode).toBe("list");
  });

  it("walks the same on-screen order in every view mode, collapsed groups excluded", () => {
    const { result } = render({ collapsePreference: collapsedSourcesPreference });
    act(() => result.current.setGroupByRepo(true));
    act(() => result.current.toggleCollapsed("solo/one"));
    const orders = (["list", "auto-grid", "dense-grid"] as const).map((mode) => {
      act(() => result.current.setViewMode(mode));
      return result.current.onScreen.map((s) => s.name);
    });
    expect(orders).toEqual([
      ["alpha", "gamma", "delta"],
      ["alpha", "gamma", "delta"],
      ["alpha", "gamma", "delta"],
    ]);
    // Delete's focus rule reads that order: removing gamma lands on delta,
    // the next card in reading order, across the collapsed group.
    const order = result.current.onScreen.map((s) => s.id);
    expect(focusAfter({ verb: "delete", removed: true }, { skillId: "1", order, surface: "skills" })).toEqual({
      kind: "row",
      skillId: "4",
    });
  });
});
