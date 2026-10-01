import * as React from "react";
import { Popover as PopoverPrimitive } from "@base-ui/react/popover";
import { cn } from "@/lib/utils";
import { claimDismissal, closeLayer, isOpenLayer, openLayer, setLayerPopup } from "./layer-stack";
import { floatingSurface } from "./styles";

/**
 * Popover — non-modal floating panel anchored to its trigger (Base UI:
 * collision-aware, origin-aware, Escape closes it and returns focus to the
 * trigger). Appears instantly. Pass the trigger through `render`:
 * `<PopoverTrigger render={<Button />}>…</PopoverTrigger>`. A popover a
 * control raises on its own terms (a row switch's inline confirmation) has no
 * Trigger: it is controlled (`open`), positioned on `anchor`, and names
 * where focus goes in (`initialFocus`) and back (`finalFocus`).
 *
 * It is a **floating layer** of the one layer stack (`layer-stack.ts`): a
 * confirmation one of its controls raises (Add's Tool scope → the
 * shared-folder ask, a sibling dialog in its own portal) sits above it, so
 * Escape, the press of the ask's answer and the focus the ask takes are the
 * ask's — the popover stays open and inert beneath it, and the ask returns
 * focus to the control that raised it. Only then does the next Escape close
 * the popover.
 */
const PopoverLayerContext = React.createContext<string | null>(null);

/** The reasons a popover is dismissed by something the layer above may own. */
const DISMISSALS = new Set<string>(["escape-key", "outside-press", "focus-out"]);

function Popover({ onOpenChange, ...props }: PopoverPrimitive.Root.Props) {
  const layerId = React.useId();
  const handleOpenChange = React.useCallback<NonNullable<PopoverPrimitive.Root.Props["onOpenChange"]>>(
    (next, details) => {
      if (
        !next &&
        DISMISSALS.has(details.reason) &&
        isOpenLayer(layerId) &&
        !claimDismissal(layerId, details.event)
      ) {
        details.cancel();
        return;
      }
      onOpenChange?.(next, details);
    },
    [layerId, onOpenChange],
  );
  return (
    <PopoverLayerContext.Provider value={layerId}>
      <PopoverPrimitive.Root onOpenChange={handleOpenChange} {...props} />
    </PopoverLayerContext.Provider>
  );
}

/**
 * Mounted exactly while the popup is: registers the popover as a floating
 * layer (it makes nothing inert itself) and releases it on unmount.
 */
function FloatingLayerRegistration({ layerId }: { layerId: string }) {
  React.useLayoutEffect(() => {
    openLayer(layerId, undefined, "floating");
    return () => closeLayer(layerId);
  }, [layerId]);
  return null;
}

/**
 * Focus coming back into the popover from nowhere — the layer above it (the
 * ask its checkbox raised) unmounted with focus inside and returned it here
 * — puts the portal's controls back in the tab order. Base UI took them out
 * when focus left for that layer, and puts them back only for a focus that
 * arrives from a known element, so without this Tab would leave the popover
 * from the very checkbox focus returned to.
 */
function restoreTabOrder(event: React.FocusEvent<HTMLElement>) {
  if (event.relatedTarget !== null) return;
  const scope = event.currentTarget.closest<HTMLElement>("[data-base-ui-portal]") ?? event.currentTarget;
  for (const element of scope.querySelectorAll<HTMLElement>("[data-tabindex]")) {
    const tabindex = element.dataset.tabindex;
    delete element.dataset.tabindex;
    if (tabindex) element.setAttribute("tabindex", tabindex);
    else element.removeAttribute("tabindex");
  }
}

const PopoverTrigger = PopoverPrimitive.Trigger;
const PopoverClose = PopoverPrimitive.Close;

function PopoverContent({
  className,
  align = "center",
  alignOffset = 0,
  side = "bottom",
  sideOffset = 4,
  anchor,
  children,
  onFocus,
  ...props
}: Omit<PopoverPrimitive.Popup.Props, "className"> &
  Pick<
    PopoverPrimitive.Positioner.Props,
    "align" | "alignOffset" | "side" | "sideOffset" | "anchor"
  > & {
    /** Layout only (width). */
    className?: string;
  }) {
  const layerId = React.useContext(PopoverLayerContext);
  const popupRef = React.useCallback(
    (node: HTMLDivElement | null) => {
      if (layerId) setLayerPopup(layerId, node);
    },
    [layerId],
  );
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Positioner
        align={align}
        alignOffset={alignOffset}
        side={side}
        sideOffset={sideOffset}
        anchor={anchor}
        className="z-50"
      >
        <PopoverPrimitive.Popup
          data-slot="popover-content"
          ref={popupRef}
          onFocus={(event) => {
            restoreTabOrder(event);
            onFocus?.(event);
          }}
          className={cn(
            "flex w-72 flex-col gap-2 p-3 text-sm",
            floatingSurface,
            className,
          )}
          {...props}
        >
          {layerId ? <FloatingLayerRegistration layerId={layerId} /> : null}
          {children}
        </PopoverPrimitive.Popup>
      </PopoverPrimitive.Positioner>
    </PopoverPrimitive.Portal>
  );
}

function PopoverTitle({
  className,
  ...props
}: Omit<PopoverPrimitive.Title.Props, "className"> & { className?: string }) {
  return (
    <PopoverPrimitive.Title
      data-slot="popover-title"
      className={cn("m-0 text-sm font-semibold text-foreground", className)}
      {...props}
    />
  );
}

function PopoverDescription({
  className,
  ...props
}: Omit<PopoverPrimitive.Description.Props, "className"> & {
  className?: string;
}) {
  return (
    <PopoverPrimitive.Description
      data-slot="popover-description"
      className={cn("m-0 text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

export type PopoverContentProps = React.ComponentProps<typeof PopoverContent>;

export {
  Popover,
  PopoverClose,
  PopoverContent,
  PopoverDescription,
  PopoverTitle,
  PopoverTrigger,
};
