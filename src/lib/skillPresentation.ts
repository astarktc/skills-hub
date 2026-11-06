/**
 * Skill presentation: the one home for how a Managed skill's source is
 * identified and displayed (git / local / imported, repo label/href, repo grouping,
 * search/sort, relative time) and for the Onboarding import's default
 * variant. Pure — no React, no i18n instance: callers pass the translate
 * function in.
 */

import type { CommandError } from "../bindings";
import { IDENTITY_HUES, type IdentityHue } from "../components/ui/styles";
import type {
  InvocationMode,
  RepointTarget,
  SkillHealth,
  SkillOutcomeKind,
  SkillProjectDto,
  SkillTargetDto,
  ToolOption,
  UnlocatableState,
  UpdateSkip,
} from "../components/skills/types";

export const INVOCATION_MODES = ["user-and-model", "user-only", "model-only", "neither"] as const;
export const INVOCATION_LABEL_KEY: Record<InvocationMode, string> = {
  "user-and-model": "invocationMode.userAndModel",
  "user-only": "invocationMode.userOnly",
  "model-only": "invocationMode.modelOnly",
  neither: "invocationMode.neither",
};
export const INVOCATION_TOOLTIP_KEY: Record<InvocationMode, string> = {
  "user-and-model": "invocationMode.userAndModelTooltip",
  "user-only": "invocationMode.userOnlyTooltip",
  "model-only": "invocationMode.modelOnlyTooltip",
  neither: "invocationMode.neitherTooltip",
};

/** The Managed-skill fields presentation needs. Keeps the module testable. */
export type SkillPresentationFields = {
  source_type: string;
  source_ref: string | null;
  central_path: string;
  created_at: number | null;
  updated_at: number | null;
};

export type RepoInfo = {
  label: string;
  href: string;
};

export type Translate = (
  key: string,
  opts?: Record<string, unknown>,
) => string;

/**
 * The list's orders. `issues` is Problems first (BACKLOG #72): failed skills,
 * then stale, then the rest, each by name — a grouped list orders its groups
 * by their loudest skill too (`problemRank`).
 */
export type SortMode = "name" | "updated" | "added" | "issues";

export type RepoGroup<T> = {
  /**
   * Stable identity for React keys; `LOCAL_GROUP_KEY` for the local folders'
   * group, `MANAGED_GROUP_KEY` for the imported skills' ("Managed here").
   */
  key: string;
  label: string;
  href: string | null;
  /**
   * A GitHub repo's owner (the identity chip's key: `identityHue` /
   * `identityInitials`); null for any other source, the local group and
   * Individual sources.
   */
  owner: string | null;
  skills: T[];
};

export const LOCAL_GROUP_KEY = "__local__";

/**
 * The group of imported skills (BACKLOG #72): they have no source — the
 * central copy is the truth — so they group under the words their source
 * line leads with, "Managed here", never under a folder source's "Local".
 */
export const MANAGED_GROUP_KEY = "__managed__";

/**
 * The order a list's rows are on screen: a grouped list walks its groups
 * (each group's skills in sort order), a flat list is the sorted list. One
 * sequence for rendering, selection navigation and focus — a grouped list's
 * neighbours are not its flat sort's.
 */
export function displayOrder<T>(visible: readonly T[], groups: readonly RepoGroup<T>[] | null): T[] {
  return groups ? groups.flatMap((group) => group.skills) : [...visible];
}

/**
 * The rows on screen (D42): the display order minus every collapsed group's
 * rows — what keyboard order (j/k, ⌘A, a shift-click range), the post-mutation
 * focus rule and the name column read. A collapsed row stays in the lens:
 * still selected, still counted. Flat (no groups) or nothing collapsed = the
 * display order itself.
 */
export function onScreenOrder<T>(
  displayed: readonly T[],
  groups: readonly RepoGroup<T>[] | null,
  collapsed: ReadonlySet<string>,
): T[] {
  return groups && collapsed.size > 0
    ? groups.filter((group) => !collapsed.has(group.key)).flatMap((group) => group.skills)
    : [...displayed];
}

/** The folded group of sources that hold one skill each (`foldSingletons`). */
export const INDIVIDUAL_GROUP_KEY = "__individual__";

/**
 * The skill's Provenance as the UI needs it: `git` (an external repo),
 * `imported` (taken over from a Tool's skills dir — the central copy is the
 * truth, there is no source) or `local` (an independent folder). Every
 * icon / label / source-line decision derives from this one rule.
 */
export type SourceKind = "git" | "local" | "imported";

export function sourceKind(
  skill: Pick<SkillPresentationFields, "source_type">,
): SourceKind {
  const type = skill.source_type.toLowerCase();
  if (type === "imported") return "imported";
  return type.includes("git") ? "git" : "local";
}

/** The source kind a Change source target can take (`→ imported` is Detach). */
export type RepointKind = RepointTarget["kind"];

/**
 * The Change source modal's preselected kind: the skill's current source
 * kind; an imported skill has no source, so it starts on the local folder.
 */
export function repointKind(
  skill: Pick<SkillPresentationFields, "source_type">,
): RepointKind {
  return sourceKind(skill) === "git" ? "git" : "local";
}

/**
 * The label a Tool is shown under, from its registry key: the `tools.*`
 * catalog entry, or the key itself for a Tool the catalog does not name.
 * The one Tool-key→label rule (cards, detail, error copy).
 */
export function toolLabel(t: Translate, key: string): string {
  return t(`tools.${key}`, { defaultValue: key });
}

/** The pieces of an imported skill's source line. */
export type ImportedSourceLine = {
  /** The found-in Tool's label, or the "unknown" label. */
  tool: string;
  /** "Managed here" — the central copy is the truth. */
  managedHere: string;
  /** "Imported from {tool}" — display-only history. */
  importedFrom: string;
  /** Both, as one line. */
  text: string;
};

/**
 * An imported skill's source line: "Managed here · Imported from {Tool}".
 * The found-in Tool is display-only history — shown, never treated as a
 * source — and the record may have kept none.
 */
export function importedSourceLine(
  skill: { imported_from_tool: string | null },
  t: Translate,
): ImportedSourceLine {
  const tool = skill.imported_from_tool
    ? toolLabel(t, skill.imported_from_tool)
    : t("unknown");
  const managedHere = t("provenance.managedHere");
  const importedFrom = t("provenance.importedFrom", { tool });
  return { tool, managedHere, importedFrom, text: `${managedHere} · ${importedFrom}` };
}

