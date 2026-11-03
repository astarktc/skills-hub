import type { ManagedSkillDto as ManagedSkill } from "../bindings";
import type { SurfaceState } from "../hooks/useSurfaceState";
import { disableCounts, enableCounts, updateCounts } from "./bulkCounts";
import { filterAndSortSkills, queryMatcher, repoInfo, type Translate } from "./skillPresentation";

/**
 * The ⌘K palette's item fold (round-19 ticket 23): what the palette lists for
 * a query, in order, each entry with the intent it runs and — when it cannot
 * run now — the reason. Pure: the palette component executes intents through
 * the surface module and the skill action door; nothing here touches either
 * (except `openSettingsPane`, the one route of every Settings entry, at the end).
 *
 * Reach (one entry each):
 * - every skill by name → focus it (the inspector opens);
 * - every area (My Skills · Explore · Projects), Settings and each Settings pane;
 * - every door verb on the **showing surface's selection** (D4), its count in
 *   the label — "Disable 3 selected on all harnesses…" — the same verbs as
 *   the bulk rail, refused for the same reasons (`bulkCounts`);
 * - Update all, Add skill…, Configure harnesses…, and the library-wide
 *   undeploy "Disable every skill on all harnesses…" (D8: the same count
 *   confirmation as select-all → Disable; found by "remove" too).
 *
 * Order: recently used first (session memory, `RECENT_MAX`, most recent
 * first), then the selection's verbs, the places, the actions, the skills.
 * Nothing is highlighted on open (the primitive's rule), so ⌘K → Enter with no
 * query runs nothing — recents may hold Update all. ↓ reaches the first entry;
 * a selection's first verb (Deploy…) only opens its dialog.
 * A query keeps the order and filters every group through the list lens's
 * query rule (`queryMatcher`; skills through `filterAndSortSkills`, the My
 * Skills fold itself, sorted by name).
 *
 * A verb refused by the lifecycle (something runs: `blockedReason`) stays
 * listed with that reason; a verb that would change nothing says so. Places
 * and skills are never refused: looking is always allowed (D19).
 */

/** How many recently used entries the palette remembers (this session only). */
export const RECENT_MAX = 5;

/** The Settings panes the palette reaches today (ticket 20 adds the rest with its panes). */
export const SETTINGS_PANES = ["github"] as const;
export type SettingsPane = (typeof SETTINGS_PANES)[number];

/** The door verbs the palette offers on a selection: the bulk rail's. */
export const SELECTION_VERBS = ["syncToTargets", "assignToProject", "disable", "enable", "update", "delete"] as const;
export type SelectionVerb = (typeof SELECTION_VERBS)[number];

export type PaletteArea = "skills" | "explore" | "projects";

/** What an entry does when chosen. Data only: the palette component runs it. */
export type PaletteIntent =
  | { kind: "skill"; skillId: string }
  | { kind: "area"; area: PaletteArea }
  | { kind: "settings"; pane: SettingsPane | null }
  | { kind: "selection"; verb: SelectionVerb; skillIds: readonly string[] }
  | { kind: "updateAll" }
  | { kind: "disableAll" }
  | { kind: "add" }
  | { kind: "configureHarnesses" };

export type PaletteGroupId = "recent" | "selection" | "actions" | "places" | "skills";

export type PaletteEntry = {
  /** Stable across renders and sessions' recents: `skill:<id>`, `place:projects`, `selection:disable`, … */
  id: string;
  label: string;
  /** Words the query also matches, never shown. */
  keywords: string;
  /** A quiet second fact (a skill's repository). */
  hint: string | null;
  /** Why it cannot run now, or null. */
  refusal: string | null;
  intent: PaletteIntent;
};

export type PaletteGroup = { id: PaletteGroupId; label: string; items: PaletteEntry[] };

export type PaletteInput = {
  query: string;
  /** The library (the skills world's catalog). */
  skills: readonly ManagedSkill[];
  /** The showing surface's selected skills (D4), resolved; empty when it has none. */
  selected: readonly ManagedSkill[];
  /** The lifecycle's refusal in words (D19), or null when nothing runs. */
  blockedReason: string | null;
  /** Recently chosen entry ids, most recent first. */
  recent: readonly string[];
  t: Translate;
};

/** Which door verbs open a dialog (so their label ends in "…") for `count` skills. */
function asksFirst(verb: SelectionVerb, count: number): boolean {
  if (verb === "syncToTargets" || verb === "assignToProject" || verb === "delete") return true;
  return count > 1;
}

function selectionEntries(selected: readonly ManagedSkill[], blocked: string | null, t: Translate): PaletteEntry[] {
  const count = selected.length;
  if (count === 0) return [];
  const skillIds = selected.map((skill) => skill.id);
  // The rail's "would change nothing" reasons, from the same folds.
  const idle: Record<SelectionVerb, string | null> = {
    syncToTargets: null,
    assignToProject: null,
    disable: disableCounts(selected, null).alreadyDisabled === count ? t("bulk.nothingToDisable") : null,
    enable: enableCounts(selected, null).alreadyEnabled === count ? t("bulk.nothingToEnable") : null,
    update: updateCounts(selected).refreshable === 0 ? t("bulk.nothingToUpdate") : null,
    delete: null,
  };
  return SELECTION_VERBS.map((verb) => ({
    id: `selection:${verb}`,
    label: t(`palette.selection.${verb}`, { count }) + (asksFirst(verb, count) ? "\u2026" : ""),
    keywords: verb === "delete" ? t("palette.keywords.delete") : "",
    hint: null,
    refusal: blocked ?? idle[verb],
    intent: { kind: "selection", verb, skillIds },
  }));
}

