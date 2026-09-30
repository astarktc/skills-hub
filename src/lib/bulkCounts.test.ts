import { describe, expect, it } from "vitest";
import type { ManagedSkillDto as ManagedSkill, SkillTargetDto } from "../bindings";
import {
  deleteScope,
  deleteTotals,
  disableCounts,
  enableCounts,
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
