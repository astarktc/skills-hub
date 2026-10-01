/**
 * The action lifecycle's pure vocabulary (D19–D21): what a running action is
 * about (its pending subject), which commands can be cancelled, why a second
 * mutation is refused, and where focus goes once an action settles. The
 * reporter (`useStatusReporter`) owns the one pending cell and executes an
 * Outcome; the skill action door (`useSkillActions`) and the surfaces read
 * this module. Pure except the DOM readers at its end (`resolveFocusTarget`,
 * `returnLostFocus`).
 */

import type { CommandName } from "./tauri";
import { mainLandmarkFocus } from "@/components/ui/focus-fallback";

type Translate = (key: string, opts?: Record<string, unknown>) => string;

// ---------------------------------------------------------------------------
// Pending subject (D19)
// ---------------------------------------------------------------------------

/**
 * What a running action is about. A small action on one skill or one matrix
 * cell (or one matrix row's cells) shows inline pending on that subject; a
 * batch shows the progress surface.
 */
export type PendingSubject =
  | { kind: "skill"; id: string }
  /** Project-matrix cells, keyed `${skillId}:${tool}` (one cell, or a row's). */
  | { kind: "cell"; ids: readonly string[] }
  /**
   * Several skills acted on from a dialog that shows the action itself (a
   * bulk Remove: the Delete dialog pends and becomes the report, D16). Each
   * row pends inline; its streamed count is the dialog's, not the progress
   * surface's.
   */
  | { kind: "skills"; ids: readonly string[] }
  /**
   * A batch shown in the header progress (D19). `ids` names the skills it
   * acts on, when it knows them: each of their rows carries the pending mark
   * while the batch runs (the library stays browsable around them).
   */
  | { kind: "batch"; id?: string; ids?: readonly string[] };

/** A streamed batch's count (`done` of `total`), for the header progress. */
export type PendingProgress = { done: number; total: number };

/** The one action running now (the reporter allows one at a time). */
export type PendingAction = {
  subject: PendingSubject;
  /** What is running, in the operator's words ("Refresh") — the refusal names it. */
  label: string | null;
  /**
   * What the header progress says while it runs, action-specific and in the
   * progressive ("Refreshing", "Adding", "Removing beta") — never one
   * generic line for every batch. Null falls back to the label.
   */
  activity: string | null;
  startedAt: number;
  /**
   * Cancel is offered: the action declared a token-taking command
   * (`CANCELLABLE_COMMANDS`) and is not in an `uncancellable` phase.
   */
  cancellable: boolean;
  /** Cancel was requested; the action is still running until its command returns. */
  cancelling: boolean;
  progress: PendingProgress | null;
};

/**
 * The commands that honour the backend's shared `CancelToken` (D20; read off
 * `commands/mod.rs`: every command taking `State<Arc<CancelToken>>`). An
 * action is cancellable only when it declares one of these; every other
 * action — sync, import, delete, Disable/Enable, project operations — shows
 * no Cancel, because nothing would stop it.
 */
export const CANCELLABLE_COMMANDS = [
  "refreshManagedSkills",
  "updateManagedSkill",
  "repointSkillSource",
  "previewRepointSource",
  "cloneExploreSkill",
  // The Add batch: cancelled between candidates, the tail reported cancelled.
  "installSelection",
] as const satisfies readonly CommandName[];
export type CancellableCommand = (typeof CANCELLABLE_COMMANDS)[number];

/**
 * Whether the header progress shows this action (D19): a batch, a streamed
 * action, or one that can be cancelled (its Cancel must be reachable). A
 * small action on one skill or cell pends inline on its subject instead, and
 * a bulk Remove (`skills`) shows its count in the Delete dialog —
 * one surface for it, never both.
 */
export function showsHeaderProgress(pending: PendingAction | null): boolean {
  if (!pending) return false;
  if (pending.subject.kind === "skills") return false;
  return pending.subject.kind === "batch" || pending.progress !== null || pending.cancellable;
}

/** What the header progress says: the action's own words, else its name, else a generic line. */
export function progressActivity(pending: PendingAction, t: Translate): string {
  if (pending.cancelling) return t("lifecycle.cancelling");
  if (pending.activity) return pending.activity;
  return pending.label ? t("activity.running", { action: pending.label }) : t("activity.working");
}

