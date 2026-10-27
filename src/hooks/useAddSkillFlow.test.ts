// Tests at the AddSkillFlow seam (ticket 16): Add as one stepped surface —
// open({ source?, preselect? }) → list → candidates (the visible-and-checked
// rule) → install() as one `installSelection` batch under one action → the
// settlement (installed / failed / cancelled rows), one overwrite ask over
// the batch's collisions, retry(ids) for what did not land, and the close
// rule (only on full success or the operator's Close). The import half of
// the world is tested below. Backend mocked at the invokeTauri module seam;
// sync and library worlds enter as mocked dependency interfaces.

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
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn(), info: vi.fn(), dismiss: vi.fn() },
}));
vi.mock("../lib/tauri", () => ({
  isTauri: true,
  invokeTauri: vi.fn(),
}));
// The batches stream progress over a channel from the platform seam, stubbed
// with a plain message sink; the folder picker is its other door.
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
  const flow = useAddSkillFlowHook({ ...deps, surface });
  return { ...flow, surface };
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

const RATE_LIMITED = { code: "RATE_LIMITED" as const, resetMinutes: 12 };

function stubBackend(overrides?: {
  gitCandidates?: GitSkillCandidate[];
  /** What the backend resolved a `targetName` to; `none` by default. */
  targetMatch?: CandidateMatch;
  localCandidates?: LocalSkillCandidate[];
  /** Tools whose target the install batch reports occupied (TARGET_EXISTS). */
  occupied?: string[];
  /** Tools the batch skips as unwritable (TOOL_NOT_WRITABLE); the set is live, so a test can repair it. */
  unwritable?: Set<string>;
  /** Subpaths the batch answers `cancelled` the first time (a Cancel between candidates). */
  cancelledOnce?: string[];
  /** Subpaths whose install fails (every attempt, or `once`). */
  failing?: string[];
  failOnce?: boolean;
  /** What a failing subpath answers (rate limited by default). */
  failError?: unknown;
  /** The listing throws this. */
  listingError?: unknown;
}) {
  const failing = new Set(overrides?.failing ?? []);
  const cancelling = new Set(overrides?.cancelledOnce ?? []);
  mockInvoke.mockImplementation((command, ...args) => {
    switch (command) {
      case "getOnboardingPlan":
        return Promise.resolve(EMPTY_PLAN);
      case "listGitSkillsCmd": {
        if (overrides?.listingError) return Promise.reject(overrides.listingError);
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
        if (overrides?.listingError) return Promise.reject(overrides.listingError);
        return Promise.resolve(overrides?.localCandidates ?? []);
      case "installSelection": {
        // One report for the whole batch (ticket 04): every candidate is
        // installed and first-synced to the request's Tools; an occupied
        // Tool is a collected TARGET_EXISTS; a failing subpath is `failed`.
        const [request] = args as [InstallRequest];
        const report: InstallBatchReport = {
          candidates: request.candidates.map((candidate) => {
            const name = candidate.name_override ?? candidate.subpath.split("/").pop()!;
            if (cancelling.delete(candidate.subpath)) {
              return { subpath: candidate.subpath, name, status: { status: "cancelled" as const } };
            }
            if (failing.has(candidate.subpath)) {
              if (overrides?.failOnce) failing.delete(candidate.subpath);
              return { subpath: candidate.subpath, name, status: { status: "failed", error: (overrides?.failError ?? RATE_LIMITED) as never } };
            }
            const skill_id = `id-${name}`;
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
                        : overrides?.unwritable?.has(tool)
                          ? { status: "skipped" as const, error: { code: "TOOL_NOT_WRITABLE" as const, tool, path: `/${tool}` } }
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
    err instanceof Error ? err.message : `formatted:${JSON.stringify(err)}`,
  );
  const setError = vi.fn();
  const setSuccessToastMessage = vi.fn();
  // Stub of the runAction contract: the body's outcome lands on the same
  // one-shot setters the real reporter uses, so assertions read naturally.
  // The lifecycle itself (pending, Cancel) is the reporter's own test.
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
    openReport: vi.fn(),
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
    refuseHeld: vi.fn(),
    cancelPending: vi.fn(),
  };
  const sync = {
    autoSyncEnabled: true,
    scanScopeRevision: 0,
    // The one overwrite ask + retry: answers the report as settled.
    resolveGlobalCollisions: vi.fn<SyncOrchestration["resolveGlobalCollisions"]>(async (report) => report),
    // cursor is deselected; goose is selected but not installed and stays
    // in the deploy set (round 12 D3: the batch reports it as a skip).
    syncTargets: { claude: true, cursor: false, goose: true } as Record<string, boolean>,
    handleSyncTargetChange: vi.fn<SyncOrchestration["handleSyncTargetChange"]>(async () => undefined),
  };
  const catalog = {
    toolLabelById: { claude: "CLAUDE", cursor: "CURSOR" },
    tools: [
      { id: "claude", label: "CLAUDE" },
      { id: "cursor", label: "CURSOR" },
      { id: "goose", label: "GOOSE" },
    ],
    installedTools: [
      { id: "claude", label: "CLAUDE" },
      { id: "cursor", label: "CURSOR" },
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

/** The install batch request a git Add sends for `subpaths`. */
const gitRequest = (
  url: string,
  subpaths: string[],
  resolution: GitSourceResolution | null = null,
  names: (string | null)[] = subpaths.map(() => null),
) => ({
  source: { kind: "git", url, resolution },
  candidates: subpaths.map((subpath, i) => ({ subpath, name_override: names[i] })),
  tools: ["claude", "goose"],
  overwrite_if_same_content: true,
  overrides: [],
  deploy: true,
});

type Flow = ReturnType<typeof useAddSkillFlow>;
type Rendered = { current: Flow };

/** Open Add on a git URL and list it (Source → Candidates). */
async function listGit(result: Rendered, url = "https://github.com/x/y") {
  act(() => result.current.open());
  act(() => result.current.add.editDraft({ url }));
  await act(async () => {
    await result.current.add.list();
  });
}

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => vi.restoreAllMocks());
// No vitest globals: unmount explicitly so a surface's window listeners never
// outlive its test.
afterEach(cleanup);

describe("Add: open and the source step", () => {
  it("open() shows the one Add dialog on the source step; the kind can be named", () => {
    stubBackend();
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    expect(result.current.add.open).toBe(false);
    act(() => result.current.open({ kind: "local" }));
    expect(result.current.add.open).toBe(true);
    expect(result.current.surface.stack.map((layer) => layer.kind)).toEqual(["addSkill"]);
    expect(result.current.add.step).toBe("source");
    expect(result.current.add.kind).toBe("local");
    expect(result.current.add.summary.source).toBeNull();
    expect(result.current.add.summary.candidates).toBeNull();
    // The Tool scope: the effective deploy set, labelled by the catalog.
    expect(result.current.add.summary.tools).toEqual([
      { id: "claude", label: "CLAUDE" },
      { id: "goose", label: "GOOSE" },
    ]);
    // The installed Tools, plus a selected one detection no longer sees (so it can be unticked).
    expect(result.current.add.toolChoices).toEqual([
      { id: "claude", label: "CLAUDE", detected: true },
      { id: "cursor", label: "CURSOR", detected: true },
      { id: "goose", label: "GOOSE", detected: false },
    ]);
  });

  it("an empty URL is refused by the field without touching the backend", async () => {
    stubBackend();
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result, "  ");
    expect(result.current.add.sourceError).toBe("errors.requireGitUrl");
    expect(mockInvoke.mock.calls.some(([cmd]) => cmd === "listGitSkillsCmd")).toBe(false);
    expect(result.current.add.step).toBe("source");
  });

  it("a listing refusal and an empty listing stay on the source, said by the field (never a toast)", async () => {
    stubBackend({ listingError: { code: "INVALID_GITHUB_URL", url: "nope" } });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result, "nope");
    expect(result.current.add.step).toBe("source");
    expect(result.current.add.sourceError).toBe('formatted:{"code":"INVALID_GITHUB_URL","url":"nope"}');
    expect(setup.reporter.setError).not.toHaveBeenCalled();

    stubBackend({ gitCandidates: [] });
    await act(async () => {
      await result.current.add.list();
    });
    expect(result.current.add.sourceError).toBe("errors.noSkillsFoundWithHint");
    expect(result.current.add.step).toBe("source");
  });

  it("the listing is a cancellable read (its Cancel discards it) under Add's label", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    const opts = vi.mocked(setup.reporter.runAction).mock.calls.at(-1)![0];
    expect(opts).toMatchObject({ read: true, cancel: "installSelection", label: "activity.labels.add", activity: "add.finding" });
  });

  it("Browse fills the local path through the platform seam; a cancel changes nothing; a failure is reported", async () => {
    stubBackend();
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    act(() => result.current.open({ kind: "local" }));
    pickFolder.mockResolvedValueOnce(["/home/op/my-skill"]);
    await act(async () => { await result.current.add.browse(); });
    expect(pickFolder).toHaveBeenCalledWith({ title: "selectLocalFolder" });
    expect(result.current.add.draft.path).toBe("/home/op/my-skill");

    pickFolder.mockResolvedValueOnce([]);
    await act(async () => { await result.current.add.browse(); });
    expect(result.current.add.draft.path).toBe("/home/op/my-skill");

    pickFolder.mockRejectedValueOnce({ code: "OTHER", message: "dialog failed" });
    await act(async () => { await result.current.add.browse(); });
    expect(setup.reporter.setError).toHaveBeenCalled();
    expect(result.current.add.draft.path).toBe("/home/op/my-skill");
  });
});

describe("Add: the candidates step (the visible-and-checked rule, #47)", () => {
  const FOUR = ["lint", "commit", "changelog", "notes"].map((name) => gitCandidate(name, name));

  it("lists every candidate checked, then only checked AND visible ones install", async () => {
    stubBackend({ gitCandidates: FOUR });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    expect(result.current.add.step).toBe("candidates");
    expect(result.current.add.summary.source).toBe("x/y");
    expect(result.current.add.summary.candidates).toEqual({ total: 4, checked: 4, hidden: 0, hiddenChecked: 0 });

    // The search hides lint and notes (still checked): they do not install.
    act(() => result.current.add.setQuery("C"));
    expect(result.current.add.visible.map((c) => c.name)).toEqual(["commit", "changelog"]);
    expect(result.current.add.summary.candidates).toEqual({ total: 4, checked: 2, hidden: 2, hiddenChecked: 2 });
    // Unchecking one shown: 1 of 4.
    act(() => result.current.add.toggle("commit", false));
    expect(result.current.add.summary.candidates?.checked).toBe(1);
    // Select all touches only what is shown.
    act(() => result.current.add.toggleAll(false));
    expect(result.current.add.selected).toEqual({ lint: true, commit: false, changelog: false, notes: true });
    act(() => result.current.add.toggleAll(true));

    await act(async () => { await result.current.install(); });
    expect(installCalls()).toEqual([
      ["installSelection", gitRequest("https://github.com/x/y", ["commit", "changelog"]), expect.anything()],
    ]);
  });

  it("local invalid candidates are listed unavailable with the reason in words, never checked", async () => {
    stubBackend({
      localCandidates: [
        { name: "good", description: null, subpath: "good", valid: true, reason: null },
        { name: "draft", description: null, subpath: "draft", valid: false, reason: "missing_name" },
      ],
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    act(() => result.current.open({ kind: "local" }));
    act(() => result.current.add.editDraft({ path: "/team" }));
    await act(async () => { await result.current.add.list(); });
    expect(result.current.add.candidates.map((c) => [c.name, c.unavailable])).toEqual([
      ["good", null],
      ["draft", "localSkillInvalid.missingName"],
    ]);
    expect(result.current.add.selected).toEqual({ good: true, draft: false });
    act(() => result.current.add.toggle("draft", true));
    expect(result.current.add.selected.draft).toBe(false);
    await act(async () => { await result.current.install(); });
    expect(installCalls()[0][1]).toMatchObject({ source: { kind: "local", root: "/team" }, candidates: [{ subpath: "good", name_override: null }] });
  });

  it("a folder inside a Tool's skills dir is the candidate's failed row, in the words that steer to Import", async () => {
    const refusal = { code: "LOCAL_SOURCE_INSIDE_TOOL_DIR", path: "/home/u/.claude/skills/alpha", tool: "claude_code" };
    stubBackend({
      localCandidates: [{ name: "alpha", description: null, subpath: ".", valid: true, reason: null }],
      failing: ["."],
      failError: refusal,
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    act(() => result.current.open({ kind: "local" }));
    act(() => result.current.add.editDraft({ path: "/home/u/.claude/skills/alpha" }));
    await act(async () => { await result.current.add.list(); });
    await act(async () => { await result.current.install(); });
    expect(result.current.add.settlement).toEqual([
      { id: ".", name: "alpha", status: "failed", detail: describeCommandError(refusal, t), attention: false, retryable: true },
    ]);
    // The report records it as an error Notification with the typed copy.
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      { title: t("add.failedTitle", { name: "alpha" }), message: describeCommandError(refusal, t), error: refusal },
    ]);
    expect(setup.sync.resolveGlobalCollisions).not.toHaveBeenCalled();
    expect(result.current.add.open).toBe(true);
  });

  it("a lone candidate takes the optional name; a taken name is refused inline, nothing sent", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps({ takenNames: ["Taken"] });
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    expect(result.current.add.nameApplies).toBe(true);
    act(() => result.current.add.editDraft({ name: "taken" }));
    await act(async () => { await result.current.install(); });
    expect(result.current.add.pickError).toBe('errors.skillAlreadyExists {"name":"taken"}');
    expect(installCalls()).toHaveLength(0);
    expect(result.current.add.step).toBe("candidates");

    act(() => result.current.add.editDraft({ name: "mine" }));
    expect(result.current.add.pickError).toBeNull();
    await act(async () => { await result.current.install(); });
    expect(installCalls()[0][1]).toEqual(gitRequest("https://github.com/x/y", ["skills/alpha"], null, ["mine"]));
  });

  it("nothing checked is refused inline", async () => {
    stubBackend({ gitCandidates: FOUR });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    act(() => result.current.add.toggleAll(false));
    await act(async () => { await result.current.install(); });
    expect(result.current.add.pickError).toBe("errors.selectAtLeastOneSkill");
    expect(installCalls()).toHaveLength(0);
  });

  it.each<GitSourceResolution | null>([
    { branch: "feature/x", subpath: "skills" },
    { branch: null, subpath: null },
    null,
  ])("passes the listing's source resolution unchanged: %j", async (resolution) => {
    stubBackend({ gitCandidates: [
      { ...gitCandidate("alpha", "skills/alpha"), resolution },
      { ...gitCandidate("beta", "skills/beta"), resolution },
    ] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result, "https://github.com/x/y/tree/feature/x/skills");
    await act(async () => { await result.current.install(); });
    expect(installCalls()).toEqual([
      ["installSelection", gitRequest("https://github.com/x/y/tree/feature/x/skills", ["skills/alpha", "skills/beta"], resolution), expect.anything()],
    ]);
  });

  it("Back returns to the source keeping the draft", async () => {
    stubBackend({ gitCandidates: FOUR });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    act(() => result.current.add.back());
    expect(result.current.add.step).toBe("source");
    expect(result.current.add.draft.url).toBe("https://github.com/x/y");
  });
});

describe("Add: the source kind while a listing runs (ticket 16 A3)", () => {
  function deferred() {
    let resolve!: () => void;
    let reject!: (err: unknown) => void;
    const promise = new Promise<void>((res, rej) => {
      resolve = res;
      reject = rej;
    });
    return { promise, resolve, reject };
  }

  const local = [{ name: "alpha", description: null, subpath: "alpha", valid: true, reason: null }];

  /** Start a listing of `kind` that waits on the returned gate. */
  async function startListing(kind: "git" | "local") {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")], localCandidates: local });
    const base = mockInvoke.getMockImplementation()!;
    const gate = deferred();
    const listCommand = kind === "git" ? "listGitSkillsCmd" : "listLocalSkillsCmd";
    mockInvoke.mockImplementation((command, ...args) =>
      command === listCommand ? gate.promise.then(() => base(command, ...args)) : base(command, ...args),
    );
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    act(() => result.current.open({ kind }));
    act(() => result.current.add.editDraft(kind === "git" ? { url: "https://github.com/x/y" } : { path: "/some/dir" }));
    let task!: Promise<void>;
    await act(async () => {
      task = result.current.add.list();
    });
    expect(result.current.add.running).toBe("listing");
    return { result, gate, task, base };
  }

  it.each([
    ["git", "local"],
    ["local", "git"],
  ] as const)("%s → %s is refused while the listing runs; the listing lands on its own source", async (from, to) => {
    const { result, gate, task } = await startListing(from);
    act(() => result.current.add.setKind(to));
    expect(result.current.add.kind).toBe(from);
    await act(async () => {
      gate.resolve();
      await task;
    });
    expect(result.current.add.step).toBe("candidates");
    expect(result.current.add.kind).toBe(from);
    expect(result.current.add.candidates.map((candidate) => candidate.name)).toEqual(["alpha"]);
    expect(result.current.add.summary.candidates).toMatchObject({ total: 1, checked: 1 });
    // After it, the kind switches again.
    act(() => result.current.add.back());
    act(() => result.current.add.setKind(to));
    expect(result.current.add.kind).toBe(to);
  });

  it.each([
    ["git", "local"],
    ["local", "git"],
  ] as const)("%s → %s refused while the listing runs; a rejection is said by the same source's field", async (from, to) => {
    const { result, gate, task } = await startListing(from);
    act(() => result.current.add.setKind(to));
    await act(async () => {
      gate.reject(new Error("unreachable"));
      await task;
    });
    expect(result.current.add.step).toBe("source");
    expect(result.current.add.kind).toBe(from);
    expect(result.current.add.sourceError).toBe("unreachable");
  });

  it("a listing superseded by a fresh opening is discarded: it never moves the step", async () => {
    const { result, gate, task, base } = await startListing("git");
    // Explore's install re-opens Add on another source while the first listing is out.
    mockInvoke.mockImplementation(base);
    act(() => result.current.open({ kind: "local" }));
    await act(async () => {
      gate.resolve();
      await task;
    });
    expect(result.current.add.step).toBe("source");
    expect(result.current.add.kind).toBe("local");
    expect(result.current.add.summary.candidates).toBeNull();
    // A fresh listing of the current source lands.
    act(() => result.current.add.editDraft({ path: "/some/dir" }));
    await act(async () => {
      await result.current.add.list();
    });
    expect(result.current.add.step).toBe("candidates");
    expect(result.current.add.candidates.map((candidate) => candidate.name)).toEqual(["alpha"]);
  });
});

describe("Add: Explore's intent — open({ source, preselect })", () => {
  it("lists the repository at once with the named skill checked, and installs nothing by itself", async () => {
    // skills.sh name vs SKILL.md name ("json-render-react" vs "react"): the
    // matching rule lives in core; the flow only follows the resolution.
    stubBackend({
      gitCandidates: [gitCandidate("react", "skills/react"), gitCandidate("vue", "skills/vue")],
      targetMatch: { kind: "resolved", subpath: "skills/react" },
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await act(async () => {
      result.current.open({ source: { kind: "git", url: "https://github.com/x/y" }, preselect: "json-render-react" });
    });
    await waitFor(() => expect(result.current.add.step).toBe("candidates"));
    expect(mockInvoke).toHaveBeenCalledWith("listGitSkillsCmd", "https://github.com/x/y", "json-render-react");
    expect(result.current.add.draft.url).toBe("https://github.com/x/y");
    expect(result.current.add.selected).toEqual({ "skills/react": true, "skills/vue": false });
    expect(result.current.add.preselectMissing).toBeNull();
    expect(installCalls()).toHaveLength(0);
  });

  it("a skill the listing does not hold is said, and nothing is checked", async () => {
    stubBackend({
      gitCandidates: [gitCandidate("react", "skills/react"), gitCandidate("vue", "skills/vue")],
      targetMatch: { kind: "none" },
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await act(async () => {
      result.current.open({ source: { kind: "git", url: "https://github.com/x/y" }, preselect: "svelte" });
    });
    await waitFor(() => expect(result.current.add.step).toBe("candidates"));
    expect(result.current.add.preselectMissing).toBe("svelte");
    expect(result.current.add.selected).toEqual({ "skills/react": false, "skills/vue": false });
  });

  it("an ambiguous name checks every candidate it matched", async () => {
    stubBackend({
      gitCandidates: [gitCandidate("react", "a/react"), gitCandidate("react", "b/react"), gitCandidate("vue", "skills/vue")],
      targetMatch: { kind: "ambiguous", subpaths: ["a/react", "b/react"] },
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await act(async () => {
      result.current.open({ source: { kind: "git", url: "https://github.com/x/y" }, preselect: "react" });
    });
    await waitFor(() => expect(result.current.add.step).toBe("candidates"));
    expect(result.current.add.selected).toEqual({ "a/react": true, "b/react": true, "skills/vue": false });
  });

  it("a manual listing sends no target name", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    expect(mockInvoke).toHaveBeenCalledWith("listGitSkillsCmd", "https://github.com/x/y", null);
  });
});

describe("Add: install() and the settlement", () => {
  it("one batch under one cancellable action; full success reloads and closes", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha"), gitCandidate("beta", "skills/beta")] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });

    expect(installCalls()).toHaveLength(1);
    const opts = vi.mocked(setup.reporter.runAction).mock.calls.at(-1)![0];
    expect(opts).toMatchObject({ cancel: "installSelection", label: "activity.labels.add", activity: "activity.adding" });
    expect(setup.sync.resolveGlobalCollisions).not.toHaveBeenCalled();
    expect(setup.library.loadManagedSkills).toHaveBeenCalledTimes(1);
    expect(setup.reporter.notify).toHaveBeenCalledWith("success", t("outcome.done.installed", { count: 2 }), undefined);
    // Full success: the dialog closes and Add forgets its draft.
    expect(result.current.add.open).toBe(false);
    expect(result.current.add.step).toBe("source");
    expect(result.current.add.draft.url).toBe("");
  });

  it("a partial failure stays on the settlement: rows ticked, the failure with its reason and Retry", async () => {
    stubBackend({
      gitCandidates: [gitCandidate("alpha", "skills/alpha"), gitCandidate("beta", "skills/beta")],
      failing: ["skills/beta"],
      failOnce: true,
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });

    expect(result.current.add.open).toBe(true);
    expect(result.current.add.step).toBe("settlement");
    expect(result.current.add.settled).toBe(false);
    expect(result.current.add.settlement.map((row) => [row.name, row.status, row.retryable])).toEqual([
      ["alpha", "installed", false],
      ["beta", "failed", true],
    ]);
    expect(result.current.add.settlement[1].detail).toBe('errors.rateLimited {"minutes":12}');
    // What landed is reloaded.
    expect(setup.library.loadManagedSkills).toHaveBeenCalledTimes(1);

    // retry(ids) re-sends the failed candidate alone; once it lands the
    // settlement is complete and the dialog closes.
    await act(async () => { await result.current.retry(["skills/alpha", "skills/beta"]); });
    expect(installCalls()).toHaveLength(2);
    expect(installCalls()[1][1]).toEqual(gitRequest("https://github.com/x/y", ["skills/beta"]));
    expect(result.current.add.open).toBe(false);
  });

  it("a failure that fails again stays retryable and the dialog stays", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")], failing: ["skills/alpha"] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });
    await act(async () => { await result.current.retry(["skills/alpha"]); });
    expect(installCalls()).toHaveLength(2);
    expect(result.current.add.open).toBe(true);
    expect(result.current.add.settlement.map((row) => row.status)).toEqual(["failed"]);
  });

  it("a whole-command failure settles every candidate failed and retryable", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const base = mockInvoke.getMockImplementation()!;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "installSelection" ? Promise.reject({ code: "OTHER", message: "roots" }) : base(command, ...args),
    );
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });
    expect(result.current.add.settlement.map((row) => [row.status, row.retryable])).toEqual([["failed", true]]);
    expect(result.current.add.open).toBe(true);
  });

  it("the Close intent closes the settlement and forgets it", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")], failing: ["skills/alpha"] });
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });
    act(() => result.current.add.close());
    expect(result.current.add.open).toBe(false);
    expect(result.current.add.settlement).toEqual([]);
  });

  it("an empty Tool selection deploys nowhere and says so (no-targets); the dialog stays", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps();
    setup.sync.syncTargets = { claude: false, cursor: false, goose: false };
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });
    expect(installCalls()[0][1]).toMatchObject({ tools: [], deploy: true });
    expect(result.current.add.open).toBe(true);
    expect(result.current.add.settlement[0]).toMatchObject({ status: "installed", detail: "add.settlement.noTools", attention: true });
    expect(setup.reporter.showActionErrors).toHaveBeenCalledWith([
      {
        title: t("errors.unsyncedTitle", { name: "alpha" }),
        message: t("errors.noSyncTargets"),
        subject: { skillId: "id-alpha", skillName: "alpha" },
      },
    ]);
  });

  it("the Tool scope edits the sync world's deploy targets", () => {
    stubBackend();
    const setup = makeDeps();
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    act(() => result.current.add.setTool("cursor", true));
    expect(setup.sync.handleSyncTargetChange).toHaveBeenCalledWith("cursor", true);
  });

  it("auto-sync off adds without deploying (the scope says so)", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")] });
    const setup = makeDeps();
    setup.sync.autoSyncEnabled = false;
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    expect(result.current.add.summary.deploy).toBe(false);
    await listGit(result);
    await act(async () => { await result.current.install(); });
    expect(installCalls()[0][1]).toMatchObject({ tools: [], deploy: false });
    expect(result.current.add.open).toBe(false);
  });
});

