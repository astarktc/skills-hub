/**
 * The Projects surface's pure presentation (ticket 17): what one matrix cell
 * says — its state, the Synced strip's mark for it, the word shown on focus
 * and hover, its accessible name and its repair — the column headers' Tool
 * labels, the matrix chips' rules and the legend. One fold, so the cell, the
 * legend and the chips never disagree; the marks are the Synced strip's
 * vocabulary (`StripMarkState`, drawn by `StripMarkIcon`).
 */
import type { ProjectSkillAssignmentDto } from "../bindings";
import { isLiveTarget, type StripMarkState } from "./skillPresentation";

type Translate = (key: string, options?: Record<string, unknown>) => string;

/** What one skill × Tool cell of a project holds. */
export type MatrixCellState =
  | "unassigned"
  | "synced"
  | "stale"
  | "missing"
  | "pending"
  | "failed"
  | "disabled";

/** The assigned states, in legend order. */
export const MATRIX_STATES = ["synced", "stale", "missing", "pending", "failed", "disabled"] as const;
export type AssignedCellState = (typeof MATRIX_STATES)[number];

/**
 * Each assigned state's mark — the Synced strip's shapes, so the matrix and
 * the strip speak one vocabulary: filled synced, half-filled stale, ×
 * missing / failed, hollow pending (assigned, not synced yet), slashed
 * disabled. The word tells × apart.
 */
export const MATRIX_CELL_MARK: Readonly<Record<AssignedCellState, StripMarkState>> = {
  synced: "synced",
  stale: "stale",
  missing: "missing",
  pending: "not_synced",
  failed: "failed",
  disabled: "disabled",
};

/** A cell's repair: re-sync this one assignment (stale, missing, failed). */
export type MatrixCellRepair = "sync";

export type MatrixCell = {
  state: MatrixCellState;
  /** The assignment checkbox's value. */
  assigned: boolean;
  /** The mark drawn beside the checkbox; null for an unassigned cell. */
  mark: StripMarkState | null;
  /** The state word shown on focus and hover ("stale"); null when unassigned. */
  word: string | null;
  /** What the state means, in words — the title and part of the name. */
  detail: string | null;
  /** The checkbox's accessible name: skill, Tool, state, detail, "also global". */
  name: string;
  /**
   * D17: the skill also has a live global target on this Tool. The cell
   * stays enabled (a project assignment and a global target coexist) and
   * is marked "also global".
   */
  alsoGlobal: boolean;
  /** The cell's own repair (**Sync this**); kept while it runs, gone once the cell is healthy. */
  repairs: readonly MatrixCellRepair[];
  /** The running action is about this cell. */
  inFlight: boolean;
};

export type MatrixCellInput = {
  skill: {
    name: string;
    disabled_at: number | null;
    targets: readonly { tool: string; status: ProjectSkillAssignmentDto["status"]; synced_at: number | null }[];
  };
  /** The project Tool: its key and the Tool catalog's label. */
  tool: { key: string; label: string };
  assignment: Pick<ProjectSkillAssignmentDto, "status" | "last_error"> | undefined;
  /** The running action is about this cell (the reporter's pending cells). */
  inFlight: boolean;
};

const STATE_OF: Record<ProjectSkillAssignmentDto["status"], AssignedCellState> = {
  synced: "synced",
  stale: "stale",
  missing: "missing",
  pending: "pending",
  error: "failed",
  disabled: "disabled",
};

const REPAIRABLE: ReadonlySet<MatrixCellState> = new Set(["stale", "missing", "failed"]);

/** One matrix cell, folded once. */
export function matrixCell({ skill, tool, assignment, inFlight }: MatrixCellInput, t: Translate): MatrixCell {
  // A skill disabled at skill grain is off everywhere: its assignments
  // read disabled whatever their row says (a failed Disable keeps `error`).
  const state: MatrixCellState = !assignment
    ? "unassigned"
    : skill.disabled_at !== null
      ? "disabled"
      : STATE_OF[assignment.status];
  const alsoGlobal = skill.targets.some((target) => target.tool === tool.key && isLiveTarget(target));
  const assigned = state !== "unassigned";
  const word = inFlight
    ? t("projectsSurface.cell.syncing")
    : assigned
      ? t(`projectsSurface.state.${state}`)
      : null;
  const detail = inFlight
    ? null
    : state === "failed"
      ? assignment?.last_error
        ? t("projectsSurface.detail.failedWith", { error: assignment.last_error })
        : t("projectsSurface.detail.failed")
      : state === "disabled" && skill.disabled_at !== null
        ? t("projectsSurface.detail.skillDisabled")
        : assigned
          ? t(`projectsSurface.detail.${state}`)
          : null;
  const parts = [word ?? t("projectsSurface.cell.notAssigned"), detail].filter(Boolean).join(" \u2014 ");
  const name = [
    t("projectsSurface.cell.name", { skill: skill.name, tool: tool.label, state: parts }),
    ...(alsoGlobal ? [t("projectsSurface.cell.alsoGlobal")] : []),
  ].join(", ");
  return {
    state,
    assigned,
    mark: assigned ? MATRIX_CELL_MARK[state as AssignedCellState] : null,
    word,
    detail,
    name,
    alsoGlobal,
    // Kept while the cell's own action runs (its button pends and holds focus).
    repairs: REPAIRABLE.has(state) ? ["sync"] : [],
    inFlight,
  };
}

