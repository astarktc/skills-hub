import { useCallback, useLayoutEffect, useMemo, useRef } from "react";
import type { ManagedSkill, ToolInfoDto } from "../components/skills/types";
import { harnessArrivals } from "../lib/bulkCounts";
import { duplicatePairs, pairsInvolving } from "../lib/harnessPresentation";
import type { RunSkillAction } from "./useSkillActions";
import type { StatusReporter } from "./useStatusReporter";
import type { SurfaceState } from "./useSurfaceState";
import type { SyncOrchestration } from "./useSyncOrchestration";

export type HarnessSelectionDeps = {
  /**
   * The sync world: the saved selection's writer, its synchronous read (the
   * check after each await) and the auto-deploy policy.
   */
  sync: Pick<SyncOrchestration, "setHarnessSelected" | "autoSyncEnabled" | "readSavedSelection">;
  /** The harnesses detected on this computer — the Tool catalog's installedness, as a value. */
  installed: readonly string[];
  /** The Tool catalog's global list: its cross-read pairs (`duplicate_readers`), as a value. */
  catalog: readonly ToolInfoDto[];
  /** The library (the skills world's catalog). */
  skills: readonly ManagedSkill[];
  /** Tool key → display label (the Tool catalog). */
  labelOf: (toolId: string) => string;
  /** The surface module: D48's ask, and withdrawing it when its intent is superseded. */
  surface: Pick<SurfaceState, "ask" | "layer" | "dismissDialog">;
  /** The skill action door: the deploy itself. */
  actions: { run: RunSkillAction };
  /** The lifecycle: the deploy waits out the import-plan rescan the save started. */
  reporter: Pick<StatusReporter, "whenIdle">;
};

/**
 * Settings → Harnesses' selection switch (round-19 ticket 20, D48), composed
 * by the binder over the sync world, the Tool catalog, the library, the
 * surface and the door.
 *
 * Turning a harness **off** saves the selection and removes nothing. Turning
 * one **on** saves it, and — only while auto-deploy is on, only for a harness
 * this computer has (the catalog's installedness: an undetected one is saved
 * and keeps its row's explanation, but every deploy would skip it) and only
 * when the library has skills to give it (`harnessArrivals`, N ≥ 1) — asks
 * once: "Deploy your N skills to {harness} now?". Yes runs one global batch
 * of exactly those skills to that harness through the door (`syncToTargets`,
 * confirmed: the sync seam's one overwrite ask, the header progress) — once
 * the import-plan rescan the save started has settled (`whenIdle`); no, or a
 * dismissed ask, does nothing — the marks stay hollow until Deploy all.
 * Auto-deploy off never asks: nothing is ever deployed automatically.
 * The ask carries the cross-read pairs turning the harness on completes —
 * the other harness of each already in the saved selection — so it can
 * repeat the Harnesses pane's warning (round 20 ticket 02); nothing is
 * deselected.
 *
 * **Latest intent only** (audit F3): an "on" asks — and, confirmed, deploys
 * — only while the harness's last **admitted** gesture is still "on" (a
 * refused gesture changed nothing and counts for nothing — D19), it is the
 * newest "on" that was not refused (an older one never asks beside it), the saved selection still includes the harness, and
 * auto-deploy and installedness still permit it. Checked before asking and
 * again after `whenIdle`; a superseded "on" drops silently. An "off"
 * withdraws the open ask of a waiting "on". A switch activated while held is
 * **not** a gesture: the lifecycle refused it and nothing changed (D19), so
 * the saved "on" still asks and the operator answers the ask.
 */
export function useHarnessSelection({
  sync,
  installed,
  catalog,
  skills,
  labelOf,
  surface,
  actions,
  reporter,
}: HarnessSelectionDeps) {
  const { setHarnessSelected, autoSyncEnabled, readSavedSelection } = sync;
  const { whenIdle } = reporter;
  const { ask, layer, dismissDialog } = surface;
  const { run } = actions;
  // The world as it stands when each check runs (an ask and a rescan wait).
  const latest = useRef({ skills, installed, catalog, autoSyncEnabled, layer });
  useLayoutEffect(() => {
    latest.current = { skills, installed, catalog, autoSyncEnabled, layer };
  });
  // Per harness: its last admitted gesture, and the "on" gestures the
  // lifecycle did not refuse (the newest of them is the one that may ask).
  const lastGesture = useRef(new Map<string, { generation: number; selected: boolean }>());
  const liveOns = useRef(new Map<string, Set<number>>());
  const nextGeneration = useRef(0);

  const select = useCallback(
    async (toolId: string, selected: boolean): Promise<void> => {
      const generation = ++nextGeneration.current;
      // Recorded now so an admitted off supersedes an on whose save is still
      // running; a refusal restores the previous word (D19: a refused
      // gesture changed nothing and withdraws nothing).
      const previous = lastGesture.current.get(toolId);
      lastGesture.current.set(toolId, { generation, selected });
      const ons = liveOns.current.get(toolId) ?? new Set<number>();
      liveOns.current.set(toolId, ons);
      if (selected) ons.add(generation);

      const result = await setHarnessSelected(toolId, selected);
      if (result === "refused") {
        ons.delete(generation);
        if (lastGesture.current.get(toolId)?.generation === generation) {
          if (previous) lastGesture.current.set(toolId, previous);
          else lastGesture.current.delete(toolId);
        }
        return;
      }
      if (!selected) {
        // The harness's waiting "on" is superseded: withdraw its open ask.
        if (latest.current.layer("deployHarness")?.toolKey === toolId) dismissDialog("deployHarness");
        return;
      }
      if (result !== "saved") return;

      const stillWanted = () =>
        lastGesture.current.get(toolId)?.selected === true &&
        Math.max(...ons) === generation &&
        (readSavedSelection()?.includes(toolId) ?? false) &&
        latest.current.autoSyncEnabled &&
        latest.current.installed.includes(toolId);

      // The harness's rows are its Deployment folder's (`folder_key`).
      const folderOf = () => latest.current.catalog.find((info) => info.key === toolId)?.folder_key ?? toolId;
      try {
        if (!stillWanted()) return;
        const count = harnessArrivals(latest.current.skills, folderOf()).length;
        if (count === 0) return;
        const selection = new Set(readSavedSelection() ?? []);
        const duplicates = pairsInvolving(duplicatePairs(latest.current.catalog, selection), toolId);
        const answer = await ask({ kind: "deployHarness", toolKey: toolId, toolLabel: labelOf(toolId), count, duplicates });
        if (answer !== "confirmed" || !(await whenIdle()) || !stillWanted()) return;
        const arrivals = harnessArrivals(latest.current.skills, folderOf());
        if (arrivals.length === 0) return;
        await run("syncToTargets", arrivals, { tools: [toolId], confirmed: true });
      } finally {
        // Settled: this request reads its generation no more. The newest one
        // stays even settled, so an older request still waiting can never
        // outrank it; any other is dropped (the set stays tiny).
        if (Math.max(...ons) !== generation) ons.delete(generation);
      }
    },
    [ask, dismissDialog, labelOf, readSavedSelection, run, setHarnessSelected, whenIdle],
  );

  return useMemo(() => ({ select }), [select]);
}

export type HarnessSelection = ReturnType<typeof useHarnessSelection>;