describe("Add: the Result closes only when every accumulated entry is complete (ticket 16 A2)", () => {
  const two = [gitCandidate("alpha", "skills/alpha"), gitCandidate("beta", "skills/beta")];

  /** List, install (beta fails once), then retry beta alone. */
  async function installThenRetryBeta(setup: ReturnType<typeof makeDeps>, between?: () => void) {
    const rendered = renderHook(() => useAddSkillFlow(setup.deps));
    const { result } = rendered;
    await listGit(result);
    await act(async () => { await result.current.install(); });
    expect(result.current.add.open).toBe(true);
    between?.();
    rendered.rerender();
    vi.mocked(setup.reporter.notify).mockClear();
    vi.mocked(setup.reporter.applyOutcome).mockClear();
    await act(async () => { await result.current.retry(["skills/beta"]); });
    // The retry sends beta alone and folds its own run once — no replayed toast for alpha.
    expect(installCalls()).toHaveLength(2);
    expect(installCalls()[1][1]).toMatchObject({ candidates: [{ subpath: "skills/beta", name_override: null }] });
    expect(setup.reporter.applyOutcome).toHaveBeenCalledTimes(1);
    expect(setup.reporter.notify).toHaveBeenCalledTimes(1);
    return result;
  }

  it("an earlier folder left to review (the ask dismissed) keeps the Result after a clean Retry", async () => {
    stubBackend({ gitCandidates: two, failing: ["skills/beta"], failOnce: true, occupied: ["claude"] });
    const setup = makeDeps();
    const result = await installThenRetryBeta(setup, () => {
      // The ask is dismissed on the first run; the retry's target is clear.
      stubBackend({ gitCandidates: two });
    });
    expect(result.current.add.open).toBe(true);
    expect(result.current.add.settled).toBe(false);
    expect(result.current.add.settlement.map((row) => [row.name, row.status, row.attention])).toEqual([
      ["alpha", "installed", true],
      ["beta", "installed", false],
    ]);
    expect(setup.reporter.notify).toHaveBeenCalledWith("success", t("add.added", { name: "beta" }), undefined);
  });

  it("an earlier target failure keeps the Result after a clean Retry (the permission repaired meanwhile)", async () => {
    const unwritable = new Set(["goose"]);
    stubBackend({ gitCandidates: two, failing: ["skills/beta"], failOnce: true, unwritable });
    const setup = makeDeps();
    const result = await installThenRetryBeta(setup, () => unwritable.clear());
    expect(result.current.add.open).toBe(true);
    expect(result.current.add.settlement.map((row) => [row.status, row.attention])).toEqual([
      ["installed", true],
      ["installed", false],
    ]);
  });

  it("an earlier no-Tools install keeps the Result after a clean Retry", async () => {
    stubBackend({ gitCandidates: two, failing: ["skills/beta"], failOnce: true });
    const setup = makeDeps();
    setup.sync.syncTargets = { claude: false, cursor: false, goose: false };
    const result = await installThenRetryBeta(setup, () => {
      setup.sync.syncTargets = { claude: true, cursor: false, goose: true };
    });
    expect(installCalls()[1][1]).toMatchObject({ tools: ["claude", "goose"] });
    expect(result.current.add.open).toBe(true);
    expect(result.current.add.settlement[0]).toMatchObject({ detail: "add.settlement.noTools", attention: true });
  });

  it("a cancelled remainder keeps the Result; adding it cleanly too closes it", async () => {
    const three = [...two, gitCandidate("gamma", "skills/gamma")];
    stubBackend({ gitCandidates: three, failing: ["skills/beta"], failOnce: true, cancelledOnce: ["skills/gamma"] });
    const setup = makeDeps();
    const result = await installThenRetryBeta(setup);
    expect(result.current.add.open).toBe(true);
    expect(result.current.add.settlement.map((row) => row.status)).toEqual(["installed", "installed", "cancelled"]);
    await act(async () => { await result.current.retry(["skills/gamma"]); });
    expect(installCalls()[2][1]).toMatchObject({ candidates: [{ subpath: "skills/gamma", name_override: null }] });
    expect(result.current.add.open).toBe(false);
  });

  it("an earlier Keep is the operator's answer: the final clean Retry closes the Result", async () => {
    stubBackend({ gitCandidates: two, failing: ["skills/beta"], failOnce: true, occupied: ["claude"] });
    const setup = makeDeps();
    setup.sync.resolveGlobalCollisions.mockImplementation(async (report) =>
      report.map((row) => (row.status.status === "failed" ? { ...row, status: { status: "declined" } } : row)),
    );
    const result = await installThenRetryBeta(setup, () => stubBackend({ gitCandidates: two }));
    expect(result.current.add.open).toBe(false);
  });

  it("a Keep-only first run closes at once", async () => {
    stubBackend({ gitCandidates: two, occupied: ["claude"] });
    const setup = makeDeps();
    setup.sync.resolveGlobalCollisions.mockImplementation(async (report) =>
      report.map((row) => (row.status.status === "failed" ? { ...row, status: { status: "declined" } } : row)),
    );
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });
    expect(result.current.add.open).toBe(false);
  });
});

