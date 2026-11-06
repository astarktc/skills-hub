import i18next from "i18next";
import { describe, expect, it } from "vitest";
import { resources } from "../i18n/resources";
import type { ManagedSkillDto as ManagedSkill, SkillTargetDto } from "../bindings";
import {
  deleteScope,
  deleteTotals,
  disableConfirmLines,
  disableCounts,
  enableConfirmLines,
  enableCounts,
  harnessArrivals,
  syncCounts,
  updateCounts,
  type AssignmentsOf,
} from "./bulkCounts";

const target = (tool: string, status: SkillTargetDto["status"] = "synced", synced = true): SkillTargetDto => ({
  tool,
  mode: "symlink",
  status,
  target_path: `/tools/${tool}/x`,
  synced_at: synced ? 1 : null,
});

function skill(id: string, targets: SkillTargetDto[], extra: Partial<ManagedSkill> = {}): ManagedSkill {
  return {
    id,
    name: id,
    description: null,
    source_type: "git",
    source_ref: `https://github.com/o/${id}`,
    imported_from_tool: null,
    central_path: `/hub/${id}`,
    created_at: 0,
    updated_at: 0,
    last_sync_at: null,
    status: "active",
    invocation_mode: "user-and-model",
    invocation_override: null,
    targets,
    disabled_at: null,
    refreshable: true,
    unlocatable: null,
    detachable: false,
    health: null,
    projects: [],
    ...extra,
  };
}

const placements: Record<string, ReturnType<AssignmentsOf>> = {
  a: [
    { projectId: "p1", projectName: "quartermaster", assignments: [{ tool: "claude", status: "synced" }, { tool: "pi", status: "disabled" }] },
    { projectId: "p2", projectName: "monorepo", assignments: [] },
  ],
  b: [
    { projectId: "p1", projectName: "quartermaster", assignments: [] },
    { projectId: "p2", projectName: "monorepo", assignments: [{ tool: "codex", status: "error" }] },
  ],
  c: [],
};
const assignmentsOf: AssignmentsOf = (id) => placements[id] ?? null;

describe("disableCounts", () => {
  it("counts the materialised targets that go, the distinct Tools and the assignments not yet disabled", () => {
    const skills = [
      skill("a", [target("claude"), target("cursor", "stale"), target("pi", "disabled")]),
      // A never-synced error row owns nothing: it only flips, it is not "removed".
      skill("b", [target("claude", "error", false), target("codex", "missing")]),
      skill("c", [target("claude", "disabled")], { disabled_at: 5 }),
    ];
    expect(disableCounts(skills, assignmentsOf)).toEqual({
      skills: 3,
      alreadyDisabled: 1,
      tools: 3, // claude, cursor, codex
      targets: 3, // a:claude, a:cursor, b:codex
      assignments: 2, // a:claude (synced), b:codex (error)
      assigned: 3, // every assignment row, a:pi (already disabled) included
    });
  });

  it("says the assignment count is unknown while any placement is unread", () => {
    expect(disableCounts([skill("a", []), skill("zz", [])], assignmentsOf).assignments).toBeNull();
    expect(disableCounts([skill("a", [])], null).assignments).toBeNull();
  });
});

describe("enableCounts", () => {
  it("counts the disabled rows that re-sync and the skills that change nothing", () => {
    const skills = [
      skill("a", [target("claude", "disabled"), target("pi", "disabled"), target("cursor")]),
      skill("b", [target("claude")]),
      skill("c", [target("codex", "disabled")], { disabled_at: 5 }),
    ];
    expect(enableCounts(skills, assignmentsOf)).toEqual({
      skills: 3,
      alreadyEnabled: 1,
      tools: 3,
      targets: 3,
      assignments: 1, // a: pi disabled
    });
  });
});

