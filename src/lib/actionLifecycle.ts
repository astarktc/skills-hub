/**
 * The action lifecycle's pure vocabulary (D19–D21): what a running action is
 * about (its pending subject), which commands can be cancelled, why a second
 * mutation is refused, and where focus goes once an action settles. The
 * reporter (`useStatusReporter`) owns the one pending cell and executes an
 * Outcome; the skill action door (`useSkillActions`) and the surfaces read
 * this module. Pure except `resolveFocusTarget`, the one DOM reader.
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
  | { kind: "batch"; id?: string };

/** A streamed batch's count (`done` of `total`), for the header progress. */
export type PendingProgress = { done: number; total: number };

/** The one action running now (the reporter allows one at a time). */
export type PendingAction = {
  subject: PendingSubject;
  /** What is running, in the operator's words ("Refresh") — the refusal names it. */
  label: string | null;
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
  "installGitSelection",
  "cloneExploreSkill",
  // The Add batch: cancelled between candidates, the tail reported cancelled.
  "installSelection",
] as const satisfies readonly CommandName[];
export type CancellableCommand = (typeof CANCELLABLE_COMMANDS)[number];

/** Whether the overlay/progress surface shows for this action (a batch or a streamed one). */
export function showsProgressSurface(pending: PendingAction | null): boolean {
  if (!pending) return false;
  return (
    pending.subject.kind === "batch" ||
    pending.progress !== null ||
    pending.cancellable
  );
}

/** Whether `pending` is about this skill (its card shows inline pending). */
export function isPendingSkill(pending: PendingAction | null, skillId: string): boolean {
  return pending?.subject.kind === "skill" && pending.subject.id === skillId;
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
    const index = context.skillId ? context.order.indexOf(context.skillId) : -1;
    const rest = context.order.filter((id) => id !== context.skillId);
    if (rest.length === 0) return { kind: "heading", surface: context.surface };
    const next = index >= 0 ? (context.order[index + 1] ?? context.order[index - 1]) : rest[0];
    return { kind: "row", skillId: next ?? rest[0] };
  }
  if (outcome.verb === "restore" && context.skillId) {
    return { kind: "health", skillId: context.skillId };
  }
  return { kind: "trigger" };
}

/**
 * The fallback chain when a dialog closes and its trigger is gone (D21):
 * the focused row, then the surface heading. (The Dialog primitive tries the
 * trigger itself first, and the main landmark last.)
 */
export function dialogFallback(context: {
  focusedSkillId: string | null;
  surface: "skills" | "projects";
}): FocusTarget[] {
  return [
    ...(context.focusedSkillId
      ? [{ kind: "row", skillId: context.focusedSkillId } as const]
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

/** The first target of a chain that is on screen. */
export function resolveFirst(
  targets: readonly FocusTarget[],
  trigger: Element | null = null,
): HTMLElement | null {
  for (const target of targets) {
    const element = resolveFocusTarget(target, trigger);
    if (element) return element;
  }
  return null;
}
