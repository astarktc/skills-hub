// Tests at the redeploy seam (round 20 #108): the launch check offers the
// move once (and not again for work the operator declined), accepting runs it
// under the lifecycle with progress, the one overwrite ask carries the
// folders in a move's way and the edited copies and its answers go back in a
// second run, and the "Move N Deployments" line is the preview's count. The
// backend is mocked at the invokeTauri module seam.

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  RedeployPreview,
  RedeployProgress,
  RedeployReport,
  RedeployResultDto,
} from "../components/skills/types";
import type { OverwriteRow } from "../lib/overwriteAsk";
import { redeployOfferDeclinedPreference } from "../lib/preferences";
import { useStatusReporter } from "./useStatusReporter";
import type { AskAnswer, RowChoices, SurfaceState } from "./useSurfaceState";

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

vi.mock("../lib/platform", () => ({
  createProgressChannel: async (onMessage: (message: unknown) => void) => ({ onmessage: onMessage }),
}));

import { invokeTauri, type CommandName } from "../lib/tauri";
import { redeployOfferSignature, redeployPendingCount, useRedeploy } from "./useRedeploy";

const mockInvoke = vi.mocked(
  invokeTauri as unknown as (command: CommandName, ...args: unknown[]) => Promise<unknown>,
);

const t = (key: string) => key;

const PREVIEW: RedeployPreview = {
  global: { correction: 2, migration: 1 },
  journal: 0,
  harnesses: ["codex", "kilo_code"],
};
const NONE: RedeployPreview = { global: { correction: 0, migration: 0 }, journal: 0, harnesses: [] };

const EMPTY_REPORT: RedeployReport = {
  moves: [],
  journal: [],
  left_behind: [],
  edited_copies: [],
  lost_disables: [],
};

const move = (skill: string, status: RedeployReport["moves"][number]["status"]) => ({
  skill_id: skill,
  skill_name: skill,
  folder_key: "agents_skills",
  path: `/home/op/.agents/skills/${skill}`,
  cause: "correction" as const,
  rows: [{ id: `row-${skill}`, tool: "codex", from: `/home/op/.codex/skills/${skill}` }],
  status,
});

/** A first run that meets a folder in alpha's way and beta's edited copy. */
const ASKING: RedeployResultDto = {
  report: {
    ...EMPTY_REPORT,
    moves: [
      move("alpha", {
        status: "failed",
        error: { code: "TARGET_EXISTS", path: "/home/op/.agents/skills/alpha", modifiedAt: 1, sameContent: false },
      }),
      move("beta", { status: "left_old", edited: true }),
    ],
    edited_copies: [
      {
        skill_id: "beta",
        skill_name: "beta",
        tool: "codex",
        path: "/home/op/.codex/skills/beta",
        modified_at: 2,
        readers: ["codex", "cursor"],
      },
    ],
  },
  skills: [],
  preview: { ...NONE, global: { correction: 1, migration: 0 }, journal: 1 },
};

const ANSWERED: RedeployResultDto = {
  report: {
    ...EMPTY_REPORT,
    moves: [move("alpha", { status: "moved" })],
    journal: [
      { skill_id: "beta", skill_name: "beta", tool: "codex", path: "/home/op/.codex/skills/beta", status: "finished" },
    ],
  },
  skills: [],
  preview: NONE,
};

type Backend = {
  check: { finished: RedeployReport | null; preview: RedeployPreview };
  runs: RedeployResultDto[];
};

function stubBackend(backend: Backend) {
  let progress: ((tick: RedeployProgress) => void) | null = null;
  mockInvoke.mockImplementation((command, ...args) => {
    if (command === "getRedeployLaunchCheck") return Promise.resolve(backend.check);
    if (command === "redeployDeployments") {
      const [, channel] = args as [unknown, { onmessage: (tick: RedeployProgress) => void }];
      progress = channel.onmessage;
      progress({ done: 0, total: 2, name: "alpha" });
      const next = backend.runs.shift();
      return next ? Promise.resolve(next) : Promise.reject(new Error("no run left"));
    }
    return Promise.resolve(undefined);
  });
}

