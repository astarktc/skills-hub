import { useMemo, useState } from "react";
import {
  displayOrder,
  filterAndSortSkills,
  groupSkillsByRepo,
  type RepoGroup,
  type SkillPresentationFields,
  type SortMode,
} from "../lib/skillPresentation";
import type { PersistedPreference } from "../lib/persistedPreference";
import type { viewModePreference } from "../lib/preferences";
import { usePersistedPreference } from "./usePersistedPreference";

export type ViewMode = ReturnType<typeof viewModePreference.read>;

/** A list surface with no view-mode choice renders as a list. */
const LIST_ONLY: PersistedPreference<ViewMode> = {
  key: "",
  read: () => "list",
  write: () => {},
};

export type LensItem = SkillPresentationFields & { id: string; name: string };

export type ListLensOptions<T extends LensItem> = {
  items: readonly T[];
  /** Group labels, localized by the caller. */
  labels: { local: string; ungrouped: string; individual: string };
  /** Where this surface persists Grouped/Flat (the literal keys live in preferences.ts). */
  groupByRepoPreference: PersistedPreference<boolean>;
  /** Where this surface persists its view mode; omitted = list only. */
  viewModePreference?: PersistedPreference<ViewMode>;
  /** Fold every single-skill source into one "Individual sources" group. */
  foldSingletons?: boolean;
  /** The surface's selection (surface state); the lens answers its visible subset. */
  selection?: ReadonlySet<string>;
  /**
   * Named filters (the surface's chips), each a predicate over one item;
   * `filter` picks one, "all" none. Pass a module constant: the counts are
   * memoised on its identity.
   */
  filters?: Readonly<Record<string, (item: T) => boolean>>;
};

/**
 * The list lens: "which skills am I looking at, and how". One instance per
 * list surface (My Skills, the assignment matrix; the pickers and the
 * palette later). It owns the query, the sort, grouping (Grouped/Flat, with
 * the optional singleton fold), the named filters (chips), the view mode
 * and their persistence; the
 * folds themselves are the pure functions in `skillPresentation.ts`.
 */
export function useListLens<T extends LensItem>({
  items,
  labels,
  groupByRepoPreference,
  viewModePreference,
  foldSingletons = false,
  selection,
  filters,
}: ListLensOptions<T>) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<string>("all");
  const [sort, setSort] = useState<SortMode>("name");
  const [groupByRepo, setGroupByRepo] = usePersistedPreference(groupByRepoPreference);
  const [viewMode, setViewMode] = usePersistedPreference(viewModePreference ?? LIST_ONLY);

  const predicate = filter === "all" ? undefined : filters?.[filter];
  const visible = useMemo(() => {
    const sorted = filterAndSortSkills([...items], { query, sort });
    return predicate ? sorted.filter(predicate) : sorted;
  }, [items, predicate, query, sort]);

  /** How many items each filter keeps, over the whole list (not the query). */
  const filterCounts = useMemo(() => {
    const counts: Record<string, number> = { all: items.length };
    for (const [name, keep] of Object.entries(filters ?? {})) counts[name] = items.filter(keep).length;
    return counts;
  }, [filters, items]);

  const { local, ungrouped, individual } = labels;
  /** null = Flat. */
  const groups: RepoGroup<T>[] | null = useMemo(
    () =>
      groupByRepo
        ? groupSkillsByRepo(visible, { local, ungrouped, individual }, { foldSingletons })
        : null,
    [foldSingletons, groupByRepo, individual, local, ungrouped, visible],
  );

  /**
   * The rows in the order they are on screen — a grouped list walks its
   * groups — the one sequence rendering, selection navigation and the
   * post-mutation focus rule (Delete's next row) read.
   */
  const displayed = useMemo(() => displayOrder(visible, groups), [groups, visible]);

  /** The selected ids the lens currently shows, in display order. */
  const visibleSelection = useMemo(
    () => (selection ? displayed.filter((item) => selection.has(item.id)).map((item) => item.id) : []),
    [displayed, selection],
  );

  // One stable value per change, so memoised consumers take the lens whole.
  return useMemo(
    () => ({
      query,
      setQuery,
      filter,
      setFilter,
      filterCounts,
      sort,
      setSort,
      groupByRepo,
      setGroupByRepo,
      viewMode,
      setViewMode,
      visible,
      groups,
      displayed,
      visibleSelection,
    }),
    [
      displayed,
      filter,
      filterCounts,
      groupByRepo,
      groups,
      query,
      setGroupByRepo,
      setViewMode,
      sort,
      viewMode,
      visible,
      visibleSelection,
    ],
  );
}

export type ListLens<T extends LensItem = LensItem> = ReturnType<typeof useListLens<T>>;
