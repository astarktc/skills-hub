/**
 * The counts a wide verb states before it runs (D8, D16; ticket 13): pure
 * folds over the selected skills, the Tool catalog's labels and the project
 * world's placements. The bulk rail's count-bearing confirmation, the row
 * switch's inline Enable confirmation and the Delete dialog read them; each
 * counts exactly what the backend's settlement for that verb touches:
 *
 * - **Disable** (skill grain, `DisableScope::Skills`): every Sync target row
 *   the app materialised (`synced_at` set) and not already `disabled` is
 *   removed and kept `disabled`; every project assignment that is not
 *   `disabled` is disabled too; never-synced rows only flip.
 * - **Enable**: every `disabled` target row and assignment re-syncs (a first
 *   sync); the skill-grain mark clears.
 * - **Sync to Tools**: skills × picked Tools, less the pairs already synced
 *   and the pairs a Disable gates (skipped as report data, never failed).
 * - **Update**: the skills Refresh can re-acquire; the rest are skipped.
 * - **Deploy to a harness just turned on** (D48): the enabled, locatable skills
 *   with no deployment there yet (`harnessArrivals`).
 * - **Delete**: the Tools and projects that carry each skill, the Edits that
 *   go with it, and whether Skills Hub's copy is the only one.
 *
 * Disable's and Enable's confirmations also take their sentences from here
 * (`disableConfirmLines`, `enableConfirmLines`): which sentence a count calls
 * for — a zero, an unread placement — is decided beside the count.
 */

import type { ManagedSkillDto as ManagedSkill, ProjectSkillAssignmentDto, SkillTargetDto } from "../bindings";

/** A skill's assignment rows across projects, by project; null while unread. */
export type AssignmentsOf = (skillId: string) => readonly SkillProjectRows[] | null;
export type SkillProjectRows = {
  projectId: string;
  projectName: string;
  assignments: readonly Pick<ProjectSkillAssignmentDto, "tool" | "status">[];
};

const ownsArtifact = (target: Pick<SkillTargetDto, "synced_at">) => target.synced_at !== null;

/**
 * Across the selection, how many project assignment rows match `wanted`;
 * null when any skill's placement has not been read (the count is unknown,
 * never a guessed zero).
 */
function countAssignments(
  skills: readonly ManagedSkill[],
  assignmentsOf: AssignmentsOf | null,
  wanted: (status: ProjectSkillAssignmentDto["status"]) => boolean,
): number | null {
  if (!assignmentsOf) return null;
  let count = 0;
  for (const skill of skills) {
    const rows = assignmentsOf(skill.id);
    if (rows === null) return null;
    for (const project of rows) count += project.assignments.filter((row) => wanted(row.status)).length;
  }
  return count;
}

export type DisableCounts = {
  /** Skills in the selection. */
  skills: number;
  /** Skills that are already off everywhere (skill grain): nothing changes for them. */
  alreadyDisabled: number;
  /** Distinct Tools whose materialised targets are removed. */
  tools: number;
  /** Sync targets removed and kept `disabled`. */
  targets: number;
  /** Project assignments disabled; null while the placements are unread. */
  assignments: number | null;
  /**
   * Every project assignment of the selection, already disabled ones
   * included — so a zero `assignments` can tell "assigned nowhere" from
   * "already off"; null while unread.
   */
  assigned: number | null;
};

/** Disable on all Tools, at skill grain, over the selection. */
export function disableCounts(skills: readonly ManagedSkill[], assignmentsOf: AssignmentsOf | null): DisableCounts {
  const tools = new Set<string>();
  let targets = 0;
  for (const skill of skills) {
    for (const target of skill.targets) {
      if (target.status === "disabled" || !ownsArtifact(target)) continue;
      targets++;
      tools.add(target.tool);
    }
  }
  return {
    skills: skills.length,
    alreadyDisabled: skills.filter((skill) => skill.disabled_at !== null).length,
    tools: tools.size,
    targets,
    assignments: countAssignments(skills, assignmentsOf, (status) => status !== "disabled"),
    assigned: countAssignments(skills, assignmentsOf, () => true),
  };
}

export type EnableCounts = {
  skills: number;
  /**
   * Skills with nothing disabled at either grain: Enable changes nothing for
   * them. With placements read, a disabled project assignment counts as
   * something to enable; without them (`assignmentsOf` null — the rail's and
   * palette's refusal) only the skill marker and global targets are seen.
   */
  alreadyEnabled: number;
  /** Distinct Tools that are synced again. */
  tools: number;
  /** Sync targets synced again (a first sync each). */
  targets: number;
  /** Project assignments enabled again; null while unread. */
  assignments: number | null;
};

