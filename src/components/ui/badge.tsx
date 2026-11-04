import * as React from "react";
import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";
import {
  disabledState,
  focusRing,
  iconSlot,
  identityGround,
  softGround,
  type IdentityHue,
} from "./styles";

/**
 * Badge — a static label (a count, a state word). Words carry the meaning;
 * colour only reinforces it (never colour-only). `tone` is the semantic
 * role, `variant` the weight, `size` the density (DESIGN.md § Badge).
 *
 * `tone="identity"` is the source chip (ticket 30): the `hue`'s tint with the
 * hue's text, always `soft` and always `chip`-sized — `outline` / `solid`
 * identity is a type error. The hue comes from a stable key
 * (`skillPresentation.identityHue`), never from state, and the chip only
 * supplements the name beside it.
 */
const badgeVariants = cva(
  [
    "inline-flex w-fit shrink-0 items-center gap-1 rounded-sm border border-transparent font-medium whitespace-nowrap",
    "[&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg]:size-3",
  ],
  {
    variants: {
      variant: { soft: "", outline: "bg-transparent", solid: "" },
      tone: { neutral: "", accent: "", ok: "", stale: "", failed: "", identity: "" },
      size: {
        sm: "h-5 px-1.5 text-xs",
        md: "h-6 px-2 text-xs",
        /* 22 px square, two bold initials (or one icon): the source chip */
        chip: "size-5.5 justify-center rounded-md px-0 text-2xs font-bold",
      },
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

type BadgeVariants = VariantProps<typeof badgeVariants>;

type BadgeCommonProps = Omit<React.ComponentProps<"span">, "className"> & {
  /** Layout only. */
  className?: string;
};

/** A state or count label: any variant × state tone × size. */
type StateBadgeProps = BadgeCommonProps &
  Omit<BadgeVariants, "tone"> & {
    tone?: Exclude<BadgeVariants["tone"], "identity">;
    hue?: never;
  };

/** The source chip: one identity hue, soft, chip-sized. */
type IdentityBadgeProps = BadgeCommonProps & {
  tone: "identity";
  hue: IdentityHue;
  variant?: "soft";
  size?: "chip";
};

type BadgeProps = StateBadgeProps | IdentityBadgeProps;

function Badge({ variant, tone, size, hue, className, ...props }: BadgeProps) {
  const identity = tone === "identity" && hue ? identityGround[hue] : null;
  return (
    <span
      data-slot="badge"
      className={cn(
        badgeVariants({ variant, tone, size: identity ? (size ?? "chip") : size }),
        identity,
        className,
      )}
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

export { Badge, Chip, type BadgeProps, type ChipProps, type IdentityBadgeProps };
