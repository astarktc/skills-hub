// Tests for the shared candidate-pick module with a stub source: the
// selection defaults, close vs cancel, the one selection validation both
// the git and local flows share (duplicate names, custom name rules, taken
// names), and the one install batch per selection (ticket 04). Reporter and
// the install batch enter as mocked dependency interfaces.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as folds from "../lib/reportOutcome";
import type { InstallResultDto } from "../components/skills/types";
import {
  useCandidatePick,
  type CandidatePickDeps,
  type CandidatePickSource,
  type InstallBatchAction,
  type InstallPick,
} from "./useCandidatePick";
import type { InstallSource } from "../components/skills/types";
import { useSurfaceState } from "./useSurfaceState";
import {
  ActionExit,
  type ActionHandle,
  type RunActionOptions,
} from "./useStatusReporter";

type Cand = { name: string; subpath: string; valid: boolean };

const t = (key: string, opts?: Record<string, unknown>) =>
  opts ? `${key} ${JSON.stringify(opts)}` : key;

function cand(name: string, subpath: string, valid = true): Cand {
  return { name, subpath, valid };
}

function installed(name: string): InstallResultDto {
  return {
    skill_id: `id-${name}`,
    name,
    central_path: `/hub/${name}`,
    content_hash: null,
  };
}

function makeDeps(overrides?: { takenNames?: string[]; loading?: boolean }) {
  const taken = new Set(overrides?.takenNames ?? []);
  const setError = vi.fn();
  const showActionErrors = vi.fn();
  const setSuccessToastMessage = vi.fn();
  const formatError = vi.fn((err: unknown) =>
    err instanceof Error ? err.message : String(err),
  );
  const runAction = vi.fn(
    async <T,>(
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
        if (typeof opts.successToast === "string") {
          setSuccessToastMessage(opts.successToast);
        }
        return outcome;
      } catch (err) {
        setError(formatError(err));
        return undefined;
      }
    },
  );
  // The add flow's batch: every pick installs, deploy disabled.
  const installBatch = vi.fn<
    (source: InstallSource, picks: InstallPick[], action: InstallBatchAction) => Promise<folds.InstallSettlement[]>
  >(async (_source, picks) =>
    picks.map((pick) => ({
      status: "installed",
      name: pick.name,
      result: installed(pick.name),
      deployment: { status: "disabled" },
    })),
  );
  // The order the completion was carried out in: reload → entries → close.
  const log: string[] = [];
  const outcomeHandlers = {
    reload: vi.fn(async () => {
      log.push("reload");
    }),
    closeModal: vi.fn(() => {
      log.push("close");
    }),
  };
  const deps: Omit<CandidatePickDeps, "surface"> = {
    t,
    reporter: {
      loading: overrides?.loading ?? false,
      runAction: runAction as CandidatePickDeps["reporter"]["runAction"],
      setActionMessage: vi.fn(),
      setError,
      // Executes an Outcome in the real executor's order onto the entry spy
      // (the executor itself is useStatusReporter's own test).
      applyOutcome: vi.fn(async (
        outcome: Parameters<CandidatePickDeps["reporter"]["applyOutcome"]>[0],
        handlers: Parameters<CandidatePickDeps["reporter"]["applyOutcome"]>[1] = {},
      ) => {
        if (outcome.completion.reload) await handlers.reload?.();
        log.push("entries");
        showActionErrors(outcome.errors);
        if (outcome.completion.closeModal) handlers.closeModal?.();
        return outcome.completion;
      }),
    },
    isSkillNameTaken: (name) => taken.has(name),
    installBatch,
    outcomeHandlers,
  };
  return { deps, setError, showActionErrors, setSuccessToastMessage, installBatch, outcomeHandlers, log };
}

function makeSource(overrides?: Partial<CandidatePickSource<Cand, string>>) {
  const installSource = vi.fn<(ctx: string, picked: Cand[]) => InstallSource>(
    (ctx) => ({ kind: "local", root: ctx }),
  );
  const resetForm = vi.fn();
  const source: CandidatePickSource<Cand, string> = {
    dialogKind: "gitPick",
    customName: "",
    selectable: (c) => c.valid,
    installSource,
    resetForm,
    ...overrides,
  };
  return { source, installSource, resetForm };
}

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(cleanup);

/** The picker as the add flow composes it: its visibility is a surface layer. */
function renderPick(
  source: CandidatePickSource<Cand, string>,
  deps: Omit<CandidatePickDeps, "surface">,
) {
  return renderHook(() => {
    const surface = useSurfaceState();
    return useCandidatePick(source, { ...deps, surface });
  });
}

