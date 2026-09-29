// Tests at the AddSkillFlow seam: git/local candidate discovery routing
// (single-candidate fast path vs pick modal), name-collision guards, the
// Explore-page auto-select (the backend resolves the name; this side only
// decides install / pick / not-found), and the deploy-target rule (the
// operator's selection as saved, never intersected with detection — a
// selected-but-undetected tool is the backend's skip to report). Backend
// mocked at the invokeTauri module
// seam; sync and library worlds enter as mocked dependency interfaces.

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  CandidateMatch,
  GitSkillCandidate,
  GitSkillListing,
  GitSourceResolution,
  ImportGroupStatus,
  ImportReport,
  InstallBatchReport,
  InstallRequest,
  LocalSkillCandidate,
  OnboardingPlan,
} from "../components/skills/types";
import * as folds from "../lib/reportOutcome";
import { describeCommandError } from "../commandError";
import type { AddSkillFlowDeps } from "./useAddSkillFlow";
import type { SyncOrchestration } from "./useSyncOrchestration";

vi.mock("sonner", () => ({
  // warning/info: the real reporter (the cancellation cases) uses every kind.
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));
// The import batch streams progress over a channel from the platform seam,
// stubbed with a plain message sink; the folder picker is its other door.
const pickFolder = vi.fn<(options: { title: string; multiple?: boolean }) => Promise<string[]>>();
vi.mock("../lib/platform", () => ({
  createProgressChannel: async (onMessage: (message: unknown) => void) => ({ onmessage: onMessage }),
  pickFolder: (options: { title: string; multiple?: boolean }) => pickFolder(options),
}));

import { invokeTauri, type CommandName, type Commands } from "../lib/tauri";
import { useAddSkillFlow as useAddSkillFlowHook } from "./useAddSkillFlow";
import { useSurfaceState } from "./useSurfaceState";

/** The deps a test supplies; the surface module is composed in, as App does. */
type FlowDeps = Omit<AddSkillFlowDeps, "surface">;
/** The flow as App composes it: its dialogs are surface layers. */
function useAddSkillFlow(deps: FlowDeps) {
  const surface = useSurfaceState();
  return useAddSkillFlowHook({ ...deps, surface });
}
import {
  ActionExit,
  useStatusReporter,
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

const EMPTY_PLAN = {
  total_tools_scanned: 0,
  group_count: 0,
  copy_count: 0,
  groups: [],
  invalid: [],
};

function gitCandidate(name: string, subpath: string): GitSkillCandidate {
  return { name, description: null, subpath, resolution: null };
}

function stubBackend(overrides?: {
  gitCandidates?: GitSkillCandidate[];
  /** What the backend resolved a `targetName` to; `none` by default. */
  targetMatch?: CandidateMatch;
  localCandidates?: LocalSkillCandidate[];
  /** Tools whose target the install batch reports occupied (TARGET_EXISTS). */
  occupied?: string[];
  /** The install batch's answer for every candidate, when not installed. */
  candidateFailure?: unknown;
}) {
  mockInvoke.mockImplementation((command, ...args) => {
    switch (command) {
      case "getOnboardingPlan":
        return Promise.resolve(EMPTY_PLAN);
      case "listGitSkillsCmd": {
        const [, targetName] = args as Parameters<Commands["listGitSkillsCmd"]>;
        const listing: GitSkillListing = {
          candidates: overrides?.gitCandidates ?? [],
          target_match: targetName
            ? (overrides?.targetMatch ?? { kind: "none" })
            : null,
        };
        return Promise.resolve(listing);
      }
      case "listLocalSkillsCmd":
        return Promise.resolve(overrides?.localCandidates ?? []);
      case "installSelection": {
        // One report for the whole batch (ticket 04): every candidate is
        // installed (a lone one as "installed-skill") and first-synced to
        // the request's Tools; an occupied Tool is a collected TARGET_EXISTS.
        const [request] = args as [InstallRequest];
        const report: InstallBatchReport = {
          candidates: request.candidates.map((candidate, i) => {
            const name =
              request.candidates.length === 1 ? "installed-skill" : candidate.subpath.split("/").pop()!;
            if (overrides?.candidateFailure) {
              return { subpath: candidate.subpath, name, status: { status: "failed", error: overrides.candidateFailure as never } };
            }
            const skill_id = request.candidates.length === 1 ? "new-id" : `new-id-${i}`;
            return {
              subpath: candidate.subpath,
              name,
              status: {
                status: "installed",
                skill: { skill_id, name, central_path: `/hub/${name}`, content_hash: null },
                targets: request.deploy
                  ? request.tools.map((tool) => ({
                      skill_id,
                      skill_name: name,
                      tool_key: tool,
                      status: overrides?.occupied?.includes(tool)
                        ? { status: "failed" as const, error: { code: "TARGET_EXISTS" as const, path: `/${tool}/${name}`, modifiedAt: 1, sameContent: false } }
                        : { status: "synced" as const, outcome: { mode_used: "symlink" as const, target_path: `/${tool}/${name}`, replaced: false } },
                    }))
                  : [],
              },
            };
          }),
        };
        return Promise.resolve(report);
      }
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
    async <T>(
      opts: RunActionOptions<T>,
      fn: (action: ActionHandle) => Promise<T | ActionExit>,
    ): Promise<T | undefined> => {
      try {
        const outcome = await fn({
          handOff: () => ActionExit.handOff(),
          fail: (message) => ActionExit.failed(message),
          uncancellable: (run) => run(),
          cancelRequested: () => false,
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
  const showActionErrors = vi.fn();
  const showActionWarnings = vi.fn();
  const notify = vi.fn();
  const reporter: StatusReporter = {
    pending: null,
    loading: false,
    refusal: null,
    actionMessage: null,
    // vi.fn erases the generic; the spy still records calls.
    runAction: runAction as StatusReporter["runAction"],
    setActionMessage: vi.fn(),
    reportProgress: vi.fn(),
    // Executes an Outcome onto the entry spies below in the real executor's
    // order — reload, entries and toast, close (the executor itself is
    // useStatusReporter's own test).
    applyOutcome: vi.fn(async (
      outcome: Parameters<StatusReporter["applyOutcome"]>[0],
      handlers: Parameters<StatusReporter["applyOutcome"]>[1] = {},
    ) => {
      if (outcome.completion.reload) await handlers.reload?.();
      showActionErrors(outcome.errors);
      showActionWarnings(outcome.warnings);
      if (outcome.toast) notify(outcome.toast.kind, outcome.toast.message, outcome.toast.detail);
      if (outcome.completion.closeModal) handlers.closeModal?.();
      return outcome.completion;
    }),
    notify,
    notifications: [],
    unreadCount: 0,
    markAllRead: vi.fn(),
    clearNotifications: vi.fn(),
    setError,
    setSuccessToastMessage,
    formatError,
    notifyError: vi.fn(),
    showActionErrors,
    showActionWarnings,
    copyToClipboard: vi.fn().mockResolvedValue(true),
    cancelPending: vi.fn(),
  };
  const sync = {
    autoSyncEnabled: true,
    scanScopeRevision: 0,
    // cursor is deselected; goose is selected but not installed and stays
    // in the deploy set (round 12 D3: the batch reports it as a skip).
    // The one overwrite ask + retry: answers the report as settled.
    resolveGlobalCollisions: vi.fn<SyncOrchestration["resolveGlobalCollisions"]>(async (report) => report),
    syncTargets: { claude: true, cursor: false, goose: true },
    targetAllInstalled: vi.fn(),
  };
  const catalog = {
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
  const deps: FlowDeps = { t, reporter, sync, catalog, library };
  return { deps, reporter, sync, library };
}

function installCalls() {
  return mockInvoke.mock.calls.filter(([cmd]) => cmd === "installSelection");
}

/** The install batch request a git install sends for `picks`. */
const gitRequest = (url: string, subpaths: string[], resolution: GitSourceResolution | null = null, tools = ["claude", "goose"]) => ({
  source: { kind: "git", url, resolution },
  candidates: subpaths.map((subpath) => ({ subpath, name_override: null })),
  tools,
  overwrite_if_same_content: true,
  overrides: [],
  deploy: true,
});

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
      mockInvoke.mock.calls.some(([cmd]) => cmd === "listGitSkillsCmd"),
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
    expect(installCalls()).toHaveLength(0);
  });

  it("a single free candidate installs and syncs to every selected target, detected or not", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => result.current.setGitUrl("https://github.com/x/y"));
    await act(async () => {
      await result.current.handleCreate();
    });

    // One batch: goose is selected but not installed and is still deployed
    // to (the backend reports the skip); cursor is deselected.
    expect(mockInvoke).toHaveBeenCalledWith(
      "installSelection",
      gitRequest("https://github.com/x/y", ["skills/alpha"]),
      expect.anything(),
    );
    // Nothing collided: no ask.
    expect(setup.sync.resolveGlobalCollisions).not.toHaveBeenCalled();
    expect(setup.reporter.notify).toHaveBeenCalledWith(
      "success", "status.gitSkillCreated", undefined,
    );
  });

  it.each<GitSourceResolution | null>([
    { branch: "feature/x", subpath: "skills" },
    { branch: null, subpath: null },
    null,
  ])("passes the listing's source resolution unchanged to direct install: %j", async (resolution) => {
    stubBackend({ gitCandidates: [{ ...gitCandidate("alpha", "skills/alpha"), resolution }] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    act(() => result.current.setGitUrl("https://github.com/x/y/tree/feature/x/skills"));
    await act(async () => { await result.current.handleCreate(); });
    expect(installCalls()).toEqual([["installSelection",
      gitRequest("https://github.com/x/y/tree/feature/x/skills", ["skills/alpha"], resolution), expect.anything()]]);
  });

  it.each<GitSourceResolution | null>([
    { branch: "feature/x", subpath: "skills" },
    { branch: null, subpath: null },
    null,
  ])("passes the listing's source resolution unchanged through the picker: %j", async (resolution) => {
    stubBackend({ gitCandidates: [
      { ...gitCandidate("alpha", "skills/alpha"), resolution },
      { ...gitCandidate("beta", "skills/beta"), resolution },
    ] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    act(() => result.current.setGitUrl("https://github.com/x/y/tree/feature/x/skills"));
    await act(async () => { await result.current.handleCreate(); });
    await act(async () => { await result.current.git.install(); });
    // The whole selection is one batch call.
    expect(installCalls()).toEqual([
      ["installSelection", gitRequest("https://github.com/x/y/tree/feature/x/skills", ["skills/alpha", "skills/beta"], resolution), expect.anything()],
    ]);
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
    expect(installCalls()).toHaveLength(0);
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
      expect(mockInvoke).toHaveBeenCalledWith(
        "installSelection",
        expect.objectContaining({
          source: { kind: "git", url: "https://github.com/x/y", resolution: null },
          candidates: [{ subpath: "skills/react", name_override: null }],
        }),
        expect.anything(),
      ),
    );
    expect(mockInvoke).toHaveBeenCalledWith(
      "listGitSkillsCmd",
      "https://github.com/x/y",
      "json-render-react",
    );
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

    expect(mockInvoke).toHaveBeenCalledWith(
      "listGitSkillsCmd",
      "https://github.com/x/y",
      null,
    );
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
    expect(installCalls()).toHaveLength(0);
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
    expect(installCalls()).toHaveLength(0);
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
    expect(installCalls()).toHaveLength(0);
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
      mockInvoke.mock.calls.some(([cmd]) => cmd === "installSelection"),
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
      mockInvoke.mock.calls.some(([cmd]) => cmd === "installSelection"),
    ).toBe(false);
  });

  it("surfaces the backend's refusal of a folder inside a Tool's skills dir as an error Notification that steers to Import", async () => {
    const refusal = {
      code: "LOCAL_SOURCE_INSIDE_TOOL_DIR",
      path: "/home/u/.claude/skills/alpha",
      tool: "claude_code",
    };
    stubBackend({
      localCandidates: [
        {
          name: "alpha",
          description: null,
          subpath: ".",
          valid: true,
          reason: null,
        },
      ],
      // The batch answers the refusal as the candidate's report row.
      candidateFailure: refusal,
    });
    const setup = makeDeps();
    // The real error copy seam, so the assertion reads the key the
    // Notification carries rather than the double's placeholder.
    vi.mocked(setup.reporter.formatError).mockImplementation((err) =>
      describeCommandError(err, t),
    );
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));

    act(() => {
      result.current.setAddModalTab("local");
      result.current.setLocalPath("/home/u/.claude/skills/alpha");
    });
    await act(async () => {
      await result.current.handleCreate();
    });

    // The refusal is the candidate's report row, an error entry the
    // reporter records as an error Notification with the typed copy.
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      {
        title: t("errors.importFailedTitle", { name: "alpha" }),
        message: describeCommandError(refusal, t),
      },
    ]);
    // Nothing was installed, so no ask and no success.
    expect(setup.sync.resolveGlobalCollisions).not.toHaveBeenCalled();
    expect(setup.reporter.setSuccessToastMessage).not.toHaveBeenCalled();
    expect(result.current.showAddModal).toBe(false);
  });
});

describe("useAddSkillFlow import flow", () => {
  const PLAN: OnboardingPlan = {
    total_tools_scanned: 1,
    group_count: 1,
    copy_count: 1,
    invalid: [],
    groups: [
      {
        name: "alpha",
        has_conflict: false,
        name_is_managed: false,
        managed_skill_id: null,
        variants: [
          {
            tool: "claude",
            name: "alpha",
            path: "/home/.claude/skills/alpha",
            fingerprint: null,
            is_link: false,
            link_target: null,
          },
        ],
      },
    ],
  };

  /** An `imported` group with nothing to report. */
  const importedGroup = (
    overrides?: Partial<Extract<ImportGroupStatus, { status: "imported" }>>,
  ): ImportGroupStatus => ({
    status: "imported",
    skill_id: "imported-id",
    skill_name: "alpha",
    targets: [],
    forced_tools: [],
    originals: [],
    ...overrides,
  });

  const report = (status: ImportGroupStatus): ImportReport => ({
    groups: [{ group_name: "alpha", status }],
  });

  /**
   * Plan loads on mount and again on Review (the reviewed plan is always a
   * fresh one); the import is one command returning one report — the hook
   * states the selection and renders what comes back.
   */
  function stubImportBackend(
    planCalls: (() => Promise<unknown>)[],
    importReport: ImportReport = report(importedGroup()),
  ) {
    let call = 0;
    mockInvoke.mockImplementation((command) => {
      switch (command) {
        case "getOnboardingPlan": {
          const next = planCalls[Math.min(call, planCalls.length - 1)];
          call += 1;
          return next() as Promise<unknown>;
        }
        case "importOnboardingSelection":
          return Promise.resolve(importReport);
        default:
          return Promise.resolve(undefined);
      }
    });
  }

  async function runImport(setup: ReturnType<typeof makeDeps>) {
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await waitFor(() => expect(result.current.plan).not.toBeNull());
    await act(async () => {
      await result.current.handleReviewImport();
    });
    await act(async () => {
      await result.current.handleImport();
    });
    return result;
  }

  function importCall() {
    return mockInvoke.mock.calls.find(
      ([cmd]) => cmd === "importOnboardingSelection",
    );
  }

  it("imports every selected group in one call carrying the auto-sync policy", async () => {
    stubImportBackend([() => Promise.resolve(PLAN)]);
    const setup = makeDeps();

    const result = await runImport(setup);

    // One command per batch — not an install/sync/remove sequence per group.
    expect(
      mockInvoke.mock.calls.filter(
        ([cmd]) => cmd === "importOnboardingSelection",
      ),
    ).toHaveLength(1);
    const [, selections, policy] = importCall()!;
    expect(selections).toEqual([
      {
        group_name: "alpha",
        chosen_path: "/home/.claude/skills/alpha",
        name: null,
      },
    ]);
    // goose is selected but not installed and still named; cursor is deselected.
    expect(policy).toEqual({ auto_sync: true, tools: ["claude", "goose"], remove_originals: true });
    expect(setup.reporter.notify).toHaveBeenCalledWith("success", t("outcome.done.imported", { count: 1 }), undefined);
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([]);
    expect(result.current.showImportModal).toBe(false);
  });

  it("names the real directory, not a link listed before it, as a consistent group's chosen variant", async () => {
    // Registry order lists Claude before Pi, so the plan lists Claude's
    // symlink first even though it points at Pi's real directory. A link
    // is never what the app copies when the directory is also in the group.
    const linked: OnboardingPlan = {
      total_tools_scanned: 2,
      group_count: 1,
      copy_count: 2,
      invalid: [],
      groups: [
        {
          name: "alpha",
          has_conflict: false,
          name_is_managed: false,
          managed_skill_id: null,
          variants: [
            {
              tool: "claude",
              name: "alpha",
              path: "/home/.claude/skills/alpha",
              fingerprint: "same",
              is_link: true,
              link_target: "/home/.pi/agent/skills/alpha",
            },
            {
              tool: "pi",
              name: "alpha",
              path: "/home/.pi/agent/skills/alpha",
              fingerprint: "same",
              is_link: false,
              link_target: null,
            },
          ],
        },
      ],
    };
    stubImportBackend([() => Promise.resolve(linked)]);
    const setup = makeDeps();

    const result = await runImport(setup);

    expect(result.current.variantChoice).toEqual({
      alpha: "/home/.pi/agent/skills/alpha",
    });
    const [, selections] = importCall()!;
    expect(selections).toEqual([
      {
        group_name: "alpha",
        chosen_path: "/home/.pi/agent/skills/alpha",
        name: null,
      },
    ]);
  });

  it("asks for original removal instead of tools when auto-sync is off", async () => {
    stubImportBackend([() => Promise.resolve(PLAN)]);
    const setup = makeDeps();
    setup.sync.autoSyncEnabled = false;

    await runImport(setup);

    const [, , policy] = importCall()!;
    expect(policy).toEqual({ auto_sync: false, tools: null, remove_originals: true });
  });

  it("Review always reviews a plan fetched now, never the banner's cached one", async () => {
    // The backend resolves the scan's scope from the saved configuration at
    // fetch time; the import acts on what it finds under that scope, so the
    // reviewed plan must be current.
    const widened = {
      ...PLAN,
      groups: [...PLAN.groups, { name: "beta", has_conflict: false, name_is_managed: false, managed_skill_id: null, variants: [
        { tool: "cursor", name: "beta", path: "/home/.cursor/skills/beta", fingerprint: "f2", is_link: false, link_target: null },
      ] }],
    };
    stubImportBackend([() => Promise.resolve(PLAN), () => Promise.resolve(widened)]);
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await waitFor(() => expect(result.current.plan).not.toBeNull());
    expect(result.current.plan!.groups.map((g) => g.name)).toEqual(["alpha"]);

    await act(async () => {
      await result.current.handleReviewImport();
    });
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === "getOnboardingPlan")).toHaveLength(2);
    expect(result.current.showImportModal).toBe(true);
    expect(result.current.plan!.groups.map((g) => g.name)).toEqual(["alpha", "beta"]);
    expect(result.current.selected).toEqual({ alpha: true, beta: true });
  });

  it("reloads the plan when the saved tool configuration changes", async () => {
    // A narrowed scope can empty the plan (the banner disappears) and a
    // widened one can populate it: the banner follows the configuration.
    const empty = { ...PLAN, group_count: 0, copy_count: 0, groups: [] };
    stubImportBackend([() => Promise.resolve(empty), () => Promise.resolve(PLAN)]);
    const setup = makeDeps();
    const { result, rerender } = renderHook((deps: FlowDeps) => useAddSkillFlow(deps), {
      initialProps: setup.deps,
    });
    await waitFor(() => expect(result.current.plan).not.toBeNull());
    expect(result.current.plan!.group_count).toBe(0);

    rerender({ ...setup.deps, sync: { ...setup.sync, scanScopeRevision: 1 } });
    await waitFor(() => expect(result.current.plan!.group_count).toBe(PLAN.group_count));
    expect(mockInvoke.mock.calls.filter(([cmd]) => cmd === "getOnboardingPlan")).toHaveLength(2);
  });

  it("completes the import even when the post-import plan reload fails", async () => {
    stubImportBackend([
      () => Promise.resolve(PLAN), // mount
      () => Promise.resolve(PLAN), // Review
      () => Promise.reject(new Error("plan reload boom")), // post-import
    ]);
    const setup = makeDeps();

    const result = await runImport(setup);

    // Every selected group imported, so the action completed: success toast
    // fires and the modal closes...
    expect(setup.reporter.notify).toHaveBeenCalledWith("success", t("outcome.done.imported", { count: 1 }), undefined);
    expect(result.current.showImportModal).toBe(false);
    // ...while the reload failure is surfaced on its own, not as an import
    // failure.
    expect(setup.reporter.setError).toHaveBeenCalledWith("plan reload boom");
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([]);
  });

  it.each([false, true])("applies import fold completion=%s and publishes its entries", async (complete) => {
    stubImportBackend([() => Promise.resolve(PLAN)]);
    const setup = makeDeps();
    const outcome: folds.Outcome = { toast: { kind: "warning", tone: "failure", message: "fold toast" }, errors: [{ title: "error", message: "detail" }], warnings: [{ title: "warning", message: "detail" }], notes: [], completion: { reload: complete, closeModal: complete, conflict: false } };
    const spy = vi.spyOn(folds, "importOutcome").mockReturnValue(outcome);
    const result = await runImport(setup);
    expect(spy).toHaveBeenCalledWith(report(importedGroup()), expect.objectContaining({ t }));
    expect(setup.library.loadManagedSkills).toHaveBeenCalledTimes(complete ? 1 : 0);
    expect(result.current.showImportModal).toBe(!complete);
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith(outcome.errors);
    expect(setup.reporter.showActionWarnings).toHaveBeenCalledWith(outcome.warnings);
    expect(setup.reporter.notify).toHaveBeenCalledWith("warning", "fold toast", undefined);
  });
});
afterEach(() => vi.restoreAllMocks());
// No vitest globals: unmount explicitly so a surface's window listeners never
// outlive its test.
afterEach(cleanup);

