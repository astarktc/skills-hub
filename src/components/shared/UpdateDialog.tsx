import { memo, useId } from "react";
import type { TFunction } from "i18next";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Dialog, DialogContent } from "@/components/ui/dialog";
import type { AppUpdate } from "../../hooks/useAppUpdate";

type UpdateDialogProps = {
  /** Whether the surface module shows the `update` dialog. */
  open: boolean;
  /** The one update module (`useAppUpdate`). */
  update: Pick<
    AppUpdate,
    "status" | "version" | "notes" | "install" | "dismiss" | "ignore"
  >;
  t: TFunction;
};

/**
 * The update dialog (opened from the passive sidebar item): the found release, its notes, Update now /
 * Don't remind me, then "installed — restart". A view of the update module;
 * Settings reads the same module, so both show one release.
 */
const UpdateDialog = ({ open, update, t }: UpdateDialogProps) => {
  const titleId = useId();
  const { status, version, notes, install, dismiss, ignore } = update;
  const installing = status === "installing";
  const done = status === "done";

  return (
    <Dialog
      open={open && Boolean(version)}
      dismissible={!installing}
      onOpenChange={(next) => {
        if (!next) dismiss();
      }}
    >
      <DialogContent size="sm" showCloseButton={!done} aria-labelledby={titleId}>
        <div className="update-modal">
          <div className="update-modal-body">
            <div className="update-modal-title" id={titleId}>
              {done ? t("updateInstalledRestart") : t("updateAvailable")}
            </div>
            {!done && (
              <div className="update-modal-text">
                {t("updateBannerText", { version })}
              </div>
            )}
            {!done && notes && (
              <div className="update-modal-notes">
                <Markdown remarkPlugins={[remarkGfm]}>{notes}</Markdown>
              </div>
            )}
          </div>
          <div className="update-modal-actions">
            {done ? (
              <button className="btn btn-primary" type="button" onClick={dismiss}>
                {t("done")}
              </button>
            ) : (
              <>
                <button
                  className="btn btn-primary"
                  type="button"
                  disabled={installing}
                  onClick={() => void install()}
                >
                  {installing ? t("installingUpdate") : t("updateNow")}
                </button>
                {!installing && (
                  <button className="btn btn-secondary" type="button" onClick={ignore}>
                    {t("updateBannerDismiss")}
                  </button>
                )}
              </>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default memo(UpdateDialog);
