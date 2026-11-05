import { useCallback } from "react";
import type { RunSkillAction } from "./useSkillActions";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";
import type { SurfaceState } from "./useSurfaceState";
import type { SyncOrchestration } from "./useSyncOrchestration";

export type NewHarnessesDeployDeps = {
  /**
   * The sync world: the harnesses newly detected here, the effective target
   * selection and the target pick's writer. Deliberately not the Auto-deploy
   * policy — it governs a new skill's arrival, never this button (D34).
   */
  sync: Pick<SyncOrchestration, "relevantNewlyInstalled" | "effectiveSyncTargetIds" | "enableTargetsFor">;
  /** The surface module: the New harnesses notice closes on its answer. */
  surface: Pick<SurfaceState, "dismissDialog">;
  /** The lifecycle: zero work is reported, never silent. */
  reporter: Pick<StatusReporter, "notify">;
  /** The skill action door: the deploy itself. */
  actions: { run: RunSkillAction };
  t: TranslateFn;
};

/**
 * Where "Deploy all skills" deploys: the newly detected harnesses that are
 * in the effective target selection. A sync target set is the operator's
 * recorded selection, not detection — a newly detected harness outside it
 * is announced but never written to (the `scan_selected_tools_only` scan
 * setting does not gate a sync).
 */
export function newHarnessDeployTargets(
  newlyRelevant: readonly string[],
  selectedTargets: readonly string[],
): string[] {
  return newlyRelevant.filter((id) => selectedTargets.includes(id));
}

/**
 * The New harnesses notice's "Deploy all skills", composed by the binder over
 * the sync world, the surface, the reporter and the door. It closes the
 * notice, then deploys every managed skill to the newly detected selected
 * harnesses (`newHarnessDeployTargets`) as one confirmed global batch
 * (`syncToTargets`: the sync seam's one overwrite ask, the header progress),
 * after enabling those targets in the pick (expanded to shared-dir groups).
 * An explicit Deploy always deploys, whatever Auto-deploy says (D34; the
 * notice was the confirmation). With no target it deploys nothing and says
 * so (`noSyncTargets`).
 */
export function useNewHarnessesDeploy({ sync, surface, reporter, actions, t }: NewHarnessesDeployDeps) {
  const { relevantNewlyInstalled, effectiveSyncTargetIds, enableTargetsFor } = sync;
  const { dismissDialog } = surface;
  const { notify } = reporter;
  const { run } = actions;
  return useCallback(() => {
    if (relevantNewlyInstalled.length === 0) return;
    const targets = newHarnessDeployTargets(relevantNewlyInstalled, effectiveSyncTargetIds);
    dismissDialog("newTools");
    if (targets.length === 0) {
      notify("warning", t("noSyncTargets"));
      return;
    }
    enableTargetsFor(targets);
    void run("syncToTargets", "all", { tools: targets, confirmed: true });
  }, [dismissDialog, effectiveSyncTargetIds, enableTargetsFor, notify, relevantNewlyInstalled, run, t]);
}