/** Whether `pending` is about this skill (its row shows the pending mark). */
export function isPendingSkill(pending: PendingAction | null, skillId: string): boolean {
  if (!pending) return false;
  const { subject } = pending;
  if (subject.kind === "skills") return subject.ids.includes(skillId);
  if (subject.kind === "batch") return subject.ids?.includes(skillId) ?? false;
  return subject.kind === "skill" && subject.id === skillId;
}

/** The matrix cells in flight (`${skillId}:${tool}`); empty unless a cell action runs. */
export function pendingCellKeys(pending: PendingAction | null): Set<string> {
  return new Set(pending?.subject.kind === "cell" ? pending.subject.ids : []);
}

// ---------------------------------------------------------------------------
// One mutation at a time (D19)
// ---------------------------------------------------------------------------

/** Why an action was not started: another one is running (never queued). */
export type MutationRefusal = { code: "MUTATION_RUNNING"; running: string | null };

/** The refusal a new action meets now, or null when it may start. */
export function refusalFor(pending: PendingAction | null): MutationRefusal | null {
  return pending ? { code: "MUTATION_RUNNING", running: pending.label } : null;
}

/** The refusal in words — the disabled control's title and the refused action's toast. */
export function describeRefusal(refusal: MutationRefusal, t: Translate): string {
  return refusal.running
    ? t("lifecycle.running", { action: refusal.running })
    : t("lifecycle.busy");
}

// ---------------------------------------------------------------------------
// Focus after a mutation (D21)
// ---------------------------------------------------------------------------

/** Where focus goes; resolved against the DOM by `resolveFocusTarget`. */
export type FocusTarget =
  /** The control that started the action (Disable / Enable / Retry / …). */
  | { kind: "trigger" }
  /** A skill row (its name control). */
  | { kind: "row"; skillId: string }
  /** A skill's health row (Restore lands there). */
  | { kind: "health"; skillId: string }
  /** The inspector's heading while it shows the skill (beside the list or as a sheet). */
  | { kind: "inspector"; skillId: string }
  /** The showing surface's heading (the list emptied). */
  | { kind: "heading"; surface: "skills" | "projects" };

/** What settled, as the focus rule needs it. */
export type FocusOutcome = {
  verb: string;
  /** The action's subject left the catalog (a completed Delete). */
  removed: boolean;
};

/** Where the action happened. */
export type FocusContext = {
  skillId: string | null;
  /**
   * Every skill a Delete removed, when it acted on several (a bulk Remove);
   * the rule steps past all of them. Defaults to `skillId` alone.
   */
  removedIds?: readonly string[];
  /** The showing surface's visible row order before the action ran. */
  order: readonly string[];
  surface: "skills" | "projects";
};

/**
 * The post-mutation focus rule (D21), one table:
 * - Delete that removed the skill → the next row, the previous one when it
 *   was last, the surface heading when the list is now empty;
 * - Restore → the skill's health row;
 * - everything else (Disable / Enable / Retry / Update / …, and a Delete
 *   that kept the skill) → the same control that started it.
 */
export function focusAfter(outcome: FocusOutcome, context: FocusContext): FocusTarget {
  if (outcome.verb === "delete" && outcome.removed) {
    const gone = new Set(context.removedIds ?? (context.skillId ? [context.skillId] : []));
    const { order } = context;
    const rest = order.filter((id) => !gone.has(id));
    if (rest.length === 0) return { kind: "heading", surface: context.surface };
    const indices = order.flatMap((id, index) => (gone.has(id) ? [index] : []));
    if (indices.length === 0) return { kind: "row", skillId: rest[0] };
    // The first row after the last one removed; else the nearest before the first.
    const next =
      order.slice(indices[indices.length - 1] + 1).find((id) => !gone.has(id)) ??
      order
        .slice(0, indices[0])
        .reverse()
        .find((id) => !gone.has(id));
    return { kind: "row", skillId: next ?? rest[0] };
  }
  if (outcome.verb === "restore" && context.skillId) {
    return { kind: "health", skillId: context.skillId };
  }
  return { kind: "trigger" };
}

/**
 * The fallback chain when a dialog closes and its trigger is gone (D21):
 * the focused row, then the inspector's heading, then the surface heading.
 * The Dialog primitive walks it with its `usable` test, so beside the list
 * the row takes focus, and inside the sheet — whose layer keeps the row
 * inert — the sheet's heading does. (The primitive tries the trigger itself
 * first, and the main landmark last.)
 */
