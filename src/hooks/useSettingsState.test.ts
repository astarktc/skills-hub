// Tests at the SettingsState seam: one settings snapshot load on mount
// (`get_settings`), writes through `update_setting` adopting the effective
// values the backend returns, and numeric clamps driven by the DTO bounds
// rather than literals. The backend is mocked at the invokeTauri module seam.

import { act, renderHook, waitFor } from "@testing-library/react";
import i18next from "i18next";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AppSettings,
  LibraryMovePreview,
  LibraryMoveProgress,
  LibraryMoveReport,
  LibraryMoveResultDto,
  SettingUpdate,
} from "../components/skills/types";
import { resources } from "../i18n/resources";
import { describeRefusal } from "../lib/actionLifecycle";
import {
  ActionExit,
  useStatusReporter,
  type StatusReporter,
  type TranslateFn,
} from "./useStatusReporter";
import type { AskAnswer, SurfaceState } from "./useSurfaceState";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
    dismiss: vi.fn(),
  }),
}));

vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

// The folder picker, webview zoom and progress channels cross the platform seam.
const pickFolder = vi.fn<(options: { title: string; multiple?: boolean }) => Promise<string[]>>();
const setZoom = vi.fn<(factor: number) => Promise<void>>();
vi.mock("../lib/platform", () => ({
  pickFolder: (options: { title: string; multiple?: boolean }) => pickFolder(options),
  setZoom: (factor: number) => setZoom(factor),
  createProgressChannel: async (onMessage: (message: unknown) => void) => ({ onmessage: onMessage }),
}));

import { invokeTauri, type CommandName } from "../lib/tauri";
import { useSettingsState } from "./useSettingsState";

// The seam is generic over the command table; the stub switches on the
// command name, so it is typed loosely (positional args, unknown result).
const mockInvoke = vi.mocked(
  invokeTauri as unknown as (
    command: CommandName,
    ...args: unknown[]
  ) => Promise<unknown>,
);

const t = (key: string) => key;

// Deliberately non-production bounds so a literal-based clamp would fail.
const BOUNDS: AppSettings["bounds"] = {
  git_cache_cleanup_days: { min: 0, max: 10 },
  git_cache_ttl_secs: { min: 0, max: 20 },
  ui_zoom_level: { min: 0.75, max: 1.5 },
};

function appSettings(overrides?: Partial<AppSettings>): AppSettings {
  return {
    central_repo_path: "/home/op/.skillshub",
    git_cache_cleanup_days: 7,
    git_cache_ttl_secs: 12,
    github_token_set: true,
    auto_sync_enabled: true,
    global_selected_tools: null,
    global_selected_tools_corrupt: false,
    scan_selected_tools_only: true,
    ui_zoom_level: 1.25,
    bounds: BOUNDS,
    ...overrides,
  };
}

/** Echo backend: `updateSetting` applies the update onto the snapshot. */
function stubBackend(initial = appSettings()) {
  let current = initial;
  mockInvoke.mockImplementation((command, ...args) => {
    switch (command) {
      case "getSettings":
        return Promise.resolve(current);
      case "updateSetting": {
        const [update] = args as [SettingUpdate];
        switch (update.key) {
          case "git_cache_cleanup_days":
            current = { ...current, git_cache_cleanup_days: update.value };
            break;
          case "git_cache_ttl_secs":
            current = { ...current, git_cache_ttl_secs: update.value };
            break;
          case "github_token":
            // The secret goes to the keychain; only presence echoes back.
            current = { ...current, github_token_set: update.value.trim() !== "" };
            break;
          case "ui_zoom_level":
            current = { ...current, ui_zoom_level: update.value };
            break;
          default:
            break;
        }
        return Promise.resolve(current);
      }
      default:
        return Promise.resolve(undefined);
    }
  });
}

// The one failure dialect: `notifyError(err)` is an error Notification of
// the formatted copy — the stub records it through `notify`.
function makeReporter(): Pick<
  StatusReporter,
  "notifyError" | "setSuccessToastMessage" | "notify" | "runAction" | "reportProgress" | "applyOutcome"
> {
  const notify = vi.fn();
  return {
    notifyError: vi.fn((err: unknown) => notify("error", `formatted:${String(err)}`)),
    setSuccessToastMessage: vi.fn(),
    notify,
    // A pass-through lifecycle: these tests are about the settings writes.
    runAction: vi.fn(async (_opts, fn) => {
      const outcome = await fn({
        handOff: () => ActionExit.handOff(),
        fail: (message: string | null) => ActionExit.failed(message),
        uncancellable: <R,>(run: () => Promise<R>) => run(),
        cancelRequested: () => false,
      });
      return outcome instanceof ActionExit ? undefined : outcome;
    }) as StatusReporter["runAction"],
    reportProgress: vi.fn(),
    applyOutcome: vi.fn(async () => ({ reload: false, closeModal: false, conflict: false })),
  };
}

