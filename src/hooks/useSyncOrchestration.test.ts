// Tests at the SyncOrchestration seam: target defaulting from the saved
// global selection, shared-dir group expansion, the batch sync command's
// policy/channel wiring, and per-target failure surfacing. The backend is
// mocked at the invokeTauri module seam with per-command responses; the
// Tauri Channel is a minimal fake capturing onmessage.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  BatchSyncReportDto,
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

import { invokeTauri } from "../lib/tauri";
import { useSyncOrchestration } from "./useSyncOrchestration";

const mockInvoke = vi.mocked(invokeTauri);

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
    skills_dir: `~/.${key}/skills`,
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
    Pick<AppSettings, "global_selected_tools" | "scan_selected_tools_only">
  >;
  status?: Partial<ToolStatusDto>;
  syncReport?: BatchSyncReportDto;
}) {
  mockInvoke.mockImplementation((command: string) => {
    switch (command) {
      case "get_settings":
        return Promise.resolve(appSettings(overrides?.config));
      case "get_tool_status":
        return Promise.resolve({ ...TOOL_STATUS, ...overrides?.status });
      case "sync_skills_to_tools":
        return Promise.resolve(
          overrides?.syncReport ?? {
            results: [],
            synced: 0,
            skipped: 0,
            failed: 0,
          },
        );
      default:
        return Promise.resolve(undefined);
    }
  });
}

function makeReporter(): Pick<
  StatusReporter,
  "loading" | "setActionMessage" | "setError" | "formatError"
> {
  return {
    loading: false,
    setActionMessage: vi.fn(),
    setError: vi.fn(),
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

describe("useSyncOrchestration shared-dir groups", () => {
  it("toggling a shared-dir tool toggles its whole group after confirm", async () => {
    stubBackend();
    const confirmSpy = vi
      .spyOn(window, "confirm")
      .mockImplementation(() => true);
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    act(() => {
      result.current.handleSyncTargetChange("claude", false);
    });

    expect(confirmSpy).toHaveBeenCalledTimes(1);
    expect(result.current.syncTargets.claude).toBe(false);
    expect(result.current.syncTargets.pi).toBe(false);
    expect(result.current.syncTargets.cursor).toBe(true);
  });

  it("a declined confirm leaves the targets untouched", async () => {
    stubBackend();
    vi.spyOn(window, "confirm").mockImplementation(() => false);
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    act(() => {
      result.current.handleSyncTargetChange("pi", false);
    });

    expect(result.current.syncTargets.claude).toBe(true);
    expect(result.current.syncTargets.pi).toBe(true);
  });

  it("a standalone tool toggles without confirmation", async () => {
    stubBackend();
    const confirmSpy = vi.spyOn(window, "confirm");
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    act(() => {
      result.current.handleSyncTargetChange("cursor", false);
    });

    expect(confirmSpy).not.toHaveBeenCalled();
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
      ([cmd]) => cmd === "sync_skills_to_tools",
    );
    expect(call).toBeDefined();
    const args = call![1] as {
      skills: unknown;
      tools: unknown;
      policy: unknown;
      onProgress: FakeChannel<SyncProgressDto>;
    };
    expect(args.skills).toBe(skills);
    expect(args.tools).toEqual(["claude"]);
    expect(args.policy).toEqual({
      overwrite: false,
      overwrite_if_same_content: false,
      overrides: [],
    });
    expect(args.onProgress).toBeInstanceOf(FakeChannel);
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
      ([cmd]) => cmd === "sync_skills_to_tools",
    );
    const channel = (call![1] as { onProgress: FakeChannel<SyncProgressDto> })
      .onProgress;

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

describe("syncFailureEntries", () => {
  const report: BatchSyncReportDto = {
    results: [
      {
        skill_id: "s1",
        skill_name: "Skill One",
        tool: "claude",
        status: { status: "synced", mode_used: "symlink" },
      },
      {
        skill_id: "s1",
        skill_name: "Skill One",
        tool: "cursor",
        status: {
          status: "failed",
          error: { code: "OTHER", message: "boom" },
        },
      },
      {
        skill_id: "s1",
        skill_name: "Skill One",
        tool: "goose",
        status: {
          status: "skipped",
          error: { code: "TOOL_NOT_INSTALLED", tool: "goose" },
        },
      },
      {
        skill_id: "s1",
        skill_name: "Skill One",
        tool: "pi",
        status: {
          status: "skipped",
          error: {
            code: "TOOL_NOT_WRITABLE",
            tool: "pi",
            path: "/x",
          },
        },
      },
    ],
    synced: 1,
    skipped: 2,
    failed: 1,
  };

  it("surfaces failures and silences skips by default", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    const entries = result.current.syncFailureEntries(report);
    expect(entries).toEqual([
      {
        title: 'errors.syncFailedTitle {"name":"Skill One","tool":"CURSOR"}',
        message: "formatted:OTHER",
      },
    ]);
  });

  it("optionally surfaces not-writable skips, never absent-tool skips", async () => {
    stubBackend();
    const { result } = renderSync();
    await waitFor(() => expect(result.current.toolStatus).not.toBeNull());

    const entries = result.current.syncFailureEntries(report, {
      includeNotWritableSkips: true,
    });
    expect(entries.map((e) => e.message)).toEqual([
      "formatted:OTHER",
      "formatted:TOOL_NOT_WRITABLE",
    ]);
  });
});
