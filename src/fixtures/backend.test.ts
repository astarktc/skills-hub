import type { Channel } from "@tauri-apps/api/core";
import { describe, expect, expectTypeOf, it } from "vitest";
import {
  commands,
  type LibraryMoveProgress,
  type ManagedSkillDto,
  type ProjectDto,
  type RefreshProgressDto,
  type SkillRefreshStatus,
  type SyncProgressDto,
  type ProjectViewDto,
  type RefreshReport,
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

  // Round 20 rows review (P3): the registry's copy-only capability reaches the fixture's placement, so a
  // fixture-driven check sees Deep Agents Code deploy as a copy (the one `supports_symlink: false` row).
  it("carries each harness's link-vs-copy capability", async () => {
    const { handlers } = backendFor("rich");
    const status = await handlers.getToolStatus();
    const copyOnly = status.tools.filter((tool) => tool.placement?.supports_symlink === false).map((tool) => tool.key);
    expect(copyOnly).toEqual(["deep_agents_code"]);
  });

  // Round 20 ticket 01: reader-only harnesses are listed under the
  // `.agents/skills` entry with their detected state, never as entries, never
  // installed keys, and are absorbed into the project group's roster.
  it("lists reader-only harnesses under the .agents/skills entry, never as targets", async () => {
    const { handlers } = backendFor("rich");
    const status = await handlers.getToolStatus();
    const agents = status.tools.find((tool) => tool.key === "agents_skills");
    expect(agents?.readers.map((reader) => [reader.key, reader.installed])).toEqual([
      ["warp", false],
      ["zed", true],
      ["dexto", false],
      ["fx", true],
      ["sarvam_code", false],
      ["vt_code", false],
      ["dimcode", false],
      ["crow_cli", false],
      ["corust_agent", false],
    ]);
    expect(status.tools.some((tool) => tool.key === "zed")).toBe(false);
    expect(status.installed).not.toContain("zed");
    expect(status.tools.filter((tool) => tool.key !== "agents_skills").every((tool) => tool.readers.length === 0)).toBe(true);
    const project = await handlers.getProjectToolStatus();
    const group = project.tools.find((tool) => tool.key === "agents_skills");
    expect(group?.constituents.slice(-2)).toEqual(["crow-cli", "Corust Agent"]);
  });

  // Ticket 31 (D41): the listing carries each skill's projects, derived from
  // the assignment rows at read time — so an assignment shows at once.
  it("lists each skill's projects: three + global, project-only, a failed project deployment", async () => {
    const { handlers } = backendFor("rich");
    const names = (skill: ManagedSkillDto) => skill.projects.map((p) => p.project_name);
    const perf = await byName(handlers, "react-perf-audit");
    expect(names(perf)).toEqual(["agent-skills", "quartermaster", "skills-hub"]);
    expect(perf.projects[2].deployments.map((d) => d.tool)).toEqual(["agents_skills", "claude_code"]);
    expect(perf.targets.length).toBeGreaterThan(0);
    const prisma = await byName(handlers, "prisma-review");
    expect([prisma.targets.length, names(prisma)]).toEqual([0, ["quartermaster"]]);
    const plans = await byName(handlers, "writing-plans");
    expect(plans.projects[0].deployments).toContainEqual({
      tool: "agents_skills",
      status: "error",
      synced_at: null,
      disabled: false,
      deployed: false,
    });
    expect((await byName(handlers, "pdf")).projects).toEqual([]);

    await handlers.toggleProjectSkillAssignment("prj-quartermaster", (await byName(handlers, "pdf")).id, "claude_code");
    expect(names(await byName(handlers, "pdf"))).toEqual(["quartermaster"]);
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
      { overrides: [{ skill_id: zod.id, tool: "cursor", path: "/Users/alex/.cursor/skills/zod-schema-first", decision: "overwrite" }] },
      sink().channel
    );
    expect(retry[0].status).toMatchObject({ status: "synced", outcome: { replaced: true } });
  });

  it("writes a shared skills dir once and records one row, keyed by its folder", async () => {
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
    expect(after.targets.map((t) => t.tool)).toContain("amp");
    expect(after.targets.map((t) => t.tool)).not.toContain("kimi_cli");
  });

  it("streams both Refresh phases and reports every refreshable skill", async () => {
    const { handlers, state } = backendFor("rich");
    const progress = sink<{ phase: string }>();
    const report = await handlers.refreshManagedSkills(null, progress.channel );
    expect(report.skills).toHaveLength(60);
    expect(new Set(progress.ticks.map((t) => t.phase))).toEqual(new Set(["acquiring", "applying"]));
    // Propagation leaves every project row content-identity consistent.
    expectRecordedHashes(state);
  });

  // S4 (replaces an assertion that errored links recover on Refresh).
  it("re-materialises errored copies on Update; links follow the source", async () => {
    const { handlers } = backendFor("rich");
    const worktrees = await byName(handlers, "using-git-worktrees");
    const { report } = await handlers.updateManagedSkill(worktrees.id, sink().channel);
    const status = report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    const global = Object.fromEntries(
      status.targets.flatMap((o) => (o.scope.scope === "global" ? [[o.scope.tool, o.status]] : [])),
    );
    // amp/kimi_cli are pre-1.4.0 rows of one folder (`amp`), copies: each row
    // is rewritten by itself and reported by itself (Rust per-row Propagation).
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

describe("Move Library folder (round 20 #03)", () => {
  it("previews, moves every locatable skill, re-points links and answers the catalog", async () => {
    const { handlers, state } = backendFor("rich");
    const preview = await handlers.previewLibraryMove("~/Skills Library");
    expect(preview.destination).toBe("/Users/alex/Skills Library");
    expect(preview.skills + preview.missing).toBe(60);
    expect(preview.cloud_sync).toBeNull();

    const progress = sink<{ phase: string }>();
    const { report, skills } = await handlers.moveLibrary(preview.destination, progress.channel);

    expect(new Set(progress.ticks.map((tick) => tick.phase))).toEqual(new Set(["copying", "relinking", "cleaning"]));
    expect(report.to).toBe("/Users/alex/Skills Library");
    expect(state.settings.central_repo_path).toBe("/Users/alex/Skills Library");
    const movedIds = report.skills.filter((s) => s.status.status === "moved").map((s) => s.skill_id);
    expect(movedIds).toHaveLength(preview.skills);
    for (const skill of skills.filter((s) => movedIds.includes(s.id))) {
      expect(skill.central_path.startsWith("/Users/alex/Skills Library/")).toBe(true);
    }
    expect(report.deployments.some((d) => d.status.status === "relinked")).toBe(true);
    expect(report.left_behind[0]).toMatchObject({ reason: "old_library" });
  });

  it("refuses a folder that is not empty, and one inside the Library", async () => {
    const { handlers, state } = backendFor("rich");
    const before = state.settings.central_repo_path;
    await expect(handlers.previewLibraryMove("~/Documents")).rejects.toMatchObject({
      code: "LIBRARY_DESTINATION_REFUSED",
      reason: "notEmpty",
    });
    await expect(handlers.moveLibrary(`${before}/nested`, sink<LibraryMoveProgress>().channel)).rejects.toMatchObject({
      code: "LIBRARY_DESTINATION_REFUSED",
      reason: "overlapsLibrary",
    });
    expect(state.settings.central_repo_path).toBe(before);
  });

  it("warns about a cloud-synced folder", async () => {
    const { handlers } = backendFor("rich");
    const preview = await handlers.previewLibraryMove("~/Library/Mobile Documents/com~apple~CloudDocs/Skills");
    expect(preview.cloud_sync).toMatchObject({ provider: "icloud_drive" });
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
    const report = await handlers.refreshManagedSkills(null, sink().channel );
    const kinds = new Set(report.skills.map((s) => s.status.status));
    expect(kinds).toEqual(new Set(["refreshed", "failed", "skipped", "skipped_acquisition"]));
    const codes = report.skills.flatMap((s) => (s.status.status === "failed" ? [s.status.error.code] : []));
    expect(codes).toEqual(expect.arrayContaining(["RATE_LIMITED", "GITHUB_SKILL_NOT_FOUND", "GIT_CLONE_FAILED", "SYMLINK_ESCAPES_REPO"]));
    const refreshed = report.skills.flatMap((s) => (s.status.status === "refreshed" ? [s.status] : []));
    expect(refreshed.some((s) => s.edit_conflict !== null)).toBe(true);
    expect(refreshed.some((s) => s.targets.some((t) => t.status.status === "failed"))).toBe(true);
  });

  // S4: `needs_new_bytes` is mode-only — an errored link is skipped and
  // stays errored; an errored copy is retried and can fail again.
  it("leaves errored links alone and retries (and fails) errored copies", async () => {
    const { handlers } = backendFor("failures");
    const worktrees = await byName(handlers, "using-git-worktrees");
    const tdd = await byName(handlers, "tdd");
    const statusOf = async (id: string) => {
      const { report } = await handlers.updateManagedSkill(id, sink().channel);
      const status = report.skills[0].status;
      if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
      return Object.fromEntries(
        status.targets.flatMap((o) => (o.scope.scope === "global" ? [[o.scope.tool, o.status]] : [])),
      );
    };

    // Amp and Kimi Code's pre-1.4.0 rows are one folder (`amp`) to readers,
    // and each its own row to Propagation, reported by itself.
    const links = await statusOf(worktrees.id);
    for (const tool of ["amp", "kimi_cli", "cursor"]) {
      expect(links[tool]).toEqual({ status: "skipped", reason: { reason: "link_follows_source" } });
    }
    const linkRows = (await byName(handlers, "using-git-worktrees")).targets;
    for (const tool of ["amp", "cursor"]) {
      expect(linkRows.find((r) => r.tool === tool)).toMatchObject({ mode: "symlink", status: "error" });
    }

    const copies = await statusOf(tdd.id);
    expect(copies.amp).toMatchObject({ status: "failed", error: { code: "REMOVAL_PERMISSION_DENIED" } });
    expect(copies.kimi_cli).toMatchObject({ status: "failed", error: { code: "REMOVAL_PERMISSION_DENIED" } });
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

  it("Refresh reports disabled targets skipped and never brings them back", async () => {
    const { handlers } = backendFor("failures");
    const noisy = await byName(handlers, "noisy-linter");
    const retired = await byName(handlers, "retired-playbook");
    const report = await handlers.refreshManagedSkills(
      [noisy.id, retired.id],
      sink<RefreshProgressDto>().channel,
    );
    // Which global Tools each skill has disabled (pair grain) — every one of
    // them must come back `skipped: disabled`, never `failed`. An Update
    // only reaches existing deployments (D34), so nothing else fails either.
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
      ).toEqual([]);
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
    expect(off.disabled_markers).toEqual([]);
    expect(statusOf(await byName(handlers, "zod-schema-first"), "claude_code")).toBe("disabled");
    const on = await handlers.enableSkillTargets({ kind: "skill_tool", skill_id: zod.id, tool: "claude_code" }, []);
    expect(on.global.map((o) => o.status.status)).toEqual(["synced"]);
    expect(statusOf(await byName(handlers, "zod-schema-first"), "claude_code")).toBe("synced");

    const prisma = await byName(handlers, "prisma-review");
    const marker = await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: prisma.id, tool: "claude_code" });
    expect(marker.targets).toEqual([]);
    // #95: the recorded intent is report data (Rust `disabled_markers`).
    expect(marker.disabled_markers).toEqual([expect.objectContaining({ scope: "global_target", tool: "claude_code" })]);
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

  it("ticket 21: each copy carries its description, modified time and comparison with the managed copy", async () => {
    const { handlers } = backendFor("first-run");
    const plan = await handlers.getOnboardingPlan();
    const handoff = plan.groups.find((g) => g.name === "handoff")!;
    expect(handoff.has_conflict).toBe(true);
    expect(handoff.variants.map((v) => v.description)).toEqual([
      "Compact the conversation into a handoff document",
      "Write a short handoff note for the next session",
    ]);
    expect(handoff.variants.every((v) => typeof v.modified_at === "number")).toBe(true);
    expect(handoff.variants.every((v) => v.same_as_managed === null)).toBe(true);
    // tdd's copies are someone's fork; frontend-design's are the managed bytes.
    const compare = (name: string) => plan.groups.find((g) => g.name === name)!.variants.map((v) => v.same_as_managed);
    expect(compare("tdd")).toEqual([false, false, false]);
    expect(compare("frontend-design")).toEqual([true, true]);
  });

  it("ticket 21: remove_divergent removes the copies that differ; without it they stay", async () => {
    for (const remove of [false, true]) {
      const { handlers } = backendFor("first-run");
      const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "handoff")!;
      const report = await handlers.importOnboardingSelection(
        [{ group_name: group.name, chosen_path: group.variants[0].path, name: null, remove_divergent: remove }],
        { auto_sync: false, tools: null, remove_originals: false },
        sink().channel,
      );
      const status = report.groups[0].status;
      if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
      expect(status.central_path).toMatch(/handoff$/);
      expect(status.originals.map((o) => [o.tool, o.status.status]).sort()).toEqual(
        [["claude_code", "left_in_place"], ["pi", remove ? "removed" : "kept_divergent"]],
      );
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

describe("D34: an Update only reaches deployments that exist (auto-sync on)", () => {
  const globalTools = (status: SkillRefreshStatus) => {
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    return status.targets.flatMap((o) => (o.scope.scope === "global" ? [o.scope.tool] : []));
  };

  it("Update, Refresh and Re-point create no deployment and raise no collision", async () => {
    const { handlers, state } = backendFor("rich");
    expect(state.settings.auto_sync_enabled).toBe(true);
    const zod = await byName(handlers, "zod-schema-first");
    const before = zod.targets.map((row) => row.tool);
    // Propagation reports each stored row (pre-1.4.0 sibling rows included);
    // the catalog reads them as their folders.
    const stored = () => state.skills.find((s) => s.dto.id === zod.id)!.dto.targets.map((row) => row.tool);
    const storedBefore = stored();
    // Cursor holds a foreign zod-schema-first folder in this scenario: an
    // Update never goes there, so nothing collides.
    const updated = await handlers.updateManagedSkill(zod.id, sink().channel);
    expect(globalTools(updated.report.skills[0].status)).toEqual(storedBefore);
    const refreshed = await handlers.refreshManagedSkills([zod.id], sink().channel);
    expect(globalTools(refreshed.skills[0].status)).toEqual(storedBefore);
    const repointed = await handlers.repointSkillSource(zod.id, {
      kind: "git",
      url: "https://github.com/mattpocock/skills/tree/main/skills/engineering/zod-schema-first",
    });
    expect(globalTools(repointed.report.skills[0].status)).toEqual(storedBefore);
    expect(stored()).toEqual(storedBefore);
    expect((await byName(handlers, "zod-schema-first")).targets.map((row) => row.tool)).toEqual(before);
    const fold = refreshOutcome(updated.report, { t, single: { name: zod.name, success: "UPDATED" } });
    expect(fold.toast?.tone).toBe("success");
    expect(fold.warnings).toEqual([]);
  });

  it("Refresh all leaves every skill's deployments as they were", async () => {
    const { handlers } = backendFor("rich");
    const before = (await handlers.getManagedSkills()).map((skill) => [skill.id, skill.targets.map((row) => row.tool)]);
    const report = await handlers.refreshManagedSkills(null, sink().channel);
    for (const outcome of report.skills) {
      const status = outcome.status;
      if (status.status !== "refreshed") continue;
      expect(status.targets.every((o) => o.status.status !== "failed" || o.status.error.code !== "TARGET_EXISTS")).toBe(true);
    }
    const after = (await handlers.getManagedSkills()).map((skill) => [skill.id, skill.targets.map((row) => row.tool)]);
    expect(after).toEqual(before);
  });

  it("a skill deployed nowhere stays deployed nowhere", async () => {
    const { handlers } = backendFor("failures");
    const prisma = await byName(handlers, "prisma-review");
    const report = await handlers.refreshManagedSkills([prisma.id], sink().channel);
    expect(globalTools(report.skills[0].status)).toEqual([]);
    expect((await byName(handlers, "prisma-review")).targets).toEqual([]);
  });
});

describe("D35: a delete_incomplete skill is never updated", () => {
  const skippedDeleteIncomplete = { status: "skipped", reason: { kind: "delete_incomplete" } };

  it("Refresh all, a named Update and a Re-point skip release-train and leave it as it was", async () => {
    const { handlers } = backendFor("failures");
    const train = await byName(handlers, "release-train");
    expect(train.health?.kind).toBe("delete_incomplete");
    const all = await handlers.refreshManagedSkills(null, sink().channel);
    expect(all.skills.find((s) => s.skill_id === train.id)?.status).toEqual(skippedDeleteIncomplete);
    const named = await handlers.updateManagedSkill(train.id, sink().channel);
    expect(named.report.skills.map((s) => s.status)).toEqual([skippedDeleteIncomplete]);
    const repointed = await handlers.repointSkillSource(train.id, { kind: "local", path: "~/Projects/pi-lens/skill" });
    expect(repointed.report.skills.map((s) => s.status)).toEqual([skippedDeleteIncomplete]);
    expect(await byName(handlers, "release-train")).toEqual(train);
    // The fold: a skipped item with the health row's repair, finish the Delete.
    const fold = refreshOutcome(all, { t });
    expect(fold.warnings).toContainEqual({
      title: t("updateSkip.deleteIncompleteTitle", { name: "release-train" }),
      message: t("updateSkip.deleteIncompleteMessage"),
      subject: { skillId: train.id, skillName: "release-train" },
      repair: { verb: "delete" },
    });
  });
});

describe("D35 boundary of Refresh all (ticket 33 audit F2)", () => {
  const skippedDeleteIncomplete = { status: "skipped", reason: { kind: "delete_incomplete" } };

  for (const state of ["source_missing", "central_missing"] as const) {
    it(`delete_incomplete takes precedence over Unlocatable (${state}); nothing is acquired or changed`, async () => {
      const { handlers, state: fixture } = backendFor("failures");
      const train = fixture.skills.find((s) => s.dto.name === "release-train")!;
      train.dto.unlocatable = state;
      if (state === "central_missing") train.files = [];
      const before = await byName(handlers, "release-train");
      const filesBefore = train.files;
      const progress = sink<RefreshProgressDto>();
      const all = await handlers.refreshManagedSkills(null, progress.channel);
      expect(all.skills.filter((s) => s.skill_id === train.dto.id).map((s) => s.status)).toEqual([skippedDeleteIncomplete]);
      expect(progress.ticks.filter((tick) => tick.skill_name === "release-train")).toEqual([]);
      expect(await byName(handlers, "release-train")).toEqual(before);
      expect(train.files).toBe(filesBefore);
    });
  }

  it("an imported skill carrying delete_incomplete is not a member: absent, never acquired", async () => {
    const { handlers, state: fixture } = backendFor("failures");
    const lint = fixture.skills.find((s) => s.dto.name === "legacy-lint")!;
    expect(lint.dto.source_type).toBe("imported");
    lint.dto.health = { kind: "delete_incomplete", error: { code: "OTHER", message: "permission denied" }, at: NOW };
    const before = await byName(handlers, "legacy-lint");
    const progress = sink<RefreshProgressDto>();
    const all = await handlers.refreshManagedSkills(null, progress.channel);
    expect(all.skills.some((s) => s.skill_id === lint.dto.id)).toBe(false);
    expect(progress.ticks.filter((tick) => tick.skill_name === "legacy-lint")).toEqual([]);
    expect(await byName(handlers, "legacy-lint")).toEqual(before);
  });
});

describe("one folder, one row: a shared skills dir records no sibling on any write path (round 20 ticket 09, audit F1)", () => {
  /**
   * The audit's state: json-canvas with only Amp's never-owned `error` row (a
   * first sync that failed before Kimi was installed); Kimi shares Amp's dir
   * and is installed now. `foreign` puts a folder in their shared path.
   */
  function canvasOnAmp({ foreign = false, kimiInstalled = true } = {}) {
    const fixture = buildScenario("failures", NOW);
    const skill = fixture.skills.find((s) => s.dto.name === "json-canvas")!;
    fixture.skills = [skill];
    fixture.projects = [];
    fixture.unwritableTools = [];
    skill.dto.targets = skill.dto.targets.filter((row) => row.tool === "amp");
    expect(skill.dto.targets.map((row) => [row.status, row.synced_at])).toEqual([["error", null]]);
    if (!kimiInstalled) fixture.installedTools = fixture.installedTools.filter((tool) => tool !== "kimi_cli");
    else expect(fixture.installedTools).toContain("kimi_cli");
    if (foreign) fixture.foreign.push({ tool: "amp", name: skill.dto.name, fingerprint: "foreign", linkTarget: null });
    const backend = createFixtureBackend(fixture, { latencyScale: 0, now: () => NOW });
    const rows = async () =>
      (await backend.handlers.getManagedSkills())[0].targets.map(({ tool, status, target_path, synced_at }) => ({
        tool,
        status,
        target_path,
        owned: synced_at !== null,
      }));
    const tools = async () => (await rows()).map((row) => row.tool);
    return { ...backend, skill, rows, tools };
  }
  const globalOutcomes = (status: SkillRefreshStatus) => {
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    return status.targets.flatMap((o) => (o.scope.scope === "global" ? [[o.scope.tool, o.status]] : []));
  };

  it("the Update's own first sync records the shared folder's one row", async () => {
    const { handlers, skill, rows, tools } = canvasOnAmp();
    const { report } = await handlers.updateManagedSkill(skill.dto.id, sink().channel);
    expect(globalOutcomes(report.skills[0].status)).toEqual([["amp", { status: "synced", mode_used: "symlink" }]]);
    expect(await tools()).toEqual(["amp"]);
    const [amp] = await rows();
    expect(amp.owned).toBe(true);
  });

  it("the audit's repro: a collision records nothing new; the confirmed retry records the folder's row, the same rows", async () => {
    const { handlers, skill, rows, tools } = canvasOnAmp({ foreign: true });
    const before = await tools();
    expect(before).toEqual(["amp"]);
    const { report } = await handlers.updateManagedSkill(skill.dto.id, sink().channel);
    const outcomes = globalOutcomes(report.skills[0].status);
    expect(outcomes).toHaveLength(1);
    expect(outcomes[0][0]).toBe("amp");
    expect(outcomes[0][1]).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS" }, first_deploy: true });
    expect(await tools()).toEqual(["amp"]);
    // The same command/policy `settleClaims` uses for this Update collision.
    await handlers.syncSkillsToTools(
      [{ skill_id: skill.dto.id, name: skill.dto.name, source_path: skill.dto.central_path }],
      ["amp"],
      { overwrite_if_same_content: true, overrides: [{ skill_id: skill.dto.id, tool: "amp", path: `${globalRoot("amp")}/${skill.dto.name}`, decision: "overwrite" }] },
      sink<SyncProgressDto>().channel,
    );
    expect(await tools()).toEqual(["amp"]);
    // Whichever writer settled the shared dir, the catalog reads the same.
    const control = canvasOnAmp();
    await control.handlers.updateManagedSkill(control.skill.dto.id, sink().channel);
    expect(await rows()).toEqual(await control.rows());
  });

  it("records no sharer that is not installed", async () => {
    const { handlers, skill, tools } = canvasOnAmp({ kimiInstalled: false });
    const { report } = await handlers.updateManagedSkill(skill.dto.id, sink().channel);
    expect(globalOutcomes(report.skills[0].status).map(([tool]) => tool)).toEqual(["amp"]);
    expect(await tools()).toEqual(["amp"]);
  });

  it("an owned artifact's update records no new sharer", async () => {
    const { handlers, skill, tools } = canvasOnAmp();
    skill.dto.targets[0].status = "synced";
    skill.dto.targets[0].synced_at = NOW - 1000;
    skill.dto.targets[0].mode = "copy";
    await handlers.updateManagedSkill(skill.dto.id, sink().channel);
    expect(await tools()).toEqual(["amp"]);
  });

  it("a disabled member skips the group and mints no sharer", async () => {
    const { handlers, skill, tools } = canvasOnAmp();
    skill.dto.targets[0].status = "disabled";
    const { report } = await handlers.updateManagedSkill(skill.dto.id, sink().channel);
    expect(globalOutcomes(report.skills[0].status)).toEqual([["amp", { status: "skipped", reason: { reason: "disabled" } }]]);
    expect(await tools()).toEqual(["amp"]);
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
    // Amp and Kimi share the dir holding the identical copy: both are reported
    // on it (#90, as Rust's plan does), so both are forced.
    expect(status.forced_tools).toEqual(["amp", "kimi_cli"]);
    const byTool = Object.fromEntries(status.targets.map((o) => [o.tool_key, o.status]));
    expect(byTool.codex).toMatchObject({ status: "synced", outcome: { replaced: true } });
    expect(byTool.amp).toMatchObject({ status: "skipped", error: { code: "TOOL_NOT_WRITABLE" } });
    // The unwritable shared original survives and stays discoverable, for both.
    const after = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "commit-message");
    expect(after?.variants.map((v) => v.tool)).toEqual(["amp", "kimi_cli"]);
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
    const report = await handlers.refreshManagedSkills(null, sink().channel);
    expect(report.skills.every((s) => s.status.status === "refreshed")).toBe(true);
  });

  it("an early cancel answers per-skill cancelled rows, pre-settled skips last, nothing applied", async () => {
    const { handlers, armCancel } = cancellingBackend("failures");
    const before = await handlers.getManagedSkills();
    armCancel(1);
    const progress = sink<{ phase: string }>();
    const report = await handlers.refreshManagedSkills(null, progress.channel);
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
    const report = await handlers.refreshManagedSkills(null, progress.channel);
    expect(progress.ticks.map((tick) => tick.phase)).toEqual(["acquiring", "acquiring"]);
    expect(report.skills).toHaveLength(60);
    expect(report.skills.every((s) => s.status.status === "cancelled")).toBe(true);
    expect(await handlers.getManagedSkills()).toEqual(before);
  });

  it("Update and Re-point answer a cancelled report row and leave the skill unchanged", async () => {
    const update = cancellingBackend("rich");
    const tdd = await byName(update.handlers, "tdd");
    update.armCancel(2);
    const updated = await update.handlers.updateManagedSkill(tdd.id, sink().channel);
    expect(updated.report.skills[0].status).toEqual({ status: "cancelled" });
    expect(updated.skills.find((s) => s.id === tdd.id)).toEqual(tdd);

    const repoint = cancellingBackend("rich");
    const pdf = await byName(repoint.handlers, "pdf");
    repoint.armCancel(1);
    const repointed = await repoint.handlers.repointSkillSource(pdf.id, { kind: "local", path: "~/Projects/pi-lens/skill" });
    expect(repointed.report.skills[0].status).toEqual({ status: "cancelled" });
    expect(repointed.skills.find((s) => s.id === pdf.id)).toEqual(pdf);
  });

  it("the Add batch reports a cancelled candidate, and Explore clone rejects with the typed CANCELLED", async () => {
    const install = cancellingBackend("rich");
    install.armCancel(1);
    const report = await install.handlers.installSelection(
      {
        source: { kind: "git", url: "https://github.com/acme/changelog-skill", resolution: null },
        candidates: [{ subpath: "./changelog-writer", name_override: null }],
        tools: [],
        overwrite_if_same_content: true,
        overrides: [],
        deploy: false,
      },
      sink().channel,
    );
    expect(report.candidates.map((c) => c.status.status)).toEqual(["cancelled"]);
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
    const add = (name_override: string | null) =>
      handlers.installSelection(
        {
          source: { kind: "git", url, resolution: null },
          candidates: [{ subpath: "./changelog-writer", name_override }],
          tools: [],
          overwrite_if_same_content: true,
          overrides: [],
          deploy: false,
        },
        sink().channel,
      );
    const installed = (await add("my-custom-name")).candidates[0];
    expect(installed).toMatchObject({ name: "my-custom-name", status: { status: "installed" } });
    expect((await handlers.getManagedSkills()).some((s) => s.name === "my-custom-name")).toBe(true);
    expect((await add("My-Custom-Name")).candidates[0].status).toEqual({
      status: "failed",
      error: { code: "SKILL_EXISTS", name: "My-Custom-Name" },
    });
    // Round 20 t19: a name is one folder name.
    expect((await add("../x")).candidates[0].status).toEqual({
      status: "failed",
      error: { code: "SKILL_NAME_INVALID", name: "../x" },
    });
    expect((await add(null)).candidates[0].name).toBe("changelog-writer");
  });
});

describe("N1: progress ticks only the work that reached each phase", () => {
  it("Refresh: no applying tick for a failed acquisition", async () => {
    const { handlers } = backendFor("failures");
    const progress = sink<{ phase: string; skill_name: string }>();
    const report = await handlers.refreshManagedSkills(null, progress.channel);
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
    const { handlers, state } = backendFor("failures");
    const docx = await byName(handlers, "docx");
    const report = await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: docx.id, tool: "amp" });
    expect(report.targets).toHaveLength(1);
    // The scenario's pre-1.4.0 sibling rows are both the folder's.
    expect(report.targets[0].rows.map((r) => r.tool).sort()).toEqual(["amp", "kimi_cli"]);
    expect(report.targets[0].status).toMatchObject({ status: "failed", error: { code: "REMOVAL_PERMISSION_DENIED" } });
    const stored = state.skills.find((s) => s.dto.id === docx.id)!.dto.targets;
    expect(stored.filter((r) => r.tool === "amp" || r.tool === "kimi_cli").map((r) => r.status)).toEqual([
      "error",
      "error",
    ]);
    // The catalog reads them as the one folder they are.
    const rows = (await byName(handlers, "docx")).targets.filter((r) => r.tool === "amp" || r.tool === "kimi_cli");
    expect(rows.map((r) => [r.tool, r.status])).toEqual([["amp", "error"]]);
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

  it("F3: a failed shared-dir removal mints no marker; the retry settles the folder's row", async () => {
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
    // Kimi Code's pre-1.4.0 row is the folder's record: no second row is minted.
    expect(shared()).toEqual([["kimi_cli", "disabled"]]);
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

    const first = await handlers.updateManagedSkill(tokens.id, sink().channel);
    const status = first.report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(`expected refreshed, got ${status.status}`);
    expect(status.edit_conflict).toEqual({ base_mode: "user-and-model", upstream_mode: "model-only", override_mode: "user-only" });
    const card = first.skills.find((s) => s.id === tokens.id)!;
    expect(card.invocation_override).toEqual({ mode: "user-only", base_mode: status.edit_conflict!.upstream_mode, conflict: true });
    expect(card.invocation_mode).toBe("user-only");

    // An unchanged Update reports nothing new; the conflict stays flagged.
    const second = await handlers.updateManagedSkill(tokens.id, sink().channel);
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
    // A failed first sync's rows own nothing (Amp and Kimi Code's, one folder).
    const canvas = await byName(handlers, "json-canvas");
    expect(canvas.targets.filter((t) => t.status === "error")).toEqual([
      expect.objectContaining({ tool: "amp", synced_at: null }),
    ]);
  });

  it("a failed Update records fetch_failed with the row's error; the next success clears it", async () => {
    const { handlers, state } = backendFor("failures");
    const pr = state.skills.find((s) => s.dto.name === "pr-review")!;
    pr.dto.health = null;
    const failed = await handlers.updateManagedSkill(pr.dto.id, sink().channel);
    const row = failed.report.skills[0].status;
    if (row.status !== "failed") throw new Error(row.status);
    const card = failed.skills.find((s) => s.id === pr.dto.id)!;
    expect(card.health).toEqual({ kind: "fetch_failed", error: row.error, at: NOW });

    pr.acquisition.fail = undefined;
    const ok = await handlers.updateManagedSkill(pr.dto.id, sink().channel);
    expect(ok.skills.find((s) => s.id === pr.dto.id)!.health).toBeNull();
  });

  it("a failed Restore records restore_failed; a cancelled Refresh records nothing", async () => {
    const { handlers, state } = backendFor("failures");
    const scratch = state.skills.find((s) => s.dto.name === "scratchpad")!;
    scratch.dto.health = null;
    const { skills } = await handlers.updateManagedSkill(scratch.dto.id, sink().channel);
    expect(skills.find((s) => s.id === scratch.dto.id)!.health).toMatchObject({
      kind: "restore_failed",
      error: { code: "SOURCE_PATH_MISSING" },
    });

    const cancelling = cancellingBackend("failures");
    const pr = cancelling.state.skills.find((s) => s.dto.name === "pr-review")!;
    pr.dto.health = null;
    cancelling.armCancel(1);
    const report = await cancelling.handlers.refreshManagedSkills([pr.dto.id], sink().channel);
    expect(report.skills[0].status).toEqual({ status: "cancelled" });
    expect(pr.dto.health).toBeNull();
  });

  it("a Re-point that fails after validation records repoint_failed; a refused target records nothing", async () => {
    const { handlers, state } = backendFor("failures");
    const zod = state.skills.find((s) => s.dto.name === "zod-schema-first")!;
    await expect(
      handlers.repointSkillSource(zod.dto.id, { kind: "git", url: "not a url" }),
    ).rejects.toMatchObject({ code: "INVALID_GITHUB_URL" });
    expect(zod.dto.health).toBeNull();

    const { report } = await handlers.repointSkillSource(
      zod.dto.id,
      { kind: "git", url: "https://github.com/corp/private-skills" },
    );
    const row = report.skills[0].status;
    if (row.status !== "failed") throw new Error(row.status);
    expect(zod.dto.health).toEqual({ kind: "repoint_failed", error: row.error, at: NOW });
  });

  it("an Update's Edit conflict records edit_conflict; saving the Edit clears it", async () => {
    const { handlers, state } = backendFor("failures");
    const tokens = state.skills.find((s) => s.dto.name === "design-tokens")!;
    const first = await handlers.updateManagedSkill(tokens.dto.id, sink().channel);
    expect(first.skills.find((s) => s.id === tokens.dto.id)!.health).toEqual({
      kind: "edit_conflict",
      error: { code: "EDIT_CONFLICT", baseMode: "user-and-model", upstreamMode: "model-only", overrideMode: "user-only" },
      at: NOW,
    });
    // An unchanged Update keeps the unresolved conflict.
    await handlers.updateManagedSkill(tokens.dto.id, sink().channel);
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
    // One folder, one row: Kimi Code reads Amp's.
    expect(zod.dto.targets.find((t) => t.tool === "amp")).toMatchObject({ status: "error", synced_at: null });
    expect(zod.dto.targets.some((t) => t.tool === "kimi_cli")).toBe(false);

    const asked = await handlers.syncSkillsToTools(batch, ["cursor"], { overwrite_if_same_content: true }, sink().channel);
    expect(asked[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS" } });
    expect(zod.dto.targets.some((t) => t.tool === "cursor")).toBe(false);

    const kept = await handlers.syncSkillsToTools(
      batch,
      ["cursor"],
      { overwrite_if_same_content: true, overrides: [{ skill_id: zod.dto.id, tool: "cursor", path: `${globalRoot("cursor")}/${zod.dto.name}`, decision: "keep" }] },
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

describe("ticket 16: the Add gate's repo — four candidates, two collisions, one transient failure", () => {
  it("release-notes fails once (rate limited) and a retry of it alone lands", async () => {
    const { handlers } = backendFor("failures");
    const listing = await handlers.listGitSkillsCmd("https://github.com/acme/collide-kit", null);
    expect(listing.candidates.map((c) => c.name)).toEqual([
      "lint-staged-rules",
      "commit-helper",
      "changelog-writer",
      "release-notes",
    ]);
    const request = (subpaths: string[]) => ({
      source: { kind: "git" as const, url: "https://github.com/acme/collide-kit", resolution: null },
      candidates: subpaths.map((subpath) => ({ subpath, name_override: null })),
      tools: ["claude_code"],
      overwrite_if_same_content: true,
      overrides: [],
      deploy: true,
    });
    const first = await handlers.installSelection(request(listing.candidates.map((c) => c.subpath)), sink().channel);
    expect(first.candidates.map((c) => [c.name, c.status.status])).toEqual([
      ["lint-staged-rules", "installed"],
      ["commit-helper", "installed"],
      ["changelog-writer", "installed"],
      ["release-notes", "failed"],
    ]);
    expect(first.candidates[3].status).toEqual({ status: "failed", error: { code: "RATE_LIMITED", resetMinutes: 12 } });
    const retry = await handlers.installSelection(request(["skills/release-notes"]), sink().channel);
    expect(retry.candidates.map((c) => [c.name, c.status.status])).toEqual([["release-notes", "installed"]]);
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
          { skill_id: lint.id, tool: "claude_code", path: `${globalRoot("claude_code")}/${lint.name}`, decision: "overwrite" },
          { skill_id: changelog.id, tool: "claude_code", path: `${globalRoot("claude_code")}/${changelog.name}`, decision: "keep" },
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

  it("Enable's collision is answered by the same scope with the decision", async () => {
    const { handlers, state } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const scope = { kind: "skill_tool" as const, skill_id: zod.id, tool: "cursor" };
    await handlers.disableSkillTargets(scope);
    const first = await handlers.enableSkillTargets(scope, []);
    expect(first.global[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS", sameContent: false } });
    expect(statusOf(await byName(handlers, "zod-schema-first"), "cursor")).toBe("disabled");

    const retried = await handlers.enableSkillTargets(scope, [
      { skill_id: zod.id, tool: "cursor", path: `${globalRoot("cursor")}/${zod.name}`, decision: "overwrite" },
    ]);
    expect(retried.global.map((o) => o.status.status)).toEqual(["synced"]);
    expect(state.foreign.some((f) => f.tool === "cursor" && f.name === "zod-schema-first")).toBe(false);
  });

  it("an Enable answer applies only at the path its ask showed: the skill renamed since, it touches nothing", async () => {
    for (const decision of ["overwrite", "keep"] as const) {
      const { handlers, state } = backendFor("rich");
      const zod = state.skills.find((s) => s.dto.name === "zod-schema-first")!;
      const scope = { kind: "skill_tool" as const, skill_id: zod.dto.id, tool: "cursor" };
      await handlers.disableSkillTargets(scope);
      const first = await handlers.enableSkillTargets(scope, []);
      const asked = `${globalRoot("cursor")}/zod-schema-first`;
      expect(first.global[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS", path: asked } });

      zod.dto.name = "zod-renamed";
      const retried = await handlers.enableSkillTargets(scope, [
        { skill_id: zod.dto.id, tool: "cursor", path: asked, decision },
      ]);

      expect(retried.global[0].status).toEqual({ status: "failed", error: { code: "NOT_FOUND", kind: "target", id: asked } });
      // The taken row is restored as it was: still disabled at the asked path.
      expect(zod.dto.targets.filter((t) => t.tool === "cursor")).toEqual([
        expect.objectContaining({ status: "disabled", target_path: asked, synced_at: null }),
      ]);
      expect(state.foreign.some((f) => f.tool === "cursor" && f.name === "zod-schema-first")).toBe(true);
    }
  });

  it("a project answer applies only at the path its ask showed: the project moved since, it touches nothing", async () => {
    for (const decision of ["overwrite", "keep"] as const) {
      const { handlers, state } = backendFor("failures");
      const project = state.projects.find((p) => p.id === "prj-quartermaster")!;
      const prisma = await byName(handlers, "prisma-review");
      const path = `${project.path}/.claude/skills/prisma-review`;
      state.projectOccupants.push(path);
      const toggled = await handlers.toggleProjectSkillAssignment(project.id, prisma.id, "claude_code");
      if (toggled.kind !== "assigned") throw new Error(toggled.kind);
      expect(toggled.report.items[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS", path } });
      const row = project.assignments.find((a) => a.skill_id === prisma.id && a.tool === "claude_code")!;
      const before = { ...row };

      project.path = `${project.path}-moved`;
      const resolved = await handlers.resolveProjectCollisions([
        { project_id: project.id, skill_id: prisma.id, tool: "claude_code", path, decision },
      ]);

      expect(resolved.items[0].status).toEqual({ status: "failed", error: { code: "NOT_FOUND", kind: "target", id: path } });
      expect(row).toEqual(before);
      expect(state.projectOccupants).toContain(path);
    }
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
        { project_id: project.id, skill_id: prisma.id, tool: "claude_code", path, decision },
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
        { project_id: project.id, skill_id: zod.id, tool: "claude_code", path, decision },
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

describe("ticket 18: ownership means the current occupant (ADR-0007)", () => {
  it("a folder that took an owned assignment's path is asked about on resync and left in place by Delete", async () => {
    const { handlers, state } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const { project } = await handlers.registerProject("/Users/alex/Projects/occupant-probe");
    await handlers.configureProjectTools(project.id, ["claude_code"], null);
    await handlers.bulkAssignSkill(project.id, zod.id);
    const path = `${project.path}/.claude/skills/${zod.name}`;
    state.projectOccupants.push(path);

    const resync = await handlers.resyncProject(project.id);
    expect(resync.report.items[0].status).toMatchObject({ status: "failed", error: { code: "TARGET_EXISTS" } });
    expect(state.projectOccupants).toContain(path);

    const deleted = await handlers.deleteManagedSkill(zod.id);
    const left = deleted.targets.find((target) => target.path === path);
    expect(left?.status).toEqual({ status: "left_in_place" });
    expect(deleted.record_deleted).toBe(true);
    expect(state.projectOccupants, "the operator's folder survives").toContain(path);
  });

  it("a successful Disable gives the path up: a folder appearing later is never Delete's", async () => {
    const { handlers, state } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const { project } = await handlers.registerProject("/Users/alex/Projects/disable-probe");
    await handlers.configureProjectTools(project.id, ["claude_code"], null);
    await handlers.bulkAssignSkill(project.id, zod.id);
    await handlers.disableSkillTargets({ kind: "skill", skill_id: zod.id });
    const row = state.projects
      .find((p) => p.id === project.id)!
      .assignments.find((a) => a.skill_id === zod.id)!;
    expect(row.synced_at).toBeNull();
    const path = `${project.path}/.claude/skills/${zod.name}`;
    state.projectOccupants.push(path);

    const deleted = await handlers.deleteManagedSkill(zod.id);

    expect(deleted.targets.some((target) => target.path === path)).toBe(false);
    expect(state.projectOccupants).toContain(path);
  });

  // Round 20 t18 review: the fixture mirrors the core's evidence model (Sol
  // 104), the link shortcut after the occupant check (Astra 1 = Sol 102) and
  // the path-bound answer to a Propagation ask (Sol 101).

  /** The stored global row of `skillId` keyed `tool` (internal evidence included). */
  const storedRow = (state: FixtureState, skillId: string, tool: string) =>
    state.skills.find((s) => s.dto.id === skillId)!.dto.targets.find((t) => t.tool === tool)!;
  const globalOutcome = (report: RefreshReport, tool: string) => {
    const status = report.skills[0].status;
    if (status.status !== "refreshed") throw new Error(`not refreshed: ${JSON.stringify(status)}`);
    return status.targets.find((o) => o.scope.scope === "global" && o.scope.tool === tool)?.status;
  };
  /** Deep Agents Code (copy-only) installed, with a copy of `skill` the app deployed there. */
  const withDeployedCopy = (state: FixtureState, skillId: string) => {
    state.installedTools.push("deep_agents_code");
    const skill = state.skills.find((s) => s.dto.id === skillId)!;
    const path = `${globalRoot("deep_agents_code")}/${skill.dto.name}`;
    skill.dto.targets.push({
      tool: "deep_agents_code",
      mode: "copy",
      status: "synced",
      target_path: path,
      synced_at: NOW - 1000,
      deployed_hash: skill.contentHash,
    });
    return path;
  };

  it("the evidence a row deployed never reaches the wire", async () => {
    const { handlers, state } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    withDeployedCopy(state, zod.id);
    const listed = (await byName(handlers, "zod-schema-first")).targets;
    expect(listed.some((t) => "deployed_hash" in t)).toBe(false);
  });

  it("a hashless global copy (recorded before 1.4.0) is left in place by Delete and by Disable", async () => {
    for (const verb of ["delete", "disable"] as const) {
      const { handlers, state } = backendFor("rich");
      const zod = await byName(handlers, "zod-schema-first");
      const path = withDeployedCopy(state, zod.id);
      storedRow(state, zod.id, "deep_agents_code").deployed_hash = null;

      const report =
        verb === "delete"
          ? await handlers.deleteManagedSkill(zod.id)
          : await handlers.disableSkillTargets({ kind: "skill_tool", skill_id: zod.id, tool: "deep_agents_code" });

      expect(report.targets.find((t) => t.path === path)?.status, verb).toEqual({ status: "left_in_place" });
      // The folder stays where it was: from now on it is a folder in the way.
      expect(state.foreign.some((f) => `${globalRoot(f.tool)}/${f.name}` === path), verb).toBe(true);
      if (verb === "disable") {
        const row = storedRow(state, zod.id, "deep_agents_code");
        expect(row).toMatchObject({ status: "disabled", synced_at: null, deployed_hash: null });
      }
    }
  });

  it("a copy edited in place is asked about by Propagation; Overwrite records evidence and the next Propagation does not ask", async () => {
    const { handlers, state } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const path = withDeployedCopy(state, zod.id);
    state.copyIdentities[path] = "the operator's edits";

    const first = (await handlers.updateManagedSkill(zod.id, sink().channel)).report;
    expect(globalOutcome(first, "deep_agents_code")).toMatchObject({
      status: "failed",
      error: { code: "TARGET_EXISTS", path, sameContent: false },
    });
    expect(state.copyIdentities[path]).toBe("the operator's edits");

    const answer = await handlers.resolveTargetCollisions([
      { skill_id: zod.id, tool: "deep_agents_code", path, decision: "overwrite" },
    ]);
    expect(answer[0].status).toMatchObject({ status: "synced", outcome: { mode_used: "copy", target_path: path } });
    const skill = state.skills.find((s) => s.dto.id === zod.id)!;
    expect(storedRow(state, zod.id, "deep_agents_code")).toMatchObject({ mode: "copy", deployed_hash: skill.contentHash });
    expect(state.copyIdentities[path]).toBeUndefined();

    const again = (await handlers.updateManagedSkill(zod.id, sink().channel)).report;
    expect(globalOutcome(again, "deep_agents_code")).toEqual({ status: "synced", mode_used: "copy" });
  });

  it("an owned link replaced by a folder is asked about on Update, never followed as the source", async () => {
    const { handlers, state } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const path = `${globalRoot("claude_code")}/${zod.name}`;
    state.foreign.push({ tool: "claude_code", name: zod.name, fingerprint: "the operator's folder", linkTarget: null });

    const { report } = await handlers.updateManagedSkill(zod.id, sink().channel);

    expect(globalOutcome(report, "claude_code")).toMatchObject({
      status: "failed",
      error: { code: "TARGET_EXISTS", path },
      first_deploy: false,
    });
    // Its untouched siblings still follow the source.
    expect(globalOutcome(report, "codex")).toEqual({ status: "skipped", reason: { reason: "link_follows_source" } });
  });

  it("a Propagation answer applies at the asked path only; an answer for a path the row no longer records touches nothing", async () => {
    const { handlers, state } = backendFor("rich");
    const zod = await byName(handlers, "zod-schema-first");
    const row = storedRow(state, zod.id, "claude_code");
    // The row records an older name; the operator's folders sit there and at
    // the skill's place now.
    const recorded = `${globalRoot("claude_code")}/zod-old-name`;
    const destination = `${globalRoot("claude_code")}/${zod.name}`;
    row.target_path = recorded;
    state.foreign.push(
      { tool: "claude_code", name: "zod-old-name", fingerprint: "old", linkTarget: null },
      { tool: "claude_code", name: zod.name, fingerprint: "new", linkTarget: null },
    );

    const stale = await handlers.resolveTargetCollisions([
      { skill_id: zod.id, tool: "claude_code", path: destination, decision: "overwrite" },
    ]);
    expect(stale[0].status).toEqual({ status: "failed", error: { code: "NOT_FOUND", kind: "target", id: destination } });

    const answered = await handlers.resolveTargetCollisions([
      { skill_id: zod.id, tool: "claude_code", path: recorded, decision: "overwrite" },
    ]);
    expect(answered[0].status).toMatchObject({ status: "synced", outcome: { target_path: recorded } });
    const left = state.foreign.map((f) => `${globalRoot(f.tool)}/${f.name}`);
    expect(left).toContain(destination);
    expect(left).not.toContain(recorded);
    expect(storedRow(state, zod.id, "claude_code").target_path).toBe(recorded);

    const kept = await handlers.resolveTargetCollisions([
      { skill_id: zod.id, tool: "codex", path: `${globalRoot("codex")}/${zod.name}`, decision: "keep" },
    ]);
    // The codex row's own link is still there: nothing to answer, it is brought into line.
    expect(kept[0].status).toMatchObject({ status: "synced" });
  });
});

describe("ticket 17: Projects — a missing folder, Sync this, the kept row, a stale view", () => {
  it("Sync All skips archived-app (folder missing) as one project entry, its rows untouched", async () => {
    const { handlers, state } = backendFor("failures");
    const archived = state.projects.find((p) => p.id === "prj-archived-app")!;
    const before = JSON.stringify(archived.assignments);
    const { report } = await handlers.resyncAllProjects();
    expect(report.skipped_projects).toEqual([{ project_id: "prj-archived-app", reason: { reason: "folder_missing" } }]);
    expect(report.items.some((item) => item.project_id === "prj-archived-app")).toBe(false);
    expect(JSON.stringify(archived.assignments)).toBe(before);
    expect((await handlers.resyncProject("prj-archived-app")).report).toEqual({
      items: [],
      skipped_projects: [{ project_id: "prj-archived-app", reason: { reason: "folder_missing" } }],
    });
  });

  it("no project sync enters archived-app: assign, toggle-on, Enable and the collision retry skip it once, rows untouched", async () => {
    const { handlers, state } = backendFor("failures");
    const archived = state.projects.find((p) => p.id === "prj-archived-app")!;
    const skip = [{ project_id: "prj-archived-app", reason: { reason: "folder_missing" } }];
    const notes = state.skills.find((s) => s.dto.name === "local-notes")!.dto.id;
    const before = JSON.stringify(archived.assignments);

    expect((await handlers.bulkAssignSkill("prj-archived-app", notes)).report).toEqual({ items: [], skipped_projects: skip });
    const toggled = await handlers.toggleProjectSkillAssignment("prj-archived-app", notes, "claude_code");
    expect(toggled).toMatchObject({ kind: "assigned", report: { items: [], skipped_projects: skip } });
    expect(JSON.stringify(archived.assignments)).toBe(before);

    // Enable: the skill's disabled row there is neither admitted nor synced.
    const tdd = archived.assignments.find((a) => a.skill_name === "tdd")!;
    await handlers.disableSkillTargets({ kind: "skill", skill_id: tdd.skill_id });
    const disabledRows = JSON.stringify(archived.assignments);
    const enabled = await handlers.enableSkillTargets({ kind: "skills", skill_ids: [tdd.skill_id, tdd.skill_id] }, []);
    expect(enabled.projects.skipped_projects).toEqual(skip);
    expect(enabled.projects.items.some((item) => item.project_id === "prj-archived-app")).toBe(false);
    expect(JSON.stringify(archived.assignments)).toBe(disabledRows);

    // The collision retry: a folder gone between the ask and the answer.
    const row = archived.assignments.find((a) => a.skill_name === "pr-review")!;
    Object.assign(row, { status: "error", synced_at: null });
    const asked = JSON.stringify(archived.assignments);
    for (const decision of ["overwrite", "keep"] as const) {
      const resolved = await handlers.resolveProjectCollisions([
        { project_id: "prj-archived-app", skill_id: row.skill_id, tool: "claude_code", path: `${archived.path}/.claude/skills/pr-review`, decision },
      ]);
      expect(resolved).toEqual({ items: [], skipped_projects: skip });
      expect(JSON.stringify(archived.assignments)).toBe(asked);
    }
  });

  it("Sync this re-syncs one cell and answers the view", async () => {
    const { handlers } = backendFor("failures");
    const view = await handlers.getProjectView("prj-monorepo");
    const stale = view.assignments.find((a) => a.status === "stale")!;
    const { view: after, report } = await handlers.resyncProjectAssignment("prj-monorepo", stale.skill_id, stale.tool);
    expect(report.items.map((item) => [item.skill_id, item.tool, item.status.status])).toEqual([[stale.skill_id, stale.tool, "synced"]]);
    expect(after.assignments.find((a) => a.id === stale.id)?.status).toBe("synced");
    await expect(handlers.resyncProjectAssignment("prj-monorepo", stale.skill_id, "pi")).rejects.toMatchObject({ code: "NOT_FOUND" });
  });

  it("removing monorepo keeps the locked .agents/skills copy (ADR-0002): the project stays listed", async () => {
    const { handlers } = backendFor("failures");
    const { projects, report } = await handlers.removeProject("prj-monorepo");
    expect(report.targets.filter((target) => target.status.status === "failed").length).toBeGreaterThan(0);
    expect(projects.some((p) => p.id === "prj-monorepo")).toBe(true);
    const view = await handlers.getProjectView("prj-monorepo");
    // The kept row reads failed with its reason; every row that owned a removable copy is gone.
    expect(view.assignments.find((a) => a.skill_name === "release-train" && a.tool === "agents_skills")).toMatchObject({ status: "error" });
    expect(view.assignments.some((a) => a.status === "synced" || a.status === "stale")).toBe(false);
  });

  it("quartermaster's first view is unreconciled once; the next read is reconciled", async () => {
    const { handlers } = backendFor("failures");
    expect((await handlers.getProjectView("prj-quartermaster")).reconciled).toBe(false);
    expect((await handlers.getProjectView("prj-quartermaster")).reconciled).toBe(true);
  });
});

// Round 20 ticket 02: the registry scenario — detected and undetected
// harnesses, a reader-only member, a row without a project dir, and the
// cross-read pairs (Rust `catalog::duplicate_reads`).
describe("registry", () => {
  it("shows detected and undetected harnesses, a reader-only member and a row without a project dir", async () => {
    const { handlers } = backendFor("registry");
    const status = await handlers.getToolStatus();
    expect(status.tools.some((tool) => tool.installed)).toBe(true);
    expect(status.tools.some((tool) => !tool.installed)).toBe(true);
    const agents = status.tools.find((tool) => tool.key === "agents_skills")!;
    expect(agents.readers.length).toBeGreaterThan(0);
    expect(status.tools.some((tool) => tool.key === "workbuddy" && tool.installed)).toBe(true);
    const project = await handlers.getProjectToolStatus();
    expect(project.tools.some((tool) => tool.key === "workbuddy")).toBe(false);
  });

  it("answers Cursor's cross-read pairs with both sides selected", async () => {
    const { handlers, state } = backendFor("registry");
    const status = await handlers.getToolStatus();
    const cursor = status.tools.find((tool) => tool.key === "cursor")!;
    // `.agents/skills` is detected, so it is a pair too; it is not selected,
    // so the pane warns about Claude Code and Codex only until it is turned on.
    expect(cursor.duplicate_readers).toEqual({
      keys: ["agents_skills", "claude_code", "codex"],
      behaviour: "shows_all",
    });
    const selected = state.settings.global_selected_tools ?? [];
    expect(selected).toEqual(expect.arrayContaining(["cursor", "claude_code", "codex"]));
    expect(selected).not.toContain("agents_skills");
  });

  // round 20 ticket 05 rows: a detected reader-only harness carries its pairs
  // on its reader entry; an undetected one carries none.
  it("answers fx's pairs on its reader entry, and none for an undetected reader", async () => {
    const { handlers } = backendFor("registry");
    const status = await handlers.getToolStatus();
    const readers = status.tools.find((tool) => tool.key === "agents_skills")!.readers;
    expect(readers.find((reader) => reader.key === "fx")?.duplicate_readers).toEqual({
      keys: ["claude_code", "codex", "opencode"],
      behaviour: "loads_none",
    });
    expect(readers.find((reader) => reader.key === "warp")?.duplicate_readers).toBeNull();
    expect(status.tools.find((tool) => tool.key === "codewhale")?.duplicate_readers).toEqual({
      keys: ["agents_skills", "claude_code"],
      behaviour: "shadows",
    });
  });
});

describe("fixture parity with Rust (#90)", () => {
  // Rust `catalog::duplicate_reads`: only installed harnesses, both sides;
  // never a shared skills dir sibling.
  it("cross-read pairs name installed harnesses only, never a shared-dir sibling", async () => {
    const { handlers, state } = backendFor("rich");
    let status = await handlers.getToolStatus();
    const of = (key: string) => status.tools.find((tool) => tool.key === key)!.duplicate_readers;
    expect(of("cursor")).toEqual({ keys: ["claude_code", "codex"], behaviour: "shows_all" });
    expect(of("amp")).toEqual({ keys: ["claude_code"], behaviour: "shadows" });
    expect(of("kimi_cli")).toBeNull();
    expect(of("opencode")).toBeNull();
    expect(of("claude_code")).toBeNull();

    state.installedTools = state.installedTools.filter((key) => key !== "claude_code");
    state.installedTools.push("crush", "agents_skills");
    status = await handlers.getToolStatus();
    expect(of("cursor")).toEqual({ keys: ["agents_skills", "codex"], behaviour: "shows_all" });
    expect(of("crush")).toEqual({ keys: ["agents_skills", "amp", "kimi_cli"], behaviour: "unknown" });
    expect(status.tools.every((tool) => tool.duplicate_readers?.keys.every((key) => key !== tool.key) ?? true)).toBe(true);
  });

  it("the project list carries no cross-read pairs", async () => {
    const { handlers } = backendFor("rich");
    expect((await handlers.getProjectToolStatus()).tools.every((tool) => tool.duplicate_readers === null)).toBe(true);
  });

  it("the onboarding plan reports every harness on a shared skills dir, one variant each (Rust has no path dedupe)", async () => {
    const { handlers } = backendFor("first-run");
    const plan = await handlers.getOnboardingPlan();
    const ego = plan.groups.find((g) => g.name === "ego-browser")!;
    const shared = `${globalRoot("amp")}/ego-browser`;
    expect(globalRoot("kimi_cli")).toBe(globalRoot("amp"));
    expect(ego.variants.filter((v) => v.path === shared).map((v) => v.tool)).toEqual(["amp", "kimi_cli"]);
    expect(plan.copy_count).toBe(plan.groups.reduce((n, g) => n + g.variants.length, 0));
  });

  it("removing a conflict's other copies settles a shared path once and reports it per harness (Rust `remove_divergent_copies`)", async () => {
    const { state, handlers } = backendFor("first-run");
    state.foreign.push({ tool: "amp", name: "handoff", fingerprint: "a fork in the shared dir", linkTarget: null });
    const group = (await handlers.getOnboardingPlan()).groups.find((g) => g.name === "handoff")!;
    const chosen = group.variants.find((v) => v.tool === "claude_code")!;
    const report = await handlers.importOnboardingSelection(
      [{ group_name: "handoff", chosen_path: chosen.path, name: null, remove_divergent: true }],
      { auto_sync: false, tools: null, remove_originals: false },
      sink().channel,
    );
    const status = report.groups[0].status;
    if (status.status !== "imported") throw new Error(`expected imported, got ${status.status}`);
    const removed = status.originals.filter((o) => o.status.status === "removed").map((o) => [o.tool, o.path]);
    expect(removed).toEqual([
      ["amp", `${globalRoot("amp")}/handoff`],
      ["kimi_cli", `${globalRoot("amp")}/handoff`],
      ["pi", `${globalRoot("pi")}/handoff`],
    ]);
    expect(state.foreign.some((f) => f.name === "handoff" && f.tool !== "claude_code")).toBe(false);
  });

  it("resolveProjectCollisions answers a missing project as a per-decision NOT_FOUND row, never a throw", async () => {
    const { handlers } = backendFor("rich");
    const report = await handlers.resolveProjectCollisions([
      { project_id: "prj-gone", skill_id: "s-any", tool: "claude_code", path: "/gone/.claude/skills/s-any", decision: "overwrite" },
    ]);
    expect(report).toEqual({
      skipped_projects: [],
      items: [
        {
          assignment_id: null,
          project_id: "prj-gone",
          skill_id: "s-any",
          skill_name: "",
          tool: "claude_code",
          folder_key: "claude_code",
          status: { status: "failed", error: { code: "NOT_FOUND", kind: "assignment", id: "prj-gone/s-any/claude_code" } },
        },
      ],
    });
  });
});

describe("one Deployment row per folder (round 20 ticket 09)", () => {
  it("saves a harness selection as folders: Kimi Code turns on Amp's folder, and both read on", async () => {
    const { handlers } = backendFor("rich");
    const saved = await handlers.updateSetting({
      key: "global_tool_config",
      value: { selected_tools: ["kimi_cli", "claude_code"], scan_selected_only: false },
    });
    expect(saved.global_selected_tools).toEqual(["amp", "kimi_cli", "claude_code"]);
  });

  it("assigns two Tools sharing a project folder as one row, keyed by the folder", async () => {
    const { handlers } = backendFor("rich");
    const pdf = (await handlers.getManagedSkills()).find((s) => s.name === "pdf")!;
    const { view } = await handlers.configureProjectTools("prj-quartermaster", ["trae", "trae_cn"], null);
    expect(view.tools.map((tool) => [tool.tool, tool.folder_key])).toEqual([
      ["trae", "trae"],
      ["trae_cn", "trae"],
    ]);
    const { report, view: assigned } = await handlers.bulkAssignSkill("prj-quartermaster", pdf.id);
    expect(report.items.map((item) => item.tool)).toEqual(["trae"]);
    expect(assigned.assignments.filter((a) => a.skill_id === pdf.id).map((a) => a.tool)).toEqual(["trae"]);

    // Dropping one of the two keeps the folder's Deployment.
    const { view: kept, report: removed } = await handlers.configureProjectTools("prj-quartermaster", ["trae"], null);
    expect(removed.targets).toEqual([]);
    expect(kept.assignments.filter((a) => a.skill_id === pdf.id).map((a) => a.tool)).toEqual(["trae"]);

    // Toggling the sibling's cell takes the folder's row off.
    const off = await handlers.toggleProjectSkillAssignment("prj-quartermaster", pdf.id, "trae_cn");
    expect(off.kind).toBe("unassigned");
    expect(off.view.assignments.some((a) => a.skill_id === pdf.id)).toBe(false);
  });
});

// Round 20 t09 review round 4 (Sol 305, Astra 204): the fixture's project
// count and total read the listing's own fold, and a folder's facts never let
// a disabled sibling hide a failure or a live copy.
describe("legacy two-row folders read through the one fold (round 4)", () => {
  const fixture = (statuses: [string, string], owned: [boolean, boolean]) => {
    const state = buildScenario("rich", NOW);
    const { handlers } = createFixtureBackend(state, { latencyScale: 0, now: () => NOW });
    const skill = state.skills[0];
    const project: FixtureProject = {
      id: "prj-trae",
      name: "trae-app",
      path: "/Users/alex/code/trae-app",
      created_at: NOW,
      updated_at: NOW,
      path_exists: true,
      tools: ["trae", "trae_cn"],
      gitignore: { in_gitignore: false, in_exclude: false },
      // A pre-1.4.0 project wrote one row per sibling of `.trae/skills`.
      assignments: (["trae", "trae_cn"] as const).map((tool, i) => ({
        id: `prj-trae-${tool}`,
        project_id: "prj-trae",
        skill_id: skill.dto.id,
        skill_name: skill.dto.name,
        tool,
        mode: "symlink" as const,
        status: statuses[i] as FixtureProject["assignments"][number]["status"],
        last_error: null,
        synced_at: owned[i] ? NOW : null,
        content_hash: null,
        created_at: NOW,
      })),
    };
    state.projects.push(project);
    const listed = async () => (await handlers.listProjects()).find((p) => p.id === "prj-trae")!;
    return { handlers, skill, listed };
  };

  it("a live link beside a disabled row: one Deployment, matrix disabled, total still synced", async () => {
    const { handlers, listed } = fixture(["synced", "disabled"], [true, true]);
    const view = await handlers.getProjectView("prj-trae");
    expect(view.assignments).toHaveLength(1);
    expect(view.assignments[0]).toMatchObject({ tool: "trae", status: "disabled", disabled: true, deployed: true });
    expect(await listed()).toMatchObject({ assignment_count: 1, sync_status: "synced" });
  });

  it("a failed removal beside a disabled row shows the error in the matrix and the total", async () => {
    const { handlers, listed } = fixture(["error", "disabled"], [true, true]);
    const view = await handlers.getProjectView("prj-trae");
    expect(view.assignments[0]).toMatchObject({ status: "error", disabled: true, deployed: true });
    expect(await listed()).toMatchObject({ assignment_count: 1, sync_status: "error" });
  });

  it("the library's project deployments carry the same facts", async () => {
    const { handlers, skill } = fixture(["error", "disabled"], [true, true]);
    const listedSkill = (await handlers.getManagedSkills()).find((s) => s.id === skill.dto.id)!;
    const trae = listedSkill.projects.find((p) => p.project_id === "prj-trae")!;
    expect(trae.deployments).toEqual([
      { tool: "trae", status: "error", synced_at: NOW, disabled: true, deployed: true },
    ]);
  });

  it("a global folder whose removal failed beside a disabled legacy row shows the failure", async () => {
    const { handlers, skill } = fixture(["synced", "synced"], [true, true]);
    skill.dto.targets = [
      { tool: "amp", mode: "symlink", status: "error", target_path: "/a", synced_at: NOW },
      { tool: "kimi_cli", mode: "symlink", status: "disabled", target_path: "/a", synced_at: NOW },
    ];
    const listedSkill = (await handlers.getManagedSkills()).find((s) => s.id === skill.dto.id)!;
    expect(listedSkill.targets.filter((t) => t.tool === "amp" || t.tool === "kimi_cli")).toEqual([
      expect.objectContaining({ tool: "amp", status: "error", disabled: true, deployed: true }),
    ]);
  });
});
