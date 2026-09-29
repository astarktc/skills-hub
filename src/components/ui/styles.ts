/**
 * Class fragments every primitive shares. Not a styling surface: call sites
 * never import these — they choose a primitive's variant / tone / size.
 */

/**
 * The focus ring: a 2 px iris outline offset from the control, shown on
 * keyboard focus only, with no transition (the ring never animates in).
 * `--ring` measures ≥ 5.04:1 against every surface in both themes (DESIGN.md;
 * the floor is light `raised`).
 *
 * `focus-visible:outline-solid` is load-bearing. Tailwind 4's `outline-none`
 * sets `--tw-outline-style: none`, and `outline-2` draws
 * `outline-style: var(--tw-outline-style)` — without an explicit style in the
 * focus-visible state the ring computes to `none` and never draws.
 * `focus-ring.test.tsx` resolves every primitive's classes against the
 * compiled CSS to keep it that way. Literal strings, so Tailwind sees them.
 */
export const focusRing =
  "outline-none focus-visible:outline-2 focus-visible:outline-solid focus-visible:outline-offset-2 focus-visible:outline-ring";

/** The same ring, 1 px offset: text fields (a tighter ring reads as the field's edge). */
export const fieldFocusRing =
  "outline-none focus-visible:outline-2 focus-visible:outline-solid focus-visible:outline-offset-1 focus-visible:outline-ring";

/**
 * The same ring drawn inside the element: menu items, whose popup scrolls
 * (`overflow-y-auto`) and would clip an outer ring. Keyboard highlight moves
 * DOM focus to the item, so the ring marks it; a pointer highlight shows the
 * ground only.
 */
export const insetFocusRing =
  "outline-none focus-visible:outline-2 focus-visible:outline-solid focus-visible:-outline-offset-2 focus-visible:outline-ring";

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
