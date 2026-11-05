/**
 * The My Skills list's one column template (DESIGN.md § My Skills row), shared
 * by the row and its group header so the header's harness initials sit over
 * the strip's marks: checkbox 32 · name `--name-col` · description 1fr ·
 * strip (`--strip-marks` × 20 px + N/M 40) · scope 56 · switch 40 · overflow 32.
 * The list publishes `--name-col` (the longest visible name + marks, clamped
 * to [12rem, 22rem], D39) and `--strip-marks` (the widest strip in view).
 * Below a 48rem list (the inspector beside it) the description goes and the
 * name takes its room.
 */
export const LIST_COLUMNS = [
  "grid-cols-[2rem_var(--name-col)_minmax(0,1fr)_calc(var(--strip-marks)_*_1.25rem_+_2.5rem)_3.5rem_2.5rem_2rem]",
  "@max-3xl:grid-cols-[2rem_minmax(0,1fr)_calc(var(--strip-marks)_*_1.25rem_+_2.5rem)_3.5rem_2.5rem_2rem]",
].join(" ");

/**
 * The grid's two densities (ticket 24): `auto-fill` columns no narrower than
 * the density's floor — `auto` 19rem (304 px), `dense` 15rem (240 px) — nor
 * than the widest strip in view plus the card's padding and border
 * (`--strip-marks` × 20 px + N/M 40 + 26 px), so a card never clips its
 * marks; never wider than the pane (`min(100%, …)`). Gaps and the group
 * card's inset are on the 4 px scale: 12 px in `auto`, 8 px in `dense`.
 * Cards fill row-major, so their reading order is the DOM's.
 */
export const GRID_COLUMNS = {
  "auto-grid":
    "gap-3 p-3 grid-cols-[repeat(auto-fill,minmax(min(100%,max(19rem,calc(var(--strip-marks)_*_1.25rem_+_4.25rem))),1fr))]",
  "dense-grid":
    "gap-2 p-2 grid-cols-[repeat(auto-fill,minmax(min(100%,max(15rem,calc(var(--strip-marks)_*_1.25rem_+_4.25rem))),1fr))]",
} as const;
