import type { Channel } from "@tauri-apps/api/core";
import { describe, expect, expectTypeOf, it } from "vitest";
import {
  commands,
  type ManagedSkillDto,
  type ProjectDto,
  type RefreshProgressDto,
  type SyncProgressDto,
  type ProjectViewDto,
  type SyncMode,
  type SyncStatus,
  type ToolStatusDto,
} from "../bindings";
import type { TranslateFn } from "../hooks/useStatusReporter";
import { refreshOutcome } from "../lib/reportOutcome";
import { skillSourceLabel } from "../lib/skillPresentation";
import type { CommandName } from "../lib/tauri";
import { createFixtureBackend, type FixtureBackend, type FixtureHandlers } from "./backend";
import { SCENARIOS, globalRoot, type FixtureProject, type FixtureState, type ScenarioName } from "./model";
import { TOOL_REGISTRY } from "./registry";
import { buildScenario, DETECTED_TOOLS } from "./scenarios";

const NOW = Date.UTC(2026, 8, 27, 12);

function backendFor(name: ScenarioName) {
  return createFixtureBackend(buildScenario(name, NOW), { latencyScale: 0, now: () => NOW });
}

/**
 * A backend whose simulated latency is an injected no-op sleep. `armCancel(n)`
 * makes the n-th sleep *after arming* request cancellation — a deterministic
 * stand-in for the operator pressing Cancel mid-operation.
 */
function cancellingBackend(name: ScenarioName) {
  let countdown = 0;
  const holder: { backend?: FixtureBackend } = {};
  holder.backend = createFixtureBackend(buildScenario(name, NOW), {
    latencyScale: 1,
    now: () => NOW,
    sleep: async () => {
      if (countdown > 0 && --countdown === 0) await holder.backend!.handlers.cancelCurrentOperation();
    },
  });
  const armCancel = (atSleep: number) => {
    countdown = atSleep;
  };
  return { ...holder.backend, armCancel };
}

const t: TranslateFn = (key) => key;

const byName = async (handlers: FixtureHandlers, name: string) =>
  (await handlers.getManagedSkills()).find((s) => s.name === name)!;

/** A progress sink recording every tick (stands in for a Tauri Channel). */
function sink<T>() {
  const ticks: T[] = [];
  const channel = { onmessage: (tick: T) => ticks.push(tick) } as unknown as Channel<T>;
  return { ticks, channel };
}

const SYNC_STATUSES: readonly SyncStatus[] = ["pending", "synced", "stale", "missing", "error", "disabled"];
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
    const { handlers, state } = backendFor(name);
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
        // A disabled row is the operator's record (per pair or per skill).
        expect(["synced", "error", "disabled"]).toContain(target.status);
        // A skill disabled at skill grain has no live global target.
        if (skill.disabled_at !== null) expect(target.status).not.toBe("synced");
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
    expectRecordedHashes(state);
  });
});

/**
 * S7 — content identity (Rust `sync_status::next_status`,
 * `project_sync::sync_assignment_target`): only a copy records a hash; a
 * synced copy recorded the current central hash, a stale copy a different one
 * (that difference is the drift); links and never-synced rows record none.
 */
function expectRecordedHashes(state: FixtureState) {
  for (const project of state.projects) {
    for (const a of project.assignments) {
      const current = state.skills.find((s) => s.dto.id === a.skill_id)!.contentHash;
      const where = `${project.id}/${a.skill_name}/${a.tool} (${a.mode}, ${a.status})`;
      if (a.mode !== "copy" || a.status === "pending") expect(a.content_hash, where).toBeNull();
      if (a.mode === "copy" && a.status === "synced") expect(a.content_hash, where).toBe(current);
      if (a.status === "stale") {
        expect(a.content_hash, where).not.toBeNull();
        expect(a.content_hash, where).not.toBe(current);
      }
    }
  }
}

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
    expect(first[0].status).toEqual({ status: "failed", error: { code: "TARGET_EXISTS", path: "/Users/alex/.cursor/skills/zod-schema-first", modifiedAt: expect.any(Number), sameContent: false } });
    expect(progress.ticks).toEqual([{ index: 1, total: 1, skill_name: "zod-schema-first", tool: "cursor" }]);

    const retry = await handlers.syncSkillsToTools(
      batch,
      ["cursor"],
      { overrides: [{ skill_id: zod.id, tool: "cursor", decision: "overwrite" }] },
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
    const { handlers, state } = backendFor("rich");
    const progress = sink<{ phase: string }>();
    const report = await handlers.refreshManagedSkills(null, { reassert_auto_sync: true }, progress.channel );
    expect(report.skills).toHaveLength(60);
    expect(new Set(progress.ticks.map((t) => t.phase))).toEqual(new Set(["acquiring", "applying"]));
    // Propagation leaves every project row content-identity consistent.
    expectRecordedHashes(state);
  });

  // S4 (replaces an assertion that errored links recover on Refresh).
  it("re-materialises errored copies on Update; links follow the source", async () => {
    const { handlers } = backendFor("rich");
    const worktrees = await byName(handlers, "using-git-worktrees");
    const { report } = await handlers.updateManagedSkill(worktrees.id, { reassert_auto_sync: false }, sink().channel);
    const status = report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    const global = Object.fromEntries(
      status.targets.flatMap((o) => (o.scope.scope === "global" ? [[o.scope.tool, o.status]] : [])),
    );
    // amp/kimi_cli are copies (one shared artifact): rewritten, recovered.
    expect(global.amp).toEqual({ status: "synced", mode_used: "symlink" });
    expect(global.kimi_cli).toEqual({ status: "synced", mode_used: "symlink" });
    // Synced links are skipped: the central copy was refreshed in place.
    expect(global.claude_code).toEqual({ status: "skipped", reason: { reason: "link_follows_source" } });
    const after = await byName(handlers, "using-git-worktrees");
    expect(after.targets.every((row) => row.status === "synced")).toBe(true);
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

  // S4: `needs_new_bytes` is mode-only — an errored link is skipped and
  // stays errored; an errored copy is retried and can fail again.
  it("leaves errored links alone and retries (and fails) errored copies", async () => {
    const { handlers } = backendFor("failures");
    const worktrees = await byName(handlers, "using-git-worktrees");
    const tdd = await byName(handlers, "tdd");
    const statusOf = async (id: string) => {
      const { report } = await handlers.updateManagedSkill(id, { reassert_auto_sync: false }, sink().channel);
      const status = report.skills[0].status;
      if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
      return Object.fromEntries(
        status.targets.flatMap((o) => (o.scope.scope === "global" ? [[o.scope.tool, o.status]] : [])),
      );
    };

    const links = await statusOf(worktrees.id);
    for (const tool of ["amp", "kimi_cli", "cursor"]) {
      expect(links[tool]).toEqual({ status: "skipped", reason: { reason: "link_follows_source" } });
    }
    const linkRows = (await byName(handlers, "using-git-worktrees")).targets;
    for (const tool of ["amp", "kimi_cli", "cursor"]) {
      expect(linkRows.find((r) => r.tool === tool)).toMatchObject({ mode: "symlink", status: "error" });
    }

    const copies = await statusOf(tdd.id);
    expect(copies.amp).toMatchObject({ status: "failed", error: { code: "OTHER" } });
    expect(copies.kimi_cli).toMatchObject({ status: "failed", error: { code: "OTHER" } });
    const copyRows = (await byName(handlers, "tdd")).targets;
    expect(copyRows.find((r) => r.tool === "amp")).toMatchObject({ mode: "copy", status: "error" });
  });

  it("keeps a row whose artifact could not be removed (ADR-0002)", async () => {
    const { handlers } = backendFor("failures");
    const train = (await handlers.getManagedSkills()).find((s) => s.name === "release-train")!;
    const report = await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: train.id, tool: "claude_code" });
    expect(report.targets[0].status).toMatchObject({ status: "failed" });
    const after = (await handlers.getManagedSkills()).find((s) => s.id === train.id)!;
    expect(after.targets.find((t) => t.tool === "claude_code")?.status).toBe("error");
  });
});