/**
 * The label a skill's source is shown under when it has no repo: a git
 * skill's URL, a local skill's recorded folder — the one fact a "source
 * folder missing" repair needs — and the central copy only when no source is
 * recorded (an imported skill shows `importedSourceLine` instead).
 */
export function skillSourceLabel(
  skill: Pick<
    SkillPresentationFields,
    "source_type" | "source_ref" | "central_path"
  >,
): string {
  const ref = skill.source_ref?.trim();
  if (sourceKind(skill) !== "imported" && ref) return ref;
  return skill.central_path;
}

/** The source a Change source dialog starts from (`repointCurrentSource`). */
export type RepointCurrentSource = { kind: RepointKind; ref: string };

/**
 * The skill's current source as a Re-point target of its own kind — the
 * value the Change source dialog pre-fills and shows as "was: …". Null for an
 * imported skill (it has no source) and for a record without a source ref.
 */
export function repointCurrentSource(
  skill: Pick<SkillPresentationFields, "source_type" | "source_ref">,
): RepointCurrentSource | null {
  const kind = sourceKind(skill);
  const ref = skill.source_ref?.trim();
  if (kind === "imported" || !ref) return null;
  return { kind, ref };
}

/**
 * The GitHub repo a source ref points at, as `owner/repo` plus a canonical
 * href. Handles `git+` prefixes, `.git` suffixes, deep paths and scp-like
 * ssh refs; returns null for non-GitHub hosts and local paths.
 */
