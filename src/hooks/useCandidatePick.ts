import { useCallback, useMemo, useState } from "react";
import {
  pickSummary,
  pickedCandidates,
  visibleCandidates,
  type PickSummary,
  type PickerCandidate,
} from "../lib/addFlow";

/** The shape every pickable candidate shares (git and local). */
export type PickCandidate = { name: string; subpath: string };

const EMPTY: never[] = [];

/**
 * Per-instance adapter: how one source's candidates read in the picker
 * (git and local differ in what makes a candidate unavailable).
 */
export type CandidatePickSource<C extends PickCandidate> = {
  toPicker: (candidate: C) => PickerCandidate;
};

export type CandidatePick<C extends PickCandidate, Ctx> = {
  /** The listing's context (repo URL, base path), captured when it opened. */
  ctx: Ctx | null;
  /** The listing's candidates as listed (the install source reads them). */
  listed: C[];
  /** The same candidates as the picker renders them. */
  candidates: PickerCandidate[];
  /** Selection keyed by subpath (the candidate identity). */
  selected: Record<string, boolean>;
  query: string;
  /** The candidates the search leaves on screen. */
  visible: PickerCandidate[];
  summary: PickSummary;
  /** What installs: checked, visible and selectable (#47), as listed. */
  picked: C[];
  /**
   * Keep a listing and its context. `preselect` names the subpaths to check;
   * omitted, every selectable candidate starts checked.
   */
  open: (ctx: Ctx, candidates: C[], preselect?: readonly string[]) => void;
  /** Discard the listing, its selection and its search. */
  reset: () => void;
  setQuery: (query: string) => void;
  toggle: (subpath: string, checked: boolean) => void;
  /** Check or uncheck every visible, selectable candidate (hidden ones keep theirs). */
  toggleAll: (checked: boolean) => void;
};

/**
 * One candidate picker's logic: the listing and its context, the selection,
 * the search, and the visible-and-checked rule. Instantiated once per source
 * (git, local) by the Add flow — a building block of that world, not a world
 * hook of its own; the flow owns the surface, the install and its report.
 */
export function useCandidatePick<C extends PickCandidate, Ctx>(
  source: CandidatePickSource<C>,
): CandidatePick<C, Ctx> {
  const { toPicker } = source;
  // The candidates and the context they were discovered in travel together.
  const [listing, setListing] = useState<{ ctx: Ctx; listed: C[] } | null>(null);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [query, setQuery] = useState("");
  const listed = listing?.listed ?? EMPTY;
  const candidates = useMemo(() => listed.map(toPicker), [listed, toPicker]);

  const open = useCallback(
    (ctx: Ctx, next: C[], preselect?: readonly string[]) => {
      setListing({ ctx, listed: next });
      setQuery("");
      const wanted = preselect ? new Set(preselect) : null;
      setSelected(
        Object.fromEntries(
          next.map((candidate) => {
            const selectable = toPicker(candidate).unavailable === null;
            return [candidate.subpath, selectable && (wanted ? wanted.has(candidate.subpath) : true)];
          }),
        ),
      );
    },
    [toPicker],
  );

  const reset = useCallback(() => {
    setListing(null);
    setSelected({});
    setQuery("");
  }, []);

  const toggle = useCallback(
    (subpath: string, checked: boolean) => {
      const candidate = candidates.find((c) => c.id === subpath);
      if (!candidate || candidate.unavailable !== null) return;
      setSelected((prev) => ({ ...prev, [subpath]: checked }));
    },
    [candidates],
  );

  const visible = useMemo(() => visibleCandidates(candidates, query), [candidates, query]);

  const toggleAll = useCallback(
    (checked: boolean) => {
      setSelected((prev) => {
        const next = { ...prev };
        for (const candidate of visible) if (candidate.unavailable === null) next[candidate.id] = checked;
        return next;
      });
    },
    [visible],
  );

  const summary = useMemo(() => pickSummary(candidates, selected, query), [candidates, query, selected]);
  const picked = useMemo(() => {
    const ids = new Set(pickedCandidates(candidates, selected, query).map((candidate) => candidate.id));
    return listed.filter((candidate) => ids.has(candidate.subpath));
  }, [candidates, listed, query, selected]);

  return {
    ctx: listing?.ctx ?? null,
    listed,
    candidates,
    selected,
    query,
    visible,
    summary,
    picked,
    open,
    reset,
    setQuery,
    toggle,
    toggleAll,
  };
}