const noSurface = (): Pick<SurfaceState, "ask"> => ({
  ask: vi.fn(async () => "dismissed" as AskAnswer) as unknown as SurfaceState["ask"],
});

function renderSettings(reporter = makeReporter()) {
  return renderHook(() =>
    useSettingsState({
      t,
      reporter,
      surface: noSurface(),
      library: { replaceCatalog: vi.fn(), loadManagedSkills: vi.fn(async () => {}) },
      projects: { reloadViews: vi.fn(async () => {}) },
    }),
  );
}

type SettingsResult = { current: ReturnType<typeof useSettingsState> };

/** Type a draft, then press Save (two acts: the save reads the new draft). */
async function saveToken(result: SettingsResult, draft: string) {
  act(() => {
    result.current.handleGithubTokenDraftChange(draft);
  });
  await act(async () => {
    await result.current.handleGithubTokenSave();
  });
}

const updateCalls = () =>
  mockInvoke.mock.calls
    .filter(([command]) => command === "updateSetting")
    .map(([, update]) => update as SettingUpdate);

// jsdom lacks the browser theme surfaces the hook touches on mount; the
// frontend-only theme preference is out of scope here, so stub minimally.
function stubThemeSurfaces() {
  const memory = new Map<string, string>();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => memory.get(key) ?? null,
      setItem: (key: string, value: string) => void memory.set(key, value),
    },
  });
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    value: () => ({
      matches: false,
      addEventListener: () => {},
      removeEventListener: () => {},
    }),
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  stubThemeSurfaces();
});

describe("useSettingsState load", () => {
  it("loads the whole snapshot with a single get_settings call", async () => {
    stubBackend();
    const { result } = renderSettings();

    await waitFor(() => {
      expect(result.current.storagePath).toBe("/home/op/.skillshub");
    });
    expect(result.current.gitCacheCleanupDays).toBe(7);
    expect(result.current.gitCacheTtlSecs).toBe(12);
    expect(result.current.githubTokenSet).toBe(true);
    expect(result.current.zoomLevel).toBe(1.25);
    expect(result.current.bounds).toEqual(BOUNDS);

    const commands = mockInvoke.mock.calls.map(([command]) => command);
    expect(commands.filter((c) => c === "getSettings")).toHaveLength(1);
    expect(commands).not.toContain("updateSetting");
  });

  it("reports a failed load through the reporter", async () => {
    mockInvoke.mockRejectedValue(new Error("boom"));
    const reporter = makeReporter();
    renderSettings(reporter);

    await waitFor(() => {
      expect(reporter.notify).toHaveBeenCalledWith("error", "formatted:Error: boom");
    });
  });
});

