// Tests at the skill action door: one parametrised suite over verbs. The
// door is composed the way App composes it — the real reporter (lifecycle +
// Outcome execution), the real surface module and skills world — with the
// backend at the invokeTauri seam, the sync world and the project world as
// stubbed dependency interfaces. Assertions read the fold call, the command,
// the reload rule and the dialog layer; the toast is the reporter's own test.

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AppSettings,
  BatchTargetOutcome,
  EnableReport,
  ManagedSkill,
  RefreshReport,
  RemovalReport,
  SkillMutationResultDto,
  ToolStatusDto,
} from "../components/skills/types";
import type { DisableScope, PropagationOutcome } from "../bindings";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));
vi.mock("../lib/platform", () => ({
  createProgressChannel: async (onMessage: (message: unknown) => void) => ({ onmessage: onMessage }),
  pickFolder: vi.fn(),
}));

import { invokeTauri, type CommandName } from "../lib/tauri";
import * as folds from "../lib/reportOutcome";
import { useSkillActions, RELOAD_ON_THROW, type SkillVerb } from "./useSkillActions";
import { useSkillLibrary } from "./useSkillLibrary";
import { useStatusReporter } from "./useStatusReporter";
import { useSurfaceState } from "./useSurfaceState";
import { useSyncOrchestration, type CollisionPart, type SyncOrchestration } from "./useSyncOrchestration";
import { useToolCatalog } from "./useToolCatalog";
import { skillHealth } from "../lib/skillPresentation";
import { rowActions } from "../lib/notificationRuns";

const mockInvoke = vi.mocked(
  invokeTauri as unknown as (command: CommandName, ...args: unknown[]) => Promise<unknown>,
);

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

function skill(id: string, name: string, targets: string[] = []): ManagedSkill {
  return {
    id,
    name,
    description: null,
    source_type: "local",
    source_ref: `/src/${name}`,
    imported_from_tool: null,
    central_path: `/hub/${name}`,
    created_at: 0,
    updated_at: 0,
    last_sync_at: null,
    status: "active",
    invocation_mode: "user-and-model",
    invocation_override: null,
    targets: targets.map((tool) => ({
      tool,
      mode: "symlink",
      status: "synced",
      target_path: `/tools/${tool}/${name}`,
      synced_at: 1,
    })),
    disabled_at: null,
    refreshable: true,
    unlocatable: null,
    detachable: false,
    health: null,
  };
}

const refreshed = (skills: ManagedSkill[]): RefreshReport => ({
  skills: skills.map((row) => ({
    skill_id: row.id,
    skill_name: row.name,
    status: {
      status: "refreshed",
      content_hash: null,
      source_revision: null,
      targets: [],
      reassert_error: null,
      edit_conflict: null,
    },
  })),
});
const EMPTY_REMOVAL: RemovalReport = {
  targets: [],
  central_removed: false,
  record_deleted: false,
  cleared_markers: [],
};
const EMPTY_ENABLE: EnableReport = { global: [], projects: { items: [] } };
const EMPTY_SYNC: BatchTargetOutcome[] = [];

function setup(options: { skills?: ManagedSkill[]; sharedDir?: boolean; effective?: string[] } = {}) {
  const skills = options.skills ?? [skill("s1", "alpha", ["claude"]), skill("s2", "beta")];
  const answer = (command: CommandName): unknown => {
    switch (command) {
      case "getManagedSkills":
        return skills;
      case "refreshManagedSkills":
        return refreshed(skills);
      case "updateManagedSkill":
      case "repointSkillSource":
        return { report: refreshed(skills.slice(0, 1)), skills } satisfies SkillMutationResultDto;
      case "setSkillInvocationOverride":
        return {
          report: { skill_id: skills[0].id, skill_name: skills[0].name, propagation: { targets: [] } },
          skills,
        };
      case "deleteManagedSkill":
        return { ...EMPTY_REMOVAL, central_removed: true, record_deleted: true };
      case "disableSkillTargets":
        return EMPTY_REMOVAL;
      default:
        return null;
    }
  };
  mockInvoke.mockImplementation((command) => Promise.resolve(answer(command)));
  const sync = {
    autoSyncEnabled: true,
    effectiveSyncTargetIds: options.effective ?? ["claude", "cursor"],
    requestSharedDirConfirmation: vi.fn(async () => options.sharedDir ?? true),
    syncSkillsToTools: vi.fn(async () => EMPTY_SYNC),
    // The sync world's Enable seam (one overwrite ask), a passthrough here.
    enableSkillTargets: vi.fn(async (scope: unknown) => {
      await mockInvoke("enableSkillTargets", scope, []);
      return EMPTY_ENABLE;
    }),
    // The non-asking collectors and the one ask over every part: a
    // passthrough that asks nothing (the real seam is composed below).
    collectSync: vi.fn(async () => EMPTY_SYNC),
    collectEnable: vi.fn(async (scope: unknown) => {
      await mockInvoke("enableSkillTargets", scope, []);
      return EMPTY_ENABLE;
    }),
    // SAFETY: a passthrough stub returns each part unchanged plus `declined`,
    // which is the seam's contract for an ask that raised nothing.
    settleCollisionParts: vi.fn(async (parts: readonly CollisionPart[]) => ({
      parts: parts.map((part) => ({ ...part, declined: [] })),
      retried: false,
      projectIds: [],
    })) as unknown as SyncOrchestration["settleCollisionParts"],
  };
  const projects = {
    assignToProject: vi.fn(async () => ({ items: [] })),
    unassignFromProject: vi.fn(async () => EMPTY_REMOVAL),
    refreshView: vi.fn(async () => true),
  };
  return { skills, sync, projects };
}

type Setup = ReturnType<typeof setup>;

function useDoor(s: Setup) {
  const surface = useSurfaceState();
  const reporter = useStatusReporter(t);
  const library = useSkillLibrary({ t, reporter, surface });
  const actions = useSkillActions({
    t,
    reporter,
    sync: s.sync,
    catalog: { toolLabelById: { claude: "CLAUDE", cursor: "CURSOR" } },
    surface,
    library,
    projects: s.projects,
    order: library.managedSkills.map((row) => row.id),
  });
  return { surface, reporter, library, actions };
}

async function render(s: Setup) {
  const rendered = renderHook(() => useDoor(s));
  await waitFor(() => expect(rendered.result.current.library.managedSkills).toEqual(s.skills));
  mockInvoke.mockClear();
  return rendered;
}

const commands = () => mockInvoke.mock.calls.map(([command]) => command);
const reloads = () => commands().filter((command) => command === "getManagedSkills").length;

beforeEach(() => {
  vi.clearAllMocks();
  vi.restoreAllMocks();
});
afterEach(cleanup);

