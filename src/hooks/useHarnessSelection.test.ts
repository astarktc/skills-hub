// D48 (round-19 ticket 20): turning a harness on with auto-deploy on asks
// once and deploys exactly the stated skills through the door; off, declined,
// dismissed or auto-deploy off deploys nothing. Fix pass (audit F2 / F3): the
// ask is for an installed harness with at least one arrival, and only for the
// harness's current intent — re-checked before asking and at dispatch.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ManagedSkill, SkillTargetDto } from "../components/skills/types";
import { useHarnessSelection, type HarnessSelectionDeps } from "./useHarnessSelection";
import type { AskAnswer } from "./useSurfaceState";
import type { HarnessSaveResult } from "./useSyncOrchestration";

afterEach(cleanup);

function skill(id: string, extra: Partial<ManagedSkill> = {}): ManagedSkill {
  return {
    id,
    name: id,
    description: null,
    source_type: "git",
    source_ref: null,
    imported_from_tool: null,
    central_path: `/hub/${id}`,
    created_at: 0,
    updated_at: 0,
    last_sync_at: null,
    status: "active",
    invocation_mode: "user-and-model",
    invocation_override: null,
    targets: [],
    disabled_at: null,
    refreshable: true,
    unlocatable: null,
    detachable: false,
    health: null,
    projects: [],
    ...extra,
  } as ManagedSkill;
}

function target(tool: string, status: string): SkillTargetDto {
  return { tool, mode: "symlink", status, target_path: `/t/${tool}`, synced_at: 1, last_error: null } as unknown as SkillTargetDto;
}

const SKILLS = [skill("a"), skill("b"), skill("off", { disabled_at: 1 })];

/** A deferred promise: the test settles each save when it chooses. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

type SetupOptions = {
  autoSync?: boolean;
  installed?: string[];
  skills?: ManagedSkill[];
  answer?: AskAnswer;
  /** The selection as saved after each write (the sync world's echo). */
  saved?: string[];
  result?: HarnessSaveResult;
};

function setup({
  autoSync = true,
  installed = ["cursor", "claude"],
  skills = SKILLS,
  answer = "confirmed",
  saved = ["cursor"],
  result = "saved",
}: SetupOptions = {}) {
  const savedSelection = { current: saved as readonly string[] | null };
  const setHarnessSelected = vi.fn(async (): Promise<HarnessSaveResult> => result);
  const readSavedSelection = vi.fn(() => savedSelection.current);
  const ask = vi.fn(async () => answer);
  const layer = vi.fn((): unknown => null);
  const dismissDialog = vi.fn();
  const run = vi.fn(async () => undefined);
  const whenIdle = vi.fn(async () => true);
  const initial = { autoSync, installed, skills };
  const { result: hook, rerender } = renderHook(
    (props: { autoSync: boolean; installed: string[]; skills: ManagedSkill[] }) =>
      useHarnessSelection({
        sync: { setHarnessSelected, autoSyncEnabled: props.autoSync, readSavedSelection },
        installed: props.installed,
        skills: props.skills,
        labelOf: (id) => id.toUpperCase(),
        // SAFETY: the mocks answer the narrowed signatures the hook calls.
        surface: { ask: ask as never, layer: layer as never, dismissDialog } as HarnessSelectionDeps["surface"],
        actions: { run: run as never },
        reporter: { whenIdle },
      }),
    { initialProps: initial },
  );
  return {
    setSelected: (toolId: string, on: boolean) => hook.current.select(toolId, on),
    rerender: (next: Partial<typeof initial>) => rerender({ ...initial, ...next }),
    savedSelection,
    setHarnessSelected,
    ask,
    layer,
    dismissDialog,
    run,
    whenIdle,
  };
}

describe("useHarnessSelection", () => {
  it("turning a detected harness on asks once with the library's count, and yes deploys exactly those skills", async () => {
    const { setSelected, setHarnessSelected, ask, run, whenIdle } = setup();
    await act(() => setSelected("cursor", true));

    expect(setHarnessSelected).toHaveBeenCalledWith("cursor", true);
    // The deploy waits out the import-plan rescan the save started.
    expect(whenIdle).toHaveBeenCalledTimes(1);
    expect(ask).toHaveBeenCalledWith({ kind: "deployHarness", toolKey: "cursor", toolLabel: "CURSOR", count: 2 });
    expect(run).toHaveBeenCalledTimes(1);
    const [verb, skills, opts] = run.mock.calls[0] as unknown as [string, ManagedSkill[], unknown];
    expect(verb).toBe("syncToTargets");
    expect(skills.map((s) => s.id)).toEqual(["a", "b"]);
    expect(opts).toEqual({ tools: ["cursor"], confirmed: true });
  });

  it.each(["declined", "dismissed"] as const)("%s: nothing is deployed", async (answer) => {
    const { setSelected, run } = setup({ answer });
    await act(() => setSelected("cursor", true));
    expect(run).not.toHaveBeenCalled();
  });

  it("with auto-deploy off, turning a harness on only saves the selection", async () => {
    const { setSelected, ask, run } = setup({ autoSync: false });
    await act(() => setSelected("cursor", true));
    expect(ask).not.toHaveBeenCalled();
    expect(run).not.toHaveBeenCalled();
  });

  it("turning a harness off saves and asks nothing", async () => {
    const { setSelected, setHarnessSelected, ask } = setup({ saved: [] });
    await act(() => setSelected("cursor", false));
    expect(setHarnessSelected).toHaveBeenCalledWith("cursor", false);
    expect(ask).not.toHaveBeenCalled();
  });

  it.each(["failed", "refused"] as const)("a %s save asks nothing", async (result) => {
    const { setSelected, ask } = setup({ result });
    await act(() => setSelected("cursor", true));
    expect(ask).not.toHaveBeenCalled();
  });
});