describe("useSettingsState writes", () => {
  it("clamps cache knobs to the DTO bounds before sending", async () => {
    stubBackend();
    const { result } = renderSettings();
    await waitFor(() => expect(result.current.bounds).not.toBeNull());

    await act(async () => {
      await result.current.handleGitCacheCleanupDaysChange(999);
    });
    await act(async () => {
      await result.current.handleGitCacheTtlSecsChange(-5);
    });

    expect(updateCalls()).toEqual([
      { key: "git_cache_cleanup_days", value: 10 },
      { key: "git_cache_ttl_secs", value: 0 },
    ]);
    expect(result.current.gitCacheCleanupDays).toBe(10);
    expect(result.current.gitCacheTtlSecs).toBe(0);
  });

  it("adopts the effective value the backend returns", async () => {
    stubBackend();
    const { result } = renderSettings();
    await waitFor(() => expect(result.current.bounds).not.toBeNull());

    // Remove sends a blank value; presence comes from the backend echo.
    await act(async () => {
      await result.current.handleGithubTokenRemove();
    });

    expect(updateCalls()).toEqual([{ key: "github_token", value: "" }]);
    expect(result.current.githubTokenSet).toBe(false);
  });

  it("sets and replaces the token by presence only, never holding the secret", async () => {
    stubBackend(appSettings({ github_token_set: false }));
    const { result } = renderSettings();
    await waitFor(() => expect(result.current.bounds).not.toBeNull());
    expect(result.current.githubTokenSet).toBe(false);

    await saveToken(result, "ghp_new");
    expect(result.current.githubTokenSet).toBe(true);
    // The draft clears once the save is confirmed: the hook holds no secret.
    expect(result.current.githubTokenDraft).toBe("");
    expect(JSON.stringify(result.current)).not.toContain("ghp_new");

    await saveToken(result, "ghp_replacement");
    expect(updateCalls()).toEqual([
      { key: "github_token", value: "ghp_new" },
      { key: "github_token", value: "ghp_replacement" },
    ]);
    expect(result.current.githubTokenSet).toBe(true);
  });

  it("reports a keychain failure and keeps the previous presence", async () => {
    stubBackend();
    const reporter = makeReporter();
    const { result } = renderSettings(reporter);
    await waitFor(() => expect(result.current.bounds).not.toBeNull());

    const failure = { code: "CREDENTIAL_STORE_UNAVAILABLE", detail: "locked" };
    mockInvoke.mockRejectedValueOnce(failure);
    await act(async () => {
      await result.current.handleGithubTokenRemove();
    });
    expect(reporter.notify).toHaveBeenCalledWith("error", `formatted:${String(failure)}`);
    expect(result.current.githubTokenSet).toBe(true);

    // A failed save keeps the draft for a retry.
    mockInvoke.mockRejectedValueOnce(failure);
    await saveToken(result, "ghp_retry");
    expect(result.current.githubTokenDraft).toBe("ghp_retry");
    expect(result.current.githubTokenPending).toBe(false);
  });

  it("reports an unreadable keychain as unknown presence, not absent", async () => {
    stubBackend(appSettings({ github_token_set: null }));
    const { result } = renderSettings();
    await waitFor(() => expect(result.current.bounds).not.toBeNull());
    expect(result.current.githubTokenSet).toBeNull();
  });

  it("warns, keeps the draft and claims nothing when a save cannot be read back", async () => {
    stubBackend(appSettings({ github_token_set: false }));
    const reporter = makeReporter();
    const { result } = renderSettings(reporter);
    await waitFor(() => expect(result.current.bounds).not.toBeNull());

    // The keychain accepted the write, but the follow-up read was denied.
    mockInvoke.mockResolvedValueOnce(appSettings({ github_token_set: null }));
    await saveToken(result, "ghp_unverified");

    expect(result.current.githubTokenSet).toBeNull();
    expect(result.current.githubTokenDraft).toBe("ghp_unverified");
    expect(reporter.notify).toHaveBeenCalledWith(
      "warning",
      "githubTokenUnconfirmedTitle",
      "githubTokenUnconfirmedMessage",
    );
    expect(reporter.setSuccessToastMessage).not.toHaveBeenCalled();
    expect(reporter.notifyError).not.toHaveBeenCalled();
  });
});

describe("useSettingsState token single-flight", () => {
  it("ignores Save/Remove while a token write is pending and keeps a newer draft", async () => {
    stubBackend(appSettings({ github_token_set: false }));
    const { result } = renderSettings();
    await waitFor(() => expect(result.current.bounds).not.toBeNull());

    // Hold the first token write open (a keychain prompt).
    let release: ((value: AppSettings) => void) | undefined;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise<AppSettings>((resolve) => {
          release = resolve;
        }),
    );

    act(() => {
      result.current.handleGithubTokenDraftChange("ghp_A");
    });
    let first: Promise<void> | undefined;
    act(() => {
      first = result.current.handleGithubTokenSave();
    });
    expect(result.current.githubTokenPending).toBe(true);

    // While A is pending: a newer draft, a second Save and a Remove.
    act(() => {
      result.current.handleGithubTokenDraftChange("ghp_B");
    });
    await act(async () => {
      await result.current.handleGithubTokenSave();
      await result.current.handleGithubTokenRemove();
    });
    expect(updateCalls()).toEqual([{ key: "github_token", value: "ghp_A" }]);

    await act(async () => {
      release?.(appSettings({ github_token_set: true }));
      await first;
    });

    expect(result.current.githubTokenPending).toBe(false);
    expect(result.current.githubTokenSet).toBe(true);
    // A landed, but the input now holds B, which was never sent: keep it.
    expect(result.current.githubTokenDraft).toBe("ghp_B");

    // After settling, the next Save goes through and clears its own draft.
    await act(async () => {
      await result.current.handleGithubTokenSave();
    });
    expect(updateCalls()).toEqual([
      { key: "github_token", value: "ghp_A" },
      { key: "github_token", value: "ghp_B" },
    ]);
    expect(result.current.githubTokenDraft).toBe("");
  });

  it("surfaces a failed write through the reporter and keeps other state", async () => {
    stubBackend();
    const reporter = makeReporter();
    const { result } = renderSettings(reporter);
    await waitFor(() => expect(result.current.bounds).not.toBeNull());

    mockInvoke.mockRejectedValueOnce(new Error("disk"));
    await act(async () => {
      await result.current.handleGitCacheTtlSecsChange(3);
    });

    expect(reporter.notify).toHaveBeenCalledWith("error", "formatted:Error: disk");
    expect(result.current.storagePath).toBe("/home/op/.skillshub");
  });
});

