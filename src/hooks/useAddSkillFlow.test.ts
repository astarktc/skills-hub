// Tests at the AddSkillFlow seam: git/local candidate discovery routing
// (single-candidate fast path vs pick modal), name-collision guards, the
// Explore-page auto-select (the backend resolves the name; this side only
// decides install / pick / not-found), and the deploy-target intersection
// (user-selected ∩ installed). Backend mocked at the invokeTauri module
// seam; sync and library worlds enter as mocked dependency interfaces.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  CandidateMatch,
  GitSkillCandidate,
  GitSkillListing,
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
import {
  ActionExit,
  type ActionHandle,
  type RunActionOptions,
  type StatusReporter,
} from "./useStatusReporter";

const mockInvoke = vi.mocked(invokeTauri);

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

const EMPTY_PLAN = { total_tools_scanned: 0, total_skills_found: 0, groups: [] };

function gitCandidate(name: string, subpath: string): GitSkillCandidate {
  return { name, description: null, subpath };
}

function stubBackend(overrides?: {
  gitCandidates?: GitSkillCandidate[];
  /** What the backend resolved a `targetName` to; `none` by default. */
  targetMatch?: CandidateMatch;
  localCandidates?: LocalSkillCandidate[];
}) {
  mockInvoke.mockImplementation((command: string, args?: unknown) => {
    switch (command) {
      case "get_onboarding_plan":
        return Promise.resolve(EMPTY_PLAN);
      case "list_git_skills_cmd": {
        const { targetName } = args as { targetName: string | null };
        const listing: GitSkillListing = {
          candidates: overrides?.gitCandidates ?? [],
          target_match: targetName
            ? (overrides?.targetMatch ?? { kind: "none" })
            : null,
        };
        return Promise.resolve(listing);
      }
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
  const formatError = vi.fn((err: unknown) =>
    err instanceof Error ? err.message : `formatted:${String(err)}`,
  );
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
      await result.current.handleCreate();
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
      await result.current.handleCreate();
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
      await result.current.handleCreate();
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
      await result.current.handleCreate();
    });

    expect(result.current.git.visible).toBe(true);
    expect(result.current.git.selected).toEqual({
      "skills/alpha": true,
      "skills/beta": true,
    });
    expect(installGitCalls()).toHaveLength(0);
    // A hand-off to the picker is neither a success nor a failure.
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
    expect(setup.reporter.setError).not.toHaveBeenCalled();
  });
});

describe("useAddSkillFlow explore auto-select", () => {
  it("hands the target name to the listing and installs what the backend resolved", async () => {
    // skills.sh name vs SKILL.md name ("json-render-react" vs "react"): the
    // matching rule lives in core; the flow only follows the resolution.
    stubBackend({
      gitCandidates: [
        gitCandidate("react", "skills/react"),
        gitCandidate("vue", "skills/vue"),
      ],
      targetMatch: { kind: "resolved", subpath: "skills/react" },
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
    expect(mockInvoke).toHaveBeenCalledWith("list_git_skills_cmd", {
      repoUrl: "https://github.com/x/y",
      targetName: "json-render-react",
    });
    // The explore path resets deploy targets to all installed tools first.
    expect(setup.sync.targetAllInstalled).toHaveBeenCalled();
  });

  it("a manual git install lists without a target name", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => result.current.setGitUrl("https://github.com/x/y"));
    await act(async () => {
      await result.current.handleCreate();
    });

    expect(mockInvoke).toHaveBeenCalledWith("list_git_skills_cmd", {
      repoUrl: "https://github.com/x/y",
      targetName: null,
    });
  });

  it("falls back to the pick modal when the backend resolves nothing", async () => {
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

    await waitFor(() => expect(result.current.git.visible).toBe(true));
    expect(installGitCalls()).toHaveLength(0);
  });

  it("falls back to the pick modal when the backend reports ambiguity", async () => {
    stubBackend({
      gitCandidates: [
        gitCandidate("react", "skills/react"),
        gitCandidate("render", "skills/render"),
      ],
      targetMatch: {
        kind: "ambiguous",
        subpaths: ["skills/react", "skills/render"],
      },
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => {
      result.current.handleExploreInstall(
        "https://github.com/x/y",
        "json-render-react",
      );
    });

    await waitFor(() => expect(result.current.git.visible).toBe(true));
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

    act(() => {
      result.current.setAddModalTab("local");
      result.current.setLocalPath("/some/dir");
    });
    await act(async () => {
      await result.current.handleCreate();
    });

    expect(result.current.local.visible).toBe(true);
    expect(result.current.local.selected).toEqual({
      alpha: true,
      broken: false,
    });
    expect(
      mockInvoke.mock.calls.some(([cmd]) => cmd === "install_local_selection"),
    ).toBe(false);
    // A hand-off to the picker is neither a success nor a failure.
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
    expect(setup.reporter.setError).not.toHaveBeenCalled();
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

    act(() => {
      result.current.setAddModalTab("local");
      result.current.setLocalPath("/some/dir");
    });
    await act(async () => {
      await result.current.handleCreate();
    });

    expect(setup.reporter.setError).toHaveBeenCalledWith(
      'errors.skillAlreadyExists {"name":"alpha"}',
    );
    expect(
      mockInvoke.mock.calls.some(([cmd]) => cmd === "install_local_selection"),
    ).toBe(false);
  });
});
