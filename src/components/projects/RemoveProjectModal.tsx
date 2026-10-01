import { memo, type ReactNode } from "react";
import type { TFunction } from "i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { FocusFallback } from "@/components/ui/dialog";
import type { ProjectDto } from "./types";

type RemoveProjectModalProps = {
  open: boolean;
  /** Why the dialog's buttons are disabled — an action (this removal included) is running — or null. */
  blockedReason: string | null;
  /** The removal is this dialog's own running action: Remove shows pending and keeps focus. */
  removing: boolean;
  project: ProjectDto | null;
  onConfirm: () => Promise<void>;
  onRequestClose: () => void;
  /** Where focus goes when the opener is gone (the project left the list): the area's heading. */
  fallbackFocus: FocusFallback;
  /** The header progress at the dialog's foot while the removal runs. */
  progress?: ReactNode;
  t: TFunction;
};

/**
 * Remove project — what is removed (the copies Skills Hub deployed into the
 * project's Tool folders, the project and its assignments), what stays (the
 * project's own files, the skills in the library), and how a copy that
 * cannot be removed shows (ADR-0002: the project stays listed and that cell
 * reads failed with its reason). Cancel takes initial focus.
 */
const RemoveProjectModal = ({
  open,
  blockedReason,
  removing,
  project,
  onConfirm,
  onRequestClose,
  fallbackFocus,
  progress,
  t,
}: RemoveProjectModalProps) => {
  const deployed = project?.skill_count ?? 0;
  return (
    <Dialog
      open={open}
      dismissible={!removing}
      onOpenChange={(next) => {
        if (!next) onRequestClose();
      }}
    >
      <DialogContent
        size="sm"
        fallbackFocus={fallbackFocus}
        initialFocus={() => document.querySelector<HTMLElement>("[data-remove-project-cancel]")}
      >
        <DialogHeader>
          <DialogTitle>{t("projectsSurface.remove.title", { name: project?.name ?? "" })}</DialogTitle>
          <DialogDescription>
            {deployed > 0
              ? t("projectsSurface.remove.removes", { count: deployed })
              : t("projectsSurface.remove.removesNone")}
          </DialogDescription>
        </DialogHeader>
        <DialogBody className="flex flex-col gap-2 text-sm">
          {project ? <p className="m-0 font-mono text-xs break-all text-muted-foreground">{project.path}</p> : null}
          <p className="m-0 text-foreground">{t("projectsSurface.remove.stays")}</p>
          <p className="m-0 text-muted-foreground">{t("projectsSurface.remove.kept")}</p>
        </DialogBody>
        <DialogFooter>
          <Button data-remove-project-cancel="" onClick={onRequestClose} disabled={removing}>
            {t("cancel")}
          </Button>
          <Button
            variant="solid"
            tone="danger"
            onClick={() => void onConfirm()}
            pending={removing}
            disabled={blockedReason !== null}
            focusableWhenDisabled
            title={blockedReason ?? undefined}
            aria-description={blockedReason ?? undefined}
          >
            {t("projectsSurface.remove.confirm")}
          </Button>
        </DialogFooter>
        {progress}
      </DialogContent>
    </Dialog>
  );
};

export default memo(RemoveProjectModal);