export function dialogFallback(context: {
  focusedSkillId: string | null;
  surface: "skills" | "projects";
}): FocusTarget[] {
  return [
    ...(context.focusedSkillId
      ? ([
          { kind: "row", skillId: context.focusedSkillId },
          { kind: "inspector", skillId: context.focusedSkillId },
        ] as const)
      : []),
    { kind: "heading", surface: context.surface },
  ];
}

const escape = (value: string) =>
  typeof CSS !== "undefined" && CSS.escape ? CSS.escape(value) : value.replace(/"/g, '\\"');

// No `inert` check: a closing modal still marks the page inert while the
// Dialog primitive asks for its return target (it focuses it once released).
const focusable = (element: Element | null): HTMLElement | null =>
  element instanceof HTMLElement && element.isConnected ? element : null;

/**
 * The element a target names, or null when it is not on screen. Rows carry
 * `data-skill-row`, health rows `data-inspector-health` (the inspector's
 * Health section) or `data-skill-health` (the row's marks), headings
 * `data-surface-heading`; a heading that is not there yet falls back to the
 * main landmark (never `<body>`).
 */
export function resolveFocusTarget(
  target: FocusTarget,
  trigger: Element | null = null,
): HTMLElement | null {
  if (typeof document === "undefined") return null;
  switch (target.kind) {
    case "trigger": {
      const element = focusable(trigger);
      return element && !(element as HTMLButtonElement).disabled ? element : null;
    }
    case "row":
      return focusable(document.querySelector(`[data-skill-row="${escape(target.skillId)}"]`));
    case "inspector":
      return focusable(document.querySelector(`[data-inspector-heading="${escape(target.skillId)}"]`));
    case "health":
      // The inspector's Health section when it is showing the skill, else
      // the row's exception marks.
      return focusable(
        document.querySelector(`[data-inspector-health="${escape(target.skillId)}"]`) ??
          document.querySelector(`[data-skill-health="${escape(target.skillId)}"]`),
      );
    case "heading":
      return (
        focusable(document.querySelector(`[data-surface-heading="${target.surface}"]`)) ??
        mainLandmarkFocus()
      );
  }
}

/**
 * The first target of a chain that is on screen — and, given a closing
 * layer's `usable` test (the Dialog primitive's `fallbackFocus` argument),
 * that will take focus once the layer has closed.
 */
export function resolveFirst(
  targets: readonly FocusTarget[],
  trigger: Element | null = null,
  usable: (element: Element) => boolean = () => true,
): HTMLElement | null {
  for (const target of targets) {
    const element = resolveFocusTarget(target, trigger);
    if (element && usable(element)) return element;
  }
  return null;
}

/**
 * Whether focus is "lost" once an action settled: on `<body>`, on the main
 * landmark (a closing layer's last fallback), on a layer's popup itself
 * (where a modal parks focus when the control holding it went away — the
 * inspector sheet's Restore), or on the header progress, whose Cancel goes
 * away with the action it cancelled.
 */
export function isFocusLost(active: Element | null): boolean {
  return (
    !active ||
    active === document.body ||
    active.tagName === "MAIN" ||
    active.matches("[role='dialog'], [role='alertdialog']") ||
    active.closest("[data-slot='header-progress']") !== null
  );
}

/**
 * D21's floor for every action, whoever ran it: once it has settled, focus
 * that was lost (`isFocusLost`: the control holding it went away, or was
 * disabled while pending) returns to the control that started the action,
 * when it is still there and usable. Focus the operator placed is kept; the
 * skill door's own rule (`focusAfter`, e.g. Restore → Health) runs after
 * this and takes over. A trigger that is gone leaves focus to the main
 * landmark, never `<body>`. One check once the settled render is on screen
 * (a frame later at most, for a control the render re-enables); no hold.
 */
export function returnLostFocus(trigger: Element | null): void {
  if (typeof window === "undefined" || !(trigger instanceof HTMLElement) || trigger === document.body) return;
  if (trigger.closest("[data-slot='header-progress']")) return;
  const place = (settling: boolean) => {
    const active = document.activeElement;
    if (!isFocusLost(active)) return;
    const usable = trigger.isConnected && !trigger.closest("[inert]");
    if (usable && (trigger as HTMLButtonElement).disabled && settling) {
      window.requestAnimationFrame(() => place(false));
      return;
    }
    if (usable && !(trigger as HTMLButtonElement).disabled) trigger.focus();
    // The control went with what it acted on (an Explore card the preview
    // replaced): never `<body>` — the main landmark, as a closing layer does.
    else if (!active || active === document.body) mainLandmarkFocus()?.focus();
  };
  window.setTimeout(() => place(true), 0);
}
