import { Switch as SwitchPrimitive } from "@base-ui/react/switch";
import { cn } from "@/lib/utils";
import { disabledState, focusRing } from "./styles";

type SwitchProps = Omit<SwitchPrimitive.Root.Props, "className"> & {
  /** `md` 20 × 36 (standalone), `sm` 16 × 28 (in a dense row; 24 px hit area). */
  size?: "sm" | "md";
  /**
   * While `disabled`, keep the switch in the tab order and holding focus
   * (`aria-disabled`, not activatable) — for a switch disabled *because* its
   * own action is running, so focus never leaves it (D21). Base UI's Switch
   * has no such option (1.8), so the primitive renders a disabled switch it
   * keeps focusable as read-only (Base UI refuses every change) and says
   * `aria-disabled` instead of `aria-readonly`.
   */
  focusableWhenDisabled?: boolean;
  /** Layout only. */
  className?: string;
};

/**
 * Switch — `role="switch"` with `aria-checked`. "On" is neutral
 * (muted-foreground), not iris: a column of switches must not make the
 * accent mean "normal". The thumb slides in 120 ms (`--motion-fast`,
 * `ease-out`, transform only); reduced motion removes the slide. Off-state
 * edge is `input` (≥ 3:1).
 */
function Switch({
  size = "md",
  className,
  disabled = false,
  focusableWhenDisabled = false,
  readOnly,
  ...props
}: SwitchProps) {
  const heldFocus = disabled && focusableWhenDisabled;
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      data-size={size}
      className={cn(
        "group/switch peer relative inline-flex shrink-0 cursor-default items-center rounded-full border border-input bg-raised p-px",
        "after:absolute after:-inset-y-1 after:-inset-x-0.5 after:content-['']",
        "data-[size=md]:h-5 data-[size=md]:w-9 data-[size=sm]:h-4 data-[size=sm]:w-7",
        "data-checked:border-muted-foreground data-checked:bg-muted-foreground",
        focusRing,
        disabledState,
        "data-disabled:opacity-50",
        className,
      )}
      disabled={heldFocus ? false : disabled}
      readOnly={heldFocus || readOnly}
      {...props}
      {...(heldFocus ? { "aria-disabled": true, "aria-readonly": undefined } : null)}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className={cn(
          "pointer-events-none block rounded-full bg-muted-foreground",
          "transition-transform duration-(--motion-fast) ease-out motion-reduce:transition-none",
          "group-data-[size=md]/switch:size-4 group-data-[size=sm]/switch:size-3",
          "data-checked:bg-background group-data-[size=md]/switch:data-checked:translate-x-4 group-data-[size=sm]/switch:data-checked:translate-x-3",
        )}
      />
    </SwitchPrimitive.Root>
  );
}

export { Switch, type SwitchProps };
