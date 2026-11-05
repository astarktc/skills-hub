// The New harnesses notice's "Deploy all skills" (D34, 1.3.2): the binder's
// decision, driven through the hook it composes. The sync double carries the
// Auto-deploy policy as the real sync world does, so a decision that read it
// again would fail here; the door's `run` is the deploy itself.

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { newHarnessDeployTargets, useNewHarnessesDeploy, type NewHarnessesDeployDeps } from "./useNewHarnessesDeploy";

const t = (key: string) => key;

function setup({
  autoSyncEnabled,
  newlyInstalled,
  selected,
}: {
  autoSyncEnabled: boolean;
  newlyInstalled: string[];
  selected: string[];
}) {
  const enableTargetsFor = vi.fn();
  const dismissDialog = vi.fn();
  const notify = vi.fn();
  const run = vi.fn(async () => undefined);
  // The real sync world's shape, Auto-deploy included.
  const sync = {
    relevantNewlyInstalled: newlyInstalled,
    effectiveSyncTargetIds: selected,
    enableTargetsFor,
    autoSyncEnabled,
  };
  const { result } = renderHook(() =>
    useNewHarnessesDeploy({
      sync,
      surface: { dismissDialog },
      reporter: { notify },
      // SAFETY: the mock answers the narrowed signature the hook calls.
      actions: { run: run as never },
      t,
    } satisfies NewHarnessesDeployDeps),
  );
  return { deployAll: () => act(() => result.current()), enableTargetsFor, dismissDialog, notify, run };
}

describe("useNewHarnessesDeploy", () => {
  it.each([false, true])(
    "Auto-deploy %s: Deploy all deploys every skill to the new selected harnesses",
    (autoSyncEnabled) => {
      const s = setup({ autoSyncEnabled, newlyInstalled: ["cursor", "kiro"], selected: ["claude", "cursor", "kiro"] });
      s.deployAll();
      expect(s.dismissDialog).toHaveBeenCalledWith("newTools");
      expect(s.enableTargetsFor).toHaveBeenCalledWith(["cursor", "kiro"]);
      expect(s.run).toHaveBeenCalledTimes(1);
      expect(s.run).toHaveBeenCalledWith("syncToTargets", "all", { tools: ["cursor", "kiro"], confirmed: true });
      expect(s.notify).not.toHaveBeenCalled();
    },
  );

  it("deploys only to the new harnesses inside the selection — never to every selected one", () => {
    const s = setup({ autoSyncEnabled: false, newlyInstalled: ["cursor", "kiro"], selected: ["claude", "kiro"] });
    s.deployAll();
    expect(s.enableTargetsFor).toHaveBeenCalledWith(["kiro"]);
    expect(s.run).toHaveBeenCalledWith("syncToTargets", "all", { tools: ["kiro"], confirmed: true });
  });

  it("with no new harness in the selection, it closes the notice, deploys nothing and says so", () => {
    const s = setup({ autoSyncEnabled: true, newlyInstalled: ["cursor"], selected: ["claude"] });
    s.deployAll();
    expect(s.dismissDialog).toHaveBeenCalledWith("newTools");
    expect(s.notify).toHaveBeenCalledWith("warning", "noSyncTargets");
    expect(s.enableTargetsFor).not.toHaveBeenCalled();
    expect(s.run).not.toHaveBeenCalled();
  });

  it("with no newly detected harness it does nothing at all", () => {
    const s = setup({ autoSyncEnabled: true, newlyInstalled: [], selected: ["claude"] });
    s.deployAll();
    expect(s.dismissDialog).not.toHaveBeenCalled();
    expect(s.notify).not.toHaveBeenCalled();
    expect(s.run).not.toHaveBeenCalled();
  });
});

describe("newHarnessDeployTargets", () => {
  it("is the newly detected harnesses ∩ the effective selection, in detection order", () => {
    expect(newHarnessDeployTargets(["kiro", "cursor", "zed"], ["cursor", "claude", "kiro"])).toEqual(["kiro", "cursor"]);
    expect(newHarnessDeployTargets(["cursor"], [])).toEqual([]);
  });
});