type Row = {
  name: string;
  verb: SkillVerb;
  selection: "one" | "two" | "all";
  opts?: Record<string, unknown>;
  prepare?: (skills: ManagedSkill[]) => void;
  fold:
    | "syncOutcome"
    | "removalOutcome"
    | "enableOutcome"
    | "refreshOutcome"
    | "deleteOutcome"
    | "invocationEditOutcome"
    | "projectSyncOutcome";
  /** The backend command it reaches, or the seam it calls. */
  reaches: CommandName | "syncSkillsToTools" | "enableSkillTargets" | "assignToProject" | "unassignFromProject";
  /** Whether the catalog is re-read after a settled run (vs. replaced from the response). */
  reloadsOnSuccess: boolean;
};

const VERBS: Row[] = [
  { name: "sync to the effective targets", verb: "syncToTargets", selection: "one", opts: { confirmed: true }, fold: "syncOutcome", reaches: "syncSkillsToTools", reloadsOnSuccess: true },
  { name: "sync one pair (pill)", verb: "syncToTargets", selection: "one", opts: { tool: "cursor" }, fold: "syncOutcome", reaches: "syncSkillsToTools", reloadsOnSuccess: true },
  { name: "retry a failed first sync", verb: "retry", selection: "one", prepare: (s) => { s[0].targets[0] = { ...s[0].targets[0], status: "error", synced_at: null }; }, fold: "syncOutcome", reaches: "syncSkillsToTools", reloadsOnSuccess: true },
  { name: "disable a skill", verb: "disable", selection: "one", fold: "removalOutcome", reaches: "disableSkillTargets", reloadsOnSuccess: true },
  { name: "disable a pair", verb: "disable", selection: "one", opts: { tool: "claude" }, fold: "removalOutcome", reaches: "disableSkillTargets", reloadsOnSuccess: true },
  { name: "disable the library", verb: "disable", selection: "all", opts: { confirmed: true }, fold: "removalOutcome", reaches: "disableSkillTargets", reloadsOnSuccess: true },
  { name: "enable a skill", verb: "enable", selection: "one", fold: "enableOutcome", reaches: "enableSkillTargets", reloadsOnSuccess: true },
  { name: "update one", verb: "update", selection: "one", fold: "refreshOutcome", reaches: "updateManagedSkill", reloadsOnSuccess: false },
  { name: "refresh all", verb: "update", selection: "all", opts: { confirmed: true }, fold: "refreshOutcome", reaches: "refreshManagedSkills", reloadsOnSuccess: true },
  { name: "restore", verb: "restore", selection: "one", fold: "refreshOutcome", reaches: "updateManagedSkill", reloadsOnSuccess: false },
  { name: "re-point", verb: "repoint", selection: "one", opts: { target: { kind: "git", url: " https://github.com/o/r " } }, fold: "refreshOutcome", reaches: "repointSkillSource", reloadsOnSuccess: false },
  { name: "delete (confirmed)", verb: "delete", selection: "one", opts: { confirmed: true }, fold: "deleteOutcome", reaches: "deleteManagedSkill", reloadsOnSuccess: true },
  { name: "edit invocation", verb: "editInvocation", selection: "one", opts: { mode: "user-only" }, fold: "invocationEditOutcome", reaches: "setSkillInvocationOverride", reloadsOnSuccess: false },
  { name: "assign to a project", verb: "assignToProject", selection: "two", opts: { projectId: "p1" }, fold: "projectSyncOutcome", reaches: "assignToProject", reloadsOnSuccess: false },
  { name: "unassign from a project", verb: "unassignFromProject", selection: "two", opts: { projectId: "p1" }, fold: "removalOutcome", reaches: "unassignFromProject", reloadsOnSuccess: false },
  { name: "deploy a group", verb: "deployGroup", selection: "two", fold: "syncOutcome", reaches: "syncSkillsToTools", reloadsOnSuccess: true },
];

const pick = (row: Row, skills: ManagedSkill[]) =>
  row.selection === "all" ? ("all" as const) : row.selection === "two" ? skills : [skills[0]];

const reached = (s: Setup, reaches: Row["reaches"]) => {
  if (reaches === "syncSkillsToTools")
    return s.sync.syncSkillsToTools.mock.calls.length + s.sync.collectSync.mock.calls.length;
  if (reaches === "enableSkillTargets")
    return s.sync.enableSkillTargets.mock.calls.length + s.sync.collectEnable.mock.calls.length;
  if (reaches === "assignToProject") return s.projects.assignToProject.mock.calls.length;
  if (reaches === "unassignFromProject") return s.projects.unassignFromProject.mock.calls.length;
  return commands().filter((command) => command === reaches).length;
};

