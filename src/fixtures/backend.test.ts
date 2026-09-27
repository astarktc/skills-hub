import type { Channel } from "@tauri-apps/api/core";
import { describe, expect, expectTypeOf, it } from "vitest";
import {
  commands,
  type ManagedSkillDto,
  type ProjectDto,
  type ProjectViewDto,
  type SyncMode,
  type SyncStatus,
  type ToolStatusDto,
} from "../bindings";
import type { CommandName } from "../lib/tauri";
import { createFixtureBackend, type FixtureHandlers } from "./backend";
import { SCENARIOS, type FixtureState, type ScenarioName } from "./model";
import { TOOL_REGISTRY } from "./registry";
import { buildScenario, DETECTED_TOOLS } from "./scenarios";

const NOW = Date.UTC(2026, 8, 27, 12);

function backendFor(name: ScenarioName) {
  return createFixtureBackend(buildScenario(name, NOW), { latencyScale: 0, now: () => NOW });
}

/** A progress sink recording every tick (stands in for a Tauri Channel). */
function sink<T>() {
  const ticks: T[] = [];
  const channel = { onmessage: (tick: T) => ticks.push(tick) } as unknown as Channel<T>;
  return { ticks, channel };
}

const SYNC_STATUSES: readonly SyncStatus[] = ["pending", "synced", "stale", "missing", "error"];
const SYNC_MODES: readonly SyncMode[] = ["symlink", "junction", "copy"];
const REGISTRY_KEYS = new Set(TOOL_REGISTRY.map((t) => t.key));

describe("fixture backend: command coverage", () => {
  it("has exactly one handler per generated command", () => {
    const { handlers } = backendFor("rich");
    expect(Object.keys(handlers).sort()).toEqual(Object.keys(commands).sort());
  });

  it("types each handler by its command's own signature", () => {
    expectTypeOf<keyof FixtureHandlers>().toEqualTypeOf<CommandName>();
    expectTypeOf<Awaited<ReturnType<FixtureHandlers["getManagedSkills"]>>>().toEqualTypeOf<ManagedSkillDto[]>();
    expectTypeOf<Awaited<ReturnType<FixtureHandlers["getProjectView"]>>>().toEqualTypeOf<ProjectViewDto>();
    expectTypeOf<Parameters<FixtureHandlers["toggleProjectSkillAssignment"]>>().toEqualTypeOf<
      [projectId: string, skillId: string, tool: string]
    >();
  });
});

describe.each(SCENARIOS)("scenario %s: wire invariants", (name) => {
  it("only holds states the real backend emits", async () => {
    const { handlers } = backendFor(name);
    const skills: ManagedSkillDto[] = await handlers.getManagedSkills();
    const projects: ProjectDto[] = await handlers.listProjects();

    expect(new Set(skills.map((s) => s.id)).size).toBe(skills.length);
    expect(new Set(skills.map((s) => s.name)).size).toBe(skills.length);
    for (const skill of skills) {
      expect(["git", "local", "imported"]).toContain(skill.source_type);
      expect(skill.refreshable).toBe(skill.source_type !== "imported");
      if (skill.source_type === "imported") expect(skill.source_ref).toBeNull();
      if (skill.detachable) expect(skill.source_type).toBe("local");
      if (skill.unlocatable === "source_missing") expect(skill.source_type).toBe("local");
      if (skill.unlocatable === "central_missing") expect(skill.refreshable).toBe(true);
      for (const target of skill.targets) {
        expect(REGISTRY_KEYS.has(target.tool)).toBe(true);
        expect(SYNC_MODES).toContain(target.mode);
        // Global target rows are synced, or error after a failed
        // propagation/removal. `stale` is copy drift of a project
        // assignment — never "update available".
        expect(["synced", "error"]).toContain(target.status);
      }
    }

    for (const project of projects) {
      const view = await handlers.getProjectView(project.id);
      expect(view.project.assignment_count).toBe(view.assignments.length);
      for (const a of view.assignments) {
        expect(SYNC_STATUSES).toContain(a.status);
        if (a.status === "stale") expect(a.mode).toBe("copy");
        if (a.status === "error") expect(a.last_error).not.toBeNull();
        expect(skills.some((s) => s.id === a.skill_id)).toBe(true);
      }
    }
  });
});