describe("Add: one overwrite ask per batch (D12)", () => {
  it("every candidate's collisions go to the ask at once, and its answer replaces them", async () => {
    stubBackend({
      gitCandidates: [gitCandidate("alpha", "skills/alpha"), gitCandidate("beta", "skills/beta"), gitCandidate("gamma", "skills/gamma")],
      occupied: ["claude"],
      // gamma fails, so the settlement stays on screen to be read.
      failing: ["skills/gamma"],
    });
    const setup = makeDeps();
    // The ask's retry answers: Keep on alpha, Overwrite on beta.
    setup.sync.resolveGlobalCollisions.mockImplementation(async (report) =>
      report.map((row) =>
        row.status.status !== "failed"
          ? row
          : row.skill_name === "alpha"
            ? { ...row, status: { status: "declined" } }
            : { ...row, status: { status: "synced", outcome: { mode_used: "symlink", target_path: "/x", replaced: true } } },
      ),
    );
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });

    expect(installCalls()).toHaveLength(1);
    expect(setup.sync.resolveGlobalCollisions).toHaveBeenCalledTimes(1);
    const [asked, skills, policy] = setup.sync.resolveGlobalCollisions.mock.calls[0];
    expect(asked.filter((row) => row.status.status === "failed").map((row) => [row.skill_name, row.tool_key])).toEqual([
      ["alpha", "claude"],
      ["beta", "claude"],
    ]);
    expect(skills).toEqual([
      { skill_id: "id-alpha", name: "alpha", source_path: "/hub/alpha" },
      { skill_id: "id-beta", name: "beta", source_path: "/hub/beta" },
    ]);
    expect(policy).toEqual({ overwriteIfSameContent: true });
    expect(result.current.add.settlement.map((row) => [row.name, row.status, row.detail])).toEqual([
      ["alpha", "installed", `${t("add.settlement.deployed", { count: 1 })} · ${t("add.settlement.kept", { tools: "CLAUDE" })}`],
      ["beta", "installed", t("add.settlement.deployed", { count: 2 })],
      ["gamma", "failed", 'errors.rateLimited {"minutes":12}'],
    ]);
    expect(result.current.add.open).toBe(true);
  });

  it("an installed skill still settles when the overwrite retry throws (the collision to review)", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha")], occupied: ["claude"] });
    const setup = makeDeps();
    setup.sync.resolveGlobalCollisions.mockRejectedValue(new Error("retry failed"));
    const { result } = renderHook(() => useAddSkillFlow(setup.deps));
    await listGit(result);
    await act(async () => { await result.current.install(); });
    expect(setup.library.loadManagedSkills).toHaveBeenCalledTimes(1);
    expect(setup.reporter.setError).toHaveBeenCalledWith("retry failed");
    expect(setup.reporter.notify).toHaveBeenCalledWith(
      "warning",
      `${t("outcome.done.installed", { count: 1 })} · ${t("runReport.collisions", { count: 1 })}`,
      undefined,
    );
    // Something to review: the dialog stays on its settlement.
    expect(result.current.add.open).toBe(true);
    expect(result.current.add.settlement[0]).toMatchObject({ status: "installed", attention: true });
  });
});

