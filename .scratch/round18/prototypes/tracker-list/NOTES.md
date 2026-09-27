# Tracker List — build notes (round 18 prototype)

**Accent: iris `#9690ff` (dark) / `#5a4fdc` (light), fill `#6a61e8`.** Hue ~245° sits in the gap between the incumbent blue and the three status hues, so it never reads as "state". It marks selection, the keyboard focus ring and the single primary action (Add skill) — nothing else.

## Decisions where the world was silent
- **Neutral switches.** The enable switch is "on" in a mid-grey (`--color-switch-on`), not accent. 58 accent tracks in one column would make accent mean "normal". Disabled rows fall back to hollow and grey.
- **Synced is quieter than stale or failed.** The synced dot is desaturated green `#29875a` (4.3:1). The 3 amber/red rows are the signal; 55 green dots are the noise floor.
- **Segments are neutral.** The 7-segment "N/7" bar lights in `fg-2` and only turns red for a failed target. Stale is skill-level, so it shows on the status dot only.
- **The overflow menu is the palette.** `…` opens ⌘K scoped to that skill, so one action surface serves row, detail and bulk. Sync, Assign, Delete and Add are palette sub-pages (with a breadcrumb; ⌫ goes back), so there are no extra menus or modals.
- **Group headers join j/k traversal.** Enter or →/← collapses a group; ← on a row jumps to its header. Headers use the lifted `panel` layer, so the second neutral does real work.
- **Detail follows the cursor once open** (Linear behaviour). Initial state opens `zod-schema-first`, to show the typed-failure recovery (TARGET_EXISTS → Overwrite and retry).
- **Status line instead of toasts.** Results appear in the footer (`aria-live`), next to the permanent state legend. Nothing pops.
- **Scope column** always shows the globe; a folder+N appears only with projects.
- **Fuzzy matching** ranks word-starts and runs, rejects scattered matches (`assign` no longer hits canvas-design), and allows acronyms (`zsf` → zod-schema-first). Selection actions get a rank boost while a selection exists.
- **960 collapse:** the inspector becomes a fixed right sheet over the list. Flat mode drops the description column via a container query. No horizontal scroll (measured: scrollWidth 960).

## What felt wrong while building
- Every Linear-list cliché fights the brief's "state readable at a glance". The fix was subtraction: neutral switches, muted synced, one filled button.
- Fixture totals: the brief says 61 skills, but its per-source counts sum to **60** (28+9+12+6+5). I rendered the data as given.
- The detail pane's two accent-filled actions (Retry, Add skill) competed, so detail actions are now neutral buttons.
- Evidence: `preview_snapshot` downscales 1440×900 to 1280×800 and returns ~150 KB of accessibility text per capture. The 4 PNGs were taken with agent-browser at native size. `t3-preview-dark-1440-crosscheck.png` is the preview-tool capture of the same state. The page also needs HTTP (the preview refuses `file://`).

## What I'd change next
- Tooltips on property icons are hover-only. Keyboard users get them from aria-labels and the detail pane; a `?`-peek would be better.
- The row is not a real ARIA grid (no cell navigation); roving tabindex plus the palette covers every action.
- Real keyboard shortcuts for enable/deploy (`e`, `d`) instead of palette-only.

## Detector (`impeccable detect`)
- Fixed: `cramped-padding` on the palette footer. Softened: border + wide shadow on the palette and bulk bar (24/64px → 16/32px blur).
- Kept on purpose (advisory): hairline border + shadow on those two floating layers. On a dark ground the edge carries separation and the shadow alone disappears.
- Edit-hook `innerHTML` warnings: prototype renders fixture data through an `esc()` helper; accepted for a throwaway.

## Tokens
- Dark: ground `#0e0f12` · panel `#15171b` · raise `#1d2025` / `#252930` · line `#262a31` / soft `#1b1e23` / strong `#343942` · fg `#e6e7ea` / `#a6abb4` / `#858b95` · ok `#3ddc84` (dot `#29875a`) · stale `#f5b400` · fail `#ff6b6b`
- Light: ground `#fcfcfd` · panel `#f3f4f6` · raise `#eceef1` / `#e3e6ea` · line `#e1e3e8` / `#e9ebee` / `#cdd1d8` · fg `#16181d` / `#494e57` / `#646a74` · ok `#16a35a` · stale `#b77f00` · fail `#c22e2e` · switch-on `#8a9099`
- AA (measured): fg-3/raise ≥4.69, accent/ground ≥5.7, white/accent-fill ≥4.68, all status and switch graphics ≥3.1.
- Type: system stack + `ui-monospace`; 12/16 · 13/20 (body) · 14/20 · 16/24; weights 400/500/600; `tabular-nums` on every count.
- Spacing 4px base; row = header = palette item = 36px; bar 44, filters 40, footer 28; columns `28 20 240 1fr 84 60 40 28`.
- Radii 4 / 6 / 10. Motion: 120ms `cubic-bezier(.16,1,.3,1)`, opacity+transform only (palette open, bulk bar rise, switch knob, chevron). Nothing moves on focus or select; the palette closes instantly; `prefers-reduced-motion` removes all transitions.