/** Enable over the selection: every disabled row re-syncs. */
export function enableCounts(skills: readonly ManagedSkill[], assignmentsOf: AssignmentsOf | null): EnableCounts {
  const tools = new Set<string>();
  let targets = 0;
  let alreadyEnabled = 0;
  for (const skill of skills) {
    const disabled = skill.targets.filter((target) => target.status === "disabled");
    const disabledAssignment =
      assignmentsOf?.(skill.id)?.some((project) => project.assignments.some((row) => row.status === "disabled")) ??
      false;
    if (skill.disabled_at === null && disabled.length === 0 && !disabledAssignment) alreadyEnabled++;
    for (const target of disabled) {
      targets++;
      tools.add(target.tool);
    }
  }
  return {
    skills: skills.length,
    alreadyEnabled,
    tools: tools.size,
    targets,
    assignments: countAssignments(skills, assignmentsOf, (status) => status === "disabled"),
  };
}

/** The translate function the sentence folds take (i18next's `t` fits). */
type Translate = (key: string, opts?: Record<string, unknown>) => string;

const phrase = (t: Translate, unit: "skills" | "tools" | "targets" | "assignments", count: number) =>
  t(`bulk.count.${unit}`, { count });

/**
 * The Disable confirmation's sentences (BACKLOG #73), in order: what leaves
 * the harnesses and what turns off in projects (one line), "Reading project
 * deployments…" while the placements are unread, that the skills stay in
 * the library switched off, and how many are off already. A zero says what
 * it means — not deployed anywhere, assigned to no project, or every project
 * deployment already off — instead of a "0 …" count.
 */
export function disableConfirmLines(counts: DisableCounts, t: Translate): string[] {
  const { skills, targets, tools, assignments, assigned } = counts;
  const harnesses =
    targets > 0
      ? t("bulk.confirm.disable.removes", { targets: phrase(t, "targets", targets), tools: phrase(t, "tools", tools) })
      : t("bulk.confirm.disable.notDeployed", { count: skills });
  const projects =
    assignments === null
      ? null
      : assignments > 0
        ? t("bulk.confirm.disable.turnsOff", { assignments: phrase(t, "assignments", assignments) })
        : assigned === 0
          ? t("bulk.confirm.disable.noAssignments", { count: skills })
          : t("bulk.confirm.disable.assignmentsAlreadyOff", { count: skills });
  return [
    projects ? `${harnesses} ${projects}` : harnesses,
    ...(assignments === null ? [t("bulk.confirm.readingProjects")] : []),
    t("bulk.confirm.disable.stays", { count: skills }),
    ...(counts.alreadyDisabled > 0 ? [t("bulk.confirm.disable.already", { count: counts.alreadyDisabled })] : []),
  ];
}

/**
 * The Enable confirmation's sentences (BACKLOG #73): what is redeployed —
 * globally, in projects, both, or nothing (the skills only switch back on) —
 * "Reading project deployments…" while the placements are unread, and how
 * many change nothing.
 */
export function enableConfirmLines(counts: EnableCounts, t: Translate): string[] {
  const { skills, targets, tools, assignments } = counts;
  const global = { targets: phrase(t, "targets", targets), tools: phrase(t, "tools", tools) };
  const inProjects = (assignments ?? 0) > 0;
  const redeploys =
    targets > 0 && inProjects
      ? t("bulk.confirm.enable.redeploysBoth", { ...global, assignments: phrase(t, "assignments", assignments ?? 0) })
      : targets > 0
        ? t("bulk.confirm.enable.redeploysGlobal", global)
        : inProjects
          ? t("bulk.confirm.enable.redeploysProjects", { assignments: phrase(t, "assignments", assignments ?? 0) })
          : assignments === null
            ? null
            : t("bulk.confirm.enable.nothingToRedeploy", { count: skills });
  return [
    ...(redeploys ? [redeploys] : []),
    ...(assignments === null ? [t("bulk.confirm.readingProjects")] : []),
    // "Nothing changes" is only true once project placements are read.
    ...(assignments !== null && counts.alreadyEnabled > 0
      ? [t("bulk.confirm.enable.already", { count: counts.alreadyEnabled })]
      : []),
  ];
}

