/**
 * The one overwrite ask (D12), as pure data: which report rows are askable,
 * the row the ask shows, and how a retry's rows replace the asked ones.
 *
 * Every `TARGET_EXISTS` producer answers a row carrying the pair identity
 * plus what the ask displays — the path, the folder's modified time and
 * whether it differs from the skill (`same_content`, never re-derived here).
 * A row is **global** (a (skill, Tool) pair, retried through the sync batch
 * or Enable with a per-pair `decision`) or **project** (an assignment,
 * retried through `resolveProjectCollisions`). The hooks own the ask and the
 * commands; this module owns only the shapes.
 */
import type {
  BatchSyncOverrideDto,
  BatchTargetOutcome,
  CommandError,
  InstallBatchReport,
  OccupantOwner,
  ProjectSyncOutcome,
  ProjectSyncReport,
  PropagationOutcome,
  RedeployAnswer,
  RedeployMove,
  RedeployReport,
  RefreshReport,
} from "../bindings";

/** Where an asked row's retry goes. */
export type OverwriteScope =
  | { kind: "global" }
  | { kind: "project"; projectId: string }
  /** A redeploy's row, answered through the next redeploy run. */
  | { kind: "redeploy" };

/** One occupied sync target the operator decides about. */
export type OverwriteRow = {
  skillId: string;
  skillName: string;
  toolKey: string;
  toolLabel: string;
  /** The occupied target path, as the backend reported it. */
  path: string;
  /** The existing folder's modification time (ms), when known. */
  modifiedAt: number | null;
  /** The folder is byte-identical to the skill (the backend's answer). */
  sameContent: boolean;
  /**
   * Who put the folder there (round 20 #70), as the backend judged it where
   * it observed the occupant: a recorded Deployment's artifact, or not ours.
   * Null for an edited copy, which is Skills Hub's old copy by its question.
   */
  owner: OccupantOwner | null;
  /** The owning Deployment's harness, labelled (null when not ours). */
  ownerToolLabel: string | null;
  scope: OverwriteScope;
  /**
   * What is asked (redeploy only): `occupied` — a folder in a move's way
   * (Overwrite / Keep); `editedCopy` — an old copy edited after Skills Hub
   * wrote it (Delete = `overwrite`, Leave = `keep`). Absent: occupied.
   */
  question?: "occupied" | "editedCopy";
  /**
   * Which question it is (redeploy only), as the backend asked it and an
   * answer must name it back: an occupied move's rows (`RedeployMove.rows`
   * ids), an edited copy's journal entry (`EditedCopy.entry_id`).
   */
  rowIds?: readonly string[];
  entryId?: string;
  /**
   * An edited copy asked **ahead** of its move (Sol 104): the move is held
   * by a folder in its way, asked in the same ask; `entryId` names the
   * moved row.
   */
  ahead?: boolean;
  /** The harnesses in which a left edited copy stays visible (labels). */
  readers?: string[];
  /** Of `readers`, those that also read the new place: shown twice (labels, Sol 102). */
  duplicateReaders?: string[];
};

/** The operator's per-row decision. */
export type OverwriteChoice = "overwrite" | "keep";

/** A row's identity: unique across global and project rows of one ask. */
export const overwriteRowKey = (row: OverwriteRow): string =>
  row.scope.kind === "project"
    ? `project\n${row.scope.projectId}\n${row.skillId}\n${row.toolKey}`
    : row.scope.kind === "redeploy"
      ? row.question === "editedCopy"
        ? `redeploy\neditedCopy\n${row.entryId ?? ""}`
        : `redeploy\noccupied\n${row.skillId}\n${row.toolKey}\n${row.path}`
      : `global\n${row.skillId}\n${row.toolKey}`;

type Labels = Readonly<Record<string, string>>;
type TargetExists = Extract<CommandError, { code: "TARGET_EXISTS" }>;

const targetExists = (error: CommandError): TargetExists | null =>
  error.code === "TARGET_EXISTS" ? error : null;

