import * as React from "react";
import { Button as ButtonPrimitive } from "@base-ui/react/button";
import { cva, type VariantProps } from "class-variance-authority";
import { LoaderCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { heldRefusalHandlers } from "./held-refusal";
import { disabledState, focusRing, iconSlot, softGroundOnHover as hoverGround } from "./styles";

/**
 * Button — `variant` is visual weight, `tone` is semantic role, `size` is
 * density (DESIGN.md § Button). No transitions: hover and press change
 * colour instantly, and the focus ring never animates in. A held button
 * (aria-disabled) shows no hover ground: it takes pointer events only to say
 * why it is refused.
 */
const buttonVariants = cva(
  [
    "relative inline-flex shrink-0 cursor-default items-center justify-center gap-1.5 rounded-md border border-transparent font-medium whitespace-nowrap select-none",
    focusRing,
    disabledState,
    iconSlot,
  ],
  {
    variants: {
      variant: { solid: "", outline: "bg-transparent", ghost: "bg-transparent" },
      tone: { neutral: "", accent: "", ok: "", stale: "", danger: "" },
      size: {
        sm: "h-6 rounded-sm px-2 text-xs [&_svg:not([class*='size-'])]:size-3.5",
        md: "h-8 px-3 text-sm",
        lg: "h-9 px-4 text-sm",
        icon: "size-8",
        "icon-sm": "size-6 rounded-sm [&_svg:not([class*='size-'])]:size-3.5",
      },
    },
    compoundVariants: [
      // solid
      { variant: "solid", tone: "neutral", class: "bg-raised text-foreground not-aria-disabled:hover:bg-border-strong" },
      { variant: "solid", tone: "accent", class: "bg-primary text-primary-foreground not-aria-disabled:hover:bg-primary/90" },
      { variant: "solid", tone: "ok", class: "bg-ok text-ok-foreground not-aria-disabled:hover:bg-ok/90" },
      { variant: "solid", tone: "stale", class: "bg-stale text-stale-foreground not-aria-disabled:hover:bg-stale/90" },
      { variant: "solid", tone: "danger", class: "bg-destructive text-destructive-foreground not-aria-disabled:hover:bg-destructive/90" },
      // outline
      { variant: "outline", tone: "neutral", class: "border-border-strong text-foreground not-aria-disabled:hover:bg-accent" },
      { variant: "outline", tone: "accent", class: ["border-primary/60 text-ring", hoverGround.accent] },
      { variant: "outline", tone: "ok", class: ["border-ok/50 text-ok", hoverGround.ok] },
      { variant: "outline", tone: "stale", class: ["border-stale/50 text-stale", hoverGround.stale] },
      { variant: "outline", tone: "danger", class: ["border-destructive/50 text-destructive", hoverGround.failed] },
      // ghost
      { variant: "ghost", tone: "neutral", class: "text-muted-foreground not-aria-disabled:hover:bg-accent not-aria-disabled:hover:text-foreground" },
      { variant: "ghost", tone: "accent", class: ["text-ring", hoverGround.accent] },
      { variant: "ghost", tone: "ok", class: ["text-ok", hoverGround.ok] },
      { variant: "ghost", tone: "stale", class: ["text-stale", hoverGround.stale] },
      { variant: "ghost", tone: "danger", class: ["text-destructive", hoverGround.failed] },
    ],
    defaultVariants: { variant: "outline", tone: "neutral", size: "md" },
  },
);

type ButtonProps = Omit<ButtonPrimitive.Props, "className"> &
  VariantProps<typeof buttonVariants> & {
    /** Layout only (margin, flex/grid placement, width). Never colour or type. */
    className?: string;
    /**
     * Work started by this button is running: `aria-busy`, not activatable,
     * but it keeps focus (so focus never drops to <body> mid-action).
     * Like `disabled` + `focusableWhenDisabled`, the button is then *held*:
     * activating it (click, Space, Enter) does nothing and dispatches the
     * held-refusal event once, so the binder shows its reason
     * (`held-refusal.ts`, DESIGN.md § Button).
     */
    pending?: boolean;
  };

function Button({
  className,
  variant,
  tone,
  size,
  pending = false,
  disabled,
  focusableWhenDisabled,
  onClickCapture,
  onKeyDownCapture,
  children,
  ...props
}: ButtonProps) {
  const focusable = pending || focusableWhenDisabled === true;
  const held = focusable && (pending || disabled === true);
  const refusal = heldRefusalHandlers(held, { onClickCapture, onKeyDownCapture });
  return (
    <ButtonPrimitive
      data-slot="button"
      className={cn(buttonVariants({ variant, tone, size }), className)}
      disabled={disabled || pending}
      focusableWhenDisabled={focusable}
      data-focusable-when-disabled={focusable ? "" : undefined}
      aria-busy={pending || undefined}
      {...props}
      {...refusal}
    >
      {pending ? (
        <LoaderCircle aria-hidden="true" className="motion-safe:animate-spin" />
      ) : null}
      {children as React.ReactNode}
    </ButtonPrimitive>
  );
}

export { Button, type ButtonProps };