describe("useHarnessSelection eligibility (audit F2)", () => {
  it("an undetected harness is saved to the selection with no ask", async () => {
    const { setSelected, setHarnessSelected, ask, run } = setup({ installed: ["claude"], saved: ["augment"] });
    await act(() => setSelected("augment", true));
    expect(setHarnessSelected).toHaveBeenCalledWith("augment", true);
    expect(ask).not.toHaveBeenCalled();
    expect(run).not.toHaveBeenCalled();
  });

  it("zero eligible skills: no ask", async () => {
    const { setSelected, ask } = setup({ skills: [] });
    await act(() => setSelected("cursor", true));
    expect(ask).not.toHaveBeenCalled();
  });

  it("disabled and unlocatable skills are not counted", async () => {
    const skills = [
      skill("a"),
      skill("off", { disabled_at: 1 }),
      skill("lost", { unlocatable: { reason: "missing" } as never }),
      skill("pair-off", { targets: [target("cursor", "disabled")] }),
    ];
    const { setSelected, ask, run } = setup({ skills });
    await act(() => setSelected("cursor", true));
    expect(ask).toHaveBeenCalledWith(expect.objectContaining({ count: 1 }));
    const [, deployed] = run.mock.calls[0] as unknown as [string, ManagedSkill[]];
    expect(deployed.map((s) => s.id)).toEqual(["a"]);
  });

  it("skills already deployed there are not counted; only none left → no ask", async () => {
    const deployed = [skill("a", { targets: [target("cursor", "synced")] }), skill("b", { targets: [target("cursor", "stale")] })];
    const { setSelected, ask } = setup({ skills: deployed });
    await act(() => setSelected("cursor", true));
    expect(ask).not.toHaveBeenCalled();
  });

  it("a detected harness with arrivals asks with the right N", async () => {
    const skills = [skill("a"), skill("b", { targets: [target("cursor", "synced")] }), skill("c"), skill("d", { targets: [target("claude", "synced")] })];
    const { setSelected, ask } = setup({ skills });
    await act(() => setSelected("cursor", true));
    expect(ask).toHaveBeenCalledWith({ kind: "deployHarness", toolKey: "cursor", toolLabel: "CURSOR", count: 3 });
  });

  it("a harness that stops being detected before dispatch deploys nothing", async () => {
    const { setSelected, rerender, run, whenIdle } = setup();
    const idle = deferred<boolean>();
    whenIdle.mockImplementationOnce(() => idle.promise);
    let done: Promise<void> = Promise.resolve();
    await act(async () => {
      done = setSelected("cursor", true);
    });
    rerender({ installed: ["claude"] });
    await act(async () => {
      idle.resolve(true);
      await done;
    });
    expect(run).not.toHaveBeenCalled();
  });
});