const facts = (error: TargetExists, labels: Labels) => ({
  path: error.path,
  modifiedAt: error.modifiedAt,
  sameContent: error.sameContent,
  owner: error.owner,
  ownerToolLabel: error.owner.kind === "deployment" ? (labels[error.owner.tool] ?? error.owner.tool) : null,
});

/**
 * The ask row a recorded collision raises again (the Notifications panel's
 * Overwrite… on a Propagation row, Astra B / Sol N1): the row and the path
 * the report kept, answered at that row and path (`resolveTargetCollisions`).
 */
export function recordedCollisionRow(
  skill: { skillId: string; skillName: string },
  tool: string,
  error: CommandError,
  toolLabelById: Labels,
): OverwriteRow | null {
  const exists = targetExists(error);
  if (!exists) return null;
  return {
    skillId: skill.skillId,
    skillName: skill.skillName,
    toolKey: tool,
    toolLabel: toolLabelById[tool] ?? tool,
    ...facts(exists, toolLabelById),
    scope: { kind: "global" },
  };
}

/** The askable rows of a global sync report (sync batch, Enable, install). */
export function globalCollisionRows(
  report: readonly BatchTargetOutcome[],
  toolLabelById: Labels,
): OverwriteRow[] {
  const rows: OverwriteRow[] = [];
  for (const result of report) {
    if (result.status.status !== "failed") continue;
    const error = targetExists(result.status.error);
    if (!error) continue;
    rows.push({
      skillId: result.skill_id,
      skillName: result.skill_name,
      toolKey: result.tool_key,
      toolLabel: toolLabelById[result.tool_key] ?? result.tool_key,
      ...facts(error, toolLabelById),
      scope: { kind: "global" },
    });
  }
  return rows;
}

/** The askable rows of a project sync report (assign, resync, Enable). */
export function projectCollisionRows(
  report: ProjectSyncReport,
  toolLabelById: Labels,
): OverwriteRow[] {
  const rows: OverwriteRow[] = [];
  for (const item of report.items) {
    if (item.status.status !== "failed") continue;
    const error = targetExists(item.status.error);
    if (!error) continue;
    rows.push({
      skillId: item.skill_id,
      skillName: item.skill_name,
      toolKey: item.tool,
      toolLabel: toolLabelById[item.tool] ?? item.tool,
      ...facts(error, toolLabelById),
      scope: { kind: "project", projectId: item.project_id },
    });
  }
  return rows;
}

/** Every installed candidate's target rows, in report order. */
export const installTargets = (report: InstallBatchReport): BatchTargetOutcome[] =>
  report.candidates.flatMap((candidate) =>
    candidate.status.status === "installed" ? candidate.status.targets : [],
  );

/**
 * The per-pair overrides for the asked global rows, each naming the path the
 * ask showed: the backend applies an answer only while the row's destination
 * is still that path.
 */
export function globalOverrides(
  rows: readonly OverwriteRow[],
  choices: readonly OverwriteChoice[],
): BatchSyncOverrideDto[] {
  return rows.flatMap((row, index) =>
    row.scope.kind === "global"
      ? [{ skill_id: row.skillId, tool: row.toolKey, path: row.path, decision: choices[index] }]
      : [],
  );
}

const pairKey = (skillId: string, toolKey: string) => `${skillId}\n${toolKey}`;

/**
 * `report` with each asked global (skill, Tool) row replaced by the retry's
 * row for that pair. Rows the retry produced for other pairs are dropped:
 * the report stays faithful to the operator's action.
 */
export function mergeGlobalRetry(
  report: readonly BatchTargetOutcome[],
  asked: readonly OverwriteRow[],
  retry: readonly BatchTargetOutcome[],
): BatchTargetOutcome[] {
  const askedKeys = new Set(
    asked.filter((row) => row.scope.kind === "global").map((row) => pairKey(row.skillId, row.toolKey)),
  );
  const retried = new Map<string, BatchTargetOutcome>();
  for (const result of retry) {
    const key = pairKey(result.skill_id, result.tool_key);
    if (askedKeys.has(key)) retried.set(key, result);
  }
  return report.map((result) => retried.get(pairKey(result.skill_id, result.tool_key)) ?? result);
}

