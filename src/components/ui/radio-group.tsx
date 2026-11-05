import { Radio as RadioPrimitive } from "@base-ui/react/radio";
import { RadioGroup as RadioGroupPrimitive } from "@base-ui/react/radio-group";
import { cn } from "@/lib/utils";
import { disabledState, focusRing, iconSlot } from "./styles";

/**
 * RadioGroup — one choice of a saved setting among 2–5 short options
 * (Settings → Appearance's theme). `role="radiogroup"` named by
 * `aria-label` / `aria-labelledby`; each item a `role="radio"` with
 * `aria-checked`, named by its content. One Tab stop (the checked item);
 * arrow keys move **and select** (Base UI Radio). It looks like Segmented
 * (a row of segments, the checked one `raised` with a 1 px iris inset edge
 * — the edge is the ≥ 3:1 state cue), but says what it is: Segmented is a
 * view switch of `aria-pressed` toggles, this is a form choice.
 * Controlled with one string value.
 */
type RadioGroupProps = Omit<
  RadioGroupPrimitive.Props<string>,
  "className" | "value" | "defaultValue" | "onValueChange"
> & {
  value: string;
  onValueChange: (value: string) => void;
  /** Layout only. */
  className?: string;
};

function RadioGroup({ value, onValueChange, className, ...props }: RadioGroupProps) {
  return (
    <RadioGroupPrimitive
      data-slot="radio-group"
      value={value}
      onValueChange={(next) => onValueChange(String(next))}
      className={cn(
        "inline-flex h-8 w-fit items-center gap-0.5 rounded-md border border-border-strong p-0.5",
        className,
      )}
      {...props}
    />
  );
}

function RadioGroupItem({
  className,
  ...props
}: Omit<RadioPrimitive.Root.Props<string>, "className"> & {
  value: string;
  /** Layout only. */
  className?: string;
}) {
  return (
    <RadioPrimitive.Root
      data-slot="radio-group-item"
      className={cn(
        "inline-flex h-6 cursor-default items-center justify-center gap-1.5 rounded-sm px-2.5 text-sm font-medium whitespace-nowrap text-muted-foreground select-none hover:text-foreground",
        "data-checked:bg-raised data-checked:text-foreground data-checked:ring-1 data-checked:ring-ring data-checked:ring-inset",
        focusRing,
        disabledState,
        "data-disabled:opacity-50",
        iconSlot,
        className,
      )}
      {...props}
    />
  );
}

export { RadioGroup, RadioGroupItem, type RadioGroupProps };
