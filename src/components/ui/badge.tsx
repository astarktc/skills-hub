import * as React from "react";
import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";
import { disabledState, focusRing, iconSlot, softGround } from "./styles";

/**
 * Badge — a static label (a count, a state word). Words carry the meaning;
 * colour only reinforces it (never colour-only). `tone` is the semantic
 * role, `variant` the weight, `size` the density (DESIGN.md § Badge).
 */
const badgeVariants = cva(
  [
    "inline-flex w-fit shrink-0 items-center gap-1 rounded-sm border border-transparent font-medium whitespace-nowrap",
    "[&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg]:size-3",
  ],
  {
    variants: {
      variant: { soft: "", outline: "bg-transparent", solid: "" },
      tone: { neutral: "", accent: "", ok: "", stale: "", failed: "" },
      size: { sm: "h-5 px-1.5 text-xs", md: "h-6 px-2 text-xs" },
    },
    compoundVariants: [
      { variant: "soft", tone: "neutral", class: "bg-muted text-muted-foreground" },
      { variant: "soft", tone: "accent", class: [softGround.accent, "text-ring"] },
      { variant: "soft", tone: "ok", class: [softGround.ok, "text-ok"] },
      { variant: "soft", tone: "stale", class: [softGround.stale, "text-stale"] },
      { variant: "soft", tone: "failed", class: [softGround.failed, "text-failed"] },
      { variant: "outline", tone: "neutral", class: "border-border-strong text-muted-foreground" },
      { variant: "outline", tone: "accent", class: "border-primary/60 text-ring" },
      { variant: "outline", tone: "ok", class: "border-ok/50 text-ok" },
      { variant: "outline", tone: "stale", class: "border-stale/50 text-stale" },
      { variant: "outline", tone: "failed", class: "border-failed/50 text-failed" },
      { variant: "solid", tone: "neutral", class: "bg-raised text-foreground" },
      { variant: "solid", tone: "accent", class: "bg-primary text-primary-foreground" },
      { variant: "solid", tone: "ok", class: "bg-ok text-ok-foreground" },
      { variant: "solid", tone: "stale", class: "bg-stale text-stale-foreground" },
      { variant: "solid", tone: "failed", class: "bg-failed text-failed-foreground" },
    ],
    defaultVariants: { variant: "soft", tone: "neutral", size: "sm" },
  },
);

type BadgeProps = Omit<React.ComponentProps<"span">, "className"> &
  VariantProps<typeof badgeVariants> & {
    /** Layout only. */
    className?: string;
  };

function Badge({ variant, tone, size, className, ...props }: BadgeProps) {
  return (
    <span
      data-slot="badge"
      className={cn(badgeVariants({ variant, tone, size }), className)}
      {...props}
    />
  );
}

/**
 * Chip — a pressable filter (All · Issues · Edited …): a toggle button with
 * `aria-pressed`. Pressed: raised ground, foreground text and an iris edge
 * (the ≥ 3:1 state cue; iris marks selection). 24 px tall (the dense floor).
 */
const chipVariants = cva(
  [
    "inline-flex h-6 shrink-0 cursor-default items-center gap-1.5 rounded-md border px-2 text-xs font-medium whitespace-nowrap select-none",
    "border-border-strong text-muted-foreground hover:text-foreground",
    "aria-pressed:border-ring aria-pressed:bg-raised aria-pressed:text-foreground",
    focusRing,
    disabledState,
    iconSlot,
  ],
);

type ChipProps = Omit<TogglePrimitive.Props, "className"> & {
  /** Layout only. */
  className?: string;
};

function Chip({ className, ...props }: ChipProps) {
  return (
    <TogglePrimitive
      data-slot="chip"
      className={cn(chipVariants(), className)}
      {...props}
    />
  );
}

export { Badge, Chip, type BadgeProps, type ChipProps };
