import * as React from "react";
import { Popover as PopoverPrimitive } from "@base-ui/react/popover";
import { cn } from "@/lib/utils";
import { floatingSurface } from "./styles";

/**
 * Popover — non-modal floating panel anchored to its trigger (Base UI:
 * collision-aware, origin-aware, Escape closes it and returns focus to the
 * trigger). Appears instantly. Pass the trigger through `render`:
 * `<PopoverTrigger render={<Button />}>…</PopoverTrigger>`. A popover a
 * control raises on its own terms (a row switch's inline confirmation) has no
 * Trigger: it is controlled (`open`), positioned on `anchor`, and names
 * where focus goes in (`initialFocus`) and back (`finalFocus`).
 */
const Popover = PopoverPrimitive.Root;
const PopoverTrigger = PopoverPrimitive.Trigger;
const PopoverClose = PopoverPrimitive.Close;

function PopoverContent({
  className,
  align = "center",
  alignOffset = 0,
  side = "bottom",
  sideOffset = 4,
  anchor,
  ...props
}: Omit<PopoverPrimitive.Popup.Props, "className"> &
  Pick<
    PopoverPrimitive.Positioner.Props,
    "align" | "alignOffset" | "side" | "sideOffset" | "anchor"
  > & {
    /** Layout only (width). */
    className?: string;
  }) {
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
          className={cn(
            "flex w-72 flex-col gap-2 p-3 text-sm",
            floatingSurface,
            className,
          )}
          {...props}
        />
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