describe("Disabled: two grains, skipped by every automatic pass (round 19 D8)", () => {
  const batchOf = (skill: ManagedSkillDto) => [
    { skill_id: skill.id, name: skill.name, source_path: skill.central_path },
  ];
  const statusOf = (skill: ManagedSkillDto, tool: string) =>
    skill.targets.find((row) => row.tool === tool)?.status;

  it("failures holds a disabled row at each grain", async () => {
    const { handlers } = backendFor("failures");
    const noisy = await byName(handlers, "noisy-linter");
    expect(noisy.disabled_at).toBeNull();
    expect([statusOf(noisy, "codex"), statusOf(noisy, "cursor")]).toEqual(["disabled", "disabled"]);
    const retired = await byName(handlers, "retired-playbook");
    expect(retired.disabled_at).not.toBeNull();
    expect(retired.targets.map((row) => row.status)).toEqual(["disabled", "disabled"]);
    const view = await handlers.getProjectView("prj-quartermaster");
    expect(view.assignments.find((a) => a.skill_id === retired.id)?.status).toBe("disabled");
    // A disabled row takes no part in the project fold.
    expect(view.project.sync_status).toBe("synced");
  });

  it("Refresh with re-assert reports disabled targets skipped and never brings them back", async () => {
    const { handlers } = backendFor("failures");
    const noisy = await byName(handlers, "noisy-linter");
    const retired = await byName(handlers, "retired-playbook");
    const report = await handlers.refreshManagedSkills(
      [noisy.id, retired.id],
      { reassert_auto_sync: true },
      sink<RefreshProgressDto>().channel,
    );
    // Which global Tools each skill has disabled (pair grain) — every one of
    // them must come back `skipped: disabled`, never `failed`. Other targets
    // keep their own outcomes: noisy-linter's re-assert onto Amp still fails
    // on the scenario's unwritable shared dir, as the Rust re-assert reports it.
    const disabledTools: Record<string, string[]> = {
      [noisy.id]: ["codex", "cursor"],
      [retired.id]: retired.targets.map((row) => row.tool),
    };
    for (const outcome of report.skills) {
      expect(outcome.status.status).toBe("refreshed");
      if (outcome.status.status !== "refreshed") continue;
      const global = outcome.status.targets.flatMap((t) =>
        t.scope.scope === "global" ? [{ tool: t.scope.tool, status: t.status }] : [],
      );
      for (const tool of disabledTools[outcome.skill_id]) {
        expect(global.filter((t) => t.tool === tool).map((t) => t.status)).toEqual([
          { status: "skipped", reason: { reason: "disabled" } },
        ]);
      }
    }
    const noisyTargets = report.skills.find((s) => s.skill_id === noisy.id)!.status;
    if (noisyTargets.status === "refreshed")
      expect(
        noisyTargets.targets
          .filter((t) => t.status.status === "failed")
          .map((t) => (t.status.status === "failed" ? [t.scope.tool, t.status.error.code] : null)),
      ).toEqual([["amp", "TOOL_NOT_WRITABLE"]]);
    const retiredTargets = report.skills.find((s) => s.skill_id === retired.id)!.status;
    if (retiredTargets.status === "refreshed")
      expect(retiredTargets.targets.every((t) => t.status.status === "skipped")).toBe(true);
    const after = await byName(handlers, "noisy-linter");
    expect([statusOf(after, "codex"), statusOf(after, "cursor")]).toEqual(["disabled", "disabled"]);
    expect((await byName(handlers, "retired-playbook")).targets.every((t) => t.status === "disabled")).toBe(true);
  });

  it("a pair disables and enables; a pair with no row gets a marker the batch honours", async () => {
    const { handlers } = backendFor("failures");
    const zod = await byName(handlers, "zod-schema-first");
    const off = await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: zod.id, tool: "claude_code" });
    expect(off.targets.map((t) => t.status.status)).toEqual(["removed"]);
    expect(statusOf(await byName(handlers, "zod-schema-first"), "claude_code")).toBe("disabled");
    const on = await handlers.enableSkillTargets({ kind: "skill_tool", skill_id: zod.id, tool: "claude_code" }, []);
    expect(on.global.map((o) => o.status.status)).toEqual(["synced"]);
    expect(statusOf(await byName(handlers, "zod-schema-first"), "claude_code")).toBe("synced");

    const prisma = await byName(handlers, "prisma-review");
    const marker = await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: prisma.id, tool: "claude_code" });
    expect(marker.targets).toEqual([]);
    const marked = await byName(handlers, "prisma-review");
    expect(statusOf(marked, "claude_code")).toBe("disabled");
    const sync = await handlers.syncSkillsToTools(batchOf(marked), ["claude_code"], { overwrite_if_same_content: true }, sink<SyncProgressDto>().channel);
    expect(sync.map((o) => o.status)).toEqual([
      { status: "skipped", error: { code: "TARGET_DISABLED", tool: "claude_code" } },
    ]);
  });

  it("a disabled skill skips the batch and project re-sync; only skill-grain Enable restores it", async () => {
    const { handlers } = backendFor("failures");
    const retired = await byName(handlers, "retired-playbook");
    const sync = await handlers.syncSkillsToTools(batchOf(retired), ["pi"], { overwrite_if_same_content: true }, sink<SyncProgressDto>().channel);
    expect(sync[0].status).toEqual({ status: "skipped", error: { code: "TARGET_DISABLED", tool: "pi" } });
    const resync = await handlers.resyncProject("prj-quartermaster");
    expect(resync.report.items.find((i) => i.skill_id === retired.id)?.status).toEqual({
      status: "skipped",
      reason: { reason: "disabled" },
    });
    const pair = await handlers.enableSkillTargets({ kind: "skill_tool", skill_id: retired.id, tool: "claude_code" }, []);
    expect(pair.global[0].status).toEqual({ status: "skipped", error: { code: "TARGET_DISABLED", tool: "claude_code" } });

    const on = await handlers.enableSkillTargets({ kind: "skill", skill_id: retired.id }, []);
    expect(on.global.map((o) => [o.tool_key, o.status.status])).toEqual([
      ["claude_code", "synced"],
      ["codex", "synced"],
    ]);
    expect(on.projects.items.map((i) => i.status.status)).toEqual(["synced"]);
    const after = await byName(handlers, "retired-playbook");
    expect(after.disabled_at).toBeNull();
    expect(after.targets.every((t) => t.status === "synced")).toBe(true);
  });

  it("skill-grain Disable covers global rows and project assignments; a new assignment is recorded disabled", async () => {
    const { handlers } = backendFor("failures");
    const tdd = await byName(handlers, "tdd");
    const report = await handlers.disableSkillTargets({ kind: "skill", skill_id: tdd.id });
    expect(report.record_deleted).toBe(false);
    const after = await byName(handlers, "tdd");
    expect(after.disabled_at).not.toBeNull();
    // amp/kimi_cli sit in the unwritable shared dir: kept `error` (ADR-0002).
    expect(statusOf(after, "claude_code")).toBe("disabled");
    expect(statusOf(after, "amp")).toBe("error");
    const quartermaster = await handlers.getProjectView("prj-quartermaster");
    expect(quartermaster.assignments.find((a) => a.skill_id === tdd.id)?.status).toBe("disabled");

    const { report: assigned } = await handlers.bulkAssignSkill("prj-quartermaster", tdd.id);
    expect(assigned.items.map((i) => i.status)).toEqual([
      { status: "skipped", reason: { reason: "disabled" } },
      { status: "skipped", reason: { reason: "disabled" } },
    ]);
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

    // Two managed-name groups and one invalid manifest (the review's cases).
    const managed = plan.groups.filter((g) => g.name_is_managed).map((g) => g.name);
    expect(managed).toEqual(["frontend-design", "tdd"]);
    expect(plan.invalid).toEqual([
      { tool: "cursor", name: "broken-skill", path: "/Users/alex/.cursor/skills/broken-skill", reason: "invalid_frontmatter" },
    ]);
    expect(plan.groups.some((g) => g.name === "broken-skill")).toBe(false);
    expect(plan.group_count).toBe(plan.groups.length);
    expect(plan.copy_count).toBe(plan.groups.reduce((n, g) => n + g.variants.length, 0));
    expect(plan.copy_count).toBeGreaterThan(plan.group_count);

    const before = (await handlers.getManagedSkills()).length;
    const selections = plan.groups.map((g) => ({ group_name: g.name, chosen_path: g.variants[0].path, name: null }));
    const report = await handlers.importOnboardingSelection(selections, { auto_sync: true, tools: null, remove_originals: true }, sink().channel );
    const byStatus = (s: string) => report.groups.filter((g) => g.status.status === s).map((g) => g.group_name);
    expect(byStatus("skipped")).toEqual(managed);
    expect(byStatus("imported")).toHaveLength(plan.groups.length - managed.length);
    expect(await handlers.getManagedSkills()).toHaveLength(before + plan.groups.length - managed.length);
  });

  it("first-run: Replace syncs the managed skill over identical copies and asks for divergent ones", async () => {
    const { handlers } = backendFor("first-run");
    const plan = await handlers.getOnboardingPlan();
    const replace = (name: string) => {
      const group = plan.groups.find((g) => g.name === name)!;
      return { group_name: name, chosen_path: group.variants[0].path, name: null, replace: true };
    };
    const report = await handlers.importOnboardingSelection(
      [replace("frontend-design"), replace("tdd")],
      { auto_sync: false, tools: null, remove_originals: false },
      sink().channel,
    );
    const [identical, divergent] = report.groups.map((g) => g.status);
    if (identical.status !== "replaced" || divergent.status !== "replaced") throw new Error("expected replaced");
    const byTool = (targets: typeof identical.targets) =>
      [...targets].sort((a, b) => a.tool_key.localeCompare(b.tool_key));
    expect(byTool(identical.targets).map((o) => [o.tool_key, o.status.status])).toEqual([
      ["claude_code", "synced"],
      ["cursor", "synced"],
    ]);
    // Askable: TARGET_EXISTS rows naming (skill, Tool), never overwritten here.
    expect(byTool(divergent.targets).map((o) => o.status)).toEqual(
      ["claude_code", "codex", "pi"].map((tool) => ({
        status: "failed",
        error: { code: "TARGET_EXISTS", path: `${globalRoot(tool)}/tdd`, modifiedAt: expect.any(Number), sameContent: false },
      })),
    );
    expect(divergent.targets.every((o) => o.skill_id === divergent.skill_id)).toBe(true);
    const after = await handlers.getOnboardingPlan();
    expect(after.groups.some((g) => g.name === "frontend-design")).toBe(false);
    expect(after.groups.find((g) => g.name === "tdd")?.variants).toHaveLength(3);
  });

  it("remove_originals: auto-sync off leaves the originals when false, removes them when true", async () => {
    for (const remove of [false, true]) {
      const { handlers } = backendFor("first-run");
      const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "systematic-debugging")!;
      const report = await handlers.importOnboardingSelection(
        [{ group_name: group.name, chosen_path: group.variants[0].path, name: null }],
        { auto_sync: false, tools: null, remove_originals: remove },
        sink().channel,
      );
      const status = report.groups[0].status;
      if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
      expect(status.originals.map((o) => [o.tool, o.status.status])).toEqual([
        ["claude_code", remove ? "removed" : "left_in_place"],
        ["pi", remove ? "removed" : "left_in_place"],
      ]);
      const after = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === group.name);
      expect(after?.variants.length ?? 0).toBe(remove ? 0 : 2);
    }
  });

  it("empty has nothing managed and nothing to import", async () => {
    const { handlers, state } = backendFor("empty");
    const typed: FixtureState = state;
    expect(typed.skills).toEqual([]);
    expect(await handlers.listProjects()).toEqual([]);
    expect((await handlers.getOnboardingPlan()).groups).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Review regressions (ticket-04 review S1–S7, N1)
// ---------------------------------------------------------------------------

describe("S1: the auto-sync re-assert is report data", () => {
  it("reports an occupied target as TARGET_EXISTS and folds to non-success", async () => {
    const { handlers } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const { report } = await handlers.updateManagedSkill(zod.id, { reassert_auto_sync: true }, sink().channel);
    const status = report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    expect(status.targets).toContainEqual({
      scope: { scope: "global", tool: "cursor" },
      status: { status: "failed", error: { code: "TARGET_EXISTS", path: "/Users/alex/.cursor/skills/zod-schema-first", modifiedAt: expect.any(Number), sameContent: false } },
    });
    const fold = refreshOutcome(report, { t, single: { name: zod.name, success: "UPDATED" } });
    expect(fold.toast?.kind).not.toBe("success");
    // Folded without an answer (the ask is the door's): "to review", never an error (D10).
    expect(fold.toast?.tone).toBe("conflict");
    expect(fold.warnings.some((entry) => entry.review)).toBe(true);
    expect((await byName(handlers, "zod-schema-first")).targets.some((row) => row.tool === "cursor")).toBe(false);
  });

  it("skips a selected-but-uninstalled Tool and fails an unwritable shared dir once", async () => {
    const { handlers } = backendFor("failures");
    const prisma = await byName(handlers, "prisma-review");
    const report = await handlers.refreshManagedSkills([prisma.id], { reassert_auto_sync: true }, sink().channel);
    const status = report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    const byTool = Object.fromEntries(status.targets.map((o) => [o.scope.tool, o.status]));
    expect(byTool.windsurf).toEqual({ status: "skipped", reason: { reason: "tool_not_installed", tool: "windsurf" } });
    // Unwritable is a *skip* in the sync batch, a *failure* in the re-assert.
    expect(byTool.amp).toMatchObject({ status: "failed", error: { code: "TOOL_NOT_WRITABLE" } });
    expect(status.targets.some((o) => o.scope.tool === "kimi_cli")).toBe(false);
    expect(byTool.cursor).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS" } });
    expect(byTool.claude_code).toEqual({ status: "synced", mode_used: "symlink" });
  });

  it("merges re-assert outcomes into Re-point's report too", async () => {
    const { handlers } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const { report } = await handlers.repointSkillSource(
      zod.id,
      { kind: "git", url: "https://github.com/mattpocock/skills/tree/main/skills/engineering/zod-schema-first" },
      { reassert_auto_sync: true },
    );
    const status = report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    expect(status.targets).toContainEqual(
      expect.objectContaining({ scope: { scope: "global", tool: "cursor" }, status: expect.objectContaining({ status: "failed" }) }),
    );
  });
});

describe("S2/S3: onboarding import goes through the global sync batch", () => {
  it("keeps an original whose takeover failed; replaces the one taken over", async () => {
    const { handlers } = backendFor("failures");
    const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "commit-message")!;
    const codex = group.variants.find((v) => v.tool === "codex")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: group.name, chosen_path: codex.path, name: null }],
      { auto_sync: true, tools: ["codex"], remove_originals: true },
      sink().channel,
    );
    const status = report.groups[0].status;
    if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
    expect(status.forced_tools).toEqual(["amp"]);
    const byTool = Object.fromEntries(status.targets.map((o) => [o.tool_key, o.status]));
    expect(byTool.codex).toMatchObject({ status: "synced", outcome: { replaced: true } });
    expect(byTool.amp).toMatchObject({ status: "skipped", error: { code: "TOOL_NOT_WRITABLE" } });
    // The unwritable Amp original survives and stays discoverable.
    const after = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "commit-message");
    expect(after?.variants.map((v) => v.tool)).toEqual(["amp"]);
  });

  it("reports a divergent sibling in a target's way once, as its TARGET_EXISTS row (ticket 04 review F4)", async () => {
    for (const synced of [["codex", "gemini_cli"], ["codex"]]) {
      const { handlers } = backendFor("first-run");
      const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "commit-message")!;
      const codex = group.variants.find((v) => v.tool === "codex")!;
      const gemini = group.variants.find((v) => v.tool === "gemini_cli")!;
      const report = await handlers.importOnboardingSelection(
        [{ group_name: group.name, chosen_path: codex.path, name: null }],
        { auto_sync: true, tools: synced, remove_originals: true },
        sink().channel,
      );
      const status = report.groups[0].status;
      if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
      const collided = status.targets.filter(
        (o) => o.status.status === "failed" && o.status.error.code === "TARGET_EXISTS",
      );
      if (synced.includes("gemini_cli")) {
        // In the sync target set: the askable row alone, not also a kept original.
        expect(collided.map((o) => o.tool_key)).toEqual(["gemini_cli"]);
        expect(status.originals).toEqual([]);
      } else {
        // Not a target: the divergent copy stays a legitimate warning.
        expect(collided).toEqual([]);
        expect(status.originals).toEqual([
          { path: gemini.path, tool: "gemini_cli", status: { status: "kept_divergent" } },
        ]);
      }
    }
  });

  it("honours a saved non-empty selection when the policy names no Tools", async () => {
    const { handlers } = backendFor("first-run");
    await handlers.updateSetting({ key: "global_tool_config", value: { selected_tools: ["claude_code"], scan_selected_only: false } });
    const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "pdf")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: "pdf", chosen_path: group.variants[0].path, name: null }],
      { auto_sync: true, tools: null, remove_originals: true },
      sink().channel,
    );
    const status = report.groups[0].status;
    if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
    expect(status.targets.map((o) => o.tool_key)).toEqual(["claude_code"]);
    expect(status.forced_tools).toEqual([]);
  });

  it("honours a saved empty selection: only identical originals' Tools are synced", async () => {
    const { handlers } = backendFor("first-run");
    await handlers.updateSetting({ key: "global_tool_config", value: { selected_tools: [], scan_selected_only: false } });
    const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "web-design-guidelines")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: group.name, chosen_path: group.variants[0].path, name: null }],
      { auto_sync: true, tools: null, remove_originals: true },
      sink().channel,
    );
    const status = report.groups[0].status;
    if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
    expect(status.forced_tools.sort()).toEqual(["codex", "cursor"]);
    expect(status.targets.map((o) => o.tool_key).sort()).toEqual(["codex", "cursor"]);
  });

  it("makes one physical attempt (one report row) per shared skills dir", async () => {
    const { handlers } = backendFor("first-run");
    const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "ego-browser")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: "ego-browser", chosen_path: group.variants[0].path, name: null }],
      { auto_sync: true, tools: null, remove_originals: true },
      sink().channel,
    );
    const status = report.groups[0].status;
    if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
    const tools = status.targets.map((o) => o.tool_key);
    expect(tools).toHaveLength(DETECTED_TOOLS.length - 1);
    expect(tools).toContain("amp");
    expect(tools).not.toContain("kimi_cli");
  });
});

