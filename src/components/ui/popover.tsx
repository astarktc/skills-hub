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
 * where focus goes in (`initialFocus`) and back (`finalFocus`); a Tab out of
 * it closes it, on past the anchor or back onto it.
 *
 * Two behaviours here lean on Base UI internals (the tab-order restore, the
 * triggerless Tab-out): `@base-ui/react` is pinned exactly, and
 * `scripts/browser/popover-focus.ts` drives both in a browser — run it before
 * a bump.
 *
 * It is a **floating layer** of the one layer stack (`layer-stack.ts`): a
 * confirmation one of its controls raises (Add's Tool scope → the
 * shared-folder ask, a sibling dialog in its own portal) sits above it, so
 * Escape, the press of the ask's answer and the focus the ask takes are the
 * ask's — the popover stays open and inert beneath it, and the ask returns
 * focus to the control that raised it. Only then does the next Escape close
 * the popover.
 */
/** The popover's layer id and its imperative actions (a Tab out of a triggerless popover closes it). */
const PopoverLayerContext = React.createContext<{
  layerId: string;
  actionsRef: React.RefObject<PopoverPrimitive.Root.Actions | null>;
} | null>(null);

/** The reasons a popover is dismissed by something the layer above may own. */
const DISMISSALS = new Set<string>(["escape-key", "outside-press", "focus-out"]);

function Popover({ onOpenChange, actionsRef: callerActionsRef, ...props }: PopoverPrimitive.Root.Props) {
  const layerId = React.useId();
  const ownActionsRef = React.useRef<PopoverPrimitive.Root.Actions | null>(null);
  const actionsRef = callerActionsRef ?? ownActionsRef;
  const layer = React.useMemo(() => ({ layerId, actionsRef }), [layerId, actionsRef]);
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
    <PopoverLayerContext.Provider value={layer}>
      <PopoverPrimitive.Root onOpenChange={handleOpenChange} actionsRef={actionsRef} {...props} />
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
 *
 * Only the *temporary* removal is undone: a control that became disabled
 * while the layer above was up keeps its current contract — a disabled
 * control (native `disabled`, or `aria-disabled` without the primitives'
 * `data-focusable-when-disabled` mark) is never put back in the tab order;
 * one that is focusable-disabled by design (Button `pending` /
 * `focusableWhenDisabled`, the held Switch) is restored.
 */
function restoreTabOrder(event: React.FocusEvent<HTMLElement>) {
  if (event.relatedTarget !== null) return;
  const scope = event.currentTarget.closest<HTMLElement>("[data-base-ui-portal]") ?? event.currentTarget;
  for (const element of scope.querySelectorAll<HTMLElement>("[data-tabindex]")) {
    const tabindex = element.dataset.tabindex;
    delete element.dataset.tabindex;
    if (
      element.matches(":disabled, [disabled]") ||
      (element.getAttribute("aria-disabled") === "true" && !element.hasAttribute("data-focusable-when-disabled"))
    )
      continue;
    if (tabindex) element.setAttribute("tabindex", tabindex);
    else element.removeAttribute("tabindex");
  }
}

const TABBABLE =
  'a[href], button, input:not([type="hidden"]), select, textarea, [tabindex], [contenteditable="true"]';

/** Whether `element` takes a sequential Tab stop (Base UI's focus guards never count). */
function isTabStop(element: HTMLElement): boolean {
  if (element.tabIndex < 0 || element.matches(":disabled") || element.closest("[inert]")) return false;
  if (element.hasAttribute("data-base-ui-focus-guard")) return false;
  return typeof element.checkVisibility === "function" ? element.checkVisibility() : true;
}

function tabStopsIn(scope: ParentNode): HTMLElement[] {
  return [...scope.querySelectorAll<HTMLElement>(TABBABLE)].filter(isTabStop);
}

/** The first Tab stop after `anchor` in document order, outside `popup`'s portal and the anchor itself. */
function tabStopAfter(anchor: Element, popup: HTMLElement): HTMLElement | null {
  const portal = popup.closest("[data-base-ui-portal]") ?? popup;
  return (
    tabStopsIn(anchor.ownerDocument).find(
      (element) =>
        !portal.contains(element) &&
        !anchor.contains(element) &&
        (anchor.compareDocumentPosition(element) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0,
    ) ?? null
  );
}

type Anchor = PopoverPrimitive.Positioner.Props["anchor"];

function anchorElement(anchor: Anchor): HTMLElement | null {
  const resolved = typeof anchor === "function" ? anchor() : anchor;
  const element = resolved && "current" in resolved ? resolved.current : resolved;
  return element instanceof HTMLElement ? element : null;
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
  onKeyDown,
  finalFocus,
  ...props
}: Omit<PopoverPrimitive.Popup.Props, "className"> &
  Pick<
    PopoverPrimitive.Positioner.Props,
    "align" | "alignOffset" | "side" | "sideOffset" | "anchor"
  > & {
    /** Layout only (width). */
    className?: string;
  }) {
  const layer = React.useContext(PopoverLayerContext);
  const layerId = layer?.layerId ?? null;
  // Set while a Tab leaves a triggerless popover: focus has already moved on,
  // so the close returns none.
  const leavingRef = React.useRef(false);
  const popupRef = React.useCallback(
    (node: HTMLDivElement | null) => {
      if (node) leavingRef.current = false;
      if (layerId) setLayerPopup(layerId, node);
    },
    [layerId],
  );
  // A popover with no Trigger (positioned on `anchor`) gives Base UI's focus
  // guards no reference to hand focus to: a Tab out of it would land on an
  // unnamed guard and leave it open, still on the layer stack. So a Tab out
  // leaves it as it leaves a trigger's popover: it closes, forward to the Tab
  // stop after the anchor, backward to the anchor itself (#97).
  const triggerless = anchor !== undefined;
  const handleKeyDown = (event: Parameters<NonNullable<PopoverPrimitive.Popup.Props["onKeyDown"]>>[0]) => {
    onKeyDown?.(event);
    if (!triggerless || event.defaultPrevented || event.key !== "Tab") return;
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const popup = event.currentTarget;
    const stops = tabStopsIn(popup);
    const edge = event.shiftKey ? stops[0] : stops[stops.length - 1];
    if (edge && popup.ownerDocument.activeElement !== edge) return;
    const anchorEl = anchorElement(anchor);
    const target = anchorEl && (event.shiftKey ? anchorEl : (tabStopAfter(anchorEl, popup) ?? anchorEl));
    if (!target) return;
    event.preventDefault();
    leavingRef.current = true;
    layer?.actionsRef.current?.close();
    target.focus();
  };
  const resolvedFinalFocus: PopoverPrimitive.Popup.Props["finalFocus"] = triggerless
    ? (closeType) => {
        if (leavingRef.current) return false;
        if (typeof finalFocus === "function") return finalFocus(closeType);
        if (finalFocus && typeof finalFocus === "object") return finalFocus.current;
        return finalFocus ?? true;
      }
    : finalFocus;
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
          onKeyDown={handleKeyDown}
          finalFocus={resolvedFinalFocus}
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
