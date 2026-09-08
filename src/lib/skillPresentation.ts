/**
 * Skill presentation: the one home for how a Managed skill's source is
 * identified and displayed (git / local / imported, repo label/href, repo grouping,
 * search/sort, relative time) and for the Onboarding import's default
 * variant. Pure — no React, no i18n instance: callers pass the translate
 * function in.
 */

import type { UnlocatableState } from "../components/skills/types";

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

export type SortMode = "name" | "updated" | "added";

export type RepoGroup<T> = {
  /** Stable identity for React keys; `LOCAL_GROUP_KEY` for the local group. */
  key: string;
  label: string;
  href: string | null;
  skills: T[];
};

export const LOCAL_GROUP_KEY = "__local__";

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

/** The input door for Re-point: a repository URL or a local folder. */
export function repointDoor(
  skill: Pick<SkillPresentationFields, "source_type">,
): "git" | "local" {
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

/** The label a skill's source is shown under when it has no repo. */
export function skillSourceLabel(
  skill: Pick<
    SkillPresentationFields,
    "source_type" | "source_ref" | "central_path"
  >,
): string {
  if (sourceKind(skill) === "git" && skill.source_ref) return skill.source_ref;
  return skill.central_path;
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
 * Fold skills into repo groups, alphabetically by label, with the local
 * group always last.
 */
export function groupSkillsByRepo<T extends Pick<SkillPresentationFields, "source_ref">>(
  skills: T[],
  labels: { local: string; ungrouped: string },
): RepoGroup<T>[] {
  const map = new Map<string, { href: string | null; skills: T[] }>();
  for (const skill of skills) {
    const ref = skill.source_ref ?? "";
    const isGit = ref !== "" && looksLikeGitRef(ref);
    const repo = isGit ? repoInfo(ref) : null;
    const key = isGit ? (repo?.label ?? ref) : LOCAL_GROUP_KEY;
    const entry = map.get(key);
    if (entry) entry.skills.push(skill);
    else map.set(key, { href: repo?.href ?? null, skills: [skill] });
  }
  const entries: RepoGroup<T>[] = [...map.entries()].map(([key, entry]) => ({
    key,
    label: key === LOCAL_GROUP_KEY ? labels.local : key || labels.ungrouped,
    href: entry.href,
    skills: entry.skills,
  }));
  entries.sort((a, b) => {
    if (a.key === LOCAL_GROUP_KEY && b.key !== LOCAL_GROUP_KEY) return 1;
    if (b.key === LOCAL_GROUP_KEY && a.key !== LOCAL_GROUP_KEY) return -1;
    return a.label.toLowerCase().localeCompare(b.label.toLowerCase());
  });
  return entries;
}

/**
 * The My Skills search/sort fold. A query containing `*` is a wildcard
 * pattern (every other regex metacharacter is escaped); otherwise it is a
 * case-insensitive substring.
 */
export function filterAndSortSkills<
  T extends SkillPresentationFields & { name: string },
>(skills: T[], options: { query: string; sort: SortMode }): T[] {
  const query = options.query.trim().toLowerCase();
  const wildcardPattern = query.includes("*")
    ? new RegExp(
        query
          .split("*")
          .map((part) => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
          .join(".*"),
      )
    : null;
  const matchesQuery = (value: string) =>
    wildcardPattern ? wildcardPattern.test(value) : value.includes(query);
  const filtered = skills.filter((skill) => {
    if (!query) return true;
    return (
      matchesQuery(skill.name.toLowerCase()) ||
      matchesQuery(skill.source_ref?.toLowerCase() ?? "") ||
      matchesQuery(skill.central_path.toLowerCase()) ||
      matchesQuery(skill.source_type.toLowerCase())
    );
  });
  return [...filtered].sort((a, b) => {
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
