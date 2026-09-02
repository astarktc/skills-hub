// Tests at the SettingsState seam: one settings snapshot load on mount
// (`get_settings`), writes through `update_setting` adopting the effective
// values the backend returns, and numeric clamps driven by the DTO bounds
// rather than literals. The backend is mocked at the invokeTauri module seam.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AppSettings, SettingUpdate } from "../components/skills/types";
import type { StatusReporter } from "./useStatusReporter";

vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
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
    github_token: "ghp_stored",
    auto_sync_enabled: true,
    global_selected_tools: null,
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
          case "central_repo_path":
            current = { ...current, central_repo_path: update.value };
            break;
          case "git_cache_cleanup_days":
            current = { ...current, git_cache_cleanup_days: update.value };
            break;
          case "git_cache_ttl_secs":
            current = { ...current, git_cache_ttl_secs: update.value };
            break;
          case "github_token":
            current = { ...current, github_token: update.value.trim() };
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

function makeReporter(): Pick<
  StatusReporter,
  "setError" | "setSuccessToastMessage" | "formatError"
> {
  return {
    setError: vi.fn(),
    setSuccessToastMessage: vi.fn(),
    formatError: vi.fn((err: unknown) => `formatted:${String(err)}`),
  };
}

function renderSettings(reporter = makeReporter()) {
  return renderHook(() =>
    useSettingsState({
      t,
      reporter,
      onManagedSkillsChanged: vi.fn(async () => {}),
    }),
  );
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
    expect(result.current.githubToken).toBe("ghp_stored");
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
      expect(reporter.setError).toHaveBeenCalledWith("formatted:Error: boom");
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

    // Backend normalises further (e.g. trims the token).
    await act(async () => {
      await result.current.handleGithubTokenChange("  ghp_new  ");
    });

    expect(updateCalls()).toEqual([
      { key: "github_token", value: "  ghp_new  " },
    ]);
    expect(result.current.githubToken).toBe("ghp_new");
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

    expect(reporter.setError).toHaveBeenCalledWith("formatted:Error: disk");
    expect(result.current.storagePath).toBe("/home/op/.skillshub");
  });
});

describe("useSettingsState single-field adoption", () => {
  it("does not let a slow write's snapshot clobber a newer field edit", async () => {
    // Two overlapping writes: the TTL write resolves *after* a later token
    // edit. Its response snapshot still carries the old token, so adopting the
    // whole snapshot would roll the token back.
    let releaseTtlWrite: ((value: AppSettings) => void) | undefined;
    const initial = appSettings({ github_token: "old-token" });
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
              appSettings({ github_token: update.value, git_cache_ttl_secs: 5 }),
            );
          }
          return Promise.resolve(initial);
        }
        default:
          return Promise.resolve(undefined);
      }
    });

    const { result } = renderSettings();
    await waitFor(() => expect(result.current.githubToken).toBe("old-token"));

    // Start the slow TTL write, then complete a token write behind its back.
    let ttlWrite: Promise<void> | undefined;
    await act(async () => {
      ttlWrite = result.current.handleGitCacheTtlSecsChange(5);
    });
    await act(async () => {
      await result.current.handleGithubTokenChange("new-token");
    });
    expect(result.current.githubToken).toBe("new-token");

    // The TTL response (which still says github_token: "old-token") lands last.
    await act(async () => {
      releaseTtlWrite?.(appSettings({ github_token: "old-token", git_cache_ttl_secs: 5 }));
      await ttlWrite;
    });

    expect(result.current.gitCacheTtlSecs).toBe(5);
    expect(result.current.githubToken).toBe("new-token");
  });
});