describe("useSettingsState single-field adoption", () => {
  it("does not let a slow write's snapshot clobber a newer field edit", async () => {
    // Two overlapping writes: the TTL write resolves *after* a later token
    // edit. Its response snapshot still carries the old token, so adopting the
    // whole snapshot would roll the token back.
    let releaseTtlWrite: ((value: AppSettings) => void) | undefined;
    const initial = appSettings({ github_token_set: false });
    mockInvoke.mockImplementation((command, ...args) => {
      switch (command) {
        case "getSettings":
          return Promise.resolve(initial);
        case "updateSetting": {
          const [update] = args as [SettingUpdate];
          if (update.key === "git_cache_ttl_secs") {
            return new Promise<AppSettings>((resolve) => {
              releaseTtlWrite = resolve;
            });
          }
          if (update.key === "github_token") {
            return Promise.resolve(
              appSettings({ github_token_set: true, git_cache_ttl_secs: 5 }),
            );
          }
          return Promise.resolve(initial);
        }
        default:
          return Promise.resolve(undefined);
      }
    });

    const { result } = renderSettings();
    await waitFor(() => expect(result.current.bounds).not.toBeNull());
    expect(result.current.githubTokenSet).toBe(false);

    // Start the slow TTL write, then complete a token write behind its back.
    let ttlWrite: Promise<void> | undefined;
    await act(async () => {
      ttlWrite = result.current.handleGitCacheTtlSecsChange(5);
    });
    await saveToken(result, "new-token");
    expect(result.current.githubTokenSet).toBe(true);

    // The TTL response (which still says no token) lands last.
    await act(async () => {
      releaseTtlWrite?.(appSettings({ github_token_set: false, git_cache_ttl_secs: 5 }));
      await ttlWrite;
    });

    expect(result.current.gitCacheTtlSecs).toBe(5);
    expect(result.current.githubTokenSet).toBe(true);
  });
});

describe("useSettingsState open log folder", () => {
  it("asks the backend to open the app log folder", async () => {
    stubBackend();
    const reporter = makeReporter();
    const { result } = renderSettings(reporter);
    await waitFor(() => expect(result.current.bounds).not.toBeNull());

    await act(async () => {
      await result.current.handleOpenLogFolder();
    });

    expect(mockInvoke).toHaveBeenCalledWith("openLogFolder");
    expect(reporter.notifyError).not.toHaveBeenCalled();
  });

  it("surfaces a failure to open through the reporter", async () => {
    stubBackend();
    const reporter = makeReporter();
    const { result } = renderSettings(reporter);
    await waitFor(() => expect(result.current.bounds).not.toBeNull());
    mockInvoke.mockImplementation((command) =>
      command === "openLogFolder"
        ? Promise.reject({ code: "OTHER", message: "no opener" })
        : Promise.resolve(undefined),
    );

    await act(async () => {
      await result.current.handleOpenLogFolder();
    });

    expect(reporter.notify).toHaveBeenCalledWith("error", "formatted:[object Object]",
    );
  });
});

describe("useSettingsState storage folder and zoom (platform seam)", () => {
  it("a cancelled picker runs nothing", async () => {
    stubBackend();
    const { result } = renderSettings();
    await waitFor(() => expect(result.current.storagePath).toBe("/home/op/.skillshub"));
    pickFolder.mockResolvedValueOnce([]);
    await act(async () => {
      await result.current.handlePickStoragePath();
    });
    expect(updateCalls()).toEqual([]);
    expect(commandsCalled()).not.toContain("previewLibraryMove");
  });

  it("a picker failure lands on the error surface", async () => {
    stubBackend();
    const reporter = makeReporter();
    const { result } = renderSettings(reporter);
    await waitFor(() => expect(result.current.storagePath).toBe("/home/op/.skillshub"));
    pickFolder.mockRejectedValueOnce(new Error("dialog failed"));
    await act(async () => {
      await result.current.handlePickStoragePath();
    });
    expect(reporter.notify).toHaveBeenCalledWith("error", "formatted:Error: dialog failed");
    expect(updateCalls()).toEqual([]);
  });

  it("zooms the webview to the clamped level and saves it", async () => {
    stubBackend();
    setZoom.mockResolvedValue(undefined);
    const { result } = renderSettings();
    await waitFor(() => expect(result.current.bounds).not.toBeNull());
    await act(async () => {
      await result.current.handleZoomLevelChange(3);
    });
    expect(setZoom).toHaveBeenCalledWith(1.5);
    expect(updateCalls()).toEqual([{ key: "ui_zoom_level", value: 1.5 }]);
    expect(result.current.zoomLevel).toBe(1.5);
  });
});

