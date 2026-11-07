import { memo } from "react";
import type { TFunction } from "i18next";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import type { LayerOf } from "../../hooks/useSurfaceState";

type RedeployOfferDialogProps = {
  /** The offered layer (`surface.layer("redeployOffer")`), or null while closed. */
  layer: LayerOf<"redeployOffer"> | null;
  labelOf: (toolId: string) => string;
  /** Move now: the redeploy runs under the lifecycle. */
  onAccept: () => void;
  /** Not now (Cancel, Escape, an outside press): nothing moves; the Harnesses pane keeps the line. */
  onDecline: () => void;
  t: TFunction;
};

/**
 * The launch check's offer (round 20 #108): "Move N Deployments to their new
 * folders?", naming the harnesses whose folders changed and any old copy from
 * an interrupted move still awaiting a decision. Offered once through
 * `offerDialog` (it waits for the stack to empty); the preview's counts are
 * the backend's.
 */
const RedeployOfferDialog = ({ layer, labelOf, onAccept, onDecline, t }: RedeployOfferDialogProps) => {
  const preview = layer?.preview ?? null;
  const moves = preview ? preview.global.correction + preview.global.migration : 0;
  const count = preview ? moves + preview.journal : 0;
  return (
    <ConfirmDialog
      open={layer !== null}
      title={preview ? t("redeploy.offer.title", { count }) : null}
      confirmLabel={t("redeploy.offer.confirm")}
      cancelLabel={t("redeploy.offer.cancel")}
      onConfirm={onAccept}
      onCancel={onDecline}
      confirmDisabled={!layer}
    >
      {preview ? (
        <div className="flex flex-col gap-2">
          <p className="m-0">
            {t("redeploy.offer.body", {
              harnesses: preview.harnesses.map(labelOf).join(t("common.listSeparator")),
            })}
          </p>
          {preview.journal > 0 ? (
            <p className="m-0">{t("redeploy.offer.journal", { count: preview.journal })}</p>
          ) : null}
        </div>
      ) : null}
    </ConfirmDialog>
  );
};

export default memo(RedeployOfferDialog);