describe("S5: cancellation", () => {
  it("resets at operation entry, so an earlier cancel does not leak", async () => {
    const { handlers } = backendFor("rich");
    await handlers.cancelCurrentOperation();
    const report = await handlers.refreshManagedSkills(null, { reassert_auto_sync: false }, sink().channel);
    expect(report.skills.every((s) => s.status.status === "refreshed")).toBe(true);
  });

  it("an early cancel answers per-skill cancelled rows, pre-settled skips last, nothing applied", async () => {
    const { handlers, armCancel } = cancellingBackend("failures");
    const before = await handlers.getManagedSkills();
    armCancel(1);
    const progress = sink<{ phase: string }>();
    const report = await handlers.refreshManagedSkills(null, { reassert_auto_sync: true }, progress.channel);
    const kinds = report.skills.map((s) => s.status.status);
    const firstSkip = kinds.indexOf("skipped");
    expect(firstSkip).toBeGreaterThan(0);
    expect(kinds.slice(0, firstSkip).every((k) => k === "cancelled")).toBe(true);
    expect(kinds.slice(firstSkip).every((k) => k === "skipped")).toBe(true);
    for (const s of report.skills.slice(0, firstSkip)) {
      expect(s.status).toEqual({ status: "cancelled" });
    }
    // D10: the fold reads it as a cancel, never "0 refreshed, N failed".
    expect(refreshOutcome(report, { t }).toast).toMatchObject({ tone: "neutral", message: "outcome.refreshCancelled" });
    expect(progress.ticks).toEqual([]);
    expect(await handlers.getManagedSkills()).toEqual(before);
  });

  it("a cancel mid-acquisition stops dispatch and applies nothing", async () => {
    // Sleeps: 1 = entry pause, 2.. = one per acquisition.
    const { handlers, armCancel } = cancellingBackend("rich");
    const before = await handlers.getManagedSkills();
    armCancel(3);
    const progress = sink<{ phase: string }>();
    const report = await handlers.refreshManagedSkills(null, { reassert_auto_sync: true }, progress.channel);
    expect(progress.ticks.map((tick) => tick.phase)).toEqual(["acquiring", "acquiring"]);
    expect(report.skills).toHaveLength(60);
    expect(report.skills.every((s) => s.status.status === "cancelled")).toBe(true);
    expect(await handlers.getManagedSkills()).toEqual(before);
  });

  it("Update and Re-point answer a cancelled report row and leave the skill unchanged", async () => {
    const update = cancellingBackend("rich");
    const tdd = await byName(update.handlers, "tdd");
    update.armCancel(2);
    const updated = await update.handlers.updateManagedSkill(tdd.id, { reassert_auto_sync: true }, sink().channel);
    expect(updated.report.skills[0].status).toEqual({ status: "cancelled" });
    expect(updated.skills.find((s) => s.id === tdd.id)).toEqual(tdd);

    const repoint = cancellingBackend("rich");
    const pdf = await byName(repoint.handlers, "pdf");
    repoint.armCancel(1);
    const repointed = await repoint.handlers.repointSkillSource(pdf.id, { kind: "local", path: "~/Projects/pi-lens/skill" }, { reassert_auto_sync: false });
    expect(repointed.report.skills[0].status).toEqual({ status: "cancelled" });
    expect(repointed.skills.find((s) => s.id === pdf.id)).toEqual(pdf);
  });

  it("Git install and Explore clone reject with the typed CANCELLED", async () => {
    const install = cancellingBackend("rich");
    install.armCancel(1);
    await expect(
      install.handlers.installGitSelection("https://github.com/acme/changelog-skill", "./changelog-writer", null, null),
    ).rejects.toEqual({ code: "CANCELLED" });
    expect(await install.handlers.getManagedSkills()).toHaveLength(60);

    const explore = cancellingBackend("rich");
    explore.armCancel(1);
    await expect(explore.handlers.cloneExploreSkill("https://github.com/acme/changelog-skill", null)).rejects.toEqual({ code: "CANCELLED" });
  });
});

