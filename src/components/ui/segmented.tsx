import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { ToggleGroup as ToggleGroupPrimitive } from "@base-ui/react/toggle-group";
import { cn } from "@/lib/utils";
import { disabledState, focusRing, iconSlot } from "./styles";

/**
 * Segmented — a single choice among 2–5 options that does not switch panels
 * (Grouped / Flat, list / grid). A group of toggle buttons with
 * `aria-pressed`; arrow keys move between items (Base UI ToggleGroup).
 * Controlled with one value: `value` / `onValueChange` take a string.
 */
type SegmentedProps = Omit<
  ToggleGroupPrimitive.Props,
  "className" | "value" | "defaultValue" | "onValueChange" | "multiple"
> & {
  value: string;
  onValueChange: (value: string) => void;
  /** Accessible name of the group (what is being chosen). */
  "aria-label": string;
  /** Layout only. */
  className?: string;
};

function Segmented({ value, onValueChange, className, ...props }: SegmentedProps) {
  return (
    <ToggleGroupPrimitive
      data-slot="segmented"
      value={[value]}
      onValueChange={(next) => {
        // A segmented control always has a selection: ignore "unpress".
        if (next.length > 0) onValueChange(String(next[next.length - 1]));
      }}
      className={cn(
        "inline-flex h-8 w-fit items-center gap-0.5 rounded-md border border-border-strong p-0.5",
        className,
      )}
      {...props}
    />
  );
}

function SegmentedItem({
  className,
  ...props
}: Omit<TogglePrimitive.Props, "className"> & {
  value: string;
  /** Layout only. */
  className?: string;
}) {
  return (
    <TogglePrimitive
      data-slot="segmented-item"
      className={cn(
        "inline-flex h-6 cursor-default items-center justify-center gap-1.5 rounded-sm px-2.5 text-sm font-medium whitespace-nowrap text-muted-foreground select-none hover:text-foreground",
        // Selected: raised ground, foreground text and a 1 px iris edge — the
        // edge is the ≥ 3:1 state cue (iris marks selection).
        "aria-pressed:bg-raised aria-pressed:text-foreground aria-pressed:ring-1 aria-pressed:ring-ring aria-pressed:ring-inset",
        focusRing,
        disabledState,
        iconSlot,
        className,
      )}
      {...props}
    />
  );
}

export { Segmented, SegmentedItem, type SegmentedProps };