// BACKLOG #73: what the Disable / Enable confirmations say, case by case
// (the numbers stay disableCounts' / enableCounts').
describe("the Disable / Enable confirmation sentences", () => {
  const t = (key: string, opts?: Record<string, unknown>) => (opts ? `${key} ${JSON.stringify(opts)}` : key);
  const disable = { skills: 3, alreadyDisabled: 0, tools: 2, targets: 4, assignments: 2, assigned: 2 };
  const enable = { skills: 3, alreadyEnabled: 0, tools: 2, targets: 4, assignments: 2 };
  const harness = 'bulk.confirm.disable.removes {"targets":"bulk.count.targets {\\"count\\":4}","tools":"bulk.count.tools {\\"count\\":2}"}';
  const stays = 'bulk.confirm.disable.stays {"count":3}';

  it("Disable: what leaves the harnesses, what turns off in projects, then that the skills stay, off", () => {
    expect(disableConfirmLines(disable, t)).toEqual([
      `${harness} bulk.confirm.disable.turnsOff {"assignments":"bulk.count.assignments {\\"count\\":2}"}`,
      stays,
    ]);
  });

  it("Disable with no project assignment says so; with every one already off says that", () => {
    expect(disableConfirmLines({ ...disable, assignments: 0, assigned: 0 }, t)[0]).toBe(
      `${harness} bulk.confirm.disable.noAssignments {"count":3}`,
    );
    expect(disableConfirmLines({ ...disable, assignments: 0, assigned: 2 }, t)[0]).toBe(
      `${harness} bulk.confirm.disable.assignmentsAlreadyOff {"count":3}`,
    );
  });

  it("Disable with nothing deployed globally, and while the projects are unread", () => {
    expect(disableConfirmLines({ ...disable, targets: 0, tools: 0, assignments: 0, assigned: 0 }, t)[0]).toBe(
      'bulk.confirm.disable.notDeployed {"count":3} bulk.confirm.disable.noAssignments {"count":3}',
    );
    expect(disableConfirmLines({ ...disable, assignments: null, assigned: null, alreadyDisabled: 1 }, t)).toEqual([
      harness,
      "bulk.confirm.readingProjects",
      stays,
      'bulk.confirm.disable.already {"count":1}',
    ]);
  });

  it("Enable: what is redeployed, globally and in projects, each part only when it has a count", () => {
    const targets = '"targets":"bulk.count.targets {\\"count\\":4}","tools":"bulk.count.tools {\\"count\\":2}"';
    const assignments = '"assignments":"bulk.count.assignments {\\"count\\":2}"';
    expect(enableConfirmLines(enable, t)).toEqual([`bulk.confirm.enable.redeploysBoth {${targets},${assignments}}`]);
    expect(enableConfirmLines({ ...enable, assignments: 0 }, t)).toEqual([`bulk.confirm.enable.redeploysGlobal {${targets}}`]);
    expect(enableConfirmLines({ ...enable, targets: 0, tools: 0 }, t)).toEqual([
      `bulk.confirm.enable.redeploysProjects {${assignments}}`,
    ]);
    expect(enableConfirmLines({ ...enable, targets: 0, tools: 0, assignments: 0 }, t)).toEqual([
      'bulk.confirm.enable.nothingToRedeploy {"count":3}',
    ]);
    // While placements are unread, "nothing changes" is not claimed.
    expect(enableConfirmLines({ ...enable, assignments: null, alreadyEnabled: 2 }, t)).toEqual([
      `bulk.confirm.enable.redeploysGlobal {${targets}}`,
      "bulk.confirm.readingProjects",
    ]);
    expect(enableConfirmLines({ ...enable, assignments: 0, alreadyEnabled: 2 }, t)).toEqual([
      `bulk.confirm.enable.redeploysGlobal {${targets}}`,
      'bulk.confirm.enable.already {"count":2}',
    ]);
  });

  // L4 review (Sol): a project-only disabled deployment is something Enable redeploys.
  it("Enable counts a skill whose only disabled row is a project deployment as changing", async () => {
    const i18n = i18next.createInstance();
    await i18n.init({ resources, lng: "en", interpolation: { escapeValue: false } });
    const en = i18n.t.bind(i18n) as unknown as typeof t;
    const counts = enableCounts([skill("a", [])], assignmentsOf); // a: pi disabled in quartermaster
    expect(counts).toMatchObject({ alreadyEnabled: 0, targets: 0, assignments: 1 });
    expect(enableConfirmLines(counts, en)).toEqual(["Redeploys 1 project deployment."]);
  });

  // L4 review (Sol): zero redeployments describes the operation, not the skills' history.
  it("Enable over a mixed selection with nothing to restore says what it does", async () => {
    const i18n = i18next.createInstance();
    await i18n.init({ resources, lng: "en", interpolation: { escapeValue: false } });
    const en = i18n.t.bind(i18n) as unknown as typeof t;
    const counts = enableCounts([skill("on", [target("cursor")]), skill("off", [], { disabled_at: 5 })], () => []);
    expect(enableConfirmLines(counts, en)).toEqual([
      "Switches the skills back on. No deployments need to be restored.",
      "1 of them is already on; nothing changes for it.",
    ]);
  });
});