describe("the door, verb by verb", () => {
  it.each(VERBS)("$name: reaches the backend once, settles through its fold, keeps the reload rule", async (row) => {
    const s = setup();
    row.prepare?.(s.skills);
    const spy = vi.spyOn(folds, row.fold);
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run(row.verb, pick(row, s.skills), row.opts as never);
    });
    expect(spy).toHaveBeenCalled();
    expect(reached(s, row.reaches)).toBeGreaterThan(0);
    expect(reloads() > 0).toBe(row.reloadsOnSuccess);
    // The lifecycle ended: nothing pending, nothing refused.
    expect(result.current.reporter.pending).toBeNull();
  });

  it("acts on ids resolved against the catalog at click time, dropping unknown ids", async () => {
    const s = setup();
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("disable", ["s2", "gone"]);
    });
    expect(mockInvoke).toHaveBeenCalledWith("disableSkillTargets", { kind: "skill", skill_id: "s2" });
  });

  it("chooses one batch for a set and a batch of one for a single skill", async () => {
    const s = setup();
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("disable", s.skills, { confirmed: true });
    });
    expect(mockInvoke).toHaveBeenCalledWith("disableSkillTargets", { kind: "skills", skill_ids: ["s1", "s2"] });
    await act(async () => {
      await result.current.actions.run("update", s.skills, { confirmed: true });
    });
    expect(mockInvoke).toHaveBeenCalledWith("refreshManagedSkills", ["s1", "s2"], { reassert_auto_sync: true }, expect.anything());
    await act(async () => {
      await result.current.actions.run("update", "all", { confirmed: true });
    });
    expect(mockInvoke).toHaveBeenCalledWith("refreshManagedSkills", null, { reassert_auto_sync: true }, expect.anything());
  });

  it("trims a Re-point target and closes Change source when the fold says so", async () => {
    const s = setup();
    const { result } = await render(s);
    act(() => void result.current.actions.run("repoint", [s.skills[0]]));
    expect(result.current.surface.layer("repoint")).toMatchObject({ skillId: "s1", preselect: "local" });
    await act(async () => {
      await result.current.actions.run("repoint", [s.skills[0]], { target: { kind: "git", url: " https://github.com/o/r " } });
    });
    expect(mockInvoke).toHaveBeenCalledWith("repointSkillSource", "s1", { kind: "git", url: "https://github.com/o/r" }, { reassert_auto_sync: true });
    expect(result.current.surface.layer("repoint")).toBeNull();
  });

  it("answers Re-point's final refusal to its dialog, which stays open; a cancel answers nothing (I4)", async () => {
    const s = setup();
    const { result } = await render(s);
    act(() => void result.current.actions.run("repoint", [s.skills[0]]));
    const refusal = { code: "GIT_CLONE_FAILED", kind: "auth", detail: "403" };
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "repointSkillSource" ? Promise.reject(refusal) : base(command, ...args),
    );
    let answer: unknown;
    await act(async () => {
      answer = await result.current.actions.run("repoint", [s.skills[0]], { target: { kind: "git", url: "https://github.com/o/r" } });
    });
    expect(answer).toEqual(refusal);
    expect(result.current.surface.layer("repoint")).toMatchObject({ skillId: "s1" });
    expect(result.current.reporter.notifications).toEqual([]);
    expect(result.current.reporter.pending).toBeNull();

    mockInvoke.mockImplementation((command, ...args) =>
      command === "repointSkillSource" ? Promise.reject({ code: "CANCELLED" }) : base(command, ...args),
    );
    await act(async () => {
      answer = await result.current.actions.run("repoint", [s.skills[0]], { target: { kind: "git", url: "https://github.com/o/r" } });
    });
    expect(answer).toBeNull();
    expect(result.current.reporter.notifications).toEqual([]);
  });

  it("Re-point's dialog-opening and no-op exits answer null, never undefined (R2)", async () => {
    const s = setup();
    const { result } = await render(s);
    let opening: unknown = "unset";
    await act(async () => {
      opening = await result.current.actions.run("repoint", [s.skills[0]]);
    });
    expect(opening).toBeNull();
    let empty: unknown = "unset";
    await act(async () => {
      empty = await result.current.actions.run("repoint", [], { target: { kind: "git", url: "https://github.com/o/r" } });
    });
    expect(empty).toBeNull();
    let batch: unknown = "unset";
    await act(async () => {
      batch = await result.current.actions.run("repoint", s.skills.slice(0, 2), { target: { kind: "git", url: "https://github.com/o/r" } });
    });
    expect(batch).toBeNull();
  });

  it("dismisses an incomplete import through clearSkillHealth and reloads", async () => {
    const s = setup();
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("dismissHealth", [s.skills[0]], { kind: "import_failed" });
    });
    expect(mockInvoke).toHaveBeenCalledWith("clearSkillHealth", "s1", "import_failed");
    expect(reloads()).toBe(1);
  });

  it("deploys a group: Enable what is disabled, sync what never deployed, leave the deployed", async () => {
    const disabled = { ...skill("s3", "gamma"), disabled_at: 5 };
    const s = setup({ skills: [skill("s1", "alpha", ["claude"]), skill("s2", "beta"), disabled] });
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("deployGroup", s.skills);
    });
    expect(s.sync.collectEnable).toHaveBeenCalledWith({ kind: "skills", skill_ids: ["s3"] });
    expect(s.sync.collectSync).toHaveBeenCalledWith(
      [{ skill_id: "s2", name: "beta", source_path: "/hub/beta" }],
      ["claude", "cursor"],
      { overwriteIfSameContent: true },
    );
    // Both reports reach the one ask together.
    expect(s.sync.settleCollisionParts).toHaveBeenCalledTimes(1);
  });

  it("a mixed group with no sync targets enables, reports the undeployed part, and reloads once (S4)", async () => {
    const disabled = { ...skill("s3", "gamma"), disabled_at: 5 };
    const s = setup({ skills: [skill("s2", "beta"), disabled], effective: [] });
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("deployGroup", s.skills);
    });
    expect(s.sync.collectEnable).toHaveBeenCalledTimes(1);
    expect(s.sync.collectSync).not.toHaveBeenCalled();
    expect(reloads()).toBe(1);
    // One run block: the Enable's headline over the warning about the part
    // that had nowhere to go.
    const rows = result.current.reporter.notifications;
    expect(new Set(rows.map((row) => row.runId))).toEqual(new Set([rows[0]!.runId]));
    expect(rows).toContainEqual(
      expect.objectContaining({ kind: "warning", title: "noSyncTargets", message: 'lifecycle.deployNoTargets {"count":1}' }),
    );
    expect(result.current.reporter.pending).toBeNull();
  });
});

