import { memo } from "react";
import type { TFunction } from "i18next";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
      <DialogContent size="sm" showCloseButton={!done}>
        <DialogHeader>
          <DialogTitle>{done ? t("updateInstalledRestart") : t("updateAvailable")}</DialogTitle>
        </DialogHeader>
        {!done ? (
          <DialogBody>
            <p className="m-0">{t("updateBannerText", { version })}</p>
            {notes ? (
              <div className="prose max-h-52 overflow-y-auto rounded-md border border-border px-4 py-3">
                <Markdown remarkPlugins={[remarkGfm]}>{notes}</Markdown>
              </div>
            ) : null}
          </DialogBody>
        ) : null}
        <DialogFooter>
          {!done && !installing ? <Button onClick={ignore}>{t("updateBannerDismiss")}</Button> : null}
          {/* One button through install → done, so focus stays on it. */}
          <Button
            variant="solid"
            tone="accent"
            pending={installing}
            onClick={done ? dismiss : () => void install()}
          >
            {done ? t("done") : installing ? t("installingUpdate") : t("updateNow")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default memo(UpdateDialog);
