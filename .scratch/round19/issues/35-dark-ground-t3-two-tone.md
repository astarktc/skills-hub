# 35 — Dark ground: true black + T3's two-tone structure (prototype 30-dark-ground pick **d**, selection edge kept)

Status: ready-for-agent
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

## Comments

- 2026-11-04 — opened after the dark-ground prototype (grounds a–f). The operator read (c) as darker than T3 and (d)
  as matching it; the measured difference is `#000000` vs `#040404` for the sidebar/header pane — below naming
  threshold alone, visible as a pane against the black list. Ticket 30's `--row` under `#0e0f12` (ground a) is
  superseded by this ticket; 30 stays done (its tokens, chip and light pass are unchanged).
