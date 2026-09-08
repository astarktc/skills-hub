// Tests at the SkillLibrary seam: the managed-skill list actions. The sync
// world and the reporter enter as dependency interfaces (mocked objects),
// the backend at the invokeTauri module seam — exactly the shape App.tsx
// wires, so these tests exercise the hook the way the app does.

import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  BatchSyncReportDto,
  ManagedSkill,
  RefreshReportDto,
  RemovalReportDto,
} from "../components/skills/types";
import type { SkillLibraryDeps } from "./useSkillLibrary";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));
// The refresh batch streams progress over a Tauri Channel; the hook imports
// it lazily, so the module seam is stubbed with a plain message sink.
vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: ((message: unknown) => void) | null = null;
  },
}));
// Re-point picks the folder through the dialog plugin (lazily imported).
const pickFolder = vi.fn<() => Promise<string | null>>();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: (...args: unknown[]) => pickFolder(...(args as [])),
}));

import { invokeTauri, type CommandName } from "../lib/tauri";
import * as folds from "../lib/reportOutcome";
import { useSkillLibrary } from "./useSkillLibrary";
import {
  ActionExit,
  type ActionHandle,
  type RunActionOptions,
  type StatusReporter,
} from "./useStatusReporter";

// The seam is generic over the command table; the stub switches on the
// command name, so it is typed loosely (positional args, unknown result).
const mockInvoke = vi.mocked(
  invokeTauri as unknown as (
    command: CommandName,
    ...args: unknown[]
  ) => Promise<unknown>,
);

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

function skill(id: string, name: string, targets: string[] = []): ManagedSkill {
  return {
    id,
    name,
    description: null,
    source_type: "local",
    source_ref: null,
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
      synced_at: null,
    })),
    refreshable: true,
    unlocatable: null,
    detachable: false,
  };
}

const EMPTY_REPORT: BatchSyncReportDto = {
  results: [],
  synced: 0,
  skipped: 0,
  failed: 0,
};

/** A refresh report in which every named skill refreshed with no targets. */
function refreshedReport(names: string[]): RefreshReportDto {
  return {
    skills: names.map((name, i) => ({
      skill_id: `s${i + 1}`,
      skill_name: name,
      status: {
        status: "refreshed",
        content_hash: null,
        source_revision: null,
        targets: [],
        reassert_error: null,
        edit_conflict: null,
      },
    })),
    refreshed: names.length,
    failed: 0,
    skipped: 0,
    target_failures: 0,
  };
}

/** A removal report in which every named tool's artifact was removed. */
function removedReport(tools: string[]): RemovalReportDto {
  return {
    targets: tools.map((tool) => ({
      scope: { scope: "global" },
      tool,
      path: `/tools/${tool}/alpha`,
      status: { status: "removed" },
    })),
    removed: tools.length,
    failed: 0,
  };
}

