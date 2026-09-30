import * as React from "react";
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import {
  applyInert,
  closeLayer,
  isTopLayer,
  openLayer,
  popupBelow,
  setLayerPopup,
} from "./layer-stack";
import { Button } from "./button";
import { mainLandmarkFocus } from "./focus-fallback";

/**
 * Dialog — the one modal shell (ADR-0006 §2; replaces `Modal.tsx`).
 *
 * What it owns so call sites do not:
 *   - focus: moved in on open, trapped while open, returned on close to the
 *     element that opened it — or, when that element is gone, to the
 *     caller's `fallbackFocus`, then to the layer beneath, then to the main
 *     landmark (`mainLandmarkFocus`) — never to `<body>` while the app has
 *     a main landmark;
 *   - a background that is `inert` (not only aria-hidden) while open;
 *   - Escape and outside press dismiss the **top** layer only, and only when
 *     `dismissible`;
 *   - a name: `<DialogTitle>` labels the popup through `aria-labelledby`
 *     (or pass `aria-labelledby` to `<DialogContent>` for a custom heading);
 *   - no motion: dialogs appear and leave instantly.
 *
 * Controlled only: pass `open` and `onOpenChange`. Keep one `<DialogContent>`
 * for the dialog's life — never key it (or a component that renders it) on
 * state the answer clears: a popup remounted while it closes is stranded as
 * a closed layer that keeps the app inert. Key the per-ask body instead.
 */

type DialogContextValue = { layerId: string; dismissible: boolean };
const DialogContext = React.createContext<DialogContextValue | null>(null);

type DialogProps = Omit<
  DialogPrimitive.Root.Props,
  "modal" | "disablePointerDismissal" | "onOpenChange"
> & {
  open: boolean;
  /** Called with `false` when the operator dismisses the dialog. */
  onOpenChange: (open: boolean) => void;
  /**
   * Escape, outside press and the header ✕ may close it. `false` for a layer
   * that must be answered through its own buttons (or cannot be closed while
   * work runs).
   */
  dismissible?: boolean;
};

function Dialog({
  open,
  onOpenChange,
  dismissible = true,
  children,
  ...props
}: DialogProps) {
  const layerId = React.useId();
  const context = React.useMemo(
    () => ({ layerId, dismissible }),
    [layerId, dismissible],
  );

  const handleOpenChange = React.useCallback(
    (next: boolean, details: DialogPrimitive.Root.ChangeEventDetails) => {
      if (!next) {
        const dismissal =
          details.reason === "escape-key" ||
          details.reason === "outside-press" ||
          details.reason === "close-press";
        // Lower layers ignore dismissals meant for the layer above them.
        if (
          (details.reason === "escape-key" ||
            details.reason === "outside-press") &&
          !isTopLayer(layerId)
        ) {
          details.cancel();
          return;
        }
        if (dismissal && !dismissible) {
          details.cancel();
          return;
        }
      }
      onOpenChange(next);
    },
    [dismissible, layerId, onOpenChange],
  );

  return (
    <DialogContext.Provider value={context}>
      <DialogPrimitive.Root
        data-slot="dialog"
        open={open}
        modal
        disablePointerDismissal={!dismissible}
        onOpenChange={handleOpenChange}
        {...props}
      >
        {children}
      </DialogPrimitive.Root>
    </DialogContext.Provider>
  );
}

function useDialogContext(part: string): DialogContextValue {
  const context = React.useContext(DialogContext);
  if (!context) throw new Error(`<${part}> must be rendered inside <Dialog>`);
  return context;
}

const sizeClass = {
  sm: "w-[min(400px,calc(100vw-48px))]",
  md: "w-[min(520px,calc(100vw-48px))]",
  lg: "w-[min(600px,calc(100vw-48px))]",
  xl: "w-[min(720px,calc(100vw-48px))]",
} as const;

type FocusTarget = DialogPrimitive.Popup.Props["finalFocus"];

type DialogContentProps = Omit<
  DialogPrimitive.Popup.Props,
  "className" | "render" | "finalFocus"
> & {
  /** Popup width: sm 400 · md 520 · lg 600 · xl 720 (capped at viewport − 48). */
  size?: keyof typeof sizeClass;
  /** Header ✕ (32 × 32); disabled while the dialog is not dismissible. */
  showCloseButton?: boolean;
  /**
   * Where focus goes on close. Default: the element that had focus when the
   * dialog opened; if it is gone, `fallbackFocus`, then the layer beneath,
   * then the main landmark.
   */
  finalFocus?: FocusTarget;
  /**
   * Focus target when the opener no longer exists (e.g. a deleted row).
   * Pass it wherever the dialog's action can remove its opener.
   */
  fallbackFocus?: () => HTMLElement | null;
  /** Layout only (never colour or type). */
  className?: string;
};

