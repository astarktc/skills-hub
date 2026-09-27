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
import type { TranslateFn } from "../hooks/useStatusReporter";
import { refreshOutcome } from "../lib/reportOutcome";
import type { CommandName } from "../lib/tauri";
import { createFixtureBackend, type FixtureBackend, type FixtureHandlers } from "./backend";
import { SCENARIOS, type FixtureState, type ScenarioName } from "./model";
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
      status: { status: "failed", error: { code: "TARGET_EXISTS", path: "/Users/alex/.cursor/skills/zod-schema-first" } },
    });
    const fold = refreshOutcome(report, { t, single: { name: zod.name, success: "UPDATED" } });
    expect(fold.toast?.kind).not.toBe("success");
    expect(fold.errors.length).toBeGreaterThan(0);
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
      { auto_sync: true, tools: ["codex"] },
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

  it("honours a saved non-empty selection when the policy names no Tools", async () => {
    const { handlers } = backendFor("first-run");
    await handlers.updateSetting({ key: "global_tool_config", value: { selected_tools: ["claude_code"], scan_selected_only: false } });
    const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "pdf")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: "pdf", chosen_path: group.variants[0].path, name: null }],
      { auto_sync: true, tools: null },
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
    const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "tdd")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: "tdd", chosen_path: group.variants[0].path, name: null }],
      { auto_sync: true, tools: null },
      sink().channel,
    );
    const status = report.groups[0].status;
    if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
    expect(status.forced_tools.sort()).toEqual(["claude_code", "codex", "pi"]);
    expect(status.targets.map((o) => o.tool_key).sort()).toEqual(["claude_code", "codex", "pi"]);
  });

  it("makes one physical attempt (one report row) per shared skills dir", async () => {
    const { handlers } = backendFor("first-run");
    const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "ego-browser")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: "ego-browser", chosen_path: group.variants[0].path, name: null }],
      { auto_sync: true, tools: null },
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

  it("an early cancel answers per-skill CANCELLED rows, pre-settled skips last, nothing applied", async () => {
    const { handlers, armCancel } = cancellingBackend("failures");
    const before = await handlers.getManagedSkills();
    armCancel(1);
    const progress = sink<{ phase: string }>();
    const report = await handlers.refreshManagedSkills(null, { reassert_auto_sync: true }, progress.channel);
    const kinds = report.skills.map((s) => s.status.status);
    const firstSkip = kinds.indexOf("skipped");
    expect(firstSkip).toBeGreaterThan(0);
    expect(kinds.slice(0, firstSkip).every((k) => k === "failed")).toBe(true);
    expect(kinds.slice(firstSkip).every((k) => k === "skipped")).toBe(true);
    for (const s of report.skills.slice(0, firstSkip)) {
      expect(s.status).toEqual({ status: "failed", error: { code: "CANCELLED" } });
    }
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
    expect(report.skills.every((s) => s.status.status === "failed" && s.status.error.code === "CANCELLED")).toBe(true);
    expect(await handlers.getManagedSkills()).toEqual(before);
  });

  it("Update and Re-point answer a CANCELLED report row and leave the skill unchanged", async () => {
    const update = cancellingBackend("rich");
    const tdd = await byName(update.handlers, "tdd");
    update.armCancel(2);
    const updated = await update.handlers.updateManagedSkill(tdd.id, { reassert_auto_sync: true }, sink().channel);
    expect(updated.report.skills[0].status).toEqual({ status: "failed", error: { code: "CANCELLED" } });
    expect(updated.skills.find((s) => s.id === tdd.id)).toEqual(tdd);

    const repoint = cancellingBackend("rich");
    const pdf = await byName(repoint.handlers, "pdf");
    repoint.armCancel(1);
    const repointed = await repoint.handlers.repointSkillSource(pdf.id, { kind: "local", path: "~/Projects/pi-lens/skill" }, { reassert_auto_sync: false });
    expect(repointed.report.skills[0].status).toEqual({ status: "failed", error: { code: "CANCELLED" } });
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

  it("Import: no applying tick for a refused admission; apply failures still tick", async () => {
    const { handlers } = backendFor("failures");
    const plan = await handlers.getOnboardingPlan();
    const pick = (name: string) => ({ group_name: name, chosen_path: plan.groups.find((g) => g.name === name)!.variants[0].path, name: null });
    const progress = sink<{ phase: string; group_name: string }>();
    const report = await handlers.importOnboardingSelection(
      [pick("broken-skill"), { group_name: "vanished", chosen_path: "/nowhere", name: null }, pick("sql-review")],
      { auto_sync: false, tools: null },
      progress.channel,
    );
    expect(progress.ticks.map((tick) => `${tick.group_name}:${tick.phase}`)).toEqual([
      "broken-skill:admitting",
      "broken-skill:applying",
      "vanished:admitting",
      "sql-review:admitting",
      "sql-review:applying",
    ]);
    expect(report.groups.map((g) => g.status)).toEqual([
      { status: "failed", error: { code: "SKILL_INVALID", reason: "missing_name" } },
      { status: "failed", error: { code: "NOT_FOUND", kind: "onboarding_group", id: "vanished" } },
      expect.objectContaining({ status: "imported" }),
    ]);
  });
});

// ---------------------------------------------------------------------------
// Flow-review fixture gaps (ticket-07 flows.md § Fixture gaps: F2, F4)
// ---------------------------------------------------------------------------

describe("F2: unwritable dirs refuse removal too (ADR-0002)", () => {
  it("unsync from a shared unwritable dir: one failed target, both rows kept as error", async () => {
    const { handlers } = backendFor("failures");
    const docx = await byName(handlers, "docx");
    const report = await handlers.unsyncSkillFromTool(docx.id, "amp");
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
    // Never-deployed rows (pending/error/missing) had no artifact: removed.
    expect(report.targets.filter((target) => target.status.status === "removed").length).toBeGreaterThan(0);
    expect(projects.some((p) => p.id === "prj-monorepo")).toBe(true);
    const view = await handlers.getProjectView("prj-monorepo");
    expect(view.assignments).toEqual([
      expect.objectContaining({ skill_name: "release-train", tool: "agents_skills", status: "error" }),
    ]);
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