describe("where confirmation happens", () => {
  it.each([
    ["delete", {}, "deleteSkill"],
    ["editInvocation", {}, "invocationEdit"],
    ["repoint", { preselect: "git" }, "repoint"],
    ["detach", {}, "detachSkill"],
    ["assignToProject", {}, "assignProject"],
  ] as const)("%s without its confirming option opens its dialog and calls nothing", async (verb, opts, dialog) => {
    const s = setup();
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run(verb, [s.skills[0]], opts as never);
    });
    expect(result.current.surface.layer(dialog)).toMatchObject(
      dialog === "deleteSkill" || dialog === "assignProject" ? { skillIds: ["s1"] } : { skillId: "s1" },
    );
    expect(commands()).toEqual([]);
  });

  it.each([
    ["disable", "two"],
    ["enable", "two"],
    ["update", "two"],
    ["disable", "all"],
    ["update", "all"],
    ["syncToTargets", "one"],
    ["syncToTargets", "two"],
  ] as const)(
    "a wide %s (%s) without `confirmed` opens the count-bearing confirmation and calls nothing (ticket 13)",
    async (verb, selection) => {
      const s = setup();
      const { result } = await render(s);
      await act(async () => {
        await result.current.actions.run(verb, selection === "all" ? "all" : selection === "two" ? s.skills : [s.skills[0]]);
      });
      expect(result.current.surface.layer("confirmCount")).toEqual({
        kind: "confirmCount",
        verb,
        skillIds: selection === "one" ? ["s1"] : ["s1", "s2"],
      });
      expect(commands()).toEqual([]);
      expect(s.sync.syncSkillsToTools).not.toHaveBeenCalled();
      expect(s.sync.enableSkillTargets).not.toHaveBeenCalled();
    },
  );

  it("one skill's Disable / Enable / Update never asks (D8: reversible, the switch shows it)", async () => {
    const s = setup();
    const { result } = await render(s);
    for (const verb of ["disable", "enable", "update"] as const) {
      await act(async () => {
        await result.current.actions.run(verb, [s.skills[1]]);
      });
    }
    expect(result.current.surface.layer("confirmCount")).toBeNull();
    expect(commands()).toEqual(expect.arrayContaining(["disableSkillTargets", "enableSkillTargets", "updateManagedSkill"]));
  });

  it("the confirmed wide verb runs one batch and closes its count confirmation", async () => {
    const s = setup();
    const { result } = await render(s);
    act(() => void result.current.actions.run("disable", s.skills));
    expect(result.current.surface.isOpen("confirmCount")).toBe(true);
    await act(async () => {
      await result.current.actions.run("disable", s.skills, { confirmed: true });
    });
    expect(commands().filter((command) => command === "disableSkillTargets")).toHaveLength(1);
    expect(result.current.surface.isOpen("confirmCount")).toBe(false);
    act(() => void result.current.actions.run("syncToTargets", s.skills));
    await act(async () => {
      await result.current.actions.run("syncToTargets", s.skills, { tools: ["cursor"], confirmed: true });
    });
    expect(s.sync.syncSkillsToTools).toHaveBeenCalledTimes(1);
    expect(s.sync.syncSkillsToTools).toHaveBeenCalledWith(
      [
        { skill_id: "s1", name: "alpha", source_path: "/hub/alpha" },
        { skill_id: "s2", name: "beta", source_path: "/hub/beta" },
      ],
      ["cursor"],
      { overwriteIfSameContent: true },
    );
    expect(result.current.surface.isOpen("confirmCount")).toBe(false);
  });

  it("Assign to project… over a selection opens the picker for every selected skill", async () => {
    const s = setup();
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("assignToProject", s.skills);
    });
    expect(result.current.surface.layer("assignProject")).toEqual({ kind: "assignProject", skillIds: ["s1", "s2"] });
    await act(async () => {
      await result.current.actions.run("assignToProject", s.skills, { projectId: "p1" });
    });
    expect(s.projects.assignToProject).toHaveBeenCalledWith("p1", ["s1", "s2"]);
  });

  it.each(["syncToTargets", "disable", "enable"] as const)(
    "%s of one pair asks the shared-dir confirmation first; a no writes nothing",
    async (verb) => {
      const s = setup({ sharedDir: false });
      const { result } = await render(s);
      await act(async () => {
        await result.current.actions.run(verb, [s.skills[0]], { tool: "claude" });
      });
      expect(s.sync.requestSharedDirConfirmation).toHaveBeenCalledWith("claude");
      expect(commands()).toEqual([]);
      expect(s.sync.syncSkillsToTools).not.toHaveBeenCalled();
      expect(s.sync.enableSkillTargets).not.toHaveBeenCalled();
    },
  );

  it("a confirmed Detach runs the command, reloads and closes its confirmation", async () => {
    const s = setup();
    const { result } = await render(s);
    act(() => void result.current.actions.run("detach", [s.skills[0]]));
    expect(result.current.surface.layer("detachSkill")).toMatchObject({ skillId: "s1" });
    await act(async () => {
      await result.current.actions.run("detach", [s.skills[0]], { confirmed: true });
    });
    expect(commands()).toEqual(["detachSkillFromSource", "getManagedSkills"]);
    expect(result.current.surface.layer("detachSkill")).toBeNull();
  });

  it("an Assign picker's answer assigns through the project world", async () => {
    const s = setup();
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("assignToProject", [s.skills[0]], { projectId: "p1" });
    });
    expect(s.projects.assignToProject).toHaveBeenCalledWith("p1", ["s1"]);
  });

  it("a sync with no effective targets is reported, never silent", async () => {
    const s = setup({ effective: [] });
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("syncToTargets", [s.skills[1]], { confirmed: true });
    });
    expect(s.sync.syncSkillsToTools).not.toHaveBeenCalled();
    expect(result.current.reporter.notifications[0]).toMatchObject({ kind: "warning", title: "noSyncTargets" });
  });

  it("a completed Delete closes its dialog through the Outcome and picks the next row for focus", async () => {
    const s = setup();
    const { result } = await render(s);
    act(() => void result.current.actions.run("delete", [s.skills[0]]));
    await act(async () => {
      await result.current.actions.run("delete", [s.skills[0]], { confirmed: true });
    });
    expect(result.current.surface.layer("deleteSkill")).toBeNull();
    const row = document.createElement("button");
    row.dataset.skillRow = "s2";
    document.body.append(row);
    expect(result.current.actions.dialogFallbackFocus()).toBe(row);
    row.remove();
  });

  it("a bulk Remove deletes each skill under one action, folds one run, and a kept skill turns the dialog into the report (D16)", async () => {
    const s = setup({ skills: [skill("s1", "alpha"), skill("s2", "beta"), skill("s3", "gamma")] });
    const kept: RemovalReport = {
      targets: [
        {
          path: "/tools/claude/beta",
          rows: [{ scope: "global_target", id: "t", skill_id: "s2", tool: "claude" }],
          status: { status: "failed", error: { code: "OTHER", message: "Permission denied" } },
        },
      ],
      central_removed: false,
      record_deleted: false,
      cleared_markers: [],
    };
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "deleteManagedSkill" && args[0] === "s2") return Promise.resolve(kept);
      if (command === "deleteManagedSkill" && args[0] === "s3") return Promise.reject({ code: "OTHER", message: "store locked" });
      return base(command, ...args);
    });
    const spy = vi.spyOn(folds, "deleteOutcome");
    const { result } = await render(s);
    act(() => void result.current.actions.run("delete", s.skills));
    expect(result.current.surface.layer("deleteSkill")).toEqual({ kind: "deleteSkill", skillIds: ["s1", "s2", "s3"] });
    let running!: Promise<unknown>;
    act(() => {
      running = result.current.actions.run("delete", s.skills, { confirmed: true });
    });
    // Several skills pend on their rows; the dialog, not the progress surface, shows it.
    await waitFor(() => expect(result.current.reporter.pending?.subject).toEqual({ kind: "skills", ids: ["s1", "s2", "s3"] }));
    await act(async () => {
      await running;
    });
    expect(commands().filter((command) => command === "deleteManagedSkill")).toHaveLength(3);
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy.mock.calls[0][0].map((part) => part.skillId)).toEqual(["s1", "s2", "s3"]);
    const layer = result.current.surface.layer("deleteSkill");
    expect(layer?.report?.map((part) => ("report" in part ? part.report.record_deleted : "error"))).toEqual([true, false, "error"]);
    // One run in the history: its headline and each failure.
    const runs = new Set(result.current.reporter.notifications.map((row) => row.runId));
    expect(runs.size).toBe(1);
  });
});

describe("a Delete answered CANCELLED (audit A4)", () => {
  it("settles as the cancelled kind: a neutral toast, no failure in the history, the dialog closed", async () => {
    const s = setup({ skills: [skill("s1", "alpha")] });
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "deleteManagedSkill" ? Promise.reject({ code: "CANCELLED" }) : base(command, ...args),
    );
    const spy = vi.spyOn(folds, "deleteOutcome");
    const { result } = await render(s);
    act(() => void result.current.actions.run("delete", s.skills));
    expect(result.current.surface.layer("deleteSkill")).toBeTruthy();
    await act(async () => {
      await result.current.actions.run("delete", s.skills, { confirmed: true });
    });
    expect(spy).toHaveBeenCalledTimes(1);
    const outcome = spy.mock.results[0].value as ReturnType<typeof folds.deleteOutcome>;
    expect(outcome.toast?.tone).toBe("neutral");
    expect(outcome.errors).toEqual([]);
    expect(result.current.surface.layer("deleteSkill")).toBeNull();
    expect(result.current.reporter.notifications.some((row) => row.kind === "error")).toBe(false);
  });
});