/** A project Tool's column header: the catalog's label, and whether detection sees it. */
export type MatrixColumn = { key: string; label: string; detected: boolean };

export function matrixColumns(
  tools: readonly { tool: string }[],
  labelOf: (tool: string) => string,
  /** Installed project Tool keys; null while the catalog has not loaded (nothing is called "not detected"). */
  installed: readonly string[] | null,
): MatrixColumn[] {
  return tools.map(({ tool }) => ({
    key: tool,
    label: labelOf(tool),
    detected: installed === null || installed.includes(tool),
  }));
}

/** The matrix chips: Assigned · Issues (All is the lens's "all"). */
export type MatrixFilter = "all" | "assigned" | "issues";

const ISSUE_STATUSES: ReadonlySet<ProjectSkillAssignmentDto["status"]> = new Set(["stale", "missing", "error"]);

/**
 * Each chip's rule over the project's assignment rows: Assigned — the skill
 * holds any assignment here; Issues — any of them is stale, missing or
 * failed (what **Sync this** repairs). A disabled skill has no issue here.
 */
export function matrixFilters(
  assignments: readonly Pick<ProjectSkillAssignmentDto, "skill_id" | "status">[],
): Readonly<Record<Exclude<MatrixFilter, "all">, (skill: { id: string; disabled_at: number | null }) => boolean>> {
  const assigned = new Set<string>();
  const issues = new Set<string>();
  for (const row of assignments) {
    assigned.add(row.skill_id);
    if (ISSUE_STATUSES.has(row.status)) issues.add(row.skill_id);
  }
  return {
    assigned: (skill) => assigned.has(skill.id),
    issues: (skill) => skill.disabled_at === null && issues.has(skill.id),
  };
}

/** The legend, from the same vocabulary as the cells: mark + word + meaning. */
export function matrixLegend(t: Translate): { state: AssignedCellState; mark: StripMarkState; word: string; detail: string }[] {
  return MATRIX_STATES.map((state) => ({
    state,
    mark: MATRIX_CELL_MARK[state],
    word: t(`projectsSurface.state.${state}`),
    detail: t(`projectsSurface.detail.${state}`),
  }));
}

/**
 * Whether the Assign-to-project picker offers a project, and the reason in
 * words when it does not. Precedence: a missing folder (the backend skips the
 * project whole — this is the reason shown, not the safety rule), then no
 * configured Tools, then every selected skill already on every Tool.
 */
export type AssignTargetState = "available" | "folderMissing" | "noTools" | "assigned";

export function assignTargetState({
  pathExists,
  toolCount,
  missingToolCount,
}: {
  pathExists: boolean;
  toolCount: number;
  /** Tools of the project at least one selected skill is not on yet. */
  missingToolCount: number;
}): AssignTargetState {
  if (!pathExists) return "folderMissing";
  if (toolCount === 0) return "noTools";
  if (missingToolCount === 0) return "assigned";
  return "available";
}

/** Where the matrix puts focus when the row holding it left the shown rows. */
export type MatrixFocusTarget = { kind: "row"; skillId: string } | { kind: "filter" };

/**
 * N2 (ticket 17 audit): a repair or an unassign under a chip (Issues,
 * Assigned) can take the row holding focus out of the shown rows. Focus then
 * goes to the next surviving row (in display order), the previous one if the
 * row was last, and the active chip when no row is left. `null` when the row
 * is still shown — the cell's own rule applies.
 */
export function matrixFocusAfterLeave(
  before: readonly string[],
  after: readonly string[],
  left: string,
): MatrixFocusTarget | null {
  if (after.includes(left)) return null;
  const shown = new Set(after);
  const index = before.indexOf(left);
  const next =
    index < 0
      ? after[0]
      : (before.slice(index + 1).find((id) => shown.has(id)) ??
        before
          .slice(0, index)
          .reverse()
          .find((id) => shown.has(id)) ??
        after[0]);
  return next === undefined ? { kind: "filter" } : { kind: "row", skillId: next };
}