describe("useSettingsState resolved theme", () => {
  function prefersDark(matches: boolean) {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: () => ({
        matches,
        addEventListener: () => {},
        removeEventListener: () => {},
      }),
    });
  }

  it("resolves 'system' against the media query from the first render", () => {
    stubBackend();
    prefersDark(true);
    const { result } = renderSettings();

    expect(result.current.themePreference).toBe("system");
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");
  });

  it("an explicit choice wins over the system and is the one value written", () => {
    stubBackend();
    prefersDark(true);
    const { result } = renderSettings();

    act(() => result.current.handleThemeChange("light"));

    expect(result.current.resolvedTheme).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.style.colorScheme).toBe("light");
  });
});

// ---------------------------------------------------------------------------
// Move Library folder (round 20 #03): Browse… → preview (a read) → the
// confirmation → the move under the real lifecycle.
// ---------------------------------------------------------------------------

const commandsCalled = () => mockInvoke.mock.calls.map(([command]) => command);

const PREVIEW: LibraryMovePreview = {
  destination: "/home/op/Library2",
  skills: 3,
  missing: 1,
  kept: 0,
  cloud_sync: null,
};

const MOVED: LibraryMoveResultDto = {
  report: {
    from: "/home/op/.skillshub",
    to: "/home/op/Library2",
    skills: [],
    deployments: [],
    left_behind: [{ path: "/home/op/.skillshub", reason: "old_library", skill_id: null, error: null }],
  },
  skills: [],
};

/** The backend for a move: `moveLibrary` waits until `finish()`. */
function stubMoveBackend() {
  stubBackend();
  let finish: () => void = () => {};
  let progress: ((tick: LibraryMoveProgress) => void) | null = null;
  const base = mockInvoke.getMockImplementation()!;
  mockInvoke.mockImplementation((command, ...args) => {
    if (command === "previewLibraryMove") return Promise.resolve(PREVIEW);
    if (command === "moveLibrary") {
      const [, channel] = args as [string, { onmessage: (tick: LibraryMoveProgress) => void }];
      progress = channel.onmessage;
      return new Promise((resolve) => (finish = () => resolve(MOVED)));
    }
    return base(command, ...args);
  });
  return {
    finish: () => finish(),
    tick: (tick: LibraryMoveProgress) => progress?.(tick),
  };
}

function renderMove(answer: (preview: LibraryMovePreview) => Promise<AskAnswer>) {
  const ask = vi.fn(async (request: { kind: "moveLibrary"; preview: LibraryMovePreview }) =>
    answer(request.preview),
  );
  const replaceCatalog = vi.fn();
  const loadManagedSkills = vi.fn(async () => {});
  const reloadViews = vi.fn(async () => {});
  const rendered = renderHook(() => {
    const reporter = useStatusReporter(t);
    const settings = useSettingsState({
      t,
      reporter,
      surface: { ask: ask as unknown as SurfaceState["ask"] },
      library: { replaceCatalog, loadManagedSkills },
      projects: { reloadViews },
    });
    return { reporter, settings };
  });
  return { ...rendered, ask, replaceCatalog, loadManagedSkills, reloadViews };
}

