/**
 * Class fragments every primitive shares. Not a styling surface: call sites
 * never import these — they choose a primitive's variant / tone / size.
 */

/**
 * The focus ring: a 2 px iris outline offset from the control, shown on
 * keyboard focus only, with no transition (the ring never animates in).
 * `--ring` measures ≥ 5.7:1 against every surface in both themes (DESIGN.md).
 */
export const focusRing =
  "outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring";

/** Disabled (native or aria-disabled) and pending (aria-busy) presentation. */
export const disabledState =
  "disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 aria-busy:cursor-progress";

/** Icons inside a control size themselves from the control, never per call site. */
export const iconSlot =
  "[&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

/**
 * Floating layers (popover, menu, tooltip) appear and leave instantly: the
 * world moves only the palette, the bulk rail and the switch (≤ 120 ms).
 */
export const floatingSurface =
  "rounded-md border border-border bg-popover text-popover-foreground shadow-lg outline-none";

/**
 * Soft state grounds: 10 % of the state colour mixed into the popover
 * surface — opaque, so the state-coloured text on it measures the same on
 * every surface (a translucent tint over `raised` drops light-theme text
 * below 4.5:1). Literal strings so Tailwind generates them.
 */
export const softGround = {
  accent: "bg-[color-mix(in_oklab,var(--primary)_10%,var(--popover))]",
  ok: "bg-[color-mix(in_oklab,var(--ok)_10%,var(--popover))]",
  stale: "bg-[color-mix(in_oklab,var(--stale)_10%,var(--popover))]",
  failed: "bg-[color-mix(in_oklab,var(--failed)_10%,var(--popover))]",
} as const;

export const softGroundOnHover = {
  accent: "hover:bg-[color-mix(in_oklab,var(--primary)_10%,var(--popover))]",
  ok: "hover:bg-[color-mix(in_oklab,var(--ok)_10%,var(--popover))]",
  stale: "hover:bg-[color-mix(in_oklab,var(--stale)_10%,var(--popover))]",
  failed: "hover:bg-[color-mix(in_oklab,var(--failed)_10%,var(--popover))]",
} as const;
