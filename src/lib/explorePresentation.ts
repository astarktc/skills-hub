// Explore's pure folds (round 19 ticket 22, D25): the candidates the rows
// show — featured skills and the online search's results, each marked
// installed (with the Managed skill it is) or hidden — the three filter
// chips, the query fold the list lens runs, the search mode line's state and
// the popularity figure. Hooks and components import these; nothing here
// touches the DOM or the backend.

import type { FeaturedSkillDto, OnlineSkillDto } from "../bindings";
import { repoInfo } from "./skillPresentation";

/** Where a candidate came from: the curated featured list, or skills.sh's search. */
export type ExploreOrigin = "featured" | "online";

/** A popularity figure: secondary, never the row's lead (D25). */
export type ExplorePopularity = { kind: "downloads" | "stars" | "installs"; count: number };

/** One Explore row. Not a Managed skill: an installed one names the skill it is (`installedId`). */
export type ExploreCandidate = {
  /** Unique per row: origin + `key`. */
  id: string;
  /** The hidden-list key (`exploreHideKey`) — persisted, so its shape is a compat contract. */
  key: string;
  origin: ExploreOrigin;
  name: string;
  description: string | null;
  /** The URL Add and the preview's clone take (a featured one may carry `/tree/<ref>/<path>`). */
  sourceUrl: string;
  /** The same URL, under the name the list lens's fields use. */
  source_ref: string;
  /** `owner/repo`, else the source as the backend gave it. */
  sourceLabel: string;
  popularity: ExplorePopularity | null;
  /** The Managed skill this candidate already is, if any: its row routes to the inspector. */
  installedId: string | null;
  hidden: boolean;
};

/** Below this many characters a query filters the featured list only; at it, skills.sh is searched too. */
export const EXPLORE_MIN_ONLINE_QUERY = 2;

/** What the search mode line says: featured only, a too-short query (the hint), or featured + online. */
export type ExploreSearchMode = "featured" | "short" | "online";

export function exploreSearchMode(query: string): ExploreSearchMode {
  const length = query.trim().length;
  if (length === 0) return "featured";
  return length < EXPLORE_MIN_ONLINE_QUERY ? "short" : "online";
}

function normalizeGithubRepo(source: string): string {
  return source
    .replace(/^https:\/\/github\.com\//i, "")
    .replace(/\.git$/i, "")
    .split("/tree/")[0]
    .toLowerCase();
}

/**
 * The hidden-list key of a candidate: `name|owner/repo`, lower-cased. The
 * backend stores these verbatim (`hide_explore_skill`), so the shape never
 * changes; it is also how a candidate is matched to a Managed skill.
 */
export function exploreHideKey(skillName: string, source: string): string {
  return `${skillName.toLowerCase()}|${normalizeGithubRepo(source)}`;
}

/** The Managed-skill fields the installed match reads. */
export type ExploreManagedFields = { id: string; name: string; source_ref: string | null };

/**
 * Every row Explore can show, in the order the sources rank them (featured
 * by popularity as curated, then the search's own relevance). An online
 * result that is a featured skill (same key) is dropped, as is a repeat
 * within one list.
 */
export function exploreCandidates(input: {
  featured: readonly FeaturedSkillDto[];
  online: readonly OnlineSkillDto[];
  managed: readonly ExploreManagedFields[];
  hidden: ReadonlySet<string>;
}): ExploreCandidate[] {
  const installed = new Map<string, string>();
  for (const skill of input.managed) {
    const key = exploreHideKey(skill.name, skill.source_ref ?? "");
    if (!installed.has(key)) installed.set(key, skill.id);
  }
  const seen = new Set<string>();
  const out: ExploreCandidate[] = [];
  const push = (
    origin: ExploreOrigin,
    name: string,
    description: string | null,
    sourceUrl: string,
    fallbackLabel: string,
    popularity: ExplorePopularity | null,
  ) => {
    const key = exploreHideKey(name, sourceUrl);
    if (seen.has(key)) return;
    seen.add(key);
    out.push({
      id: `${origin}:${key}`,
      key,
      origin,
      name,
      description: description?.trim() ? description : null,
      sourceUrl,
      source_ref: sourceUrl,
      sourceLabel: repoInfo(sourceUrl)?.label ?? fallbackLabel,
      popularity,
      installedId: installed.get(key) ?? null,
      hidden: input.hidden.has(key),
    });
  };
  for (const skill of input.featured) {
    const popularity: ExplorePopularity | null =
      skill.downloads > 0
        ? { kind: "downloads", count: skill.downloads }
        : skill.stars > 0
          ? { kind: "stars", count: skill.stars }
          : null;
    push("featured", skill.name, skill.summary, skill.source_url, skill.source_url, popularity);
  }
  for (const skill of input.online) {
    const popularity: ExplorePopularity | null =
      skill.installs > 0 ? { kind: "installs", count: skill.installs } : null;
    push("online", skill.name, null, skill.source_url, skill.source || skill.source_url, popularity);
  }
  return out;
}

/** The Explore filter chips, in order; "Not in library" is the default (D25). */
export const EXPLORE_FILTER_ORDER = ["notInLibrary", "installed", "hidden"] as const;
export type ExploreFilter = (typeof EXPLORE_FILTER_ORDER)[number];

/**
 * The chips' predicates. They partition the candidates: a hidden candidate
 * is only under Hidden (installed or not), so the three counts add up.
 */
export const EXPLORE_FILTERS: Readonly<Record<ExploreFilter, (candidate: ExploreCandidate) => boolean>> = {
  notInLibrary: (candidate) => !candidate.hidden && candidate.installedId === null,
  installed: (candidate) => !candidate.hidden && candidate.installedId !== null,
  hidden: (candidate) => candidate.hidden,
};

/**
 * The query fold the Explore lens runs: a featured candidate matches on its
 * name, description or source (case-insensitive substring); an online result
 * is kept as is — skills.sh matched it to this query already. The ranked
 * order is kept (no sort).
 */
export function exploreQueryFold(items: readonly ExploreCandidate[], query: string): ExploreCandidate[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [...items];
  return items.filter(
    (candidate) =>
      candidate.origin === "online" ||
      candidate.name.toLowerCase().includes(needle) ||
      (candidate.description?.toLowerCase().includes(needle) ?? false) ||
      candidate.sourceLabel.toLowerCase().includes(needle),
  );
}

const COMPACT = new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 });
const FULL = new Intl.NumberFormat("en");

/** A popularity count as the row shows it ("12.3K") and in full ("12,345"). */
export function formatPopularity(count: number): { short: string; full: string } {
  return { short: COMPACT.format(count), full: FULL.format(count) };
}