it.each(["git", "local"] as const)("%s install applies fold completion instead of assuming success", async (source) => {
  for (const complete of [false, true]) {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "alpha")], localCandidates: [{ name: "alpha", subpath: "alpha", description: null, valid: true, reason: null }] });
    const setup = makeDeps();
    const outcome: folds.Outcome = { toast: { kind: "warning", tone: "failure", message: "install fold" }, errors: [{ title: "error", message: "detail" }], warnings: [], notes: [], completion: { reload: complete, closeModal: complete, conflict: false } };
    const spy = vi.spyOn(folds, "installOutcome").mockReturnValue(outcome);
    const { result, unmount } = renderHook(() => useAddSkillFlow(setup.deps));
    act(() => {
      result.current.handleOpenAdd();
      result.current.setAddModalTab(source);
      result.current.setGitUrl("https://github.com/x/y");
      result.current.setLocalPath("/source");
    });
    await act(async () => { await result.current.handleCreate(); });
    expect(spy).toHaveBeenCalledTimes(1);
    expect(setup.library.loadManagedSkills).toHaveBeenCalledTimes(complete ? 1 : 0);
    expect(result.current.showAddModal).toBe(!complete);
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith(outcome.errors);
    expect(setup.reporter.notify).toHaveBeenCalledWith("warning", "install fold", undefined);
    // One awaited applyOutcome carries the completion (S9): the reload runs
    // before the toast, never after an unawaited publish.
    expect(setup.reporter.applyOutcome).toHaveBeenCalledTimes(1);
    if (complete)
      expect(vi.mocked(setup.library.loadManagedSkills).mock.invocationCallOrder[0]).toBeLessThan(
        vi.mocked(setup.reporter.notify).mock.invocationCallOrder.at(-1)!,
      );
    unmount(); spy.mockRestore();
  }
});

