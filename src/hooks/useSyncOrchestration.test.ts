// Tests at the SyncOrchestration seam: target defaulting from the saved
// global selection, shared-dir group expansion, the batch sync command's
// policy/channel wiring, and per-target failure surfacing. The backend is
// mocked at the invokeTauri module seam with per-command responses; the
// Tauri Channel is a minimal fake capturing onmessage.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  BatchTargetOutcome,
  AppSettings,
  SyncProgressDto,
  ToolInfoDto,
  ToolStatusDto,
} from "../components/skills/types";
import type { StatusReporter } from "./useStatusReporter";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

// Minimal stand-in for Tauri's Channel: the hook only sets onmessage and
// passes the instance to the command; tests drive onmessage directly.
class FakeChannel<T> {
  onmessage: ((message: T) => void) | null = null;
}
vi.mock("@tauri-apps/api/core", () => ({ Channel: FakeChannel }));

import { invokeTauri, type CommandName } from "../lib/tauri";
import { useSyncOrchestration } from "./useSyncOrchestration";

// The seam is generic over the command table; the stub switches on the
// command name, so it is typed loosely (positional args, unknown result).
const mockInvoke = vi.mocked(
  invokeTauri as unknown as (
    command: CommandName,
    ...args: unknown[]
  ) => Promise<unknown>,
);

// Mimics i18next for a catalog-less test: a missing key falls back to the
// caller-provided defaultValue (how tool labels resolve), everything else
// renders key + params so assertions can see both.
const t = (key: string, opts?: Record<string, unknown>) => {
  if (opts && "defaultValue" in opts) return String(opts.defaultValue);
  return opts ? `${key} ${JSON.stringify(opts)}` : key;
};

function toolInfo(key: string, shared_with: string[] = [key]): ToolInfoDto {
  return {
    key,
    label: key.toUpperCase(),
    installed: true,
    shared_with,
    constituents: [],
  };
}

// claude and pi share a skills dir; cursor stands alone; goose is known
// but not installed.
const TOOL_STATUS: ToolStatusDto = {
  tools: [
    toolInfo("claude", ["claude", "pi"]),
    toolInfo("pi", ["claude", "pi"]),
    toolInfo("cursor"),
    { ...toolInfo("goose"), installed: false },
  ],
  installed: ["claude", "pi", "cursor"],
  newly_installed: [],
};

// Only the fields this hook reads; the rest of the snapshot is filler.
function appSettings(overrides?: Partial<AppSettings>): AppSettings {
  return {
    central_repo_path: "/tmp/central",
    git_cache_cleanup_days: 30,
    git_cache_ttl_secs: 60,
    github_token: "",
    auto_sync_enabled: true,
    global_selected_tools: null,
    global_selected_tools_corrupt: false,
    scan_selected_tools_only: true,
    ui_zoom_level: 1,
    bounds: {
      git_cache_cleanup_days: { min: 0, max: 3650 },
      git_cache_ttl_secs: { min: 0, max: 3600 },
      ui_zoom_level: { min: 0.5, max: 3 },
    },
    ...overrides,
  };
}

function stubBackend(overrides?: {
  config?: Partial<
    Pick<
      AppSettings,
      | "global_selected_tools"
      | "global_selected_tools_corrupt"
      | "scan_selected_tools_only"
    >
  >;
  status?: Partial<ToolStatusDto>;
  syncReport?: BatchTargetOutcome[];
}) {
  mockInvoke.mockImplementation((command) => {
    switch (command) {
      case "getSettings":
        return Promise.resolve(appSettings(overrides?.config));
      case "getToolStatus":
        return Promise.resolve({ ...TOOL_STATUS, ...overrides?.status });
      case "syncSkillsToTools":
        return Promise.resolve(
          overrides?.syncReport ?? [],
        );
      default:
        return Promise.resolve(undefined);
    }
  });
}

function makeReporter(): Pick<
  StatusReporter,
  | "loading"
  | "notify"
  | "setActionMessage"
  | "setError"
  | "setSuccessToastMessage"
  | "formatError"
