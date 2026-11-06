import { memo } from "react";
import type { TFunction } from "i18next";
import { TriangleAlert } from "lucide-react";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import type { PendingAsk } from "../../hooks/useSurfaceState";
import { duplicateWarning } from "../../lib/harnessPresentation";

type DeployHarnessAskProps = {
  /** The one pending D48 ask (`surface.pendingAsk("deployHarness")`). */
  pending: PendingAsk<"deployHarness"> | null;
  /** Tool key → display label (the Tool catalog). */
  labelOf: (toolId: string) => string;
  t: TFunction;
};

/**
 * D48: a harness just turned on in Settings → Harnesses while auto-deploy
 * is on — "Deploy your N skills to {harness} now?". An ask layer above the
 * Settings dialog: Deploy now answers `confirmed` (one global batch follows);
 * Not now, Escape or an outside press answer `declined` (nothing runs). Focus
 * returns to the switch that raised it. When turning the harness on completes
 * a cross-read pair, one line repeats the Harnesses pane's warning (round 20
 * ticket 02); Deploy now still deploys — nothing is deselected.
 */
const DeployHarnessAsk = ({ pending, labelOf, t }: DeployHarnessAskProps) => {
  const warning = pending ? duplicateWarning(pending.duplicates, labelOf, t) : null;
  return (
  <ConfirmDialog
    open={pending !== null}
    title={
      pending
        ? t("settingsDialog.deployAsk.title", { count: pending.count, harness: pending.toolLabel })
        : null
    }
    confirmLabel={t("settingsDialog.deployAsk.confirm")}
    cancelLabel={t("settingsDialog.deployAsk.cancel")}
    onConfirm={() => pending?.answer("confirmed")}
    onCancel={() => pending?.answer("declined")}
    confirmDisabled={!pending}
  >
    {pending ? <p className="m-0">{t("settingsDialog.deployAsk.body", { harness: pending.toolLabel })}</p> : null}
    {warning ? (
      <p data-deploy-ask-warning className="m-0 flex items-start gap-1 text-xs text-stale">
        <TriangleAlert aria-hidden="true" className="mt-px size-3.5 shrink-0" />
        {warning}
      </p>
    ) : null}
  </ConfirmDialog>
  );
};

export default memo(DeployHarnessAsk);