describe("useSettingsState Move Library folder", () => {
  it("Browse… asks with the backend's preview, then moves under the lifecycle", async () => {
    const backend = stubMoveBackend();
    const { result, ask, replaceCatalog } = renderMove(async () => "confirmed");
    await waitFor(() => expect(result.current.settings.storagePath).toBe("/home/op/.skillshub"));
    pickFolder.mockResolvedValueOnce(["/home/op/Library2"]);

    let done: Promise<void> = Promise.resolve();
    act(() => {
      done = result.current.settings.handlePickStoragePath();
    });
    await waitFor(() => expect(commandsCalled()).toContain("moveLibrary"));

    expect(ask).toHaveBeenCalledWith({ kind: "moveLibrary", preview: PREVIEW });
    expect(mockInvoke).toHaveBeenCalledWith("previewLibraryMove", "/home/op/Library2");
    expect(mockInvoke).toHaveBeenCalledWith("moveLibrary", "/home/op/Library2", expect.anything());
    // It runs as the one pending mutation, not cancellable, with header progress.
    expect(result.current.reporter.pending).toMatchObject({
      label: "libraryMove.label",
      activity: "libraryMove.activity",
      cancellable: false,
    });
    act(() => backend.tick({ phase: "copying", done: 1, total: 3, name: "beta" }));
    expect(result.current.reporter.pending?.progress).toEqual({ done: 1, total: 3 });
    expect(result.current.reporter.actionMessage).toBe("libraryMove.phase.copying");

    await act(async () => {
      backend.finish();
      await done;
    });
    expect(replaceCatalog).toHaveBeenCalledWith(MOVED.skills);
    expect(result.current.settings.storagePath).toBe("/home/op/Library2");
    expect(result.current.reporter.pending).toBeNull();
    expect(updateCalls()).toEqual([]);
  });

  it("cancelling the confirmation runs nothing", async () => {
    stubMoveBackend();
    const { result, ask, replaceCatalog } = renderMove(async () => "declined");
    await waitFor(() => expect(result.current.settings.storagePath).toBe("/home/op/.skillshub"));
    pickFolder.mockResolvedValueOnce(["/home/op/Library2"]);

    await act(async () => {
      await result.current.settings.handlePickStoragePath();
    });

    expect(ask).toHaveBeenCalledTimes(1);
    expect(commandsCalled()).not.toContain("moveLibrary");
    expect(replaceCatalog).not.toHaveBeenCalled();
    expect(result.current.settings.storagePath).toBe("/home/op/.skillshub");
  });

  it("is refused while another action runs", async () => {
    stubMoveBackend();
    let release: () => void = () => {};
    let other: Promise<unknown> = Promise.resolve();
    // Another mutation starts while the confirmation is open; then Move.
    const { result } = renderMove(async () => {
      act(() => {
        other = result.current.reporter.runAction(
          { label: "Refresh" },
          () => new Promise<void>((resolve) => (release = resolve)),
        );
      });
      return "confirmed";
    });
    await waitFor(() => expect(result.current.settings.storagePath).toBe("/home/op/.skillshub"));
    pickFolder.mockResolvedValueOnce(["/home/op/Library2"]);

    await act(async () => {
      await result.current.settings.handlePickStoragePath();
    });

    expect(commandsCalled()).not.toContain("moveLibrary");
    expect(result.current.settings.storagePath).toBe("/home/op/.skillshub");
    await act(async () => {
      release();
      await other;
    });
  });

  it("a refused destination is reported and asks nothing", async () => {
    stubBackend();
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "previewLibraryMove"
        ? Promise.reject({
            code: "LIBRARY_DESTINATION_REFUSED",
            path: "/home/op/Documents",
            reason: "notEmpty",
            conflict: null,
          })
        : base(command, ...args),
    );
    const { result, ask } = renderMove(async () => "confirmed");
    await waitFor(() => expect(result.current.settings.storagePath).toBe("/home/op/.skillshub"));
    pickFolder.mockResolvedValueOnce(["/home/op/Documents"]);

    await act(async () => {
      await result.current.settings.handlePickStoragePath();
    });

    expect(ask).not.toHaveBeenCalled();
    expect(commandsCalled()).not.toContain("moveLibrary");
    const { toast } = await import("sonner");
    expect(vi.mocked(toast.error)).toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Round-4 regressions (the round-3 reviews' S1, S2, S3).
// ---------------------------------------------------------------------------

/** Confirm a move to Library2 and wait for it to settle. */
async function moveToLibrary2(result: { current: { settings: ReturnType<typeof useSettingsState> } }) {
  await waitFor(() => expect(result.current.settings.storagePath).toBe("/home/op/.skillshub"));
  pickFolder.mockResolvedValueOnce(["/home/op/Library2"]);
  await act(async () => {
    await result.current.settings.handlePickStoragePath();
  });
}

describe("useSettingsState after a Library move settles", () => {
  // S1: a failure after the switch (dropping the journal, reading the
  // catalog) used to leave Settings on the old folder and the inspector on
  // removed paths.
  it("a move that throws re-reads the Library folder and the catalog", async () => {
    stubBackend();
    const base = mockInvoke.getMockImplementation()!;
    let settingsNow = appSettings();
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "getSettings") return Promise.resolve(settingsNow);
      if (command === "previewLibraryMove") return Promise.resolve(PREVIEW);
      if (command === "moveLibrary") {
        // Switched, then failed reading the catalog.
        settingsNow = appSettings({ central_repo_path: "/home/op/Library2" });
        return Promise.reject({ code: "OTHER", message: "database is locked" });
      }
      return base(command, ...args);
    });
    const { result, replaceCatalog, loadManagedSkills } = renderMove(async () => "confirmed");

    await moveToLibrary2(result);

    expect(result.current.settings.storagePath).toBe("/home/op/Library2");
    expect(loadManagedSkills).toHaveBeenCalledTimes(1);
    expect(replaceCatalog).not.toHaveBeenCalled();
    const { toast } = await import("sonner");
    expect(vi.mocked(toast.error)).toHaveBeenCalled();
  });

  // S2: a failed project re-link is `error` in the database; the project
  // world re-reads its views whether the move resolved or threw.
  it("the project world re-reads its views after a move that resolved", async () => {
    const backend = stubMoveBackend();
    const { result, reloadViews } = renderMove(async () => "confirmed");
    await waitFor(() => expect(result.current.settings.storagePath).toBe("/home/op/.skillshub"));
    pickFolder.mockResolvedValueOnce(["/home/op/Library2"]);
    let done: Promise<void> = Promise.resolve();
    act(() => {
      done = result.current.settings.handlePickStoragePath();
    });
    await waitFor(() => expect(commandsCalled()).toContain("moveLibrary"));
    expect(reloadViews).not.toHaveBeenCalled();

    await act(async () => {
      backend.finish();
      await done;
    });

    expect(reloadViews).toHaveBeenCalledTimes(1);
  });

  it("the project world re-reads its views after a move that threw", async () => {
    stubBackend();
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "previewLibraryMove") return Promise.resolve(PREVIEW);
      if (command === "moveLibrary") return Promise.reject({ code: "OTHER", message: "boom" });
      return base(command, ...args);
    });
    const { result, reloadViews } = renderMove(async () => "confirmed");

    await moveToLibrary2(result);

    expect(reloadViews).toHaveBeenCalledTimes(1);
  });
});