describe("S6: Git Add honours the operator's name", () => {
  it("finalizes under the explicit name, and collides on it", async () => {
    const { handlers } = backendFor("rich");
    const url = "https://github.com/acme/changelog-skill";
    const installed = await handlers.installGitSelection(url, "./changelog-writer", "my-custom-name", null);
    expect(installed.name).toBe("my-custom-name");
    expect((await handlers.getManagedSkills()).some((s) => s.name === "my-custom-name")).toBe(true);
    await expect(handlers.installGitSelection(url, "./changelog-writer", "My-Custom-Name", null)).rejects.toEqual({
      code: "SKILL_EXISTS",
      name: "My-Custom-Name",
    });
    expect((await handlers.installGitSelection(url, "./changelog-writer", null, null)).name).toBe("changelog-writer");
  });
});

describe("N1: progress ticks only the work that reached each phase", () => {
  it("Refresh: no applying tick for a failed acquisition", async () => {
    const { handlers } = backendFor("failures");
    const progress = sink<{ phase: string; skill_name: string }>();
    const report = await handlers.refreshManagedSkills(null, { reassert_auto_sync: true }, progress.channel);
    const applying = progress.ticks.filter((tick) => tick.phase === "applying").map((tick) => tick.skill_name);
    const reachedApply = report.skills
      .filter((s) => s.status.status === "refreshed" || s.status.status === "skipped_acquisition")
      .map((s) => s.skill_name);
    expect(applying).toEqual(reachedApply);
    const acquiring = progress.ticks.filter((tick) => tick.phase === "acquiring");
    expect(acquiring).toHaveLength(report.skills.filter((s) => s.status.status !== "skipped").length);
    expect(applying).not.toContain("pr-review");
  });

  it("Import: no applying tick for a refused admission or a managed-name skip; apply failures still tick", async () => {
    const { handlers } = backendFor("failures");
    const plan = await handlers.getOnboardingPlan();
    expect(plan.invalid.map((i) => i.name)).toEqual(["broken-skill"]);
    const pick = (name: string) => ({ group_name: name, chosen_path: plan.groups.find((g) => g.name === name)!.variants[0].path, name: null });
    const tddId = plan.groups.find((g) => g.name === "tdd")!.managed_skill_id;
    const progress = sink<{ phase: string; group_name: string }>();
    const report = await handlers.importOnboardingSelection(
      [
        pick("tdd"),
        { group_name: "vanished", chosen_path: "/nowhere", name: null },
        { ...pick("sql-review"), name: "tdd" },
        pick("commit-message"),
      ],
      { auto_sync: false, tools: null, remove_originals: true },
      progress.channel,
    );
    expect(progress.ticks.map((tick) => `${tick.group_name}:${tick.phase}`)).toEqual([
      "tdd:admitting",
      "vanished:admitting",
      "sql-review:admitting",
      "sql-review:applying",
      "commit-message:admitting",
      "commit-message:applying",
    ]);
    expect(report.groups.map((g) => g.status)).toEqual([
      { status: "skipped", reason: { reason: "name_managed", skill_id: tddId } },
      { status: "failed", error: { code: "NOT_FOUND", kind: "onboarding_group", id: "vanished" } },
      { status: "failed", error: { code: "SKILL_EXISTS", name: "tdd" } },
      expect.objectContaining({ status: "imported" }),
    ]);
  });
});

// ---------------------------------------------------------------------------
// Flow-review fixture gaps (ticket-07 flows.md § Fixture gaps: F2, F4)
// ---------------------------------------------------------------------------