function render(answer: (rows: OverwriteRow[]) => AskAnswer | RowChoices = () => "dismissed") {
  const offerDialog = vi.fn();
  const dismissDialog = vi.fn();
  const ask = vi.fn(async (request: { kind: "overwrite"; rows: OverwriteRow[] }) => answer(request.rows));
  const replaceCatalog = vi.fn();
  const loadManagedSkills = vi.fn(async () => {});
  const rendered = renderHook(() => {
    const reporter = useStatusReporter(t);
    const redeploy = useRedeploy({
      t,
      reporter,
      surface: {
        ask: ask as unknown as SurfaceState["ask"],
        offerDialog,
        dismissDialog,
      },
      library: { replaceCatalog, loadManagedSkills },
      toolLabelById: { codex: "Codex", agents_skills: ".agents/skills" },
    });
    return { reporter, redeploy };
  });
  return { ...rendered, offerDialog, dismissDialog, ask, replaceCatalog, loadManagedSkills };
}

const calls = (command: string) => mockInvoke.mock.calls.filter(([name]) => name === command);

// jsdom here provides no localStorage: the declined offer's memory is
// stubbed in memory (as `persistedPreference.test.ts` does).
function installMemoryStorage() {
  const memory = new Map<string, string>();
  Object.defineProperty(window, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => memory.get(key) ?? null,
      setItem: (key: string, value: string) => void memory.set(key, value),
      removeItem: (key: string) => void memory.delete(key),
      clear: () => memory.clear(),
    },
  });
}

beforeEach(() => {
  mockInvoke.mockReset();
  installMemoryStorage();
});

describe("useRedeploy launch check", () => {
  it("offers the move once when the preview has work, and the line counts it", async () => {
    stubBackend({ check: { finished: null, preview: PREVIEW }, runs: [] });
    const { result, offerDialog } = render();

    await waitFor(() => expect(result.current.redeploy.pendingCount).toBe(3));
    expect(offerDialog).toHaveBeenCalledTimes(1);
    expect(offerDialog).toHaveBeenCalledWith({ kind: "redeployOffer", preview: PREVIEW });
    expect(calls("redeployDeployments")).toHaveLength(0);
  });

  it("offers nothing when nothing is pending", async () => {
    stubBackend({ check: { finished: null, preview: NONE }, runs: [] });
    const { result, offerDialog } = render();

    await waitFor(() => expect(calls("getRedeployLaunchCheck")).toHaveLength(1));
    await waitFor(() => expect(result.current.redeploy.preview).toEqual(NONE));
    expect(offerDialog).not.toHaveBeenCalled();
    expect(result.current.redeploy.pendingCount).toBe(0);
  });

  it("does not offer again work the operator declined, but keeps the line", async () => {
    redeployOfferDeclinedPreference.write(redeployOfferSignature(PREVIEW));
    stubBackend({ check: { finished: null, preview: PREVIEW }, runs: [] });
    const { result, offerDialog } = render();

    await waitFor(() => expect(result.current.redeploy.pendingCount).toBe(3));
    expect(offerDialog).not.toHaveBeenCalled();
  });

  it("offers new work although an earlier offer was declined", async () => {
    redeployOfferDeclinedPreference.write(redeployOfferSignature({ ...PREVIEW, harnesses: ["codex"] }));
    stubBackend({ check: { finished: null, preview: PREVIEW }, runs: [] });
    const { offerDialog } = render();

    await waitFor(() => expect(offerDialog).toHaveBeenCalledTimes(1));
  });

  it("records the journal the launch check finished as a background run", async () => {
    const finished: RedeployReport = {
      ...EMPTY_REPORT,
      journal: [{ skill_id: "a", skill_name: "a", tool: "codex", path: "/x/a", status: "finished" }],
    };
    stubBackend({ check: { finished, preview: NONE }, runs: [] });
    const { result } = render();

    await waitFor(() =>
      expect(result.current.reporter.notifications.some((n) => n.runLabel === "redeploy.resumedLabel")).toBe(true),
    );
  });

  it("declining remembers the declined work and closes the offer", async () => {
    stubBackend({ check: { finished: null, preview: PREVIEW }, runs: [] });
    const { result, dismissDialog } = render();
    await waitFor(() => expect(result.current.redeploy.pendingCount).toBe(3));

    act(() => result.current.redeploy.declineOffer());

    expect(dismissDialog).toHaveBeenCalledWith("redeployOffer");
    expect(redeployOfferDeclinedPreference.read()).toBe(redeployOfferSignature(PREVIEW));
    expect(calls("redeployDeployments")).toHaveLength(0);
    expect(result.current.redeploy.pendingCount).toBe(3);
  });
});