/** The install report with its target rows merged the same way. */
export function mergeInstallRetry(
  report: InstallBatchReport,
  asked: readonly OverwriteRow[],
  retry: readonly BatchTargetOutcome[],
): InstallBatchReport {
  return {
    candidates: report.candidates.map((candidate) =>
      candidate.status.status === "installed"
        ? {
            ...candidate,
            status: {
              ...candidate.status,
              targets: mergeGlobalRetry(candidate.status.targets, asked, retry),
            },
          }
        : candidate,
    ),
  };
}

const assignmentKey = (item: Pick<ProjectSyncOutcome, "project_id" | "skill_id" | "tool">) =>
  `${item.project_id}\n${item.skill_id}\n${item.tool}`;

/**
 * A project report with each resolved assignment's row replaced. A project
 * the retry skipped whole (its folder went away before the answer arrived —
 * the backend answers no row for it, `folder_missing`) takes its unanswered
 * collisions out of the report and joins its `skipped_projects` once: the
 * skip line replaces the "to review" rows, never a silent success.
 */
export function mergeProjectRetry(
  report: ProjectSyncReport,
  retry: ProjectSyncReport,
): ProjectSyncReport {
  const resolved = new Map(retry.items.map((item) => [assignmentKey(item), item]));
  const skippedByRetry = new Set(retry.skipped_projects.map((skip) => skip.project_id));
  const touched = new Set<string>();
  const items: ProjectSyncOutcome[] = [];
  for (const item of report.items) {
    const answered = resolved.get(assignmentKey(item));
    if (answered) items.push(answered);
    else if (
      skippedByRetry.has(item.project_id) &&
      item.status.status === "failed" &&
      targetExists(item.status.error)
    )
      touched.add(item.project_id);
    else items.push(item);
  }
  const skipped_projects = [...report.skipped_projects];
  for (const skip of retry.skipped_projects) {
    if (touched.has(skip.project_id) && !skipped_projects.some((s) => s.project_id === skip.project_id))
      skipped_projects.push(skip);
  }
  return { ...report, items, skipped_projects };
}

// ---------------------------------------------------------------------------
// Refresh / Update / Restore / Re-point / Edit: Propagation
// ---------------------------------------------------------------------------

/**
 * What an answered ask's retries settled, for the asked rows only: global
 * pairs as the sync batch (or Enable) reported them, project assignments as
 * `resolveProjectCollisions` did. `rows` are the asked rows in ask order.
 */
export type CollisionRetry = {
  rows: readonly OverwriteRow[];
  global: readonly BatchTargetOutcome[];
  projects: ProjectSyncReport | null;
};

/** A pair the operator kept ("Keep existing"), for the fold's quiet note. */
export type DeclinedPair = { skillName: string; tool: string };

/**
 * A pair whose folder the operator's Overwrite replaced (round 20 #70): an
 * outcome kind beside `declined`, recorded in the run with the path it
 * replaced, for the shapes whose report has no such row (Propagation).
 */
export type OverwrittenPair = { skillName: string; tool: string; path: string };

/**
 * The askable rows of one skill's Sync targets as Propagation settled them
 * (a `TARGET_EXISTS` from the first sync of a row that owns nothing): a
 * global row is a (skill, Tool) pair, a project row an assignment.
 */
export function propagationCollisionRows(
  skill: { skillId: string; skillName: string },
  targets: readonly PropagationOutcome[],
  toolLabelById: Labels,
): OverwriteRow[] {
  const rows: OverwriteRow[] = [];
  for (const target of targets) {
    if (target.status.status !== "failed") continue;
    const error = targetExists(target.status.error);
    if (!error) continue;
    const tool = target.scope.tool;
    rows.push({
      skillId: skill.skillId,
      skillName: skill.skillName,
      toolKey: tool,
      toolLabel: toolLabelById[tool] ?? tool,
      ...facts(error, toolLabelById),
      scope:
        target.scope.scope === "project"
          ? { kind: "project", projectId: target.scope.project_id }
          : { kind: "global" },
    });
  }
  return rows;
}

