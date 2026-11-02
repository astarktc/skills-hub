# Prototype 30: identity colour for My Skills' source groups

Throwaway. Open `index.html` directly (`file://`) or from any static server.
Query: `?variant=A|B|C&theme=dark|light`. Add `&grounds=current` to the light theme to compare it with today's grounds.
The floating bottom bar switches the variant and the theme and shows the current state and the viewport width.
Keys: ←/→ cycle the variant, `T` toggles the theme. Clicking a chevron collapses or expands its group, and clicking a
checkbox selects the row.

Screenshots are gitignored, under `.scratch/round19/evidence/30-proto/`, named `{A,B,C}-{dark,light}-{1800,1280}.png`
(1800×1000 and 1280×800).

## The question

Ticket 30 (D43) asks which identity-colour treatment My Skills' source groups should use. The treatment also has to
answer the operator's complaint that the app is "very, very monochrome", without spending the iris accent and without
diluting the state colours.

Every variant renders the same anatomy from ticket 31 (D38–D42):

- each group is a bordered rounded card, with its header on `card` and its rows on `background`;
- the name column is `--name-col`, clamped to [12rem, 22rem], and the description truncates;
- each row has a strip of 8 harness marks plus N/M, a scope cell (globe and/or folder + N), the switch and the overflow
  menu;
- one focused row, drawn as a raised rounded block (frontend-design);
- two selected rows with the 3 px iris edge (skill-creator, pdf);
- one collapsed group (vercel-labs/agent-skills);
- a header with chevron · identity · name · count · state summary · Deploy all, plus a row of harness initials over the
  strip column.

All on-screen words use the D45 vocabulary.

## The three treatments

| | Treatment | What it costs |
|---|---|---|
| **A**: source chip | A 22 px, 2-letter chip in the header (`AN`, `VL`), drawn as hue text on the hue's tint. "Individual sources" gets a neutral chip with the layers icon. Its rows add a 16 px mini-chip before the muted source label, and a local folder gets a neutral folder glyph. | It adds one small coloured object per group. Hash collisions become visible: `obra` lands on teal, the same hue as `anthropics`. |
| **B**: source edge | A 3 px hue bar on the card's leading side, running the full height and clipped by the radius. There is no chip, so the header keeps today's branch/layers icon. | **It collides with selection.** The selected row's 3 px iris edge is also a leading-edge bar, and the prototype has to push it 3 px inward, so a selected row in a coloured group shows two adjacent bars (hue + iris). On a collapsed card the edge is only a 40 px stub, and the bar reads as "this group is selected or special" rather than as "this is anthropics". |
| **C**: none | State colours only. The header keeps today's faint branch/layers icon. | It solves nothing about monochrome. The only colour left is iris, red and amber, and when the library is healthy the screen is grey. |

## State colours (all variants)

- Deployed is the neutral filled mark (`muted-foreground`, or `input` when the strip is complete); it is never green.
- Not deployed is a hollow `input` mark.
- Failed and Copy missing are an × in the existing `failed` red.
- Disabled is a slashed `faint` mark.
- Out of date copy is a half-filled mark in the **new amber**, below.

### Proposed amber: `--warning` (+ `-foreground`)

The repo already has `--stale` (`#f5b400` / `#8a5d00`). In light, `#8a5d00` (OKLCH L 0.51, C 0.108, h 76) reads brown
or olive at 12 px, which is one reason light looks monochrome. The proposal re-values the token, either renaming
`stale` to `warning` or adding `warning` and retiring `stale`:

| | Dark | Light |
|---|---|---|
| `--warning` | `#fbac16` (OKLCH 0.80 / 0.165 / 75) | `#9e5d00` (OKLCH 0.54 / 0.125 / 66) |
| on `background` / `card` / `raised` | **10.05 / 9.41 / 8.57** | **5.22 / 4.63 / 4.21** |
| `--warning-foreground` | `#0e0f12` (10.05 on warning) | `#ffffff` (5.22 on warning) |
| soft badge (10 % into `popover`), text on it | 7.87 | 4.57 |

- Light amber text on `raised` measures 4.21, so it may appear there only as a non-text mark (≥ 3:1). Amber text sits
  on `background`, `card` or the soft badge, and all three pass 4.5.
- The dark value is 7° warmer than `#f5b400`, which separates it from yellow and from the moss identity hue.
- The rendered page measured 10.05 (mark on `background`) / 9.41 (summary text on the header) in dark and 5.22 / 4.63
  in light, matching the computed values.

## Identity hue palette (A and B)

**Derivation:** `hue = PALETTE[fnv1a32(owner.toLowerCase()) % 7]`, keyed on the repo **owner** and never on state.
FNV-1a 32-bit starts at `0x811c9dc5` and, for each code unit c, applies `h ^= c; h = (h * 0x01000193) >>> 0`.

| Owner | Hash | Hash % 7 | Hue |
|---|---|---|---|
| anthropics | 2569612480 | 1 | teal |
| vercel-labs | 703213605 | 3 | sky |
| obra | 3167613737 | 1 | teal (a collision) |
| mattpocock | 3169730402 | 6 | slate |
| local folder (no owner) | — | — | neutral, no hue |

Chip initials are the first letters of the first two `-_.` parts of the owner, otherwise its first two letters.

The palette has seven hues. They avoid failed red (h≈23), amber (h≈66–82), iris (h≈281) and the old ok green (h≈154),
so identity never reads as state or selection. That is why plum and blue were dropped (they sit too close to iris) and
orange was dropped (too close to amber). All values are OKLCH-derived at equal lightness, so every hue measures the same:

