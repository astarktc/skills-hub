import { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip";
import { cn } from "@/lib/utils";

/**
 * Tooltip — a supplementary hint on hover or keyboard focus; never the only
 * carrier of a name or a state (give the trigger its own accessible name).
 * Inverted surface (foreground ground, background text). Appears instantly
 * after the provider's delay; Escape dismisses it.
 */
const TooltipProvider = TooltipPrimitive.Provider;
const Tooltip = TooltipPrimitive.Root;
const TooltipTrigger = TooltipPrimitive.Trigger;

function TooltipContent({
  className,
  side = "top",
  sideOffset = 6,
  align = "center",
  ...props
}: Omit<TooltipPrimitive.Popup.Props, "className"> &
  Pick<TooltipPrimitive.Positioner.Props, "align" | "side" | "sideOffset"> & {
    /** Layout only. */
    className?: string;
  }) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Positioner
        side={side}
        sideOffset={sideOffset}
        align={align}
        className="z-50"
      >
        <TooltipPrimitive.Popup
          data-slot="tooltip-content"
          className={cn(
            "max-w-xs rounded-sm bg-foreground px-2 py-1 text-xs text-background shadow-sm",
            className,
          )}
          {...props}
        />
      </TooltipPrimitive.Positioner>
    </TooltipPrimitive.Portal>
  );
}

export { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger };
