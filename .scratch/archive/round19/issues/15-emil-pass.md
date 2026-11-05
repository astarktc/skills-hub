# 15 — Emil pass on primitives + My Skills (motion, details), with fixes

Status: done — be367c8
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

Implemented 2026-09-30 (child, Opus 5.5). Nothing committed. Evidence: `.scratch/round19/evidence/15/` (local-only;
drivers `motion.ts` · `fixes.ts` · `a11y.ts` · `ring.ts` · `details.ts` · `settle.ts` · `zoom.ts`, the 13/14 keyboard
transcripts re-run as `gate.ts` / `flows.ts`, keys through `keys-cdp.ts`; `summarize.mjs` prints the motion runs).
Rubric: Emil's checklist (frequency first; transform/opacity only; ≤ 120 ms; interruptible; reduced motion removes),
`review-animations`' ten standards, impeccable `polish`, the contract's "practical, not flashy".

### Before / After (every change is a primitive, a token, or the owning component — no call-site override)

| Before | After | Why |
|---|---|---|
| The contract's signature interaction — "the Synced strip lighting on sync completion (≤ 120 ms)" — did not exist: a mark changed glyph with no transition | `StripMarkIcon` keys its glyph by state; a mark that **becomes** synced on screen (`useSyncLight`, a pure-state hook + 9 tests) enters through `@starting-style` opacity 0 → 1, `transition-opacity duration-(--motion-fast) ease-out motion-reduce:transition-none`; never on mount, filter, scroll or inspector switch | State indication at sync completion; frequency first — navigation never animates; opacity only; transitions retarget. The ticket-21 cascade now only adds a per-mark delay |
| Toasts: sonner's own `transition: transform .4s, opacity .4s, height .4s, box-shadow .2s`, 400 ms content fade, 400 ms toaster lift, 200 ms swipe-out | `ui/sonner.tsx` owns the world's motion: inline `transition: transform, opacity 120 ms ease-out` on each toast (+ `animation-duration` for the swipe-out) and on the toaster, `*:duration-(--motion-fast)! *:ease-out!` for the stacked content fade; no height glide | Sub-300 ms and GPU-only; the toast still enters from and leaves through its edge (spatial consistency). Inline beats sonner's injected CSS; sonner's own `transition:none!important` and the base layer still win under reduce |
| `App.css` carried 17 `transition: all 0.2s/0.15s` and 3 colour/opacity transitions (Add, Explore, Projects, Settings legacy rules) | All 20 removed (hover instant, as in every primitive); the legacy settings toggle knob re-timed to `transform var(--motion-fast) var(--ease-out)`; ceiling **1882 → 1862** | "No `transition: all`" holds for the built CSS; delete is Emil's first remedy for tens-a-day hover motion |
| Row switch: `disabled={busy}` — Base UI 1.8's Switch has no `focusableWhenDisabled`; a pressed switch went `tabindex=-1` and left the tab order while its action ran, with no reason | `Switch` gains `focusableWhenDisabled` (held focus = Base UI `readOnly` + `aria-disabled="true"`, never `aria-readonly`); the row switch uses it with the reason as `title` + `aria-description` | D21 "focus never leaves the control"; D19 "refused with the reason". 3 new primitive tests (`checkbox.test.tsx`), fail on the old primitive |
| A focused control held while disabled (rail, header verbs, row switch, `pending` Buttons) drew at 50 % opacity **ring included**: 2.60:1 dark / 2.19:1 light | `disabledState` adds `aria-disabled:focus-visible:opacity-75`: ring **4.42 dark / 3.49 light** (measured, `ring.ts`), the control still reads dimmed | The focus-state review of every primitive found it; a keyboard user's ring must stay ≥ 3:1. Button test pins the class |
| N2: the inspector's side ↔ sheet reparent (resize) went through one or two frames with focus on `<body>` (A/B: 1 frame on the Change-source close, 2 each way on a plain resize) | `App.tsx`'s placement change is `reparentWithFocus`: `flushSync` the new placement, then focus its heading in the same task | Every frame sampled (`rAF` probe): **0 BODY frames** in all three transitions, both themes; the A/B run without the fix shows 1 / 2 / 2 (`n2-ab-without-fix.json`) |
| N4: a note recorded while the Notifications panel is open ("Nothing to overwrite…", a run started from it) was silent to a screen reader | The panel carries a polite `role="status"` (`data-run-arrival`, `aria-atomic`) that reads `arrivalAnnouncement` — the newest group's title once it is no longer the one the panel opened on (pure fold + 4 tests) | Measured: the region is empty at open, then reads "Enabled and synced." and "Nothing to overwrite: retired-playbook is already deployed to Claude Code."; focus stays on Overwrite… (0/300/2000 ms) |
| N5: with the panel open mid-run, progress drew twice (inert header under the scrim + the panel foot) | `HeaderProgress yielded` — the header (or bar) copy draws nothing while a dialog whose foot owns the progress is open (`progressInDialog` in App: Add, Import, the pickers, Re-point, Notifications); its live region stays | Measured: header only → panel foot only → header only again (both themes) |
| Header summary ↔ progress swap: instant, flagged by 14 | **Decided: stays instant** (DESIGN.md § Motion "What does not move") | A crossfade would show two statements at once, at the start and end of every run |
| Row menu's "Remove" opened a dialog without the ellipsis its siblings carry (Change source…, Edit invocation…; rail "Remove…", inspector "Remove skill…") | `rowMenu.remove` "Remove…" (appended at the end of `en`) | Capitalisation / punctuation consistency: an item that asks first says so |
| Truncated group name, group state summary and inspector source link had no title | `title` on each | "Names never clipped without a title" — measured: 0 clipped elements without a title in My Skills (`details.ts`) |
| Row name button 20 px tall, "tdd" 20.9 px wide; group-header and inspector-source links 20 px tall | `h-6 min-w-6 leading-6` on the name; `h-6` on the two links | D28's 24 px floor — measured: **0 controls under 24 × 24** in My Skills + inspector (Checkbox/Switch `::after` hit areas counted) |
| Notifications item rows `py-2.5` (10 px, off the 4 px grid) | `py-2` | 4 px rhythm (run rows stay `py-3`) |
| DESIGN.md: "Filter bar (40 px)", "Header row (48 px)"; one-paragraph Motion | Measured and corrected: header 48 + hairline, filter bar 48 + hairline (8 px around its 32 px controls); bars are 4 px multiples + hairline, rows 36 incl. theirs. **§ Motion rewritten** with the motion table, what does not move, the two rare moments, reduce; Switch, Toaster, Focus ring, My Skills row, header progress, panel, inspector focus notes | Documentation of every primitive change (ADR-0006 / AGENTS.md invariant) |