describe("rich", () => {
  it("is the prototype roster: 60 skills, 7 detected tools, 3 projects", async () => {
    const { handlers } = backendFor("rich");
    const status: ToolStatusDto = await handlers.getToolStatus();
    expect(status.installed.sort()).toEqual([...DETECTED_TOOLS].sort());
    expect(status.newly_installed).toEqual([]);
    expect(await handlers.getManagedSkills()).toHaveLength(60);
    expect((await handlers.listProjects()).map((p) => p.name)).toEqual(["skills-hub", "quartermaster", "agent-skills"]);
  });

  it("raises TARGET_EXISTS on an occupied target, and replaces it under an override", async () => {
    const { handlers } = backendFor("rich");
    const zod = (await handlers.getManagedSkills()).find((s) => s.name === "zod-schema-first")!;
    const batch = [{ skill_id: zod.id, name: zod.name, source_path: zod.central_path }];
    const progress = sink<{ index: number; total: number }>();

    const first = await handlers.syncSkillsToTools(batch, ["cursor"], { overwrite_if_same_content: true }, progress.channel );
    expect(first[0].status).toEqual({ status: "failed", error: { code: "TARGET_EXISTS", path: "/Users/alex/.cursor/skills/zod-schema-first" } });
    expect(progress.ticks).toEqual([{ index: 1, total: 1, skill_name: "zod-schema-first", tool: "cursor" }]);

    const retry = await handlers.syncSkillsToTools(
      batch,
      ["cursor"],
      { overrides: [{ skill_id: zod.id, tool: "cursor", overwrite: true }] },
      sink().channel
    );
    expect(retry[0].status).toMatchObject({ status: "synced", outcome: { replaced: true } });
  });

  it("dedupes a shared skills dir and records every detected sharer", async () => {
    const { handlers } = backendFor("rich");
    const vitest = (await handlers.getManagedSkills()).find((s) => s.name === "vitest-migration")!;
    const report = await handlers.syncSkillsToTools(
      [{ skill_id: vitest.id, name: vitest.name, source_path: vitest.central_path }],
      ["amp", "kimi_cli"],
      {},
      sink().channel
    );
    expect(report.map((r) => r.tool_key)).toEqual(["amp"]);
    const after = (await handlers.getManagedSkills()).find((s) => s.id === vitest.id)!;
    expect(after.targets.map((t) => t.tool)).toEqual(expect.arrayContaining(["amp", "kimi_cli"]));
  });

  it("streams both Refresh phases and reports every refreshable skill", async () => {
    const { handlers } = backendFor("rich");
    const progress = sink<{ phase: string }>();
    const report = await handlers.refreshManagedSkills(null, { reassert_auto_sync: true }, progress.channel );
    expect(report.skills).toHaveLength(60);
    expect(new Set(progress.ticks.map((t) => t.phase))).toEqual(new Set(["acquiring", "applying"]));
    const worktrees = (await handlers.getManagedSkills()).find((s) => s.name === "using-git-worktrees")!;
    expect(worktrees.targets.every((t) => t.status === "synced")).toBe(true);
  });

  it("assigns and unassigns a project skill through one toggle", async () => {
    const { handlers } = backendFor("rich");
    const pdf = (await handlers.getManagedSkills()).find((s) => s.name === "pdf")!;
    const on = await handlers.toggleProjectSkillAssignment("prj-quartermaster", pdf.id, "claude_code");
    expect(on.kind).toBe("assigned");
    if (on.kind === "assigned") expect(on.report.items[0].status).toEqual({ status: "synced" });
    const off = await handlers.toggleProjectSkillAssignment("prj-quartermaster", pdf.id, "claude_code");
    expect(off.kind).toBe("unassigned");
    if (off.kind === "unassigned") expect(off.report.targets[0].status).toEqual({ status: "removed" });
  });
});

