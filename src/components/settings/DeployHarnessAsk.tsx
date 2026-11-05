import { memo } from "react";
import type { TFunction } from "i18next";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import type { PendingAsk } from "../../hooks/useSurfaceState";

type DeployHarnessAskProps = {
  /** The one pending D48 ask (`surface.pendingAsk("deployHarness")`). */
  pending: PendingAsk<"deployHarness"> | null;
  t: TFunction;
};

/**
 * D48: a harness just turned on in Settings → Harnesses while auto-deploy
 * is on — "Deploy your N skills to {harness} now?". An ask layer above the
 * Settings dialog: Deploy now answers `confirmed` (one global batch follows);
 * Not now, Escape or an outside press answer `declined` (nothing runs). Focus
 * returns to the switch that raised it.
 */
const DeployHarnessAsk = ({ pending, t }: DeployHarnessAskProps) => (
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
    {pending ? t("settingsDialog.deployAsk.body", { harness: pending.toolLabel }) : null}
  </ConfirmDialog>
);

export default memo(DeployHarnessAsk);
