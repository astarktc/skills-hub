import { memo, useId } from "react";
import { TriangleAlert } from "lucide-react";
import type { TFunction } from "i18next";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "@/components/ui/dialog";
import { mainLandmarkFocus } from "@/components/ui/focus-fallback";

type RemoveProjectModalProps = {
  open: boolean;
  /** Why the dialog's buttons are disabled — an action (this removal included) is running — or null. */
  blockedReason: string | null;
  projectName: string | null;
  onConfirm: () => Promise<void>;
  onRequestClose: () => void;
  t: TFunction;
};

const RemoveProjectModal = ({
  open,
  blockedReason,
  projectName,
  onConfirm,
  onRequestClose,
  t,
}: RemoveProjectModalProps) => {
  const titleId = useId();
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onRequestClose();
      }}
    >
      <DialogContent aria-labelledby={titleId} fallbackFocus={() => mainLandmarkFocus()}>
        <DialogBody>
          <div className="delete-title" id={titleId}>
            <TriangleAlert size={20} />
            {t("projects.removeTitle")}
          </div>
          <div className="delete-desc">
            {t("projects.removeBody", { name: projectName ?? "" })}
          </div>
          <div className="delete-warning">
            <ul>
              <li>{t("projects.removeWarning1")}</li>
              <li>{t("projects.removeWarning2")}</li>
            </ul>
          </div>
        </DialogBody>
        <DialogFooter className="justify-between">
          <button
            className="btn btn-secondary"
            onClick={onRequestClose}
            disabled={blockedReason !== null}
          >
            {t("cancel")}
          </button>
          <button
            className="btn btn-danger-solid"
            onClick={onConfirm}
            disabled={blockedReason !== null}
            title={blockedReason ?? undefined}
          >
            {t("projects.removeConfirm")}
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default memo(RemoveProjectModal);
