import i18next from "i18next";
import { beforeAll, describe, expect, it, vi } from "vitest";
import type { ManagedSkillDto as ManagedSkill, SkillTargetDto } from "../bindings";
import { resources } from "../i18n/resources";
import {
  RECENT_MAX,
  openSettingsPane,
  paletteGroups,
  rememberRecent,
  type PaletteGroup,
  type PaletteInput,
  isApplePlatform,
  isPaletteChord,
  type ChordKeys,
} from "./commandPalette";
import type { Translate } from "./skillPresentation";

let t: Translate;
beforeAll(async () => {
  const i18n = i18next.createInstance();
  await i18n.init({ resources, lng: "en", interpolation: { escapeValue: false } });
  t = i18n.t.bind(i18n) as unknown as Translate;
});

const deployed: SkillTargetDto = {
  tool: "claude",
  mode: "symlink",
  status: "synced",
  target_path: "/t/x",
  synced_at: 1,
  disabled: false,
  deployed: true,
};

function skill(id: string, extra: Partial<ManagedSkill> = {}): ManagedSkill {
  return {
    id,
    name: id,
    description: null,
    source_type: "git",
    source_ref: `https://github.com/acme/${id}-repo`,
    imported_from_tool: null,
    central_path: `/hub/${id}`,
    created_at: 0,
    updated_at: 0,
    last_sync_at: null,
    status: "active",
    invocation_mode: "user-and-model",
    invocation_override: null,
    targets: [deployed],
    disabled_at: null,
    refreshable: true,
    unlocatable: null,
    detachable: false,
    health: null,
    projects: [],
    ...extra,
  };
}

const library = [skill("zeta-writer"), skill("alpha-review"), skill("brainstorming")];

const fold = (input: Partial<PaletteInput> = {}): PaletteGroup[] =>
  paletteGroups({ query: "", skills: library, selected: [], blockedReason: null, recent: [], t, ...input });

const ids = (groups: PaletteGroup[]) => groups.flatMap((group) => group.items.map((item) => item.id));
const group = (groups: PaletteGroup[], id: PaletteGroup["id"]) => groups.find((entry) => entry.id === id)!;
const entry = (groups: PaletteGroup[], id: string) => groups.flatMap((g) => g.items).find((item) => item.id === id)!;

describe("paletteGroups — reach", () => {
  it("lists every skill by name, every area, Settings and its panes, and the library-wide actions", () => {
    const groups = fold();
    expect(group(groups, "skills").items.map((item) => item.label)).toEqual(["alpha-review", "brainstorming", "zeta-writer"]);
    expect(group(groups, "skills").items[0]).toMatchObject({
      hint: "acme/alpha-review-repo",
      refusal: null,
      intent: { kind: "skill", skillId: "alpha-review" },
    });
    expect(group(groups, "places").items.map((item) => [item.label, item.intent])).toEqual([
      ["My Skills", { kind: "area", area: "skills" }],
      ["Explore", { kind: "area", area: "explore" }],
      ["Projects", { kind: "area", area: "projects" }],
      ["Settings", { kind: "settings", pane: null }],
      ["Settings \u2192 General", { kind: "settings", pane: "general" }],
      ["Settings \u2192 Harnesses", { kind: "settings", pane: "harnesses" }],
      ["Settings \u2192 Appearance", { kind: "settings", pane: "appearance" }],
      ["Settings \u2192 Updates", { kind: "settings", pane: "updates" }],
      ["Settings \u2192 Diagnostics", { kind: "settings", pane: "diagnostics" }],
      ["Keyboard shortcuts", { kind: "shortcuts" }],
    ]);
    expect(group(groups, "actions").items.map((item) => [item.label, item.intent.kind])).toEqual([
      ["Update all", "updateAll"],
      ["Add skill\u2026", "add"],
      ["Configure harnesses\u2026", "configureHarnesses"],
      ["Disable every skill on all harnesses\u2026", "disableAll"],
    ]);
  });

  it("offers no library-wide verb on an empty library, and no selection group without a selection", () => {
    const groups = fold({ skills: [] });
    expect(group(groups, "actions").items.map((item) => item.id)).toEqual(["add", "harnesses"]);
    expect(group(groups, "selection").items).toEqual([]);
    expect(group(groups, "skills").items).toEqual([]);
  });

  it("puts every door verb of the showing surface's selection in its own group, its count in each label", () => {
    const selected = [library[0], library[1], library[2]];
    const groups = fold({ selected });
    const selection = group(groups, "selection");
    expect(selection.label).toBe("3 selected");
    expect(selection.items.map((item) => item.label)).toEqual([
      "Deploy 3 selected to harnesses\u2026",
      "Deploy 3 selected to a project\u2026",
      "Disable 3 selected on all harnesses\u2026",
      "Enable 3 selected\u2026",
      "Update 3 selected\u2026",
      "Remove 3 selected\u2026",
    ]);
    expect(entry(groups, "selection:disable").intent).toEqual({
      kind: "selection",
      verb: "disable",
      skillIds: ["zeta-writer", "alpha-review", "brainstorming"],
    });
  });

  it("ends a label in \u2026 exactly when the door opens a dialog first (one skill: Disable / Enable / Update act at once)", () => {
    const groups = fold({ selected: [skill("one", { disabled_at: 5 })] });
    expect(group(groups, "selection").items.map((item) => item.label)).toEqual([
      "Deploy 1 selected to harnesses\u2026",
      "Deploy 1 selected to a project\u2026",
      "Disable 1 selected on all harnesses",
      "Enable 1 selected",
      "Update 1 selected",
      "Remove 1 selected\u2026",
    ]);
  });
});

