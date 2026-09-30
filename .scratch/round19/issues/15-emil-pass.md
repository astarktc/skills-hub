# 15 — Emil pass on primitives + My Skills (motion, details), with fixes

Status: ready-for-agent
Blocked by: 11, 12, 13, 14
Spec: `.scratch/round19/spec.md` — D32 order; round-18 D6. Skills: `emil-design-eng`, `review-animations` (via
`animate`'s references), `impeccable` `polish`.

## Work

- Review, on the fixture in `preview_*`, every interaction My Skills now has: row focus, selection, strip lighting on
  sync completion (≤120 ms), bulk rail enter/exit, switch, inspector open/close and sheet, dialogs, toasts, header
  progress, hover/focus states of every primitive in both themes. Rubric: Emil's checklist (frequency-first: frequent
  actions near-invisible; no `transition: all`; transform/opacity only; durations ≤120 ms except the two permitted
  rare moments; interruptible; reduced motion removes); the contract's "practical, not flashy".
- Fix in place: primitives (`src/components/ui/`) and My Skills files; every change is a token/variant change or a
  primitive-owned transition — never a call-site override. Record each Before/After in the Result.
- Details pass: alignment on the 4 px grid, tabular numerals in counts, optical alignment of marks, hit areas,
  truncation rules (names never clipped mid-word without a title), empty-state copy, capitalisation consistency.
- `DESIGN.md` gains the motion table (what moves, duration, easing, reduce behaviour).

## Gate

Contrast table still holds; `npm run check`; a short `preview_recording` of the signature interactions saved under
`.scratch/round19/evidence/` (local-only) and referenced in the Result.

## Result

(child fills in)

## Comments

- 2026-09-30 — from ticket 14: motion added — the header progress's bar fill (`scaleX`, `--motion-fast`,
  `motion-reduce:transition-none`) and its spinner (`motion-safe:animate-spin`, `motion-reduce:hidden`); the row's
  pending mark is the overflow Button's `pending` spinner. The LoadingOverlay's spinner/shimmer/fade are gone
  (`shimmer` keyframes stay for `.skeleton-row`, `spin` for `.spinner`). Worth an Emil look: the header's
  summary ↔ progress swap is instant (no crossfade), the run toast is headline-only with "View report", and the
  Switch primitive has no `focusableWhenDisabled` (a pressed row switch drops focus while its action runs; the door
  restores it after).