/** The askable rows of a Refresh / Update report: every refreshed skill's targets. */
export function refreshCollisionRows(
  report: RefreshReport,
  toolLabelById: Labels,
): OverwriteRow[] {
  return report.skills.flatMap((skill) =>
    skill.status.status === "refreshed"
      ? propagationCollisionRows(
          { skillId: skill.skill_id, skillName: skill.skill_name },
          skill.status.targets,
          toolLabelById,
        )
      : [],
  );
}

const rowKeyOf = (skillId: string, scope: PropagationOutcome["scope"]) =>
  scope.scope === "project"
    ? `project\n${scope.project_id}\n${skillId}\n${scope.tool}`
    : `global\n${skillId}\n${scope.tool}`;

/**
 * One skill's targets with the answered rows settled. An asked row stays in
 * the report only when its retry failed (with the retry's error); a kept one
 * ("Keep existing") leaves it for `declined`; one that synced or was skipped
 * leaves it too — the Propagation folds count only failures, and a project
 * resolution carries no sync mode to report. Rows nobody asked about, and
 * asked rows the retry did not answer, are unchanged.
 */
export function mergePropagationRetry(
  skill: { skillId: string; skillName: string },
  targets: readonly PropagationOutcome[],
  retry: CollisionRetry,
): { targets: PropagationOutcome[]; declined: DeclinedPair[]; overwritten: OverwrittenPair[] } {
  const asked = new Set(retry.rows.map(overwriteRowKey));
  const global = new Map(retry.global.map((row) => [pairKey(row.skill_id, row.tool_key), row.status]));
  const projects = new Map((retry.projects?.items ?? []).map((item) => [assignmentKey(item), item.status]));
  const projectsSkipped = new Set((retry.projects?.skipped_projects ?? []).map((skip) => skip.project_id));
  const declined: DeclinedPair[] = [];
  const overwritten: OverwrittenPair[] = [];
  const out: PropagationOutcome[] = [];
  for (const target of targets) {
    if (!asked.has(rowKeyOf(skill.skillId, target.scope))) {
      out.push(target);
      continue;
    }
    const settled =
      target.scope.scope === "project"
        ? projects.get(assignmentKey({ project_id: target.scope.project_id, skill_id: skill.skillId, tool: target.scope.tool }))
        : global.get(pairKey(skill.skillId, target.scope.tool));
    if (!settled) {
      // The retry did not enter a project whose folder went away: the row
      // is what Propagation itself answers for it, never a collision again.
      if (target.scope.scope === "project" && projectsSkipped.has(target.scope.project_id))
        out.push({
          ...target,
          status: { status: "skipped", reason: { reason: "project_unavailable", project_id: target.scope.project_id } },
        });
      else out.push(target);
      continue;
    }
    if (settled.status === "declined") {
      declined.push({ skillName: skill.skillName, tool: target.scope.tool });
    } else if (settled.status === "overwritten") {
      overwritten.push({
        skillName: skill.skillName,
        tool: target.scope.tool,
        path: "outcome" in settled ? settled.outcome.target_path : settled.path,
      });
    } else if (settled.status === "failed") {
      // The retry wrote the same row: still a first deploy if the asked one was.
      const firstDeploy = target.status.status === "failed" && target.status.first_deploy;
      out.push({ ...target, status: { status: "failed", error: settled.error, first_deploy: firstDeploy } });
    }
  }
  return { targets: out, declined, overwritten };
}