it("an installed skill still completes when the overwrite retry throws", async () => {
  stubBackend({ gitCandidates: [gitCandidate("alpha", "alpha")], occupied: ["claude"] });
  const setup = makeDeps();
  setup.sync.resolveGlobalCollisions.mockRejectedValue(new Error("retry failed"));
  const { result } = renderHook(() => useAddSkillFlow(setup.deps));
  act(() => { result.current.handleOpenAdd(); result.current.setGitUrl("https://github.com/x/y"); });
  await act(async () => { await result.current.handleCreate(); });
  expect(result.current.showAddModal).toBe(false);
  expect(setup.library.loadManagedSkills).toHaveBeenCalledTimes(1);
  expect(setup.reporter.setError).toHaveBeenCalledWith("retry failed");
  // The collided row settles as it first answered: a target failure.
  expect(setup.reporter.notify).toHaveBeenCalledWith(
    "warning",
    `${t("outcome.done.installed", { count: 1 })} · ${t("outcome.targetsFailed", { count: 1 })}`,
    undefined,
  );
});

it("one ask per Add batch: every candidate's collisions go to the ask at once, and its answer replaces them", async () => {
  stubBackend({
    gitCandidates: [gitCandidate("alpha", "skills/alpha"), gitCandidate("beta", "skills/beta")],
    occupied: ["claude"],
  });
  const setup = makeDeps();
  // The ask's retry answers both asked pairs synced.
  setup.sync.resolveGlobalCollisions.mockImplementation(async (report) =>
    report.map((row) => ({
      ...row,
      status: { status: "synced", outcome: { mode_used: "symlink", target_path: "/x", replaced: true } },
    })),
  );
  const fold = vi.spyOn(folds, "installOutcome");
  const { result } = renderHook(() => useAddSkillFlow(setup.deps));
  act(() => result.current.setGitUrl("https://github.com/x/y"));
  await act(async () => { await result.current.handleCreate(); });
  await act(async () => { await result.current.git.install(); });

  expect(installCalls()).toHaveLength(1);
  expect(setup.sync.resolveGlobalCollisions).toHaveBeenCalledTimes(1);
  const [asked, skills, policy] = setup.sync.resolveGlobalCollisions.mock.calls[0];
  expect(asked.filter((row) => row.status.status === "failed").map((row) => [row.skill_name, row.tool_key])).toEqual([
    ["alpha", "claude"],
    ["beta", "claude"],
  ]);
  expect(skills).toEqual([
    { skill_id: "new-id-0", name: "alpha", source_path: "/hub/alpha" },
    { skill_id: "new-id-1", name: "beta", source_path: "/hub/beta" },
  ]);
  expect(policy).toEqual({ overwriteIfSameContent: true });
  const settled = fold.mock.calls[0][0];
  expect(
    settled.flatMap((item) =>
      item.status === "installed" && item.deployment.status === "reported"
        ? item.deployment.report.map((row) => row.status.status)
        : [],
    ),
  ).toEqual(["synced", "synced", "synced", "synced"]);
});

