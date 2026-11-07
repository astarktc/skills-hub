import { useCallback, useEffect, useRef, useState } from "react";
import type {
  RedeployAnswer,
  RedeployPreview,
  RedeployProgress,
  RedeployResultDto,
} from "../components/skills/types";
import { createProgressChannel } from "../lib/platform";
import {
  mergeRedeployRetry,
  redeployAnswers,
  redeployAskRows,
  type OverwriteChoice,
} from "../lib/overwriteAsk";
import { redeployOfferDeclinedPreference } from "../lib/preferences";
import { redeployOutcome } from "../lib/reportOutcome";
import { invokeTauri, isTauri } from "../lib/tauri";
import type { SkillLibrary } from "./useSkillLibrary";
import type { StatusReporter, TranslateFn } from "./useStatusReporter";
import type { AskAnswer, RowChoices, SurfaceState } from "./useSurfaceState";

export type RedeployDeps = {
  t: TranslateFn;
  reporter: Pick<
    StatusReporter,
    "runAction" | "reportProgress" | "applyOutcome" | "notifyError" | "setActionMessage" | "whenIdle"
  >;
  /** The launch offer (`offerDialog`), its dismissal and the one overwrite ask. */
  surface: Pick<SurfaceState, "ask" | "offerDialog" | "dismissDialog">;
  /** The skills world: the redeploy answers the catalog it settled. */
  library: Pick<SkillLibrary, "replaceCatalog" | "loadManagedSkills">;
  /** The Tool catalog's labels, for the ask's rows and the report. */
  toolLabelById: Readonly<Record<string, string>>;
};

/** How many Deployments a preview says are still to move or decide. */
export function redeployPendingCount(preview: RedeployPreview | null): number {
  if (!preview) return 0;
  return preview.global.correction + preview.global.migration + preview.journal;
}

/**
 * The work a preview describes, as the offer's "declined" memory spells it:
 * an offer is raised again only for different work (a later correction), and
 * never twice for the same.
 */
export function redeployOfferSignature(preview: RedeployPreview): string {
  return JSON.stringify([
    preview.global.correction,
    preview.global.migration,
    preview.journal,
    [...preview.harnesses].sort(),
  ]);
}

/** The per-row choices an overwrite ask settled with. */
function choicesOf(answer: AskAnswer | RowChoices, rowCount: number): readonly OverwriteChoice[] | null {
  if (answer === "dismissed") return null;
  if (answer === "confirmed") return Array<OverwriteChoice>(rowCount).fill("overwrite");
  if (answer === "declined") return Array<OverwriteChoice>(rowCount).fill("keep");
  return answer.choices;
}

/**
 * The redeploy (round 20 #108), as the app shows it: the launch check, the
 * offer, the run and the Harnesses pane's "Move N Deployments" line.
 *
 * - **Launch check** (once per app): the backend finished the journal at
 *   startup; `getRedeployLaunchCheck` hands out that report (once — folded
 *   as a background run) and the preview. When the preview has work the
 *   operator has not already declined, the offer is raised once through
 *   `offerDialog` (it waits for the stack to empty).
 * - **Accept** waits for the startup reads to settle (`whenIdle`), then
 *   runs the mutation under `runAction` (header progress, no
 *   Cancel). Its report's folders in a move's way and edited copies are asked
 *   in **one** overwrite ask, answered per row; the answers go back in a
 *   second run (`redeployAnswers`) whose report is merged into the first
 *   (`mergeRedeployRetry`) and folded once (`redeployOutcome`). A dismissed
 *   ask sends nothing. The catalog and the preview are replaced from the
 *   command's own answer; a run that throws re-reads the catalog.
 * - **Decline** remembers the declined work (`redeployOfferSignature`), and
 *   the line keeps it reachable: `pendingCount` is backed by the preview.
 */
export function useRedeploy({ t, reporter, surface, library, toolLabelById }: RedeployDeps) {
  const { runAction, reportProgress, applyOutcome, notifyError, setActionMessage, whenIdle } = reporter;
  const { ask, offerDialog, dismissDialog } = surface;
  const { replaceCatalog, loadManagedSkills } = library;
  const [preview, setPreview] = useState<RedeployPreview | null>(null);

  const launchCheck = useCallback(async () => {
    const check = await invokeTauri("getRedeployLaunchCheck");
    if (check.finished) {
      await applyOutcome(redeployOutcome(check.finished, { t, toolLabelById }, { resumed: true }), {
        background: { label: t("redeploy.resumedLabel") },
      });
    }
    setPreview(check.preview);
    if (
      redeployPendingCount(check.preview) > 0 &&
      redeployOfferDeclinedPreference.read() !== redeployOfferSignature(check.preview)
    ) {
      offerDialog({ kind: "redeployOffer", preview: check.preview });
    }
  }, [applyOutcome, offerDialog, t, toolLabelById]);

  // Once per app (a ref, so StrictMode's second effect run checks nothing).
  const checked = useRef(false);
  useEffect(() => {
    if (!isTauri || checked.current) return;
    checked.current = true;
    launchCheck().catch(notifyError);
  }, [launchCheck, notifyError]);

  const run = useCallback(async () => {
    await runAction({ label: t("redeploy.label"), activity: t("redeploy.activity") }, async () => {
      const channel = await createProgressChannel<RedeployProgress>((progress) =>
        reportProgress({
          done: progress.done,
          total: progress.total,
          message: t("redeploy.step", { name: progress.name }),
        }),
      );
      const redeploy = async (answers: RedeployAnswer[]): Promise<RedeployResultDto> => {
        try {
          return await invokeTauri("redeployDeployments", answers, channel);
        } catch (err) {
          // It may have failed after some moves: re-read, never assume.
          await loadManagedSkills();
          await invokeTauri("getRedeployLaunchCheck").then(
            (check) => setPreview(check.preview),
            () => {},
          );
          throw err;
        }
      };
      let latest: RedeployResultDto = await redeploy([]);
      let report = latest.report;
      const rows = redeployAskRows(report, toolLabelById);
      if (rows.length > 0) {
        setActionMessage(t("overwrite.waiting"));
        const choices = choicesOf(await ask({ kind: "overwrite", rows }), rows.length);
        if (choices) {
          latest = await redeploy(redeployAnswers(rows, choices));
          report = mergeRedeployRetry(report, latest.report);
        }
      }
      replaceCatalog(latest.skills);
      setPreview(latest.preview);
      await applyOutcome(redeployOutcome(report, { t, toolLabelById }));
    });
  }, [
    applyOutcome,
    ask,
    loadManagedSkills,
    replaceCatalog,
    reportProgress,
    runAction,
    setActionMessage,
    t,
    toolLabelById,
  ]);

  /**
   * The offer's Move now: close it, then run — once the startup reads (the
   * harness scan, the import plan) have settled, so the answer is never
   * refused by a read the operator did not start.
   */
  const acceptOffer = useCallback(async () => {
    dismissDialog("redeployOffer");
    if (!(await whenIdle())) return;
    await run();
  }, [dismissDialog, run, whenIdle]);

  /** The offer's Not now: remember the declined work; the line keeps it. */
  const declineOffer = useCallback(() => {
    if (preview) redeployOfferDeclinedPreference.write(redeployOfferSignature(preview));
    dismissDialog("redeployOffer");
  }, [dismissDialog, preview]);

  return {
    preview,
    pendingCount: redeployPendingCount(preview),
    run,
    acceptOffer,
    declineOffer,
  };
}

export type Redeploy = ReturnType<typeof useRedeploy>;