| Hue | h | Dark tint | Dark fg | **fg on tint** | Dark edge (on bg) | Light tint | Light fg | **fg on tint** | Light edge (on bg / card) |
|---|---|---|---|---|---|---|---|---|---|
| moss | 125 | `#283215` | `#bddb8a` | **8.77** | `#92b255` 7.96 | `#e1edcf` | `#475f06` | **5.92** | `#698726` 4.12 / 3.65 |
| teal | 178 | `#09362e` | `#74e5ce` | **8.77** | `#22bea5` 8.19 | `#cbf2e8` | `#006855` | **5.59** | `#00927a` 3.89 / 3.45 |
| cyan | 202 | `#0a3437` | `#76e1ea` | **8.80** | `#28b9c3` 8.05 | `#ccf0f3` | `#00656d` | **5.62** | `#008d97` 3.99 / 3.54 |
| sky | 230 | `#0e3242` | `#7cdbff` | **8.65** | `#38b2e4` 7.90 | `#cdeeff` | `#005f86` | **5.79** | `#0086b6` 4.13 / 3.66 |
| orchid | 318 | `#39253f` | `#efb5ff` | **8.42** | `#c887dc` 7.26 | `#f5dffc` | `#703c80` | **6.36** | `#9c5daf` 4.57 / 4.04 |
| rose | 350 | `#422232` | `#ffadd9` | **8.12** | `#e47eb1` 7.23 | `#ffdcec` | `#84345f` | **6.33** | `#b45486` 4.61 / 4.08 |
| slate | 255 | `#292e36` | `#bfcfe5` | **8.63** | `#94a6be` 7.72 | `#e2e8f1` | `#485669` | **6.07** | `#6b7c92` 4.27 / 3.78 |

- Chip text is 10.5 px bold, so the small-text rule (4.5:1) applies. Every hue passes it in both themes, with a worst
  case of 5.59 (light teal).
- B's edge is decorative (the group name carries identity), but it still clears 3:1 on both the row ground and the
  header ground in both themes.
- With 7 hues, collisions among about 5 sources are likely (the birthday problem). The prototype shows one on screen.
  Colour therefore only ever *supplements* the name.

## Light ground proposal

| Token | Today | **Proposed** |
|---|---|---|
| `background` | `#fcfcfd` | **`#ffffff`** |
| `card` (= `sidebar`) | `#f3f4f6` | **`#f0f1f4`** |
| `raised` (= `muted`, `accent`) | `#eceef1` | **`#e4e7ec`** |
| `border` | `#e1e3e8` | **`#d5d9e0`** |
| `border-strong` | `#cdd1d8` | **`#bcc2cc`** |
| `input` | `#7c828b` | **`#737983`** |
| `popover` | `#ffffff` | `#ffffff` (unchanged) |

- **Ground steps** (contrast ratio between adjacent grounds): today `background`→`card` is 1.07 and `card`→`raised` is
  1.06. The proposal is **1.13** and **1.10**.
- **`border`** measures 1.42 against `background` (today 1.25) and 1.25 against `card`. `border-strong` measures 1.79
  against `background`.
- **`input` has to move with the grounds.** On the new `raised`, today's `#7c828b` would measure 3.12, which only just
  passes. `#737983` measures 4.38 / 3.88 / 3.54 on `background` / `card` / `raised`.
- **Text re-measured on the darkest ground (`raised`):**

  | Pair | Ratio |
  |---|---|
  | `foreground` | 14.33 |
  | `muted-foreground` | 6.75 |
  | `faint` | 5.04 (5.53 on `card`) |
  | `ring` | 4.72 |
  | `failed` | 4.55 |

  All pass.
- Dark is unchanged; its steps are already visible.
- To compare the two light grounds, use `?theme=light&grounds=current`.

## Finding outside the colour question (for ticket 31)

At ticket 31's strip pitch of 16 px (a 12 px mark plus a 4 px gap), the 2-letter harness initials run together:
"GMAM" read as one word in the first capture. The prototype uses a **20 px pitch** (8 px gap), which makes the strip
column `marks × 20 + 40` = 200 px. That is the narrowest pitch at which 10 px initials stay separate. At 1280 px this
still leaves the description about 340 px.

## Recommendation

**A, the source chip, limited to the group header.**

- **Why A:** it is the only variant that answers both halves of the question. It gives each group a glanceable identity
  ("the teal AN group"), and it adds colour where the eye starts a scan. It also costs nothing structurally: the chip
  sits in a slot the header already has (today's branch icon), so no other element moves. This is T3's pattern, and it
  carries over cleanly because our state colours stay unambiguous: the chip palette excludes red, amber, iris and green
  by construction.
- **Why not B:** its edge competes with the selection edge, the one leading-side bar the product already has (ticket
  13). Two coloured bars side by side make "selected" harder to read, which is the opposite of the goal.
- **Why not C:** it is honest and calm, but it leaves the operator's monochrome complaint unanswered.
- **What to drop from A:** the per-row mini-chips in "Individual sources". They put up to one chip per row in a group
  meant to read as "miscellaneous", and they make hash collisions visible on one screen (obra and anthropics are both
  teal). The muted mono source label is enough there.
- **What to pair with it:** if the operator picks A, ship it together with the amber re-value and the stronger light
  grounds. The light theme gains the most from all three.