describe("useCandidatePick selection", () => {
  it("starts hidden and empty", () => {
    const { deps } = makeDeps();
    const { source } = makeSource();
    const { result } = renderPick(source, deps);

    expect(result.current.visible).toBe(false);
    expect(result.current.candidates).toEqual([]);
    expect(result.current.selected).toEqual({});
  });

  it("open preselects the selectable candidates and shows the picker", () => {
    const { deps } = makeDeps();
    const { source } = makeSource();
    const { result } = renderPick(source, deps);

    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("broken", "broken", false),
      ]),
    );

    expect(result.current.visible).toBe(true);
    expect(result.current.selected).toEqual({ alpha: true, broken: false });
  });

  it("without a selectable rule every candidate is preselected", () => {
    const { deps } = makeDeps();
    const { source } = makeSource({ selectable: undefined });
    const { result } = renderPick(source, deps);

    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("broken", "broken", false),
      ]),
    );

    expect(result.current.selected).toEqual({ alpha: true, broken: true });
  });

  it("toggle and toggleAll respect the selectable rule", () => {
    const { deps } = makeDeps();
    const { source } = makeSource();
    const { result } = renderPick(source, deps);
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("beta", "beta"),
        cand("broken", "broken", false),
      ]),
    );

    act(() => result.current.toggle("alpha", false));
    expect(result.current.selected).toEqual({
      alpha: false,
      beta: true,
      broken: false,
    });

    act(() => result.current.toggleAll(false));
    expect(result.current.selected).toEqual({
      alpha: false,
      beta: false,
      broken: false,
    });

    act(() => result.current.toggleAll(true));
    expect(result.current.selected).toEqual({
      alpha: true,
      beta: true,
      broken: false,
    });
  });

  it("close hides but keeps the candidates; cancel discards them", () => {
    const { deps } = makeDeps();
    const { source } = makeSource();
    const { result } = renderPick(source, deps);
    act(() => result.current.open("/base", [cand("alpha", "alpha")]));

    act(() => result.current.close());
    expect(result.current.visible).toBe(false);
    expect(result.current.candidates).toHaveLength(1);
    expect(result.current.selected).toEqual({ alpha: true });

    act(() => result.current.open("/base", [cand("alpha", "alpha")]));
    act(() => result.current.cancel());
    expect(result.current.visible).toBe(false);
    expect(result.current.candidates).toEqual([]);
    expect(result.current.selected).toEqual({});
  });

  it("close and cancel are no-ops while an action is running", () => {
    const { deps } = makeDeps({ loading: true });
    const { source } = makeSource();
    const { result } = renderPick(source, deps);
    act(() => result.current.open("/base", [cand("alpha", "alpha")]));

    act(() => result.current.close());
    expect(result.current.visible).toBe(true);
    act(() => result.current.cancel());
    expect(result.current.visible).toBe(true);
    expect(result.current.candidates).toHaveLength(1);
  });
});

describe("useCandidatePick validation", () => {
  it("rejects an empty selection", async () => {
    const { deps, setError } = makeDeps();
    const { source } = makeSource();
    const { result } = renderPick(source, deps);
    act(() => result.current.open("/base", [cand("alpha", "alpha")]));
    act(() => result.current.toggleAll(false));

    await act(() => result.current.install());

    expect(setError).toHaveBeenCalledWith("errors.selectAtLeastOneSkill");
    expect(deps.installBatch).not.toHaveBeenCalled();
  });

  it("rejects a custom name for a multi-selection", async () => {
    const { deps, setError } = makeDeps();
    const { source } = makeSource({ customName: "mine" });
    const { result } = renderPick(source, deps);
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("beta", "beta"),
      ]),
    );

    await act(() => result.current.install());

    expect(setError).toHaveBeenCalledWith("errors.multiSelectNoCustomName");
    expect(deps.installBatch).not.toHaveBeenCalled();
  });

  it("rejects duplicate names within the selection", async () => {
    const { deps, setError } = makeDeps();
    const { source } = makeSource();
    const { result } = renderPick(source, deps);
    act(() =>
      result.current.open("/base", [
        cand("alpha", "one/alpha"),
        cand("alpha", "two/alpha"),
      ]),
    );

    await act(() => result.current.install());

    expect(setError).toHaveBeenCalledWith(
      'errors.duplicateSelectedSkills {"name":"alpha"}',
    );
    expect(deps.installBatch).not.toHaveBeenCalled();
  });

  it("rejects a taken candidate name in a multi-selection", async () => {
    const { deps, setError } = makeDeps({ takenNames: ["beta"] });
    const { source } = makeSource();
    const { result } = renderPick(source, deps);
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("beta", "beta"),
      ]),
    );

    await act(() => result.current.install());

    expect(setError).toHaveBeenCalledWith(
      'errors.skillAlreadyExists {"name":"beta"}',
    );
    expect(deps.installBatch).not.toHaveBeenCalled();
  });

  it("checks the custom name (not the candidate name) for a single selection", async () => {
    const { deps, setError } = makeDeps({ takenNames: ["mine"] });
    const { source } = makeSource({ customName: " mine " });
    const { result } = renderPick(source, deps);
    act(() => result.current.open("/base", [cand("alpha", "alpha")]));

    await act(() => result.current.install());

    expect(setError).toHaveBeenCalledWith(
      'errors.skillAlreadyExists {"name":"mine"}',
    );
    expect(deps.installBatch).not.toHaveBeenCalled();
  });
});

