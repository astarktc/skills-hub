// Tests at the SkillLibrary seam: the managed-skill list actions. The sync
// world and the reporter enter as dependency interfaces (mocked objects),
// the backend at the invokeTauri module seam — exactly the shape App.tsx
// wires, so these tests exercise the hook the way the app does.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  BatchSyncReportDto,
  ManagedSkill,
  RefreshReportDto,
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

import { invokeTauri, type CommandName } from "../lib/tauri";
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
    central_path: `/hub/${name}`,
    created_at: 0,
    updated_at: 0,
    last_sync_at: null,
    status: "active",
    invocation_mode: "user-and-model",
    targets: targets.map((tool) => ({
      tool,
      mode: "symlink",
      status: "synced",
      target_path: `/tools/${tool}/${name}`,
      synced_at: null,
    })),
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
      },
    })),
    refreshed: names.length,
    failed: 0,
    target_failures: 0,
  };
}

function makeDeps(overrides?: {
  skills?: ManagedSkill[];
  autoSyncEnabled?: boolean;
  installedToolIds?: string[];
  sharedDirConfirmation?: boolean | Promise<boolean>;
  syncReport?: BatchSyncReportDto;
  refreshReport?: RefreshReportDto;
}) {
  const skills = overrides?.skills ?? [skill("s1", "alpha")];
  const refreshReport =
    overrides?.refreshReport ?? refreshedReport(skills.map((s) => s.name));
  mockInvoke.mockImplementation((command) => {
    switch (command) {
      case "getManagedSkills":
        return Promise.resolve(skills);
      case "refreshManagedSkills":
        return Promise.resolve(refreshReport);
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
    async <T,>(
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
    setError,
    setSuccessToastMessage,
    formatError,
    showActionErrors: vi.fn(),
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
    syncFailureEntries: vi.fn((report: BatchSyncReportDto) =>
      report.results
        .filter((r) => r.status.status === "failed")
        .map((r) => ({ title: r.skill_name, message: "sync failed" })),
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

describe("useSkillLibrary refresh", () => {
  it("issues one backend batch for every skill and never fans out a sync itself", async () => {
    const setup = makeDeps({ skills: [skill("s1", "alpha"), skill("s2", "beta")] });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRefresh();
    });

    const refreshCalls = mockInvoke.mock.calls.filter(
      ([command]) => command === "refreshManagedSkills",
    );
    expect(refreshCalls).toHaveLength(1);
    const [, skillIds, policy] = refreshCalls[0];
    expect(skillIds).toBeNull(); // null = every Managed skill
    expect(policy).toEqual({ reassert_auto_sync: true });
    expect(setup.sync.syncSkillsToTools).not.toHaveBeenCalled();
    // The whole pass ran as one action (the loading surface wraps it).
    expect(setup.reporter.runAction).toHaveBeenCalledTimes(1);
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      "status.refreshCompleted",
    );
  });

  it("passes the auto-sync setting as the re-assert policy", async () => {
    const setup = makeDeps({ autoSyncEnabled: false });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRefresh();
    });

    const [, , policy] = mockInvoke.mock.calls.find(
      ([command]) => command === "refreshManagedSkills",
    )!;
    expect(policy).toEqual({ reassert_auto_sync: false });
  });

  it("renders per-skill failures and per-target failures from the one report", async () => {
    const setup = makeDeps({
      skills: [skill("s1", "alpha"), skill("s2", "beta")],
      refreshReport: {
        skills: [
          {
            skill_id: "s1",
            skill_name: "alpha",
            status: {
              status: "refreshed",
              content_hash: null,
              source_revision: null,
              targets: [
                {
                  scope: { scope: "global", tool: "cursor" },
                  status: {
                    status: "failed",
                    error: { code: "OTHER", message: "boom" },
                  },
                },
                {
                  scope: { scope: "global", tool: "claude" },
                  status: { status: "skipped", reason: { reason: "link_follows_source" } },
                },
              ],
            },
          },
          {
            skill_id: "s2",
            skill_name: "beta",
            status: {
              status: "failed",
              error: { code: "GIT_CLONE_FAILED", kind: "unknown", detail: "boom" },
            },
          },
        ],
        refreshed: 1,
        failed: 1,
        target_failures: 1,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRefresh();
    });

    // Skips are not failures and stay silent.
    expect(setup.reporter.showActionErrors).toHaveBeenCalledTimes(1);
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      {
        title: 'errors.updateFailedTitle {"name":"beta"}',
        message: "formatted:GIT_CLONE_FAILED",
      },
      {
        title:
          'errors.propagationFailedTitle {"name":"alpha","tool":"CURSOR"}',
        message: "formatted:OTHER",
      },
    ]);
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      'status.refreshSummary {"refreshed":1,"failed":1}',
    );
  });
});