function makeDeps(overrides?: {
  skills?: ManagedSkill[];
  autoSyncEnabled?: boolean;
  installedToolIds?: string[];
  sharedDirConfirmation?: boolean | Promise<boolean>;
  syncReport?: BatchSyncReportDto;
  refreshReport?: RefreshReportDto;
  removalReport?: RemovalReportDto;
}) {
  const skills = overrides?.skills ?? [skill("s1", "alpha")];
  const refreshReport =
    overrides?.refreshReport ?? refreshedReport(skills.map((s) => s.name));
  mockInvoke.mockImplementation((command) => {
    switch (command) {
      case "getManagedSkills":
        return Promise.resolve(skills);
      case "refreshManagedSkills":
      case "repointLocalSkillSource":
      case "repointGitSkillSource":
        return Promise.resolve(refreshReport);
      case "unsyncSkill":
      case "unsyncAllSkills":
      case "unsyncSkillFromTool":
        return Promise.resolve(
          overrides?.removalReport ?? removedReport(["claude"]),
        );
      default:
        return Promise.resolve(undefined);
    }
  });

  const formatError = vi.fn((err: unknown) => {
    const code = (err as { code?: string })?.code;
    return code === "CANCELLED" ? null : `formatted:${code ?? String(err)}`;
  });
  const setError = vi.fn();
  const setSuccessToastMessage = vi.fn();
  // Stub of the runAction contract: the body's outcome lands on the same
  // one-shot setters the real reporter uses, so assertions read naturally.
  // The lifecycle itself (loading surface) is the reporter's own test.
  const runAction = vi.fn(
    async <T>(
      opts: RunActionOptions<T>,
      fn: (action: ActionHandle) => Promise<T | ActionExit>,
    ): Promise<T | undefined> => {
      try {
        const outcome = await fn({
          handOff: () => ActionExit.handOff(),
          fail: (message) => ActionExit.failed(message),
        });
        if (outcome instanceof ActionExit) {
          if (outcome.kind === "failed") setError(outcome.message);
          return undefined;
        }
        const { successToast } = opts;
        if (typeof successToast === "function") {
          setSuccessToastMessage(successToast(outcome));
        } else if (successToast) {
          setSuccessToastMessage(successToast);
        }
        return outcome;
      } catch (err) {
        setError(formatError(err));
        return undefined;
      }
    },
  );
  const reporter: StatusReporter = {
    loading: false,
    loadingStartAt: null,
    actionMessage: null,
    // vi.fn erases the generic; the spy still records calls.
    runAction: runAction as StatusReporter["runAction"],
    setActionMessage: vi.fn(),
    notify: vi.fn(),
    notifications: [],
    unreadCount: 0,
    markAllRead: vi.fn(),
    clearNotifications: vi.fn(),
    setError,
    setSuccessToastMessage,
    formatError,
    notifyError: vi.fn(),
    showActionErrors: vi.fn(),
    showActionWarnings: vi.fn(),
    copyToClipboard: vi.fn().mockResolvedValue(true),
    cancelLoading: vi.fn(),
  };

  const sync = {
    autoSyncEnabled: overrides?.autoSyncEnabled ?? true,
    installedToolIds: overrides?.installedToolIds ?? ["claude", "cursor"],
    // The shared-dir confirmation seam: its own decision/label arithmetic
    // is tested in useSharedDirConfirmation.test.ts; here it is a stub that
    // answers with the given verdict.
    requestSharedDirConfirmation: vi.fn(
      () => Promise.resolve(overrides?.sharedDirConfirmation ?? true) as Promise<boolean>,
    ),
    syncSkillsToTools: vi
      .fn()
      .mockResolvedValue(overrides?.syncReport ?? EMPTY_REPORT),
    toolLabelById: { claude: "CLAUDE", cursor: "CURSOR", pi: "PI" },
    tools: [
      { id: "claude", label: "CLAUDE" },
      { id: "cursor", label: "CURSOR" },
      { id: "pi", label: "PI" },
    ],
  };

  const deps: SkillLibraryDeps = { t, reporter, sync };
  return { deps, reporter, sync, skills };
}

async function renderLibrary(setup: ReturnType<typeof makeDeps>) {
  const rendered = renderHook(() => useSkillLibrary(setup.deps));
  await waitFor(() =>
    expect(rendered.result.current.managedSkills).toEqual(setup.skills),
  );
  return rendered;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("invocation Edits", () => {
  it("replaces the returned DTO in place and closes without a refetch", async () => {
    const setup = makeDeps({ skills: [skill("s1", "alpha"), skill("s2", "beta")] });
    const { result } = await renderLibrary(setup);
    act(() => result.current.openInvocationEdit("s1"));
    const updated: ManagedSkill = { ...setup.skills[0], invocation_mode: "user-only", invocation_override: { mode: "user-only", base_mode: "user-and-model", conflict: false } };
    mockInvoke.mockResolvedValueOnce({ entry: updated, propagation: [] });
    await act(async () => { await result.current.setInvocationOverride("s1", "user-only"); });
    expect(mockInvoke).toHaveBeenLastCalledWith("setSkillInvocationOverride", "s1", "user-only");
    expect(result.current.managedSkills).toEqual([updated, setup.skills[1]]);
    expect(result.current.managedSkills[1]).toBe(setup.skills[1]);
    expect(result.current.invocationEditSkillId).toBeNull();
    expect(mockInvoke.mock.calls.filter(([command]) => command === "getManagedSkills")).toHaveLength(1);
  });

  it("keeps the list and modal on failure", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    act(() => result.current.openInvocationEdit("s1"));
    mockInvoke.mockRejectedValueOnce({ code: "CENTRAL_PATH_MISSING", path: "/hub/alpha" });
    await act(async () => { await result.current.setInvocationOverride("s1", null); });
    expect(result.current.managedSkills).toEqual(setup.skills);
    expect(result.current.invocationEditSkillId).toBe("s1");
    expect(setup.reporter.setError).toHaveBeenCalledWith("formatted:CENTRAL_PATH_MISSING");
  });

});