describe("F2: unwritable dirs refuse removal too (ADR-0002)", () => {
  it("disable from a shared unwritable dir: one failed target, both rows kept as error", async () => {
    const { handlers } = backendFor("failures");
    const docx = await byName(handlers, "docx");
    const report = await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: docx.id, tool: "amp" });
    expect(report.targets).toHaveLength(1);
    expect(report.targets[0].rows.map((r) => r.tool).sort()).toEqual(["amp", "kimi_cli"]);
    expect(report.targets[0].status).toMatchObject({ status: "failed", error: { code: "OTHER" } });
    const rows = (await byName(handlers, "docx")).targets.filter((r) => r.tool === "amp" || r.tool === "kimi_cli");
    expect(rows.map((r) => r.status)).toEqual(["error", "error"]);
  });

  it("remove_project keeps the project and the row whose artifact stayed", async () => {
    const { handlers } = backendFor("failures");
    const { projects, report } = await handlers.removeProject("prj-monorepo");
    const failed = report.targets.filter((target) => target.status.status === "failed");
    expect(failed).toHaveLength(1);
    expect(failed[0].path).toBe("/Users/alex/Projects/monorepo/.agents/skills/release-train");
    // Deployed rows whose artifact is gone (missing) are removed trivially.
    expect(report.targets.filter((target) => target.status.status === "removed").length).toBeGreaterThan(0);
    expect(projects.some((p) => p.id === "prj-monorepo")).toBe(true);
    const view = await handlers.getProjectView("prj-monorepo");
    // Never-deployed rows (pending, a failed first sync's error: synced_at
    // null) own nothing, so no path is planned for them; the scope failed, so
    // they are not cleared either and keep their status and diagnostic.
    expect(report.cleared_markers).toEqual([]);
    expect(report.targets.map((t) => t.path)).not.toContain("/Users/alex/Projects/monorepo/.agents/skills/tdd");
    expect(view.assignments).toEqual([
      expect.objectContaining({ skill_name: "tdd", tool: "agents_skills", status: "error", synced_at: null, last_error: expect.stringContaining("Permission denied") }),
      expect.objectContaining({ skill_name: "release-train", tool: "claude_code", status: "pending", synced_at: null }),
      expect.objectContaining({ skill_name: "release-train", tool: "agents_skills", status: "error" }),
    ]);
  });
});

// ---------------------------------------------------------------------------
// Ticket-01 review (2026-09-29): Rust parity for F1–F4
// ---------------------------------------------------------------------------

describe("ticket-01 review: ownership, partial Disable, artifact presence", () => {
  const fixture = () => {
    const state = buildScenario("failures", NOW);
    const { handlers } = createFixtureBackend(state, { latencyScale: 0, now: () => NOW });
    const tdd = state.skills.find((s) => s.dto.name === "tdd")!;
    const quartermaster = state.projects.find((p) => p.id === "prj-quartermaster")!;
    const tddAssignment = () => quartermaster.assignments.find((a) => a.skill_id === tdd.dto.id)!;
    return { state, handlers, tdd, quartermaster, tddAssignment };
  };
  const skip = { status: "skipped", reason: { reason: "disabled" } };

  it("F2: a failed skill Disable keeps the row error; resync (one, all) and bulk assign skip it", async () => {
    const { state, handlers, tdd, quartermaster, tddAssignment } = fixture();
    state.unwritableProjectTools.push(`${quartermaster.id}:claude_code`);
    await handlers.disableSkillTargets({ kind: "skill", skill_id: tdd.dto.id });
    expect(tdd.dto.disabled_at).not.toBeNull();
    expect(tddAssignment().status).toBe("error");
    const failure = tddAssignment().last_error;
    state.unwritableProjectTools = [];

    const one = await handlers.resyncProject(quartermaster.id);
    const all = await handlers.resyncAllProjects();
    for (const report of [one.report, all.report])
      expect(report.items.find((i) => i.skill_id === tdd.dto.id)?.status).toEqual(skip);
    const { report: assigned } = await handlers.bulkAssignSkill(quartermaster.id, tdd.dto.id);
    expect(assigned.items.find((i) => i.tool === "claude_code")?.status).toEqual(skip);
    expect(tddAssignment().status).toBe("error");
    expect(tddAssignment().last_error).toBe(failure);
  });

  it("F3: a failed shared-dir removal mints no disabled sibling; the retry settles both", async () => {
    const { state, handlers } = fixture();
    const docx = state.skills.find((s) => s.dto.name === "docx")!;
    docx.dto.targets = docx.dto.targets.filter((t) => t.tool !== "amp");
    const failed = await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: docx.dto.id, tool: "amp" });
    expect(failed.targets.map((t) => t.status.status)).toEqual(["failed"]);
    const shared = () => docx.dto.targets.filter((t) => t.tool === "amp" || t.tool === "kimi_cli").map((t) => [t.tool, t.status]);
    expect(shared()).toEqual([["kimi_cli", "error"]]);

    state.unwritableTools = state.unwritableTools.filter((tool) => tool !== "amp" && tool !== "kimi_cli");
    const retried = await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: docx.dto.id, tool: "amp" });
    expect(retried.targets.map((t) => t.status.status)).toEqual(["removed"]);
    expect(shared()).toEqual([["kimi_cli", "disabled"], ["amp", "disabled"]]);
  });

  it("F4: presence is independent of status — a repeated Disable, then Delete, on a locked path keeps failing", async () => {
    const { state, handlers, tdd, quartermaster, tddAssignment } = fixture();
    state.unwritableProjectTools.push(`${quartermaster.id}:claude_code`);
    for (let attempt = 0; attempt < 2; attempt++) {
      const report = await handlers.disableSkillTargets({ kind: "skill", skill_id: tdd.dto.id });
      const assignment = report.targets.filter((t) => t.rows.some((r) => r.scope === "assignment"));
      expect(assignment.map((t) => t.status.status)).toContain("failed");
      expect(tddAssignment().status).toBe("error");
    }
    const deleted = await handlers.deleteManagedSkill(tdd.dto.id);
    expect(deleted.record_deleted).toBe(false);
    expect(state.skills).toContain(tdd);

    state.unwritableProjectTools = [];
    const retry = await handlers.disableSkillTargets({ kind: "skill", skill_id: tdd.dto.id });
    // The shared global dir stays unwritable in this scenario; the project
    // artifact, no longer locked, is removed at last.
    const assignmentTargets = retry.targets.filter((t) => t.rows.some((r) => r.scope === "assignment"));
    expect(assignmentTargets.map((t) => t.status.status)).toEqual(["removed"]);
    expect(tddAssignment().status).toBe("disabled");
  });

  it("F1: an intent-only marker gives Delete no path; the row is cleared, the occupant kept", async () => {
    const { state, handlers } = fixture();
    const prisma = state.skills.find((s) => s.dto.name === "prisma-review")!;
    expect(prisma.dto.targets.some((t) => t.tool === "claude_code")).toBe(false);
    await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: prisma.dto.id, tool: "claude_code" });
    state.foreign.push({ tool: "claude_code", name: prisma.dto.name, fingerprint: "someone-else", linkTarget: null });
    const occupants = state.foreign.length;

    const report = await handlers.deleteManagedSkill(prisma.dto.id);

    expect(report.record_deleted).toBe(true);
    expect(report.targets.map((t) => t.path)).not.toContain(`${globalRoot("claude_code")}/${prisma.dto.name}`);
    expect(report.cleared_markers).toEqual([expect.objectContaining({ scope: "global_target", tool: "claude_code" })]);
    expect(state.foreign).toHaveLength(occupants);
  });

  it("F1-R1: a failed project Enable leaves the row error with no owner; no removal or re-sync takes the folder", async () => {
    // Typed like the global batch's collision (ticket 02).
    const collision = {
      status: "failed",
      error: { code: "TARGET_EXISTS", path: "/Users/alex/Projects/fresh/.claude/skills/prisma-review", modifiedAt: expect.any(Number), sameContent: false },
    };
    type Act = (
      handlers: FixtureHandlers,
      ids: { skill: string; project: string },
    ) => Promise<unknown>;
    const acts: [string, Act][] = [
      ["bulk unassign", async (h, ids) => {
        const { report } = await h.bulkUnassignSkill(ids.project, ids.skill);
        expect(report.targets).toEqual([]);
        expect(report.cleared_markers).toEqual([expect.objectContaining({ scope: "assignment", tool: "claude_code" })]);
      }],
      ["toggle-off", async (h, ids) => {
        const toggled = await h.toggleProjectSkillAssignment(ids.project, ids.skill, "claude_code");
        expect(toggled).toMatchObject({ kind: "unassigned", report: { targets: [], cleared_markers: [expect.anything()] } });
      }],
      ["remove project", async (h, ids) => {
        const { report } = await h.removeProject(ids.project);
        expect(report.targets.map((t) => t.path)).not.toContain(occupied);
        expect(report.cleared_markers).toEqual([expect.objectContaining({ tool: "claude_code" })]);
      }],
      ["Disable again, then Enable retries", async (h, ids) => {
        const report = await h.disableSkillTargets({ kind: "skill", skill_id: ids.skill });
        expect(report.targets.map((t) => t.path)).not.toContain(occupied);
        const row = (await h.getProjectView(ids.project)).assignments.find((a) => a.skill_id === ids.skill)!;
        expect(row).toMatchObject({ status: "disabled", synced_at: null });
        const enabled = await h.enableSkillTargets({ kind: "skill", skill_id: ids.skill }, []);
        expect(enabled.projects.items.map((i) => i.status)).toEqual([collision]);
      }],
      ["Delete", async (h, ids) => {
        const report = await h.deleteManagedSkill(ids.skill);
        expect(report.record_deleted).toBe(true);
        expect(report.targets.map((t) => t.path)).not.toContain(occupied);
        expect(report.cleared_markers).toEqual([expect.objectContaining({ scope: "assignment" })]);
      }],
      ["resync one", async (h, ids) => {
        const { report } = await h.resyncProject(ids.project);
        expect(report.items.find((i) => i.skill_id === ids.skill)?.status).toEqual(collision);
      }],
      ["resync all", async (h, ids) => {
        const { report } = await h.resyncAllProjects();
        expect(report.items.find((i) => i.skill_id === ids.skill)?.status).toEqual(collision);
      }],
    ];
    const occupied = "/Users/alex/Projects/fresh/.claude/skills/prisma-review";
    for (const [label, act] of acts) {
      const { state, handlers } = fixture();
      const prisma = state.skills.find((s) => s.dto.name === "prisma-review")!;
      await handlers.disableSkillTargets({ kind: "skill", skill_id: prisma.dto.id });
      const project: FixtureProject = {
        id: "prj-fresh", name: "fresh", path: "/Users/alex/Projects/fresh", created_at: NOW, updated_at: NOW,
        path_exists: true, tools: ["claude_code"], gitignore: { in_gitignore: false, in_exclude: false }, assignments: [],
      };
      state.projects.push(project);
      const assigned = await handlers.bulkAssignSkill(project.id, prisma.dto.id);
      expect(assigned.report.items.map((i) => i.status), label).toEqual([skip]);
      state.projectOccupants.push(occupied);

      const enabled = await handlers.enableSkillTargets({ kind: "skill", skill_id: prisma.dto.id }, []);

      expect(enabled.projects.items.map((i) => i.status), label).toEqual([collision]);
      expect(project.assignments[0], label).toMatchObject({ status: "error", synced_at: null, last_error: expect.stringContaining("already exists") });

      await act(handlers, { skill: prisma.dto.id, project: project.id });

      expect(state.projectOccupants, `${label}: the foreign folder survives`).toContain(occupied);
    }
  });
});