describe("useCandidatePick batch install", () => {
  it("installs the selection as one batch from its context, resets and finishes", async () => {
    const { deps, installBatch, outcomeHandlers, showActionErrors, log } = makeDeps();
    const { source, installSource, resetForm } = makeSource({
      customName: " custom ",
    });
    const { result } = renderPick(source, deps);
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("broken", "broken", false),
      ]),
    );

    await act(() => result.current.install());

    expect(installSource).toHaveBeenCalledWith("/base", [cand("alpha", "alpha")]);
    expect(installBatch).toHaveBeenCalledTimes(1);
    expect(installBatch).toHaveBeenCalledWith(
      { kind: "local", root: "/base" },
      [{ subpath: "alpha", name: "alpha", nameOverride: "custom" }],
      expect.objectContaining({ cancelRequested: expect.any(Function) }),
    );
    expect(resetForm).toHaveBeenCalled();
    // One executed completion: reload, then the entries, then the close.
    expect(log).toEqual(["reload", "entries", "close"]);
    expect(outcomeHandlers.closeModal).toHaveBeenCalledTimes(1);
    expect(showActionErrors).toHaveBeenCalledWith([]);
    expect(deps.reporter.applyOutcome).toHaveBeenCalledWith(
      expect.objectContaining({
        toast: expect.objectContaining({
          kind: "success",
          message: t("outcome.done.installed", { count: 1 }),
        }),
      }),
      expect.anything(),
    );
    expect(result.current.visible).toBe(false);
    expect(result.current.candidates).toEqual([]);
  });

  it("passes no name for a multi-selection, every pick in one call", async () => {
    const { deps, installBatch } = makeDeps();
    const { source } = makeSource();
    const { result } = renderPick(source, deps);
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("beta", "beta"),
      ]),
    );

    await act(() => result.current.install());

    expect(installBatch).toHaveBeenCalledTimes(1);
    expect(installBatch.mock.calls[0][1]).toEqual([
      { subpath: "alpha", name: "alpha", nameOverride: null },
      { subpath: "beta", name: "beta", nameOverride: null },
    ]);
  });

  it("folds per-candidate failures and a cancelled tail as report data, never an action failure", async () => {
    const { deps, installBatch, setError, showActionErrors } = makeDeps();
    const fold = vi.spyOn(folds, "installOutcome");
    installBatch.mockImplementation(async () => [
      { status: "installed", name: "alpha", result: installed("alpha"), deployment: { status: "no-targets" } },
      { status: "failed", name: "beta", error: { code: "SUBPATH_MISSING", subpath: "beta" } },
      { status: "cancelled", name: "gamma" },
    ]);
    const { source } = makeSource();
    const { result } = renderPick(source, deps);
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("beta", "beta"),
        cand("gamma", "gamma"),
      ]),
    );

    await act(() => result.current.install());

    expect(fold.mock.calls[0][0].map((item) => item.status)).toEqual(["installed", "failed", "cancelled"]);
    expect(showActionErrors).toHaveBeenCalledWith(fold.mock.results[0].value.errors);
    // Per-candidate failures are report data, not an action failure.
    expect(setError).not.toHaveBeenCalled();
    expect(result.current.visible).toBe(false);
  });
});

afterEach(() => vi.restoreAllMocks());
it.each([false, true])("picker hands completion=%s to the one applyOutcome, never interpreting it", async (complete) => {
  const { deps, outcomeHandlers, log } = makeDeps();
  const { source, resetForm } = makeSource();
  const outcome: folds.Outcome = { toast: { kind: "warning", tone: "failure", message: "fold" }, errors: [{ title: "error", message: "detail" }], warnings: [], notes: [], completion: { reload: complete, closeModal: complete, conflict: false } };
  const fold = vi.spyOn(folds, "installOutcome").mockReturnValue(outcome);
  const { result } = renderPick(source, deps);
  act(() => result.current.open("/base", [cand("alpha", "alpha")]));
  await act(async () => { await result.current.install(); });
  expect(fold).toHaveBeenCalledTimes(1);
  expect(deps.reporter.applyOutcome).toHaveBeenCalledTimes(1);
  expect(deps.reporter.applyOutcome).toHaveBeenCalledWith(outcome, expect.objectContaining({ reload: outcomeHandlers.reload }));
  expect(log).toEqual(complete ? ["reload", "entries", "close"] : ["entries"]);
  expect(resetForm).toHaveBeenCalledTimes(complete ? 1 : 0);
  expect(result.current.visible).toBe(!complete);
});