export type SyncCounts = {
  skills: number;
  tools: number;
  /** skills × Tools. */
  pairs: number;
  /** Pairs already synced (a re-sync; identical content is replaced silently). */
  alreadySynced: number;
  /** Pairs a Disable gates (pair or skill grain): skipped, never failed. */
  disabled: number;
  /** Pairs that get a new Sync target. */
  added: number;
};

/** Sync to the picked Tools, over the selection. */
export function syncCounts(skills: readonly ManagedSkill[], tools: readonly string[]): SyncCounts {
  let alreadySynced = 0;
  let disabled = 0;
  for (const skill of skills) {
    for (const tool of tools) {
      const target = skill.targets.find((row) => row.tool === tool);
      if (skill.disabled_at !== null || target?.status === "disabled") disabled++;
      else if (target && (target.status === "synced" || target.status === "stale")) alreadySynced++;
    }
  }
  const pairs = skills.length * tools.length;
  return { skills: skills.length, tools: tools.length, pairs, alreadySynced, disabled, added: pairs - alreadySynced - disabled };
}

/**
 * The skills a harness just turned on would receive (D48's "Deploy your N
 * skills to {harness} now?"): enabled (no skill-grain Disable), locatable
 * (a central copy to deploy) and without a deployment there already — a pair
 * Disable gates is skipped, a deployed or out-of-date one is not new. The ask
 * states this count and the batch deploys exactly these skills.
 */
export function harnessArrivals(skills: readonly ManagedSkill[], tool: string): ManagedSkill[] {
  return skills.filter((skill) => {
    if (skill.disabled_at !== null || skill.unlocatable !== null) return false;
    const target = skill.targets.find((row) => row.tool === tool);
    return !target || !(target.status === "disabled" || target.status === "synced" || target.status === "stale");
  });
}

export type UpdateCounts = {
  skills: number;
  /** Skills Refresh re-acquires from their source. */
  refreshable: number;
  /** Skills with no source to re-acquire from (imported, detached): skipped. */
  skipped: number;
};

/** Update over the selection. */
export function updateCounts(skills: readonly ManagedSkill[]): UpdateCounts {
  const refreshable = skills.filter((skill) => skill.refreshable).length;
  return { skills: skills.length, refreshable, skipped: skills.length - refreshable };
}

/** What deleting one skill takes with it (D16). */
export type DeleteScope = {
  skillId: string;
  name: string;
  /** Tools the skill is deployed to (a materialised target, not disabled), registry order kept. */
  tools: string[];
  /** Projects that carry an assignment of it; null while unread. */
  projects: string[] | null;
  /** Edits lost with it (an invocation override is one Edit). */
  edits: number;
  /** No external source: Skills Hub's copy is the only copy (imported, detached). */
  onlyCopy: boolean;
};

/** One skill's Delete scope; Tool keys go through `labelOf`. */
export function deleteScope(
  skill: ManagedSkill,
  assignmentsOf: AssignmentsOf | null,
  labelOf: (tool: string) => string,
): DeleteScope {
  const rows = assignmentsOf ? assignmentsOf(skill.id) : null;
  return {
    skillId: skill.id,
    name: skill.name,
    tools: skill.targets
      .filter((target) => target.status !== "disabled" && ownsArtifact(target))
      .map((target) => labelOf(target.tool)),
    projects: rows === null ? null : rows.filter((row) => row.assignments.length > 0).map((row) => row.projectName),
    edits: skill.invocation_override ? 1 : 0,
    onlyCopy: skill.source_ref === null,
  };
}

/** Several skills' Delete scope, summed for the bulk confirmation. */
export type DeleteTotals = {
  skills: number;
  /** Distinct Tools that lose at least one skill. */
  tools: number;
  /** Deployed targets removed. */
  targets: number;
  /** Distinct projects that lose at least one assignment; null while unread. */
  projects: number | null;
  edits: number;
  /** Skills whose only copy is Skills Hub's. */
  onlyCopy: number;
};

export function deleteTotals(scopes: readonly DeleteScope[]): DeleteTotals {
  const tools = new Set(scopes.flatMap((scope) => scope.tools));
  const unread = scopes.some((scope) => scope.projects === null);
  return {
    skills: scopes.length,
    tools: tools.size,
    targets: scopes.reduce((sum, scope) => sum + scope.tools.length, 0),
    projects: unread ? null : new Set(scopes.flatMap((scope) => scope.projects ?? [])).size,
    edits: scopes.reduce((sum, scope) => sum + scope.edits, 0),
    onlyCopy: scopes.filter((scope) => scope.onlyCopy).length,
  };
}