Considered and rejected (Emil's rubric applied to the contract): press scale on Buttons (frequent; the world changes
colour instantly — "practical, not flashy"); inspector / sheet / dialog / menu / popover motion (every row click opens
the inspector; modals and menus stay instant); a colour transition on the switch track (transform only, 120 ms makes
the snap imperceptible); a faster spinner period (1 s kept; loops are not the review's target). The rail's entry
motion (keyboard `x`) is kept: the contract names it, it fires once per selection (not per keystroke), is interactive
from its first frame, and leaves instantly.

### The motion table (DESIGN.md § Motion — pointer)

Switch thumb (translate) · bulk rail entry (opacity + translate, entry only) · Synced strip light (opacity, on sync
completion) · header progress fill (scaleX) · toast (transform + opacity) — each **120 ms `ease-out`**
(`cubic-bezier(0.16, 1, 0.3, 1)`), each with its reduce counterpart; spinners are the only loops (1 s `animate-spin`;
legacy 0.9 s / 0.6 s / 1.5 s in `App.css`), stopped under reduce. ⌘K palette row reserved (ticket 23). The two
permitted rare moments (a large import completing, an install landing) are sequences of ≤ 120 ms steps, not built yet
(tickets 21, 16). Instant by decision: row focus / selection / hover, inspector + sheet, dialogs / popovers / menus /
tooltips, summary ↔ progress swap, chevrons, Files expand, list ↔ grid.

### Verification (fresh, fixture 5175 `failures`; dark 1280×800 and light 960×640)

- **Measured motion** (`motion-{dark,light}[-reduce]-output.txt`, rAF samples + `document.getAnimations()`): switch
  thumb `translate` 120 ms `cubic-bezier(0.16,1,0.3,1)` (change → settled 117 ms); strip light `opacity` 120 ms
  (0 → 1 in 117 ms of frames); bulk rail `opacity` + `translate` 120 ms (0 / 8 px → 1 / none in 116–117 ms), exit
  instant (gone the next frame); toast `transform` + `opacity` 120 ms each; header progress fill `transform` 120 ms;
  inspector (aside and sheet), row menu and Remove dialog: settled on their first frame, **no animation**. After a
  Refresh 20 marks lit and **0 transitions still running 600 ms later** (`settle.ts`). **Reduced motion emulated**:
  every one of them settled on its first sample, no transition listed, header spinner `display:none`, 0 running.
- **Built CSS** (`dist/assets/index-*.css`): **0 `transition: all`**; every transition duration is
  `var(--motion-fast)` / `--default-transition-duration` = **.12s**; the only durations over 120 ms are the loops
  `--animate-spin: spin 1s linear infinite` (used only as `motion-safe:animate-spin`), `.spinner` .9s, `.cell-spinner`
  .6s, `.skeleton-row` shimmer 1.5s; `@keyframes` are `spin` and `shimmer` only, both zeroed by the base layer's
  `@media (prefers-reduced-motion: reduce)` (`animation-duration: 0s !important; animation-iteration-count: 1`), plus
  `motion-reduce:transition-none` / `motion-reduce:hidden` on each moving element. (sonner's CSS is injected at
  runtime, not in the bundle; its durations are overridden inline as measured above.)
- **Fixes** (`fixes-*.json`): switch — its own Disable: the switch holds focus, `aria-disabled="true"`, `tabindex=0`
  for the whole pending window (80 → 1040 ms of frames), 0 BODY; refused mid-Refresh: focusable, `aria-disabled`,
  "Refresh is running — wait for it to finish" as title + description, Space and Enter change nothing, no popover;
  settled → enabled again, focus kept — **ok both themes**. N2 **ok** both (0 / 0 / 0 BODY frames). N5 **ok** both. N4
  **ok** both.
- **Keyboard matrix**: ticket 13's gate transcript (select ×3 → rail Disable → Refresh keeps them disabled → Enable
  → select-all counts = hand count 25 / 8 / 80 / 12 / 1): **90 stops each at dark 1280 and light 960, 0 BODY, every
  stop `:focus-visible`, `solid 2px`**. Ticket 14's flows (bulk Disable, bulk Remove, late navigation, resize with a
  child dialog, dismissed ask): 15 + 158 stops per configuration, **0 BODY**, all visible.
