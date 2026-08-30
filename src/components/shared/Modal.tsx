import { useEffect, useId, useRef, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

type ModalProps = {
  open: boolean;
  /**
   * Invoked by backdrop click and the header ✕. Omit to make the backdrop
   * inert (dialog closes only through its own buttons).
   */
  onRequestClose?: () => void;
  /** Renders a modal-header with this title; absent = headerless dialog. */
  title?: ReactNode;
  /**
   * Show the header ✕ button. Defaults to true when both title and
   * onRequestClose are present. Ignored when there is no header.
   */
  showCloseButton?: boolean;
  /** Disables both the ✕ button and backdrop click-to-close. */
  closeDisabled?: boolean;
  /** Extra classes on the dialog box, e.g. "modal-delete" or "modal-lg". */
  className?: string;
  /** Extra classes on the modal-body wrapper, e.g. "delete-body". */
  bodyClassName?: string;
  /** Renders a modal-footer when present. */
  footer?: ReactNode;
  /** Extra classes on the modal-footer wrapper, e.g. "space-between". */
  footerClassName?: string;
  /**
   * Chrome-only mode: render children directly inside the dialog box with
   * no header/body/footer wrappers. For dialogs with fully custom insides.
   */
  plain?: boolean;
  /**
   * Accessible name for the dialog when no title is rendered (or in plain
   * mode). Ignored when a title is present — the title labels the dialog.
   */
  "aria-label"?: string;
  children: ReactNode;
};

/**
 * The one modal shell: backdrop (click-to-close) → dialog (stopPropagation,
 * role="dialog", aria-modal) → header (title + ✕) → body → footer.
 * Owns the open gate: children are not mounted while closed, so per-modal
 * state resets on reopen exactly as the hand-rolled skeletons did.
 * Styling is global CSS (.modal*) — this is a JSX shell, not a style system.
 */
const Modal = ({
  open,
  onRequestClose,
  title,
  showCloseButton,
  closeDisabled = false,
  className,
  bodyClassName,
  footer,
  footerClassName,
  plain = false,
  "aria-label": ariaLabel,
  children,
}: ModalProps) => {
  const { t } = useTranslation();
  const titleId = useId();
  const dialogRef = useRef<HTMLDivElement>(null);

  const canClose = onRequestClose !== undefined && !closeDisabled;
  const hasTitle = !plain && title !== undefined;

  // Move focus into the dialog on open (component unmounts while closed,
  // so mount === open).
  useEffect(() => {
    if (open) dialogRef.current?.focus();
  }, [open]);

  // Escape closes exactly when the backdrop click would.
  useEffect(() => {
    if (!open || !canClose) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onRequestClose?.();
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [open, canClose, onRequestClose]);

  if (!open) return null;
  const showClose =
    title !== undefined &&
    (showCloseButton ?? onRequestClose !== undefined);

  return (
    <div
      className="modal-backdrop"
      onClick={canClose ? onRequestClose : undefined}
    >
      <div
        ref={dialogRef}
        className={className ? `modal ${className}` : "modal"}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby={hasTitle ? titleId : undefined}
        aria-label={hasTitle ? undefined : ariaLabel}
        tabIndex={-1}
      >
        {plain ? (
          children
        ) : (
          <>
            {title !== undefined && (
              <div className="modal-header">
                <div className="modal-title" id={titleId}>
                  {title}
                </div>
                {showClose && (
                  <button
                    className="modal-close"
                    type="button"
                    onClick={onRequestClose}
                    aria-label={t("close")}
                    disabled={closeDisabled}
                  >
                    &#10005;
                  </button>
                )}
              </div>
            )}
            <div
              className={
                bodyClassName ? `modal-body ${bodyClassName}` : "modal-body"
              }
            >
              {children}
            </div>
            {footer !== undefined && (
              <div
                className={
                  footerClassName
                    ? `modal-footer ${footerClassName}`
                    : "modal-footer"
                }
              >
                {footer}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
};

export default Modal;