/** A Refresh / Update report with every refreshed skill's answered rows settled. */
export function mergeRefreshRetry(
  report: RefreshReport,
  retry: CollisionRetry,
): { report: RefreshReport; declined: DeclinedPair[]; overwritten: OverwrittenPair[] } {
  const declined: DeclinedPair[] = [];
  const overwritten: OverwrittenPair[] = [];
  const skills = report.skills.map((skill) => {
    if (skill.status.status !== "refreshed") return skill;
    const merged = mergePropagationRetry(
      { skillId: skill.skill_id, skillName: skill.skill_name },
      skill.status.targets,
      retry,
    );
    declined.push(...merged.declined);
    overwritten.push(...merged.overwritten);
    return { ...skill, status: { ...skill.status, targets: merged.targets } };
  });
  return { report: { skills }, declined, overwritten };
}

// ---------------------------------------------------------------------------
// Redeploy (round 20 #108): folders in a move's way and edited old copies,
// one ask
// ---------------------------------------------------------------------------

/**
 * The askable rows of a redeploy report: every move a different folder
 * stands in the way of (`TARGET_EXISTS`, keyed by the move's folder) and
 * every edited old copy awaiting its answer (keyed by its row's key) — one
 * ask, answered per row.
 */
export function redeployAskRows(report: RedeployReport, toolLabelById: Labels): OverwriteRow[] {
  const label = (key: string) => toolLabelById[key] ?? key;
  const rows: OverwriteRow[] = [];
  for (const move of report.moves) {
    if (move.status.status !== "failed") continue;
    const error = targetExists(move.status.error);
    if (!error) continue;
    rows.push({
      skillId: move.skill_id,
      skillName: move.skill_name,
      toolKey: move.folder_key,
      toolLabel: label(move.folder_key),
      ...facts(error, toolLabelById),
      scope: { kind: "redeploy" },
      question: "occupied",
      rowIds: move.rows.map((row) => row.id),
    });
  }
  for (const copy of report.edited_copies) {
    rows.push({
      skillId: copy.skill_id,
      skillName: copy.skill_name,
      toolKey: copy.tool,
      toolLabel: label(copy.tool),
      path: copy.path,
      modifiedAt: copy.modified_at,
      sameContent: false,
      owner: null,
      ownerToolLabel: null,
      scope: { kind: "redeploy" },
      question: "editedCopy",
      entryId: copy.entry_id,
      ...(copy.ahead ? { ahead: true } : {}),
      readers: copy.readers.map(label),
      duplicateReaders: copy.duplicate_readers.map(label),
    });
  }
  return rows;
}

/**
 * The answers for the asked redeploy rows, each naming exactly the question
 * it answers — the move's rows or the edited copy's journal entry — and the
 * path it showed; the backend refuses one whose question changed since.
 */
export function redeployAnswers(
  rows: readonly OverwriteRow[],
  choices: readonly OverwriteChoice[],
): RedeployAnswer[] {
  return rows.flatMap((row, index): RedeployAnswer[] => {
    if (row.scope.kind !== "redeploy") return [];
    const choice = choices[index];
    return row.question === "editedCopy"
      ? [
          {
            question: "edited_copy",
            entry_id: row.entryId ?? "",
            path: row.path,
            decision: choice === "overwrite" ? "delete" : "leave",
          },
        ]
      : [
          {
            question: "occupied",
            skill_id: row.skillId,
            folder_key: row.toolKey,
            row_ids: [...(row.rowIds ?? [])],
            path: row.path,
            decision: choice,
          },
        ];
  });
}

const moveKey = (move: Pick<RedeployMove, "skill_id" | "folder_key">) => `${move.skill_id}\n${move.folder_key}`;

/**
 * One operator action's redeploy report, after the answered retry: each
 * asked move replaced by the retry's move for it; an edited copy the retry
 * settled no longer awaits, and its move reads `moved` once the copy was
 * deleted (`left_old` when it was left); a journal entry the first run left
 * unfinished reads as the retry settled it; everything else the retry found
 * (its journal, left-behind copies, lost disables, copies still awaiting,
 * answers it refused as stale) joins the report.
 */