it("an empty selection deploys nowhere and says so (no-targets), even with tools detected", async () => {
  stubBackend({ gitCandidates: [gitCandidate("alpha", "alpha")] });
  const setup = makeDeps();
  setup.sync.syncTargets = { claude: false, cursor: false, goose: false };
  const { result } = renderHook(() => useAddSkillFlow(setup.deps));
  act(() => { result.current.handleOpenAdd(); result.current.setGitUrl("https://github.com/x/y"); });
  await act(async () => { await result.current.handleCreate(); });
  expect(installCalls()[0][1]).toMatchObject({ tools: [], deploy: true });
  expect(result.current.showAddModal).toBe(false);
  expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([{ title: t("errors.unsyncedTitle", { name: "installed-skill" }), message: t("errors.noSyncTargets") }]);
  expect(setup.reporter.notify).toHaveBeenCalledWith(
    "warning",
    `${t("outcome.done.installed", { count: 1 })} · ${t("outcome.targetsFailed", { count: 1 })}`,
    undefined,
  );
});

describe("useAddSkillFlow local folder picker (platform seam)", () => {
  it("fills the local path from the picked folder; a cancel changes nothing; a failure is reported", async () => {
    stubBackend();
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    pickFolder.mockResolvedValueOnce(["/home/op/my-skill"]);
    await act(async () => { await result.current.handlePickLocalPath(); });
    expect(pickFolder).toHaveBeenCalledWith({ title: "selectLocalFolder" });
    expect(result.current.localPath).toBe("/home/op/my-skill");

    pickFolder.mockResolvedValueOnce([]);
    await act(async () => { await result.current.handlePickLocalPath(); });
    expect(result.current.localPath).toBe("/home/op/my-skill");

    pickFolder.mockRejectedValueOnce({ code: "OTHER", message: "dialog failed" });
    await act(async () => { await result.current.handlePickLocalPath(); });
    expect(setup.reporter.setError).toHaveBeenCalled();
    expect(result.current.localPath).toBe("/home/op/my-skill");
  });
});

