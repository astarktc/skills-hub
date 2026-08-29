// Tests at the AddSkillFlow seam: git/local candidate discovery routing
// (single-candidate fast path vs pick modal), name-collision guards, the
// Explore-page auto-select matching, and the deploy-target intersection
// (user-selected ∩ installed). Backend mocked at the invokeTauri module
// seam; sync and library worlds enter as mocked dependency interfaces.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  GitSkillCandidate,
  LocalSkillCandidate,
} from "../components/skills/types";
import type { AddSkillFlowDeps } from "./useAddSkillFlow";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));

import { invokeTauri } from "../lib/tauri";
import { useAddSkillFlow } from "./useAddSkillFlow";

const mockInvoke = vi.mocked(invokeTauri);

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

const EMPTY_PLAN = { total_tools_scanned: 0, total_skills_found: 0, groups: [] };

function gitCandidate(name: string, subpath: string): GitSkillCandidate {
  return { name, description: null, subpath };
}

function stubBackend(overrides?: {
  gitCandidates?: GitSkillCandidate[];
  localCandidates?: LocalSkillCandidate[];
}) {
  mockInvoke.mockImplementation((command: string) => {
    switch (command) {
      case "get_onboarding_plan":
        return Promise.resolve(EMPTY_PLAN);
      case "list_git_skills_cmd":
        return Promise.resolve(overrides?.gitCandidates ?? []);
      case "list_local_skills_cmd":
        return Promise.resolve(overrides?.localCandidates ?? []);
      case "install_git_selection":
      case "install_local_selection":
        return Promise.resolve({
          skill_id: "new-id",
          name: "installed-skill",
          central_path: "/hub/installed-skill",
          content_hash: null,
        });
      default:
        return Promise.resolve(undefined);
    }
  });
}

function makeDeps(overrides?: { takenNames?: string[] }) {
  const taken = new Set(
    (overrides?.takenNames ?? []).map((n) => n.toLowerCase()),
  );
  const reporter = {
    loading: false,
    loadingStartAt: null,
    actionMessage: null,
    setLoading: vi.fn(),
    setLoadingStartAt: vi.fn(),
    setActionMessage: vi.fn(),
    setError: vi.fn(),
    setSuccessToastMessage: vi.fn(),
    formatError: vi.fn((err: unknown) =>
      err instanceof Error ? err.message : `formatted:${String(err)}`,
    ),
    showActionErrors: vi.fn(),
    cancelLoading: vi.fn(),
  };
  const sync = {
    autoSyncEnabled: true,
    // cursor is deselected, goose is selected but not installed → the
    // deploy set must intersect down to just claude.
    isInstalled: (id: string) => id === "claude" || id === "cursor",
    syncFailureEntries: vi.fn(() => []),
    syncSkillsToTools: vi.fn().mockResolvedValue({
      results: [],
      synced: 0,
      skipped: 0,
      failed: 0,
    }),
    syncTargets: { claude: true, cursor: false, goose: true },
    targetAllInstalled: vi.fn(),
    toolLabelById: { claude: "CLAUDE", cursor: "CURSOR" },
    tools: [
      { id: "claude", label: "CLAUDE" },
      { id: "cursor", label: "CURSOR" },
      { id: "goose", label: "GOOSE" },
    ],
  };
  const library = {
    isSkillNameTaken: vi.fn((name: string) => taken.has(name.toLowerCase())),
    loadManagedSkills: vi.fn().mockResolvedValue(undefined),
  };
  const deps: AddSkillFlowDeps = { t, reporter, sync, library };
  return { deps, reporter, sync, library };
}

function installGitCalls() {
  return mockInvoke.mock.calls.filter(
    ([cmd]) => cmd === "install_git_selection",
  );
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("useAddSkillFlow git flow", () => {
  it("rejects an empty git URL without touching the backend", async () => {
    stubBackend();
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    await act(async () => {
      await result.current.handleCreateGit();
    });

    expect(setup.reporter.setError).toHaveBeenCalledWith(
      "errors.requireGitUrl",
    );
    expect(
      mockInvoke.mock.calls.some(([cmd]) => cmd === "list_git_skills_cmd"),
    ).toBe(false);
  });

  it("a single candidate whose name is taken blocks the install", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps({ takenNames: ["Alpha"] });
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => result.current.setGitUrl("https://github.com/x/y"));
    await act(async () => {
      await result.current.handleCreateGit();
    });

    expect(setup.reporter.setError).toHaveBeenCalledWith(
      'errors.skillAlreadyExists {"name":"alpha"}',
    );
    expect(installGitCalls()).toHaveLength(0);
  });

  it("a single free candidate installs and syncs to selected∩installed targets", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => result.current.setGitUrl("https://github.com/x/y"));
    await act(async () => {
      await result.current.handleCreateGit();
    });

    expect(mockInvoke).toHaveBeenCalledWith("install_git_selection", {
      repoUrl: "https://github.com/x/y",
      subpath: "skills/alpha",
      name: undefined,
    });
    // goose is selected but not installed; cursor installed but deselected.
    expect(setup.sync.syncSkillsToTools).toHaveBeenCalledWith(
      [
        {
          skill_id: "new-id",
          name: "installed-skill",
          source_path: "/hub/installed-skill",
        },
      ],
      ["claude"],
      { overwriteIfSameContent: true },
    );
    expect(setup.reporter.setSuccessToastMessage).toHaveBeenCalledWith(
      "status.gitSkillCreated",
    );
  });

  it("multiple candidates open the pick modal with everything preselected", async () => {
    stubBackend({
      gitCandidates: [
        gitCandidate("alpha", "skills/alpha"),
        gitCandidate("beta", "skills/beta"),
      ],
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => result.current.setGitUrl("https://github.com/x/y"));
    await act(async () => {
      await result.current.handleCreateGit();
    });

    expect(result.current.showGitPickModal).toBe(true);
    expect(result.current.gitCandidateSelected).toEqual({
      "skills/alpha": true,
      "skills/beta": true,
    });
    expect(installGitCalls()).toHaveLength(0);
  });
});

