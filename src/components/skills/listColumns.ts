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