function actionEntries(skills: readonly ManagedSkill[], blocked: string | null, t: Translate): PaletteEntry[] {
  const entries: PaletteEntry[] = [];
  if (skills.length > 0) {
    entries.push({
      id: "all:update",
      label: t("library.updateAll"),
      keywords: "",
      hint: null,
      refusal: blocked ?? (updateCounts(skills).refreshable === 0 ? t("palette.nothingToUpdateAll") : null),
      intent: { kind: "updateAll" },
    });
  }
  entries.push(
    {
      id: "add",
      label: `${t("addSkillButton")}\u2026`,
      keywords: t("palette.keywords.add"),
      hint: null,
      refusal: blocked,
      intent: { kind: "add" },
    },
    {
      id: "harnesses",
      label: `${t("configureTools")}\u2026`,
      keywords: "",
      hint: null,
      refusal: blocked,
      intent: { kind: "configureHarnesses" },
    },
  );
  if (skills.length > 0) {
    const allOff = disableCounts(skills, null).alreadyDisabled === skills.length;
    entries.push({
      id: "all:disable",
      label: t("palette.disableAll"),
      keywords: t("palette.keywords.disableAll"),
      hint: null,
      refusal: blocked ?? (allOff ? t("palette.nothingToDisableAll") : null),
      intent: { kind: "disableAll" },
    });
  }
  return entries;
}

function placeEntries(t: Translate): PaletteEntry[] {
  const place = (area: PaletteArea, label: string): PaletteEntry => ({
    id: `place:${area}`,
    label,
    keywords: t("palette.keywords.place"),
    hint: null,
    refusal: null,
    intent: { kind: "area", area },
  });
  return [
    place("skills", t("navMySkills")),
    place("explore", t("navExplore")),
    place("projects", t("navProjects")),
    {
      id: "place:settings",
      label: t("settings"),
      keywords: t("palette.keywords.place"),
      hint: null,
      refusal: null,
      intent: { kind: "settings", pane: null },
    },
    ...SETTINGS_PANES.map(
      (pane): PaletteEntry => ({
        id: `settings:${pane}`,
        label: t("palette.settingsPane", { pane: t(`palette.panes.${pane}`) }),
        keywords: t("palette.keywords.place"),
        hint: null,
        refusal: null,
        intent: { kind: "settings", pane },
      }),
    ),
  ];
}

function skillEntry(skill: ManagedSkill): PaletteEntry {
  return {
    id: `skill:${skill.id}`,
    label: skill.name,
    keywords: "",
    hint: repoInfo(skill.source_ref)?.label ?? null,
    refusal: null,
    intent: { kind: "skill", skillId: skill.id },
  };
}

/** The palette's groups for one query, in order; a group with nothing to show is empty. */
export function paletteGroups({ query, skills, selected, blockedReason, recent, t }: PaletteInput): PaletteGroup[] {
  const matches = queryMatcher(query);
  const commandMatches = (entry: PaletteEntry) => matches(entry.label) || (entry.keywords !== "" && matches(entry.keywords));

  const selection = selectionEntries(selected, blockedReason, t).filter(commandMatches);
  const actions = actionEntries(skills, blockedReason, t).filter(commandMatches);
  const places = placeEntries(t).filter(commandMatches);
  // The lens's own fold: the same fields, the same rule, sorted by name.
  const skillRows = filterAndSortSkills([...skills], { query, sort: "name" }).map(skillEntry);

  // Recently used first: pulled out of their home groups, most recent first.
  const recentIds = new Set(recent);
  const byId = new Map<string, PaletteEntry>();
  for (const entry of [...selection, ...actions, ...places, ...skillRows]) byId.set(entry.id, entry);
  const recentEntries = recent.flatMap((id) => {
    const entry = byId.get(id);
    return entry ? [entry] : [];
  });
  const rest = (entries: PaletteEntry[]) => entries.filter((entry) => !recentIds.has(entry.id));

  return [
    { id: "recent", label: t("palette.groups.recent"), items: recentEntries },
    { id: "selection", label: t("palette.groups.selection", { count: selected.length }), items: rest(selection) },
    { id: "places", label: t("palette.groups.places"), items: rest(places) },
    { id: "actions", label: t("palette.groups.actions"), items: rest(actions) },
    { id: "skills", label: t("palette.groups.skills"), items: rest(skillRows) },
  ];
}

/** Remember a chosen entry: most recent first, no duplicates, at most `max`. */
export function rememberRecent(recent: readonly string[], id: string, max: number = RECENT_MAX): string[] {
  return [id, ...recent.filter((entry) => entry !== id)].slice(0, max);
}

/**
 * Settings, or one of its panes — the **one** route every "Settings → …"
 * palette entry takes. Today Settings is an area (`#/settings/<pane>`);
 * ticket 20 makes it a dialog (D37) and retargets this function alone.
 */
export function openSettingsPane(
  surface: Pick<SurfaceState, "open" | "focus">,
  pane: SettingsPane | null,
): void {
  if (pane) surface.focus({ kind: "settings", pane });
  else surface.open("settings");
}

/** The modifier keys a chord reads; a subset of `KeyboardEvent`. */
export type ChordKeys = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">;

/** Whether `platform` (as `navigator.platform` spells it) is an Apple one, where ⌘ is the command modifier. */
export const isApplePlatform = (platform: string): boolean => /Mac|iPhone|iPad|iPod/.test(platform);

/**
 * The palette's chord: ⌘K on Apple platforms, Ctrl+K elsewhere — never both,
 * since Ctrl+K inside a macOS text field is kill-line and the palette must not
 * take it. A chord, so it works from a text field; nothing else held.
 */
export const isPaletteChord = (event: ChordKeys, apple: boolean): boolean =>
  (apple ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey) &&
  !event.altKey &&
  !event.shiftKey &&
  event.key.toLowerCase() === "k";