describe("paletteGroups — order", () => {
  it("orders recent \u2192 selection \u2192 places \u2192 actions \u2192 skills (the primitive highlights nothing on open, so the order never makes Enter run a mutation)", () => {
    const groups = fold();
    expect(groups.map((g) => g.id)).toEqual(["recent", "selection", "places", "actions", "skills"]);
    expect(ids(groups)[0]).toBe("place:skills");
  });

  it("lists recently used entries first, most recent first, out of their home groups; unknown ids drop", () => {
    const groups = fold({ recent: ["skill:brainstorming", "all:update", "skill:gone"] });
    expect(group(groups, "recent").items.map((item) => item.id)).toEqual(["skill:brainstorming", "all:update"]);
    expect(group(groups, "skills").items.map((item) => item.id)).toEqual(["skill:alpha-review", "skill:zeta-writer"]);
    expect(group(groups, "actions").items.map((item) => item.id)).not.toContain("all:update");
    expect(ids(groups)[0]).toBe("skill:brainstorming");
  });

  it("remembers at most RECENT_MAX ids, most recent first, without duplicates", () => {
    let recent: string[] = [];
    for (const id of ["a", "b", "c", "d", "e", "f", "b"]) recent = rememberRecent(recent, id);
    expect(RECENT_MAX).toBe(5);
    expect(recent).toEqual(["b", "f", "e", "d", "c"]);
  });
});

describe("paletteGroups — typed-ahead (the list lens's query rule)", () => {
  it("matches skills through the My Skills fold: name, source, wildcard", () => {
    expect(group(fold({ query: "BRAIN" }), "skills").items.map((item) => item.label)).toEqual(["brainstorming"]);
    expect(group(fold({ query: "zeta-writer-repo" }), "skills").items.map((item) => item.label)).toEqual(["zeta-writer"]);
    expect(group(fold({ query: "a*review" }), "skills").items.map((item) => item.label)).toEqual(["alpha-review"]);
  });

  it("matches commands by label and by their hidden keywords, with the same rule", () => {
    expect(ids(fold({ query: "proj" }))).toEqual(["place:projects"]);
    expect(ids(fold({ query: "remove all" }))).toEqual(["all:disable"]);
    const selected = [library[0], library[1]];
    expect(ids(fold({ query: "delete", selected }))).toEqual(["selection:delete"]);
    expect(ids(fold({ query: "disable 2", selected }))).toEqual(["selection:disable"]);
  });

  it("keeps a matching recent entry first while typing", () => {
    const groups = fold({ query: "e", recent: ["place:explore"] });
    expect(ids(groups)[0]).toBe("place:explore");
  });
});

