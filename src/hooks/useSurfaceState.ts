import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { RepointKind } from "../lib/skillPresentation";
import type { OverwriteChoice, OverwriteRow } from "../lib/overwriteAsk";
import type { DeleteSettlement } from "../lib/reportOutcome";

/**
 * Surface state: "what is on screen", as one value — the area, the focused
 * subject, the per-surface selection and the dialog stack.
 *
 * World hooks keep their data and actions and ask this module to open or
 * close surfaces (`openDialog`, `focus`, …); no world keeps a visibility flag
 * or a "which skill is this modal about" cell of its own. Invariants hold by
 * construction:
 *
 * - The focused subject always belongs to the area showing it (`Route` is a
 *   union per area), and a focused id that leaves its catalog clears itself
 *   (`useFocusReconciliation`).
 * - One dialog at a time: `openDialog` replaces the stack; a nested ask (a
 *   picker over Add, a confirmation over anything) is a layer pushed on top,
 *   and a UI close (`closeDialog`) closes only the top layer. System-raised
 *   announcements (`offerDialog`) wait until no dialog is open.
 * - Every ask settles exactly once, with one of three answers: `confirmed`
 *   or `declined` — only ever an explicit button — or `dismissed`: the
 *   operator closed it (Escape, outside press), a newer ask of its kind
 *   displaced it, it went with its stack, or the module unmounted. A caller
 *   records nothing durable for `dismissed`.
 *
 * Only the area and the focused subject are addressable (D5): they round-trip
 * through `location.hash` (`#/skills/<id>`, `#/projects/<id>`, `#/explore`,
 * `#/settings/<pane>`), so Back/Forward and deep links work. Dialogs,
 * selection and drafts never enter the hash; neither does an Explore preview
 * (it needs a clone, so it is not a place one can link to).
 */

export type Area = "skills" | "explore" | "projects" | "settings";

/** An online skill cloned for preview: shown in the detail, never managed. */
export type ExplorePreview = {
  sourceUrl: string;
  name: string;
  summary: string | null;
  /** The clone-cache directory the preview's files are read from. */
  cachePath: string;
  /** When the clone landed (the detail's "updated" line). */
  fetchedAt: number;
};

export type Route =
  | { area: "skills"; focused: { kind: "skill"; id: string } | null }
  | { area: "explore"; focused: { kind: "explore"; ref: ExplorePreview } | null }
  | { area: "projects"; focused: { kind: "project"; id: string } | null }
  | { area: "settings"; focused: { kind: "settings"; pane: string } | null };

export type FocusedSubject = NonNullable<Route["focused"]>;

/** A list surface that owns a selection (D4). */
export type SelectionSurface = "skills" | "matrix" | "explore";

/**
 * The wide verbs whose count-bearing confirmation is a `confirmCount` dialog
 * (ticket 13): the door opens it for a verb over several skills called
 * without `confirmed`; its Confirm re-runs the verb with `confirmed: true`.
 */
export type CountVerb = "syncToTargets" | "disable" | "enable" | "update";

/** Every modal the app shows, and what it is about. One union across worlds. */
export type DialogState =
  // Skills world
  | { kind: "addSkill" }
  | { kind: "import" }
  | { kind: "gitPick" }
  | { kind: "localPick" }
  /**
   * Delete (D16): the confirmation for one or several skills, and — once a
   * Delete left something behind — the same dialog as its report (`report`,
   * one settlement per skill).
   */
  | { kind: "deleteSkill"; skillIds: readonly string[]; report?: readonly DeleteSettlement[] }
  /** A wide verb's count-bearing confirmation (Sync to Tools carries its Tool pick). */
  | { kind: "confirmCount"; verb: CountVerb; skillIds: readonly string[] }
  | { kind: "invocationEdit"; skillId: string }
  | { kind: "repoint"; skillId: string; name: string; preselect: RepointKind }
  | { kind: "detachSkill"; skillId: string }
  /** Assign to project…: the picker for one skill or a selection. */
  | { kind: "assignProject"; skillIds: readonly string[] }
  // Tools + sync world
  | { kind: "toolConfig" }
  | { kind: "newTools" }
  // App shell
  /** The session's report, grouped by run (D11); `runId` opens it on one run ("View report"). */
  | { kind: "notifications"; runId?: number }
  | { kind: "update" }
  // Project world (formerly its own `ProjectDialog` union)
  | { kind: "projectAdd" }
  | { kind: "projectEdit"; projectId: string }
  | { kind: "projectToolConfig" }
  | { kind: "projectRemove"; projectId: string };

