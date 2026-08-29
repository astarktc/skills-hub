// Tests at the SkillLibrary seam: the managed-skill list actions. The sync
// world and the reporter enter as dependency interfaces (mocked objects),
// the backend at the invokeTauri module seam — exactly the shape App.tsx
// wires, so these tests exercise the hook the way the app does.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  BatchSyncReportDto,
  ManagedSkill,
} from "../components/skills/types";
import type { SkillLibraryDeps } from "./useSkillLibrary";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

import { invokeTauri } from "../lib/tauri";
import { useSkillLibrary } from "./useSkillLibrary";

const mockInvoke = vi.mocked(invokeTauri);

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

function makeDeps(overrides?: {
  skills?: ManagedSkill[];
  autoSyncEnabled?: boolean;
  installedToolIds?: string[];
  sharedToolIdsByToolId?: Record<string, string[]>;
  syncReport?: BatchSyncReportDto;
}) {
  const skills = overrides?.skills ?? [skill("s1", "alpha")];
  mockInvoke.mockImplementation((command: string) => {
    switch (command) {
      case "get_managed_skills":
        return Promise.resolve(skills);
      default:
        return Promise.resolve(undefined);
    }
  });

  const reporter = {
    loading: false,
    loadingStartAt: null,
    actionMessage: null,
    setLoading: vi.fn(),
    setLoadingStartAt: vi.fn(),
    setActionMessage: vi.fn(),
    setError: vi.fn(),
    setSuccessToastMessage: vi.fn(),
    formatError: vi.fn((err: unknown) => {
      const code = (err as { code?: string })?.code;
      return code === "CANCELLED" ? null : `formatted:${code ?? String(err)}`;
    }),
    showActionErrors: vi.fn(),
    cancelLoading: vi.fn(),
  };

  const sync = {
    autoSyncEnabled: overrides?.autoSyncEnabled ?? true,
    installedToolIds: overrides?.installedToolIds ?? ["claude", "cursor"],
    sharedToolIdsByToolId: overrides?.sharedToolIdsByToolId ?? {},
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
  it("collects per-skill update failures and shows them once at the end", async () => {
    const setup = makeDeps({
      skills: [skill("s1", "alpha"), skill("s2", "beta")],
      autoSyncEnabled: false,
    });
    const { result } = await renderLibrary(setup);
    // beta's update fails; alpha's succeeds.
    mockInvoke.mockImplementation((command: string, args?) => {
      if (command === "get_managed_skills") return Promise.resolve(setup.skills);
      if (
        command === "update_managed_skill" &&
        (args as { skillId: string }).skillId === "s2"
      ) {
        return Promise.reject({ code: "GIT_CLONE_FAILED" });
      }
      return Promise.resolve(undefined);
    });

    await act(async () => {
      await result.current.handleRefresh();
    });

    expect(setup.reporter.showActionErrors).toHaveBeenCalledTimes(1);
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      {
        title: 'errors.updateFailedTitle {"name":"beta"}',
        message: "formatted:GIT_CLONE_FAILED",
      },
    ]);
    // Loading surface opened and closed around the whole pass.
    expect(setup.reporter.setLoading).toHaveBeenNthCalledWith(1, true);
    expect(setup.reporter.setLoading).toHaveBeenLastCalledWith(false);
  });

  it("with auto-sync on, pushes updated content with overwrite (the refresh contract)", async () => {
    const setup = makeDeps();
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRefresh();
    });

    expect(setup.sync.syncSkillsToTools).toHaveBeenCalledWith(
      [{ skill_id: "s1", name: "alpha", source_path: "/hub/alpha" }],
      ["claude", "cursor"],
      { overwrite: true },
    );
  });

  it("with auto-sync off, refresh never fans out a sync", async () => {
    const setup = makeDeps({ autoSyncEnabled: false });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRefresh();
    });

    expect(setup.sync.syncSkillsToTools).not.toHaveBeenCalled();
  });

  it("appends sync failures to the collected refresh errors", async () => {
    const setup = makeDeps({
      syncReport: {
        results: [
          {
            skill_id: "s1",
            skill_name: "alpha",
            tool: "cursor",
            status: {
              status: "failed",
              error: { code: "OTHER", message: "boom" },
            },
          },
        ],
        synced: 0,
        skipped: 0,
        failed: 1,
      },
    });
    const { result } = await renderLibrary(setup);

    await act(async () => {
      await result.current.handleRefresh();
    });

    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      { title: "alpha", message: "sync failed" },
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
      expect(mockInvoke).toHaveBeenCalledWith("unsync_skill_from_tool", {
        skillId: "s1",
        tool: "claude",
      }),
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

  it("a shared-dir tool defers to the confirmation modal, then runs on confirm", async () => {
    const setup = makeDeps({
      sharedToolIdsByToolId: { claude: ["claude", "pi"], pi: ["claude", "pi"] },
    });
    const { result } = await renderLibrary(setup);

    act(() => {
      result.current.handleToggleToolForSkill(setup.skills[0], "claude");
    });

    // No sync yet — the modal owns the decision.
    expect(setup.sync.syncSkillsToTools).not.toHaveBeenCalled();
    expect(result.current.pendingSharedToggle).toEqual({
      skill: setup.skills[0],
      toolId: "claude",
    });
    expect(result.current.pendingSharedLabels).toEqual({
      toolLabel: "CLAUDE",
      otherLabels: "PI",
    });

    await act(async () => {
      result.current.handleSharedConfirm();
    });

    await waitFor(() =>
      expect(setup.sync.syncSkillsToTools).toHaveBeenCalledWith(
        [{ skill_id: "s1", name: "alpha", source_path: "/hub/alpha" }],
        ["claude"],
        { overwriteIfSameContent: true },
      ),
    );
    expect(result.current.pendingSharedToggle).toBeNull();
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