describe("useRedeploy run", () => {
  it("accepting runs the move under the lifecycle, asks once, retries with the answers", async () => {
    stubBackend({ check: { finished: null, preview: PREVIEW }, runs: [ASKING, ANSWERED] });
    const { result, ask, dismissDialog, replaceCatalog } = render((rows) => {
      // Overwrite alpha's folder; delete beta's edited copy.
      expect(rows.map((row) => [row.skillId, row.question])).toEqual([
        ["alpha", "occupied"],
        ["beta", "editedCopy"],
      ]);
      return { choices: ["overwrite", "overwrite"] };
    });
    await waitFor(() => expect(result.current.redeploy.pendingCount).toBe(3));

    await act(async () => {
      await result.current.redeploy.acceptOffer();
    });

    expect(dismissDialog).toHaveBeenCalledWith("redeployOffer");
    expect(ask).toHaveBeenCalledTimes(1);
    const runs = calls("redeployDeployments");
    expect(runs).toHaveLength(2);
    expect(runs[0][1]).toEqual([]);
    expect(runs[1][1]).toEqual([
      {
        question: "occupied",
        skill_id: "alpha",
        folder_key: "agents_skills",
        path: "/home/op/.agents/skills/alpha",
        decision: "overwrite",
      },
      { question: "edited_copy", skill_id: "beta", tool: "codex", path: "/home/op/.codex/skills/beta", decision: "delete" },
    ]);
    expect(replaceCatalog).toHaveBeenCalledWith(ANSWERED.skills);
    expect(result.current.redeploy.pendingCount).toBe(0);
    expect(result.current.reporter.pending).toBeNull();
    // One run recorded, from the merged report.
    const run = result.current.reporter.notifications.filter((n) => n.runLabel === "redeploy.label");
    expect(run.length).toBeGreaterThan(0);
  });

  it("a dismissed ask sends nothing; what it asked stays pending", async () => {
    stubBackend({ check: { finished: null, preview: NONE }, runs: [ASKING] });
    const { result, ask } = render(() => "dismissed");
    await waitFor(() => expect(calls("getRedeployLaunchCheck")).toHaveLength(1));

    await act(async () => {
      await result.current.redeploy.run();
    });

    expect(ask).toHaveBeenCalledTimes(1);
    expect(calls("redeployDeployments")).toHaveLength(1);
    expect(result.current.redeploy.pendingCount).toBe(2);
  });

  it("runs as the one pending mutation with the redeploy's label and no Cancel", async () => {
    let release: (value: RedeployResultDto) => void = () => {};
    mockInvoke.mockImplementation((command, ...args) => {
      if (command === "getRedeployLaunchCheck") return Promise.resolve({ finished: null, preview: PREVIEW });
      if (command === "redeployDeployments") {
        const [, channel] = args as [unknown, { onmessage: (tick: RedeployProgress) => void }];
        channel.onmessage({ done: 1, total: 3, name: "beta" });
        return new Promise((resolve) => (release = resolve));
      }
      return Promise.resolve(undefined);
    });
    const { result } = render();
    await waitFor(() => expect(result.current.redeploy.pendingCount).toBe(3));

    let done: Promise<void> = Promise.resolve();
    act(() => {
      done = result.current.redeploy.run();
    });
    await waitFor(() => expect(calls("redeployDeployments")).toHaveLength(1));

    expect(result.current.reporter.pending).toMatchObject({
      label: "redeploy.label",
      activity: "redeploy.activity",
      cancellable: false,
    });
    expect(result.current.reporter.pending?.progress).toEqual({ done: 1, total: 3 });
    expect(result.current.reporter.actionMessage).toBe("redeploy.step");

    await act(async () => {
      release({ report: EMPTY_REPORT, skills: [], preview: NONE });
      await done;
    });
    expect(result.current.reporter.pending).toBeNull();
  });

  it("a run that throws re-reads the catalog and the preview", async () => {
    stubBackend({ check: { finished: null, preview: PREVIEW }, runs: [] });
    const { result, loadManagedSkills } = render();
    await waitFor(() => expect(result.current.redeploy.pendingCount).toBe(3));

    await act(async () => {
      await result.current.redeploy.run();
    });

    expect(loadManagedSkills).toHaveBeenCalled();
    expect(calls("getRedeployLaunchCheck").length).toBeGreaterThanOrEqual(2);
  });
});

describe("redeployPendingCount", () => {
  it("counts every move and every journal entry", () => {
    expect(redeployPendingCount(null)).toBe(0);
    expect(redeployPendingCount({ ...PREVIEW, journal: 2 })).toBe(5);
  });
});