describe("useSkillLibrary single update", () => {
  it("is the same batch, of one", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);

    await act(async () => {
      result.current.handleUpdateSkill(setup.skills[0]);
    });

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "refreshManagedSkills",
        ["s1"],
        { reassert_auto_sync: true },
        expect.anything(),
      ),
    );
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      'status.updated {"name":"alpha"}',
    );
  });

  it("surfaces the skill's own failure from the report", async () => {
    const setup = makeDeps({
      refreshReport: {
        skills: [
          {
            skill_id: "s1",
            skill_name: "alpha",
            status: { status: "failed", error: { code: "GIT_CLONE_FAILED", kind: "unknown", detail: "boom" } },
          },
        ],
        refreshed: 0,
        failed: 1,
        target_failures: 0,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      result.current.handleUpdateSkill(setup.skills[0]);
    });

    await waitFor(() =>
      expect(setup.reporter.setError).toHaveBeenCalledWith(
        "formatted:GIT_CLONE_FAILED",
      ),
    );
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
  });
});

describe("useSkillLibrary per-tool toggle", () => {
  it("an unsynced tool syncs with overwrite-if-same-content", async () => {
    const setup = makeDeps({
      syncReport: {
        results: [
          {
            skill_id: "s1",
            skill_name: "alpha",
            tool: "cursor",
            status: { status: "synced", mode_used: "copy" },
          },
        ],
        synced: 1,
        skipped: 0,
        failed: 0,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      result.current.handleToggleToolForSkill(setup.skills[0], "cursor");
    });

    await waitFor(() =>
      expect(setup.sync.syncSkillsToTools).toHaveBeenCalledWith(
        [{ skill_id: "s1", name: "alpha", source_path: "/hub/alpha" }],
        ["cursor"],
        { overwriteIfSameContent: true },
      ),
    );
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      "status.syncEnabled",
    );
  });

  it("a synced tool unsyncs instead", async () => {
    const setup = makeDeps({ skills: [skill("s1", "alpha", ["claude"])] });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      result.current.handleToggleToolForSkill(setup.skills[0], "claude");
    });

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith(
        "unsyncSkillFromTool",
        "s1",
        "claude",
      ),
    );
    expect(setup.sync.syncSkillsToTools).not.toHaveBeenCalled();
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      "status.syncDisabled",
    );
  });

  it("a single toggle surfaces TARGET_EXISTS with the conflicting path", async () => {
    const setup = makeDeps({
      syncReport: {
        results: [
          {
            skill_id: "s1",
            skill_name: "alpha",
            tool: "cursor",
            status: {
              status: "skipped",
              error: { code: "TARGET_EXISTS", path: "/tools/cursor/alpha" },
            },
          },
        ],
        synced: 0,
        skipped: 1,
        failed: 0,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      result.current.handleToggleToolForSkill(setup.skills[0], "cursor");
    });

    await waitFor(() =>
      expect(setup.reporter.setError).toHaveBeenCalledWith(
        'errors.targetExistsDetail {"path":"/tools/cursor/alpha"}',
      ),
    );
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

describe("useSkillLibrary name collisions", () => {
  it("isSkillNameTaken matches case-insensitively", async () => {
    const setup = makeDeps({ skills: [skill("s1", "Alpha")] });
    const { result } = await renderLibrary(setup);

    expect(result.current.isSkillNameTaken("alpha")).toBe(true);
    expect(result.current.isSkillNameTaken("ALPHA")).toBe(true);
    expect(result.current.isSkillNameTaken("beta")).toBe(false);
  });
});
