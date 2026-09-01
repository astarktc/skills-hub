// Tests for the shared candidate-pick module with a stub source: the
// selection defaults, close vs cancel, the one selection validation both
// the git and local flows share (duplicate names, custom name rules, taken
// names), and batch-install error collection. Reporter and deploy tail
// enter as mocked dependency interfaces.

import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InstallResultDto } from "../components/skills/types";
import {
  useCandidatePick,
  type CandidatePickDeps,
  type CandidatePickSource,
} from "./useCandidatePick";
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
  const deploy = vi.fn<
    (created: InstallResultDto) => Promise<{ title: string; message: string }[]>
  >(async () => []);
  const afterBatch = vi.fn(async () => {});
  const deps: CandidatePickDeps = {
    t,
    reporter: {
      loading: overrides?.loading ?? false,
      runAction: runAction as CandidatePickDeps["reporter"]["runAction"],
      setActionMessage: vi.fn(),
      setError,
      formatError,
      showActionErrors,
    },
    isSkillNameTaken: (name) => taken.has(name),
    deploy,
    afterBatch,
  };
  return { deps, setError, showActionErrors, setSuccessToastMessage, deploy, afterBatch };
}

function makeSource(overrides?: Partial<CandidatePickSource<Cand, string>>) {
  const installOne = vi.fn<
    (ctx: string, c: Cand, name: string | undefined) => Promise<InstallResultDto>
  >(async (_ctx, c) => installed(c.name));
  const resetForm = vi.fn();
  const source: CandidatePickSource<Cand, string> = {
    customName: "",
    selectable: (c) => c.valid,
    installOne,
    resetForm,
    ...overrides,
  };
  return { source, installOne, resetForm };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("useCandidatePick selection", () => {
  it("starts hidden and empty", () => {
    const { deps } = makeDeps();
    const { source } = makeSource();
    const { result } = renderHook(() => useCandidatePick(source, deps));

    expect(result.current.visible).toBe(false);
    expect(result.current.candidates).toEqual([]);
    expect(result.current.selected).toEqual({});
  });

  it("open preselects the selectable candidates and shows the picker", () => {
    const { deps } = makeDeps();
    const { source } = makeSource();
    const { result } = renderHook(() => useCandidatePick(source, deps));

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
    const { result } = renderHook(() => useCandidatePick(source, deps));

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
    const { result } = renderHook(() => useCandidatePick(source, deps));
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
    const { result } = renderHook(() => useCandidatePick(source, deps));
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
    const { result } = renderHook(() => useCandidatePick(source, deps));
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
    const { source, installOne } = makeSource();
    const { result } = renderHook(() => useCandidatePick(source, deps));
    act(() => result.current.open("/base", [cand("alpha", "alpha")]));
    act(() => result.current.toggleAll(false));

    await act(() => result.current.install());

    expect(setError).toHaveBeenCalledWith("errors.selectAtLeastOneSkill");
    expect(installOne).not.toHaveBeenCalled();
  });

  it("rejects a custom name for a multi-selection", async () => {
    const { deps, setError } = makeDeps();
    const { source, installOne } = makeSource({ customName: "mine" });
    const { result } = renderHook(() => useCandidatePick(source, deps));
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("beta", "beta"),
      ]),
    );

    await act(() => result.current.install());

    expect(setError).toHaveBeenCalledWith("errors.multiSelectNoCustomName");
    expect(installOne).not.toHaveBeenCalled();
  });

  it("rejects duplicate names within the selection", async () => {
    const { deps, setError } = makeDeps();
    const { source, installOne } = makeSource();
    const { result } = renderHook(() => useCandidatePick(source, deps));
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
    expect(installOne).not.toHaveBeenCalled();
  });

  it("rejects a taken candidate name in a multi-selection", async () => {
    const { deps, setError } = makeDeps({ takenNames: ["beta"] });
    const { source, installOne } = makeSource();
    const { result } = renderHook(() => useCandidatePick(source, deps));
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
    expect(installOne).not.toHaveBeenCalled();
  });

  it("checks the custom name (not the candidate name) for a single selection", async () => {
    const { deps, setError } = makeDeps({ takenNames: ["mine"] });
    const { source, installOne } = makeSource({ customName: " mine " });
    const { result } = renderHook(() => useCandidatePick(source, deps));
    act(() => result.current.open("/base", [cand("alpha", "alpha")]));

    await act(() => result.current.install());

    expect(setError).toHaveBeenCalledWith(
      'errors.skillAlreadyExists {"name":"mine"}',
    );
    expect(installOne).not.toHaveBeenCalled();
  });
});

describe("useCandidatePick batch install", () => {
  it("installs each selected candidate with its context, deploys, resets and finishes", async () => {
    const { deps, deploy, afterBatch, setSuccessToastMessage, showActionErrors } =
      makeDeps();
    const { source, installOne, resetForm } = makeSource({
      customName: " custom ",
    });
    const { result } = renderHook(() => useCandidatePick(source, deps));
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("broken", "broken", false),
      ]),
    );

    await act(() => result.current.install());

    expect(installOne).toHaveBeenCalledTimes(1);
    expect(installOne).toHaveBeenCalledWith(
      "/base",
      cand("alpha", "alpha"),
      "custom",
    );
    expect(deploy).toHaveBeenCalledWith(installed("alpha"));
    expect(resetForm).toHaveBeenCalled();
    expect(afterBatch).toHaveBeenCalled();
    expect(showActionErrors).not.toHaveBeenCalled();
    expect(setSuccessToastMessage).toHaveBeenCalledWith(
      "status.selectedSkillsInstalled",
    );
    expect(result.current.visible).toBe(false);
    expect(result.current.candidates).toEqual([]);
  });

  it("passes no name for a multi-selection", async () => {
    const { deps } = makeDeps();
    const { source, installOne } = makeSource();
    const { result } = renderHook(() => useCandidatePick(source, deps));
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("beta", "beta"),
      ]),
    );

    await act(() => result.current.install());

    expect(installOne).toHaveBeenCalledTimes(2);
    expect(installOne).toHaveBeenNthCalledWith(
      2,
      "/base",
      cand("beta", "beta"),
      undefined,
    );
  });

  it("collects per-candidate install and deploy failures without aborting the batch", async () => {
    const { deps, deploy, showActionErrors, setError } = makeDeps();
    const { source, installOne } = makeSource();
    installOne.mockImplementation(async (_ctx, c) => {
      if (c.name === "beta") throw new Error("clone failed");
      return installed(c.name);
    });
    deploy.mockImplementation(async (created) =>
      created.name === "gamma"
        ? [{ title: "unsynced gamma", message: "no targets" }]
        : [],
    );
    const { result } = renderHook(() => useCandidatePick(source, deps));
    act(() =>
      result.current.open("/base", [
        cand("alpha", "alpha"),
        cand("beta", "beta"),
        cand("gamma", "gamma"),
      ]),
    );

    await act(() => result.current.install());

    expect(installOne).toHaveBeenCalledTimes(3);
    expect(showActionErrors).toHaveBeenCalledWith([
      {
        title: 'errors.importFailedTitle {"name":"beta"}',
        message: "clone failed",
      },
      { title: "unsynced gamma", message: "no targets" },
    ]);
    // Per-candidate failures are report data, not an action failure.
    expect(setError).not.toHaveBeenCalled();
    expect(result.current.visible).toBe(false);
  });
});