describe("the reload rule on a thrown request", () => {
  it.each(VERBS)("$name: reloads iff RELOAD_ON_THROW says so, and the failure is reported", async (row) => {
    const s = setup();
    row.prepare?.(s.skills);
    const failure = { code: "OTHER", message: "request failed" };
    const fail = () => Promise.reject(failure);
    mockInvoke.mockImplementation((command) =>
      command === "getManagedSkills" ? Promise.resolve(s.skills) : fail(),
    );
    s.sync.syncSkillsToTools.mockImplementation(fail);
    s.sync.enableSkillTargets.mockImplementation(fail);
    s.sync.collectSync.mockImplementation(fail);
    s.sync.collectEnable.mockImplementation(fail);
    s.projects.assignToProject.mockImplementation(fail);
    s.projects.unassignFromProject.mockImplementation(fail);
    const rendered = renderHook(() => useDoor(s));
    await waitFor(() => expect(rendered.result.current.library.managedSkills).toEqual(s.skills));
    mockInvoke.mockClear();
    let answer: unknown;
    await act(async () => {
      answer = await rendered.result.current.actions.run(row.verb, pick(row, s.skills), row.opts as never);
    });
    // A thrown skill of a Delete is a settlement, not a thrown action: the
    // run folds and reloads like any Delete.
    expect(reloads() > 0).toBe(row.verb === "delete" ? true : RELOAD_ON_THROW[row.verb]);
    if (row.verb === "repoint") {
      // Re-point's final refusal is answered to its dialog (D15, I4): shown
      // by the chosen target, never a toast over it.
      expect(answer).toEqual(failure);
      expect(rendered.result.current.reporter.notifications).toEqual([]);
    } else {
      expect(rendered.result.current.reporter.notifications).toContainEqual(expect.objectContaining({ kind: "error" }));
    }
    expect(rendered.result.current.reporter.pending).toBeNull();
  });
});

describe("one mutation at a time", () => {
  it("refuses a second verb while an Update runs, then accepts once it settled", async () => {
    const s = setup();
    let release!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "updateManagedSkill" ? gate.then(() => base(command, ...args)) : base(command, ...args),
    );
    const { result } = await render(s);
    let running!: Promise<void>;
    act(() => {
      running = result.current.actions.run("update", [s.skills[0]]);
    });
    await waitFor(() => expect(result.current.reporter.pending).not.toBeNull());
    expect(result.current.reporter.pending).toMatchObject({
      subject: { kind: "skill", id: "s1" },
      cancellable: true,
      label: "lifecycle.verbs.update",
    });
    await act(async () => {
      await result.current.actions.run("disable", [s.skills[1]]);
    });
    expect(commands()).not.toContain("disableSkillTargets");
    await act(async () => {
      release();
      await running;
    });
    await act(async () => {
      await result.current.actions.run("disable", [s.skills[1]]);
    });
    expect(commands()).toContain("disableSkillTargets");
  });
});

describe("Change source opens on the skill's kind", () => {
  it.each([
    ["git", undefined, "git"],
    ["local", undefined, "local"],
    // An imported skill has no source: it opens on a folder.
    ["imported", undefined, "local"],
    // The source_missing repair names its kind explicitly.
    ["git", "local", "local"],
  ] as const)("a %s skill (preselect %s) opens on %s", async (source_type, preselect, expected) => {
    const s = setup({ skills: [{ ...skill("s1", "alpha"), source_type }] });
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("repoint", [s.skills[0]], preselect ? { preselect } : {});
    });
    expect(result.current.surface.layer("repoint")).toEqual({ kind: "repoint", skillId: "s1", name: "alpha", preselect: expected });
  });

  it("a health row's repoint repair on a missing folder opens on local", async () => {
    const row = { ...skill("s1", "alpha"), source_type: "git", unlocatable: "source_missing" as const };
    const s = setup({ skills: [row] });
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.repair(row, { kind: "source_missing", scope: "skill", severity: "stale", count: 0, tools: [], error: null, at: null, repairs: ["repoint"] }, "repoint");
    });
    expect(result.current.surface.layer("repoint")).toMatchObject({ preselect: "local" });
  });
});