export function repoInfo(url: string | null | undefined): RepoInfo | null {
  if (!url) return null;
  const normalized = url.replace(/^git\+/, "");
  const build = (owner: string, repo: string): RepoInfo => ({
    label: `${owner}/${repo}`,
    href: `https://github.com/${owner}/${repo}`,
  });
  try {
    const parsed = new URL(normalized);
    if (!parsed.hostname.includes("github.com")) return null;
    const parts = parsed.pathname.split("/").filter(Boolean);
    const owner = parts[0];
    const repo = parts[1]?.replace(/\.git$/, "");
    if (!owner || !repo) return null;
    return build(owner, repo);
  } catch {
    const match = normalized.match(/github\.com[:/]([^/]+)\/([^/#?]+)/i);
    if (!match) return null;
    return build(match[1], match[2].replace(/\.git$/, ""));
  }
}

/**
 * FNV-1a 32-bit over UTF-16 code units: offset basis 0x811c9dc5, prime
 * 0x01000193. Exported for its test; the identity fold is the only reader.
 */
export function fnv1a32(value: string): number {
  let hash = 0x811c9dc5;
  for (let i = 0; i < value.length; i++) {
    hash ^= value.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash;
}

/**
 * A source's identity hue (ticket 30): derived from a stable key — the
 * repo owner, case-insensitively — and never from state. Seven hues, so
 * collisions are expected; the chip supplements the name, never replaces it.
 * A source without an owner (local folder, Individual sources) has no hue.
 */
export function identityHue(owner: string): IdentityHue {
  return IDENTITY_HUES[fnv1a32(owner.toLowerCase()) % IDENTITY_HUES.length];
}

/**
 * The chip's two initials: the first letters of the owner's first two
 * `-` / `_` / `.` parts, else its first two letters, upper-cased
 * (`vercel-labs` → `VL`, `anthropics` → `AN`).
 */
export function identityInitials(owner: string): string {
  const parts = owner.split(/[-_.]+/).filter(Boolean);
  const initials = parts.length >= 2 ? parts[0][0] + parts[1][0] : (parts[0] ?? owner).slice(0, 2);
  return initials.toUpperCase();
}

/** Whether a source ref looks like a git remote rather than a local path. */
function looksLikeGitRef(ref: string): boolean {
  return (
    ref.startsWith("git+") ||
    ref.startsWith("https://") ||
    ref.startsWith("http://") ||
    ref.includes("github.com")
  );
}

/**
 * The fields the source grouping reads. A row without a `source_type` (an
 * Explore candidate) is never imported.
 */
export type GroupingFields = Pick<SkillPresentationFields, "source_ref"> & { source_type?: string };

/** A skill's group key and its GitHub repo, if any. */
function repoGroupOf(skill: GroupingFields): { key: string; repo: RepoInfo | null } {
  const { source_type } = skill;
  if (source_type !== undefined && sourceKind({ source_type }) === "imported") return { key: MANAGED_GROUP_KEY, repo: null };
  const ref = skill.source_ref ?? "";
  const isGit = ref !== "" && looksLikeGitRef(ref);
  const repo = isGit ? repoInfo(ref) : null;
  return { key: isGit ? (repo?.label ?? ref) : LOCAL_GROUP_KEY, repo };
}

/**
 * Fold skills into repo groups, alphabetically by label, then the local
 * folders' group, then the imported skills' "Managed here" group
 * (`labels.managed`). With `foldSingletons`, every repo that holds exactly
 * one skill folds into one "Individual sources" group (`labels.individual`),
 * placed after the repo groups and before the local group, its skills in
 * input order (the lens's sort); Local and Managed here never fold.
 *
 * `catalog` is the unfiltered library (D42, BACKLOG #81 (4)): when given, a
 * repo is a singleton only if it holds one skill **in the catalog**, so a
 * query or chip that leaves one skill of a multi-skill repo keeps it under
 * its repo header. Without it the rows given decide.
 */
export function groupSkillsByRepo<T extends GroupingFields>(
  skills: T[],
  labels: { local: string; managed: string; ungrouped: string; individual?: string },
  options: {
    foldSingletons?: boolean;
    catalog?: readonly GroupingFields[];
    /**
     * Problems first: groups are ordered by their loudest skill's rank
     * (higher first) before the order above.
     */
    problemRank?: (skill: T) => number;
  } = {},
): RepoGroup<T>[] {
  const map = new Map<string, { href: string | null; owner: string | null; skills: T[] }>();
  for (const skill of skills) {
    const { key, repo } = repoGroupOf(skill);
    const entry = map.get(key);
    if (entry) entry.skills.push(skill);
    else map.set(key, { href: repo?.href ?? null, owner: repo ? repo.label.split("/")[0] : null, skills: [skill] });
  }
  const catalogCounts = new Map<string, number>();
  for (const skill of options.catalog ?? skills) {
    const { key } = repoGroupOf(skill);
    catalogCounts.set(key, (catalogCounts.get(key) ?? 0) + 1);
  }
  const isSingleton = (key: string) =>
    Boolean(options.foldSingletons) &&
    key !== LOCAL_GROUP_KEY &&
    key !== MANAGED_GROUP_KEY &&
    (catalogCounts.get(key) ?? 0) <= 1;
  const entries: RepoGroup<T>[] = [...map.entries()]
    .filter(([key]) => !isSingleton(key))
    .map(([key, entry]) => ({
      key,
      label:
        key === LOCAL_GROUP_KEY ? labels.local : key === MANAGED_GROUP_KEY ? labels.managed : key || labels.ungrouped,
      href: entry.href,
      owner: entry.owner,
      skills: entry.skills,
    }));
  const rank = (key: string) =>
    key === MANAGED_GROUP_KEY ? 3 : key === LOCAL_GROUP_KEY ? 2 : key === INDIVIDUAL_GROUP_KEY ? 1 : 0;
  const singles = [...map.entries()].filter(([key]) => isSingleton(key));
  if (singles.length > 0) {
    const members = new Set(singles.flatMap(([, entry]) => entry.skills));
    entries.push({
      key: INDIVIDUAL_GROUP_KEY,
      label: labels.individual ?? labels.ungrouped,
      href: null,
      owner: null,
      skills: skills.filter((skill) => members.has(skill)),
    });
  }
  const { problemRank } = options;
  const loudest = new Map(
    entries.map((group) => [
      group.key,
      problemRank ? group.skills.reduce((max, skill) => Math.max(max, problemRank(skill)), 0) : 0,
    ]),
  );
  entries.sort((a, b) => {
    const byProblem = (loudest.get(b.key) ?? 0) - (loudest.get(a.key) ?? 0);
    if (byProblem !== 0) return byProblem;
    const byRank = rank(a.key) - rank(b.key);
    if (byRank !== 0) return byRank;
    return a.label.toLowerCase().localeCompare(b.label.toLowerCase());
  });
  return entries;
}

/**
 * The query rule of the list lens: a query containing `*` is a wildcard
 * pattern (every other regex metacharacter is escaped); otherwise it is a
 * case-insensitive substring. An empty query matches everything. The My
 * Skills fold below and the ⌘K palette's commands read it, so a query means
 * the same thing wherever it is typed.
 */
export function queryMatcher(rawQuery: string): (value: string) => boolean {
  const query = rawQuery.trim().toLowerCase();
  if (!query) return () => true;
  const wildcardPattern = query.includes("*")
    ? new RegExp(
        query
          .split("*")
          .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
          .join(".*"),
      )
    : null;
  return (value: string) => {
    const lower = value.toLowerCase();
    return wildcardPattern ? wildcardPattern.test(lower) : lower.includes(query);
  };
}

/**
 * The My Skills search/sort fold: `queryMatcher` over a skill's name, source,
 * central path and source type, then the sort. Problems first reads
 * `problemRank` (higher first, then name); without one it sorts by name.
 */
export function filterAndSortSkills<
  T extends SkillPresentationFields & { name: string },
>(skills: T[], options: { query: string; sort: SortMode; problemRank?: (skill: T) => number }): T[] {
  const query = options.query.trim();
  const matchesQuery = queryMatcher(query);
  const filtered = skills.filter((skill) => {
    if (!query) return true;
    return (
      matchesQuery(skill.name.toLowerCase()) ||
      matchesQuery(skill.source_ref?.toLowerCase() ?? "") ||
      matchesQuery(skill.central_path.toLowerCase()) ||
      matchesQuery(skill.source_type.toLowerCase())
    );
  });
  const { problemRank } = options;
  // Ranked once per row, not per comparison (the rank runs the health fold).
  const ranks =
    options.sort === "issues" && problemRank ? new Map(filtered.map((skill) => [skill, problemRank(skill)])) : null;
  return [...filtered].sort((a, b) => {
    if (options.sort === "issues") {
      const byProblem = ranks ? (ranks.get(b) ?? 0) - (ranks.get(a) ?? 0) : 0;
      return byProblem !== 0 ? byProblem : a.name.localeCompare(b.name);
    }
    if (options.sort === "name") return a.name.localeCompare(b.name);
    if (options.sort === "added")
      return (b.created_at ?? 0) - (a.created_at ?? 0);
    return (b.updated_at ?? 0) - (a.updated_at ?? 0);
  });
}

/** The onboarding-variant fields the default-variant rule needs. */
export type ImportVariantFields = {
  path: string;
  is_link: boolean;
};

/**
 * The variant an Onboarding import chooses by default for a group: in a
 * consistent (no-conflict) group, the first real directory — a link is never
 * what the app copies when the directory it points at is also in the group;
 * ties, and a group with no real directory, fall back to the plan's order
 * (the Tool registry's). A conflicting group stays at its first variant: the
 * operator resolves it by hand. `undefined` for an empty group.
 */
export function defaultImportVariantPath(group: {
  has_conflict: boolean;
  variants: ImportVariantFields[];
}): string | undefined {
  const preferred = group.has_conflict
    ? undefined
    : group.variants.find((variant) => !variant.is_link);
  return (preferred ?? group.variants[0])?.path;
}

/** The Unlocatable-skill fields the repair rule reads (all backend-owned). */
export type UnlocatableRepairFields = {
  unlocatable: UnlocatableState | null;
  refreshable: boolean;
  detachable: boolean;
};

/** One repair the card offers an Unlocatable skill; Remove is always there. */
export type UnlocatableRepair = "repoint" | "detach" | "restore";

/** Wire states and repairs meet the i18n catalog only in these maps. */
export const UNLOCATABLE_STATE_KEY = {
  source_missing: "unlocatable.sourceMissing",
  central_missing: "unlocatable.centralMissing",
} satisfies Record<UnlocatableState, string>;

/** The panel row's message for an Update whose acquired result was not admitted. */
export const ACQUISITION_SKIP_KEY = {
  skill_gone: "errors.refreshSkippedSkillGone",
  stale_acquisition: "errors.refreshSkippedStaleAcquisition",
} satisfies Record<UpdateSkip, string>;

export const UNLOCATABLE_TOOLTIP_KEY = {
  source_missing: "unlocatable.sourceMissingTooltip",
  central_missing: "unlocatable.centralMissingTooltip",
} satisfies Record<UnlocatableState, string>;

export const UNLOCATABLE_REPAIR_KEY = {
  repoint: "unlocatable.repoint",
  detach: "unlocatable.detach",
  restore: "unlocatable.restore",
} satisfies Record<UnlocatableRepair, string>;

/** The panel row's message for a skill Refresh (all) skipped, by state. */
export const SKIPPED_REASON_KEY = {
  source_missing: "errors.refreshSkippedSourceMissing",
  central_missing: "errors.refreshSkippedCentralMissing",
} satisfies Record<UnlocatableState, string>;

/**
 * Which repairs an Unlocatable skill is offered, in display order. The
 * state and both affordance bits are the backend's answers (computed at
 * list time); this only pairs them: a missing source offers Re-point, and
 * Detach when the central copy is there to become the truth; a missing
 * central copy offers Restore (an Update) when the skill is refreshable.
 */
export function unlocatableRepairs(
  skill: UnlocatableRepairFields,
): UnlocatableRepair[] {
  if (skill.unlocatable === "source_missing") {
    return skill.detachable ? ["repoint", "detach"] : ["repoint"];
  }
  if (skill.unlocatable === "central_missing" && skill.refreshable) {
    return ["restore"];
  }
  return [];
}

/**
 * One tool chip on a skill card: a Tool the card must offer the operator,
 * with the target row it has (if any) and whether detection still sees the
 * Tool.
 */
export type SkillToolChip = {
  id: string;
  label: string;
  /** The skill's Sync target row for this Tool, or null when not synced. */
  target: SkillTargetDto | null;
  /** False when detection no longer sees the Tool (an orphaned row). */
  detected: boolean;
};

/**
 * Which tool chips a skill card renders: the union of the target rows the
 * skill actually has and the installed tools (the "not synced here yet"
 * affordance). Detection filters what is *offered*, never what the skill
 * already has — a row for an undetected Tool would otherwise be invisible
 * and unreachable. Installed tools keep their registry order; undetected
 * target-bearing tools follow. A row naming a Tool absent from `allTools`
 * still renders, labelled by its key.
 */
export function skillToolChips(
  targets: SkillTargetDto[],
  installedTools: ToolOption[],
  allTools: ToolOption[],
): SkillToolChip[] {
  const targetByTool = new Map(targets.map((target) => [target.tool, target]));
  const chips: SkillToolChip[] = installedTools.map((tool) => ({
    id: tool.id,
    label: tool.label,
    target: targetByTool.get(tool.id) ?? null,
    detected: true,
  }));
  const seen = new Set(chips.map((chip) => chip.id));
  const labelById = new Map(allTools.map((tool) => [tool.id, tool.label]));
  for (const target of targets) {
    if (seen.has(target.tool)) continue;
    seen.add(target.tool);
    chips.push({
      id: target.tool,
      label: labelById.get(target.tool) ?? target.tool,
      target,
      detected: false,
    });
  }
  return chips;
}

/**
 * Which tools a Tool-selection list renders: every tool when the operator is
 * not filtering, otherwise the detected ones plus any tool the saved
 * selection already carries. A selected-but-undetected tool stays visible so
 * it can be unticked instead of being silently re-persisted. A selected key
 * the registry no longer knows never reaches here: the backend prunes it
 * when it reads the selection (round 12 D2).
 */
export function visibleToolChoices<T extends { key: string }>(
  allTools: T[],
  installed: string[],
  selected: ReadonlySet<string>,
  detectedOnly: boolean,
): T[] {
  if (!detectedOnly) return allTools;
  return allTools.filter(
    (tool) => installed.includes(tool.key) || selected.has(tool.key),
  );
}

/**
 * Relative time using the `relative.*` i18n family only. `now` defaults to
 * the current clock; tests (and any caller needing a fixed clock) pass it.
 */
export function formatRelativeTime(
  ms: number | null | undefined,
  t: Translate,
  now: number = Date.now(),
): string {
  if (!ms) return t("relative.empty");
  const diff = now - ms;
  if (diff < 0) return t("relative.empty");
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return t("relative.justNow");
  if (minutes < 60) return t("relative.minutesAgo", { minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t("relative.hoursAgo", { hours });
  const days = Math.floor(hours / 24);
  return t("relative.daysAgo", { days });
}

/**
 * Whether a target row is live: materialised by the app and not switched
 * off. A `disabled` row is the operator's "not on this Tool"; an `error` row
 * that was never synced (`synced_at` null — a failed first sync, D9) owns
 * nothing on disk, so the skill is not deployed there.
 */
export function isLiveTarget(target: Pick<SkillTargetDto, "status" | "synced_at">): boolean {
  if (target.status === "disabled") return false;
  return !(target.status === "error" && target.synced_at === null);
}

/**
 * The card's deploy action for a skill with no live target. Disabled at
 * either grain it is an Enable (`enable` — one skill-grain batch that
 * re-syncs every disabled row, never a sync the Disabled gate would skip);
 * `disabledTargets` names how many pairs that turns back on. Otherwise a sync
 * to the effective target set (`sync`) — which is also the retry of a skill
 * whose only targets are failed first syncs. `deployed` means some target is
 * live (`isLiveTarget`).
 */
export type DeployAction =
  | { kind: "deployed" }
  | { kind: "enable"; disabledTargets: number }
  | { kind: "sync" };
export function deployAction(skill: {
  disabled_at: number | null;
  targets: readonly Pick<SkillTargetDto, "status" | "synced_at">[];
}): DeployAction {
  const disabledTargets = skill.targets.filter((target) => target.status === "disabled").length;
  if (skill.disabled_at !== null) return { kind: "enable", disabledTargets };
  if (skill.targets.some(isLiveTarget)) return { kind: "deployed" };
  return disabledTargets > 0 ? { kind: "enable", disabledTargets } : { kind: "sync" };
}

// ---------------------------------------------------------------------------
// Skill health and the source line: one fold (D7 candidate 3, D9)
// ---------------------------------------------------------------------------

/** How loud a health row is: failed › stale › muted; a skill with no row is healthy. */
export type HealthSeverity = "failed" | "stale" | "muted" | "healthy";

/**
 * One reason a skill is not simply healthy. Skill-level kinds are the
 * Unlocatable states (computed at list time), the stored outcome kinds
 * (`ManagedSkill.health`) and skill-grain Disable; target-level kinds count
 * Sync target rows by status.
 */
export type HealthRowKind =
  | UnlocatableState
  | SkillOutcomeKind
  | "disabled"
  | "targets_failed"
  | "targets_missing"
  | "targets_stale"
  | "targets_disabled"
  | "not_deployed";

/**
 * A repair a health row offers; each is a verb of the skill action door
 * (`dismiss` = the `import_failed` Dismiss, `retryTargets` = a re-sync from
 * the central copy of the row's pairs — failed, missing or stale, never an
 * upstream fetch — `deploy` = a sync to the effective targets, `disable` =
 * finish a skill-grain Disable whose targets did not all settle).
 */
export type HealthRepair =
  | "restore"
  | "repoint"
  | "detach"
  | "update"
  | "editInvocation"
  | "delete"
  | "dismiss"
  | "retryTargets"
  | "enable"
  | "deploy"
  | "disable";

export type HealthRow = {
  kind: HealthRowKind;
  scope: "skill" | "target";
  severity: HealthSeverity;
  /**
   * Target rows: how many (global pairs + project deployments), and the
   * global ones' Tools (registry keys, in row order) — the pairs the row's
   * `retryTargets` repairs.
   */
  count: number;
  tools: string[];
  /**
   * The project deployments a failed / missing target row counts (D41): each
   * project, named, with its Tools. Empty for every other row.
   */
  projects: { id: string; name: string; tools: string[] }[];
  /** A stored outcome's typed error and when it settled; null otherwise. */
  error: CommandError | null;
  at: number | null;
  repairs: HealthRepair[];
};

/**
 * The one precedence rule: rows are listed in this order, which is also
 * severity order (every failed kind before every stale kind before every
 * muted one). The first row is the skill's headline problem.
 */
export const HEALTH_ROW_ORDER = [
  "central_missing",
  "restore_failed",
  "delete_incomplete",
  "targets_failed",
  "targets_missing",
  "source_missing",
  "repoint_failed",
  "fetch_failed",
  "edit_conflict",
  "import_failed",
  "targets_stale",
  "disabled",
  "targets_disabled",
  "not_deployed",
] as const satisfies readonly HealthRowKind[];

const ROW_SEVERITY: Record<HealthRowKind, HealthSeverity> = {
  central_missing: "failed",
  restore_failed: "failed",
  delete_incomplete: "failed",
  targets_failed: "failed",
  targets_missing: "failed",
  source_missing: "stale",
  repoint_failed: "stale",
  fetch_failed: "stale",
  edit_conflict: "stale",
  import_failed: "stale",
  targets_stale: "stale",
  disabled: "muted",
  targets_disabled: "muted",
  not_deployed: "muted",
};

const SEVERITY_RANK: Record<HealthSeverity, number> = {
  failed: 3,
  stale: 2,
  muted: 1,
  healthy: 0,
};

/** The label of each row kind (`health.rows.*`; counted kinds take `count`). */
export const HEALTH_ROW_LABEL_KEY = {
  central_missing: "unlocatable.centralMissing",
  source_missing: "unlocatable.sourceMissing",
  fetch_failed: "skillHealth.fetchFailed",
  edit_conflict: "skillHealth.editConflict",
  delete_incomplete: "skillHealth.deleteIncomplete",
  repoint_failed: "skillHealth.repointFailed",
  restore_failed: "skillHealth.restoreFailed",
  import_failed: "skillHealth.importFailed",
  disabled: "health.rows.disabled",
  targets_failed: "health.rows.targetsFailed",
  targets_missing: "health.rows.targetsMissing",
  targets_stale: "health.rows.targetsStale",
  targets_disabled: "health.rows.targetsDisabled",
  not_deployed: "health.rows.notDeployed",
} as const satisfies Record<HealthRowKind, string>;

/** The label of each repair (`health.repairs.*`, the Unlocatable ones reused). */
export const HEALTH_REPAIR_LABEL_KEY = {
  restore: "unlocatable.restore",
  repoint: "unlocatable.repoint",
  detach: "unlocatable.detach",
  update: "health.repairs.retryUpdate",
  editInvocation: "health.repairs.editInvocation",
  delete: "health.repairs.finishDelete",
  dismiss: "health.repairs.dismiss",
  retryTargets: "health.repairs.retryTargets",
  enable: "health.repairs.enable",
  deploy: "health.repairs.deploy",
  disable: "health.repairs.finishDisable",
} as const satisfies Record<HealthRepair, string>;

/** How the source line is drawn. */
export type SourceIcon = "github" | "git" | "folder" | "imported";

/**
 * A skill's source line: what it is labelled, where it links, what Copy
 * puts on the clipboard (null = nothing to copy — an imported skill has no
 * source) and its icon. For an imported skill `label` is "Managed here"
 * and `detail` the display-only "Imported from {Tool}" history.
 */
export type SkillSourceLine = {
  kind: SourceKind;
  label: string;
  detail: string | null;
  /** Hover text (the imported line's explanation), when there is one. */
  title: string | null;
  href: string | null;
  copyValue: string | null;
  icon: SourceIcon;
};

/**
 * The one source-line rule (card, inspector, Explore preview): a GitHub
 * repo shows `owner/repo`, links and copies the repo; any other git source
 * and a local folder show and copy the recorded source ref (a local skill's
 * `source_ref`, D15) — the central copy only when none is recorded; an
 * imported skill shows "Managed here · Imported from {Tool}" and copies
 * nothing.
 */
export function skillSourceLine(
  skill: Pick<SkillPresentationFields, "source_type" | "source_ref" | "central_path"> & {
    imported_from_tool?: string | null;
  },
  t: Translate,
): SkillSourceLine {
  const kind = sourceKind(skill);
  if (kind === "imported") {
    const imported = importedSourceLine({ imported_from_tool: skill.imported_from_tool ?? null }, t);
    return {
      kind,
      label: imported.managedHere,
      detail: imported.importedFrom,
      title: t("provenance.managedHereTooltip", { tool: imported.tool }),
      href: null,
      copyValue: null,
      icon: "imported",
    };
  }
  const github = kind === "git" ? repoInfo(skill.source_ref) : null;
  if (github) {
    return { kind, label: github.label, detail: null, title: null, href: github.href, copyValue: github.href, icon: "github" };
  }
  const label = skillSourceLabel(skill);
  return {
    kind,
    label,
    detail: null,
    title: null,
    href: null,
    copyValue: label.trim() || null,
    icon: kind === "git" ? "git" : "folder",
  };
}

/** The skill fields the health fold reads (all on the wire). */
export type SkillHealthFields = Pick<
  SkillPresentationFields,
  "source_type" | "source_ref" | "central_path"
> & {
  imported_from_tool: string | null;
  unlocatable: UnlocatableState | null;
  refreshable: boolean;
  detachable: boolean;
  disabled_at: number | null;
  health: SkillHealth | null;
  invocation_override: { conflict: boolean } | null;
  targets: readonly Pick<SkillTargetDto, "tool" | "status" | "synced_at">[];
  /** The skill's projects and its deployments there (D41); absent = none. */
  projects?: readonly SkillProjectFields[];
};

/** The project fields the folds read (`SkillProjectDto`). */
export type SkillProjectFields = Pick<SkillProjectDto, "project_id" | "project_name"> & {
  deployments: readonly Pick<SkillProjectDto["deployments"][number], "tool" | "status" | "synced_at">[];
};

/** What the fold answers: the rows by precedence, the loudest severity, the source line. */
export type SkillHealthFold = {
  rows: HealthRow[];
  severity: HealthSeverity;
  source: SkillSourceLine;
};

const STORED_REPAIRS: Record<SkillOutcomeKind, (skill: SkillHealthFields) => HealthRepair[]> = {
  fetch_failed: (skill) => (skill.refreshable ? ["update", "repoint"] : ["repoint"]),
  edit_conflict: () => ["editInvocation"],
  // Retry = finish the Delete (the kept artifacts go, then the row).
  delete_incomplete: () => ["delete"],
  repoint_failed: () => ["repoint"],
  restore_failed: (skill) => (skill.refreshable ? ["restore", "repoint"] : ["repoint"]),
  // Nothing to retry (the skill is imported; an original stayed): Dismiss.
  import_failed: () => ["dismiss"],
};

/**
 * Skill health: every reason this skill is not simply healthy, one row per
 * kind in `HEALTH_ROW_ORDER`, its loudest severity, and its source line.
 * Card, inspector ladder, issues banner/chip and the strip read this answer;
 * nothing else composes health. `context.t` labels the source line.
 *
 * - Skill level: the Unlocatable state (repairs from `unlocatableRepairs`,
 *   plus Remove), the stored outcome (`ManagedSkill.health`), an Edit
 *   conflict the override reports live (deduplicated with a stored one),
 *   skill-grain Disable.
 * - Target level: `error` rows (failed — a never-synced one is a failed first
 *   sync), `missing`, `stale` (copy drift), pair-grain `disabled` rows (only
 *   when the skill is not disabled as a whole), and "not deployed" when no
 *   target is live and nothing is disabled. A failed / missing / stale row is
 *   repaired from the central copy (`retryTargets` on its Tools) — for every
 *   provenance, since the central bytes are what a target should hold; under
 *   skill-grain Disable the only repair honouring the operator's policy is
 *   finishing the Disable (a re-sync would be skipped by the Disabled gate).
 */
export function skillHealth(
  skill: SkillHealthFields,
  context: { t: Translate },
): SkillHealthFold {
  const rows: HealthRow[] = [];
  const row = (
    kind: HealthRowKind,
    fields: Partial<Omit<HealthRow, "kind" | "scope" | "severity">> = {},
  ): HealthRow => ({
    kind,
    scope: kind.startsWith("targets_") || kind === "not_deployed" ? "target" : "skill",
    severity: ROW_SEVERITY[kind],
    count: 0,
    tools: [],
    projects: [],
    error: null,
    at: null,
    repairs: [],
    ...fields,
  });

  if (skill.unlocatable) {
    rows.push(row(skill.unlocatable, { repairs: [...unlocatableRepairs(skill), "delete"] }));
  }
  const stored = skill.health;
  if (stored) {
    rows.push(row(stored.kind, { error: stored.error, at: stored.at, repairs: STORED_REPAIRS[stored.kind](skill) }));
  }
  if (skill.invocation_override?.conflict && stored?.kind !== "edit_conflict") {
    rows.push(row("edit_conflict", { repairs: ["editInvocation"] }));
  }
  if (skill.disabled_at !== null) rows.push(row("disabled", { at: skill.disabled_at, repairs: ["enable"] }));

  const byStatus = (status: SkillTargetDto["status"]) =>
    skill.targets.filter((target) => target.status === status).map((target) => target.tool);
  const inProjects = (status: SkillTargetDto["status"]) =>
    (skill.projects ?? []).flatMap((project) => {
      const tools = project.deployments.filter((d) => d.status === status).map((d) => d.tool);
      return tools.length > 0 ? [{ id: project.project_id, name: project.project_name, tools }] : [];
    });
  const counted = (kind: HealthRowKind, tools: string[], repairs: HealthRepair[], projects: HealthRow["projects"] = []) => {
    const count = tools.length + projects.reduce((sum, project) => sum + project.tools.length, 0);
    if (count > 0) rows.push(row(kind, { count, tools, projects, repairs }));
  };
  // A project deployment repairs from the project (the matrix's Re-deploy);
  // the row's retry is the global pairs' only, so it is offered only with them.
  const targetRepairs = (tools: string[]): HealthRepair[] =>
    skill.disabled_at !== null ? ["disable"] : tools.length > 0 ? ["retryTargets"] : [];
  const failedTools = byStatus("error");
  const missingTools = byStatus("missing");
  const staleTools = byStatus("stale");
  counted("targets_failed", failedTools, targetRepairs(failedTools), inProjects("error"));
  counted("targets_missing", missingTools, targetRepairs(missingTools), inProjects("missing"));
  counted("targets_stale", staleTools, targetRepairs(staleTools));
  if (skill.disabled_at === null) counted("targets_disabled", byStatus("disabled"), ["enable"]);
  if (skill.disabled_at === null && deployAction(skill).kind === "sync" && !skill.unlocatable) {
    rows.push(row("not_deployed", { repairs: ["deploy"] }));
  }

  const rank = (kind: HealthRowKind) => (HEALTH_ROW_ORDER as readonly HealthRowKind[]).indexOf(kind);
  rows.sort((a, b) => rank(a.kind) - rank(b.kind));
  const severity = rows.reduce<HealthSeverity>(
    (loudest, current) => (SEVERITY_RANK[current.severity] > SEVERITY_RANK[loudest] ? current.severity : loudest),
    "healthy",
  );
  return { rows, severity, source: skillSourceLine(skill, context.t) };
}

/** The one-line label of each stored outcome kind (`skillHealth.*`). */
export const SKILL_OUTCOME_LABEL_KEY = {
  fetch_failed: "skillHealth.fetchFailed",
  edit_conflict: "skillHealth.editConflict",
  delete_incomplete: "skillHealth.deleteIncomplete",
  repoint_failed: "skillHealth.repointFailed",
  restore_failed: "skillHealth.restoreFailed",
  import_failed: "skillHealth.importFailed",
} as const satisfies Record<SkillOutcomeKind, string>;

// ---------------------------------------------------------------------------
// My Skills: the Synced strip, the library's exceptions, the filter chips
// and a source group's state (ticket 11). Every one reads the health fold or
// the target rows; nothing here composes health by hand.
// ---------------------------------------------------------------------------

/** The fold's rows need no labels here; only the source line would. */
const KEY_ONLY: Translate = (key) => key;

/**
 * One mark's state on the Synced strip (D22: read-only). Shape carries it —
 * filled synced, hollow not synced, × failed / missing / broken, half-filled
 * stale, slashed disabled — colour only reinforces.
 */
export type StripMarkState =
  | "synced"
  | "not_synced"
  | "failed"
  | "missing"
  | "stale"
  | "disabled"
  | "broken";

/** The accessible name of each mark (`strip.*`, `{{tool}}` = the Tool label). */
export const STRIP_MARK_NAME_KEY = {
  synced: "strip.synced",
  not_synced: "strip.notSynced",
  pending: "strip.pending",
  failed: "strip.failed",
  failed_first: "strip.failedFirst",
  missing: "strip.missing",
  stale: "strip.stale",
  disabled: "strip.disabled",
  skill_disabled: "strip.skillDisabled",
  broken: "strip.broken",
} as const;

export type StripMark = {
  tool: string;
  /** The Tool catalog's label — never a raw key. */
  label: string;
  state: StripMarkState;
  /** Which sentence names it (`STRIP_MARK_NAME_KEY`). */
  nameKey: keyof typeof STRIP_MARK_NAME_KEY;
  /** False when detection no longer sees the Tool. */
  detected: boolean;
};

export type SyncedStrip = {
  marks: StripMark[];
  /** Marks holding a deployed copy (synced or stale) — the N of N/M. */
  synced: number;
  /** The M of N/M: every Tool the strip shows. */
  total: number;
  /** Every mark synced: the strip goes quiet (healthy is silent). */
  complete: boolean;
};

/**
 * The Synced strip: one mark per Tool of the effective target set (in the
 * order given — the registry's), then any Tool the skill still has a row
 * for outside that set, so a failure is never hidden. A skill disabled at
 * skill grain shows every mark disabled; a skill whose central copy is
 * missing never shows a filled mark (its links dangle: `broken`).
 */
export function syncedStrip(
  skill: {
    disabled_at: number | null;
    unlocatable: UnlocatableState | null;
    targets: readonly Pick<SkillTargetDto, "tool" | "status" | "synced_at">[];
  },
  tools: readonly { id: string; label: string }[],
  context: { labelOf: (tool: string) => string; isDetected: (tool: string) => boolean },
): SyncedStrip {
  const byTool = new Map(skill.targets.map((target) => [target.tool, target]));
  const order = tools.map((tool) => ({ id: tool.id, label: tool.label }));
  const shown = new Set(order.map((tool) => tool.id));
  for (const target of skill.targets) {
    if (shown.has(target.tool)) continue;
    shown.add(target.tool);
    order.push({ id: target.tool, label: context.labelOf(target.tool) });
  }
  const centralMissing = skill.unlocatable === "central_missing";
  const marks = order.map(({ id, label }): StripMark => {
    const mark = (state: StripMarkState, nameKey: StripMark["nameKey"]): StripMark => ({
      tool: id,
      label,
      state,
      nameKey,
      detected: context.isDetected(id),
    });
    if (skill.disabled_at !== null) return mark("disabled", "skill_disabled");
    const target = byTool.get(id);
    if (!target) return mark("not_synced", "not_synced");
    switch (target.status) {
      case "disabled":
        return mark("disabled", "disabled");
      case "error":
        return mark("failed", target.synced_at === null ? "failed_first" : "failed");
      case "missing":
        return mark("missing", "missing");
      case "pending":
        return mark("not_synced", "pending");
      case "stale":
        return centralMissing ? mark("broken", "broken") : mark("stale", "stale");
      case "synced":
        return centralMissing ? mark("broken", "broken") : mark("synced", "synced");
    }
  });
  const synced = marks.filter((mark) => mark.state === "synced" || mark.state === "stale").length;
  return {
    marks,
    synced,
    total: marks.length,
    complete: marks.length > 0 && marks.every((mark) => mark.state === "synced"),
  };
}

/** A mark's accessible name: "Amp: failed — the last sync did not complete". */
export function stripMarkName(mark: StripMark, t: Translate): string {
  const tool = mark.detected ? mark.label : t("strip.notDetected", { name: mark.label });
  return t(STRIP_MARK_NAME_KEY[mark.nameKey], { tool });
}

/** A skill has an issue when its loudest health row is failed or stale. */
export function skillHasIssues(skill: SkillHealthFields): boolean {
  return problemRank(skill) > 0;
}

/**
 * How loud a skill's problem is, for the Problems first sort: 2 failed, 1
 * stale, 0 for anything else (muted — disabled, not deployed — is not an
 * issue, as the Issues chip says).
 */
export function problemRank(skill: SkillHealthFields): number {
  const { severity } = skillHealth(skill, { t: KEY_ONLY });
  return severity === "failed" ? 2 : severity === "stale" ? 1 : 0;
}

/**
 * The header's exception summary: failed and stale Sync targets across the
 * library (the fold's target rows), and how many skills have an issue — the
 * count the Issues chip shows.
 */
export type LibraryExceptions = {
  failedTargets: number;
  staleTargets: number;
  skillsWithIssues: number;
};

export function libraryExceptions(skills: readonly SkillHealthFields[]): LibraryExceptions {
  let failedTargets = 0;
  let staleTargets = 0;
  let skillsWithIssues = 0;
  for (const skill of skills) {
    const { rows, severity } = skillHealth(skill, { t: KEY_ONLY });
    for (const row of rows) {
      if (row.kind === "targets_failed" || row.kind === "targets_missing") failedTargets += row.count;
      if (row.kind === "targets_stale") staleTargets += row.count;
    }
    if (severity === "failed" || severity === "stale") skillsWithIssues += 1;
  }
  return { failedTargets, staleTargets, skillsWithIssues };
}

/** The My Skills filter chips (All · Issues · Edited · Disabled). */
export type SkillFilter = "all" | "issues" | "edited" | "disabled";

/**
 * Each chip's rule. Issues is exactly the header summary's count; Edited is
 * a skill with a direct Edit (an invocation override); Disabled is either
 * grain (the skill, or any of its Tools).
 */
export const SKILL_FILTERS: Readonly<Record<Exclude<SkillFilter, "all">, (skill: SkillHealthFields) => boolean>> = {
  issues: skillHasIssues,
  edited: (skill) => skill.invocation_override !== null,
  disabled: (skill) =>
    skill.disabled_at !== null || skill.targets.some((target) => target.status === "disabled"),
};

/**
 * A source group's state line: how many of its skills are failed, stale,
 * not deployed or disabled (zeros are dropped by the caller), and how many
 * its Deploy all would act on (Enable the disabled, sync the never
 * deployed — the door's `deployGroup` rule).
 */
export type GroupState = {
  failed: number;
  stale: number;
  notDeployed: number;
  disabled: number;
  deployable: number;
};

export function groupState(skills: readonly SkillHealthFields[]): GroupState {
  const state: GroupState = { failed: 0, stale: 0, notDeployed: 0, disabled: 0, deployable: 0 };
  for (const skill of skills) {
    const { rows, severity } = skillHealth(skill, { t: KEY_ONLY });
    if (severity === "failed") state.failed += 1;
    else if (severity === "stale") state.stale += 1;
    if (rows.some((row) => row.kind === "not_deployed")) state.notDeployed += 1;
    if (skill.disabled_at !== null) state.disabled += 1;
    if (deployAction(skill).kind !== "deployed") state.deployable += 1;
  }
  return state;
}

// ---------------------------------------------------------------------------
// My Skills row and group anatomy (ticket 31: D39–D42)
// ---------------------------------------------------------------------------

/**
 * Rows the row never repeats as an exception mark: Disabled at either grain
 * (the strip and the switch say it) and "Not deployed" (D40 — outright false
 * for a skill that lives in projects; the strip's hollow marks say it
 * globally). The kinds stay in the fold for the inspector ladder, the group
 * summary and Deploy all's count.
 */
const SILENT_ROW_MARKS = new Set<HealthRowKind>(["disabled", "targets_disabled", "not_deployed"]);

/** The exception marks a My Skills row shows: the fold's rows minus the silent ones. */
export function rowMarks(fold: Pick<SkillHealthFold, "rows">): HealthRow[] {
  return fold.rows.filter((row) => !SILENT_ROW_MARKS.has(row.kind));
}

/**
 * A row mark's words: the target kinds in the group summary's short form
 * ("2 failed", "1 missing", "3 out of date" — the row's "N failed", D41),
 * every other kind its ladder label. The ladder's full sentence stays the
 * mark's tooltip.
 */
export const ROW_MARK_LABEL_KEY: Readonly<Record<HealthRowKind, string>> = {
  ...HEALTH_ROW_LABEL_KEY,
  targets_failed: "library.row.marks.failed",
  targets_missing: "library.row.marks.missing",
  targets_stale: "library.row.marks.stale",
};

/** Where a skill is live (D41): the row's scope cell. */
export type SkillScope = {
  /** Some global deployment is live (`deployAction` = deployed). */
  global: boolean;
  /** The projects holding at least one live deployment (`isLiveTarget`), in the listing's order. */
  projects: { id: string; name: string }[];
};

export function skillScope(skill: Pick<SkillHealthFields, "disabled_at" | "targets" | "projects">): SkillScope {
  return {
    global: deployAction(skill).kind === "deployed",
    projects: (skill.projects ?? [])
      .filter((project) => project.deployments.some(isLiveTarget))
      .map((project) => ({ id: project.project_id, name: project.project_name })),
  };
}

/**
 * The list's name column (D39): the longest visible name plus its exception
 * marks (each its label + 5 ch for the badge's icon, padding and gap) and the
 * Edit pencil (4 ch), + 2 ch of slack, in `ch`, clamped by CSS to
 * [12rem, 22rem]. The list publishes it as `--name-col`, as it does
 * `--strip-marks`; descriptions take the rest.
 */
export function nameColumnWidth(
  rows: readonly { name: string; marks: readonly string[]; edited: boolean }[],
): string {
  const widest = rows.reduce((max, row) => {
    const marks = row.marks.reduce((sum, label) => sum + label.length + 5, 0);
    return Math.max(max, row.name.length + 2 + marks + (row.edited ? 4 : 0));
  }, 0);
  return `clamp(12rem, ${widest}ch, 22rem)`;
}

/**
 * Two-letter initials per harness, for the group header's row over the strip
 * (D42): from the Harness catalog's label — the first letters of its first
 * two words, or the first two letters of a one-word name — upper-cased.
 * Unique within the set given (the strip's harnesses, in order): a later
 * harness whose pair is taken keeps its first letter and takes the next
 * unused letter of its label; a label with no free letter keeps its pair.
 */
export function harnessInitials(tools: readonly { id: string; label: string }[]): Record<string, string> {
  const out: Record<string, string> = {};
  const taken = new Set<string>();
  for (const { id, label } of tools) {
    const words = label.split(/\s+/).filter(Boolean);
    const letters = label.replace(/[^\p{L}\p{N}]/gu, "").toUpperCase();
    const first = (words[0]?.[0] ?? label[0] ?? "").toUpperCase();
    const preferred = (words.length >= 2 ? first + words[1][0] : letters.slice(0, 2)).toUpperCase();
    let initials = preferred;
    if (taken.has(initials)) {
      const next = [...letters.slice(1)].find((letter) => !taken.has(first + letter));
      if (next) initials = first + next;
    }
    taken.add(initials);
    out[id] = initials;
  }
  return out;
}

/**
 * Where a target row's pairs are, in words — "On Amp, Kimi Code CLI" and
 * "In quartermaster (.agents/skills), api (Claude Code)" — for the inspector
 * ladder and the row mark's tooltip alike. Empty for a skill-level row.
 */
export function healthRowPlaces(
  row: Pick<HealthRow, "tools" | "projects">,
  context: { t: Translate; labelOf: (tool: string) => string },
): string[] {
  const { t, labelOf } = context;
  const parts: string[] = [];
  if (row.tools.length > 0) parts.push(t("inspector.health.onTools", { tools: row.tools.map(labelOf).join(", ") }));
  if (row.projects.length > 0) {
    const projects = row.projects
      .map((project) => `${project.name} (${project.tools.map(labelOf).join(", ")})`)
      .join(", ");
    parts.push(t("inspector.health.inProjects", { projects }));
  }
  return parts;
}
