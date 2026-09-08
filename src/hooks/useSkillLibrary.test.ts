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

describe("invocation Edits", () => {
  it("replaces the returned DTO in place and closes without a refetch", async () => {
    const setup = makeDeps({ skills: [skill("s1", "alpha"), skill("s2", "beta")] });
    const { result } = await renderLibrary(setup);
    act(() => result.current.openInvocationEdit("s1"));
    const updated: ManagedSkill = { ...setup.skills[0], invocation_mode: "user-only", invocation_override: { mode: "user-only", base_mode: "user-and-model", conflict: false } };
    mockInvoke.mockResolvedValueOnce(updated);
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

  it.each(["refresh", "update"])("shows conflict warning data after %s", async (action) => {
    const report = refreshedReport(["alpha"]);
    const status = report.skills[0].status;
    if (status.status !== "refreshed") throw new Error("fixture");
    status.edit_conflict = { base_mode: "user-and-model", upstream_mode: "model-only", override_mode: "user-only" };
    const setup = makeDeps({ refreshReport: report });
    const { result } = await renderLibrary(setup);
    await act(async () => {
      if (action === "refresh") await result.current.handleRefresh();
      else await result.current.handleRestoreSkill(setup.skills[0]);
    });
    expect(setup.reporter.showActionWarnings).toHaveBeenCalledWith([{
      title: 'invocationEdit.warningTitle {"name":"alpha"}',
      message: 'invocationEdit.refreshWarning {"name":"alpha","upstream":"invocationMode.modelOnly","override":"invocationMode.userOnly"}',
    }]);
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(expect.objectContaining({ kind: "warning" }));
  });
});

describe("useSkillLibrary refresh", () => {
  it.each(["git", "GitHub"])("offers Re-point only for a known %s skill's GitHub-not-found failure", async (source_type) => {
    const gitSkill = { ...skill("s1", "alpha"), source_type };
    const otherGit = { ...skill("s2", "beta"), source_type: "git" };
    const localSkill = skill("s3", "local");
    const setup = makeDeps({
      skills: [gitSkill, otherGit, localSkill],
      refreshReport: {
        skills: [
          { skill_id: "s1", skill_name: "alpha", status: { status: "failed", error: { code: "GITHUB_SKILL_NOT_FOUND", url: "https://github.com/old/repo" } } },
          { skill_id: "s2", skill_name: "beta", status: { status: "failed", error: { code: "OTHER", message: "network failed" } } },
          { skill_id: "s3", skill_name: "local", status: { status: "failed", error: { code: "GITHUB_SKILL_NOT_FOUND", url: "https://github.com/old/repo" } } },
          { skill_id: "gone", skill_name: "gone", status: { status: "failed", error: { code: "GITHUB_SKILL_NOT_FOUND", url: "https://github.com/old/repo" } } },
        ],
        refreshed: 0, failed: 4, skipped: 0, target_failures: 0,
      },
    });
    const { result } = renderHook(() => useSkillLibrary(setup.deps));
    await waitFor(() => expect(result.current.managedSkills).toHaveLength(3));
    await act(async () => { await result.current.handleRefresh(); });

    const entries = vi.mocked(setup.reporter.showActionErrors).mock.calls[0][0];
    expect(entries).toHaveLength(4);
    expect(entries[0].action?.label).toBe("gitRepoint.action");
    expect(entries.slice(1).every((entry) => entry.action === undefined)).toBe(true);
    expect(result.current.pendingGitRepointSkill).toBeNull();
    act(() => entries[0].action?.onClick());
    expect(result.current.pendingGitRepointSkill).toEqual(gitSkill);
    expect(mockInvoke.mock.calls.map(([command]) => command)).not.toContain("repointGitSkillSource");
  });

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

  it("issues one backend batch for every skill and never fans out a sync itself", async () => {
    const setup = makeDeps({
      skills: [skill("s1", "alpha"), skill("s2", "beta")],
    });
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
                  status: {
                    status: "skipped",
                    reason: { reason: "link_follows_source" },
                  },
                },
              ],
              reassert_error: null,
              edit_conflict: null,
            },
          },
          {
            skill_id: "s2",
            skill_name: "beta",
            status: {
              status: "failed",
              error: {
                code: "GIT_CLONE_FAILED",
                kind: "unknown",
                detail: "boom",
              },
            },
          },
        ],
        refreshed: 1,
        failed: 1,
        skipped: 0,
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
        title: 'errors.propagationFailedTitle {"name":"alpha","tool":"CURSOR"}',
        message: "formatted:OTHER",
      },
    ]);
    // A batch that finished with failures is a warning (lingers, unread),
    // not a success.
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith({
      kind: "warning",
      title: 'status.refreshSummary {"refreshed":1,"failed":1}',
    });
  });

  it("reports a failed auto-sync re-assert even though the skill refreshed", async () => {
    const setup = makeDeps({
      refreshReport: {
        skills: [
          {
            skill_id: "s1",
            skill_name: "alpha",
            status: {
              status: "refreshed",
              content_hash: null,
              source_revision: null,
              targets: [],
              reassert_error: { code: "OTHER", message: "store is gone" },
              edit_conflict: null,
            },
          },
        ],
        refreshed: 1,
        failed: 0,
        skipped: 0,
        target_failures: 1,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRefresh();
    });

    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      {
        title: 'errors.reassertFailedTitle {"name":"alpha"}',
        message: "formatted:OTHER",
      },
    ]);
  });

  it("reports skipped Unlocatable skills as a warning summary with one entry per skill", async () => {
    const setup = makeDeps({
      skills: [skill("s1", "alpha"), skill("s2", "beta"), skill("s3", "gamma")],
      refreshReport: {
        skills: [
          {
            skill_id: "s1",
            skill_name: "alpha",
            status: {
              status: "refreshed",
              content_hash: null,
              source_revision: null,
              targets: [],
              reassert_error: null,
              edit_conflict: null,
            },
          },
          {
            skill_id: "s2",
            skill_name: "beta",
            status: { status: "skipped", state: "source_missing" },
          },
          {
            skill_id: "s3",
            skill_name: "gamma",
            status: { status: "skipped", state: "central_missing" },
          },
        ],
        refreshed: 1,
        failed: 0,
        skipped: 2,
        target_failures: 0,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRefresh();
    });

    // Skips are not failures: nothing goes through the error batch...
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([]);
    // ...but each skipped skill is its own warning row in the panel.
    expect(setup.reporter.showActionWarnings).toHaveBeenCalledWith([
      {
        title: 'errors.refreshSkippedTitle {"name":"beta"}',
        message: "errors.refreshSkippedSourceMissing",
      },
      {
        title: 'errors.refreshSkippedTitle {"name":"gamma"}',
        message: "errors.refreshSkippedCentralMissing",
      },
    ]);
    // The summary is a warning that counts the skipped.
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith({
      kind: "warning",
      title:
        'status.refreshSummarySkipped {"refreshed":1,"failed":0,"skipped":2}',
    });
  });
});