- **axe 4.10.3, 6 runs** (list + inspector/sheet after a strip light, panel with its progress foot mid-Refresh, the
  refused switch mid-Refresh; × dark 1280 / light 960): **0 violations**; incompletes only the known Base UI
  `aria-hidden-focus` and `color-contrast` on disabled (exempt) controls.
- **Contrast** (re-measured, rendered, canvas-composited; no colour token changed): My Skills text min **4.68 dark**
  (white on primary) / **4.89 light** (failed badge) — the audits' floors, unchanged; strip marks dark / light:
  synced 8.31–7.08 / 8.16, not synced 3.90–3.32 / 3.78, failed 6.91–5.89 / 5.50, disabled 6.12 / 6.09, a lit mark
  settled 8.31 / 8.16 (= synced); panel min 7.78 / 7.60; toast (warning) 8.62 / 5.13 (= DESIGN's 8 % table); **the
  one changed pair**: the ring of a focused control held disabled, **2.60 → 4.42 dark, 2.19 → 3.49 light**.
- **Details** (`details-*.json`): 59 counts checked, 0 without tabular numerals; 0 controls under 24 × 24; 0
  truncations without a title; row 36, header / filter bar 48 + hairline, banner / rail 40 + hairline.
- **Recording**: `.scratch/round19/evidence/15/signature-interactions.mp4` (T3 `preview_recording`, 44 s, 2560×1600)
  — row focus by `j`, `x` selection with the rail rising, Esc clearing it, the inspector opening, the switch off /
  on, the strip lighting on Enable. The hidden delegated tab records at ~1 fps, so a 120 ms motion is not visible
  frame by frame in it; the rAF measurements above are the motion proof, and the preview client disconnected before
  the Refresh / toast / panel segment — those are stills instead: `a11y-refused-*.png` (header progress),
  `a11y-toast-*.png`, `n5-panel-running-*.png`, `n4-panel-*.png`, `motion-dialog-*.png`
  (`rec-frames/contact.png` is the recording's contact sheet).

## Comments

- 2026-09-30 — from ticket 14: motion added — the header progress's bar fill (`scaleX`, `--motion-fast`,
  `motion-reduce:transition-none`) and its spinner (`motion-safe:animate-spin`, `motion-reduce:hidden`); the row's
  pending mark is the overflow Button's `pending` spinner. The LoadingOverlay's spinner/shimmer/fade are gone
  (`shimmer` keyframes stay for `.skeleton-row`, `spin` for `.spinner`). Worth an Emil look: the header's
  summary ↔ progress swap is instant (no crossfade), the run toast is headline-only with "View report", and the
  Switch primitive has no `focusableWhenDisabled` (a pressed row switch drops focus while its action runs; the door
  restores it after).


- 2026-09-30 — from ticket 14's audit re-check (P2s deferred here): **N2** a single sub-frame `activeElement === BODY` sample when the inspector reparents side↔sheet on a resize while a dialog closes (`App.tsx` deferred placement) — preserve focus across the unmount/mount boundary or document it as a non-settled transition; **N4** the stale-collision note ("this pair changed since the report") is recorded in history but not announced — a polite live region when the panel adds a note while open; **N5** with the panel open during a run, progress shows in the inert header *and* the panel foot — pick one surface (hide the header copy while a dialog owns the foot) so there is never a duplicate progress statement. Also from 14's Result: the Switch primitive lacks `focusableWhenDisabled`, so a pressed row switch loses focus (to its SPAN control, never BODY) while its action runs.

- 2026-09-30 — implemented (child, uncommitted over `11430b2`; D31 Astra audit next). Evidence in the Result above and
  `.scratch/round19/evidence/15/`. Status → `implemented`. The four 14 comments are closed here: Switch
  `focusableWhenDisabled` (primitive + 3 tests + DESIGN.md), N2 (flushSync reparent, 0 BODY frames; A/B file shows the
  gap without it), N4 (panel `role="status"` arrival region + `arrivalAnnouncement` tests), N5 (`HeaderProgress
  yielded`). The summary ↔ progress swap stays instant (decided, DESIGN.md § Motion). Handoffs written to 16, 17, 20,
  21, 22, 23 (`## Comments`). For the audit: the one contrast pair that moved is the held-focus ring (2.60 → 4.42 dark,
  2.19 → 3.49 light); every other measured pair equals the 11–14 tables. The recording is ~1 fps (hidden delegated
  tab) and lacks the Refresh / toast / panel segment (the preview client disconnected) — stills and rAF numbers cover
  those. Open: `.transition` / `.transition-colors` / `.ease-in-out` utilities are generated from prose Tailwind scans
  (a `styles.ts` comment, dormant `Layout.tsx`, docs) and applied to nothing — harmless, no `all`, 120 ms; ticket 25
  can add `@source not` for docs if the audit wants the bundle clean.

- 2026-09-30 — independent D31 audit of `34a7624`: **fix-then-pass**, report `.scratch/round19/review/ticket-15-audit.md`, fresh ignored evidence `.scratch/round19/evidence/15-audit/`. **P1 A1:** a second identical arriving note adds a history group but does not mutate the panel's polite status text (runs 3→4, announcement mutations 2→2); preserve arrival identity and publish once per new group even when titles match, without moving focus. **P2:** bind the new Button test's unused click spy; distinguish Sonner's invisible reduced-motion staging frame from its first visible frame; carry dormant/generated utility cleanup to ticket 25. Five motions measured at 120ms, interruptible; no false strip light on mount/filter/scroll/theme; N2 zero BODY frames, N5 only one accessible progress status; six axe runs zero violations, prior contrast floors preserved and held-disabled ring 4.42/3.49; version + full check green (917 frontend / 807 Rust), additional cargo test --all green, bindings unchanged. Re-check scope: identical-title arrivals and no-op rerenders, panel reopen/Clear, N5 AX tree, panel axe in both configurations, affected tests and full gate. No source edits or commit; ticket remains implemented but its audit gate is not closed.


- 2026-09-30 — **D31 gate passed** (orchestrator). Astra's audit: five motions measured at 120 ms, interruption works, no false strip lighting, 0 BODY frames, one accessible progress surface, contrast floors held, 6 axe runs clean; one P1 (consecutive equal-text arrivals not announced — the live region's text node never changed) fixed directly: the arrival fold carries the group key and the panel republishes by identity (empty → refill on the next task); the auditor's `repeat-note.ts` now records mutations 3→5 for runs 3→4 in both themes. P2s: Button test spy bound; DESIGN.md reduced-motion claim corrected to "first visible frame"; the generated unused `.transition*` utilities stay with ticket 25. Gate 918 / 807 / 1862.
- 2026-10-29 — **done — be367c8 (+ ab98c40)** (post-rebase SHAs on `origin/main`; the pre-rebase SHAs cited above exist only in the orchestrator's local history). Shipped in 1.3.0 (`v1.3.0`, published 2026-10-28; operator smoke on the main Mac passed 2026-10-29, ticket 19 § Smoke outcome). 19a closure: `round19/spec.md` § Closure — 2026-10-29.