describe("Add: Cancel (D20) over the real reporter", () => {
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
    "%s: a Cancel pressed while the listing runs discards it and stays on the source; a later listing works",
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
      // Let the mount's plan read settle.
      await act(async () => {});
      act(() => result.current.flow.open({ kind }));
      act(() => result.current.flow.add.editDraft(kind === "git" ? { url: "https://github.com/x/y" } : { path: "/some/dir" }));

      let task!: Promise<void>;
      await act(async () => {
        task = result.current.flow.add.list();
      });
      expect(result.current.flow.add.running).toBe("listing");
      expect(result.current.reporter.pending?.cancellable).toBe(true);
      act(() => result.current.reporter.cancelPending());
      expect(cancelCalls()).toHaveLength(1);
      await act(async () => {
        listing.resolve();
        await task;
      });
      expect(result.current.flow.add.step).toBe("source");
      expect(result.current.flow.add.running).toBeNull();
      expect(result.current.reporter.pending).toBeNull();

      mockInvoke.mockImplementation(base);
      await act(async () => {
        await result.current.flow.add.list();
      });
      expect(result.current.flow.add.step).toBe("candidates");
    },
  );

  it("a Cancel during the batch marks the tail cancelled, the dialog stays, and a cancelled row can be added again", async () => {
    stubBackend({
      gitCandidates: [gitCandidate("alpha", "skills/alpha"), gitCandidate("beta", "skills/beta")],
      occupied: ["claude"],
    });
    const base = mockInvoke.getMockImplementation()!;
    const batch = deferred<void>();
    let calls = 0;
    mockInvoke.mockImplementation((command, ...args) =>
      command === "installSelection" && calls++ === 0
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
    const { result } = renderHook(() => useFlowWithReporter(setup.deps));
    await act(async () => {});
    act(() => result.current.flow.open());
    act(() => result.current.flow.add.editDraft({ url: "https://github.com/x/y" }));
    await act(async () => { await result.current.flow.add.list(); });

    let task!: Promise<void>;
    await act(async () => {
      task = result.current.flow.install();
    });
    expect(result.current.flow.add.step).toBe("settlement");
    expect(result.current.flow.add.running).toBe("installing");
    expect(result.current.flow.add.settlement.map((row) => row.status)).toEqual(["queued", "queued"]);
    expect(result.current.reporter.pending?.cancellable).toBe(true);
    act(() => result.current.reporter.cancelPending());
    expect(cancelCalls()).toHaveLength(1);

    await act(async () => {
      batch.resolve();
    });
    // The ask's retry is a plain sync: Cancel is withdrawn while it runs.
    expect(setup.sync.resolveGlobalCollisions).toHaveBeenCalledTimes(1);
    expect(result.current.reporter.pending?.cancellable).toBe(false);

    await act(async () => {
      retry.resolve();
      await task;
    });
    expect(result.current.flow.add.settlement.map((row) => [row.name, row.status, row.retryable])).toEqual([
      ["alpha", "installed", false],
      ["beta", "cancelled", true],
    ]);
    expect(result.current.flow.add.open).toBe(true);
    expect(result.current.reporter.pending).toBeNull();

    // retry(ids) for the cancelled row sends it alone.
    await act(async () => {
      await result.current.flow.retry(["skills/beta"]);
    });
    expect(installCalls()).toHaveLength(2);
    expect(installCalls()[1][1]).toMatchObject({ candidates: [{ subpath: "skills/beta", name_override: null }] });
  });

  it("the batch's progress ticks mark the row being added and the ones awaiting their result", async () => {
    stubBackend({ gitCandidates: [gitCandidate("alpha", "skills/alpha"), gitCandidate("beta", "skills/beta")] });
    const base = mockInvoke.getMockImplementation()!;
    const batch = deferred<void>();
    let channel: { onmessage: (message: unknown) => void } | null = null;
    mockInvoke.mockImplementation((command, ...args) => {
      if (command !== "installSelection") return base(command, ...args);
      channel = args[1] as { onmessage: (message: unknown) => void };
      return batch.promise.then(() => base(command, ...args));
    });
    const setup = makeDeps();
    const { result } = renderHook(() => useFlowWithReporter(setup.deps));
    await act(async () => {});
    act(() => result.current.flow.open());
    act(() => result.current.flow.add.editDraft({ url: "https://github.com/x/y" }));
    await act(async () => { await result.current.flow.add.list(); });
    let task!: Promise<void>;
    await act(async () => {
      task = result.current.flow.install();
    });
    act(() => channel!.onmessage({ index: 2, total: 2, name: "beta" }));
    expect(result.current.flow.add.settlement.map((row) => row.status)).toEqual(["awaiting", "adding"]);
    expect(result.current.reporter.pending?.progress).toEqual({ done: 1, total: 2 });
    expect(result.current.reporter.actionMessage).toBe('installBatch.step {"index":2,"total":2,"name":"beta"}');
    await act(async () => {
      batch.resolve();
      await task;
    });
    expect(result.current.flow.add.open).toBe(false);
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