describe("repair notification actions", () => {
  it("warns instead of opening a stale Re-point action after deletion", async () => {
    const gitSkill = { ...skill("s1", "alpha"), source_type: "git" };
    const setup = makeDeps({
      skills: [gitSkill],
      refreshReport: {
        skills: [{ skill_id: "s1", skill_name: "alpha", status: {
          status: "failed", error: { code: "GITHUB_SKILL_NOT_FOUND", url: "https://github.com/old/repo" },
        } }],
        refreshed: 0, failed: 1, skipped: 0, target_failures: 0,
      },
    });
    const { result } = await renderLibrary(setup);
    await act(async () => { await result.current.handleRefresh(); });
    const action = vi.mocked(setup.reporter.showActionErrors).mock.calls[0][0][0].action!;
    mockInvoke.mockResolvedValue([]);
    await act(async () => { await result.current.handleDeleteManaged(gitSkill); });
    act(() => action.onClick());
    expect(setup.reporter.notify).toHaveBeenCalledWith(
      "warning", 'errors.skillGone {"name":"alpha"}',
    );
    expect(result.current.pendingGitRepointSkill).toBeNull();
  });

});

describe("cancellation and non-report actions", () => {
  it("cancelling git Re-point performs no invoke or action", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    mockInvoke.mockClear();
    act(() => result.current.handleRepointGitSkill({ ...setup.skills[0], source_type: "git" }));
    act(() => result.current.handleCloseRepointGitSkill());
    await act(async () => { await result.current.handleConfirmRepointGitSkill("https://github.com/new/repo"); });
    expect(result.current.pendingGitRepointSkill).toBeNull();
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(setup.reporter.runAction).not.toHaveBeenCalled();
  });

  it("Re-point does nothing when the folder picker is cancelled", async () => {
    const setup = makeDeps();
    pickFolder.mockResolvedValue(null);
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRepointSkill(setup.skills[0]);
    });

    expect(mockInvoke).not.toHaveBeenCalledWith(
      "repointLocalSkillSource",
      expect.anything(),
      expect.anything(),
      expect.anything(),
    );
    expect(setup.reporter.runAction).not.toHaveBeenCalled();
  });

  it("Detach hands the skill to the backend and reloads", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleDetachSkill(setup.skills[0]);
    });

    expect(mockInvoke).toHaveBeenCalledWith("detachSkillFromSource", "s1");
    const calls = mockInvoke.mock.calls.map(([command]) => command);
    expect(calls.indexOf("getManagedSkills", 1)).toBeGreaterThan(
      calls.indexOf("detachSkillFromSource"),
    );
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      'status.detached {"name":"alpha"}',
    );
  });

  it("Detach surfaces a backend refusal", async () => {
    const setup = makeDeps();
    mockInvoke.mockImplementation((command) =>
      command === "detachSkillFromSource"
        ? Promise.reject({ code: "NOT_FOUND", kind: "skill", id: "s1" })
        : Promise.resolve(setup.skills),
    );
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleDetachSkill(setup.skills[0]);
    });

    expect(setup.reporter.setError).toHaveBeenCalledWith("formatted:NOT_FOUND");
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
  });

  it("asks the shared-dir confirmation, then syncs when it is granted", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleToggleToolForSkill(setup.skills[0], "claude");
    });

    expect(setup.sync.requestSharedDirConfirmation).toHaveBeenCalledWith(
      "claude",
    );
    await waitFor(() =>
      expect(setup.sync.syncSkillsToTools).toHaveBeenCalledWith(
        [{ skill_id: "s1", name: "alpha", source_path: "/hub/alpha" }],
        ["claude"],
        { overwriteIfSameContent: true },
      ),
    );
  });

  it("does not touch the tool when the shared-dir confirmation is declined", async () => {
    const setup = makeDeps({ sharedDirConfirmation: false });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleToggleToolForSkill(setup.skills[0], "claude");
    });

    expect(setup.sync.requestSharedDirConfirmation).toHaveBeenCalledWith(
      "claude",
    );
    expect(setup.sync.syncSkillsToTools).not.toHaveBeenCalled();
  });
});

