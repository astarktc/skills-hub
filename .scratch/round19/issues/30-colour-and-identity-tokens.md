# 30 — Colour and identity tokens: state hues, the identity treatment the prototype picks, light-theme ground steps

Status: ready-for-agent
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