describe("useAddSkillFlow explore auto-select", () => {
  it("installs the exact-name match from a multi-candidate repo", async () => {
    stubBackend({
      gitCandidates: [
        gitCandidate("alpha", "skills/alpha"),
        gitCandidate("beta", "skills/beta"),
      ],
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => {
      result.current.handleExploreInstall("https://github.com/x/y", "beta");
    });

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("install_git_selection", {
        repoUrl: "https://github.com/x/y",
        subpath: "skills/beta",
        name: undefined,
      }),
    );
    // The explore path resets deploy targets to all installed tools first.
    expect(setup.sync.targetAllInstalled).toHaveBeenCalled();
  });

  it("installs a unique containment match (skills.sh name vs SKILL.md name)", async () => {
    stubBackend({
      gitCandidates: [
        gitCandidate("react", "skills/react"),
        gitCandidate("vue", "skills/vue"),
      ],
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => {
      result.current.handleExploreInstall(
        "https://github.com/x/y",
        "json-render-react",
      );
    });

    await waitFor(() =>
      expect(mockInvoke).toHaveBeenCalledWith("install_git_selection", {
        repoUrl: "https://github.com/x/y",
        subpath: "skills/react",
        name: undefined,
      }),
    );
  });

  it("falls back to the pick modal when nothing matches", async () => {
    stubBackend({
      gitCandidates: [
        gitCandidate("alpha", "skills/alpha"),
        gitCandidate("beta", "skills/beta"),
      ],
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => {
      result.current.handleExploreInstall("https://github.com/x/y", "gamma");
    });

    await waitFor(() => expect(result.current.showGitPickModal).toBe(true));
    expect(installGitCalls()).toHaveLength(0);
  });

  it("errors instead of silently installing a mismatched single candidate", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => {
      result.current.handleExploreInstall("https://github.com/x/y", "gamma");
    });

    await waitFor(() =>
      expect(setup.reporter.setError).toHaveBeenCalledWith(
        'errors.skillNotFoundInRepo {"name":"gamma"}',
      ),
    );
    expect(installGitCalls()).toHaveLength(0);
  });
});

describe("useAddSkillFlow local flow", () => {
  it("multiple local candidates open the pick modal preselecting only valid ones", async () => {
    stubBackend({
      localCandidates: [
        {
          name: "alpha",
          description: null,
          subpath: "alpha",
          valid: true,
          reason: null,
        },
        {
          name: "broken",
          description: null,
          subpath: "broken",
          valid: false,
          reason: "missing_skill_md",
        },
      ],
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => result.current.setLocalPath("/some/dir"));
    await act(async () => {
      await result.current.handleCreateLocal();
    });

    expect(result.current.showLocalPickModal).toBe(true);
    expect(result.current.localCandidateSelected).toEqual({
      alpha: true,
      broken: false,
    });
    expect(
      mockInvoke.mock.calls.some(([cmd]) => cmd === "install_local_selection"),
    ).toBe(false);
  });

  it("a single valid candidate with a taken name blocks the install", async () => {
    stubBackend({
      localCandidates: [
        {
          name: "alpha",
          description: null,
          subpath: "alpha",
          valid: true,
          reason: null,
        },
      ],
    });
    const setup = makeDeps({ takenNames: ["alpha"] });
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => result.current.setLocalPath("/some/dir"));
    await act(async () => {
      await result.current.handleCreateLocal();
    });

    expect(setup.reporter.setError).toHaveBeenCalledWith(
      'errors.skillAlreadyExists {"name":"alpha"}',
    );
    expect(
      mockInvoke.mock.calls.some(([cmd]) => cmd === "install_local_selection"),
    ).toBe(false);
  });
});