describe("F4: one source of truth for invocation-Edit conflicts", () => {
  it("the report's upstream becomes the card's base; the flag persists until convergence", async () => {
    const { handlers } = backendFor("failures");
    const tokens = await byName(handlers, "design-tokens");
    expect(tokens.invocation_override).toEqual({ mode: "user-only", base_mode: "user-and-model", conflict: false });

    const first = await handlers.updateManagedSkill(tokens.id, { reassert_auto_sync: false }, sink().channel);
    const status = first.report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    expect(status.edit_conflict).toEqual({ base_mode: "user-and-model", upstream_mode: "model-only", override_mode: "user-only" });
    const card = first.skills.find((s) => s.id === tokens.id)!;
    expect(card.invocation_override).toEqual({ mode: "user-only", base_mode: status.edit_conflict!.upstream_mode, conflict: true });
    expect(card.invocation_mode).toBe("user-only");

    // An unchanged Update reports nothing new; the conflict stays flagged.
    const second = await handlers.updateManagedSkill(tokens.id, { reassert_auto_sync: false }, sink().channel);
    const again = second.report.skills[0].status;
    if (again.status !== "refreshed") throw new Error(`expected refreshed, got ${again.status}`);
    expect(again.edit_conflict).toBeNull();
    expect(second.skills.find((s) => s.id === tokens.id)!.invocation_override).toEqual(card.invocation_override);
  });
});