/** A question an action awaits. */
export type AskRequest =
  | { kind: "sharedDir"; toolKey: string; toolLabel: string; labels: string[] }
  | { kind: "overwrite"; rows: OverwriteRow[] };

export type AskLayer = AskRequest & { askId: number };

/**
 * How an ask settled. `confirmed` / `declined` are the operator's explicit
 * buttons; `dismissed` is every other ending (Escape, outside press,
 * displacement, the stack replaced, unmount) and means "no answer".
 */
export type AskAnswer = "confirmed" | "declined" | "dismissed";

/**
 * The overwrite ask may also answer per row (D12): one choice per row, in
 * row order, when the operator's choices differ. `confirmed` / `declined`
 * remain "every row overwrite" / "every row keep".
 */
export type RowChoices = { choices: readonly OverwriteChoice[] };

/** What an ask of kind `K` resolves with. */
export type AskAnswerOf<K extends AskRequest["kind"]> = K extends "overwrite"
  ? AskAnswer | RowChoices
  : AskAnswer;

export type DialogLayer = DialogState | AskLayer;
export type DialogKind = DialogLayer["kind"];
export type LayerOf<K extends DialogKind> = Extract<DialogLayer, { kind: K }>;
/** An ask as a modal renders it: its payload plus the one way to answer it. */
export type PendingAsk<K extends AskRequest["kind"]> = LayerOf<K> & {
  /** Settle the ask; a second call (or a stale layer's) is a no-op. */
  answer: (answer: AskAnswerOf<K>) => void;
};

export type SurfaceValue = {
  route: Route;
  /** Bottom → top. Empty when no dialog is open. */
  stack: readonly DialogLayer[];
  /** Offered dialogs waiting for the stack to empty. */
  queue: readonly DialogState[];
  selection: Readonly<Record<SelectionSurface, ReadonlySet<string>>>;
};

const ASK_KINDS: ReadonlySet<DialogKind> = new Set(["sharedDir", "overwrite"]);
const isAsk = (layer: DialogLayer): layer is AskLayer => ASK_KINDS.has(layer.kind);

const DEFAULT_ROUTE: Route = { area: "skills", focused: null };
const EMPTY_SELECTION: ReadonlySet<string> = new Set();

// ─── Hash grammar (pure) ─────────────────────────────

/** The hash for a route. An Explore preview is not addressable: `#/explore`. */
export function formatHash(route: Route): string {
  const seg = (value: string) => encodeURIComponent(value);
  switch (route.area) {
    case "skills":
      return route.focused ? `#/skills/${seg(route.focused.id)}` : "#/skills";
    case "projects":
      return route.focused ? `#/projects/${seg(route.focused.id)}` : "#/projects";
    case "settings":
      return route.focused ? `#/settings/${seg(route.focused.pane)}` : "#/settings";
    case "explore":
      return "#/explore";
  }
}

/** The route a hash addresses, or null when it addresses none. */
export function parseHash(hash: string): Route | null {
  const match = /^#\/(skills|explore|projects|settings)(?:\/([^/]+))?\/?$/.exec(hash);
  if (!match) return null;
  const [, area, raw] = match;
  let subject: string | null = null;
  if (raw) {
    try {
      subject = decodeURIComponent(raw);
    } catch {
      return null;
    }
  }
  switch (area) {
    case "skills":
      return { area, focused: subject ? { kind: "skill", id: subject } : null };
    case "projects":
      return { area, focused: subject ? { kind: "project", id: subject } : null };
    case "settings":
      return { area, focused: subject ? { kind: "settings", pane: subject } : null };
    default:
      return { area: "explore", focused: null };
  }
}

/** The route a subject lives in: focusing always shows its area. */
export function routeFor(subject: FocusedSubject): Route {
  switch (subject.kind) {
    case "skill":
      return { area: "skills", focused: subject };
    case "explore":
      return { area: "explore", focused: subject };
    case "project":
      return { area: "projects", focused: subject };
    case "settings":
      return { area: "settings", focused: subject };
  }
}

/** Same place: same address, and for Explore the same preview. */
function sameRoute(a: Route, b: Route): boolean {
  if (formatHash(a) !== formatHash(b)) return false;
  if (a.area === "explore" && b.area === "explore") {
    return (a.focused?.ref ?? null) === (b.focused?.ref ?? null);
  }
  return true;
}

const readHash = () => (typeof window === "undefined" ? "" : window.location.hash);

// ─── Transitions (pure) ──────────────────────────────

/** An empty stack takes the next offered dialog. */
function drainQueue(value: SurfaceValue): SurfaceValue {
  if (value.stack.length > 0 || value.queue.length === 0) return value;
  const [next, ...rest] = value.queue;
  return { ...value, stack: [next], queue: rest };
}

