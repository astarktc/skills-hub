import { memo } from "react";
import type { TFunction } from "i18next";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import type { PendingAsk } from "../../hooks/useSurfaceState";

type MoveLibraryAskProps = {
  /** The one pending Move Library folder ask (`surface.pendingAsk("moveLibrary")`). */
  pending: PendingAsk<"moveLibrary"> | null;
  t: TFunction;
};

/**
 * Move Library folder's confirmation (round 20 #03): "Move N skills to X?
 * Every harness link is re-pointed", with the Unlocatable count, the count
 * of skills kept in place and the backend's cloud-sync warning when they
 * apply (N counts only the skills that move). An ask layer above the
 * Settings dialog: Move answers `confirmed` (the move runs under the
 * lifecycle); Cancel, Escape or an outside press answer `declined` (nothing
 * runs).
 */
const MoveLibraryAsk = ({ pending, t }: MoveLibraryAskProps) => {
  const preview = pending?.preview ?? null;
  return (
    <ConfirmDialog
      open={pending !== null}
      title={
        preview
          ? t("libraryMove.ask.title", { count: preview.skills, path: preview.destination })
          : null
      }
      confirmLabel={t("libraryMove.ask.confirm")}
      cancelLabel={t("libraryMove.ask.cancel")}
      onConfirm={() => pending?.answer("confirmed")}
      onCancel={() => pending?.answer("declined")}
      confirmDisabled={!pending}
    >
      {preview ? (
        <div className="flex flex-col gap-2">
          <p className="m-0">{t("libraryMove.ask.body")}</p>
          {preview.missing > 0 ? (
            <p className="m-0">{t("libraryMove.ask.missing", { count: preview.missing })}</p>
          ) : null}
          {preview.kept > 0 ? <p className="m-0">{t("libraryMove.ask.kept", { count: preview.kept })}</p> : null}
          {preview.cloud_sync ? (
            <p className="m-0 text-stale">{t("libraryMove.ask.cloud", { root: preview.cloud_sync.root })}</p>
          ) : null}
        </div>
      ) : null}
    </ConfirmDialog>
  );
};

export default memo(MoveLibraryAsk);