> {
  return {
    loading: false,
    notify: vi.fn(),
    setActionMessage: vi.fn(),
    setError: vi.fn(),
    setSuccessToastMessage: vi.fn(),
    // Same shape as the real formatError contract: null silences an entry.
    formatError: vi.fn((err: unknown) => {
      const code = (err as { code?: string })?.code;
      return code === "CANCELLED" ? null : `formatted:${code}`;
    }),
  };
}

function renderSync(reporter = makeReporter()) {
  return renderHook(() => useSyncOrchestration({ t, reporter }));
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("useSyncOrchestration target defaulting", () => {
  it("defaults sync targets to the installed tools when nothing is saved", async () => {
    stubBackend();
    const { result } = renderSync();

    await waitFor(() =>
      expect(result.current.syncTargets).toEqual({
        claude: true,
        pi: true,
        cursor: true,
        goose: false,
      }),
    );
    expect(result.current.showNewToolsModal).toBe(false);
  });

  it("defaults sync targets to the saved selection when one exists", async () => {
    stubBackend({
      config: {
        global_selected_tools: ["cursor"],
        scan_selected_tools_only: true,
      },
    });
    const { result } = renderSync();

    await waitFor(() =>
      expect(result.current.syncTargets).toEqual({
        claude: false,
        pi: false,
        cursor: true,
        goose: false,
      }),
    );
  });

  it("warns once at startup when the saved selection is corrupt, and defaults like unconfigured", async () => {
    // The backend reads a corrupt row as `null` for display and flags it;
    // syncs will refuse with SETTING_CORRUPT until the operator re-saves.
    stubBackend({
      config: { global_selected_tools: null, global_selected_tools_corrupt: true },
    });
    const reporter = makeReporter();
    const { result } = renderSync(reporter);

    await waitFor(() =>
      expect(result.current.syncTargets).toEqual({
        claude: true,
        pi: true,
        cursor: true,
        goose: false,
      }),
    );
    expect(reporter.notify).toHaveBeenCalledTimes(1);
    expect(reporter.notify).toHaveBeenCalledWith("warning", "errors.settingCorruptStartup");
  });

  it("stays quiet at startup when the saved selection reads cleanly", async () => {
    stubBackend({ config: { global_selected_tools: ["cursor"] } });
    const reporter = makeReporter();
    const { result } = renderSync(reporter);
    await waitFor(() => expect(result.current.globalSelectedTools).toEqual(["cursor"]));
    expect(reporter.notify).not.toHaveBeenCalled();
  });

  it("scan-selected-only hides newly installed tools outside the selection", async () => {
    stubBackend({
      config: {
        global_selected_tools: ["cursor"],
        scan_selected_tools_only: true,
      },
      status: { newly_installed: ["goose"] },
    });
    const { result } = renderSync();

    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    expect(result.current.showNewToolsModal).toBe(false);
    expect(result.current.relevantNewlyInstalled).toEqual([]);
  });

  it("surfaces newly installed tools when scanning is unrestricted", async () => {
    stubBackend({ status: { newly_installed: ["goose"] } });
    const { result } = renderSync();

    await waitFor(() => expect(result.current.showNewToolsModal).toBe(true));
    expect(result.current.relevantNewlyInstalled).toEqual(["goose"]);
  });
});

// Mirrors the backend rule (settings::effective_global_tool_targets):
// the recorded selection wins, detection is only the never-configured
// fallback, and the set is never intersected with detection.
describe("useSyncOrchestration effective sync targets", () => {
  it("falls back to the installed tools when no selection was ever saved", async () => {
    stubBackend();
    const { result } = renderSync();

    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    expect(result.current.effectiveSyncTargetIds).toEqual([
      "claude",
      "pi",
      "cursor",
    ]);
  });

  it("takes the saved selection verbatim, keeping an uninstalled entry", async () => {
    stubBackend({
      config: {
        global_selected_tools: ["cursor", "goose"],
        scan_selected_tools_only: true,
      },
    });
    const { result } = renderSync();

    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    // goose is not installed and survives: the backend reports it as a skip,
    // which is the operator's only signal that the selection is stale.
    expect(result.current.effectiveSyncTargetIds).toEqual(["cursor", "goose"]);
  });

  it("an empty saved selection means sync nowhere, never a fallback", async () => {
    stubBackend({
      config: { global_selected_tools: [], scan_selected_tools_only: true },
    });
    const { result } = renderSync();

    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    expect(result.current.effectiveSyncTargetIds).toEqual([]);
  });

  it("follows the selection saved through the tool config modal", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    await act(async () => {
      await result.current.handleToolConfigConfirm(["claude"]);
    });

    expect(result.current.effectiveSyncTargetIds).toEqual(["claude"]);
  });

  it("bumps the scan-scope revision on every saved configuration", async () => {
    // The onboarding plan's scope is resolved by the backend from the saved
    // configuration; the add/import world reloads its plan on this signal.
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    expect(result.current.scanScopeRevision).toBe(0);

    await act(async () => {
      await result.current.handleToolConfigConfirm(["claude"], true);
    });
    expect(result.current.scanScopeRevision).toBe(1);
    await act(async () => {
      await result.current.handleToolConfigConfirm(["claude"], false);
    });
    expect(result.current.scanScopeRevision).toBe(2);
  });
});