export function mergeRedeployRetry(first: RedeployReport, retry: RedeployReport): RedeployReport {
  const retried = new Map(retry.moves.map((move) => [moveKey(move), move]));
  // Settlement is associated by the journal entry each row left (Astra,
  // round 2), never by (skill, tool, path): two entries may share those.
  const settled = new Map(retry.journal.map((entry) => [entry.entry_id, entry.status]));
  const moves = first.moves.map((move) => {
    const again = retried.get(moveKey(move));
    if (again) return again;
    if (move.status.status !== "left_old") return move;
    const outcomes = move.rows
      .map((row) => (row.entry_id === null ? undefined : settled.get(row.entry_id)))
      .filter((status) => status !== undefined);
    if (outcomes.length > 0 && outcomes.every((status) => status === "finished" || status === "retained"))
      return { ...move, status: { status: "moved" as const } };
    return move;
  });
  const firstKeys = new Set(first.moves.map(moveKey));
  // An entry the first run left unfinished is settled again by the retry: its outcome is the retry's.
  const resettled = new Set(
    retry.journal.filter((entry) => entry.status !== "awaiting_answer").map((entry) => entry.entry_id),
  );
  return {
    moves: [...moves, ...retry.moves.filter((move) => !firstKeys.has(moveKey(move)))],
    journal: [
      ...first.journal.filter((entry) => !(entry.status === "unfinished" && resettled.has(entry.entry_id))),
      ...retry.journal.filter((entry) => entry.status !== "awaiting_answer"),
    ],
    left_behind: [...first.left_behind, ...retry.left_behind],
    edited_copies: retry.edited_copies,
    lost_disables: [...first.lost_disables, ...retry.lost_disables],
    refused_answers: [...first.refused_answers, ...retry.refused_answers],
  };
}

// ---------------------------------------------------------------------------
// What the ask says per row (round 20 #70): who put the folder there, what
// each answer does, and a label that tells two rows apart
// ---------------------------------------------------------------------------

type Translate = (key: string, options?: Record<string, unknown>) => string;

/** The words of one ask row. */
export type AskRowCopy = {
  /** Who put the folder there: Skills Hub (naming the Deployment) or not ours. */
  provenance: string;
  /** The two answers, short (the per-row control's items and the buttons). */
  keepLabel: string;
  overwriteLabel: string;
  /**
   * What each answer does, as one sentence each (D12: the operator predicts
   * the result). An edited copy asked ahead of its move says each happens
   * only if the move goes ahead.
   */
  keep: string;
  overwrite: string;
  /**
   * Why neither answer does anything now (null: the row's answer applies):
   * an edited copy asked ahead of a move whose folder in the way is answered
   * Keep — the move does not run, so its old copy is neither deleted nor
   * left (`redeploy::execute` returns at the Keep answer, before the step
   * that settles old copies).
   */
  voided: string | null;
};

const readerList = (readers: readonly string[], t: Translate) => readers.join(t("common.listSeparator"));

/**
 * The words one ask row shows, from the backend's facts alone: the owner it
 * judged, the scope the answer goes to, and — an edited copy — the readers it
 * reported, a proven duplicate ("shows twice in …") kept apart from a copy
 * that is merely still visible (Sol 102).
 */