describe("report data (D11)", () => {
  it("a Refresh tells the fold each skill's Tools before the run (its re-assert additions are announced)", async () => {
    const s = setup();
    const spy = vi.spyOn(folds, "refreshOutcome");
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("update", "all", { confirmed: true });
    });
    const ctx = spy.mock.calls[0][1];
    expect(ctx.priorGlobalTools).toEqual(
      Object.fromEntries(
        s.skills.map((skill) => [skill.id, skill.targets.filter((target) => target.status !== "disabled").map((target) => target.tool)]),
      ),
    );
  });

  it("an entry lands in the run's history block as data — its subject and typed error, no callback", async () => {
    const s = setup();
    vi.spyOn(folds, "refreshOutcome").mockReturnValue({
      toast: null,
      warnings: [],
      notes: [],
      completion: { reload: false, closeModal: false, conflict: false },
      errors: [
        {
          title: "gone",
          message: "missing upstream",
          subject: { skillId: "s1", skillName: "alpha" },
          error: { code: "GITHUB_SKILL_NOT_FOUND", url: "https://github.com/o/r" },
          repair: { verb: "update" },
        },
      ],
    });
    const { result } = await render(s);
    await act(async () => {
      await result.current.actions.run("update", "all", { confirmed: true });
    });
    const entry = result.current.reporter.notifications.find((n) => n.title === "gone");
    expect(entry).toMatchObject({
      role: "entry",
      runLabel: "lifecycle.verbs.refresh",
      subject: { skillId: "s1", skillName: "alpha" },
      error: { code: "GITHUB_SKILL_NOT_FOUND" },
    });
    expect(entry?.action).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// The one overwrite ask across a door action (B2/B3): the real sync world and
// surface compose the ask; the backend stays at the invokeTauri seam.
// ---------------------------------------------------------------------------

describe("one operator action, one overwrite ask (the real sync world)", () => {
  const TOOL_STATUS: ToolStatusDto = {
    tools: ["claude", "cursor"].map((key) => ({ key, label: key.toUpperCase(), installed: true, shared_with: [key], constituents: [] })),
    installed: ["claude", "cursor"],
    newly_installed: [],
  };
  const SETTINGS: AppSettings = {
    central_repo_path: "/hub",
    git_cache_cleanup_days: 30,
    git_cache_ttl_secs: 60,
    github_token_set: false,
    auto_sync_enabled: true,
    global_selected_tools: ["claude", "cursor"],
    global_selected_tools_corrupt: false,
    scan_selected_tools_only: true,
    ui_zoom_level: 1,
    bounds: {
      git_cache_cleanup_days: { min: 0, max: 3650 },
      git_cache_ttl_secs: { min: 0, max: 3600 },
      ui_zoom_level: { min: 0.5, max: 3 },
    },
  };
  const exists = (path: string) => ({ code: "TARGET_EXISTS" as const, path, modifiedAt: 7, sameContent: false });
  const occupiedTarget = (tool: string, project?: string): PropagationOutcome => ({
    scope: project ? { scope: "project", project_id: project, tool } : { scope: "global", tool },
    status: { status: "failed", error: exists(`/${project ?? "tools"}/${tool}/alpha`) },
  });
  const pair = (skill_id: string, tool_key: string, status: BatchTargetOutcome["status"]): BatchTargetOutcome => ({
    skill_id,
    skill_name: skill_id === "s1" ? "alpha" : skill_id === "s2" ? "beta" : "gamma",
    tool_key,
    status,
  });
  const collided = (skill_id: string, tool: string) => pair(skill_id, tool, { status: "failed", error: exists(`/tools/${tool}/${skill_id}`) });
  const done = (skill_id: string, tool: string) =>
    pair(skill_id, tool, { status: "synced", outcome: { mode_used: "symlink", target_path: `/tools/${tool}/${skill_id}`, replaced: true } });
  const refreshedWith = (targets: PropagationOutcome[]): RefreshReport => ({
    skills: [{
      skill_id: "s1",
      skill_name: "alpha",
      status: { status: "refreshed", content_hash: null, source_revision: null, targets, reassert_error: null, edit_conflict: null },
    }],
  });

  function backend(skills: ManagedSkill[], answers: Partial<Record<CommandName, unknown[]>>) {
    const queues = Object.fromEntries(Object.entries(answers).map(([k, v]) => [k, [...(v ?? [])]]));
    mockInvoke.mockImplementation((command) => {
      if (command === "getSettings") return Promise.resolve(SETTINGS);
      if (command === "getToolStatus") return Promise.resolve(TOOL_STATUS);
      if (command === "getManagedSkills") return Promise.resolve(skills);
      const queue = queues[command];
      return Promise.resolve(queue?.length ? queue.shift() : null);
    });
  }

  function renderReal() {
    const projects = {
      assignToProject: vi.fn(),
      unassignFromProject: vi.fn(),
      refreshView: vi.fn(async () => true),
    };
    const rendered = renderHook(() => {
      const surface = useSurfaceState();
      const reporter = useStatusReporter(t);
      const catalog = useToolCatalog({ t });
      const sync = useSyncOrchestration({ t, reporter, surface, catalog });
      const library = useSkillLibrary({ t, reporter, surface });
      const actions = useSkillActions({
        t,
        reporter,
        sync,
        catalog,
        surface,
        library,
        projects,
        order: library.managedSkills.map((row) => row.id),
      });
      return { surface, reporter, library, catalog, sync, actions, overwrite: surface.pendingAsk("overwrite") };
    });
    return { ...rendered, projects };
  }

  async function ready(rendered: ReturnType<typeof renderReal>, skills: ManagedSkill[]) {
    await waitFor(() => {
      expect(rendered.result.current.library.managedSkills).toEqual(skills);
      expect(rendered.result.current.catalog.toolStatus).not.toBeNull();
      expect(rendered.result.current.sync.effectiveSyncTargetIds).toEqual(["claude", "cursor"]);
    });
    mockInvoke.mockClear();
  }

  /** Run a door action, answering every overwrite ask it raises; how many, and their rows. */
  async function drive(
    rendered: ReturnType<typeof renderReal>,
    start: () => Promise<void>,
    answer: "confirmed" | "declined" | "dismiss",
  ) {
    let finished = false;
    let running!: Promise<void>;
    act(() => {
      running = start().finally(() => {
        finished = true;
      });
    });
    const asked: string[][] = [];
    while (!finished) {
      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 0));
      });
      const pending = rendered.result.current.overwrite;
      if (!pending) continue;
      asked.push(pending.rows.map((row) => `${row.scope.kind === "project" ? row.scope.projectId : "global"}:${row.skillId}:${row.toolKey}`));
      await act(async () => {
        if (answer === "dismiss") rendered.result.current.surface.closeDialog("overwrite");
        else pending.answer(answer);
      });
    }
    await act(async () => {
      await running;
    });
    return asked;
  }

  const calls = (command: CommandName) => mockInvoke.mock.calls.filter(([name]) => name === command);
  const runIds = (rendered: ReturnType<typeof renderReal>) =>
    new Set(rendered.result.current.reporter.notifications.map((row) => row.runId));

  it.each([
    ["update", "updateManagedSkill", {}],
    ["restore", "updateManagedSkill", {}],
    ["repoint", "repointSkillSource", { target: { kind: "local", path: "/elsewhere" } }],
  ] as const)(
    "%s: the re-assert's and Propagation's collisions are one ask; the answered pairs are retried and replace the failures",
    async (verb, command, opts) => {
      const skills = [skill("s1", "alpha", ["claude"])];
      backend(skills, {
        [command]: [{ report: refreshedWith([occupiedTarget("cursor"), occupiedTarget("claude", "p1")]), skills }],
        syncSkillsToTools: [[done("s1", "cursor")]],
        resolveProjectCollisions: [{ items: [{ assignment_id: "a1", project_id: "p1", skill_id: "s1", skill_name: "alpha", tool: "claude", status: { status: "synced" } }] }],
      });
      const rendered = renderReal();
      await ready(rendered, skills);
      const asked = await drive(rendered, async () => { await rendered.result.current.actions.run(verb, [skills[0]!], opts as never); }, "confirmed");
      expect(asked).toEqual([["global:s1:cursor", "p1:s1:claude"]]);
      expect(calls("syncSkillsToTools").map(([, items, tools, policy]) => [items, tools, policy])).toEqual([
        [
          [{ skill_id: "s1", name: "alpha", source_path: "/hub/alpha" }],
          ["cursor"],
          { overwrite: false, overwrite_if_same_content: true, overrides: [{ skill_id: "s1", tool: "cursor", decision: "overwrite" }] },
        ],
      ]);
      expect(calls("resolveProjectCollisions")[0]![1]).toEqual([
        { project_id: "p1", skill_id: "s1", tool: "claude", decision: "overwrite" },
      ]);
      // The pre-retry envelope is stale: the catalog and the project view are re-read.
      expect(calls("getManagedSkills").length).toBeGreaterThan(0);
      expect(rendered.projects.refreshView).toHaveBeenCalledWith("p1");
      // The answered rows replaced the failures: no target error is reported.
      const { notifications } = rendered.result.current.reporter;
      expect(notifications.filter((row) => row.kind === "error")).toEqual([]);
      expect(runIds(rendered).size).toBe(1);
      expect(rendered.result.current.reporter.pending).toBeNull();
    },
  );

  it("a dismissed ask retries nothing; the collision is to review, never an error (ticket 11 audit Q4)", async () => {
    const skills = [skill("s1", "alpha", ["claude"])];
    backend(skills, { updateManagedSkill: [{ report: refreshedWith([occupiedTarget("cursor")]), skills }] });
    const rendered = renderReal();
    await ready(rendered, skills);
    const asked = await drive(rendered, () => rendered.result.current.actions.run("update", [skills[0]!]), "dismiss");
    expect(asked).toHaveLength(1);
    // No retry, no implicit Keep: the pair stays as it was.
    expect(calls("syncSkillsToTools")).toEqual([]);
    expect(calls("resolveProjectCollisions")).toEqual([]);
    const rows = rendered.result.current.reporter.notifications;
    expect(rows.filter((row) => row.kind === "error")).toEqual([]);
    expect(rows.filter((row) => row.review)).toMatchObject([
      { role: "entry", kind: "warning", subject: { skillId: "s1", tool: "cursor" }, repair: { verb: "overwrite", tool: "cursor", via: "sync" } },
    ]);
    expect(rows.find((row) => row.role === "headline")).toMatchObject({ kind: "warning" });
  });

  it("Refresh (all): Keep existing retries with a keep decision and the kept pair is a declined note, not a failure", async () => {
    const skills = [skill("s1", "alpha", ["claude"])];
    backend(skills, {
      refreshManagedSkills: [refreshedWith([occupiedTarget("cursor")])],
      syncSkillsToTools: [[pair("s1", "cursor", { status: "declined" })]],
    });
    const rendered = renderReal();
    await ready(rendered, skills);
    const spy = vi.spyOn(folds, "refreshOutcome");
    const asked = await drive(rendered, () => rendered.result.current.actions.run("update", "all", { confirmed: true }), "declined");
    expect(asked).toEqual([["global:s1:cursor"]]);
    expect(calls("syncSkillsToTools")[0]![3]).toMatchObject({ overrides: [{ skill_id: "s1", tool: "cursor", decision: "keep" }] });
    // Folded once, over the merged report.
    expect(spy).toHaveBeenCalledTimes(1);
    const [report, ctx] = spy.mock.calls[0]!;
    expect(report.skills[0]!.status).toMatchObject({ targets: [] });
    expect(ctx.declined).toEqual([{ skillName: "alpha", tool: "cursor" }]);
    expect(rendered.result.current.reporter.notifications.filter((row) => row.kind === "error")).toEqual([]);
  });

  it("Edit: its Propagation's collisions reach the ask and the retry", async () => {
    const skills = [skill("s1", "alpha", ["claude"])];
    backend(skills, {
      setSkillInvocationOverride: [{ report: { skill_id: "s1", skill_name: "alpha", propagation: { targets: [occupiedTarget("claude")] } }, skills }],
      syncSkillsToTools: [[done("s1", "claude")]],
    });
    const rendered = renderReal();
    await ready(rendered, skills);
    const asked = await drive(rendered, () => rendered.result.current.actions.run("editInvocation", [skills[0]!], { mode: "user-only" }), "confirmed");
    expect(asked).toEqual([["global:s1:claude"]]);
    expect(calls("syncSkillsToTools")).toHaveLength(1);
    expect(rendered.result.current.reporter.notifications.filter((row) => row.kind === "error")).toEqual([]);
  });

  it("deploy a group: a disabled skill's Enable and a new skill's sync collide — one ask with both pairs, one run", async () => {
    const a = { ...skill("s1", "alpha"), disabled_at: 3 };
    const b = skill("s2", "beta");
    const skills = [a, b];
    backend(skills, {
      enableSkillTargets: [
        { global: [collided("s1", "claude")], projects: { items: [] } },
        { global: [done("s1", "claude")], projects: { items: [] } },
      ],
      syncSkillsToTools: [
        [done("s2", "claude"), collided("s2", "cursor")],
        [done("s2", "cursor")],
      ],
    });
    const rendered = renderReal();
    await ready(rendered, skills);
    const asked = await drive(rendered, () => rendered.result.current.actions.run("deployGroup", skills), "confirmed");
    expect(asked).toEqual([["global:s1:claude", "global:s2:cursor"]]);
    expect(calls("enableSkillTargets")).toEqual([
      ["enableSkillTargets", { kind: "skills", skill_ids: ["s1"] }, []],
      ["enableSkillTargets", { kind: "skills", skill_ids: ["s1"] }, [{ skill_id: "s1", tool: "claude", decision: "overwrite" }]],
    ]);
    // The sync retry is exactly B's asked pair.
    expect(calls("syncSkillsToTools")[1]!.slice(1, 3)).toEqual([[{ skill_id: "s2", name: "beta", source_path: "/hub/beta" }], ["cursor"]]);
    expect(runIds(rendered).size).toBe(1);
    expect(rendered.result.current.reporter.notifications.filter((row) => row.kind === "error")).toEqual([]);
    // One complete report (B3-R1): both of B's synced pairs AND A's enabled
    // target are in the one headline, not only the sync's half.
    expect(rendered.result.current.reporter.notifications.map(({ kind, title }) => ({ kind, title }))).toContainEqual({
      kind: "success",
      title: 'outcome.done.synced {"count":2} \u00b7 outcome.done.enabled {"count":1}',
    });
  });

  it("retry over two occupied Tools: one batch, one ask with both pairs, one retry of exactly them", async () => {
    const failed = skill("s1", "alpha", ["claude", "cursor"]);
    failed.targets = failed.targets.map((target) => ({ ...target, status: target.tool === "claude" ? "error" : "missing", synced_at: null }));
    const skills = [failed];
    backend(skills, {
      syncSkillsToTools: [
        [collided("s1", "claude"), collided("s1", "cursor")],
        [done("s1", "claude"), done("s1", "cursor")],
      ],
    });
    const rendered = renderReal();
    await ready(rendered, skills);
    const asked = await drive(rendered, () => rendered.result.current.actions.run("retry", skills), "confirmed");
    expect(asked).toEqual([["global:s1:claude", "global:s1:cursor"]]);
    expect(calls("syncSkillsToTools").map(([, , tools]) => tools)).toEqual([["claude", "cursor"], ["claude", "cursor"]]);
    expect(runIds(rendered).size).toBe(1);
  });

  // Ticket 14 audit A3: the panel's Overwrite… on an Enable's collision (the ask
  // dismissed) answers it by the pair's Enable — an ordinary sync skips a disabled pair.
  describe("Overwrite… on a dismissed Enable collision answers it by Enable (A3)", () => {
    const PAIR: DisableScope = { kind: "skill_tool", skill_id: "s1", tool: "claude" };
    /** alpha, disabled at skill grain; after its Enable met a folder in the way, on again with the pair a disabled marker. */
    const before = () => ({
      ...skill("s1", "alpha", ["claude"]),
      disabled_at: 3,
      targets: [{ tool: "claude", mode: "symlink" as const, status: "disabled" as const, target_path: "/tools/claude/alpha", synced_at: null }],
    });
    const after = () => ({ ...before(), disabled_at: null });

    /** The backend: the catalog now (`catalog.current`) and queued Enable answers. */
    function backendNow(catalog: { current: ManagedSkill[] }, enables: unknown[]) {
      mockInvoke.mockImplementation((command) => {
        if (command === "getSettings") return Promise.resolve(SETTINGS);
        if (command === "getToolStatus") return Promise.resolve(TOOL_STATUS);
        if (command === "getManagedSkills") return Promise.resolve(catalog.current);
        if (command === "enableSkillTargets") return Promise.resolve(enables.shift() ?? null);
        return Promise.resolve(null);
      });
    }

    /** Enable \u2192 the ask \u2192 dismissed; the report's row, and the panel's action for it. */
    async function dismissedEnable(enables: unknown[]) {
      const catalog = { current: [before()] as ManagedSkill[] };
      backendNow(catalog, [{ global: [collided("s1", "claude")], projects: { items: [] } }, ...enables]);
      const rendered = renderReal();
      await ready(rendered, catalog.current);
      // The backend keeps the pair disabled (a marker owning nothing) and turns the skill on.
      catalog.current = [after()];
      const asked = await drive(rendered, () => rendered.result.current.actions.run("enable", ["s1"]), "dismiss");
      expect(asked).toEqual([["global:s1:claude"]]);
      expect(calls("enableSkillTargets")).toEqual([["enableSkillTargets", { kind: "skill", skill_id: "s1" }, []]]);
      const row = rendered.result.current.reporter.notifications.find((n) => n.review)!;
      expect(row).toMatchObject({ repair: { verb: "overwrite", tool: "claude", via: "enable" } });
      const overwrite = rowActions(row, () => true).find((action) => action.kind === "overwrite");
      expect(overwrite).toEqual({ kind: "overwrite", skillId: "s1", tool: "claude", via: "enable" });
      mockInvoke.mockClear();
      return { rendered, catalog };
    }

    /** What the panel's Overwrite\u2026 dispatches for a row whose route is Enable. */
    const overwriteFromPanel = (rendered: ReturnType<typeof renderReal>) => () =>
      rendered.result.current.actions.run("enable", ["s1"], { tool: "claude", answering: "collision" });

    it("Overwrite: one ask, then the pair's Enable with the overwrite answer \u2014 never an ordinary sync", async () => {
      const { rendered } = await dismissedEnable([
        { global: [collided("s1", "claude")], projects: { items: [] } },
        { global: [done("s1", "claude")], projects: { items: [] } },
      ]);
      const asked = await drive(rendered, overwriteFromPanel(rendered), "confirmed");
      expect(asked).toEqual([["global:s1:claude"]]);
      expect(calls("enableSkillTargets")).toEqual([
        ["enableSkillTargets", PAIR, []],
        ["enableSkillTargets", PAIR, [{ skill_id: "s1", tool: "claude", decision: "overwrite" }]],
      ]);
      expect(calls("syncSkillsToTools")).toEqual([]);
      const latest = rendered.result.current.reporter.notifications[0];
      expect(latest).toMatchObject({ role: "headline", kind: "success", title: "status.targetsEnabled" });
      expect(rendered.result.current.reporter.pending).toBeNull();
    });

    it("Keep existing: the pair's Enable with a keep answer \u2014 the pair stays disabled, reported declined", async () => {
      const { rendered } = await dismissedEnable([
        { global: [collided("s1", "claude")], projects: { items: [] } },
        { global: [pair("s1", "claude", { status: "declined" })], projects: { items: [] } },
      ]);
      const asked = await drive(rendered, overwriteFromPanel(rendered), "declined");
      expect(asked).toHaveLength(1);
      expect(calls("enableSkillTargets")[1]).toEqual(["enableSkillTargets", PAIR, [{ skill_id: "s1", tool: "claude", decision: "keep" }]]);
      const rows = rendered.result.current.reporter.notifications;
      expect(rows.filter((row) => row.kind === "error")).toEqual([]);
      expect(rows[0]).toMatchObject({ kind: "info", title: expect.stringMatching(/^outcome\.declinedLine \{"name":"alpha"/) });
    });

    it("dismissed again: nothing retried, the collision is to review once more", async () => {
      const { rendered } = await dismissedEnable([{ global: [collided("s1", "claude")], projects: { items: [] } }]);
      const asked = await drive(rendered, overwriteFromPanel(rendered), "dismiss");
      expect(asked).toHaveLength(1);
      expect(calls("enableSkillTargets")).toEqual([["enableSkillTargets", PAIR, []]]);
      expect(rendered.result.current.reporter.notifications.filter((row) => row.kind === "error")).toEqual([]);
      expect(rendered.result.current.reporter.notifications.filter((row) => row.review)).toHaveLength(2);
    });

    it("stale: the skill was disabled since \u2014 nothing runs, and the panel says so", async () => {
      const { rendered, catalog } = await dismissedEnable([]);
      catalog.current = [before()];
      await act(async () => {
        await rendered.result.current.library.loadManagedSkills();
      });
      mockInvoke.mockClear();
      const asked = await drive(rendered, overwriteFromPanel(rendered), "confirmed");
      expect(asked).toEqual([]);
      expect(calls("enableSkillTargets")).toEqual([]);
      expect(calls("syncSkillsToTools")).toEqual([]);
      expect(rendered.result.current.reporter.notifications[0]).toMatchObject({
        kind: "info",
        title: expect.stringMatching(/^staleCollision\.skillDisabled \{"name":"alpha"/),
      });
    });

    it("a sync's collision whose pair was disabled since is stale too (a Disable is never overwritten)", async () => {
      const { rendered } = await dismissedEnable([]);
      await drive(rendered, () => rendered.result.current.actions.run("retry", ["s1"], { tool: "claude", answering: "collision" }), "confirmed");
      expect(calls("syncSkillsToTools")).toEqual([]);
      expect(rendered.result.current.reporter.notifications[0]).toMatchObject({
        title: expect.stringMatching(/^staleCollision\.pairDisabled \{"name":"alpha"/),
      });
    });
  });

  it("a stale target's repair re-syncs that Tool from the central copy, never an upstream Update (S5)", async () => {
    const imported = { ...skill("s1", "alpha", ["claude", "cursor"]), source_type: "imported", refreshable: false };
    imported.targets = imported.targets.map((target) => (target.tool === "cursor" ? { ...target, status: "stale" } : target));
    const skills = [imported];
    backend(skills, { syncSkillsToTools: [[done("s1", "cursor")]] });
    const rendered = renderReal();
    await ready(rendered, skills);
    const row = skillHealth(imported, { t }).rows[0]!;
    expect(row).toMatchObject({ kind: "targets_stale", tools: ["cursor"], repairs: ["retryTargets"] });
    await act(async () => {
      await rendered.result.current.actions.repair(imported, row, "retryTargets");
    });
    expect(calls("updateManagedSkill")).toEqual([]);
    expect(calls("syncSkillsToTools")[0]!.slice(1, 3)).toEqual([[{ skill_id: "s1", name: "alpha", source_path: "/hub/alpha" }], ["cursor"]]);
  });
});

describe("a failed confirmation never escapes a fire-and-forget run", () => {
  it("a rejected shared-dir confirmation reaches notifyError and writes nothing", async () => {
    const s = setup();
    s.sync.requestSharedDirConfirmation.mockImplementation(() => Promise.reject({ code: "OTHER", message: "ask broke" }));
    const { result } = await render(s);
    let outcome: unknown = "unset";
    await act(async () => {
      outcome = await result.current.actions.run("disable", [s.skills[0]], { tool: "claude" }).then(
        () => "resolved",
        () => "rejected",
      );
    });
    expect(outcome).toBe("resolved");
    expect(commands()).toEqual([]);
    expect(result.current.reporter.notifications[0]).toMatchObject({ kind: "error" });
  });
});