describe("useSyncOrchestration shared-dir groups", () => {
  it("toggling a shared-dir tool toggles its whole group after confirm", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    act(() => {
      void result.current.handleSyncTargetChange("claude", false);
    });

    // The modal owns the decision; nothing changes until it is answered.
    expect(result.current.sharedDirPending?.toolKey).toBe("claude");
    expect(result.current.sharedDirPending?.labels).toEqual(["PI"]);
    expect(result.current.syncTargets.claude).toBe(true);

    await act(async () => {
      result.current.sharedDirPending!.resolve(true);
    });

    expect(result.current.sharedDirPending).toBeNull();
    expect(result.current.syncTargets.claude).toBe(false);
    expect(result.current.syncTargets.pi).toBe(false);
    expect(result.current.syncTargets.cursor).toBe(true);
  });

  it("a declined confirm leaves the targets untouched", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    act(() => {
      void result.current.handleSyncTargetChange("pi", false);
    });
    await act(async () => {
      result.current.cancelSharedDirConfirmation();
    });

    expect(result.current.syncTargets.claude).toBe(true);
    expect(result.current.syncTargets.pi).toBe(true);
  });

  it("a standalone tool toggles without confirmation", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    await act(async () => {
      await result.current.handleSyncTargetChange("cursor", false);
    });

    expect(result.current.sharedDirPending).toBeNull();
    expect(result.current.syncTargets.cursor).toBe(false);
  });

  it("enableTargetsFor expands to the shared-dir group", async () => {
    stubBackend({
      config: { global_selected_tools: [], scan_selected_tools_only: true },
    });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    // Saved empty selection → everything off.
    expect(result.current.syncTargets.claude).toBe(false);

    act(() => {
      result.current.enableTargetsFor(["claude"]);
    });

    expect(result.current.syncTargets.claude).toBe(true);
    expect(result.current.syncTargets.pi).toBe(true);
    expect(result.current.syncTargets.cursor).toBe(false);
  });
});

describe("syncSkillsToTools", () => {
  it("sends the wire policy with defaults and a progress channel", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    const skills = [
      { skill_id: "s1", name: "Skill One", source_path: "/repo/s1" },
    ];
    await act(async () => {
      await result.current.syncSkillsToTools(skills, ["claude"]);
    });

    const call = mockInvoke.mock.calls.find(
      ([cmd]) => cmd === "syncSkillsToTools",
    );
    expect(call).toBeDefined();
    const [, sentSkills, tools, policy, onProgress] = call!;
    expect(sentSkills).toBe(skills);
    expect(tools).toEqual(["claude"]);
    expect(policy).toEqual({
      overwrite: false,
      overwrite_if_same_content: false,
      overrides: [],
    });
    expect(onProgress).toBeInstanceOf(FakeChannel);
  });

  it("streams progress into the reporter with localized tool labels", async () => {
    stubBackend();
    const reporter = makeReporter();
    const { result } = renderSync(reporter);
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    await act(async () => {
      await result.current.syncSkillsToTools([], ["claude"]);
    });
    const call = mockInvoke.mock.calls.find(
      ([cmd]) => cmd === "syncSkillsToTools",
    );
    const channel = call![4] as FakeChannel<SyncProgressDto>;

    act(() => {
      channel.onmessage!({
        index: 2,
        total: 5,
        skill_name: "Skill One",
        tool: "claude",
      });
    });

    expect(reporter.setActionMessage).toHaveBeenCalledWith(
      'actions.syncStep {"index":2,"total":5,"name":"Skill One","tool":"CLAUDE"}',
    );
  });
});

