// Tests at the SyncOrchestration seam: target defaulting from the saved
// global selection, shared-dir group expansion, the batch sync command's
// policy/channel wiring, and per-target failure surfacing. The backend is
// mocked at the invokeTauri module seam with per-command responses; the
// progress channel comes from the platform seam, stubbed with a minimal fake
// capturing onmessage. The Tool catalog is composed in, as App does.

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectSyncOutcome } from "../components/projects/types";
import type {
  BatchTargetOutcome,
  AppSettings,
  SyncProgressDto,
  ToolInfoDto,
  ToolStatusDto,
} from "../components/skills/types";
import type { StatusReporter } from "./useStatusReporter";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), info: vi.fn(), warning: vi.fn(), dismiss: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

// Minimal stand-in for Tauri's Channel: the platform seam builds one per
// batch with onmessage set; tests drive onmessage directly.
class FakeChannel<T> {
  onmessage: ((message: T) => void) | null = null;
}
vi.mock("../lib/platform", () => ({
  createProgressChannel: async <T,>(onMessage: (message: T) => void) => {
    const channel = new FakeChannel<T>();
    channel.onmessage = onMessage;
    return channel;
  },
}));

import { toast } from "sonner";
import { invokeTauri, type CommandName } from "../lib/tauri";
import { useSyncOrchestration } from "./useSyncOrchestration";
import { ActionExit, useStatusReporter } from "./useStatusReporter";
import { useSurfaceState } from "./useSurfaceState";
import { useToolCatalog } from "./useToolCatalog";

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
    readers: [],
    placement: { skills_dir: `~/.${key}/skills`, detect_dirs: [`~/.${key}`], supports_symlink: true },
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
    github_token_set: false,
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
  mockInvoke.mockImplementation((command, ...args) => {
    switch (command) {
      case "getSettings":
        return Promise.resolve(appSettings(overrides?.config));
      case "getToolStatus":
        return Promise.resolve({ ...TOOL_STATUS, ...overrides?.status });
      case "syncSkillsToTools":
        return Promise.resolve(
          overrides?.syncReport ?? [],
        );
      case "updateSetting": {
        // The command answers the effective snapshot it saved.
        const update = args[0] as { key: string; value: { selected_tools: string[]; scan_selected_only: boolean } };
        return Promise.resolve(
          update.key === "global_tool_config"
            ? appSettings({
                ...overrides?.config,
                global_selected_tools: update.value.selected_tools,
                scan_selected_tools_only: update.value.scan_selected_only,
              })
            : appSettings(overrides?.config),
        );
      }
      default:
        return Promise.resolve(undefined);
    }
  });
}

function makeReporter(): Pick<
  StatusReporter,
  | "loading"
  | "notify"
  | "notifyError"
  | "runAction"
  | "setActionMessage"
  | "setSuccessToastMessage"
> {
  return {
    loading: false,
    notify: vi.fn(),
    notifyError: vi.fn(),
    // Admits every action (the lifecycle's own rules are pinned in
    // useStatusReporter.test.ts and, composed, by the harness tests below).
    runAction: vi.fn(async (_opts, fn) => {
      const value = await fn({
        handOff: () => ActionExit.handOff(),
        fail: (message: string | null) => ActionExit.failed(message),
        uncancellable: <R,>(run: () => Promise<R>) => run(),
        cancelRequested: () => false,
      });
      return value instanceof ActionExit ? undefined : value;
    }) as StatusReporter["runAction"],
    setActionMessage: vi.fn(),
    setSuccessToastMessage: vi.fn(),
  };
}

/**
 * The sync world as App composes it: its dialogs and its two asks are
 * surface state. The read-outs are what App renders the modals from.
 */
function renderSync(reporter = makeReporter()) {
  return renderHook(() => {
    const surface = useSurfaceState();
    const catalog = useToolCatalog({ t });
    const sync = useSyncOrchestration({ t, reporter, surface, catalog });
    const sharedDirPending = surface.pendingAsk("sharedDir");
    const overwritePending = surface.pendingAsk("overwrite");
    return {
      ...sync,
      toolStatus: catalog.toolStatus,
      surface,
      showNewToolsModal: surface.isOpen("newTools"),
      sharedDirPending,
      cancelSharedDirConfirmation: () => sharedDirPending?.answer("declined"),
      overwritePending,
      // "Keep existing": the explicit button (App's OverwriteModal).
      keepExisting: () => overwritePending?.answer("declined"),
      // Escape / outside press: the operator's top-layer close.
      dismissOverwrite: () => surface.closeDialog("overwrite"),
    };
  });
}