describe("D9: Skill health and the failed / declined first sync (ticket 02)", () => {
  const KINDS = ["fetch_failed", "edit_conflict", "delete_incomplete", "repoint_failed", "restore_failed", "import_failed"];

  it("the failures scenario seeds every outcome kind, each with a typed error", async () => {
    const { handlers } = backendFor("failures");
    const skills = await handlers.getManagedSkills();
    const seeded = skills.flatMap((s) => (s.health ? [s.health.kind] : []));
    expect([...new Set(seeded)].sort()).toEqual([...KINDS].sort());
    for (const s of skills) {
      if (!s.health) continue;
      expect(typeof s.health.error.code).toBe("string");
      expect(s.health.at).toBeLessThanOrEqual(NOW);
    }
    // A failed first sync's rows own nothing.
    const canvas = await byName(handlers, "json-canvas");
    expect(canvas.targets.filter((t) => t.status === "error")).toEqual([
      expect.objectContaining({ tool: "amp", synced_at: null }),
      expect.objectContaining({ tool: "kimi_cli", synced_at: null }),
    ]);
  });

  it("a failed Update records fetch_failed with the row's error; the next success clears it", async () => {
    const { handlers, state } = backendFor("failures");
    const pr = state.skills.find((s) => s.dto.name === "pr-review")!;
    pr.dto.health = null;
    const failed = await handlers.updateManagedSkill(pr.dto.id, { reassert_auto_sync: false }, sink().channel);
    const row = failed.report.skills[0].status;
    if (row.status !== "failed") throw new Error(row.status);
    const card = failed.skills.find((s) => s.id === pr.dto.id)!;
    expect(card.health).toEqual({ kind: "fetch_failed", error: row.error, at: NOW });

    pr.acquisition.fail = undefined;
    const ok = await handlers.updateManagedSkill(pr.dto.id, { reassert_auto_sync: false }, sink().channel);
    expect(ok.skills.find((s) => s.id === pr.dto.id)!.health).toBeNull();
  });

  it("a failed Restore records restore_failed; a cancelled Refresh records nothing", async () => {
    const { handlers, state } = backendFor("failures");
    const scratch = state.skills.find((s) => s.dto.name === "scratchpad")!;
    scratch.dto.health = null;
    const { skills } = await handlers.updateManagedSkill(scratch.dto.id, { reassert_auto_sync: false }, sink().channel);
    expect(skills.find((s) => s.id === scratch.dto.id)!.health).toMatchObject({
      kind: "restore_failed",
      error: { code: "SOURCE_PATH_MISSING" },
    });

    const cancelling = cancellingBackend("failures");
    const pr = cancelling.state.skills.find((s) => s.dto.name === "pr-review")!;
    pr.dto.health = null;
    cancelling.armCancel(1);
    const report = await cancelling.handlers.refreshManagedSkills([pr.dto.id], { reassert_auto_sync: false }, sink().channel);
    expect(report.skills[0].status).toEqual({ status: "cancelled" });
    expect(pr.dto.health).toBeNull();
  });

  it("a Re-point that fails after validation records repoint_failed; a refused target records nothing", async () => {
    const { handlers, state } = backendFor("failures");
    const zod = state.skills.find((s) => s.dto.name === "zod-schema-first")!;
    await expect(
      handlers.repointSkillSource(zod.dto.id, { kind: "git", url: "not a url" }, { reassert_auto_sync: false }),
    ).rejects.toMatchObject({ code: "INVALID_GITHUB_URL" });
    expect(zod.dto.health).toBeNull();

    const { report } = await handlers.repointSkillSource(
      zod.dto.id,
      { kind: "git", url: "https://github.com/corp/private-skills" },
      { reassert_auto_sync: false },
    );
    const row = report.skills[0].status;
    if (row.status !== "failed") throw new Error(row.status);
    expect(zod.dto.health).toEqual({ kind: "repoint_failed", error: row.error, at: NOW });
  });

  it("an Update's Edit conflict records edit_conflict; saving the Edit clears it", async () => {
    const { handlers, state } = backendFor("failures");
    const tokens = state.skills.find((s) => s.dto.name === "design-tokens")!;
    const first = await handlers.updateManagedSkill(tokens.dto.id, { reassert_auto_sync: false }, sink().channel);
    expect(first.skills.find((s) => s.id === tokens.dto.id)!.health).toEqual({
      kind: "edit_conflict",
      error: { code: "EDIT_CONFLICT", baseMode: "user-and-model", upstreamMode: "model-only", overrideMode: "user-only" },
      at: NOW,
    });
    // An unchanged Update keeps the unresolved conflict.
    await handlers.updateManagedSkill(tokens.dto.id, { reassert_auto_sync: false }, sink().channel);
    expect(tokens.dto.health?.kind).toBe("edit_conflict");
    const saved = await handlers.setSkillInvocationOverride(tokens.dto.id, "user-only");
    expect(saved.skills.find((s) => s.id === tokens.dto.id)!.health).toBeNull();
  });

  it("a kept Delete records delete_incomplete with the failed target's error", async () => {
    const { handlers, state } = backendFor("failures");
    const train = state.skills.find((s) => s.dto.name === "release-train")!;
    train.dto.health = null;
    const report = await handlers.deleteManagedSkill(train.dto.id);
    expect(report.record_deleted).toBe(false);
    const failed = report.targets.find((t) => t.status.status === "failed")!;
    if (failed.status.status !== "failed") throw new Error("expected a failed target");
    expect(train.dto.health).toEqual({ kind: "delete_incomplete", error: failed.status.error, at: NOW });
  });

  it("an import that finalised but could not remove an original records import_failed", async () => {
    const { handlers, state } = backendFor("failures");
    const plan = await handlers.getOnboardingPlan();
    const group = plan.groups.find((g) => g.name === "commit-message")!;
    const chosen = group.variants.find((v) => v.tool === "codex")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: group.name, chosen_path: chosen.path, name: null }],
      { auto_sync: false, tools: null, remove_originals: true },
      sink().channel,
    );
    const status = report.groups[0].status;
    if (status.status !== "imported") throw new Error(status.status);
    const failed = status.originals.find((o) => o.status.status === "failed")!;
    if (failed.status.status !== "failed") throw new Error("expected a failed original");
    const skill = state.skills.find((s) => s.dto.id === status.skill_id)!;
    expect(skill.dto.health).toEqual({ kind: "import_failed", error: failed.status.error, at: NOW });
  });

  it("a failed first sync records error rows that own nothing; TARGET_EXISTS records none until decided", async () => {
    const { handlers, state } = backendFor("failures");
    const zod = state.skills.find((s) => s.dto.name === "zod-schema-first")!;
    const batch = [{ skill_id: zod.dto.id, name: zod.dto.name, source_path: zod.dto.central_path }];

    const unwritable = await handlers.syncSkillsToTools(batch, ["amp"], { overwrite_if_same_content: true }, sink().channel);
    expect(unwritable[0].status).toMatchObject({ status: "skipped", error: { code: "TOOL_NOT_WRITABLE" } });
    for (const tool of ["amp", "kimi_cli"]) {
      expect(zod.dto.targets.find((t) => t.tool === tool)).toMatchObject({ status: "error", synced_at: null });
    }

    const asked = await handlers.syncSkillsToTools(batch, ["cursor"], { overwrite_if_same_content: true }, sink().channel);
    expect(asked[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS" } });
    expect(zod.dto.targets.some((t) => t.tool === "cursor")).toBe(false);

    const kept = await handlers.syncSkillsToTools(
      batch,
      ["cursor"],
      { overwrite_if_same_content: true, overrides: [{ skill_id: zod.dto.id, tool: "cursor", decision: "keep" }] },
      sink().channel,
    );
    expect(kept[0].status).toEqual({ status: "declined" });
    expect(zod.dto.targets.find((t) => t.tool === "cursor")).toMatchObject({ status: "disabled", synced_at: null });
    expect(state.foreign.some((f) => f.tool === "cursor" && f.name === "zod-schema-first")).toBe(true);

    // Delete never takes the kept folder: the marker owns nothing.
    await handlers.deleteManagedSkill(zod.dto.id);
    expect(state.foreign.some((f) => f.tool === "cursor" && f.name === "zod-schema-first")).toBe(true);
  });

  it("an Enable meeting a folder in the way keeps the pair disabled", async () => {
    const { handlers, state } = backendFor("failures");
    const zod = state.skills.find((s) => s.dto.name === "zod-schema-first")!;
    await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: zod.dto.id, tool: "cursor" });
    const report = await handlers.enableSkillTargets({ kind: "skill_tool", skill_id: zod.dto.id, tool: "cursor" }, []);
    expect(report.global[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS" } });
    expect(zod.dto.targets.find((t) => t.tool === "cursor")).toMatchObject({ status: "disabled", synced_at: null });
  });
});

describe("fixture backend: Re-point preview (D15)", () => {
  it("flags a name mismatch and writes nothing", async () => {
    const { handlers } = backendFor("failures");
    const helper = await byName(handlers, "deprecated-helper");
    const before = await handlers.getManagedSkills();
    const preview = await handlers.previewRepointSource(helper.id, {
      kind: "git",
      url: "https://github.com/mattpocock/skills/tree/main/skills/engineering/ts-error-fixer",
    });
    expect(preview).toEqual({
      resolved_name: "ts-error-fixer",
      ref: "main",
      subpath: "skills/engineering/ts-error-fixer",
      name_matches: false,
      candidates: null,
    });
    expect(await handlers.getManagedSkills()).toEqual(before);
  });

  it("answers a multi-skill repository's candidates", async () => {
    const { handlers } = backendFor("failures");
    const helper = await byName(handlers, "deprecated-helper");
    const preview = await handlers.previewRepointSource(helper.id, { kind: "git", url: "https://github.com/mattpocock/skills" });
    expect(preview.resolved_name).toBeNull();
    expect(preview.name_matches).toBe(false);
    expect(preview.candidates?.map((c) => c.subpath)).toContain("skills/engineering/ts-error-fixer");
    expect(preview.candidates!.length).toBeGreaterThan(1);
  });

  it("resolves a local folder and refuses a missing one typed", async () => {
    const { handlers } = backendFor("failures");
    const notes = await byName(handlers, "local-notes");
    await expect(
      handlers.previewRepointSource(notes.id, { kind: "local", path: "~/Projects/pi-lens/skill" }),
    ).resolves.toEqual({ resolved_name: "pi-lens", ref: null, subpath: null, name_matches: false, candidates: null });
    await expect(
      handlers.previewRepointSource(notes.id, { kind: "local", path: "~/Downloads/scratchpad" }),
    ).rejects.toEqual({ code: "SOURCE_PATH_MISSING", path: "/Users/alex/Downloads/scratchpad" });
  });

  it("carries a local skill's recorded folder, which the source label shows", async () => {
    const { handlers } = backendFor("failures");
    const scratchpad = await byName(handlers, "scratchpad");
    expect(scratchpad.source_ref).toBe("/Users/alex/Downloads/scratchpad");
    expect(skillSourceLabel(scratchpad)).toBe("/Users/alex/Downloads/scratchpad");
  });
});