describe("syncSkillsToTools overwrite ask", () => {
  const skills = [
    { skill_id: "s1", name: "Skill One", source_path: "/repo/s1" },
    { skill_id: "s2", name: "Skill Two", source_path: "/repo/s2" },
  ];
  const synced = (skill_id: string, tool_key: string): BatchTargetOutcome => ({
    skill_id,
    skill_name: skill_id,
    tool_key,
    status: {
      status: "synced",
      outcome: { mode_used: "symlink", target_path: `/t/${tool_key}/${skill_id}`, replaced: false },
    },
  });
  const occupied = (skill_id: string, tool_key: string): BatchTargetOutcome => ({
    skill_id,
    skill_name: skill_id,
    tool_key,
    status: {
      status: "failed",
      error: { code: "TARGET_EXISTS", path: `/t/${tool_key}/${skill_id}` },
    },
  });
  const syncCalls = () =>
    mockInvoke.mock.calls.filter(([cmd]) => cmd === "syncSkillsToTools");

  it("returns a report without occupied targets after one batch and no ask", async () => {
    stubBackend({ syncReport: [synced("s1", "claude")] });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    let report: BatchTargetOutcome[] = [];
    await act(async () => {
      report = await result.current.syncSkillsToTools([skills[0]!], ["claude"]);
    });
    expect(report).toEqual([synced("s1", "claude")]);
    expect(syncCalls()).toHaveLength(1);
    expect(result.current.overwritePending).toBeNull();
  });

  it("raises the ask with the occupied rows and, confirmed, retries exactly those pairs", async () => {
    const first = [
      synced("s1", "claude"),
      occupied("s1", "cursor"),
      occupied("s2", "claude"),
      synced("s2", "cursor"),
    ];
    const retry = [
      synced("s1", "claude"), // cross pair, re-synced by the retry batch: dropped
      synced("s1", "cursor"),
      synced("s2", "claude"),
      synced("s2", "cursor"),
    ];
    let batch = 0;
    mockInvoke.mockImplementation((command) => {
      switch (command) {
        case "getSettings":
          return Promise.resolve(appSettings());
        case "getToolStatus":
          return Promise.resolve(TOOL_STATUS);
        case "syncSkillsToTools":
          batch += 1;
          return Promise.resolve(batch === 1 ? first : retry);
        default:
          return Promise.resolve(undefined);
      }
    });
    const reporter = makeReporter();
    const { result } = renderSync(reporter);
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    let reportPromise!: Promise<BatchTargetOutcome[]>;
    act(() => {
      reportPromise = result.current.syncSkillsToTools(skills, ["claude", "cursor"], {
        overwriteIfSameContent: true,
      });
    });
    await waitFor(() => expect(result.current.overwritePending).not.toBeNull());
    expect(reporter.setActionMessage).toHaveBeenCalledWith("overwrite.waiting");
    expect(result.current.overwritePending!.rows).toEqual([
      { skillId: "s1", skillName: "s1", toolKey: "cursor", toolLabel: "CURSOR", path: "/t/cursor/s1" },
      { skillId: "s2", skillName: "s2", toolKey: "claude", toolLabel: "CLAUDE", path: "/t/claude/s2" },
    ]);

    let report: BatchTargetOutcome[] = [];
    await act(async () => {
      result.current.overwritePending!.resolve(true);
      report = await reportPromise;
    });

    const calls = syncCalls();
    expect(calls).toHaveLength(2);
    const [, retrySkills, retryTools, retryPolicy] = calls[1]!;
    expect(retrySkills).toEqual(skills);
    expect(retryTools).toEqual(["cursor", "claude"]);
    expect(retryPolicy).toEqual({
      overwrite: false,
      overwrite_if_same_content: true,
      overrides: [
        { skill_id: "s1", tool: "cursor", overwrite: true },
        { skill_id: "s2", tool: "claude", overwrite: true },
      ],
    });
    // Asked rows replaced in place; the others are the first batch's rows.
    expect(report).toEqual([
      synced("s1", "claude"),
      synced("s1", "cursor"),
      synced("s2", "claude"),
      synced("s2", "cursor"),
    ]);
    expect(result.current.overwritePending).toBeNull();
  });

  it("retries only the affected skills, carrying the caller's same-content rule", async () => {
    let batch = 0;
    mockInvoke.mockImplementation((command) => {
      switch (command) {
        case "getSettings":
          return Promise.resolve(appSettings());
        case "getToolStatus":
          return Promise.resolve(TOOL_STATUS);
        case "syncSkillsToTools":
          batch += 1;
          return Promise.resolve(
            batch === 1
              ? [synced("s1", "claude"), occupied("s2", "claude")]
              : [synced("s2", "claude")],
          );
        default:
          return Promise.resolve(undefined);
      }
    });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    let reportPromise!: Promise<BatchTargetOutcome[]>;
    act(() => {
      reportPromise = result.current.syncSkillsToTools(skills, ["claude"]);
    });
    await waitFor(() => expect(result.current.overwritePending).not.toBeNull());
    await act(async () => {
      result.current.overwritePending!.resolve(true);
      await reportPromise;
    });
    const [, retrySkills, , retryPolicy] = syncCalls()[1]!;
    expect(retrySkills).toEqual([skills[1]]);
    expect(retryPolicy).toEqual({
      overwrite: false,
      overwrite_if_same_content: false,
      overrides: [{ skill_id: "s2", tool: "claude", overwrite: true }],
    });
  });

  it("a thrown retry propagates as a whole-command error (the caller reloads)", async () => {
    let batch = 0;
    mockInvoke.mockImplementation((command) => {
      switch (command) {
        case "getSettings":
          return Promise.resolve(appSettings());
        case "getToolStatus":
          return Promise.resolve(TOOL_STATUS);
        case "syncSkillsToTools":
          batch += 1;
          return batch === 1
            ? Promise.resolve([synced("s1", "claude"), occupied("s1", "cursor")])
            : Promise.reject(new Error("retry transport down"));
        default:
          return Promise.resolve(undefined);
      }
    });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    let reportPromise!: Promise<BatchTargetOutcome[]>;
    act(() => {
      reportPromise = result.current.syncSkillsToTools([skills[0]!], ["claude", "cursor"]);
      // A rejection is expected; attach the handler before the ask resolves.
      reportPromise.catch(() => undefined);
    });
    await waitFor(() => expect(result.current.overwritePending).not.toBeNull());
    await act(async () => {
      result.current.overwritePending!.resolve(true);
      await expect(reportPromise).rejects.toThrow("retry transport down");
    });
    expect(syncCalls()).toHaveLength(2);
    expect(result.current.overwritePending).toBeNull();
  });

  it("declined, returns the first report unchanged after one batch", async () => {
    const first = [synced("s1", "claude"), occupied("s1", "cursor")];
    stubBackend({ syncReport: first });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    let reportPromise!: Promise<BatchTargetOutcome[]>;
    act(() => {
      reportPromise = result.current.syncSkillsToTools([skills[0]!], ["claude", "cursor"]);
    });
    await waitFor(() => expect(result.current.overwritePending).not.toBeNull());
    let report: BatchTargetOutcome[] = [];
    await act(async () => {
      result.current.cancelOverwriteConfirmation();
      report = await reportPromise;
    });
    expect(report).toEqual(first);
    expect(syncCalls()).toHaveLength(1);
    expect(result.current.overwritePending).toBeNull();
  });
});