describe("useHarnessSelection superseded intent (audit F3)", () => {
  it("on → off before the first save settles: no ask, no deploy", async () => {
    const { setSelected, setHarnessSelected, savedSelection, ask, run } = setup();
    const first = deferred<HarnessSaveResult>();
    const second = deferred<HarnessSaveResult>();
    setHarnessSelected.mockImplementationOnce(() => first.promise).mockImplementationOnce(() => second.promise);
    let on: Promise<void> = Promise.resolve();
    let off: Promise<void> = Promise.resolve();
    await act(async () => {
      on = setSelected("cursor", true);
      off = setSelected("cursor", false);
    });
    await act(async () => {
      first.resolve("saved");
      await on;
      savedSelection.current = [];
      second.resolve("saved");
      await off;
    });
    expect(ask).not.toHaveBeenCalled();
    expect(run).not.toHaveBeenCalled();
  });

  it("on → off → on: two writes superseded, one ask, one deploy", async () => {
    const { setSelected, setHarnessSelected, ask, run } = setup();
    const writes = [deferred<HarnessSaveResult>(), deferred<HarnessSaveResult>(), deferred<HarnessSaveResult>()];
    for (const write of writes) setHarnessSelected.mockImplementationOnce(() => write.promise);
    const gestures: Promise<void>[] = [];
    await act(async () => {
      gestures.push(setSelected("cursor", true), setSelected("cursor", false), setSelected("cursor", true));
    });
    await act(async () => {
      for (const write of writes) write.resolve("saved");
      await Promise.all(gestures);
    });
    // Three writes reached the sync world (this hook queues nothing; the
    // lifecycle refuses overlapping ones in the app — see the sync tests).
    expect(setHarnessSelected.mock.calls).toEqual([
      ["cursor", true],
      ["cursor", false],
      ["cursor", true],
    ]);
    expect(ask).toHaveBeenCalledTimes(1);
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("an off refused while the on's save runs changes nothing (D19): the saved on still asks once", async () => {
    const { setSelected, setHarnessSelected, ask, run } = setup();
    const first = deferred<HarnessSaveResult>();
    setHarnessSelected.mockImplementationOnce(() => first.promise).mockImplementationOnce(async () => "refused");
    let on: Promise<void> = Promise.resolve();
    await act(async () => {
      on = setSelected("cursor", true);
      await setSelected("cursor", false);
    });
    await act(async () => {
      first.resolve("saved");
      await on;
    });
    expect(ask).toHaveBeenCalledTimes(1);
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("on, then a refused off and a refused on: the saved on asks once", async () => {
    const { setSelected, setHarnessSelected, ask, run } = setup();
    const first = deferred<HarnessSaveResult>();
    setHarnessSelected
      .mockImplementationOnce(() => first.promise)
      .mockImplementationOnce(async () => "refused")
      .mockImplementationOnce(async () => "refused");
    let on: Promise<void> = Promise.resolve();
    await act(async () => {
      on = setSelected("cursor", true);
      await setSelected("cursor", false);
      await setSelected("cursor", true);
    });
    await act(async () => {
      first.resolve("saved");
      await on;
    });
    expect(ask).toHaveBeenCalledTimes(1);
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("auto-deploy turned off while the save runs: no ask", async () => {
    const { setSelected, setHarnessSelected, rerender, ask } = setup();
    const first = deferred<HarnessSaveResult>();
    setHarnessSelected.mockImplementationOnce(() => first.promise);
    let on: Promise<void> = Promise.resolve();
    await act(async () => {
      on = setSelected("cursor", true);
    });
    rerender({ autoSync: false });
    await act(async () => {
      first.resolve("saved");
      await on;
    });
    expect(ask).not.toHaveBeenCalled();
  });

  it("a saved selection that no longer includes the harness: no ask", async () => {
    const { setSelected, ask } = setup({ saved: ["claude"] });
    await act(() => setSelected("cursor", true));
    expect(ask).not.toHaveBeenCalled();
  });

  it("an off dismisses the open ask of a waiting on; the dismissed on deploys nothing", async () => {
    const { setSelected, ask, layer, dismissDialog, savedSelection, run } = setup();
    const answer = deferred<AskAnswer>();
    ask.mockImplementationOnce(() => answer.promise);
    layer.mockImplementation(() => ({ kind: "deployHarness", toolKey: "cursor", toolLabel: "CURSOR", count: 2, askId: 1 }));
    dismissDialog.mockImplementation(() => answer.resolve("dismissed"));
    let on: Promise<void> = Promise.resolve();
    await act(async () => {
      on = setSelected("cursor", true);
    });
    expect(ask).toHaveBeenCalledTimes(1);
    savedSelection.current = [];
    await act(async () => {
      await setSelected("cursor", false);
      await on;
    });
    expect(dismissDialog).toHaveBeenCalledWith("deployHarness");
    expect(run).not.toHaveBeenCalled();
  });

  it("an off for another harness leaves this harness's ask alone", async () => {
    const { setSelected, ask, layer, dismissDialog } = setup({ saved: ["cursor"] });
    const answer = deferred<AskAnswer>();
    ask.mockImplementationOnce(() => answer.promise);
    layer.mockImplementation(() => ({ kind: "deployHarness", toolKey: "cursor", toolLabel: "CURSOR", count: 2, askId: 1 }));
    await act(async () => {
      void setSelected("cursor", true);
    });
    await act(() => setSelected("claude", false));
    expect(dismissDialog).not.toHaveBeenCalled();
    answer.resolve("declined");
  });

  it("an intent superseded while the confirmed deploy waits for the rescan deploys nothing", async () => {
    const { setSelected, run, whenIdle, setHarnessSelected, savedSelection } = setup();
    const idle = deferred<boolean>();
    whenIdle.mockImplementationOnce(() => idle.promise);
    let on: Promise<void> = Promise.resolve();
    await act(async () => {
      on = setSelected("cursor", true);
    });
    setHarnessSelected.mockImplementationOnce(async () => "saved");
    savedSelection.current = [];
    await act(async () => {
      await setSelected("cursor", false);
      idle.resolve(true);
      await on;
    });
    expect(run).not.toHaveBeenCalled();
  });
});