function withStack(value: SurfaceValue, stack: readonly DialogLayer[]): SurfaceValue {
  return drainQueue({ ...value, stack });
}

/** Remove the topmost layer of `kind` (anywhere in the stack). */
function withoutTopmost(value: SurfaceValue, kind: DialogKind): SurfaceValue {
  for (let i = value.stack.length - 1; i >= 0; i--) {
    if (value.stack[i].kind === kind) {
      return withStack(value, [...value.stack.slice(0, i), ...value.stack.slice(i + 1)]);
    }
  }
  return value;
}

/**
 * The dialog kinds that are about skills / one project. A dialog about
 * several skills stays while any of them does (a Delete's report describes
 * the kept ones).
 */
const subjectOf = (layer: DialogLayer): { kind: "skill" | "project"; ids: readonly string[] } | null => {
  switch (layer.kind) {
    case "deleteSkill":
    case "confirmCount":
    case "assignProject":
      return { kind: "skill", ids: layer.skillIds };
    case "invocationEdit":
    case "repoint":
    case "detachSkill":
      return { kind: "skill", ids: [layer.skillId] };
    case "projectEdit":
    case "projectRemove":
      return { kind: "project", ids: [layer.projectId] };
    default:
      return null;
  }
};

export function initialSurfaceValue(hash: string = readHash()): SurfaceValue {
  return {
    route: parseHash(hash) ?? DEFAULT_ROUTE,
    stack: [],
    queue: [],
    selection: { skills: EMPTY_SELECTION, matrix: EMPTY_SELECTION, explore: EMPTY_SELECTION },
  };
}

type HistoryMode = "push" | "replace" | "none";

function writeHash(route: Route, mode: HistoryMode) {
  if (mode === "none" || typeof window === "undefined") return;
  const hash = formatHash(route);
  if (window.location.hash === hash) return;
  if (mode === "push") window.history.pushState(window.history.state, "", hash);
  else window.history.replaceState(window.history.state, "", hash);
}

// ─── The hook ────────────────────────────────────────