beforeEach(() => {
  vi.clearAllMocks();
});
// No vitest globals: unmount explicitly so a surface's window listeners never
// outlive its test.
afterEach(cleanup);

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

  it("offers the announcement: it waits behind a dialog already open", async () => {
    stubBackend({ status: { newly_installed: ["goose"] } });
    const { result } = renderSync();
    act(() => result.current.surface.openDialog({ kind: "update" }));

    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    expect(result.current.showNewToolsModal).toBe(false);

    act(() => result.current.surface.closeDialog("update"));
    expect(result.current.showNewToolsModal).toBe(true);
  });
});

/** A deferred promise: the test settles a backend call when it chooses. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** The sync world over the real lifecycle (one mutation at a time, D19). */
function renderSyncWithLifecycle() {
  return renderHook(() => {
    const reporter = useStatusReporter(t);
    const surface = useSurfaceState();
    const catalog = useToolCatalog({ t });
    const sync = useSyncOrchestration({ t, reporter, surface, catalog });
    return { ...sync, reporter, toolStatus: catalog.toolStatus };
  });
}

describe("useSyncOrchestration harness configuration (Settings → Harnesses)", () => {
  const configWrites = () =>
    mockInvoke.mock.calls.filter(([command, update]) => command === "updateSetting" && (update as { key: string }).key === "global_tool_config").map(([, update]) => (update as { value: unknown }).value);

  it("turning a harness on writes the whole selection, starting from the detected set when none was saved", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    let saved = "";
    await act(async () => {
      saved = await result.current.setHarnessSelected("goose", true);
    });
    expect(saved).toBe("saved");
    // Registry order, the never-configured default (every detected harness) made explicit.
    expect(configWrites()).toEqual([{ selected_tools: ["claude", "pi", "cursor", "goose"], scan_selected_only: true }]);
    expect(result.current.effectiveSyncTargetIds).toEqual(["claude", "pi", "cursor", "goose"]);
    expect(result.current.readSavedSelection()).toEqual(["claude", "pi", "cursor", "goose"]);
    // It deploys nothing by itself (D48 asks first).
    expect(mockInvoke.mock.calls.some(([command]) => command === "syncSkillsToTools")).toBe(false);
  });

  it("the save is a lifecycle action: it holds the slot, and every other mutation is refused while it runs", async () => {
    stubBackend({ config: { global_selected_tools: ["claude"] } });
    const { result } = renderSyncWithLifecycle();
    await waitFor(() => expect(result.current.globalSelectedTools).toEqual(["claude"]));
    const base = mockInvoke.getMockImplementation()!;
    const write = deferred<AppSettings>();
    mockInvoke.mockImplementation((command, ...args) => (command === "updateSetting" ? write.promise : base(command, ...args)));

    let first: Promise<string> = Promise.resolve("");
    await act(async () => {
      first = result.current.setHarnessSelected("cursor", true);
    });
    // Held with the reason in words (the switches read `blockedReason`).
    expect(result.current.reporter.pending?.label).toBe("lifecycle.verbs.harnessSettings");
    expect(result.current.reporter.refusal).toEqual({ code: "MUTATION_RUNNING", running: "lifecycle.verbs.harnessSettings" });
    // Optimistic only once admitted.
    expect(result.current.globalSelectedTools).toEqual(["claude", "cursor"]);

    await act(async () => {
      write.resolve(appSettings({ global_selected_tools: ["claude", "cursor"] }));
      await first;
    });
    expect(result.current.reporter.pending).toBeNull();
  });

  it("two rapid toggles, the first write failing: the second is refused (not queued) and the UI equals the persisted selection", async () => {
    stubBackend({ config: { global_selected_tools: ["claude", "pi", "cursor"], scan_selected_tools_only: false } });
    const { result } = renderSyncWithLifecycle();
    await waitFor(() => expect(result.current.globalSelectedTools).toEqual(["claude", "pi", "cursor"]));
    const persisted = { selected: ["claude", "pi", "cursor"] };
    const base = mockInvoke.getMockImplementation()!;
    const write = deferred<AppSettings>();
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "getSettings") return Promise.resolve(appSettings({ global_selected_tools: persisted.selected, scan_selected_tools_only: false }));
      if (command === "updateSetting") return write.promise;
      return base(command, ...args);
    });

    let first: Promise<string> = Promise.resolve("");
    let second = "";
    await act(async () => {
      first = result.current.setHarnessSelected("cursor", false);
    });
    await act(async () => {
      second = await result.current.setHarnessSelected("pi", false);
    });
    // Refused with the reason; never sent, never queued.
    expect(second).toBe("refused");
    expect(vi.mocked(toast.info)).toHaveBeenCalledWith(
      'lifecycle.running {"action":"lifecycle.verbs.harnessSettings"}',
      expect.anything(),
    );
    expect(configWrites()).toEqual([{ selected_tools: ["claude", "pi"], scan_selected_only: false }]);

    let outcome = "";
    await act(async () => {
      write.reject({ code: "OTHER", message: "first write failed" });
      outcome = await first;
    });
    expect(outcome).toBe("failed");
    // The failed write's error, through the one failure dialect.
    expect(vi.mocked(toast.error)).toHaveBeenCalledTimes(1);
    // Rolled back to exactly what the backend holds; no write replays an intent.
    expect(result.current.globalSelectedTools).toEqual(persisted.selected);
    expect(result.current.effectiveSyncTargetIds).toEqual(persisted.selected);
    expect(result.current.readSavedSelection()).toEqual(persisted.selected);
    expect(configWrites()).toHaveLength(1);
    expect(result.current.reporter.pending).toBeNull();
  });

  it("a successful save adopts the backend's echoed configuration", async () => {
    stubBackend({ config: { global_selected_tools: ["claude"], scan_selected_tools_only: false } });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.globalSelectedTools).toEqual(["claude"]));
    const base = mockInvoke.getMockImplementation()!;
    // The backend normalises what it stores (here: drops an unknown key).
    mockInvoke.mockImplementation((command, ...args) =>
      command === "updateSetting"
        ? Promise.resolve(appSettings({ global_selected_tools: ["claude"], scan_selected_tools_only: true }))
        : base(command, ...args),
    );

    await act(async () => {
      await result.current.saveToolConfig({ selected: ["claude", "retired"], scanSelectedOnly: false });
    });
    expect(result.current.globalSelectedTools).toEqual(["claude"]);
    expect(result.current.scanSelectedToolsOnly).toBe(true);
    expect(result.current.syncTargets).toEqual({ claude: true, pi: false, cursor: false, goose: false });
  });

  it("the scan scope saves on its own, keeping the selection", async () => {
    stubBackend({ config: { global_selected_tools: ["cursor"], scan_selected_tools_only: true } });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.globalSelectedTools).toEqual(["cursor"]));

    await act(async () => {
      await result.current.saveToolConfig({ scanSelectedOnly: false });
    });
    expect(configWrites()).toEqual([{ selected_tools: ["cursor"], scan_selected_only: false }]);
    expect(result.current.scanSelectedToolsOnly).toBe(false);
  });

  it("a failed auto-deploy toggle keeps the setting and says why through notifyError (#68)", async () => {
    stubBackend({ config: { global_selected_tools: ["claude"] } });
    const reporter = makeReporter();
    const { result } = renderSync(reporter);
    await waitFor(() => expect(result.current.globalSelectedTools).toEqual(["claude"]));
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "updateSetting" ? Promise.reject({ code: "OTHER" }) : base(command, ...args),
    );
    const before = result.current.autoSyncEnabled;
    await act(async () => {
      await result.current.handleAutoSyncToggle(!before);
    });
    expect(reporter.notifyError).toHaveBeenCalledWith({ code: "OTHER" });
    expect(result.current.autoSyncEnabled).toBe(before);
  });

  it("a failed write rolls back only itself and says why through notifyError", async () => {
    stubBackend({ config: { global_selected_tools: ["claude"] } });
    const reporter = makeReporter();
    const { result } = renderSync(reporter);
    await waitFor(() => expect(result.current.globalSelectedTools).toEqual(["claude"]));
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "updateSetting" ? Promise.reject({ code: "OTHER" }) : base(command, ...args),
    );

    let saved = "";
    await act(async () => {
      saved = await result.current.setHarnessSelected("cursor", true);
    });
    expect(saved).toBe("failed");
    expect(reporter.notifyError).toHaveBeenCalledWith({ code: "OTHER" });
    expect(result.current.globalSelectedTools).toEqual(["claude"]);
    expect(result.current.scanScopeRevision).toBe(0);
  });

  it("save → scan → deploy: the import-plan read waits for the save to release the slot, and a deploy after it is admitted", async () => {
    stubBackend({ config: { global_selected_tools: ["claude"] } });
    const { result } = renderSyncWithLifecycle();
    await waitFor(() => expect(result.current.globalSelectedTools).toEqual(["claude"]));
    const base = mockInvoke.getMockImplementation()!;
    const write = deferred<AppSettings>();
    mockInvoke.mockImplementation((command, ...args) => (command === "updateSetting" ? write.promise : base(command, ...args)));

    let save: Promise<string> = Promise.resolve("");
    await act(async () => {
      save = result.current.setHarnessSelected("cursor", true);
    });
    // The scan the add/import world starts on the revision bump is a read: it waits its turn.
    const order: string[] = [];
    let scan: Promise<unknown> = Promise.resolve();
    await act(async () => {
      scan = result.current.reporter.runAction({ read: true, label: "Scan" }, async () => {
        order.push(`scan while pending=${String(result.current.reporter.pending?.label)}`);
      });
    });
    expect(order).toEqual([]);
    await act(async () => {
      write.resolve(appSettings({ global_selected_tools: ["claude", "cursor"] }));
      await save;
      await scan;
    });
    expect(result.current.scanScopeRevision).toBe(1);
    expect(order).toHaveLength(1);
    let deployed = false;
    await act(async () => {
      expect(await result.current.reporter.whenIdle()).toBe(true);
      deployed = (await result.current.reporter.runAction({ label: "Deploy" }, async () => true)) === true;
    });
    expect(deployed).toBe(true);
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

  it("follows the selection saved in Settings → Harnesses", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    await act(async () => {
      await result.current.saveToolConfig({ selected: ["claude"] });
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
      await result.current.saveToolConfig({ selected: ["claude"], scanSelectedOnly: true });
    });
    expect(result.current.scanScopeRevision).toBe(1);
    await act(async () => {
      await result.current.saveToolConfig({ selected: ["claude"], scanSelectedOnly: false });
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
      result.current.sharedDirPending!.answer("confirmed");
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
      error: { code: "TARGET_EXISTS", path: `/t/${tool_key}/${skill_id}`, modifiedAt: 1_000, sameContent: false },
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
    // One retry batch per asked pair set: s1 only on cursor, s2 only on
    // claude — never the skills × Tools product (pair identity).
    const retries = [[synced("s1", "cursor")], [synced("s2", "claude")]];
    let batch = 0;
    mockInvoke.mockImplementation((command) => {
      switch (command) {
        case "getSettings":
          return Promise.resolve(appSettings());
        case "getToolStatus":
          return Promise.resolve(TOOL_STATUS);
        case "syncSkillsToTools":
          batch += 1;
          return Promise.resolve(batch === 1 ? first : retries[batch - 2]);
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
    // Each row carries what the ask shows (D12): path, modified time, and
    // whether it differs — the backend's answers, never re-derived.
    const facts = { modifiedAt: 1_000, sameContent: false, scope: { kind: "global" } };
    expect(result.current.overwritePending!.rows).toEqual([
      { skillId: "s1", skillName: "s1", toolKey: "cursor", toolLabel: "CURSOR", path: "/t/cursor/s1", ...facts },
      { skillId: "s2", skillName: "s2", toolKey: "claude", toolLabel: "CLAUDE", path: "/t/claude/s2", ...facts },
    ]);

    let report: BatchTargetOutcome[] = [];
    await act(async () => {
      result.current.overwritePending!.answer("confirmed");
      report = await reportPromise;
    });

    const calls = syncCalls();
    expect(calls).toHaveLength(3);
    expect(calls.slice(1).map(([, retrySkills, retryTools, retryPolicy]) => [retrySkills, retryTools, retryPolicy])).toEqual([
      [[skills[0]], ["cursor"], { overwrite: false, overwrite_if_same_content: true, overrides: [{ skill_id: "s1", tool: "cursor", decision: "overwrite" }] }],
      [[skills[1]], ["claude"], { overwrite: false, overwrite_if_same_content: true, overrides: [{ skill_id: "s2", tool: "claude", decision: "overwrite" }] }],
    ]);
    // Asked rows replaced in place; the others are the first batch's rows.
    expect(report).toEqual([
      synced("s1", "claude"),
      synced("s1", "cursor"),
      synced("s2", "claude"),
      synced("s2", "cursor"),
    ]);
    expect(result.current.overwritePending).toBeNull();
  });

  it("skills asking about the same Tools share one retry batch", async () => {
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
              ? [occupied("s1", "claude"), occupied("s2", "claude")]
              : [synced("s1", "claude"), synced("s2", "claude")],
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
      result.current.overwritePending!.answer("confirmed");
      await reportPromise;
    });
    expect(syncCalls()).toHaveLength(2);
    expect(syncCalls()[1]!.slice(1, 3)).toEqual([skills, ["claude"]]);
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
      result.current.overwritePending!.answer("confirmed");
      await reportPromise;
    });
    const [, retrySkills, , retryPolicy] = syncCalls()[1]!;
    expect(retrySkills).toEqual([skills[1]]);
    expect(retryPolicy).toEqual({
      overwrite: false,
      overwrite_if_same_content: false,
      overrides: [{ skill_id: "s2", tool: "claude", decision: "overwrite" }],
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
      result.current.overwritePending!.answer("confirmed");
      await expect(reportPromise).rejects.toThrow("retry transport down");
    });
    expect(syncCalls()).toHaveLength(2);
    expect(result.current.overwritePending).toBeNull();
  });

  it("Keep existing retries the asked pairs with a keep decision", async () => {
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
      result.current.keepExisting();
      report = await reportPromise;
    });
    // The stub answers the retry with the same rows: the asked row is
    // replaced by the retry's row for that pair.
    expect(report).toEqual(first);
    const calls = syncCalls();
    expect(calls).toHaveLength(2);
    const [, retrySkills, retryTools, retryPolicy] = calls[1]!;
    expect(retrySkills).toEqual([skills[0]]);
    expect(retryTools).toEqual(["cursor"]);
    expect(retryPolicy).toEqual({
      overwrite: false,
      overwrite_if_same_content: false,
      overrides: [{ skill_id: "s1", tool: "cursor", decision: "keep" }],
    });
    expect(result.current.overwritePending).toBeNull();
  });

  it.each([
    ["Escape", "escape"],
    ["a displacing ask", "displaced"],
    ["the stack replaced", "replaced"],
  ] as const)("dismissed (%s), records nothing: no retry batch, no decision", async (_label, how) => {
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
      if (how === "escape") result.current.dismissOverwrite();
      if (how === "displaced") void result.current.surface.ask({ kind: "overwrite", rows: [] });
      if (how === "replaced") result.current.surface.openDialog({ kind: "addSkill" });
      report = await reportPromise;
    });
    // The report is returned as it settled: the occupied row stays failed.
    expect(report).toEqual(first);
    expect(syncCalls()).toHaveLength(1);
  });
});