describe("failures", () => {
  it("skips an uninstalled selected Tool and an unwritable shared dir", async () => {
    const { handlers } = backendFor("failures");
    const prisma = (await handlers.getManagedSkills()).find((s) => s.name === "prisma-review")!;
    const report = await handlers.syncSkillsToTools(
      [{ skill_id: prisma.id, name: prisma.name, source_path: prisma.central_path }],
      ["windsurf", "amp", "cursor", "claude_code"],
      { overwrite_if_same_content: true },
      sink().channel
    );
    const byTool = Object.fromEntries(report.map((r) => [r.tool_key, r.status]));
    expect(byTool.windsurf).toEqual({ status: "skipped", error: { code: "TOOL_NOT_INSTALLED", tool: "windsurf" } });
    expect(byTool.amp).toMatchObject({ status: "skipped", error: { code: "TOOL_NOT_WRITABLE" } });
    expect(byTool.cursor).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS" } });
    expect(byTool.claude_code).toMatchObject({ status: "synced" });
  });

  it("answers Refresh (all) with every per-skill outcome kind", async () => {
    const { handlers } = backendFor("failures");
    const report = await handlers.refreshManagedSkills(null, { reassert_auto_sync: true }, sink().channel );
    const kinds = new Set(report.skills.map((s) => s.status.status));
    expect(kinds).toEqual(new Set(["refreshed", "failed", "skipped", "skipped_acquisition"]));
    const codes = report.skills.flatMap((s) => (s.status.status === "failed" ? [s.status.error.code] : []));
    expect(codes).toEqual(expect.arrayContaining(["RATE_LIMITED", "GITHUB_SKILL_NOT_FOUND", "GIT_CLONE_FAILED", "SYMLINK_ESCAPES_REPO"]));
    const refreshed = report.skills.flatMap((s) => (s.status.status === "refreshed" ? [s.status] : []));
    expect(refreshed.some((s) => s.edit_conflict !== null)).toBe(true);
    expect(refreshed.some((s) => s.reassert_error !== null)).toBe(true);
    expect(refreshed.some((s) => s.targets.some((t) => t.status.status === "failed"))).toBe(true);
  });

  it("keeps a row whose artifact could not be removed (ADR-0002)", async () => {
    const { handlers } = backendFor("failures");
    const train = (await handlers.getManagedSkills()).find((s) => s.name === "release-train")!;
    const report = await handlers.unsyncSkillFromTool(train.id, "claude_code");
    expect(report.targets[0].status).toMatchObject({ status: "failed" });
    const after = (await handlers.getManagedSkills()).find((s) => s.id === train.id)!;
    expect(after.targets.find((t) => t.tool === "claude_code")?.status).toBe("error");
  });
});

describe("first-run and empty", () => {
  it("first-run reports every Tool newly detected once, with a plan to import", async () => {
    const { handlers } = backendFor("first-run");
    expect((await handlers.getSettings()).global_selected_tools).toBeNull();
    expect((await handlers.getToolStatus()).newly_installed).toHaveLength(DETECTED_TOOLS.length);
    expect((await handlers.getToolStatus()).newly_installed).toEqual([]);
    const plan = await handlers.getOnboardingPlan();
    expect(plan.groups.length).toBeGreaterThanOrEqual(10);
    expect(plan.groups.some((g) => g.has_conflict)).toBe(true);

    const selections = plan.groups.map((g) => ({ group_name: g.name, chosen_path: g.variants[0].path, name: null }));
    const report = await handlers.importOnboardingSelection(selections, { auto_sync: true, tools: null }, sink().channel );
    expect(report.groups.every((g) => g.status.status === "imported")).toBe(true);
    expect(await handlers.getManagedSkills()).toHaveLength(plan.groups.length);
  });

  it("empty has nothing managed and nothing to import", async () => {
    const { handlers, state } = backendFor("empty");
    const typed: FixtureState = state;
    expect(typed.skills).toEqual([]);
    expect(await handlers.listProjects()).toEqual([]);
    expect((await handlers.getOnboardingPlan()).groups).toEqual([]);
  });
});