// S3: a move a crash interrupted is finished by the startup resume; its
// report used to reach only the log.
describe("useSettingsState a resumed Library move", () => {
  const clean: LibraryMoveReport = {
    from: "/home/op/.skillshub",
    to: "/home/op/Library2",
    skills: [
      { skill_id: "a", name: "a", status: { status: "moved", from: "/home/op/.skillshub/a", to: "/home/op/Library2/a" } },
    ],
    deployments: [
      {
        skill_id: "a",
        name: "a",
        path: "/home/op/.claude/skills/a",
        rows: [{ scope: "global_target", id: "a-row", skill_id: "a", tool: "claude_code", folder_key: "claude_code" }],
        status: { status: "relinked", mode: "symlink" },
      },
    ],
    left_behind: [{ path: "/home/op/.skillshub", reason: "old_library", skill_id: null, error: null }],
  };
  const changed: LibraryMoveReport = {
    ...clean,
    left_behind: [
      ...clean.left_behind,
      { path: "/home/op/.skillshub/a", reason: "changed_since_copy", skill_id: "a", error: null },
    ],
  };

  /** The backend holds one resumed report and hands it out once. */
  function stubResumed(report: LibraryMoveReport) {
    stubBackend();
    const base = mockInvoke.getMockImplementation()!;
    let waiting: LibraryMoveReport | null = report;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "takeResumedLibraryMove") {
        const taken = waiting;
        waiting = null;
        return Promise.resolve(taken);
      }
      return base(command, ...args);
    });
  }

  it("is taken once at startup and recorded as one run, with no toast when nothing needs attention", async () => {
    stubResumed(clean);
    const { result } = renderMove(async () => "dismissed");

    await waitFor(() => expect(result.current.reporter.notifications.length).toBeGreaterThan(0));

    expect(commandsCalled().filter((command) => command === "takeResumedLibraryMove")).toHaveLength(1);
    const runs = new Set(result.current.reporter.notifications.map((entry) => entry.runId));
    expect(runs.size).toBe(1);
    expect(result.current.reporter.notifications[0]).toMatchObject({
      runLabel: "libraryMove.resumedLabel",
      title: "libraryMove.report.resumed",
    });
    const { toast } = await import("sonner");
    for (const show of [toast.success, toast.info, toast.warning, toast.error]) {
      expect(vi.mocked(show)).not.toHaveBeenCalled();
    }
  });

  it("toasts when the resumed move left a warning", async () => {
    stubResumed(changed);
    const { result } = renderMove(async () => "dismissed");

    await waitFor(() => expect(result.current.reporter.notifications.length).toBeGreaterThan(0));

    const { toast } = await import("sonner");
    expect(vi.mocked(toast.warning)).toHaveBeenCalledTimes(1);
    expect(
      result.current.reporter.notifications.some(
        (entry) => entry.title === "libraryMove.report.changedSinceCopyTitle",
      ),
    ).toBe(true);
  });

  it("a move that first finished an interrupted one hands its report out after it settles", async () => {
    const backend = stubMoveBackend();
    const base = mockInvoke.getMockImplementation()!;
    let waiting: LibraryMoveReport | null = null;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "takeResumedLibraryMove") {
        const taken = waiting;
        waiting = null;
        return Promise.resolve(taken);
      }
      return base(command, ...args);
    });
    const { result } = renderMove(async () => "confirmed");
    await waitFor(() => expect(commandsCalled()).toContain("takeResumedLibraryMove"));
    await waitFor(() => expect(result.current.settings.storagePath).toBe("/home/op/.skillshub"));
    pickFolder.mockResolvedValueOnce(["/home/op/Library2"]);
    let done: Promise<void> = Promise.resolve();
    act(() => {
      done = result.current.settings.handlePickStoragePath();
    });
    await waitFor(() => expect(commandsCalled()).toContain("moveLibrary"));
    waiting = changed;

    await act(async () => {
      backend.finish();
      await done;
    });

    expect(commandsCalled().filter((command) => command === "takeResumedLibraryMove")).toHaveLength(2);
    expect(
      result.current.reporter.notifications.some((entry) => entry.runLabel === "libraryMove.resumedLabel"),
    ).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Clean Now (round 20 #04): under the real lifecycle, in English.
// ---------------------------------------------------------------------------

async function english(): Promise<TranslateFn> {
  const i18n = i18next.createInstance();
  await i18n.init({ resources, lng: "en", interpolation: { escapeValue: false } });
  return i18n.t.bind(i18n) as unknown as TranslateFn;
}

/** The backend for Clean Now: `clearGitCacheNow` waits until `finish()`. */
function stubCleanBackend(answer: { cleared: number; in_use: number }) {
  stubBackend();
  let finish: () => void = () => {};
  const base = mockInvoke.getMockImplementation()!;
  mockInvoke.mockImplementation((command, ...args) =>
    command === "clearGitCacheNow"
      ? new Promise((resolve) => (finish = () => resolve(answer)))
      : base(command, ...args),
  );
  return { finish: () => finish() };
}

function renderClean(tr: TranslateFn) {
  return renderHook(() => {
    const reporter = useStatusReporter(tr);
    const settings = useSettingsState({
      t: tr,
      reporter,
      surface: noSurface(),
      library: { replaceCatalog: vi.fn(), loadManagedSkills: vi.fn(async () => {}) },
      projects: { reloadViews: vi.fn(async () => {}) },
    });
    return { reporter, settings };
  });
}

describe("useSettingsState Clean Now", () => {
  it("runs under the lifecycle and toasts what it cleared and what was in use", async () => {
    const en = await english();
    const backend = stubCleanBackend({ cleared: 3, in_use: 1 });
    const { result } = renderClean(en);
    await waitFor(() => expect(commandsCalled()).toContain("getSettings"));

    let done: Promise<void> = Promise.resolve();
    act(() => {
      done = result.current.settings.handleClearGitCacheNow();
    });
    await waitFor(() => expect(commandsCalled()).toContain("clearGitCacheNow"));
    expect(result.current.reporter.pending).toMatchObject({
      label: "Clean the git cache",
      activity: "Cleaning the git cache",
      cancellable: false,
    });

    await act(async () => {
      backend.finish();
      await done;
    });
    const { toast } = await import("sonner");
    expect(vi.mocked(toast.warning)).toHaveBeenCalledWith(
      "Cleared 3 cached repos; 1 in use, skipped",
      expect.anything(),
    );
    expect(result.current.reporter.pending).toBeNull();
  });

  it("omits the in-use clause when nothing was in use", async () => {
    const en = await english();
    const backend = stubCleanBackend({ cleared: 1, in_use: 0 });
    const { result } = renderClean(en);
    await waitFor(() => expect(commandsCalled()).toContain("getSettings"));

    await act(async () => {
      const done = result.current.settings.handleClearGitCacheNow();
      backend.finish();
      await done;
    });
    const { toast } = await import("sonner");
    expect(vi.mocked(toast.success)).toHaveBeenCalledWith("Cleared 1 cached repo", expect.anything());
  });

  it("is refused with the running action's reason while another action runs", async () => {
    const en = await english();
    stubCleanBackend({ cleared: 0, in_use: 0 });
    const { result } = renderClean(en);
    await waitFor(() => expect(commandsCalled()).toContain("getSettings"));
    let release: () => void = () => {};
    let other: Promise<unknown> = Promise.resolve();
    act(() => {
      other = result.current.reporter.runAction(
        { label: "Update all" },
        () => new Promise<void>((resolve) => (release = resolve)),
      );
    });
    // The reason App hands the held Clean Now control (`blockedReason`: its
    // title, focusable while disabled) names what runs.
    const reason = describeRefusal(result.current.reporter.refusal!, en);
    expect(reason).toContain("Update all");

    await act(async () => {
      await result.current.settings.handleClearGitCacheNow();
    });

    expect(commandsCalled()).not.toContain("clearGitCacheNow");
    const { toast } = await import("sonner");
    expect(vi.mocked(toast.info)).toHaveBeenCalledWith(
      reason,
      expect.objectContaining({ id: "lifecycle-refusal" }),
    );
    await act(async () => {
      release();
      await other;
    });
  });
});