describe("paletteGroups — refusal (D19)", () => {
  it("keeps every verb listed while something runs, refused with the lifecycle's reason; places and skills stay open", () => {
    const groups = fold({ selected: [library[0], library[1]], blockedReason: "Update all is running" });
    for (const item of [...group(groups, "selection").items, ...group(groups, "actions").items]) {
      expect(item.refusal).toBe("Update all is running");
    }
    for (const item of [...group(groups, "places").items, ...group(groups, "skills").items]) {
      expect(item.refusal).toBeNull();
    }
  });

  it("says when a verb would change nothing, from the bulk rail's folds", () => {
    const off = [skill("a", { disabled_at: 1, refreshable: false }), skill("b", { disabled_at: 1, refreshable: false })];
    const groups = fold({ skills: off, selected: off });
    expect(entry(groups, "selection:disable").refusal).toBe("Every selected skill is already off everywhere");
    expect(entry(groups, "selection:update").refusal).toBe("No selected skill has a source to update from");
    expect(entry(groups, "selection:enable").refusal).toBeNull();
    expect(entry(groups, "all:disable").refusal).toBe("Every skill is already off everywhere");
    expect(entry(groups, "all:update").refusal).toBe("No skill has a source to update from");
  });
});

describe("openSettingsPane — the one route of every Settings entry", () => {
  it("opens the Settings dialog on the named pane (General by default), never an area", () => {
    const surface = { openDialog: vi.fn() };
    openSettingsPane(surface, null);
    openSettingsPane(surface, "harnesses");
    openSettingsPane(surface, null, "githubToken");
    expect(surface.openDialog.mock.calls).toEqual([
      [{ kind: "settings", pane: "general" }],
      [{ kind: "settings", pane: "harnesses" }],
      [{ kind: "settings", pane: "diagnostics", field: "githubToken" }],
    ]);
  });
});

describe("paletteGroups — Settings panes by what they hold", () => {
  it("finds the token's pane by its old name and the Harnesses pane by its own", () => {
    const ids = (query: string) => group(fold({ query }), "places").items.map((item) => item.id);
    expect(ids("github")).toContain("settings:diagnostics");
    expect(ids("harnesses")).toContain("settings:harnesses");
    expect(ids("theme")).toContain("settings:appearance");
  });
});

describe("isPaletteChord — one chord per platform", () => {
  const k = (mods: Partial<ChordKeys>): ChordKeys => ({ key: "k", metaKey: false, ctrlKey: false, altKey: false, shiftKey: false, ...mods });
  it("is ⌘K on Apple platforms and never Ctrl+K there (kill-line in a text field)", () => {
    expect(isPaletteChord(k({ metaKey: true }), true)).toBe(true);
    expect(isPaletteChord(k({ ctrlKey: true }), true)).toBe(false);
    expect(isPaletteChord(k({ metaKey: true, ctrlKey: true }), true)).toBe(false);
  });
  it("is Ctrl+K elsewhere and never ⌘K there", () => {
    expect(isPaletteChord(k({ ctrlKey: true }), false)).toBe(true);
    expect(isPaletteChord(k({ metaKey: true }), false)).toBe(false);
  });
  it("rejects extra modifiers and other keys; the key is case-insensitive", () => {
    expect(isPaletteChord(k({ metaKey: true, shiftKey: true }), true)).toBe(false);
    expect(isPaletteChord(k({ metaKey: true, altKey: true }), true)).toBe(false);
    expect(isPaletteChord(k({ metaKey: true, key: "K" }), true)).toBe(true);
    expect(isPaletteChord(k({ metaKey: true, key: "j" }), true)).toBe(false);
  });
  it("reads the platform string the way navigator.platform spells it", () => {
    expect(isApplePlatform("MacIntel")).toBe(true);
    expect(isApplePlatform("iPhone")).toBe(true);
    expect(isApplePlatform("Win32")).toBe(false);
    expect(isApplePlatform("Linux x86_64")).toBe(false);
  });
});