describe("useSkillLibrary detail selection", () => {
  it("derives the current row, clears when gone, and supports closing", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    act(() => result.current.openDetail("s1"));
    expect(result.current.detailSkill).toEqual(setup.skills[0]);
    const updated = { ...setup.skills[0], description: "updated" };
    mockInvoke.mockResolvedValue([updated]);
    await act(async () => { await result.current.loadManagedSkills(); });
    expect(result.current.detailSkill).toEqual(updated);
    act(() => result.current.closeDetail());
    expect(result.current.detailSkill).toBeNull();
    act(() => result.current.openDetail("s1"));
    mockInvoke.mockResolvedValue([]);
    await act(async () => { await result.current.loadManagedSkills(); });
    expect(result.current.detailSkill).toBeNull();
  });
});

describe("useSkillLibrary name collisions", () => {
  it("isSkillNameTaken matches case-insensitively", async () => {
    const setup = makeDeps({ skills: [skill("s1", "Alpha")] });
    const { result } = await renderLibrary(setup);

    expect(result.current.isSkillNameTaken("alpha")).toBe(true);
    expect(result.current.isSkillNameTaken("ALPHA")).toBe(true);
    expect(result.current.isSkillNameTaken("beta")).toBe(false);
  });
});

afterEach(() => vi.restoreAllMocks());