export function askRowCopy(row: OverwriteRow, t: Translate, opts: { moveKept?: boolean } = {}): AskRowCopy {
  const names = { name: row.skillName, tool: row.toolLabel };
  if (row.question === "editedCopy") {
    const duplicates = row.duplicateReaders ?? [];
    const visible = (row.readers ?? []).filter((reader) => !duplicates.includes(reader));
    const leaveParts = [
      ...(duplicates.length > 0 ? [t("overwriteAsk.leaveShowsTwice", { readers: readerList(duplicates, t) })] : []),
      ...(visible.length > 0 ? [t("overwriteAsk.leaveStillVisible", { readers: readerList(visible, t) })] : []),
    ];
    return {
      provenance: t(row.ahead ? "overwriteAsk.owner.editedAhead" : "overwriteAsk.owner.edited", names),
      keepLabel: t("overwriteAsk.leave"),
      overwriteLabel: t("overwriteAsk.delete"),
      keep: t(row.ahead ? "overwriteAsk.consequence.leaveAhead" : "overwriteAsk.consequence.leave", {
        ...names,
        where: leaveParts.length > 0 ? leaveParts.join("; ") : t("overwriteAsk.leaveNowhere"),
      }),
      overwrite: t(row.ahead ? "overwriteAsk.consequence.deleteAhead" : "overwriteAsk.consequence.delete", names),
      voided: row.ahead && opts.moveKept ? t("overwriteAsk.consequence.aheadKept", names) : null,
    };
  }
  const owner = row.owner;
  const provenance =
    owner?.kind === "deployment"
      ? t(owner.projectId === null ? "overwriteAsk.owner.ours" : "overwriteAsk.owner.oursInProject", {
          owner: owner.skillName,
          ownerTool: row.ownerToolLabel ?? owner.tool,
        })
      : t("overwriteAsk.owner.foreign");
  const overwrite =
    owner?.kind === "deployment"
      ? t(
          row.scope.kind === "redeploy" ? "overwriteAsk.consequence.moveOverOurs" : "overwriteAsk.consequence.replaceOurs",
          { ...names, owner: owner.skillName, ownerTool: row.ownerToolLabel ?? owner.tool },
        )
      : t(row.scope.kind === "redeploy" ? "overwriteAsk.consequence.moveOver" : "overwriteAsk.consequence.replace", names);
  const keep = t(
    row.scope.kind === "redeploy"
      ? "overwriteAsk.consequence.keepMove"
      : row.scope.kind === "project"
        ? "overwriteAsk.consequence.keepProject"
        : "overwriteAsk.consequence.keep",
    names,
  );
  return {
    provenance,
    keepLabel: t("overwriteAsk.keep"),
    overwriteLabel: t("overwriteAsk.overwrite"),
    keep,
    overwrite,
    voided: null,
  };
}

/**
 * For each row, the row asking about the folder in its move's way when it
 * is an edited copy asked **ahead** of that move (Sol 104): the occupied
 * redeploy row whose move holds the moved row the copy names (`entryId` ∈
 * `rowIds`) — its answer decides whether this one applies. Null otherwise.
 */
export function aheadMoveRows(rows: readonly OverwriteRow[]): (number | null)[] {
  return rows.map((row) => {
    if (row.question !== "editedCopy" || !row.ahead) return null;
    const index = rows.findIndex(
      (other) =>
        other.scope.kind === "redeploy" &&
        other.question !== "editedCopy" &&
        (other.rowIds ?? []).includes(row.entryId ?? ""),
    );
    return index === -1 ? null : index;
  });
}

/**
 * The ask's lines about its edited copies: one for copies whose skill has
 * moved already (it works from its new folder), one for copies asked ahead
 * of a move a folder holds (their answer applies only if it goes ahead).
 */
export function editedCopyHints(rows: readonly OverwriteRow[], t: Translate): string[] {
  const edited = rows.filter((row) => row.question === "editedCopy");
  return [
    ...(edited.some((row) => !row.ahead) ? [t("overwriteAsk.editedHint")] : []),
    ...(edited.some((row) => row.ahead) ? [t("overwriteAsk.editedAheadHint")] : []),
  ];
}

/**
 * Each row's accessible name, distinct across the ask (Sol 112): skill,
 * harness and path — an edited copy says it is the old copy — and, should two
 * rows still read alike, their order ("2 of 2").
 */
export function askRowLabels(rows: readonly OverwriteRow[], t: Translate): string[] {
  const base = rows.map((row) =>
    t(row.question === "editedCopy" ? "overwriteAsk.rowLabelEdited" : "overwriteAsk.rowLabel", {
      name: row.skillName,
      tool: row.toolLabel,
      path: row.path,
    }),
  );
  return base.map((label, index) => {
    const same = base.filter((other) => other === label).length;
    if (same === 1) return label;
    const nth = base.slice(0, index + 1).filter((other) => other === label).length;
    return t("overwriteAsk.rowLabelNth", { label, index: nth, count: same });
  });
}