describe("syncCounts", () => {
  it("splits skills × Tools into added, already synced and gated pairs", () => {
    const skills = [
      skill("a", [target("claude"), target("pi", "disabled")]),
      skill("b", [target("cursor", "error", false)]),
      skill("c", [], { disabled_at: 1 }),
    ];
    expect(syncCounts(skills, ["claude", "pi", "cursor"])).toEqual({
      skills: 3,
      tools: 3,
      pairs: 9,
      alreadySynced: 1, // a:claude
      disabled: 4, // a:pi + c × 3
      added: 4,
    });
  });

  it("counts one Deployment per folder when two picked harnesses share it (Sol 105)", () => {
    // Amp and Kimi Code both picked: each names the folder `amp`
    // (`ToolInfoDto.folder_key`), which the batch writes once.
    const skills = [skill("fresh", []), skill("there", [target("amp")])];
    expect(syncCounts(skills, ["amp", "amp", "claude"])).toEqual({
      skills: 2,
      tools: 2,
      pairs: 4,
      alreadySynced: 1, // there:amp
      disabled: 0,
      added: 3, // fresh:amp, fresh:claude, there:claude
    });
  });
});

describe("harnessArrivals (D48)", () => {
  it("counts the enabled, locatable skills with no deployment on the harness yet", () => {
    const skills = [
      skill("fresh", [target("claude")]),
      skill("failedThere", [target("cursor", "error", false)]),
      skill("deployed", [target("cursor")]),
      skill("outOfDate", [target("cursor", "stale")]),
      skill("pairOff", [target("cursor", "disabled")]),
      skill("skillOff", [], { disabled_at: 5 }),
      skill("gone", [], { unlocatable: "central_missing" }),
    ];
    expect(harnessArrivals(skills, "cursor").map((s) => s.id)).toEqual(["fresh", "failedThere"]);
  });

  it("reads the harness's Deployment folder: Kimi Code gets nothing Amp's folder already has", () => {
    const skills = [skill("onAmp", [target("amp")]), skill("nowhere", [])];
    // Kimi Code is served by the folder `amp` (`ToolInfoDto.folder_key`).
    expect(harnessArrivals(skills, "amp").map((s) => s.id)).toEqual(["nowhere"]);
  });
});

describe("updateCounts", () => {
  it("skips what Refresh cannot re-acquire", () => {
    expect(updateCounts([skill("a", []), skill("b", [], { refreshable: false })])).toEqual({
      skills: 2,
      refreshable: 1,
      skipped: 1,
    });
  });
});

describe("deleteScope / deleteTotals", () => {
  const labelOf = (tool: string) => tool.toUpperCase();

  it("lists the deployed Tools by label, the projects carrying it, the Edits lost and the only-copy flag", () => {
    const edited = skill("a", [target("claude"), target("pi", "disabled"), target("cursor", "error", false)], {
      invocation_override: { mode: "user-only", base_mode: "user-and-model", conflict: false },
    });
    expect(deleteScope(edited, assignmentsOf, labelOf)).toEqual({
      skillId: "a",
      name: "a",
      tools: ["CLAUDE"],
      projects: ["quartermaster"],
      edits: 1,
      onlyCopy: false,
    });
    const imported = skill("c", [], { source_type: "imported", source_ref: null });
    expect(deleteScope(imported, assignmentsOf, labelOf)).toMatchObject({ projects: [], onlyCopy: true, edits: 0 });
    expect(deleteScope(skill("zz", []), assignmentsOf, labelOf).projects).toBeNull();
  });

  it("sums scopes with distinct Tools and projects", () => {
    const scopes = [
      deleteScope(skill("a", [target("claude"), target("cursor")]), assignmentsOf, labelOf),
      deleteScope(skill("b", [target("claude")], { source_ref: null }), assignmentsOf, labelOf),
    ];
    expect(deleteTotals(scopes)).toEqual({ skills: 2, tools: 2, targets: 3, projects: 2, edits: 0, onlyCopy: 1 });
    expect(deleteTotals([...scopes, deleteScope(skill("zz", []), assignmentsOf, labelOf)]).projects).toBeNull();
  });
});
