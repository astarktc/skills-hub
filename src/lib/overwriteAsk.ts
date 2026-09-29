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
  ProjectSyncOutcome,
  ProjectSyncReport,
  PropagationOutcome,
  RefreshReport,
} from "../bindings";

/** Where an asked row's retry goes. */
export type OverwriteScope =
  | { kind: "global" }
  | { kind: "project"; projectId: string };

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
  scope: OverwriteScope;
};

/** The operator's per-row decision. */
export type OverwriteChoice = "overwrite" | "keep";

/** A row's identity: unique across global and project rows of one ask. */
export const overwriteRowKey = (row: OverwriteRow): string =>
  row.scope.kind === "project"
    ? `project\n${row.scope.projectId}\n${row.skillId}\n${row.toolKey}`
    : `global\n${row.skillId}\n${row.toolKey}`;

type Labels = Readonly<Record<string, string>>;
type TargetExists = Extract<CommandError, { code: "TARGET_EXISTS" }>;

const targetExists = (error: CommandError): TargetExists | null =>
  error.code === "TARGET_EXISTS" ? error : null;

const facts = (error: TargetExists) => ({
  path: error.path,
  modifiedAt: error.modifiedAt,
  sameContent: error.sameContent,
});

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
      ...facts(error),
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
      ...facts(error),
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

/** The per-pair overrides for the asked global rows. */
export function globalOverrides(
  rows: readonly OverwriteRow[],
  choices: readonly OverwriteChoice[],
): BatchSyncOverrideDto[] {
  return rows.flatMap((row, index) =>
    row.scope.kind === "global"
      ? [{ skill_id: row.skillId, tool: row.toolKey, decision: choices[index] }]
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

/** A project report with each resolved assignment's row replaced. */
export function mergeProjectRetry(
  report: ProjectSyncReport,
  retry: ProjectSyncReport,
): ProjectSyncReport {
  const resolved = new Map(retry.items.map((item) => [assignmentKey(item), item]));
  return {
    items: report.items.map((item) => resolved.get(assignmentKey(item)) ?? item),
  };
}

// ---------------------------------------------------------------------------
// Refresh / Update / Restore / Re-point / Edit: Propagation and the re-assert
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
 * The askable rows of one skill's Sync targets as Propagation (and the
 * auto-sync re-assert, which reports in the same shape) settled them: a
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
      ...facts(error),
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
): { targets: PropagationOutcome[]; declined: DeclinedPair[] } {
  const asked = new Set(retry.rows.map(overwriteRowKey));
  const global = new Map(retry.global.map((row) => [pairKey(row.skill_id, row.tool_key), row.status]));
  const projects = new Map((retry.projects?.items ?? []).map((item) => [assignmentKey(item), item.status]));
  const declined: DeclinedPair[] = [];
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
      out.push(target);
      continue;
    }
    if (settled.status === "declined") {
      declined.push({ skillName: skill.skillName, tool: target.scope.tool });
    } else if (settled.status === "failed") {
      out.push({ scope: target.scope, status: { status: "failed", error: settled.error } });
    }
  }
  return { targets: out, declined };
}

/** A Refresh / Update report with every refreshed skill's answered rows settled. */
export function mergeRefreshRetry(
  report: RefreshReport,
  retry: CollisionRetry,
): { report: RefreshReport; declined: DeclinedPair[] } {
  const declined: DeclinedPair[] = [];
  const skills = report.skills.map((skill) => {
    if (skill.status.status !== "refreshed") return skill;
    const merged = mergePropagationRetry(
      { skillId: skill.skill_id, skillName: skill.skill_name },
      skill.status.targets,
      retry,
    );
    declined.push(...merged.declined);
    return { ...skill, status: { ...skill.status, targets: merged.targets } };
  });
  return { report: { skills }, declined };
}
