# 30 — Colour and identity tokens: state hues, the identity treatment the prototype picks, light-theme ground steps

Status: done — 9ca0bc8
Blocked by: 32; the colour prototype's pick (`round19/prototypes/30-colour/`, operator decision)
Blocks: 31
Source: BACKLOG #78 (absorbed here 2026-11-02 — operator 2026-10-27 and 10-29: *"in both light and dark mode, it's
very, very monochrome. The only splash of color is the little bit of purple … it looks better in dark mode than in
light"*; 11-02: *"I really like current T3 Code's design language and I'd be inclined to get closer to it"*). Spec: D43;
ADR-0006; DESIGN.md's measured-contrast table. Reference traits: gitignored
`evidence/design-ref/t3-design-traits.md` (near-black single ground; identity colour only as 2-letter chips; green/red
diff stats; one blue accent; rounded raised current item).

## Problem

The token world (`src/styles/theme.css`) is deliberately quiet: one iris accent for selection / primary / focus, red for
`destructive`, and grey steps for everything else. Both themes read monochrome; the light theme's ground steps
(`card` #f3f4f6 on `background` #fcfcfd) are nearly invisible, so light looks flatter than dark. There is no amber
("out of date" renders in grey), no identity colour (every source group and every harness mark looks alike — the
operator identified Cursor's mark by counting positions, #72).

## Decisions already taken (D43)

- **State colours are semantic**: failed = red (exists), **out-of-date = amber (new token)**, disabled = muted,
  deployed = the neutral filled mark — **never green**. Green is reserved for diff-style "+N" moments if any.
- **The iris accent does not move** (selection edge, primary, focus ring).
- **Identity colour is decided by the prototype** (three treatments, both themes, with a harness-initials row over the
  strip column in each): (A) 2-letter source chips, hue derived deterministically from the repo owner; (B) a thin
  coloured source edge on the group card; (C) no identity colour. The operator picks from screenshots; the pick is
  recorded in `## Comments` here before this ticket is dispatched.
- **Light theme gets stronger ground steps** — the same pass, measured.

## Work

- `src/styles/theme.css` only, dark `:root` **and** `[data-theme='light']`, `@theme inline` names: `--warning` (+
  `-foreground`), the identity hue set the pick needs (6–8 hues, each with a dark and a light value and a `-foreground`
  that passes 4.5:1 on it — or a 3:1 large-text/non-text rule where the chip's text is ≥ 14 px bold, stated per hue),
  revised light grounds (`background` / `card` / `raised` / `border` / `border-strong` with visible steps), and the
  hover/focused/selected row grounds D38 names if the existing tokens cannot express them.
- **DESIGN.md**: every new token in the contrast table with its measured pair(s); the identity rule written once
  ("hue from a stable key, never from state"); the T3 reference named as the direction, not copied.
- **Primitives**: a `Chip` / `Badge` tone for identity if the pick is (A) or (B) — a documented `tone`, not a call-site
  class (ADR-0006).
- **No surface work here** — ticket 31 applies the tokens to the rows; ticket 30 proves them on the primitives and on
  the prototype page re-rendered against the real tokens.

## Proof

- Contrast: a script or `preview_evaluate` measuring every new pair in both themes, numbers in DESIGN.md.
- `npm run styles:check` green (no raw colours, no new stylesheets); `focus-ring.test.tsx` green.
- Screenshots: the primitives gallery (if any) and My Skills (unchanged layout, new tokens flowing through) in both
  themes at 1280×800 — one round of **the operator's eyes on screenshots before the Astra audit** (handoff 2026-10-29).
- `npm run version:check && npm run check` green.

## Result

Implemented 2026-11-02 by a delegated child (uncommitted; the orchestrator reviews and commits). Tokens and primitives
only — no surface consumes `row` or the chip yet (ticket 31).

**Minted / re-valued** (`src/styles/theme.css`, dark `:root` + `[data-theme='light']` + `@theme inline`):

- `--row` / `bg-row`: dark `#09090b`, light `#ffffff`.
- `--stale` (name kept; DESIGN.md role "Out of date / warning (amber)"): dark `#fbac16`, light **`#985700`**, with
  `-foreground` `#0e0f12` / `#ffffff`.
- `--identity-{moss,teal,cyan,sky,orchid,rose,slate}` (tint) and `--identity-<hue>-foreground` (text), the NOTES values
  exactly, registered as `bg-identity-<hue>` / `text-identity-<hue>-foreground`. The edge values were not minted.
- Light grounds: `background #ffffff` · `card #f0f1f4` · `raised #e4e7ec` · `border #d5d9e0` · `border-strong #bcc2cc` ·
  `input #737983`, with the aliases moved alongside (`sidebar`, `muted`, `accent`, `sidebar-accent`, `sidebar-border`). `popover` is unchanged; dark grounds are unchanged.
- Type scale: `--text-2xs` 10.5 px, used only for bold initials marks. It keeps the prototype's chip type, and ADR-0006 is amended to match.

**Decisions:**

1. **Light `--row` = `background` (`#ffffff`)**. Rows take the extreme of the ground scale in both themes. Dark goes one step
   below `background`, but nothing in light is lighter than white, so the light value is `background` itself. The
   `row → card` step (header, hover) is 1.13 and `row → raised` (focused) is 1.24. In dark those steps are 1.11 and 1.22,
   and `row → background` is 1.04.
2. **Badge interface**: `tone="identity"` plus a required `hue: IdentityHue`, typed as a union. Identity is **always
   `soft`**, so `variant="outline"` / `"solid"` with identity is a type error, and so is `hue` on any other tone.
   `badge.test.tsx` pins all of this with `@ts-expect-error`. Identity is drawn at a new `size="chip"`: 22 × 22, `rounded-md`,
   10.5 px bold. That is also its only size, and it is the default for identity. `size="chip"` with `tone="neutral"` gives the
   ownerless chip (Individual sources, a local folder) that 31 needs. `IDENTITY_HUES` and the `identityGround`
   class map live in `ui/styles.ts`, a pure module, so `lib/skillPresentation.ts` imports the list without React.
3. **Light amber is `#985700`, not NOTES' `#9e5d00`** (one OKLCH step darker: L 0.54 → 0.52, same hue and chroma). At
   `#9e5d00` the measuring script failed two pairs. `stale-foreground` on `stale/90` (the solid stale Button's hover) measured
   4.32, and amber text on `raised` measured 4.21. At `#985700` they measure 4.64 and 4.58. DESIGN.md still keeps NOTES' rule, the
   tighter one: amber text goes on `background` / `card` / `row` / the soft badge, and on `raised` amber is only a non-text
   mark. **The operator should confirm this value on the screenshots.**

**Pure fold** (`lib/skillPresentation.ts`): `fnv1a32`, `identityHue(owner)` and `identityInitials(owner)`. The vitest pins
anthropics → 2569612480 / teal, vercel-labs → 703213605 / sky, obra → 3167613737 / teal and mattpocock → 3169730402 / slate. It also
pins case-insensitivity and the initials rule (`VL`, `AN`, `MO`, `X`). The fold is not wired into any surface.

**Measured** (`.scratch/round19/evidence/30-contrast.mjs`, output in `30-contrast.out.txt` beside it; gitignored). The
script keeps mixed/composited intermediates in floating point (audit F1) and exits 1 on a failing gated pair. It exits 0. Worst pairs:

- Text: light `failed` on `raised` 4.55, light `stale` on `raised` 4.58 (used only as a mark), light `stale-foreground` on `stale/90` 4.64,
  light `ok` on `raised` 4.71, light `ring`/`primary` on `raised` 4.72, dark `primary-foreground` on `primary` 4.68 (unchanged).
- Non-text: light `input` on `raised` 3.54. Dark `primary` fill on `raised` is 3.49 (unchanged).
- On `row`: dark is better than `background` everywhere (foreground 16.09, faint 6.35, input 4.05). Light equals `background`.
- Identity text on tint: dark 8.12–8.80, light 5.59–6.36, which confirms NOTES. The worst is light teal at 5.59.
- Ground steps in light: `background → card` 1.13 and `card → raised` 1.10 (they were 1.07 and 1.06).
- Not a rendered pair: light `faint` on the legacy `--bg-element-hover` measures 4.41. App.css still defines that alias, but no rule or
  utility reads it any more. DESIGN.md says so; `faint` is left unchanged.
- Knock-on: the focus ring's floor drops from ≥ 5.04 to ≥ 4.72 (light `raised`). DESIGN.md's ring claims are updated, along with
  the `styles.ts` and `dropdown-menu.tsx` comments. The light prose syntax worst-ground figures are re-measured too.

The rendered page agrees: on the fixture specimen, the teal chip computes to `rgb(9,54,46)` / `rgb(116,229,206)` in dark and
`rgb(203,242,232)` / `rgb(0,104,85)` in light, at 22 × 22 px, 10.5 px / 700. `bg-row` computes to `rgb(9,9,11)` / `rgb(255,255,255)`.

**Screenshots** (gitignored, 1280×800, fixture `?scenario=rich`, port 5176), in `.scratch/round19/evidence/30-tokens/`:

- `myskills-{dark,light}-1280.png`: layout unchanged, with only the token values flowing through.
- `specimen-{dark,light}-1280.png`: the `bg-background` surface with the 7 hue chips, the real owners and the neutral chip.
- `specimen-row-{dark,light}-1280.png`: the new `bg-row` surface of the specimen.

**Gate**: `npm run version:check && npm run check` exits 0. That covers vitest (53 files, 1120 tests), `cargo test` (837 + 0 + 0), clippy, fmt and the build.
`styles:check` passes: App.css at 812 counted lines (the ceiling, unchanged), 38 migrated files, no raw colours.

**Files**: `src/styles/theme.css`, `src/components/ui/{badge.tsx,styles.ts,specimen.tsx,dropdown-menu.tsx}` (the last has a
comment change only), `src/components/ui/badge.test.tsx` (new: each hue compiled against the real theme.css, defined in both
themes, plus the type contract), `src/lib/skillPresentation{.ts,.test.ts}`, `vite.config.ts` (the test config adds `css.include`
for `theme.css`, because vitest blanks every CSS import, `?raw` included), `DESIGN.md`, and
`docs/adr/0006-styling-architecture.md` (an amendment line).

**Not proven here:** the operator has not yet looked at the screenshots, and the Astra audit has not run. Neither impeccable
nor tailwind-design-system is installed in this environment, so the work follows DESIGN.md / ADR-0006 directly.

## Comments

- 2026-11-02 — opened from BACKLOG #78 at the grill (D36, D43). Prototype dispatched the same evening.
- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.

- 2026-11-02 — **Operator pick: A (source chip), header only.** `round19/prototypes/30-colour/` (`?variant=A`),
  `NOTES.md` for the palette (7 hues, dark/light chip ground + text, measured), the amber re-value of `--stale`
  (`#fbac16` dark / `#9e5d00` light — a mark only on light `raised`), and the stronger light grounds (`background`
  `#ffffff` · `card` `#f0f1f4` · `raised` `#e4e7ec` · `border` `#d5d9e0` · `border-strong` `#bcc2cc` · `input` `#737983`).
  Per-row mini-chips under "Individual sources" are **dropped** (hash collisions visible on one screen). The operator
  adds: **skill rows sit on a black / near-black ground** — darker than today's `background` (#0e0f12): a new
  `--row` (or re-valued `background`) at ≈ #09090b with the group header on `card` above it, hover `card`, focused
  `raised`; measure every row text pair on it. Ticket 31 applies it; this ticket mints the token.
  Carry to 31: at 16 px mark spacing the harness initials run together ("GMAM") — the prototype used 20 px (strip
  column 200 px, not 168).

- 2026-11-02 — implemented by delegated child; awaiting operator screenshots + Astra audit.
- 2026-11-04 — operator accepted both decisions (light amber `#985700`, light `row` = `background`) on the screenshots.
  Astra audit `round19/review/ticket-30-audit.md`: **PASS-with-findings**. F1 (P1) the measuring script quantised OKLab
  mixes and `/90` composites to 8-bit hex before the luminance step — 14 of 184 cells off by > 0.01, no classification
  change; fixed (float intermediates), tables regenerated. F2 (P2) the amber-on-`raised` rule is written as a rule for
  new surfaces with the measured `ProjectList` exception named. Follow-up prototype `round19/prototypes/30-dark-ground/`
  (grounds a–e) asks how dark the dark theme should be; the answer lands as a second token pass on this ticket or on 31.