describe("useSkillLibrary unlocatable skill actions", () => {
  it.each([
    ["git", true], ["git", false], ["GitHub", true], ["GitHub", false],
  ] as const)("%s Re-point passes auto-sync=%s, submits the new URL and reloads", async (source_type, autoSyncEnabled) => {
    const setup = makeDeps({ autoSyncEnabled });
    const gitSkill = { ...setup.skills[0], source_type };
    const { result } = await renderLibrary(setup);
    await act(async () => { await result.current.handleRepointSkill(gitSkill); });
    expect(result.current.pendingGitRepointSkill).toEqual(gitSkill);
    expect(mockInvoke.mock.calls.map(([command]) => command)).not.toContain("repointGitSkillSource");
    await act(async () => { await result.current.handleConfirmRepointGitSkill("https://github.com/new/repo/tree/main/skill"); });
    expect(mockInvoke).toHaveBeenCalledWith("repointGitSkillSource", "s1", "https://github.com/new/repo/tree/main/skill", { reassert_auto_sync: autoSyncEnabled });
    expect(result.current.pendingGitRepointSkill).toBeNull();
    expect(mockInvoke.mock.calls.at(-1)?.[0]).toBe("getManagedSkills");
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith('status.repointed {"name":"alpha"}');
  });

  it("keeps git Re-point open until the request succeeds", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);
    const gitSkill = { ...setup.skills[0], source_type: "git" };
    act(() => result.current.handleRepointGitSkill(gitSkill));
    let resolve!: (report: RefreshReportDto) => void;
    const response = new Promise<RefreshReportDto>((done) => { resolve = done; });
    mockInvoke.mockImplementation((command) => command === "repointGitSkillSource"
      ? response : Promise.resolve(setup.skills));
    let pending!: Promise<void>;
    act(() => { pending = result.current.handleConfirmRepointGitSkill("https://github.com/new/repo"); });
    expect(result.current.pendingGitRepointSkill).toEqual(gitSkill);
    await act(async () => { resolve(refreshedReport(["alpha"])); await pending; });
    expect(result.current.pendingGitRepointSkill).toBeNull();
  });

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

  it.each(["refused", "acquisition failed"])("git Re-point %s surfaces the error and reloads", async (failure) => {
    const setup = makeDeps();
    mockInvoke.mockImplementation((command) => {
      if (command === "repointGitSkillSource") {
        if (failure === "refused") return Promise.reject({ code: "GIT_REPOINT_REQUIRES_GIT", name: "alpha" });
        return Promise.resolve({
          skills: [{ skill_id: "s1", skill_name: "alpha", status: {
            status: "failed", error: { code: "GITHUB_SKILL_NOT_FOUND", url: "https://github.com/new/repo" },
          } }], refreshed: 0, failed: 1, skipped: 0, target_failures: 0,
        } satisfies RefreshReportDto);
      }
      return Promise.resolve(setup.skills);
    });
    const { result } = await renderLibrary(setup);
    act(() => result.current.handleRepointGitSkill({ ...setup.skills[0], source_type: "git" }));
    await act(async () => { await result.current.handleConfirmRepointGitSkill("https://github.com/new/repo"); });
    expect(result.current.pendingGitRepointSkill).toEqual({ ...setup.skills[0], source_type: "git" });
    expect(setup.reporter.setError).toHaveBeenCalledWith(failure === "refused"
      ? "formatted:GIT_REPOINT_REQUIRES_GIT" : "formatted:GITHUB_SKILL_NOT_FOUND");
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
    expect(mockInvoke.mock.calls.at(-1)?.[0]).toBe("getManagedSkills");
  });

  it.each([true, false])("local Re-point passes auto-sync=%s, picks a folder and reloads", async (autoSyncEnabled) => {
    const setup = makeDeps({ autoSyncEnabled });
    pickFolder.mockResolvedValue("/new/place/alpha");
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRepointSkill(setup.skills[0]);
    });

    expect(pickFolder).toHaveBeenCalledWith(
      expect.objectContaining({ directory: true, multiple: false }),
    );
    expect(mockInvoke).toHaveBeenCalledWith(
      "repointLocalSkillSource",
      "s1",
      "/new/place/alpha",
      { reassert_auto_sync: autoSyncEnabled },
    );
    const calls = mockInvoke.mock.calls.map(([command]) => command);
    expect(calls.indexOf("getManagedSkills", 1)).toBeGreaterThan(
      calls.indexOf("repointLocalSkillSource"),
    );
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      'status.repointed {"name":"alpha"}',
    );
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

  it("Re-point surfaces a refused folder as the action's failure", async () => {
    const setup = makeDeps();
    pickFolder.mockResolvedValue("/home/u/.claude/skills/alpha");
    mockInvoke.mockImplementation((command) => {
      if (command === "repointLocalSkillSource") {
        return Promise.reject({
          code: "LOCAL_SOURCE_INSIDE_TOOL_DIR",
          path: "/home/u/.claude/skills/alpha",
          tool: "claude_code",
        });
      }
      return Promise.resolve(setup.skills);
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRepointSkill(setup.skills[0]);
    });

    expect(setup.reporter.setError).toHaveBeenCalledWith(
      "formatted:LOCAL_SOURCE_INSIDE_TOOL_DIR",
    );
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
  });

  it("Re-point surfaces the Update's own failure from the report", async () => {
    const setup = makeDeps();
    pickFolder.mockResolvedValue("/new/place/alpha");
    mockInvoke.mockImplementation((command) => {
      if (command === "repointLocalSkillSource") {
        return Promise.resolve({
          skills: [
            {
              skill_id: "s1",
              skill_name: "alpha",
              status: {
                status: "failed",
                error: { code: "SKILL_INVALID", reason: "missing_skill_md" },
              },
            },
          ],
          refreshed: 0,
          failed: 1,
          skipped: 0,
          target_failures: 0,
        } satisfies RefreshReportDto);
      }
      return Promise.resolve(setup.skills);
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRepointSkill(setup.skills[0]);
    });

    expect(setup.reporter.setError).toHaveBeenCalledWith(
      "formatted:SKILL_INVALID",
    );
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
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

  it("Restore is the single-skill Update with its own copy", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRestoreSkill(setup.skills[0]);
    });

    expect(mockInvoke).toHaveBeenCalledWith(
      "refreshManagedSkills",
      ["s1"],
      { reassert_auto_sync: true },
      expect.anything(),
    );
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      'status.restored {"name":"alpha"}',
    );
  });

  it("Restore surfaces the skill's own failure from the report", async () => {
    const setup = makeDeps({
      refreshReport: {
        skills: [
          {
            skill_id: "s1",
            skill_name: "alpha",
            status: {
              status: "failed",
              error: { code: "SOURCE_PATH_MISSING", path: "/old/alpha" },
            },
          },
        ],
        refreshed: 0,
        failed: 1,
        skipped: 0,
        target_failures: 0,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRestoreSkill(setup.skills[0]);
    });

    expect(setup.reporter.setError).toHaveBeenCalledWith(
      "formatted:SOURCE_PATH_MISSING",
    );
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
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
            status: {
              status: "failed",
              error: {
                code: "GIT_CLONE_FAILED",
                kind: "unknown",
                detail: "boom",
              },
            },
          },
        ],
        refreshed: 0,
        failed: 1,
        skipped: 0,
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

describe("useSkillLibrary unsync", () => {
  it("reports how many deployments were removed", async () => {
    const setup = makeDeps({
      removalReport: removedReport(["claude", "cursor"]),
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleUnsyncAll();
    });

    expect(mockInvoke).toHaveBeenCalledWith("unsyncAllSkills");
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      'unsyncAllComplete {"count":2}',
    );
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([]);
  });

  it("surfaces every path it could not remove instead of a silent count", async () => {
    const setup = makeDeps({
      removalReport: {
        targets: [
          {
            scope: { scope: "global" },
            tool: "claude",
            path: "/tools/claude/alpha",
            status: { status: "removed" },
          },
          {
            scope: { scope: "global" },
            tool: "cursor",
            path: "/tools/cursor/alpha",
            status: {
              status: "failed",
              error: { code: "OTHER", message: "permission denied" },
            },
          },
        ],
        removed: 1,
        failed: 1,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleUnsyncAll();
    });

    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith({
      kind: "warning",
      title: 'unsyncPartial {"count":1,"failed":1}',
    });
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      {
        title: 'errors.unsyncFailedTitle {"tool":"CURSOR"}',
        message: "formatted:OTHER",
      },
    ]);
  });

  it("unsyncing one skill reports its failed targets too", async () => {
    const setup = makeDeps({
      removalReport: {
        targets: [
          {
            scope: { scope: "global" },
            tool: "claude",
            path: "/tools/claude/alpha",
            status: {
              status: "failed",
              error: { code: "OTHER", message: "busy" },
            },
          },
        ],
        removed: 0,
        failed: 1,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleUnsyncSkill("s1");
    });

    expect(mockInvoke).toHaveBeenCalledWith("unsyncSkill", "s1");
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      {
        title: 'errors.unsyncFailedTitle {"tool":"CLAUDE"}',
        message: "formatted:OTHER",
      },
    ]);
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

  it("a failed unsync toggle fails the action instead of reporting success", async () => {
    const setup = makeDeps({
      skills: [skill("s1", "alpha", ["claude"])],
      removalReport: {
        targets: [
          {
            scope: { scope: "global" },
            tool: "claude",
            path: "/tools/claude/alpha",
            status: {
              status: "failed",
              error: { code: "OTHER", message: "permission denied" },
            },
          },
        ],
        removed: 0,
        failed: 1,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      result.current.handleToggleToolForSkill(setup.skills[0], "claude");
    });

    await waitFor(() =>
      expect(setup.reporter.setError).toHaveBeenCalledWith("formatted:OTHER"),
    );
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
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