// Presentation is table-tested through the pure module. Here a deliberately
// synthetic outcome proves that each caller obeys the fold, not DTO fields.
describe("invoke → fold → completion", () => {
  it.each([
    ["Update", "refreshOutcome", "refreshManagedSkills"],
    ["Restore", "refreshOutcome", "refreshManagedSkills"],
    ["git Re-point", "refreshOutcome", "repointGitSkillSource"],
    ["local Re-point", "refreshOutcome", "repointLocalSkillSource"],
    ["Refresh-all", "refreshOutcome", "refreshManagedSkills"],
    ["unsync-all", "removalOutcome", "unsyncAllSkills"],
    ["unsync", "removalOutcome", "unsyncSkill"],
    ["delete", "deleteOutcome", "deleteManagedSkill"],
    ["sync-to-all", "syncOutcome", null],
    ["auto-sync", "syncOutcome", null],
    ["toggle-on", "syncOutcome", null],
    ["toggle-off", "removalOutcome", "unsyncSkillFromTool"],
  ] as const)("%s applies the returned outcome (including refusal to reload/close)", async (action, fold, command) => {
    for (const complete of [false, true]) {
      const setup = makeDeps({ skills: [skill("s1", "alpha", action === "toggle-off" ? ["claude"] : [])] });
      const outcome: folds.Outcome = {
        toast: { kind: "warning", message: "fold toast", detail: "detail" },
        errors: [{ title: "fold error", message: "error detail" }],
        warnings: [{ title: "fold warning", message: "warning detail" }],
        completion: { reload: complete, closeModal: complete, conflict: true },
      };
      const spy = vi.spyOn(folds, fold).mockReturnValue(outcome);
      const { result, unmount } = await renderLibrary(setup);
      pickFolder.mockResolvedValue("/new/alpha");
      act(() => {
        result.current.handleDeletePrompt("s1");
        result.current.handleRepointGitSkill(setup.skills[0]);
      });
      mockInvoke.mockClear();
      await act(async () => {
        const lib = result.current;
        const row = setup.skills[0];
        switch (action) {
          case "Update": lib.handleUpdateSkill(row); break;
          case "Restore": await lib.handleRestoreSkill(row); break;
          case "git Re-point": await lib.handleConfirmRepointGitSkill(" https://github.com/new/repo "); break;
          case "local Re-point": await lib.handleRepointSkill(row); break;
          case "Refresh-all": await lib.handleRefresh(); break;
          case "unsync-all": await lib.handleUnsyncAll(); break;
          case "unsync": await lib.handleUnsyncSkill(row.id); break;
          case "delete": await lib.handleDeleteManaged(row); break;
          case "sync-to-all": await lib.handleSyncSkillToAllTools(row); break;
          case "auto-sync": await lib.syncAllManagedToTools(["claude"]); break;
          default: await lib.handleToggleToolForSkill(row, "claude");
        }
      });
      await waitFor(() => expect(spy).toHaveBeenCalledTimes(1));
      if (command) expect(mockInvoke.mock.calls.map(([cmd]) => cmd)).toContain(command);
      else expect(setup.sync.syncSkillsToTools).toHaveBeenCalledTimes(1);
      expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === "getManagedSkills")).toHaveLength(complete ? 1 : 0);
      expect(setup.reporter.showActionErrors).toHaveBeenCalledWith(outcome.errors);
      expect(setup.reporter.showActionWarnings).toHaveBeenCalledWith(outcome.warnings);
      expect(setup.reporter.notify).toHaveBeenCalledWith("warning", "fold toast", "detail");
      if (action === "delete") expect(result.current.pendingDeleteId).toBe(complete ? null : "s1");
      if (action === "git Re-point") expect(result.current.pendingGitRepointSkill).toEqual(complete ? null : setup.skills[0]);
      unmount();
      spy.mockRestore();
    }
  });

  it.each(["Update", "Restore", "git", "local"])("%s reloads after a thrown request without closing the repair modal", async (action) => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    act(() => result.current.handleRepointGitSkill(setup.skills[0]));
    pickFolder.mockResolvedValue("/new/alpha");
    mockInvoke.mockImplementation((cmd) => cmd === "getManagedSkills" ? Promise.resolve(setup.skills) : Promise.reject({ code: "OTHER", message: "request failed" }));
    mockInvoke.mockClear();
    await act(async () => {
      if (action === "Update") result.current.handleUpdateSkill(setup.skills[0]);
      else if (action === "Restore") await result.current.handleRestoreSkill(setup.skills[0]);
      else if (action === "git") await result.current.handleConfirmRepointGitSkill("https://github.com/new/repo");
      else await result.current.handleRepointSkill(setup.skills[0]);
    });
    await waitFor(() => expect(setup.reporter.setError).toHaveBeenCalledWith("formatted:OTHER"));
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === "getManagedSkills")).toHaveLength(1);
    expect(result.current.pendingGitRepointSkill).toEqual(setup.skills[0]);
  });

  it("resolves a notification id against a replaced row at click time", async () => {
    const setup = makeDeps();
    vi.spyOn(folds, "refreshOutcome").mockReturnValue({
      toast: null, warnings: [], completion: { reload: false, closeModal: false, conflict: false },
      errors: [{ title: "repair", message: "missing", action: { label: "repoint", skillId: "s1", skillName: "alpha" } }],
    });
    const { result } = await renderLibrary(setup);
    await act(async () => { await result.current.handleRefresh(); });
    const click = vi.mocked(setup.reporter.showActionErrors).mock.calls[0][0][0].action!.onClick;
    const updated = { ...setup.skills[0], source_ref: "new source" };
    mockInvoke.mockResolvedValue([updated]);
    await act(async () => { await result.current.loadManagedSkills(); });
    act(() => click());
    expect(result.current.pendingGitRepointSkill).toEqual(updated);
  });
});