describe("useAddSkillFlow cancellation at the action boundary (ticket 04 review F2)", () => {
  /** The flow over the real reporter: its pending record and Cancel are under test. */
  function useFlowWithReporter(deps: FlowDeps) {
    const reporter = useStatusReporter(t);
    const flow = useAddSkillFlow({ ...deps, reporter });
    return { flow, reporter };
  }

  function deferred<T>() {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>((r) => (resolve = r));
    return { promise, resolve };
  }

  const cancelCalls = () => mockInvoke.mock.calls.filter(([cmd]) => cmd === "cancelCurrentOperation");

  it.each(["git", "local"] as const)(
    "%s: a Cancel pressed while the listing runs sends no installSelection; a later Add installs",
    async (kind) => {
      stubBackend({
        gitCandidates: [gitCandidate("alpha", "skills/alpha")],
        localCandidates: [{ name: "alpha", description: null, subpath: "alpha", valid: true, reason: null }],
      });
      const listCommand = kind === "git" ? "listGitSkillsCmd" : "listLocalSkillsCmd";
      const base = mockInvoke.getMockImplementation()!;
      const listing = deferred<void>();
      mockInvoke.mockImplementation((command, ...args) =>
        command === listCommand ? listing.promise.then(() => base(command, ...args)) : base(command, ...args),
      );
      const setup = makeDeps();
      const { result } = renderHook(() => useFlowWithReporter(setup.deps));
      // Let the mount's plan read settle (a mutation is refused while it runs).
      await act(async () => {});
      act(() => {
        if (kind === "git") {
          result.current.flow.setGitUrl("https://github.com/x/y");
        } else {
          result.current.flow.setAddModalTab("local");
          result.current.flow.setLocalPath("/some/dir");
        }
      });

      let task!: Promise<void>;
      await act(async () => {
        task = result.current.flow.handleCreate();
      });
      expect(result.current.reporter.pending?.cancellable).toBe(true);
      act(() => result.current.reporter.cancelPending());
      expect(cancelCalls()).toHaveLength(1);
      await act(async () => {
        listing.resolve();
        await task;
      });

      expect(installCalls()).toHaveLength(0);
      expect(result.current.reporter.pending).toBeNull();

      // A fresh Add is not poisoned by the earlier Cancel.
      mockInvoke.mockImplementation(base);
      await act(async () => {
        await result.current.flow.handleCreate();
      });
      expect(installCalls()).toHaveLength(1);
    },
  );

  it("a Cancel during the batch reports the cancelled tail; the ask's retry offers no Cancel", async () => {
    stubBackend({
      gitCandidates: [gitCandidate("alpha", "skills/alpha"), gitCandidate("beta", "skills/beta")],
      occupied: ["claude"],
    });
    const base = mockInvoke.getMockImplementation()!;
    const batch = deferred<void>();
    mockInvoke.mockImplementation((command, ...args) =>
      command === "installSelection"
        ? batch.promise.then(async () => {
            // Cancelled between candidates: the second answers cancelled.
            const report = (await base(command, ...args)) as InstallBatchReport;
            return {
              candidates: report.candidates.map((c, i) => (i === 0 ? c : { ...c, status: { status: "cancelled" } })),
            };
          })
        : base(command, ...args),
    );
    const setup = makeDeps();
    const retry = deferred<void>();
    setup.sync.resolveGlobalCollisions.mockImplementation(async (report) => {
      await retry.promise;
      return report;
    });
    const fold = vi.spyOn(folds, "installOutcome");
    const { result } = renderHook(() => useFlowWithReporter(setup.deps));
    await act(async () => {});
    act(() => result.current.flow.setGitUrl("https://github.com/x/y"));
    await act(async () => {
      await result.current.flow.handleCreate();
    });

    let task!: Promise<void>;
    await act(async () => {
      task = result.current.flow.git.install();
    });
    expect(installCalls()).toHaveLength(1);
    expect(result.current.reporter.pending?.cancellable).toBe(true);
    act(() => result.current.reporter.cancelPending());
    expect(cancelCalls()).toHaveLength(1);

    await act(async () => {
      batch.resolve();
    });
    // The retry is a plain sync: Cancel is withdrawn while it runs.
    expect(setup.sync.resolveGlobalCollisions).toHaveBeenCalledTimes(1);
    expect(result.current.reporter.pending?.cancellable).toBe(false);

    await act(async () => {
      retry.resolve();
      await task;
    });
    expect(fold.mock.calls[0][0].map((item) => item.status)).toEqual(["installed", "cancelled"]);
    expect(result.current.reporter.pending).toBeNull();
    expect(installCalls()).toHaveLength(1);
  });
});
