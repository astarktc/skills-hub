# 35 — Dark ground: true black + T3's two-tone structure (prototype 30-dark-ground pick **d**, selection edge kept)

Status: done — 03065db
Blocked by: 30 (done 9ca0bc8)
Blocks: 31
Source: operator 2026-11-04, on `round19/prototypes/30-dark-ground/` (grounds a–f; screenshots gitignored under
`round19/evidence/30-dark-ground/`): *"(d) … is the look I like most so far"*; then (1) keep ticket 13's 3 px iris
selection edge — ground **(f)** is (d) + the edge; (2) the pass re-values the **whole** dark theme, every surface, not
just My Skills; (3) tracked as its own ticket between 30 and 31. Reference values were **sampled from a T3 Code window
capture** (2026-11-04; the capture is not kept — it carried client text): ground `#000000`, sidebar `#030303`–`#040404`,
panels / composer `#0d0d0d`–`#0e0e0e`, panel border `#171717`, current item and bubbles `#1e1e1e`, code pill `#191919`,
text `#f5f5f5`, muted `#898989`, icons `#bebebe`. Spec: D43 (iris does not move; state colours semantic); ADR-0006.

## Problem

Ticket 30 minted `--row` (#09090b) under a `#0e0f12` app ground, i.e. "rows only" (prototype ground a). Seen beside
T3, (a) reads as a dark well inside a grey frame, (b) "whole theme one step darker" does not go far enough, and (c)
true black everywhere loses the sidebar/header pane. (d) is T3's actual structure: a true-black ground with a pane
tone four levels above it, a panel tone for hover / floating layers, and one raised tone for the current item —
neutral greys throughout, no blue-grey tint.

## Decisions (operator 2026-11-04)

- **Dark gets four ground levels** (light keeps its three, untouched):

  | Role | Today | New | Measured (fg · muted · faint · ring · input · primary) |
  |---|---|---|---|
  | `background` = `row` — app ground, skill rows | `#0e0f12` / `#09090b` | **`#000000`** | 19.26 · 8.33 · 6.00 · 7.73 · 3.94 · 4.49 |
  | `card` = `sidebar` — sidebar, group headers, list panes, dialog footer | `#15171b` | **`#040404`** | 18.81 · 8.13 · 5.86 · 7.55 · 3.85 · 4.38 |
  | `popover` = `accent` = `muted` — hover ground, dialogs, menus, toasts, panels | `#15171b` / `#1d2025` | **`#0e0e0e`** | 17.71 · 7.65 · 5.52 · 7.11 · 3.62 · 4.13 |
  | `raised` — focused / selected row, solid-neutral controls, inline code | `#1d2025` | **`#1e1e1e`** | 15.29 · 6.61 · 4.77 · 6.14 · 3.13 · 3.56 |
  | `border` / `border-strong` | `#262a31` / `#343942` | **`#171717` / `#262626`** | decorative |

  **`accent` splits from `raised` in dark** (today `muted` = `accent` = `raised`): hover is the panel tone, the current
  item the raised tone. DESIGN.md's "`raised` (= `muted`, `accent`)" line becomes per-theme. Light is unchanged.
- **Neutral greys replace the blue-grey tint in dark**: `foreground #f5f5f5`, `muted-foreground #a3a3a3`, `faint
  #898989`, `input #6b6b6b` (3.13 on `raised` — the floor; move it up one step if the child's measurement lands below
  3.0 anywhere), `sidebar-foreground` = muted-foreground. State and iris tokens do not move (D43); identity tints stay
  the hue values (slate keeps its blue-grey — it is a hue, not a neutral). Re-check `*-foreground` on the state fills:
  `#0e0f12` was chosen as "background on the fill"; with a black ground use `#000000`? — decide by measurement and
  by whether the dark state fills should read as "ink on the fill"; state the choice.
- **`--row` stays**, = `background` in both themes (31 uses it by role).
- **Selection edge stays** (ticket 13): selected = `raised` + 3 px iris edge; focused = `raised` + inset ring; the
  two coexist as today. Prototype (f) is the reference, not (d).
- **Elevation on black**: `--elevation-*` shadows are invisible on `#000`; floating layers rely on the 1 px `border`
  (`#171717`) — verify Dialog / Popover / DropdownMenu / Toast each draw a border; if one relies on shadow alone, say so
  (a border is a primitive change, documented in DESIGN.md, not a call-site class).
- **Every surface moves** — this is the dark theme, not My Skills: Projects, Explore, Settings, dialogs, the sidebar,
  the legacy `App.css` alias block (its `--bg-*` / `--text-*` aliases map onto tokens, so they follow; confirm no
  alias carries a literal dark value).

## Work

- `src/styles/theme.css` dark `:root` only (plus any `@theme inline` name a split needs — none expected: `accent`,
  `muted`, `popover` are already registered). Comments: the T3 reference and the sampled values, dated.
- `DESIGN.md`: every dark column re-measured (the same float-precision script, `round19/evidence/30-contrast.mjs`,
  extended for the fourth level; the "Dark, on grounds" convention becomes background / card / popover / raised / row
  — unchanged order, new values), the Surfaces table and ground-steps table, the "hardest pairs" list, the focus-ring
  floor (dark `ring` on `raised` 6.14; the 75 % focused-disabled case re-measured — must clear 3.0 on every dark
  ground), the Direction paragraph (T3 named as the reference, now followed for the dark ground structure).
- `src/components/ui/specimen.tsx`: nothing new unless a border is added to a primitive.
- Prototype note: append the pick and the (d)/(f) decision to `round19/prototypes/30-dark-ground/index.html`'s header
  comment (it is the primary source; it stays in the tree, not archived).

## Proof

- `node .scratch/round19/evidence/30-contrast.mjs` exits 0 with the new dark values; the numbers in DESIGN.md are its
  output; the worst pairs listed in the Result.
- `npm run styles:check`, `focus-ring.test.tsx`, `badge.test.tsx` green; `npm run version:check && npm run check`.
- Screenshots at 1280×800 from the child's own fixture port (`?scenario=rich`), dark: My Skills, Projects, Explore,
  Settings, one open dialog (e.g. Add skill), the ⌘K palette, a dropdown menu, a toast — **operator's eyes before the
  Astra audit**, as for 30. Light: My Skills only, to prove it did not move.
- Astra audit (fix-then-pass): independent contrast on every changed dark cell; the elevation-on-black check; a
  consumer sweep for any surface that drew `bg-accent` and `bg-raised` as the *same* ground and now shows a step it
  did not intend.

## Result

Implemented 2026-11-04 by a delegated child (uncommitted; the orchestrator reviews and commits). Files:
`src/styles/theme.css` (dark `:root` only), `src/components/ui/dropdown-menu.tsx`, `src/components/ui/command-palette.tsx`,
`src/components/shell/AppSidebar.tsx`, `DESIGN.md`, `docs/adr/0006-styling-architecture.md` (one amendment).

**Tokens changed (dark, old → new)** — light untouched:

| Token | Old | New |
|---|---|---|
| `background`, `row` | `#0e0f12`, `#09090b` | `#000000` |
| `card`, `sidebar` | `#15171b` | `#040404` |
| `popover` | `#15171b` | `#0e0e0e` |
| `accent`, `muted`, `sidebar-accent` | `#1d2025` | `#0e0e0e` |
| `raised` | `#1d2025` | `#1e1e1e` |
| `border`, `sidebar-border` | `#262a31` | `#171717` |
| `border-strong` | `#343942` | `#262626` |
| `input` | `#6b717b` | `#6b6b6b` |
| `foreground` (+ `card-` / `popover-` / `accent-` / `sidebar-accent-foreground`) | `#e6e7ea` | `#f5f5f5` |
| `muted-foreground`, `sidebar-foreground` | `#a6abb4` | `#a3a3a3` |
| `faint` | `#8c929c` | `#898989` |
| `ok-` / `stale-` / `failed-` / `destructive-foreground` | `#0e0f12` | `#000000` |

Not moved: `primary`, `primary-foreground`, `ring`, `sidebar-primary*`, `sidebar-ring`, `ok`, `stale`, `failed`,
`destructive`, the seven identity pairs, the syntax tokens, `--elevation-*`.

**Ink choice: `#000000`.** The ground as ink on the fill (the `#0e0f12` rule's own reading, carried to the new ground),
and it measures higher on every fill and every `/90` composite: ok 11.77 / 9.50, stale 11.01 / 8.92, failed 7.57 / 6.20
(solid / worst `/90`) vs `#0e0f12`'s 10.74 / 8.67, 10.05 / 8.14, 6.91 / 5.66. Stated in a theme.css comment.

**Elevation on black: no primitive needed a border.** Popover and DropdownMenu draw `border` (`floatingSurface`), Dialog
(the ⌘K palette included) and Toast `border-strong`; Tooltip is inverted (`foreground` ground, 19.26 on black) and
needs none. Documented in DESIGN.md § Popover, DropdownMenu, Tooltip.

**Two primitive/shell changes the split forced (light unchanged in both, since light `accent` = `raised`):**
- DropdownMenu and CommandPalette highlighted item `bg-accent` → `bg-raised`: in dark `accent` = `popover`, so the
  highlight measured **1.00** on its own popup — a pointer highlight drew nothing. Now `raised` on `popover` 1.16
  (verified rendered: menu `#0e0e0e` / highlight `#1e1e1e`, palette highlight `rgb(30,30,30)` + iris ring).
- AppSidebar's current area `aria-[current=page]:bg-sidebar-accent` → `bg-raised`: `sidebar-accent` is now the hover
  tone, so current = hover would have held; the ticket's "the current item the raised tone", prototype (f)'s
  `.nav.on { background: var(--raised) }` and DESIGN.md's "a `raised` ground" all say raised. Rendered: current
  `#1e1e1e` at rest and under hover; hover elsewhere `#0e0e0e`.

**Measured** (`.scratch/round19/evidence/30-contrast.mjs`, output `30-contrast.out.txt`; gitignored). The script was
lost with ticket 30's worktree and **rebuilt** from its surviving fragments (conversation log) with float intermediates
(audit F1); `--old` prints the pre-35 dark and reproduces every former DESIGN.md dark ratio (15.50, 8.31, 6.01, 5.31,
8.76, 6.05, 4.42 …), and light reproduces ticket 30's light table. Exits 0. Ground steps dark: bg→card 1.02 (a pane cue,
documented as such), card→popover 1.06, popover→raised 1.16, row→accent (hover) 1.09, row→raised 1.26.

Worst dark pairs — text: `faint` on `raised` 4.77, `primary-foreground` on `primary` 4.68 (unchanged), `faint` on
`popover` 5.52, `primary-foreground` on `primary/90` 5.32; non-text: `input` on `raised` 3.13, `primary` fill on
`raised` 3.56, `input` on `popover` 3.62; focused-disabled ring (75 %): 4.60 / 4.55 / 4.43 / **4.05** / 4.60 on
background / card / popover / raised / row (floor 4.05; 50 %: 2.53–2.60). Nothing below threshold, so no value was moved
beyond the table (`input` stays `#6b6b6b`). Syntax on new `raised`: 8.52 / 8.89 / 8.99 / 8.59. Identity: fg on tint
unchanged (8.12–8.80); tint vs new `card` 1.47–1.54, fg vs `card` 11.94–13.51.

**App.css alias block:** every `--bg-*` / `--text-*` / `--border-*` / `--accent-*` / `--status-*` alias resolves to a
token or a `color-mix()` of tokens — no literal dark value. Outside the block, two `var()` fallbacks carry literals
(`App.css:840` `var(--success, #22c55e)`, `:863` `var(--danger, #ef4444)`); both aliases are defined, so the fallbacks
never apply. Reported, not fixed (App.css only shrinks).

**Consumer sweep — for the audit / ticket 31** (not fixed: surface work):
- Rows hovering to `card` now step 1.02 on `row` (near invisible): `SkillRow.tsx:432` (ticket 31 moves it to
  `accent`), `ExplorePage.tsx:263`, `AssignmentMatrix.tsx:304`.
- `accent` hover drawn *inside* a `popover` layer shows no step in dark (1.00): Button `outline` / `ghost` neutral
  (`button.tsx:43,49`) inside a dialog body — e.g. the Dialog ✕ and Notifications' ghost actions — and
  `NotificationsModal.tsx:417` (`hover:bg-accent`). In the dialog footer (`card`) the step is 1.06. Open decision for the
  operator: leave (hover is not a gated cue) or give in-panel hovers `raised`.
- Stale light figures noticed (pre-ticket-30 light grounds) and corrected only where a line was rewritten anyway:
  Explore Installed light 7.20 → 6.75, menu highlight light 1.16 → 1.24, palette hint on highlight light 7.20 → 6.75.
  Untouched elsewhere (e.g. the strip table's light `raised` column). The script prints the light 75 % ring floor as
  3.09 (3.088); DESIGN.md keeps 3.08.

**Screenshots** (`.scratch/round19/evidence/35-dark/`, 1280 × 800, `?scenario=rich`, own fixture port 5176; converted from
ego's Rec2020 tag to sRGB — pixel-checked: ground `#000000`, sidebar `#040404`, menu `#0e0e0e` / highlight `#1e1e1e`,
dialog `#0e0e0e` / footer `#040404`): `myskills-1280.png`, `projects-1280.png`, `explore-1280.png`, `settings-1280.png`,
`dialog-1280.png` (Add skill), `palette-1280.png` (⌘K, "up", a skill highlighted), `menu-1280.png` (pdf's ⋯, keyboard
highlight on Change source…), `toast-1280.png` ("60 updated", success tint), `specimen-1280.png`; light:
`myskills-light-1280.png` (white ground, `#f0f1f4` sidebar — unchanged).

**Gate:** `npm run version:check && npm run check` green — lint, styles:check (App.css 812 counted lines = ceiling, 38
migrated files), vitest 1120 passed (53 files), build, cargo fmt, clippy `-D warnings`, cargo test 837 passed.
`npx vitest run src/components/ui src/lib/skillPresentation.test.ts`: 250 passed (13 files).

Unproven: the operator's eyes on the screenshots; the Astra audit (independent contrast, elevation check, consumer sweep).

## Comments

- 2026-11-04 — opened after the dark-ground prototype (grounds a–f). The operator read (c) as darker than T3 and (d)
  as matching it; the measured difference is `#000000` vs `#040404` for the sidebar/header pane — below naming
  threshold alone, visible as a pane against the black list. Ticket 30's `--row` under `#0e0f12` (ground a) is
  superseded by this ticket; 30 stays done (its tokens, chip and light pass are unchanged).
- 2026-11-04 — implemented by delegated child; awaiting operator screenshots + Astra audit.
- 2026-11-04 — orchestrator, on the child's two open findings (before the audit): **row hover → `accent`** in
  `SkillRow`, `ExplorePage`, `AssignmentMatrix` (`card` is 1.02 on the dark ground — invisible; light hover becomes
  `raised`, which the focused / selected row's ring / edge still distinguishes) and **control hover → `raised`** for
  `ghost` / `outline neutral` Button and the notification rows (an `accent` hover inside a `popover` layer drew
  nothing in dark). Both rules written into DESIGN.md § Tokens; ticket 31's hover line already says `accent`.
- 2026-11-04 — operator: screenshots "looks good". Astra audit `round19/review/ticket-35-audit.md`: **PASS-with-findings**
  — 95 dark cells agree within 0.01, focused-disabled ring floor 4.05. F1 (P1) `.prose .md-inline-code` still drew
  `muted`, which the split made equal to the popover ground — fixed to `raised` (its documented role). F2 (P2) neutral
  `outline` on a `raised` parent has no hover delta (pre-existing) — noted in DESIGN.md § Button. F3 (P2) a `solid ok`
  Button's `/90` hover on `raised` steps 1.02 — no production caller; left as a note.
