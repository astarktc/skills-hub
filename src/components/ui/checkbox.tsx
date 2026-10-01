import { Checkbox as CheckboxPrimitive } from "@base-ui/react/checkbox";
import { Check, Minus } from "lucide-react";
import { cn } from "@/lib/utils";
import { useFieldControlProps } from "./field-context";
import { disabledState, focusRing } from "./styles";

type CheckboxProps = Omit<CheckboxPrimitive.Root.Props, "className"> & {
  /**
   * While `disabled`, keep the checkbox in the tab order and holding focus
   * (`aria-disabled`, not activatable) — for a checkbox disabled *because*
   * its own action is running (a matrix cell mid-toggle), so focus never
   * leaves it (D21). Base UI's Checkbox has no such option, so — like the
   * Switch — the primitive renders it read-only (Base UI refuses every
   * change) and says `aria-disabled` instead of `aria-readonly`.
   */
  focusableWhenDisabled?: boolean;
  /** Layout only. */
  className?: string;
};

/**
 * Checkbox — `role="checkbox"` with `aria-checked` (true / false / mixed).
 * A 16 px box with a 24 × 24 hit area (the dense target floor); the unchecked
 * edge is `input` (≥ 3:1 on every surface), checked fills with iris. Name it
 * with a wrapping `<label>`, `aria-label`, or a Field label: inside a Field
 * the checkbox renders the Field's control id on its form input, the
 * FieldLabel points at it, and Base UI names the `role="checkbox"` element
 * from that label (`aria-labelledby`).
 */
function Checkbox({ className, disabled = false, focusableWhenDisabled = false, readOnly, ...rest }: CheckboxProps) {
  const props = useFieldControlProps(rest);
  const heldFocus = disabled && focusableWhenDisabled;
  return (
    <CheckboxPrimitive.Root
      data-slot="checkbox"
      className={cn(
        "peer relative inline-flex size-4 shrink-0 cursor-default items-center justify-center rounded-sm border border-input bg-transparent text-primary-foreground",
        "after:absolute after:-inset-1 after:content-['']",
        "data-checked:border-primary data-checked:bg-primary data-indeterminate:border-primary data-indeterminate:bg-primary",
        "aria-invalid:border-destructive",
        focusRing,
        disabledState,
        "data-disabled:opacity-50",
        className,
      )}
      disabled={heldFocus ? false : disabled}
      readOnly={heldFocus || readOnly}
      data-focusable-when-disabled={heldFocus ? "" : undefined}
      {...props}
      {...(heldFocus ? { "aria-disabled": true, "aria-readonly": undefined } : null)}
    >
      <CheckboxPrimitive.Indicator
        data-slot="checkbox-indicator"
        className="flex items-center justify-center [&_svg]:size-3.5"
        render={(indicatorProps, state) => (
          <span {...indicatorProps}>
            {state.indeterminate ? (
              <Minus aria-hidden="true" strokeWidth={3} />
            ) : (
              <Check aria-hidden="true" strokeWidth={3} />
            )}
          </span>
        )}
      />
    </CheckboxPrimitive.Root>
  );
}

export { Checkbox, type CheckboxProps };