describe("ticket 04: one install batch, one overwrite ask (D12, D20)", () => {
  const statusOf = (skill: ManagedSkillDto, tool: string) =>
    skill.targets.find((row) => row.tool === tool)?.status;
  const COLLIDE = "https://github.com/acme/collide-kit";
  const picks = ["lint-staged-rules", "commit-helper", "changelog-writer"].map((name) => ({
    subpath: `skills/${name}`,
    name_override: null,
  }));
  const request = (overrides: { skill_name: string; tool: string; decision: "overwrite" | "keep" }[] = []) => ({
    source: { kind: "git" as const, url: COLLIDE, resolution: null },
    candidates: picks,
    tools: ["claude_code", "codex"],
    overwrite_if_same_content: true,
    overrides,
    deploy: true,
  });

  it("failures: a three-candidate repo with two collisions answers one report with two askable rows", async () => {
    const { handlers, state } = backendFor("failures");
    const occupants = state.foreign.length;
    const progress = sink<{ index: number; total: number; name: string }>();
    const report = await handlers.installSelection(request(), progress.channel);

    expect(progress.ticks.map((tick) => [tick.index, tick.total, tick.name])).toEqual([
      [1, 3, "lint-staged-rules"],
      [2, 3, "commit-helper"],
      [3, 3, "changelog-writer"],
    ]);
    expect(report.candidates.map((c) => c.status.status)).toEqual(["installed", "installed", "installed"]);
    const collided = report.candidates.flatMap((c) =>
      c.status.status === "installed"
        ? c.status.targets.filter((o) => o.status.status === "failed").map((o) => [o.skill_name, o.tool_key, o.status])
        : [],
    );
    expect(collided).toEqual(
      ["lint-staged-rules", "changelog-writer"].map((name) => [
        name,
        "claude_code",
        {
          status: "failed",
          error: {
            code: "TARGET_EXISTS",
            path: `${globalRoot("claude_code")}/${name}`,
            modifiedAt: expect.any(Number),
            sameContent: false,
          },
        },
      ]),
    );
    // Collected, never written through: the folders are still there, no row.
    expect(state.foreign).toHaveLength(occupants);
    const lint = await byName(handlers, "lint-staged-rules");
    expect(lint.targets.map((row) => row.tool)).toEqual(["codex"]);
  });

  it("the ask's retry is an ordinary sync batch: Overwrite replaces, Keep records disabled", async () => {
    const { handlers, state } = backendFor("failures");
    await handlers.installSelection(request(), sink().channel);
    const lint = await byName(handlers, "lint-staged-rules");
    const changelog = await byName(handlers, "changelog-writer");
    const retry = await handlers.syncSkillsToTools(
      [lint, changelog].map((s) => ({ skill_id: s.id, name: s.name, source_path: s.central_path })),
      ["claude_code"],
      {
        overwrite: false,
        overwrite_if_same_content: true,
        overrides: [
          { skill_id: lint.id, tool: "claude_code", decision: "overwrite" },
          { skill_id: changelog.id, tool: "claude_code", decision: "keep" },
        ],
      },
      sink().channel,
    );
    expect(retry.map((o) => [o.skill_name, o.status.status])).toEqual([
      ["lint-staged-rules", "synced"],
      ["changelog-writer", "declined"],
    ]);
    expect(state.foreign.some((f) => f.name === "lint-staged-rules")).toBe(false);
    expect(state.foreign.some((f) => f.name === "changelog-writer")).toBe(true);
    expect(statusOf(await byName(handlers, "changelog-writer"), "claude_code")).toBe("disabled");
  });

  it("a request override settles a collision in the first pass", async () => {
    const { handlers } = backendFor("failures");
    const report = await handlers.installSelection(
      request([{ skill_name: "lint-staged-rules", tool: "claude_code", decision: "overwrite" }]),
      sink().channel,
    );
    const lint = report.candidates[0].status;
    if (lint.status !== "installed") throw new Error(lint.status);
    expect(lint.targets.map((o) => [o.tool_key, o.status.status])).toEqual([
      ["claude_code", "synced"],
      ["codex", "synced"],
    ]);
  });

  it("cancel during candidate 2 reports 2–3 cancelled and finalizes nothing of them", async () => {
    const { handlers, armCancel } = cancellingBackend("failures");
    armCancel(2);
    const report = await handlers.installSelection(request(), sink().channel);
    expect(report.candidates.map((c) => [c.name, c.status.status])).toEqual([
      ["lint-staged-rules", "installed"],
      ["commit-helper", "cancelled"],
      ["changelog-writer", "cancelled"],
    ]);
    const names = (await handlers.getManagedSkills()).map((s) => s.name);
    expect(names).toContain("lint-staged-rules");
    expect(names).not.toContain("commit-helper");
    expect(names).not.toContain("changelog-writer");
  });

  it("a failed candidate is report data; the rest proceed", async () => {
    const { handlers } = backendFor("failures");
    const report = await handlers.installSelection(
      { ...request(), candidates: [picks[1], { subpath: "skills/nope", name_override: null }] },
      sink().channel,
    );
    expect(report.candidates.map((c) => c.status.status)).toEqual(["installed", "failed"]);
    expect(report.candidates[1]).toMatchObject({
      name: "nope",
      status: { status: "failed", error: { code: "SUBPATH_MISSING" } },
    });
  });

  it("the re-assert's collisions carry the ask's facts (round-18 fixture gap: prisma-review → Cursor/Codex)", async () => {
    const { handlers } = backendFor("failures");
    const prisma = await byName(handlers, "prisma-review");
    const report = await handlers.refreshManagedSkills([prisma.id], { reassert_auto_sync: true }, sink().channel);
    const status = report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(status.status);
    for (const tool of ["cursor", "codex"]) {
      expect(status.targets).toContainEqual({
        scope: { scope: "global", tool },
        status: {
          status: "failed",
          error: {
            code: "TARGET_EXISTS",
            path: `${globalRoot(tool)}/prisma-review`,
            modifiedAt: expect.any(Number),
            sameContent: false,
          },
        },
      });
    }
  });

  it("Enable's collision is answered by the same scope with the decision", async () => {
    const { handlers, state } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const scope = { kind: "skill_tool" as const, skill_id: zod.id, tool: "cursor" };
    await handlers.disableSkillTargets(scope);
    const first = await handlers.enableSkillTargets(scope, []);
    expect(first.global[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS", sameContent: false } });
    expect(statusOf(await byName(handlers, "zod-schema-first"), "cursor")).toBe("disabled");

    const retried = await handlers.enableSkillTargets(scope, [
      { skill_id: zod.id, tool: "cursor", decision: "overwrite" },
    ]);
    expect(retried.global.map((o) => o.status.status)).toEqual(["synced"]);
    expect(state.foreign.some((f) => f.tool === "cursor" && f.name === "zod-schema-first")).toBe(false);
  });

  it("a project collision resolves: Overwrite syncs, Keep records the row disabled (declined)", async () => {
    for (const decision of ["overwrite", "keep"] as const) {
      const { handlers, state } = backendFor("failures");
      const project = state.projects.find((p) => p.id === "prj-quartermaster")!;
      const prisma = await byName(handlers, "prisma-review");
      const path = `${project.path}/.claude/skills/prisma-review`;
      state.projectOccupants.push(path);
      const toggled = await handlers.toggleProjectSkillAssignment(project.id, prisma.id, "claude_code");
      if (toggled.kind !== "assigned") throw new Error(toggled.kind);
      expect(toggled.report.items[0].status).toEqual({
        status: "failed",
        error: { code: "TARGET_EXISTS", path, modifiedAt: expect.any(Number), sameContent: false },
      });

      const resolved = await handlers.resolveProjectCollisions([
        { project_id: project.id, skill_id: prisma.id, tool: "claude_code", decision },
      ]);
      const row = project.assignments.find((a) => a.skill_id === prisma.id && a.tool === "claude_code")!;
      if (decision === "overwrite") {
        expect(resolved.items[0].status).toEqual({ status: "synced" });
        expect(row.status).toBe("synced");
        expect(state.projectOccupants).not.toContain(path);
      } else {
        expect(resolved.items[0].status).toEqual({ status: "declined" });
        expect(row.status).toBe("disabled");
        expect(row.synced_at).toBeNull();
        expect(state.projectOccupants).toContain(path);
      }
    }
  });

  it("review F1: a synced → disabled → re-enabled assignment's collision honours Keep and Overwrite", async () => {
    for (const decision of ["overwrite", "keep"] as const) {
      const { handlers, state } = backendFor("rich");
      const zod = await byName(handlers, "zod-schema-first");
      const { project } = await handlers.registerProject("/Users/alex/Projects/review-probe");
      await handlers.configureProjectTools(project.id, ["claude_code"], null);
      await handlers.bulkAssignSkill(project.id, zod.id);
      const row = () =>
        state.projects
          .find((p) => p.id === project.id)!
          .assignments.find((a) => a.skill_id === zod.id && a.tool === "claude_code")!;
      expect(row().synced_at).not.toBeNull();
      await handlers.disableSkillTargets({ kind: "skill", skill_id: zod.id });
      const path = `${project.path}/.claude/skills/${zod.name}`;
      state.projectOccupants.push(path);

      const enabled = await handlers.enableSkillTargets({ kind: "skill", skill_id: zod.id }, []);
      expect(enabled.projects.items[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS" } });
      expect(row().synced_at).toBeNull();

      const resolved = await handlers.resolveProjectCollisions([
        { project_id: project.id, skill_id: zod.id, tool: "claude_code", decision },
      ]);
      if (decision === "overwrite") {
        expect(resolved.items[0].status).toEqual({ status: "synced" });
        expect(row().synced_at).not.toBeNull();
        expect(state.projectOccupants).not.toContain(path);
      } else {
        expect(resolved.items[0].status).toEqual({ status: "declined" });
        expect(row().status).toBe("disabled");
        const resync = await handlers.resyncProject(project.id);
        expect(resync.report.items[0].status).toEqual({ status: "skipped", reason: { reason: "disabled" } });
        expect(state.projectOccupants).toContain(path);
      }
    }
  });
});