describe("the one overwrite ask across producers (ticket 04)", () => {
  const facts = { modifiedAt: 5_000, sameContent: false };
  const globalRow = (skill_id: string, tool_key: string, collided: boolean): BatchTargetOutcome => ({
    skill_id,
    skill_name: skill_id,
    tool_key,
    status: collided
      ? { status: "failed", error: { code: "TARGET_EXISTS", path: `/g/${tool_key}/${skill_id}`, ...facts } }
      : { status: "synced", outcome: { mode_used: "symlink", target_path: `/g/${tool_key}/${skill_id}`, replaced: true } },
  });
  const projectItem = (status: ProjectSyncOutcome["status"]): ProjectSyncOutcome => ({
    assignment_id: "a1",
    project_id: "p1",
    skill_id: "s1",
    skill_name: "s1",
    tool: "claude",
    status,
  });
  const projectCollision = projectItem({
    status: "failed",
    error: { code: "TARGET_EXISTS", path: "/p1/.claude/skills/s1", ...facts },
  });

  /** Backend: settings + tools, plus the given per-command answers (in call order). */
  function backend(answers: Partial<Record<CommandName, unknown[]>>) {
    const queues = Object.fromEntries(Object.entries(answers).map(([k, v]) => [k, [...(v ?? [])]]));
    mockInvoke.mockImplementation((command) => {
      if (command === "getSettings") return Promise.resolve(appSettings());
      if (command === "getToolStatus") return Promise.resolve(TOOL_STATUS);
      const queue = queues[command];
      return Promise.resolve(queue?.length ? queue.shift() : undefined);
    });
  }
  const calls = (command: CommandName) => mockInvoke.mock.calls.filter(([cmd]) => cmd === command);

  it("answers per row: each asked pair is retried with its own decision", async () => {
    backend({
      syncSkillsToTools: [
        [globalRow("s1", "claude", true), globalRow("s2", "claude", true)],
        [globalRow("s1", "claude", false), { ...globalRow("s2", "claude", false), status: { status: "declined" } }],
      ],
    });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    let done!: Promise<BatchTargetOutcome[]>;
    act(() => {
      done = result.current.syncSkillsToTools(
        [
          { skill_id: "s1", name: "s1", source_path: "/r/s1" },
          { skill_id: "s2", name: "s2", source_path: "/r/s2" },
        ],
        ["claude"],
        { overwriteIfSameContent: true },
      );
    });
    await waitFor(() => expect(result.current.overwritePending?.rows).toHaveLength(2));
    let report: BatchTargetOutcome[] = [];
    await act(async () => {
      result.current.overwritePending!.answer({ choices: ["overwrite", "keep"] });
      report = await done;
    });
    expect(calls("syncSkillsToTools")[1]![3]).toEqual({
      overwrite: false,
      overwrite_if_same_content: true,
      overrides: [
        { skill_id: "s1", tool: "claude", decision: "overwrite" },
        { skill_id: "s2", tool: "claude", decision: "keep" },
      ],
    });
    expect(report.map((row) => row.status.status)).toEqual(["synced", "declined"]);
  });

  it("a project report's collisions are asked and resolved by resolveProjectCollisions", async () => {
    backend({ resolveProjectCollisions: [{ skipped_projects: [], items: [projectItem({ status: "declined" })] }] });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    let done!: ReturnType<typeof result.current.settleProjectCollisions>;
    act(() => {
      done = result.current.settleProjectCollisions({ skipped_projects: [], items: [projectCollision] });
    });
    await waitFor(() => expect(result.current.overwritePending).not.toBeNull());
    expect(result.current.overwritePending!.rows).toEqual([
      {
        skillId: "s1",
        skillName: "s1",
        toolKey: "claude",
        toolLabel: "CLAUDE",
        path: "/p1/.claude/skills/s1",
        ...facts,
        scope: { kind: "project", projectId: "p1" },
      },
    ]);
    let settled!: Awaited<typeof done>;
    await act(async () => {
      result.current.keepExisting();
      settled = await done;
    });
    expect(calls("resolveProjectCollisions")).toEqual([
      ["resolveProjectCollisions", [{ project_id: "p1", skill_id: "s1", tool: "claude", decision: "keep" }]],
    ]);
    expect(settled).toEqual({ report: { skipped_projects: [], items: [projectItem({ status: "declined" })] }, resolved: true });
  });

  it("a dismissed project ask resolves nothing", async () => {
    backend({});
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    let done!: ReturnType<typeof result.current.settleProjectCollisions>;
    act(() => {
      done = result.current.settleProjectCollisions({ skipped_projects: [], items: [projectCollision] });
    });
    await waitFor(() => expect(result.current.overwritePending).not.toBeNull());
    let settled!: Awaited<typeof done>;
    await act(async () => {
      result.current.dismissOverwrite();
      settled = await done;
    });
    expect(calls("resolveProjectCollisions")).toHaveLength(0);
    expect(settled.resolved).toBe(false);
  });

  it("Enable: global and project collisions are one ask; the retry is the same scope with the decisions", async () => {
    const scope = { kind: "skill", skill_id: "s1" } as const;
    backend({
      enableSkillTargets: [
        { global: [globalRow("s1", "claude", true), globalRow("s1", "pi", false)], projects: { skipped_projects: [], items: [projectCollision] } },
        { global: [globalRow("s1", "claude", false)], projects: { skipped_projects: [], items: [] } },
      ],
      resolveProjectCollisions: [{ skipped_projects: [], items: [projectItem({ status: "synced" })] }],
    });
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());
    let done!: ReturnType<typeof result.current.enableSkillTargets>;
    act(() => {
      done = result.current.enableSkillTargets(scope);
    });
    await waitFor(() => expect(result.current.overwritePending?.rows).toHaveLength(2));
    let report!: Awaited<typeof done>;
    await act(async () => {
      result.current.overwritePending!.answer("confirmed");
      report = await done;
    });
    expect(calls("enableSkillTargets")).toEqual([
      ["enableSkillTargets", scope, []],
      ["enableSkillTargets", scope, [{ skill_id: "s1", tool: "claude", decision: "overwrite" }]],
    ]);
    expect(calls("resolveProjectCollisions")[0]![1]).toEqual([
      { project_id: "p1", skill_id: "s1", tool: "claude", decision: "overwrite" },
    ]);
    expect(report.global.map((row) => [row.tool_key, row.status.status])).toEqual([
      ["claude", "synced"],
      ["pi", "synced"],
    ]);
    expect(report.projects.items.map((item) => item.status.status)).toEqual(["synced"]);
  });
});