function DialogContent({
  size = "md",
  showCloseButton = true,
  initialFocus,
  finalFocus,
  fallbackFocus,
  className,
  children,
  ...props
}: DialogContentProps) {
  const { layerId } = useDialogContext("DialogContent");
  const openerRef = React.useRef<HTMLElement | null>(null);

  const popupRef = React.useCallback(
    (node: HTMLDivElement | null) => setLayerPopup(layerId, node),
    [layerId],
  );

  const resolveFinalFocus = React.useCallback<
    Extract<FocusTarget, (...args: never[]) => unknown>
  >(
    (closeType) => {
      if (typeof finalFocus === "function") return finalFocus(closeType);
      if (typeof finalFocus === "boolean") return finalFocus;
      if (finalFocus) return finalFocus.current;
      const target = openerRef.current;
      if (target?.isConnected && !target.closest("[inert]")) return target;
      return fallbackFocus?.() ?? popupBelow(layerId) ?? mainLandmarkFocus() ?? true;
    },
    [fallbackFocus, finalFocus, layerId],
  );

  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Backdrop
        data-slot="dialog-backdrop"
        forceRender
        className="fixed inset-0 z-50 bg-background/60"
      />
      <DialogPrimitive.Popup
        data-slot="dialog-content"
        ref={popupRef}
        initialFocus={initialFocus}
        finalFocus={resolveFinalFocus}
        className={cn(
          "fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100vh-48px)] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-lg border border-border-strong bg-popover text-sm text-popover-foreground shadow-lg outline-none",
          sizeClass[size],
          className,
        )}
        {...props}
      >
        <LayerRegistration layerId={layerId} openerRef={openerRef} />
        {children}
        {showCloseButton ? <DialogCloseCorner /> : null}
      </DialogPrimitive.Popup>
    </DialogPrimitive.Portal>
  );
}

/**
 * Mounted exactly while the popup is (Base UI renders nothing when closed).
 * Registers the layer before Base UI moves focus in (it waits a frame),
 * captures the opener, and makes the background inert once focus has moved.
 * Unmounting releases the layer synchronously, before Base UI returns focus.
 */
function LayerRegistration({
  layerId,
  openerRef,
}: {
  layerId: string;
  openerRef: React.RefObject<HTMLElement | null>;
}) {
  React.useLayoutEffect(() => {
    const active = document.activeElement;
    openerRef.current =
      active instanceof HTMLElement && active !== document.body ? active : null;
    openLayer(layerId);
    let second = 0;
    const first = requestAnimationFrame(() => {
      second = requestAnimationFrame(applyInert);
    });
    return () => {
      cancelAnimationFrame(first);
      cancelAnimationFrame(second);
      closeLayer(layerId);
    };
  }, [layerId, openerRef]);
  return null;
}

/** The header ✕: 32 × 32 target, named, disabled when not dismissible. */
function DialogCloseCorner() {
  const { t } = useTranslation();
  const { dismissible } = useDialogContext("DialogContent");
  return (
    <DialogPrimitive.Close
      data-slot="dialog-close"
      render={
        <Button
          variant="ghost"
          tone="neutral"
          size="icon"
          className="absolute top-3 right-3"
          aria-label={t("close")}
          disabled={!dismissible}
        />
      }
    >
      <X aria-hidden="true" />
    </DialogPrimitive.Close>
  );
}

function DialogHeader({
  className,
  ...props
}: Omit<React.ComponentProps<"div">, "className"> & { className?: string }) {
  return (
    <div
      data-slot="dialog-header"
      className={cn(
        "flex shrink-0 flex-col gap-1 border-b border-border py-4 pr-14 pl-5",
        className,
      )}
      {...props}
    />
  );
}

function DialogTitle({
  className,
  ...props
}: Omit<DialogPrimitive.Title.Props, "className"> & { className?: string }) {
  return (
    <DialogPrimitive.Title
      data-slot="dialog-title"
      className={cn(
        "m-0 text-lg leading-6 font-semibold text-foreground",
        className,
      )}
      {...props}
    />
  );
}

function DialogDescription({
  className,
  ...props
}: Omit<DialogPrimitive.Description.Props, "className"> & {
  className?: string;
}) {
  return (
    <DialogPrimitive.Description
      data-slot="dialog-description"
      className={cn("m-0 text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

type DialogBodyProps = Omit<React.ComponentProps<"div">, "className"> & {
  /** `default` pads the content; `flush` runs it edge to edge (lists). */
  variant?: "default" | "flush";
  className?: string;
};

function DialogBody({ variant = "default", className, ...props }: DialogBodyProps) {
  return (
    <div
      data-slot="dialog-body"
      className={cn(
        "flex min-h-0 flex-1 flex-col overflow-y-auto text-sm leading-normal text-muted-foreground",
        variant === "default" ? "gap-3 p-5" : "gap-0 p-0",
        className,
      )}
      {...props}
    />
  );
}

function DialogFooter({
  className,
  ...props
}: Omit<React.ComponentProps<"div">, "className"> & { className?: string }) {
  return (
    <div
      data-slot="dialog-footer"
      className={cn(
        "flex shrink-0 items-center justify-end gap-3 border-t border-border bg-card px-5 py-4",
        className,
      )}
      {...props}
    />
  );
}

const DialogClose = DialogPrimitive.Close;

export {
  Dialog,
  DialogBody,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
};
