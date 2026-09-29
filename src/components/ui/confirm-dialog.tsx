import * as React from "react";
import { Button } from "./button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "./dialog";

/**
 * ConfirmDialog — a question the operator answers with one of two buttons.
 * Escape / outside press answer "cancel" (the safe answer); there is no ✕.
 * Initial focus is on the safe button for `danger`, on the confirm button
 * for `accent`. The "ask" hooks (useSharedDirConfirmation,
 * useOverwriteConfirmation) resolve their `request()` promise from these
 * two callbacks.
 */
type ConfirmDialogProps = {
  open: boolean;
  title: React.ReactNode;
  /** The question's detail (body). */
  children?: React.ReactNode;
  confirmLabel: React.ReactNode;
  cancelLabel: React.ReactNode;
  /** `danger` for destructive answers (destructive button, focus on Cancel). */
  tone?: "accent" | "danger";
  onConfirm: () => void;
  onCancel: () => void;
  /** The confirmed work is running: confirm shows busy, cancel is disabled. */
  pending?: boolean;
  confirmDisabled?: boolean;
  cancelDisabled?: boolean;
  size?: React.ComponentProps<typeof DialogContent>["size"];
  fallbackFocus?: () => HTMLElement | null;
};

function ConfirmDialog({
  open,
  title,
  children,
  confirmLabel,
  cancelLabel,
  tone = "accent",
  onConfirm,
  onCancel,
  pending = false,
  confirmDisabled = false,
  cancelDisabled = false,
  size = "md",
  fallbackFocus,
}: ConfirmDialogProps) {
  const cancelRef = React.useRef<HTMLButtonElement>(null);
  const confirmRef = React.useRef<HTMLButtonElement>(null);
  const answerable = !pending && !cancelDisabled;
  return (
    <Dialog
      open={open}
      dismissible={answerable}
      onOpenChange={(next) => {
        if (!next) onCancel();
      }}
    >
      <DialogContent
        size={size}
        showCloseButton={false}
        initialFocus={tone === "danger" ? cancelRef : confirmRef}
        fallbackFocus={fallbackFocus}
      >
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>
        {children !== undefined && children !== null ? (
          <DialogBody>{children}</DialogBody>
        ) : null}
        <DialogFooter>
          <Button
            ref={cancelRef}
            variant="outline"
            tone="neutral"
            onClick={onCancel}
            disabled={!answerable}
          >
            {cancelLabel}
          </Button>
          <Button
            ref={confirmRef}
            variant="solid"
            tone={tone === "danger" ? "danger" : "accent"}
            onClick={onConfirm}
            disabled={confirmDisabled}
            pending={pending}
          >
            {confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export { ConfirmDialog, type ConfirmDialogProps };