export function useSurfaceState() {
  const [value, setValue] = useState<SurfaceValue>(() => initialSurfaceValue());
  // Actions read and write through the ref so every call sees the value the
  // previous call produced (several transitions can run in one handler), and
  // side effects (settling asks, writing history) stay out of updaters.
  const valueRef = useRef(value);
  const resolvers = useRef(new Map<number, (answer: AskAnswer | RowChoices) => void>());
  const nextAskId = useRef(1);

  const settle = useCallback((askId: number, answer: AskAnswer | RowChoices) => {
    const resolve = resolvers.current.get(askId);
    if (!resolve) return;
    resolvers.current.delete(askId);
    resolve(answer);
  }, []);

  const commit = useCallback(
    (next: SurfaceValue, history: HistoryMode) => {
      const prev = valueRef.current;
      if (next === prev) return;
      valueRef.current = next;
      setValue(next);
      // An ask that left the stack without an answer was dismissed.
      for (const layer of prev.stack) {
        if (isAsk(layer) && !next.stack.includes(layer)) settle(layer.askId, "dismissed");
      }
      if (next.route !== prev.route) writeHash(next.route, history);
    },
    [settle],
  );

  const navigate = useCallback(
    (route: Route, history: HistoryMode = "push") => {
      const prev = valueRef.current;
      if (sameRoute(route, prev.route)) return;
      commit({ ...prev, route }, history);
    },
    [commit],
  );

  // Hash → state: Back/Forward, a typed or linked hash. A hash that already
  // describes the current route is not a navigation (it keeps an Explore
  // preview, which the hash does not carry).
  useEffect(() => {
    const onHash = () => {
      const hash = readHash();
      const route = parseHash(hash);
      if (!route) {
        writeHash(valueRef.current.route, "replace");
        return;
      }
      if (formatHash(route) === formatHash(valueRef.current.route)) return;
      commit({ ...valueRef.current, route }, "none");
    };
    // Normalise the address the app opened on (empty or unknown hash).
    if (readHash() !== formatHash(valueRef.current.route)) {
      writeHash(valueRef.current.route, "replace");
    }
    window.addEventListener("hashchange", onHash);
    window.addEventListener("popstate", onHash);
    return () => {
      window.removeEventListener("hashchange", onHash);
      window.removeEventListener("popstate", onHash);
    };
  }, [commit]);

  // Unmounting dismisses every pending ask: no action waits forever.
  useEffect(() => {
    const pending = resolvers.current;
    return () => {
      for (const resolve of pending.values()) resolve("dismissed");
      pending.clear();
    };
  }, []);

  // ── Area and focus ──
  const open = useCallback(
    (area: Area) => navigate({ area, focused: null } as Route),
    [navigate],
  );
  const focus = useCallback(
    (subject: FocusedSubject) => navigate(routeFor(subject)),
    [navigate],
  );
  const unfocus = useCallback(() => {
    const { area } = valueRef.current.route;
    navigate({ area, focused: null } as Route);
  }, [navigate]);

  /**
   * A catalog changed: a focused subject, a selected id or a dialog about
   * an id no longer in it goes. Replaces history (the address was not the
   * operator's navigation).
   */
  const reconcile = useCallback(
    (kind: "skill" | "project", ids: readonly string[]) => {
      const known = new Set(ids);
      const prev = valueRef.current;
      let next = prev;
      const focused = prev.route.focused;
      if (focused && focused.kind === kind && !known.has(focused.id)) {
        next = { ...next, route: { area: prev.route.area, focused: null } as Route };
      }
      if (kind === "skill") {
        const skills = prev.selection.skills;
        const kept = [...skills].filter((id) => known.has(id));
        if (kept.length !== skills.size) {
          next = { ...next, selection: { ...next.selection, skills: new Set(kept) } };
        }
      }
      const stack = next.stack.filter((layer) => {
        const subject = subjectOf(layer);
        return !subject || subject.kind !== kind || subject.ids.some((id) => known.has(id));
      });
      if (stack.length !== next.stack.length) next = withStack(next, stack);
      commit(next, "replace");
    },
    [commit],
  );

  // ── Selection (per surface) ──
  const setSelection = useCallback(
    (surface: SelectionSurface, ids: ReadonlySet<string>) => {
      const prev = valueRef.current;
      commit({ ...prev, selection: { ...prev.selection, [surface]: ids } }, "none");
    },
    [commit],
  );
  const select = useCallback(
    (surface: SelectionSurface, ids: Iterable<string>) => setSelection(surface, new Set(ids)),
    [setSelection],
  );
  const toggle = useCallback(
    (surface: SelectionSurface, id: string) => {
      const next = new Set(valueRef.current.selection[surface]);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      setSelection(surface, next);
    },
    [setSelection],
  );
  const clearSelection = useCallback(
    (surface: SelectionSurface) => {
      if (valueRef.current.selection[surface].size === 0) return;
      setSelection(surface, EMPTY_SELECTION);
    },
    [setSelection],
  );

  // ── Dialogs ──
  /** Open a dialog as the only one (replacing whatever was open). */
  const openDialog = useCallback(
    (dialog: DialogState) => commit(withStack(valueRef.current, [dialog]), "none"),
    [commit],
  );
  /** Open a dialog above the current one (a nested step, e.g. a picker over Add). */
  const pushDialog = useCallback(
    (dialog: DialogState) => {
      const prev = valueRef.current;
      commit(withStack(prev, [...prev.stack, dialog]), "none");
    },
    [commit],
  );
  /** A system-raised dialog: shown now if nothing is open, else when the stack empties. */
  const offerDialog = useCallback(
    (dialog: DialogState) => {
      const prev = valueRef.current;
      commit(
        prev.stack.length === 0
          ? { ...prev, stack: [dialog] }
          : { ...prev, queue: [...prev.queue, dialog] },
        "none",
      );
    },
    [commit],
  );
  /** The operator closed a dialog (Escape, ×, Cancel): only the top layer closes. */
  const closeDialog = useCallback(
    (kind: DialogKind) => {
      const prev = valueRef.current;
      const top = prev.stack[prev.stack.length - 1];
      if (top?.kind !== kind) return;
      commit(withStack(prev, prev.stack.slice(0, -1)), "none");
    },
    [commit],
  );
  /** A flow is done with its dialog: remove it wherever it sits. */
  const dismissDialog = useCallback(
    (kind: DialogKind) => commit(withoutTopmost(valueRef.current, kind), "none"),
    [commit],
  );
  /**
   * Replace the topmost open dialog of the same kind in place (its position
   * and the layers above it kept): a dialog that changes state — the Delete
   * confirmation becoming its report — keeps its identity.
   */
  const updateDialog = useCallback(
    (dialog: DialogState) => {
      const prev = valueRef.current;
      for (let i = prev.stack.length - 1; i >= 0; i--) {
        if (prev.stack[i].kind === dialog.kind) {
          const stack = [...prev.stack];
          stack[i] = dialog;
          commit({ ...prev, stack }, "none");
          return;
        }
      }
    },
    [commit],
  );
  /** Withdraw an offered dialog that is no longer relevant, open or queued. */
  const withdrawDialog = useCallback(
    (kind: DialogState["kind"]) => {
      const prev = valueRef.current;
      const queue = prev.queue.filter((dialog) => dialog.kind !== kind);
      const next = withoutTopmost(queue.length === prev.queue.length ? prev : { ...prev, queue }, kind);
      commit(next, "none");
    },
    [commit],
  );

  /**
   * Ask the operator: pushes the ask on top and resolves with the answer. A
   * newer ask of the same kind dismisses the one it displaces.
   */
  const askAnyKind = useCallback(
    (request: AskRequest): Promise<AskAnswer | RowChoices> =>
      new Promise<AskAnswer | RowChoices>((resolve) => {
        const askId = nextAskId.current++;
        const layer = { ...request, askId } as AskLayer;
        resolvers.current.set(askId, resolve);
        const prev = valueRef.current;
        const kept = prev.stack.filter((l) => l.kind !== request.kind);
        commit(withStack(prev, [...kept, layer]), "none");
      }),
    [commit],
  );
  // A kind's answers are typed at `pendingAsk(kind).answer`, which is the
  // only way an ask settles; the shared resolver store holds every kind's.
  const ask = askAnyKind as <R extends AskRequest>(
    request: R,
  ) => Promise<AskAnswerOf<R["kind"]>>;
  const answer = useCallback(
    (askId: number, answer: AskAnswer | RowChoices) => {
      settle(askId, answer);
      const prev = valueRef.current;
      const stack = prev.stack.filter((l) => !(isAsk(l) && l.askId === askId));
      if (stack.length !== prev.stack.length) commit(withStack(prev, stack), "none");
    },
    [commit, settle],
  );

  const { route, stack, selection } = value;

  const isOpen = useCallback((kind: DialogKind) => stack.some((l) => l.kind === kind), [stack]);
  const layer = useCallback(
    <K extends DialogKind>(kind: K): LayerOf<K> | null => {
      for (let i = stack.length - 1; i >= 0; i--) {
        if (stack[i].kind === kind) return stack[i] as LayerOf<K>;
      }
      return null;
    },
    [stack],
  );
  const pendingAsk = useCallback(
    <K extends AskRequest["kind"]>(kind: K): PendingAsk<K> | null => {
      const found = layer(kind);
      if (!found) return null;
      const { askId } = found as AskLayer;
      return {
        ...found,
        answer: (settled: AskAnswerOf<K>) => answer(askId, settled),
      } as PendingAsk<K>;
    },
    [answer, layer],
  );

  return useMemo(
    () => ({
      route,
      area: route.area,
      focused: route.focused,
      stack,
      top: stack[stack.length - 1] ?? null,
      selection,
      isOpen,
      layer,
      pendingAsk,
      open,
      focus,
      unfocus,
      reconcile,
      select,
      toggle,
      clearSelection,
      openDialog,
      pushDialog,
      offerDialog,
      updateDialog,
      closeDialog,
      dismissDialog,
      withdrawDialog,
      ask,
    }),
    [
      route,
      stack,
      selection,
      isOpen,
      layer,
      pendingAsk,
      open,
      focus,
      unfocus,
      reconcile,
      select,
      toggle,
      clearSelection,
      openDialog,
      pushDialog,
      offerDialog,
      updateDialog,
      closeDialog,
      dismissDialog,
      withdrawDialog,
      ask,
    ],
  );
}

export type SurfaceState = ReturnType<typeof useSurfaceState>;

/** What a world hook may do to the dialog stack. */
export type DialogDoor = Pick<
  SurfaceState,
  | "isOpen"
  | "layer"
  | "openDialog"
  | "pushDialog"
  | "offerDialog"
  | "updateDialog"
  | "closeDialog"
  | "dismissDialog"
  | "withdrawDialog"
>;

/**
 * Keep the focused subject, the selection and subject dialogs inside a
 * catalog: an id that leaves it — or a focus or selection that names an id
 * it never had (a stale link) — clears itself before paint. `ids` is null
 * while the catalog has not loaded (a deep link must survive the first,
 * empty render).
 */
export function useFocusReconciliation(
  surface: Pick<SurfaceState, "reconcile" | "route" | "selection">,
  kind: "skill" | "project",
  ids: readonly string[] | null,
) {
  const { reconcile, route, selection } = surface;
  useLayoutEffect(() => {
    if (ids) reconcile(kind, ids);
  }, [ids, kind, reconcile, route, selection]);
}
