# DESIGN.md — the Skills Hub visual world

The one reference for tokens, primitives and their rules. The design lives in
[`docs/adr/0006-styling-architecture.md`](docs/adr/0006-styling-architecture.md). The values come from the direction contract
[`.impeccable/surfaces/src-components-skills-skillslist-tsx.md`](.impeccable/surfaces/src-components-skills-skillslist-tsx.md).
AGENTS.md § Invariants requires this file to change whenever a primitive's `variant` / `tone` / `size` set or a token
changes.

- **Tokens** are defined in exactly one file, `src/styles/theme.css`. Dark values are in `:root` because dark is
  designed first. Light values are in `:root[data-theme='light']` as a measured derivation, not an inversion.
- **Utilities** go in JSX through `cn()`.
- **Recurring shapes** are the primitives in `src/components/ui/`.
- **Stylesheets**: the permanent ones are `src/index.css` (entry and base layer), `src/styles/theme.css` and
  `src/styles/prose.css`. `src/App.css` is legacy and only shrinks.

## Tokens

Each ratio is measured on the rendered page, not computed from the hex values by hand. The measuring pages are the
primitives specimen (`npm run dev:fixture`, then `http://localhost:5175/?specimen`) and the fixture app
(`?scenario=rich|failures`). The method:

- WCAG 2.x relative luminance.
- Translucent layers and `color-mix()` grounds are composited over their ancestors through an in-page canvas.
- Transitions are disabled so that every value is the settled one.

Thresholds: **text ≥ 4.5:1** and **non-text ≥ 3:1** (focus ring, a control boundary that identifies the control on
its own, state marks). Measured 2026-09-29.

**Ticket 30 (2026-11-02)** re-valued the light grounds, the amber and added `row` and the identity hues. Every pair
those touch is computed from the hex values by `.scratch/round19/evidence/30-contrast.mjs` (WCAG 2.x luminance;
`color-mix(in oklab, …)` mixed in OKLab and `/90` fills alpha-composited over the ground, in floating point — a mixed
or composited intermediate is never quantised to 8 bits before the luminance step, so the numbers are the CSS
mixture's, not a rounded pixel's). It exits non-zero on any failing gated pair.

**Ticket 35 (2026-11-04)** re-valued the whole dark theme (four ground levels, neutral greys, black ink on the state
fills). Every dark cell in the tables below is that script's output on the new values (rebuilt for ticket 35; its
`--old` flag prints the pre-35 dark and reproduces every former dark ratio). Light did not move.

"On grounds" lists the ratios against **background / card / popover / raised / row**, in that order (`row` is the
skill-row ground, ticket 30; it equals `background` in both themes since ticket 35). Surface sections further down
quote ratios measured on their own dates; where ticket 30's light grounds or ticket 35's dark grounds moved a value
there, the token tables here supersede it (the floors are in § The hardest pairs; token-pair cells in the surface
tables carry the ticket 35 dark values).

**Direction (tickets 30, 35).** T3 Code's design language is the reference, not a template: identity colour only as
2-letter source chips, and one accent (iris) for selection, focus and the primary action. State colours stay semantic
and never green for "deployed". **Dark now follows T3's two-tone ground structure** (ticket 35, prototype
`30-dark-ground` pick f, values sampled from a T3 Code window capture on 2026-11-04): a true-black ground (`#000000`)
for the app and the rows, a pane tone just above it for the sidebar and group headers (`#040404`), a panel tone for
hover and every floating layer (`#0e0e0e`), one raised tone for the current item (`#1e1e1e`), `#171717` hairlines, and
neutral greys for text (`#f5f5f5` / `#a3a3a3` / `#898989`) — no blue-grey tint. The 3 px iris selection edge (ticket 13)
stays. Light is unchanged (ticket 30's three grounds).

### Surfaces

| Token | Dark | Light | Role |
|---|---|---|---|
| `background` | `#000000` | `#ffffff` | App ground. |
| `row` | `#000000` (= `background`) | `#ffffff` (= `background`) | Skill-row ground (D38), kept as its own role so ticket 31 draws rows by name: the app ground in both themes (true black in dark; in light the brightest ground, because nothing is lighter than `background`). The group header sits on `card` above it, a hovered row lifts to `accent`, the focused row to `raised` (ticket 31 applies it). |
| `card` | `#040404` | `#f0f1f4` | The pane: sidebar, group headers, list panes, dialog footer, code blocks. In dark 1.02 on the ground — a pane cue, not a contrast cue. |
| `popover` | `#0e0e0e` | `#ffffff` | The panel: dialogs, menus, popovers, toasts, panels. |
| `accent`, `muted` | `#0e0e0e` (= `popover`) | `#e4e7ec` (= `raised`) | Hover ground (`accent`); quiet fills such as the neutral soft Badge (`muted`). |
| `raised` | `#1e1e1e` | `#e4e7ec` | The current item: focused / selected row, a menu's or the palette's highlighted item, the sidebar's current area, solid-neutral controls, inline code. |
| `sidebar*` | = card / muted-foreground / primary / accent / border / ring | same mapping | App sidebar (shadcn names); its current area draws `raised`. |

**`accent` and `raised` are one value in light and two in dark** (ticket 35): in dark, hover is the panel tone and the
current item the raised tone. Two hover rules follow: **a row hovers to `accent`** (My Skills, Explore, the matrix —
`card` is 1.02 on the dark ground, invisible; in light `accent` = `raised`, and the focused / selected row still carries
its ring / edge), and **a control hovers to `raised`** (`ghost` / `outline neutral` Button, the notification rows) — a
hover drawn as `accent` *inside* a `popover` layer shows no step in dark (both `#0e0e0e`); the menus and the palette
highlight with `raised` for the same reason (§ Popover, DropdownMenu).

Ground steps (ratio between grounds; ticket 30 strengthened light, whose steps were 1.07 / 1.06 and read flat; ticket
35 gave dark its four levels):

| Step | Dark | Light |
|---|---|---|
| `background` → `card` (ground → pane) | 1.02 † | 1.13 |
| `card` → `popover` (pane → panel) | 1.06 | 1.13 |
| `popover` → `raised` (panel → current item) | 1.16 | 1.24 |
| `card` → `raised` | 1.23 | 1.10 |
| `row` → `card` (row → its header) | 1.02 † | 1.13 |
| `row` → `accent` (row → hover) | 1.09 | 1.24 |
| `row` → `raised` (row → focused row) | 1.26 | 1.24 |
| `row` → `background` | 1.00 (same value) | 1.00 (same value) |

† **A pane cue, not a contrast cue.** `#040404` on `#000000` is below any naming threshold on its own; it reads as a
pane because it is a large field beside a black one and a `#171717` hairline (`border`) separates them. Nothing relies on
it to say state or identity.

### Text

| Token | Dark | Light | Role | Dark, on grounds | Light, on grounds |
|---|---|---|---|---|---|
| `foreground` | `#f5f5f5` | `#16181d` | Primary text. | 19.26 / 18.81 / 17.71 / 15.29 / 19.26 | 17.76 / 15.72 / 17.76 / 14.33 / 17.76 |
| `muted-foreground` | `#a3a3a3` | `#494e57` | Secondary text; the "on" switch. | 8.33 / 8.13 / 7.65 / 6.61 / 8.33 | 8.36 / 7.41 / 8.36 / 6.75 / 8.36 |
| `faint` | `#898989` ¹ | `#5b616a` ¹ | Third level: timestamps, helper text, placeholders, sort labels. | 6.00 / 5.86 / 5.52 / 4.77 / 6.00 | 6.24 / 5.53 / 6.24 / 5.04 / 6.24 |
| `ring` (iris as text) | `#9690ff` | `#5a4fdc` | Selection and focus; accent text (ghost/outline accent). | 7.73 / 7.55 / 7.11 / 6.14 / 7.73 | 5.86 / 5.18 / 5.86 / 4.72 / 5.86 |

¹ **Deviation from the contract.** The contract's dark tertiary is `#858b95`, and the first light derivation was
`#646a74`. Both were lifted one step (to `#8c929c` / `#5b616a`): the legacy hover ground (`--bg-element-hover`, from the
App.css alias block) measured them at 4.20:1 and 4.08:1 under the tertiary "(Disabled)" label of a hovered harness pill.
Dark is now T3's sampled muted grey, `#898989` (ticket 35): 4.77 on `raised`, its floor, and 5.04 on that legacy hover
ground (4.41 light on ticket 30's grounds, where the alias has no reader left — § The hardest pairs). The step between
`faint` and `muted-foreground` stays visible (luminance ratio 1.39 dark, 1.34 light).

### Accent and state

The state colours say state and nothing else; healthy is silent. Colour never carries meaning alone: a word or a
shape always carries it too.

| Token | Dark | Light | Role | Dark, on grounds | Light, on grounds |
|---|---|---|---|---|---|
| `primary` | `#6a61e8` | `#5a4fdc` | Iris fill: the one primary action, checked checkbox. | 4.49 / 4.38 / 4.13 / 3.56 / 4.49 (non-text) | 5.86 / 5.18 / 5.86 / 4.72 / 5.86 |
| `ok` | `#3ddc84` | `#127440` | Deployed / success. | 11.77 / 11.49 / 10.82 / 9.34 / 11.77 | 5.84 / 5.17 / 5.84 / 4.71 / 5.84 |
| `stale` | `#fbac16` | `#985700` | Out of date / warning (amber): an Out of date copy (a deployment drifted). | 11.01 / 10.75 / 10.13 / 8.74 / 11.01 | 5.68 / 5.03 / 5.68 / 4.58 / 5.68 |
| `failed` (= `destructive`) | `#ff6b6b` | `#c22e2e` | Failed / destructive. | 7.57 / 7.39 / 6.96 / 6.01 / 7.57 | 5.64 / 4.99 / 5.64 / 4.55 / 5.64 |

**Amber (ticket 30).** The token keeps the name `stale` (`text-stale`, `bg-stale`, the soft ground), re-valued from
`#f5b400` / `#8a5d00`: the old light value read brown or olive at 12 px. Dark is the prototype's `#fbac16` (OKLCH
0.80 / 0.165 / 75, 7° warmer than before, clear of yellow and of the moss identity hue). Light is the prototype's
`#9e5d00` one step darker, **`#985700`** (OKLCH L 0.54 → 0.52, same hue and chroma): at `#9e5d00` the solid
Button's `stale/90` hover measured 4.32 and amber text on `raised` 4.21, both failing. At `#985700` every amber pair
passes as text, `raised` included (4.58). **The rule for new surfaces (ticket 31's row anatomy onward) stays the
tighter one:** amber *text* sits on `background`, `card`, `row` or the soft badge; on `raised` amber is a non-text
mark (≥ 3:1), because 4.58 leaves no margin for a hover ground drawn over `raised`. Existing exception, measured:
the project list's "Folder not found" / stale status words (`ProjectList.tsx`) are amber text inside a button whose
hover and selected ground is `raised` — 4.58, passing; the specimen's all-ground matrix prints amber on `raised` on
purpose.

Text on the fills (dark ink `#000000` since ticket 35 — the ground as ink on the fill; the former `#0e0f12` measured
lower on every fill, e.g. `failed/90` 5.66):

| Pair | Dark | Light |
|---|---|---|
| `primary-foreground` on `primary` (`#fff`) | 4.68 | 5.86 |
| … on `primary/90` (hover, worst of the five grounds) | 5.32 | 4.79 |
| `ok-foreground` on `ok` | 11.77 (`#000`) | 5.84 (`#fff`) |
| … on `ok/90` | 9.50 | 4.76 |
| `stale-foreground` on `stale` | 11.01 (`#000`) | 5.68 (`#fff`) |
| … on `stale/90` | 8.92 | 4.64 |
| `destructive-foreground` on `destructive` | 7.57 | 5.64 |
| … on `destructive/90` | 6.20 | 4.84 |

Soft grounds are the state colour mixed into `popover`, opaque, so the text on them measures the same on every
surface:

| Soft ground | Dark | Light |
|---|---|---|
| Badge `soft` or hover ground: 10 % state into `popover`, state text on it (accent / ok / stale / failed) | 6.61 / 9.49 / 8.93 / 6.29 | 5.08 / 5.05 / 4.93 / 4.90 |
| Toast: 8 % state into `popover`, state text on it (accent / ok / stale / failed) | 6.72 / 9.80 / 9.20 / 6.44 | 5.23 / 5.20 / 5.07 / 5.04 |

### Identity (ticket 30)

A source group's 2-letter chip (`Badge tone="identity"`, § Badge): the hue's text on the hue's tint. **The hue comes
from a stable key — the repo owner — never from state** (`skillPresentation.identityHue`: FNV-1a 32-bit of the
lower-cased owner, modulo 7, into `IDENTITY_HUES` in `ui/styles.ts`, whose order is a compatibility contract). The
seven hues avoid failed red (h≈23), amber (h≈66–82), iris (h≈281) and green (h≈154) by construction, so identity never
reads as state or selection; plum and blue sit too close to iris and orange to amber, so they are not in it. All seven
are OKLCH-derived at equal lightness, so every pair measures alike. With 7 hues collisions are expected
(`anthropics` and `obra` are both teal): the chip **supplements the name, never replaces it**. A source without an
owner (a local folder, Individual sources) gets the neutral chip, no hue.

| Hue (h) | Dark tint | Dark text | Dark text on tint | Light tint | Light text | Light text on tint |
|---|---|---|---|---|---|---|
| `moss` (125) | `#283215` | `#bddb8a` | 8.77 | `#e1edcf` | `#475f06` | 5.92 |
| `teal` (178) | `#09362e` | `#74e5ce` | 8.77 | `#cbf2e8` | `#006855` | 5.59 |
| `cyan` (202) | `#0a3437` | `#76e1ea` | 8.80 | `#ccf0f3` | `#00656d` | 5.62 |
| `sky` (230) | `#0e3242` | `#7cdbff` | 8.65 | `#cdeeff` | `#005f86` | 5.79 |
| `orchid` (318) | `#39253f` | `#efb5ff` | 8.42 | `#f5dffc` | `#703c80` | 6.36 |
| `rose` (350) | `#422232` | `#ffadd9` | 8.12 | `#ffdcec` | `#84345f` | 6.33 |
| `slate` (255) | `#292e36` | `#bfcfe5` | 8.63 | `#e2e8f1` | `#485669` | 6.07 |

Tokens: `--identity-<hue>` (tint) and `--identity-<hue>-foreground` (text), utilities `bg-identity-<hue>` /
`text-identity-<hue>-foreground`, drawn only by the Badge. The chip's text is 10.5 px bold, so the small-text rule
(4.5:1) applies; the worst pair is light teal, 5.59. The tint itself is decorative (1.47–1.54 dark, 1.07–1.11 light
against `card`, the header it sits on): the letters identify the chip, and they measure 11.94–13.51 dark /
5.98–7.05 light against `card` as well (dark re-measured on ticket 35's `card`; the hues themselves did not move —
slate keeps its blue-grey, it is a hue, not a neutral). The prototype's "edge" values (variant B) were not minted.

### Lines

| Token | Dark | Light | Role | Dark, on grounds | Light, on grounds |
|---|---|---|---|---|---|
| `border` | `#171717` | `#d5d9e0` | Structural hairline. **Decorative**: never the only cue. | 1.17 / 1.14 / 1.08 / 1.08 / 1.17 | 1.42 / 1.25 / 1.42 / 1.14 / 1.42 |
| `border-strong` | `#262626` | `#bcc2cc` | Edge of a text-bearing control (outline button, segmented group). Decorative, because the text identifies the control. | 1.39 / 1.35 / 1.28 / 1.10 / 1.39 | 1.79 / 1.59 / 1.79 / 1.44 / 1.79 |
| `input` | `#6b6b6b` | `#737983` | Boundary that identifies a control on its own: text field, checkbox, switch track. Light moved with the grounds (`#7c828b` would measure 3.12 on the new `raised`); dark is the neutral `#6b6b6b` (ticket 35), 3.13 on `raised`, its floor. | 3.94 / 3.85 / 3.62 / 3.13 / 3.94 | 4.38 / 3.88 / 4.38 / 3.54 / 4.38 |
| `ring` | `#9690ff` | `#5a4fdc` | Focus ring (non-text). | 7.73 / 7.55 / 7.11 / 6.14 / 7.73 | 5.86 / 5.18 / 5.86 / 4.72 / 5.86 |

Other non-text pairs, dark / light:

- Switch thumb (`muted-foreground`) on its `raised` track: 6.61 / 6.75.
- Checked thumb (`background`) on the `muted-foreground` track: 8.33 / 8.36.
- Check glyph on `primary`: 4.68 / 5.86.
- Selected segment/tab iris edge on `raised`: 6.14 / 4.72.
- Neutral soft Badge text (`muted-foreground` on `muted`): 7.65 / 6.75; solid neutral (`foreground` on `raised`):
  15.29 / 14.33.
- Tooltip (`background` on `foreground`, inverted): 19.26 / 17.76.

### Prose (read only through `var()` by `styles/prose.css`)

| Token | Dark | Light | Worst ground (dark / light) |
|---|---|---|---|
| `syntax-keyword` | `#f29fd8` | `#a1306e` | 8.52 on raised / 5.36 on raised |
| `syntax-string` | `#7fc8dc` | `#16677d` | 8.89 / 5.18 |
| `syntax-number` | `#e8b48a` | `#87460f` | 8.99 / 5.82 |
| `syntax-function` | `#a9b6ff` | `#3a4cb0` | 8.59 / 5.96 |

Light values re-measured on ticket 30's `raised`, dark on ticket 35's (the syntax colours did not move). Comments and line numbers use `faint` (≥ 4.77 dark, ≥ 5.04 light). Punctuation uses `muted-foreground`.

### The hardest pairs, and the round-18 regression list

Every pair the round-18 audit measured below threshold now passes. These were measured on the rendered page, in the
fixture app, through the alias block:

| Audit pair (round 18) | Was | Dark now | Light now |
|---|---|---|---|
| Light tertiary text (timestamps, inactive nav, helper) | 2.56 | 6.12 (`.skill-source.time`) | 6.09 |
| Deployed harness pill | 3.67 light | 10.06 | 5.30 |
| Failed harness pill | 4.41 light | 6.39 | 4.94 |
| "+N more" pill | 2.33 light | 5.22 | 5.37 |
| Explore Install | 2.14 dark | 4.68 ² | 5.86 ² |
| Explore Installed | 1.82 dark / 3.58 light | 7.65 ² | 6.75 ² |
| Success toast ("Harness selection saved.") | 4.26 both | 8.89 | 5.22 |
| Search placeholder | 3.36 light | 5.73 | 5.67 |
| Import review "Match" / "Conflict" | 3.07 / 4.41 light | 9.41 / 6.39 | 5.03 / 4.94 |
| Light `faint` on the new `raised` (ticket 30) | — | — | 5.04 |
| Light `input` on the new `raised` (ticket 30, non-text) | — | — | 3.54 |
| Light amber on `raised` (ticket 30) | — | — | 4.58; drawn there only as a non-text mark (§ Accent and state) |
| Dark `faint` on `raised` (ticket 35) | — | 4.77 | — |
| Dark `primary` fill on `raised` (ticket 35, non-text: the checked box of a selected row) | — | 3.56 | — |
| Dark `input` on `raised` (ticket 35, non-text) | — | 3.13 | — |

The "Dark now" / "Light now" columns of the round-18 rows are the 2026-09-29 rendered measurements on the former grounds,
**superseded by the token tables above** (light by ticket 30's grounds, dark by ticket 35's). On the current tokens the
lowest light text pair on any ground is `failed` on `raised`, 4.55, and the lowest dark text pair is `faint` on
`raised`, 4.77.

² Re-measured on tokens when Explore became rows (ticket 22, 2026-11-02; § Explore): Install is the `solid accent`
Button (`primary-foreground` on `primary`), Installed the `soft neutral` Badge (`muted-foreground` on `muted`) — dark
re-measured on ticket 35's `muted`, light on ticket 30's.

Legacy identifying input boundaries (`.search-input`, `.input` in Add / Re-point, `.settings-input`,
`.settings-select`, `.explore-search-input` — gone with ticket 22; Explore's search is the `Input` primitive) draw their edge in `input`, not the decorative `border` (ticket-07
review B2: they measured 1.33 / 1.25). Rendered edge against its outside ground, dark / light, 2026-09-29: Library
search 3.90 / 3.78, Add `.input` 3.65 / 3.87, Settings select and input 3.90 / 3.78, Explore search 3.90 / 3.78;
focused (the `accent-primary` iris edge) ≥ 6.61 / ≥ 5.71; disabled keeps the same edge. On ticket 30's light grounds
`input` measures 4.38 on `background` and 3.88 on `card`; on ticket 35's dark grounds 3.94 on `background`, 3.85 on
`card` and 3.62 on `popover`, and the focused iris edge ≥ 7.11 on those three.

The lowest passing pairs anywhere are:

- `input` on `raised`: 3.13 dark, 3.54 light (non-text);
- `primary` fill on `raised`: 3.56 dark (non-text);
- `failed` on `raised`: 4.55 light;
- `stale-foreground` on `stale/90`: 4.64 light;
- `primary-foreground` on `primary`: 4.68 dark;
- `faint` on `raised`: 4.77 dark;
- the focused-disabled ring (75 %, § Focus ring): 4.05 dark on `raised`, 3.08 light on `raised` (non-text).

Re-measure them after any change to `faint`, `primary`, `stale`, `failed`, `input` or the grounds
(`node .scratch/round19/evidence/30-contrast.mjs`). `faint` on the legacy hover ground `--bg-element-hover` (5.04 dark
on ticket 35's, 4.41 light on ticket 30's grounds) is no longer a rendered pair: the App.css alias block still defines it, but no rule or
utility reads it.

**Exempt, by rule:**

- Disabled controls.
- `border` / `border-strong` hairlines.
- ~~The legacy decorative `·` separator in the detail header (`.detail-meta-dot`)~~ — gone with the detail (ticket 12).

## Non-colour scale

| Token | Value | Utility |
|---|---|---|
| Type | 12 / 13 / 14 / 16 px (`xs` / `sm` / `base` / `lg`); tabular numerals app-wide (`body`) | `text-xs` … `text-lg` |
| Initials type | 10.5 px (`2xs`, ticket 30): **bold initials marks only** (the identity chip), never words or running text | `text-2xs` |
| Fonts | System UI stack; `ui-monospace` for identifiers; no network fonts | `font-sans`, `font-mono` |
| Radius | 4 / 6 / 10 px | `rounded-sm` / `rounded-md` / `rounded-lg` (`rounded-full` kept) |
| Row height | 36 px (rows, headers, palette items) | `h-row` |
| Spacing | Tailwind's 4 px base | — |
| Elevation | `shadow-sm`, `shadow-lg` (floating layers only) | — |
| Motion | 120 ms (`--motion-fast`); `ease-out` = `cubic-bezier(0.16, 1, 0.3, 1)` (see [Motion](#motion)) | `duration-(--motion-fast)`, default transition |
| Shell breakpoint | 1200 px (`--breakpoint-shell: 75rem`; see [Layout](#layout)) | `shell:` / `max-shell:` |

The Tailwind palette (`red-500`, `gray-900`, …) is removed (`--color-*: initial`). A palette utility therefore
generates nothing, and `styles:check` fails it anyway.

## Primitives (`src/components/ui/`)

A primitive's interface is its props:

- `variant` is visual weight;
- `tone` is semantic role;
- `size` is density;
- behaviour props come from Base UI.

**`className` is layout only**: margin, flex or grid placement, width. It never recolours, and it never changes type
or radius. A styling need that the props cannot express becomes a new documented variant (see
[How to add a variant](#how-to-add-a-variant)).

### Button (`button.tsx`)

Props: `variant` · `tone` · `size`, plus `pending`.

| Prop | Values | Meaning |
|---|---|---|
| `variant` | `solid` | Filled. `solid accent` is **the one primary action** of a view. |
| | `outline` (default) | `border-strong` edge, transparent. Secondary actions. |
| | `ghost` | No edge, and a ground (`raised`) only on hover. Toolbars and dense rows. |
| `tone` | `neutral` (default) | No semantic role. |
| | `accent` | Iris: primary action or selection-related. |
| | `ok` | Confirms a healthy outcome, used rarely. |
| | `stale` | Acts on drift ("Update"). |
| | `danger` | Destructive. The label always says what is destroyed. |
| `size` | `sm` | 24 px, 12 px text. Dense rows only (the 24 px floor). |
| | `md` (default) | 32 px. |
| | `lg` | 36 px (row height). |
| | `icon` | 32 × 32, standalone icon buttons. |
| | `icon-sm` | 24 × 24, dense rows only. |

- `pending`: the button's work is running. It gets `aria-busy`, stops being activatable, shows a spinner
  (`motion-safe` only) and **keeps focus**.
- `disabled`: not activatable, 50 % opacity (contrast-exempt); 75 % while a `focusableWhenDisabled` / `pending`
  button holds keyboard focus, so its ring stays ≥ 3:1 (§ Focus ring).
- **Held** (`disabled` + `focusableWhenDisabled`, or `pending`): the held-control refusal (§ Held-control refusal)
  — activating it changes nothing and dispatches one refusal event with its stated reason. A held button shows no
  hover ground (`not-aria-disabled:hover:*`): it takes the pointer only to say why it is refused.
- An icon-only button needs `aria-label`.

### Badge and Chip (`badge.tsx`)

**Badge** is a static label (a count or a state word). Words carry the meaning; colour only reinforces it.

| Prop | Values | Meaning |
|---|---|---|
| `variant` | `soft` (default) | 10 % state ground (opaque, into `popover`) with state text. |
| | `outline` | Tinted edge, transparent ground. Neutral `outline` on a `raised` parent has no hover delta (its hover ground is `raised`) — do not rely on it for pointer feedback there; use `ghost` (its text brightens) or a semantic tone. |
| | `solid` | State fill with its `-foreground` text. |
| `tone` | `neutral` (default) · `accent` · `ok` · `stale` · `failed` | Semantic role. |
| | `identity` (requires `hue`) | A source's identity chip: the hue's tint with the hue's text (§ Identity). **Always `soft`, always `chip`**: `variant="outline"` / `"solid"` with `identity` is a type error, and so is `hue` on any other tone. |
| `hue` | `moss` · `teal` · `cyan` · `sky` · `orchid` · `rose` · `slate` (`IDENTITY_HUES`, `ui/styles.ts`) | With `tone="identity"` only. Never chosen by hand: `skillPresentation.identityHue(owner)`. |
| `size` | `sm` (default) | 20 px. |
| | `md` | 24 px. |
| | `chip` | 22 × 22 px square, `rounded-md`, centred, 10.5 px bold (`text-2xs`): two initials or one icon. The default (and only) size of `identity`; with `solid neutral` it is the ownerless chip (Individual sources, a local folder, a non-GitHub git source: an icon on `raised` — `muted` is 1.06 on the dark header, invisible). |

**The identity rule** (ticket 30): the hue comes from a stable key — the repo owner — never from state; the chip
supplements the name, never replaces it (it sits beside the name, `aria-hidden`, and the name is the label); with 7
hues collisions are expected. Initials are `skillPresentation.identityInitials(owner)`: the first letters of the
owner's first two `-` / `_` / `.` parts, else its first two letters (`vercel-labs` → `VL`, `anthropics` → `AN`).
The chip appears only in a source group's header; rows carry no chip (operator pick A, 2026-11-02).

**Chip** is a pressable filter (All · Issues · Edited …): a toggle button with `aria-pressed`, 24 px tall. When
pressed it shows a `raised` ground, `foreground` text and an **iris edge**. The edge is the ≥ 3:1 state cue.

### Field (`field.tsx`)

`Field` · `FieldLabel` · `Input` · `Textarea` · `Select` · `FieldDescription` · `FieldError` · `FieldSet` / `FieldLegend`.

- The label is always associated with the control (`htmlFor` ↔ `id`). The Field owns the control id: the control
  reports the id it renders — its own `id` prop when given, else a generated one — and the label points at it, so an
  explicit `id` never severs the association. One control per Field.
- Controls: `Input`, `Textarea`, `Select` and `Checkbox`. Inside a Field a Checkbox renders the Field's id on its form
  input; the label points at it, and Base UI names the `role="checkbox"` element from that label
  (`aria-labelledby`). A group of checkboxes is a `FieldSet` with a `FieldLegend`.
- Helper and error text form the control's `aria-describedby`.
- `invalid` sets `aria-invalid` and a `destructive` edge.
- A placeholder is never the label. Placeholders are `faint`.
- `size` on `Input` / `Select`: `sm` is 24 px (dense), `md` (default) is 32 px.
- The boundary is `input` (≥ 3:1).

### Checkbox and Switch (`checkbox.tsx`, `switch.tsx`)

- **Checkbox**: a 16 px box with a 24 × 24 hit area. It reports `aria-checked` true, false or mixed. Its edge is
  `input`, and when checked it fills with `primary`.
  - `focusableWhenDisabled` (ticket 17): the Switch's option, for a checkbox disabled *because its own action runs*
    (a matrix cell mid-toggle) — it keeps its Tab stop and focus, says `aria-disabled="true"` (never
    `aria-readonly`) and refuses every change; Base UI renders it read-only. `checkbox.test.tsx` pins it. Held
    (`disabled` with it), it follows the held-control refusal (§ Held-control refusal).
- **Switch**: `role="switch"`. `size` is `md` (20 × 36, standalone) or `sm` (16 × 28 with a 24 px hit area, dense
  rows). "On" is **neutral** (`muted-foreground`), not iris: a column of switches must not make the accent mean
  "normal". The thumb slides in 120 ms, and reduced motion removes the slide.
  - `focusableWhenDisabled` (ticket 15): a switch disabled *because its own action runs* keeps its Tab stop and
    focus, says `aria-disabled="true"` (never `aria-readonly`) and refuses every change — click, Space, Enter. Base
    UI's Switch has no such option (1.8), so the primitive renders it read-only while it holds focus. The row
    switch uses it with the refusal reason as `title` + `aria-description` (D19, D21); `checkbox.test.tsx` pins it.
    Held (`disabled` with it), it follows the held-control refusal (§ Held-control refusal).
- **Held-control refusal** (ticket 27, D19; `ui/held-refusal.ts`, pinned for all three primitives by
  `held-refusal.test.tsx`). A Button, Switch or Checkbox *held* disabled by a reason (`aria-disabled="true"` with the
  `data-focusable-when-disabled` mark) never refuses silently:
  - **Activation** is a pointer click (`click` with `detail ≥ 1`), Enter or Space (`keydown`, never an auto-repeat).
    The click a key press synthesizes (`detail: 0`) is the same activation and is not counted again. Each activation
    dispatches one bubbling `skillshub:held-refusal` event whose `detail.reason` is the control's stated reason — its
    `aria-description`, else its `title`, else `null`. A Checkbox or Switch label is part of that hit target:
    its forwarded click on Base UI's hidden input emits the same refusal, using the visible control's reason.
    Direct-control and key-forwarded input clicks are not counted again; the public `inputRef` still works.
  - **Nothing changes**: the control stays as it was, nothing is queued, focus stays on it (never `<body>`).
  - **Pointer**: a held control takes pointer events (`disabledState` keeps `pointer-events: none` for every other
    disabled control), so a click lands on it rather than on the row beneath.
  - **The notice is the binder's, not the primitive's.** App's `useHeldRefusal` hands each event to the reporter's
    `refuseHeld`, which shows the one refusal toast (`info`, 2 s, id `lifecycle-refusal`, shared with a refused
    `runAction`): the stated reason — else the running action, named ("Disable is running — wait for it to finish")
    — over "Nothing was changed." A repeat while it shows refreshes the same toast (its timer restarts) instead of
    stacking a copy, so the toaster's polite region announces it once. The primitive renders no live region, and a
    control stating no reason while nothing runs shows nothing. Like every toast, it yields to the open
    Notifications panel.

### Tabs and Segmented (`tabs.tsx`, `segmented.tsx`)

- **Tabs** (`tablist` / `tab` / `tabpanel`, arrow keys) take `variant` on `TabsList`:
  - `line` (default): an iris underline marks the selected tab. Use it for switching sections.
  - `segmented`: a `raised` segment with an iris inset edge marks it. Use it for a compact switch of views.
- **Segmented** is a single choice among 2–5 options that does **not** switch panels (Grouped / Flat). It is a named
  group of `aria-pressed` toggles that always has exactly one selection.

### Dialog and ConfirmDialog (`dialog.tsx`, `confirm-dialog.tsx`)

**Dialog** is the one modal shell. It replaces `Modal.tsx` and is controlled (`open` / `onOpenChange`). It owns:

- **Focus.** It moves focus in on open (Base UI; `initialFocus` accepts a ref or a function of the interaction type)
  and traps it while open. On close it returns focus to the element that opened it. If that element is gone, focus
  goes to `fallbackFocus(usable)`, then to the layer beneath, then to the app's main landmark (`mainLandmarkFocus`,
  `ui/focus-fallback.ts`: `<main>` made programmatically focusable with `tabindex="-1"`, never a Tab stop). While the
  app renders a `<main>`, focus does not land on `<body>`.
- **Every return target is tested**, the fallback's as well as the opener: connected, and not inert once this layer
  has closed. `fallbackFocus` receives that `usable` test (type `FocusFallback`) and walks its own chain with it —
  the skill dialogs' chain is the focused row, the inspector's heading, the surface heading, so beside the list the
  row takes focus and inside the sheet (which keeps the row inert) the sheet's heading does (ticket 12 I1).
- **A layer closing beneath another moves no focus.** Its return chain (opener + fallback) passes to every layer
  above that it opened (whose opener lay inside it, or is gone): a count confirmation closed while the overwrite ask
  its action raised is still up hands its chain to the ask, which returns focus by it when it closes
  (`layer-stack.handOverReturn`). Progress is never a layer (D19, ticket 14).
- **Opened from a menu item** (a row's "More actions" → Change source…), the opener is the menu's trigger (the
  popup's `aria-labelledby`), not the item, which is gone once the menu closes (ticket 14, D21).
- **Where the opener can disappear** (a removal: Delete skill, Remove project), the dialog passes `fallbackFocus`.
  Today both removal dialogs pass `mainLandmarkFocus`; the row-level target (the next row, then the list heading) is
  ticket 09's surface lifecycle, which will supply it through the same prop.
- **Background.** Everything behind the top modal layer is `inert`, not only `aria-hidden`: the app and every lower
  layer's popup — a Popover beneath it included.
  The one exception is the toaster's live host (§ Toaster): not a layer, never inert, so a toast raised under a modal
  is announced and its action takes a pointer.
- **Dismissal.** Escape and outside press dismiss **the top layer only** — a dialog or a Popover (below) — and one
  event dismisses one layer (`layer-stack.claimDismissal`), and only when `dismissible`. A
  non-dismissible layer also disables its ✕. `interceptEscape` (on `Dialog`) is offered the top layer's Escape
  first; answering `true` spends it (the inspector sheet collapses expanded Files) and the layer stays open.
- **Name.** `<DialogTitle>` names the popup through `aria-labelledby`.
- **Close.** The corner ✕ is 32 × 32 and named.
- **Size.** `sm` 400 · `md` 520 (default) · `lg` 600 · `xl` 720, each capped at viewport − 48.
- **Placement** (`DialogContent placement`): `center` (default) is the card above; `top` is the same card pinned
  12 vh from the top edge (`max-h` 76 vh, the same sizes) — for a popup whose content grows and shrinks as one
  types, so its input never moves (the ⌘K palette, ticket 23); `sheet` is a full-height panel on
  the right edge, `min(560px, 100vw − 56px)` wide so the 56 px icon rail stays visible, with a `border-strong` left
  edge. Same focus, inert, dismissal and no-motion rules. The inspector below the shell breakpoint is its one user
  (ticket 12); its width grows to `100vw − 56px` (layout `className`) when its Files expand.
- **Focus return is decided while the layer is still open**, when everything behind it carries the stack's own
  `inert`. So the opener (and the main landmark) are judged by `staysInertAfterClose` — only inert that outlives
  the layer disqualifies them (ticket 12: the plain `closest("[inert]")` check sent every opener to the fallback in
  Chrome). A layer opened with focus nowhere returns it nowhere. The landmark is focused itself,
  not its first control (Base UI would hand a non-tabbable return element's focus to its first tabbable child).
- **Keep the shell mounted.** A dialog that is mounted only while open never returns focus (Base UI returns it on
  close, and an unmounted popup does not close). Re-point, Invocation, Detach and Deploy to project keep one `Dialog` +
  `DialogContent` for the app's life and key only their body per subject.
- **Body.** `DialogBody` `variant`: `default` (padded) or `flush` (edge-to-edge lists).
- **Motion.** None: dialogs appear and leave instantly.

**ConfirmDialog** asks one question with two answers. Escape or an outside press means "cancel", the safe answer,
and there is no ✕.

- `tone: "danger"` puts the destructive button on the right and initial focus on Cancel.
- `tone: "accent"` (default) puts focus on Confirm.
- `initialFocus: "cancel" | "confirm"` overrides the tone's default. `cancel` is for a large answer that is not
  destructive: the bulk rail's count-bearing confirmations (Disable / Enable / Update / Deploy over several skills) open
  on Cancel with an `accent` Confirm (ticket 13).
- `pending` shows Confirm as busy and disables Cancel.

The asks are `useSurfaceState` ask layers that settle exactly once: the shared-dir confirmation
(`useSharedDirConfirmation`) renders through ConfirmDialog; the overwrite ask (`useSyncOrchestration`, per-row
choices) uses the Dialog parts directly.

### CommandPalette (`command-palette.tsx`)

The ⌘K palette's shell (ticket 23): a search input over a grouped list of commands, inside the Dialog. **Base
(decided 2026-11-02):** Base UI `Autocomplete`, rendered `inline` inside our `Dialog` (`placement="top"`, `size="lg"`,
no ✕) — Base UI's own command-palette pattern — not shadcn Command / `cmdk`. Every D5 / a11y requirement the
fixture audit checked is met by the Base UI route, so `cmdk` (and the one Radix dialog it drags in, ADR-0006 §3)
stays out: the palette is a layer of the one layer stack like every dialog, with no second dialog implementation.

- **No `variant`, `tone` or `size`.** There is one palette; its props are its content (`groups`, `query`,
  `onSelect`) and its names (`label`, `inputLabel`, `placeholder`, `emptyText`, `footer`).
- **Semantics.** The popup is a `dialog` named by `label`. The input is a `combobox` (`aria-autocomplete="list"`,
  `aria-expanded="true"`) that `aria-controls` a `listbox`; each group is a `group` named by its label; each command is
  an `option`. DOM focus stays in the input; the highlighted option is its `aria-activedescendant` (↑ / ↓ move
  it, Enter runs it; Home / End stay with the text). Nothing is highlighted on open — recents come first, so a reflexive ⌘K → Enter must not repeat the last Update all; the first match is highlighted once a query is typed, and ↓ reaches a recent without typing. The foot (key hints)
  describes the input. "Nothing matches" is Base UI's polite `status`.
- **Filtering is the caller's.** The primitive filters nothing (`filter={null}`): `groups` are what to show for
  `query`; empty groups are skipped.
- **Rows.** `h-row` (36 px), `sm` `foreground`; a quiet hint on the right (`xs muted`). The highlighted option has the
  `raised` ground (the current item's tone, ticket 35 — in dark `accent` is the palette's own `popover` ground) **and**
  the inset ring (2 px `ring`, offset −2 px: 6.14 dark / 4.72 light on the ground, 7.11 / 5.86 on `popover`) — the
  ground alone (1.16 / 1.24 on `popover`) is never the cue. Group labels `xs` medium `faint` (5.52 / 6.24 on
  `popover`); hints `muted` (7.65 / 8.36 on `popover`, 6.61 / 6.75 on the highlight). The input draws no ring of its
  own: it is the popup's one control and always focused.
- **A refused command** (`refusal`: something runs, or it would change nothing) stays listed and reachable —
  `aria-disabled`, its reason replacing the hint. Activating it (Enter, a click) runs nothing and dispatches the held
  refusal (`held-refusal.dispatchRefusal`), which the binder shows as a toast — as a held Button does (D19).
- **Focus, inert, dismissal** are the Dialog's: focus to the input on open, trapped; the app behind inert; Escape and
  an outside press close it (top layer only — over the inspector sheet it closes alone); focus returns to the
  opener, else to `fallbackFocus` (the palette passes D21's chain), never `<body>`. `onOpenChangeComplete(false)`
  tells the caller the palette is gone (the shell waits a few frames before acting, since focus return follows it).
- **Motion: none** (§ Motion). Pinned by `command-palette.test.tsx`.

### Popover, DropdownMenu, Tooltip (`popover.tsx`, `dropdown-menu.tsx`, `tooltip.tsx`)

These are floating surfaces on `popover` with a `border` and `shadow-lg`. All three **appear instantly**.
Escape closes them and returns focus to the trigger.

**Elevation on black** (ticket 35): on the dark `#000000` ground a black shadow draws nothing, so a floating layer is
told apart by its 1 px edge and its `popover` ground (1.06 on `card`, 1.09 on the ground) — every one already draws
the edge in the primitive: Popover and DropdownMenu `border` (`floatingSurface`, `ui/styles.ts`), Dialog — the ⌘K
palette included — and Toast `border-strong`. The Tooltip needs none: it is inverted (`foreground`, 19.26 on the dark
ground). No primitive relied on its shadow alone; the shadow stays for light, where it still reads.

- **DropdownMenu** items are 32 px tall. `variant="destructive"` both words the action and tints it.
  - The highlighted item takes the `raised` ground — the current item's tone (ticket 35: in dark `accent` equals
    `popover`, so an `accent` highlight would draw nothing; in light the two are one value, so light is unchanged).
  - The item holding keyboard focus draws the ring **inset** (`insetFocusRing`: 2 px `ring`, offset −2 px, so the
    scrolling popup cannot clip it): 6.14 / 4.72 against the highlight ground, 7.11 / 5.86 against `popover`. The
    highlight ground alone (`raised` on `popover`, 1.16 / 1.24) is never the cue. A pointer highlight shows the
    ground only.
  - `DropdownMenuLabel` names a group and must sit inside a `DropdownMenuGroup` (Base UI throws otherwise).
  - **axe `region` on an open menu is an accepted exception** (ticket 17 audit, recorded by ticket 23): the open
    menu is a named, focused `role=menu` portal directly under `body`, a transient composite popup — not a document
    region. Do not wrap it in a landmark to green axe; any other `region` finding is still a defect. (The ⌘K palette
    is a `dialog` and passes `region`.)
- **Tooltip** is a hint on an inverted surface. It is never the only carrier of a name or a state.
- **Popover is a floating layer** of the one layer stack (ticket 16 A1): non-modal (it makes nothing inert), live above
  the top dialog, inert beneath one. A confirmation one of its controls raises — Add's harness scope → the shared-folder
  ask, a sibling dialog in its own portal — sits above it: Escape, the press on the ask's answer and the focus the ask
  takes belong to the ask (the popover ignores them, `claimDismissal`), and the ask returns focus to the checkbox that
  raised it with the popover still open — its controls back in the tab order (Base UI takes them out while focus is
  away and restores them only for a focus arriving from a known element; the primitive restores them for a focus
  returned from a layer that is gone). The next Escape closes the popover alone → its trigger; the next closes the
  dialog → its opener. DropdownMenu and Tooltip are not stack layers (nothing raises a dialog from inside them while
  they stay open).
- **Popover without a trigger** (`PopoverContent anchor`): a control that raises a popover on its own terms — the
  row switch's inline Enable confirmation — renders a controlled Popover with no Trigger, positioned on `anchor`,
  with `initialFocus` on its answer and `finalFocus` back on the control. Escape closes it; focus returns to the
  anchor (ticket 13, `popover.test.tsx`).

### Toaster (`sonner.tsx`)

The toaster is `sonner`, themed entirely by tokens: every colour it draws is one of our variables, so no JavaScript
theme value is needed. State toasts put state-coloured text on an 8 % tint of that state over `popover` (ratios
above). `useStatusReporter` owns toast lifetime; the binder mounts the one `<Toaster>`. A run shows **one** toast —
its composite headline (no description) with **View report**, which opens the Notifications panel on that run — and
no toast is shown while the panel is open (opening it dismisses those on screen): a toast never covers its header
(ticket 14).

**Live above every layer** (ticket 29). The primitive renders sonner into its own host — the first `body` child,
outside the app root, marked `data-skillshub-live` (`layer-stack.LIVE_HOST_ATTRIBUTE`) — and the layer stack never makes
that host inert. So a toast raised while a Dialog, ConfirmDialog or the inspector sheet stands open (a refusal, a run's
outcome) is announced by sonner's one polite region and its **View report** works by pointer. The toaster is **not a
layer**: it opens nothing on the stack, dismisses nothing and claims no event. Under a modal, Tab still cycles inside
the top layer (Base UI's focus guards sit in its popup), a press on a toast is not an outside press (a modal dialog
closes only on its own backdrop), and Escape still dismisses the top layer (sonner only collapses its expanded stack).
With no modal open the Tab order is unchanged: first in the body is where it was as the app's first child. Sonner's
hotkey (⌥T) can still move focus into the toaster under a modal; leaving it, sonner returns focus where it was. The
props are sonner's own; the host is the primitive's.

**Placement** (ticket 31, BACKLOG #80): `bottom-right`, lifted `6rem` above the window's foot (`offset={{ bottom:
"6rem" }}` in `App.tsx`). The header's verbs — the next action after a notice — and the filter bar are never under it,
nor the bulk rail (41 px at 960 and 1280), a dialog's foot or the inspector's; it passes over list content, which
scrolls. Measured on the fixture with the rail up: at 960×640 the toast spans y 491–544 (rail 599–640), at 1280×800
y 651–704 (rail 759–800); no header, filter-bar or rail control intersects it
(`.scratch/round19/evidence/31/toast-dark-960.png`). Pointer events and hover-pause are sonner's own — a notice under
the pointer stays while it is read, and **View report** still takes a click; neither is disabled to make room.
The stack is **three** toasts (`visibleToasts={3}`, sonner's default; ticket 31 audit F2): five persistent errors
expanded under the pointer climbed to the header at 960×640 — three expanded multi-line errors stay below the bulk
rail (~130 px each from the foot). A run is one toast anyway (above); a fourth notice queues behind.

Motion (ticket 15): sonner's own 400 ms slide, fade and height glide are replaced, inside the primitive, by the
world's 120 ms (`--motion-fast`) transform + opacity on `ease-out` — inline styles on the toaster and each toast
(they outrank sonner's injected CSS), an important child utility for the content fade of a stacked toast. The toast
enters from and leaves through its own edge; its height never animates; a swipe-out takes the same 120 ms. Reduced
motion: sonner's `transition: none !important` and the base layer both still win.

## Focus ring

- One ring everywhere: a **2 px solid `ring` outline, offset 2 px** (1 px for text fields, −2 px inside menu items),
  on `:focus-visible` only.
- **The style is explicit.** Tailwind 4's `outline-none` sets `--tw-outline-style: none` and `outline-2` draws
  `outline-style: var(--tw-outline-style)`, so every ring carries `focus-visible:outline-solid` (`focusRing`,
  `fieldFocusRing`, `insetFocusRing` in `ui/styles.ts`). `ui/focus-ring.test.tsx` resolves each primitive's rendered
  classes against CSS compiled by the installed Tailwind; a trusted-keyboard run measured `solid 2px` on every
  primitive in both themes (ticket 07, 2026-09-29).
- **No transition**: the ring never animates in.
- It measures ≥ 4.72:1 against every ground in both themes (dark ≥ 6.14, on `raised`).
- **A control that holds focus while disabled** (`focusableWhenDisabled`, `pending` — it keeps focus while its action
  runs, or is refused with a reason) is drawn at 50 % opacity, and the ring with it: 2.53–2.60 dark / 2.19 light. While it
  holds keyboard focus it steps up to **75 %** (`aria-disabled:focus-visible:opacity-75`, in the shared
  `disabledState`), so the ring measures **4.60 dark / 3.49 light** (≥ 3:1; on ticket 35's dark grounds 4.60 / 4.55 /
  4.43 / 4.05 / 4.60 on background / card / popover / raised / row, floor 4.05 on `raised`; on ticket 30's light grounds
  3.54 on `background`, floor 3.08 on `raised`) and the control still reads dimmed
  (ticket 15, measured on Add skill and a row switch mid-Update all). Such a control carries the
  `data-focusable-when-disabled` mark (Button, Switch): when a Popover restores its controls' Tab order after a layer
  above it closed, that mark is what keeps a focusable-disabled control in the order, while a control that became
  disabled meanwhile (`aria-disabled` without the mark, or native `disabled`) stays out (ticket 16 re-check R2).
- Primitives draw it through the ring constants in `ui/styles.ts`. The base layer (`index.css`) gives every other element the
  same ring, legacy surfaces included.
- A legacy rule that removes the outline must replace it with an `accent-primary` (iris) edge. Every current one
  does.

## Motion

Frequency first (round-18 D6, Emil's rubric): what the operator does tens or hundreds of times a day does not move;
what moves is **opacity and transform only**, **≤ 120 ms** (`--motion-fast`), on `ease-out`
(`--ease-out: cubic-bezier(0.16, 1, 0.3, 1)`, `theme.css`), interruptible (CSS transitions, which retarget — never
keyframes on anything that can fire twice), and **gone under `prefers-reduced-motion: reduce`**. No `transition: all`
anywhere: the built CSS has none, `App.css` included since ticket 15. Every motion is owned by a primitive or by the
one component that owns the moving element — never by a call site's `className`.

### What moves (the motion table)

| What moves | When (frequency) | Property | Duration | Easing | Reduced motion | Owner |
|---|---|---|---|---|---|---|
| Switch thumb | a switch is toggled (tens a day) | `translate` of the thumb | 120 ms | `ease-out` | no slide (`motion-reduce:transition-none`) | `ui/switch.tsx` |
| Bulk rail | the first row is selected (occasional) | opacity 0 → 1 and `translate` 8 px → 0, **entry only** (`@starting-style`); it leaves instantly | 120 ms | `ease-out` | appears settled | `skills/BulkRail.tsx` |
| Deployments strip light — the signature interaction | a mark **becomes** deployed on screen (a Deploy, Retry, Enable or Update completing); never on mount, filter or scroll | opacity 0 → 1 of the new glyph (`@starting-style`) | 120 ms | `ease-out` | shown at once | `skills/StripMarkIcon.tsx` + `useSyncLight` |
| Header progress fill | each tick of a streamed action | `transform: scaleX`, retargeting tick to tick | 120 ms | `ease-out` | jumps | `shell/HeaderProgress.tsx` |
| Toast | once per settled run (occasional) | `transform` from its own edge + opacity; never height | 120 ms | `ease-out` | none | `ui/sonner.tsx` |
| Spinners: Button `pending`, header progress, Files loading, a matrix cell in flight | while work runs | rotation loop (`animate-spin`) | 1 s period | linear | hidden (header) / not running (`motion-safe:` only) | `ui/button.tsx`, `HeaderProgress`, `SkillFiles`, `projects/AssignmentMatrix` |
| Install landing (the rare moment, ticket 16) | an Add's settlement rows settle (a few times a week) | opacity 0 → 1 of a row's settled glyph (`@starting-style`; added / failed / cancelled), **staggered 20 ms per row** (capped at 8 rows) — a sequence of 120 ms steps | 120 ms each | `ease-out` | shown at once | `skills/add/AddSkillDialog.tsx` |
| ⌘K palette (ticket 23) | open / close (many times a day, from the keyboard) | **nothing** — decided 2026-11-02: it appears and leaves instantly, like every dialog | — | — | nothing to remove (0 transitions, 0 animations measured under `reduce`) | `ui/command-palette.tsx` |

The loops are the only durations over 120 ms in the built CSS, and they are periods, not transitions: `animate-spin`
1 s, and the legacy `.spinner` 0.9 s in `App.css` — all stopped by the base layer's reduced-motion rule. (`.cell-spinner`
and the `.skeleton-row` shimmer left with Projects' migration, ticket 17: a cell in flight spins `animate-spin`
`motion-safe`, and the loading placeholders are static `raised` blocks.) **The two permitted rare moments** (D6's delight
budget: a large import completing, an install landing) may run as a *sequence* of ≤ 120 ms steps: the import cascade
(§ Empty and first-run, ticket 21 — the strip light above, staggered 20 ms per mark; not built yet) and an install
landing (ticket 16, built: the settlement glyphs above — nothing else in the dialog moves, the rows' words are there
at once, and the strip light lights the new skill's marks only when they become deployed on screen).

### What does not move (decided)

- **Row focus, selection and hover**: instant ground and edge changes. `j` / `k` / `x` are keyboard actions, and
  keyboard actions never animate.
- **The inspector** opening, closing and switching skills beside the list, and **the sheet** below 1200 px: both
  follow every row click, and a panel that slides on each click is exactly the frequent motion Emil forbids.
- **Dialogs, popovers, menus and tooltips** appear and leave instantly. **The ⌘K palette** too (ticket 23): it
  opens tens of times a day from the keyboard, and its reserved 120 ms row would only delay the input the operator is
  already typing into (Emil: frequency first).
- **The header's exception summary ↔ progress swap** is instant: a crossfade would show two statements at once, and
  the swap happens at the start and end of every run.
- **Hover and press** change colour instantly in every primitive (no press scale — "practical, not flashy"). The
  focus ring never animates in. Disclosure chevrons turn instantly.
- **Files expand / collapse** and the Deployments list ↔ grid toggle are instant layout changes.

### Reduced motion

`prefers-reduced-motion: reduce` removes all motion. The base layer zeroes every transition and animation duration
(`!important`; legacy `App.css` loops and third-party layers included), and each moving element also carries its own
`motion-reduce:` / `motion-safe:` counterpart, so nothing is left mid-way: the rail and the strip light appear
settled, the fill jumps, the header spinner is hidden, Button spinners do not run — text and counts only. The
guarantee is **no interpolated motion**: every element is settled on its first *visible* frame. Sonner still stages
a toast for one DOM frame at opacity 0 before it mounts (`data-mounted=false`, no transition, nothing running) —
invisible, not motion — so "first DOM frame" is not claimed for toasts.

## Target size

- **24 × 24 CSS px** is the floor for dense controls (Button `sm` / `icon-sm`, Chip, Checkbox and Switch `sm` hit
  areas, strip marks, matrix checkboxes with their hit area extended to the cell).
- **32 × 32** is the floor for standalone icon buttons and the dialog Close.
- Primitives own the floor, and call sites never shrink it.

## Prose: the one exemption

- Rendered Markdown and syntax colours style content we do not author as JSX, so utilities cannot reach it. They live
  in `src/styles/prose.css`.
- Every selector there is scoped under the **`.prose`** root, and the file uses tokens only, with no colour literal
  (`styles:check` rule 4).
- The highlighter renders class output (`useInlineStyles={false}`) with an empty theme object, so its default light
  theme never reaches the page.
- `prose.css` is not a place for surface styling.

## Theme

- `useSettingsState` is the single writer of `data-theme` and `color-scheme` on `<html>` and returns `resolvedTheme`.
- Before first paint, the inline script in `index.html` sets both from the persisted preference (key `skills-theme`),
  so a light user never sees the dark-first `:root`. It is the one sanctioned storage read outside
  `src/lib/preferences.ts`, and `src/lib/preferences.test.ts` pins the key literal equal.
- Styling reads the theme only through tokens. Reading `data-theme` from the DOM is forbidden.

## Layout

- **Minimum window**: 960 × 640 (`tauri.conf.json`).
- **Breakpoint**: one structural collapse at **1200 px** (`--breakpoint-shell: 75rem` in `theme.css`, the
  `max-shell:` variant; spec D23; `SHELL_WIDE_QUERY` in `hooks/useMediaQuery.ts` is the same value for script). Below
  it the sidebar is a 56 px icon rail (ticket 11), the inspector a sheet (ticket 12) and the Projects list a header
  select (ticket 17).
- **How 1200 was measured** (ticket 11, fixture `failures`, 2026-09-29): the 36 px row's fixed columns (checkbox
  32 · strip 8 marks × 16 + N/M 40 = 168 · scope 24 · switch 40 · overflow 32 · six 12 px gaps · 24 px padding)
  take 392 px; name and description share the rest in one track, the name group holding at least 60 % of it (the
  former 1.5 : 1 split, so descriptions start on one line down the list). The widest name-plus-exception-marks
  group in the fixture is **347 px** (`scratchpad` with two marks). At 1200 with the 200 px sidebar the list pane
  is 970 px and the name share exactly 347 px, so the sidebar folds to the rail there, which returns 144 px
  (1199 → pane 1113, name share 433).
- **Names before descriptions** (ticket 11 audit A3, 2026-09-29): a name group wider than its 60 % share takes the
  description's room — the description truncates, then collapses — so the name keeps its words at the 960 minimum.
  There, with the rail, the shared track is 524 px with overlay scrollbars and 509 px with a classic 15 px list
  scrollbar (name share 307 / 298): `scratchpad` + two marks takes 347 px and reads in full while its description
  yields 40 / 49 px; every other row keeps the one description line. A name truncates (full name as the hover
  title) only when name + marks exceed the whole track — beside two marks at 960, a name over 220–235 px.
  No horizontal scroll at either size. Below a 768 px *list pane* (a container query, `@max-3xl`) the description
  goes.
- **Superseded by ticket 31 (D39, 2026-11-04)**: the shared name/description track and its 60 % floor are gone. The row
  is `32 · --name-col · 1fr · strip (marks × 20 + 40) · scope 56 · 40 · 32` (§ My Skills); the two bullets above are
  the 1.3.0 measurement, kept as history. Re-measured on the fixture (`rich`, 7 harnesses, `--name-col` = 34 ch =
  278 px): at 960 the description keeps 141 px, at 1280 317 px, at 1800 837 px — one line everywhere, no horizontal
  scroll. Below a 48 rem list pane the description still goes and the name takes `1fr`.

## My Skills (the first migrated surface, ticket 11; row and group anatomy, ticket 31)

Composed from the primitives and tokens only; no variant was added. Files: `components/shell/AppSidebar.tsx`,
`components/skills/{LibraryHeader,FilterBar,SkillsList,SkillRow,SyncedStrip,listColumns}.tsx`.

- **Shell.** Sidebar 200 px on `card` (`sidebar` tokens): the areas My Skills · Explore · Projects, then at the
  foot Notifications (unread count as a `solid accent` Badge) · Theme (a menu) · Keyboard shortcuts (a popover) ·
  Settings. Items are 32 px; the current area has `aria-current="page"`, a `raised` ground and an iris icon
  (6.14 dark / 4.72 light on `raised`); hover is `sidebar-accent` (in dark the panel tone, a step below `raised` —
  ticket 35). In the icon rail every label stays in the accessibility tree (`sr-only`)
  and is the hover title.
- **Header row** (48 px + its 1 px hairline: 8 px around 32 px controls): title (`lg`, the `data-surface-heading` focus target) + count (`faint`) · the
  exception summary · a status region reserved for ticket 14's progress · Configure harnesses (`ghost`) · Update
  all (`outline`) · **Add skill** (`solid accent`, the one primary). The summary reads the health fold over the
  whole library: "N failed deployments" (`failed` text) · "N out of date copies" (`stale` text) · "View N skills with
  issues" (`ghost accent` Button that presses the Issues chip). Zero parts are dropped; a healthy library shows
  nothing. A skill has an issue when its loudest fold row is failed or out of date — the Issues chip keeps exactly those.
- **Filter bar** (48 px + hairline, measured ticket 15 — 8 px around the 32 px Segmented and Sort controls; chips and search are 24 px; every bar is a 4 px multiple plus its hairline, rows are 36 px including theirs): search (`Input sm`, labelled "Search skills" through a Field; `/` focuses it, Escape
  clears it; Enter or ↓ moves focus to the first row shown — a rule of the field, the list keymap still ignores
  editables; `autoCorrect` / `autoCapitalize` off and `spellCheck={false}` on every `type="search"` field) · Chips
  All · Issues · Edited · Disabled with their counts (`faint`) · Segmented Grouped / Flat · one "Sort and view" menu
  (radio groups, then — while Grouped — a "Groups" group: Collapse all · Expand all). Auto-deploy moved to Settings;
  library-wide Disable is the bulk rail's (ticket 13) — nothing library-wide is one click away.
- **Columns** (`listColumns.ts`, one template for the row and its group header): checkbox 32 · name `--name-col` ·
  description `1fr` · strip `--strip-marks` × 20 px + 40 (N/M) · scope 56 · switch 40 · overflow 32, 12 px gaps.
  The list publishes both measures on its pane, as it always did `--strip-marks` (the widest strip in view, collapsed
  groups included so a collapse never shifts the columns). **`--name-col`** (D39, `skillPresentation.nameColumnWidth`)
  is `clamp(12rem, N ch, 22rem)` where N is the longest *shown* row's name + 2 ch, + each exception mark's words + 5 ch
  (icon, padding, gap), + 4 ch for the Edit pencil; a collapsed group's rows are not shown, so they do not count. The
  pane sets `text-sm`, so `ch` resolves at the names' own size. A name wider than the column truncates (its title is
  the full name); the description takes the rest, truncates on one line, and carries its full text (source label ·
  description) as its title. Below a 48 rem list pane (`@max-3xl`, the inspector beside it) the description goes and
  the name takes `1fr`. No row carries a source chip (operator pick A).
- **Row** (36 px, `h-row`, no hairline — the card holds the rows; D38): on the card's `row` ground; hover `accent`
  (ticket 35); the **focused** row (the inspector's, `aria-current`) is a `raised` rounded block (`rounded-md`), inset
  4 px from the card's edges (`mx-1 px-2`, so its columns stay put) with a 1 px inset `ring`; a **selected** row is
  `raised` with the 3 px iris edge (ticket 13) — both together when both hold. Cells: checkbox · name (`sm` medium;
  the Tab stop that opens the skill) + Edit pencil (`ghost icon-sm`, only when overridden) + **exception marks** (soft
  Badges: `failed` with a ⊗ icon, `stale` with a ⚠ icon, `neutral` otherwise; the fold's rows minus the silent ones
  — `skillPresentation.rowMarks`: Disabled at either grain (the strip and the switch say it) and **Not deployed (D40,
  never a row mark** — outright false for a skill that lives in projects; the strip's hollow marks say it globally;
  the kind stays in the inspector ladder, the group summary and Deploy all's count). A target kind reads in the group
  summary's short form — "2 failed", "1 missing", "1 out of date" (`ROW_MARK_LABEL_KEY`) — and every mark's tooltip
  carries the ladder's full sentence, then what happened and where ("1 deployment failed — In quartermaster
  (.agents/skills)", `healthRowPlaces`). In a list narrower than 48 rem the marks compact to their icons (`neutral`
  gains ⊖), the words kept as accessible text) · description (`muted-foreground`; in Flat and in "Individual sources"
  preceded by the source in `font-mono xs faint`) · **Deployments strip** (`SyncedStrip.tsx`) + N/M · **scope** ·
  Switch `sm` (= skill-grain Disable / Enable through the door; `focusableWhenDisabled`, so a pressed switch keeps
  focus while its action runs and every switch says why it is refused meanwhile) · overflow (`ghost icon-sm` →
  DropdownMenu: a "Fix" group of the fold's repairs, then Update, Change source…, Detach, Edit invocation…, Remove).
  Clicking anywhere but a control opens the skill.
- **Scope cell** (D41, `skillPresentation.skillScope`): a globe when some global deployment is live (`deployAction` =
  deployed), and/or a folder + N when N projects hold at least one live deployment (`isLiveTarget` over the skill's
  `projects`, now on the wire — `ManagedSkillDto.projects`). One `role="img"` with one name, also its title:
  "Deployed globally · In 3 projects: agent-skills, quartermaster, skills-hub"; a project-only skill shows the folder
  alone. `xs faint` on the row: 6.00 dark / 6.24 light on `row`, 5.52 / 5.04 on `accent` (hover), 4.77 / 5.04 on
  `raised` (focused, selected). N/M stays global. A failed or missing project deployment joins the row's "N failed" /
  "N missing" mark (and the header's failed count, the Issues chip, the ladder) with its project named.
- **Group card** (D38): each source group is a card — `border` + `rounded-lg`, `overflow-clip` so the radius clips
  its rows, rows on `row` inside; cards stack 12 px apart inside 16 px of list padding. Flat is one card without a
  header. In dark the card's `border` measures 1.17 against `row` (1.14 against `card`) and the header's `card` is a
  1.02 pane step on black; the border carries the shape (light: 1.42 / 1.25, header 1.13).
- **Group header** (40 px, sticky to the top of the scroll area within its card, `card`, a `border` hairline under it
  unless collapsed; it shares the row's column template): checkbox (the group's skills in / out) · then, across the
  name and description columns, **chevron** (`ghost icon-sm`, `aria-expanded`, named "Collapse {source}" / "Expand
  {source}") · **source chip** (ticket 30's pick A, header only, `aria-hidden` — the name beside it is the label: a
  GitHub owner's `Badge tone="identity"` with `identityHue(owner)` / `identityInitials(owner)`; otherwise the neutral
  `size="chip"` Badge with the source's icon — layers for "Individual sources", a folder for the local group, a branch
  for a git host other than GitHub) · name (`h2`, links to the repo; the text inside the link truncates, never an
  ancestor, so the link's ring paints whole) · count (`faint`; hidden below a 48 rem list, where the name and the
  summary need the room) · state summary ("1 failed · 2 out of date · 1 not deployed · 1 disabled", state-coloured
  words; it yields first, `shrink-8`) · **harness initials** over the strip column (list view only) · **Deploy all**
  (`ghost sm`, across scope · switch · overflow; disabled with a reason when every skill is deployed).
- **Harness initials** (D42, `skillPresentation.harnessInitials`): one 2-letter mark per harness of the strip's set,
  in its order, each 12 px wide on the strip's 20 px pitch so it sits over its mark — `text-2xs` (10.5 px) bold
  `faint`, `role="img"` named with the Harness catalog's full label (also its title), inside a group named
  "Harnesses". `faint` on `card`: **5.86 dark / 5.53 light**. The rule: the first letters of the label's first two
  words, else its first two letters (Claude Code → CC, Cursor → CU, Gemini CLI → GC, Kimi Code CLI → KC); a pair
  already taken in the set keeps its first letter and takes the next unused letter of its label (Codex CO, CodeBuddy
  CD, Continue CN). The fixture's harnesses read CU · CC · CO · AM · KC · PI · GC (+ WI for Windsurf) — eight
  distinct pairs, no tiebreak needed.
- **Collapse** (D42): the chevron, ←/→ on a focused header (any control in it), and Collapse all / Expand all in the
  view menu. State persists per source key through the list lens (`useListLens`'s `collapsePreference`,
  `preferences.collapsedSourcesPreference`, storage key `skills-collapsedSources`). While a query or a chip narrows
  the list every group is expanded for its duration; a collapse made meanwhile is the narrowing's own, and clearing it
  restores the stored state. A collapsed group's rows stay in the lens and keep their selection — counted by the bulk
  rail and the header checkbox — but are not on screen (`useListLens`'s `onScreen`): `j` / `k`, ⌘A, a shift-click
  range and the post-mutation focus rule skip them.
- **Grouping** is decided on the **unfiltered catalog** (D42, BACKLOG #81 (4)): a source holds one skill in the whole
  library or it keeps its header — a query that leaves one skill of a multi-skill repo shows it under that repo, never
  under "Individual sources" (`groupSkillsByRepo`'s `catalog`). Sources with one skill fold into "Individual sources";
  no source pill repeats inside a group.
- **Card view** (Auto / Dense grid) renders the same anatomy on a `card` tile with the same strip — a stop-gap
  until round 19b designs the grid (F07's clipping is gone: marks never clip).

### The Deployments strip

One 12 px mark per harness of the effective target set (registry order), then any harness the skill still has a
deployment row for outside that set, on a **20 px pitch** (8 px gaps; ticket 31 — at 16 px the header's two-letter
initials ran together, "GMAM"), so the strip column is marks × 20 + 40 px. It is **read-only** (D22): each mark is `role="img"` with its own name, and the row opens the
skill. Shape carries the state; colour only reinforces it; the strip was checked in grayscale
(`.scratch/round19/evidence/11/*-grayscale.png`).

| Mark | Shape | Colour | Accessible name | Dark (ticket 35 tokens) / light on background · card · raised |
|---|---|---|---|---|
| deployed (`synced`) | filled dot | `muted-foreground`; `input` when every mark is deployed (the quiet N/N strip) | "Cursor: deployed" | 8.33 · 8.13 · 6.61 / 8.36 · 7.41 · 6.75; quiet 3.94 · 3.85 · 3.13 / 4.38 · 3.88 · 3.54 |
| not deployed | hollow ring | `input` | "Amp: not deployed" ("…not deployed yet" for a `pending` row) | 3.94 · 3.85 · 3.13 / 4.38 · 3.88 · 3.54 |
| out of date copy (`stale`) | half-filled ring | `stale` | "Codex: out of date copy — the copy differs from the skill" | 11.01 · 10.75 · 8.74 / 5.68 · 5.03 · 4.58 (token table; light on `raised` is a mark, not text) |
| failed / copy missing | × | `failed` | "Amp: failed — the last deploy did not complete" ("…first deploy…"; "Amp: copy missing — the deployed copy is gone") | 7.57 · 7.39 · 6.01 / 5.64 · 4.99 · 4.55 |
| central copy missing (`broken`) | × | `failed` | "Pi: central copy missing — the link has nothing to follow" | as failed |
| disabled | slashed ring | `faint` | "Codex: disabled" / "…disabled — the skill is off everywhere" | 6.00 · 5.86 · 4.77 / 6.24 · 5.53 · 5.04 |

- A harness detection no longer sees is named "Windsurf (not detected): …"; a label is always the Harness catalog's (`useToolCatalog`).
- A skill whose **central copy is missing never shows a filled mark**: a deployed or out-of-date row is `broken` (its
  link dangles). A skill disabled at skill grain shows every mark disabled and its switch off.
- N/M: N counts deployed and out-of-date marks (a deployed copy exists), M every mark; `faint` when complete.
- The strip is near-invisible at N/N by *colour step*, not below 3:1: the quiet marks drop from
  `muted-foreground` to `input`, the lowest non-text token that still identifies a mark on its own.

### Empty and first-run

- **Empty library** (`?scenario=empty`): the shell stays; the header keeps only Configure harnesses; the body is one
  heading, one sentence (naming example harnesses once), and the next step — **Add from GitHub** (`solid accent`),
  Add from a folder (`outline`), Browse Explore (`ghost`). No filter bar, no Update all.
- **Discovered skills** (`?scenario=first-run`): a quiet `card` row at the top of the list (inbox icon, neutral
  words, no amber) — "Skills already in your harnesses · 12 skills sit in your harnesses' folders…" — with **Review N
  discovered skills**. While the discovery outweighs the library (N ≥ library size) Review is the one
  `solid accent` action and Add skill steps back to `outline`; otherwise Review is `outline accent` and the list's
  real issues lead. In an empty library with discoveries, Review leads the empty state's buttons.
- **The rare delight** (D6; implemented with Import's migration, ticket 21): when an import of ≥ 10 skills
  completes, the imported rows' strips light mark by mark, left to right, 20 ms apart (opacity 0 → 1, each
  ≤ 120 ms, `ease-out`), once, and the header summary reads "12 skills imported · all deployed" for one run.
  Nothing else moves; reduced motion shows the settled strip at once. The per-mark light exists since ticket 15
  (§ Motion, `useSyncLight`); the cascade adds only the stagger (a `transition-delay` per mark index).

## The inspector (ticket 12)

The focused skill — surface state's `focused`, a selection not a route (D3). Files:
`components/skills/inspector/{SkillInspector,InspectorSection,HealthSection,TargetsSection,SourceSection,EditsSection,ProjectsSection,FilesSection}.tsx`,
`components/skills/{SkillFiles,StripMarkIcon}.tsx`, `modals/{RepointModal,DetachModal,AssignProjectModal}.tsx` (the Explore
preview takes the same slot — § Explore).
Composed from primitives and tokens; the one new variant is Dialog `placement="sheet"`.

- **Placement.** ≥ 1200 px: an `<aside>` (landmark, `aria-labelledby` its `h2`) on `card`, beside the list below the
  header and filter bar — `max(22.5rem, ⅓)` of My Skills, `border` left edge. < 1200 px: the same content in a Dialog
  `sheet`. Header: name (`lg` semibold, the `data-inspector-heading` focus target) · description (`sm`
  `muted-foreground`, 3 lines) · Close (`ghost icon`, 32 px).
- **The focused row** shows it: `aria-current="true"`, `raised` ground and a 1 px inset `ring` edge (the ≥ 3:1 cue:
  6.14 dark / 4.72 light on `raised`); a selected (checked) row has the ground and a 3 px `ring` edge on its leading
  side (ticket 13) — the two coexist.
- **Sections** (`h3` `sm` semibold, a `border` hairline between): Health · Deployments · Source · Edits · Projects ·
  Files, then Remove (`outline danger sm`) in a footer. Section controls are `sm` (24 px) Buttons.
- **Health** reads `skillHealth` alone: per row an icon (⊗ `failed`, ⚠ `stale`, ⊖ `faint`), the label (`sm` medium),
  what happened (`xs` `muted-foreground`: the typed error in words, the harnesses, when) and its repairs (`outline sm`; the
  first repair of the first row `accent`, removal `danger`). Healthy is one quiet line. The section is Restore's
  focus target (`data-inspector-health`); spanning the scroll area edge to edge, it draws its ring inset
  (`insetFocusRing`) so all four sides paint.
- **Deployments** (D22): a list by default — the strip's glyph (`StripMarkIcon`, shared with the row), the harness,
  `link`/`copy` in `faint`, the state in words (`failed` text for failed / copy missing / central copy missing, else
  `muted-foreground`), inline actions: Retry (`outline`), Disable / Enable / Deploy (`ghost`). A row's first action is
  the same element as it changes role, so focus stays on it. Disable shows "Removed from {harness}" (`role="status"`)
  with **Undo** (`ghost accent`) for 5 s — in the list on the harness's row, under the grid as one line (the tile keeps
  focus). The grid (Segmented List / Grid, persisted `skills-inspector-targetsView`)
  is `outline md` tiles, glyph + harness; each names its state and what a press does. A skill off everywhere or with
  its central copy gone offers no per-harness action — Health carries the repair.
- **Source**: icon · the source line (`sm` underlined link for a repo; a recorded folder in `font-mono xs`) · Copy
  (`ghost icon-sm`) · provenance (`xs`; a missing folder in `stale`) · "Last updated" (`faint`) · Update, Change
  source…, Detach.
- **Edits**: the invocation mode as a term/definition pair; a conflict in `stale` text and the Edit button toned
  `stale` ("Resolve…"). The Edit dialog then leads with the plain choice: **Keep yours ({mode})** (`solid accent`) ·
  **Use the skill's ({mode})** (`outline`).
- **Projects**: each project with its harnesses' statuses (`failed` text for failed / copy missing) and Remove (`ghost sm`);
  **Deploy to project…** opens a picker listing every project with its configured harnesses (radio rows; a project with
  no harnesses, or already on all of them, cannot be chosen).
- **Files**: tree (28 px items, `raised` when open) above a reader (max 28 rem, a scroll region that is one Tab stop;
  Markdown under `.prose`, 72 ch). **Expand** covers My Skills in place (the list stays mounted and `inert`
  underneath, its scroll and focused row kept); tree 256 px beside the reader. Escape collapses first — in the sheet
  too (`interceptEscape`) — and focus returns to Expand; the next Escape closes.
- **Re-point** (D15): pre-filled with the current source; **Check source** resolves it first; the summary
  "Replaces *X*'s bytes with **Y** (`subpath @ ref`)" takes focus; a name mismatch adds a `stale` "Different name"
  Badge and the confirm reads **Continue with Y**; a multi-skill repository lists its skills as radio rows first.
  Refusals render under the field (`FieldError`), never as a toast over the dialog — the preview's under the input,
  the final command's (the door answers it to the dialog) under the summary, with the chosen target kept.
- **Focus** (D21, D23): opening from a row leaves focus on the row, and Tab from the row's last control enters the
  inspector's heading (Shift+Tab walks back; Tab from its last control continues after the row); an arrival without
  a row (deep link, history) focuses the heading; the sheet opens on its heading; a resize that reparents a focused
  inspector focuses the new heading — in the same task as the reparent (`flushSync`), so no frame finds focus on
  `<body>` between the old placement and the new (ticket 15, 14 audit N2); closing returns to the row; Restore lands
  on Health. A resize never reorders
  layers: while a dialog is open above the inspector it keeps its placement, and the new
  one lands when the last layer closes. The Projects section and the picker read the project world's held views,
  which every project deployment's returned view updates — no read follows a successful Deploy to project.

## Selection, the bulk rail and the Delete dialog (ticket 13)

Files: `components/skills/{BulkRail,useSkillListKeymap}.ts(x)`, `modals/{CountConfirmModal,DeleteModal}.tsx`, the
selection in `SkillRow` / `SkillsList`, the counts in `lib/bulkCounts.ts`. Composed from primitives and tokens; the
new primitive options are ConfirmDialog `initialFocus` and PopoverContent `anchor` (above).

- **Selection** is My Skills' surface selection (`useSurfaceState`, D4). A selected row: checked checkbox (`primary`
  fill) + `raised` ground + a 3 px `ring` edge on its leading side — iris's selection use. Measured on `raised`:
  edge 6.14 dark / 4.72 light; checked box 3.56 / 4.72. A group header's checkbox selects its source (`mixed` when
  part of it is selected).
- **Keys** (one keymap, `useSkillListKeymap`; listed in the sidebar's Keyboard shortcuts): `/` search · `j` / `k`
  next / previous row · Enter open · `x` select the row holding focus · ⇧-click a range from the last row toggled ·
  ⌘A every row the lens shows (the visible subset) · Esc clears the selection, then collapses Files, then closes the
  inspector. None fires while typing or while a dialog, sheet, popover or menu is open. The row keys (`/ j k x ⌘A`)
  fire only on the list surface (header, filters, rail, rows — `inListScope`), never from the inspector or its
  SKILL.md reader, where they stay native keys (⌘A selects text); while expanded Files cover the list only Escape
  runs. Escape's hierarchy runs from the reader too. Clearing from inside the rail hands focus to the row last
  toggled (the rail goes with the selection).
- **Bulk rail** (`card`, `border-strong` top edge, at the foot of the list; first in the DOM, so `/` then Tab reaches
  it): "N selected" (`sm` semibold, a `role="status"`) · the `x` / `Esc` hints (kbd, hidden below the shell
  breakpoint) · `outline sm` Buttons Deploy to harnesses… · Deploy to project… · Disable on all harnesses · Enable · Update ·
  Remove… (`danger`) · Clear (`ghost icon-sm`). A verb that would change nothing is disabled with the reason as its
  title; every rail Button keeps focus while disabled (`focusableWhenDisabled`), so D21's "same control" holds after
  it ran. Text on the rail: ≥ 5.86 dark (`faint` on `card`, ticket 35) / 4.85 light.
- **Count-bearing confirmation** (`confirmCount`): ConfirmDialog, focus on Cancel, one sentence of counts from
  `bulkCounts` ("Disable 25 skills on 8 harnesses — 80 global deployments are removed and kept disabled; 12 project
  deployments for these skills are disabled too. Updates will not re-add them."), a `muted-foreground` second line for
  what changes nothing. Deploy to harnesses carries its harness pick (a two-column checkbox FieldSet) and the counts
  follow the pick.
- **Row switch**: Disable is never asked; Enable that will deploy the skill to harnesses again asks in a quiet anchored
  Popover ("Enable retired-playbook and deploy it to 2 harnesses again?" · Not now `ghost` · Enable `outline accent`).
- **Delete dialog** (D16), one Dialog with two states. Confirm: the harnesses it is removed from, the projects that carry
  it, the Edit lost, "Skills Hub's copy is deleted" (`muted-foreground`), and for a skill with no external source
  "Skills Hub's copy is the only copy — this cannot be undone" (`failed` text with ⚠); several skills read as counts
  plus their names (six, then "and N more"). Footer: Cancel (initial focus) · Remove (`solid danger`, `pending` while
  it runs — it keeps focus, so focus never reaches `<body>`). Report: the heading takes focus ("release-train was not
  deleted" / "1 of 3 skills was not deleted"); per kept skill "Removed from Codex, Pi." then per kept path a `card`
  block — "Still in Claude Code:", the path (`font-mono xs`), the reason (`failed xs`, `describeCommandError`),
  **Reveal in Finder** (`outline sm`; selects the recorded path — or its nearest existing ancestor — in the file
  manager, never opens it; a path the app does not record is refused); footer Close · Retry (`solid danger`, the kept skills only). The heading is
  `tabIndex=-1` (focusable, never a Tab stop).

## Header progress and the report by run (ticket 14)

- **Header progress** (`shell/HeaderProgress.tsx`, D19) reads the reporter's `pending` alone: the action's own words
  (`activity`: "Updating all", "Adding", "Removing beta"; else "{label} running") · `done/total` (`tabular-nums`,
  `muted-foreground`) · a 64 × 4 px determinate bar (`raised` track, `primary` fill, `scaleX` over
  `--motion-fast`, `motion-reduce:transition-none`) · the step line (`xs muted`, truncated) · **Cancel** (`ghost sm`,
  named "Cancel {label}") only while `cancellable` — withdrawn once the action enters a phase its command no longer
  cancels (an Update all's apply phase) — reading "Cancelling…" until the command returns. A 14 px
  `LoaderCircle` spins `motion-safe` and is hidden `motion-reduce`. It sits in My Skills' header slot, as a
  `border-b` strip above any other area, and at a dialog's foot (`border-t`, `card`) while that dialog's action runs
  — the page behind a modal is inert. Never a layer, never focus-taking; its live region announces the activity,
  not each tick. A bulk Remove stays the Delete dialog's (one surface). **One visible progress** (ticket 15, 14
  audit N5): while a dialog whose foot shows the progress is open (Add — in its sticky summary, Import, Re-point, Notifications),
  the header's (or the bar's) copy under the scrim yields — it keeps its live region and draws nothing. In My Skills' header it takes the exception
  summary's place while it runs (the summary is out of date until the run settles), so the row never wraps at 960 px.
- **Focus when an action settles** (D21, `actionLifecycle.returnLostFocus`): focus that was lost — `<body>`, the
  main landmark, a layer's popup itself, or the vanished Cancel — returns to the control that started the action;
  the skill door's `focusAfter` (Restore → Health, Delete → next row) then applies its own rule. Verbs keep focus
  while disabled (`focusableWhenDisabled`), so a pending action never drops it.
- **Refusal** (D19): verbs stay focusable while anything runs (`focusableWhenDisabled`), with the reason as `title`
  and `aria-description` ("Update all is running — wait for it to finish"). Activating one shows that reason as the
  refusal toast over "Nothing was changed." (§ Held-control refusal, ticket 27). A row whose skill is in the running
  action shows its overflow button `pending` (spinner, `aria-busy`, keeps focus); the row `li` is `aria-busy`.
- **Notifications panel** (`NotificationsModal.tsx`, `lg` dialog, `flush` body): one row per run — kind glyph
  (state token), the headline (`sm semibold`), "{action} · {relative time} · N items" (`xs muted`), a disclosure
  (`aria-expanded`) when it has items. Items (`card` ground, indented) carry a kind glyph (**To review** is
  `CircleDot` in `stale`), words (a project row prefixed "in {project}:"), and actions: **Show** (`outline sm`),
  Retry / Overwrite… / Change source… / Remove… (`ghost accent sm`, disabled with the reason while an action runs),
  Open Settings → GitHub. Every Copy is named by its row ("Copy: alpha failed"). The run "View report" asked for is
  expanded and focused; otherwise the newest. What arrives while the panel stands open — a run started from it, or
  a note such as an outdated collision's "Nothing to overwrite…" — is announced once through the panel's polite
  `role="status"` (its title), since no toast shows meanwhile; focus stays put (ticket 15, 14 audit N4).
- **Update affordance** (D8): a sidebar-foot item above the bell, `ArrowUpCircle` in `ring`, "1.3.1 available" —
  opens the update dialog; nothing is raised on startup.

## Add (ticket 16)

- **One stepped Dialog** (`skills/add/AddSkillDialog.tsx`, `lg`, one `addSkill` kind on the surface stack; the step is
  the flow hook's): **Source** → **Skills** → **Result**, named in the header as an `ol` with `aria-current="step"`.
  Two layers can sit above it: the one overwrite ask (D12) and — over the Edit harnesses popover — the shared-folder
  confirmation that toggling a harness sharing a skills folder (Amp, Kimi) raises through `useSharedDirConfirmation`;
  that ask returns focus to its checkbox with the popover still open (§ Popover). Dismissible except while Add's own action runs; the shell
  stays mounted so Close returns focus to the control that opened it.
- **Sticky summary** (`card` ground, `border-b`, under the header): Source (`owner/repo` or the folder) · Skills
  ("3 of 4 checked · 1 hidden by filter", may wrap) · Deploys to (the effective target set, first three named,
  "+N" never clipped) with **Edit harnesses** (`ghost accent sm`) — a Popover (a trigger, so not `anchor`) over the
  catalog's installed harnesses plus any selected one detection no longer sees ("not detected"), as visible Checkbox
  rows in two columns, under one sentence naming example harnesses once, and a count line ("4 skills to 7 harnesses"). Add's progress (`HeaderProgress placement="dialog"`)
  sits at the summary's foot, with its Cancel.
- **Source**: real Tabs (`segmented`, `aria-selected`) — Git repository / Local folder — each panel one Field with its
  help and its refusal (`FieldError`, never a toast); **Continue** (`solid accent`) lists the source as a read
  action that Cancel discards. While the listing runs the kind is frozen: the other tab is disabled (focusable,
  announced unavailable) and the intent refuses, and a listing lands only on the source it was started for.
- **The candidate picker** (`skills/add/CandidatePicker.tsx`, one component, a `mode` prop): a search Input (once
  there is more than one candidate), "Select all [shown]" over the visible selectable ones, the status line, and
  bordered rows — each control named by the candidate's name and described by its description, path and, when it
  cannot be picked, "Can't be added: {reason}" (`stale`; the Checkbox disabled, out of the Tab order). `multiple`
  (Add): only checked **and** visible candidates install. `single` (Re-point's multi-skill repository): native radios
  in a `radiogroup` fieldset; only a choice still shown is sent. A lone candidate offers "Name (optional)".
- **Result** rows (bordered list, one per sent candidate): a 16 px state glyph — Queued (`CircleDashed`), Adding
  (spinning `LoaderCircle`, `motion-safe`), Waiting for the result (`Clock`), Added (`CircleCheck`, `ok`; `stale`
  when its first deploy left something to look at), Failed (`CircleX`, `failed`), Cancelled (`Ban`, muted) — the name,
  the state in words, and one detail line (what the first deploy did, the typed reason). Failed rows carry **Retry**,
  cancelled rows **Add** (`outline accent sm`, named with the skill); "Retry failed (N)" when several. An installed
  row names only the harnesses its first deploy missed ("not deployed to Amp"); each distinct reason is said **once**
  under the rows (`xs`: the harnesses in `foreground` medium, the reason, then "Affects 4 skills: …" muted) — never repeated per
  row, never clipped. A candidate's own failure keeps its reason on its row beside Retry. The dialog stays on its
  Result after a failure, a cancel or a folder left to review, and closes only when **every** row it holds is complete
  (a kept folder counts) — a clean Retry of one row does not close over another's folder to review — or on Close.
- **Focus**: each step's first control takes focus in the commit that changes the step (the field, the search — else
  the first selectable candidate, else Back when nothing can be picked — the Result heading); a focused control a render removes (the summary's Cancel, withdrawn for the ask's retry) hands
  focus to the same target — never `<body>`, never the dialog's first tabbable by fallback.

## Projects (ticket 17)

Files: `components/projects/{ProjectsPage,ProjectList,ProjectHeader,AssignmentMatrix,AddProjectModal,EditProjectModal,RemoveProjectModal}.tsx`,
the cell fold in `lib/projectPresentation.ts`. Composed from primitives and tokens; the one new primitive option is
Checkbox `focusableWhenDisabled`.

- **Selection** (≥ 1200 px): the project list (`card`, 256 px) is one **named selection button** per project —
  name (`sm` medium), path (`font-mono xs faint`, wrapping only after a `/`), then "3 harnesses · 5 skills" (`xs muted`;
  a `failed` / `stale` word when the project has one) or "Folder not found" (`stale` with ⚠). The selected project has
  `aria-current="true"`, a `raised` ground and a 3 px `ring` edge (6.14 / 4.72 on `raised`); the focused one draws the
  inset focus ring — the two never share a cue. ↑ / ↓ / Home / End move focus between projects (Enter / Space select;
  Tab keeps the native order). Each project's overflow (`ghost icon-sm`, "More actions for {project}") holds Update
  path… (a missing folder), Project settings…, Remove project… (`destructive`) — separate from selection.
- **Header** (D23): name (`h2`, `base` semibold, the `data-project-heading` focus target) · path (`font-mono xs`) ·
  "Last deployed …" (`xs faint`, `whitespace-nowrap` — one line) · ⚠ "Folder not found — Update path or Remove the
  project." Name and path wrap, never truncate; the actions take only the room beside them. ≥ 1200: Configure
  harnesses… (`ghost`) · Re-deploy all (`outline`) · **Re-deploy project** (`solid accent`, the one primary) · overflow
  (the project's own actions). < 1200: the list becomes the header's `Select` (after the "Projects" `h1`, with Add
  project), and the toolbar collapses to Re-deploy project + the overflow (Configure harnesses…, Re-deploy all, then
  the project's actions). Every
  mutation control stays focusable while refused, with the reason as `title` + `aria-description`.
- **"Statuses may be out of date — Reload view"** (`xs muted`, ⓘ, a `role="status"` line under the header) when the
  view's reconcile pass was skipped (`reconciled: false`); Reload view is a read, and its focus moves to the
  project heading once the line goes.
- **Filter bar** (the matrix's one list lens): search (`Input sm`) · Chips **All · Deployed · Issues** with counts
  (Deployed: any deployment here; Issues: out of date copy, copy missing or failed — what Deploy this repairs) · Segmented Grouped /
  Flat. Sorted by name; no sort control (My Skills keeps the menu).
- **Legend** (`xs muted`, one line, `aria-label` "What the marks mean"): each state's mark + word, its meaning as the
  title, then ⊕ "also global" — the same vocabulary the cells draw.
- **Matrix**: column headers are the Harness catalog's **labels** (`xs` medium), "not detected" (`xs faint`) under a
  configured harness detection no longer sees. A **cell** is a `<label>` spanning it (the checkbox's hit area: the
  36 px row × the column) with the Checkbox, the state's mark (`StripMarkIcon`), ⊕ when the skill is **also global**
  on that harness (D17: the cell stays enabled; toggle and bulk obey the same rule), and the state's word, shown on hover
  and focus (`visibility`, so nothing moves). The checkbox's name carries all of it: "docx in Claude Code: copy
  missing — the deployed copy is gone — Deploy this to fix, also global". **Deploy this** (`ghost icon-sm`, "Deploy
  {skill} in {harness}") follows an out-of-date, copy-missing or failed cell; it pends while it runs and, once the cell is healthy, hands its focus to the
  cell's checkbox. A cell in flight shows a spinner and holds focus (`focusableWhenDisabled`). With the folder
  missing, cells and row verbs are disabled with "Folder not found — update the path to change deployments".
- **Row verbs** (the "All harnesses" column, `ghost sm`): **Deploy to all** (`accent`) — named "Deploy to all
  configured harnesses — {skill}" — and **Remove from all**, both the door's verbs (`assignToProject` /
  `unassignFromProject`, the inspector's). Both stay mounted: a verb that would change nothing is refused with the
  reason ("Deployed to every configured harness already"), so focus never drops.

| Cell state | Mark (shape) | Word | Meaning (in the name and title) | Mark dark (ticket 35 tokens) / light on background |
|---|---|---|---|---|
| `synced` | filled dot, `muted-foreground` | deployed | the copy matches the skill | 8.33 / 8.16 |
| `stale` | half-filled ring, `stale` | out of date copy | the copy differs from the skill — Deploy this to fix | 11.01 / 5.62 |
| `missing` | ×, `failed` | copy missing | the deployed copy is gone — Deploy this to fix | 7.57 / 5.50 |
| `pending` | hollow ring, `input` | not deployed yet | recorded here, not deployed yet | 3.94 / 3.78 |
| `failed` | ×, `failed` | failed | {the error} — Deploy this to retry | 7.57 / 5.50 |
| `disabled` | slashed ring, `faint` | disabled | not deployed here (disabled) / the skill is off everywhere | 6.00 / 6.09 |
| also global | ⊕ (lucide Globe), `muted-foreground` | also global | the skill is also deployed globally for this harness | 8.33 / 8.16 |

Copy missing and failed share the strip's ×; the word (on hover / focus, in the name and the legend) tells them apart.
Words are `muted-foreground` (8.33 / 8.16). On a hovered cell (`raised`) the marks keep ≥ 3:1 (token table).
Measured 2026-10-01 on the fixture (`failures`), `.scratch/round19/evidence/17/a11y-*.json`; grayscale capture
`matrix-grayscale-*.png`.

- **Remove project** (`sm`): "Remove {project}?" · what goes (the skills Skills Hub deployed into the project's harness
  folders; the project and its deployments) · the path · what stays (`foreground`: the project's own files, the
  skills in the Library) · how a kept copy shows (`muted`: the project stays listed and that cell reads failed with
  its reason — ADR-0002). Cancel takes focus; **Remove project** (`solid danger`) pends while it runs. It closes
  either way; a removed project's focus goes to the area's heading.
- **Add project / Project settings**: Field + Input + **Browse…** in one row (Add's local-folder pattern; a folder
  already registered is refused under the field), a FieldSet "Keep deployed skills out of git" with two Checkboxes.
  All four project dialogs (Add, Project settings, Configure harnesses, Remove) show the header progress at their foot
  while their action runs (`placement="dialog"`; the page copy yields).

## The ⌘K palette (ticket 23)

Files: `components/shell/CommandPalette.tsx` (mounted once by the binder) over the `CommandPalette` primitive; what
it lists is the pure fold `lib/commandPalette.ts` (`paletteGroups`). It is the surface module's `commandPalette`
dialog, so it never reaches the hash (D5).

- **Open / close.** ⌘K on Apple platforms, Ctrl+K elsewhere — never both, since Ctrl+K inside a macOS text field is
  kill-line — opens it over an empty dialog stack, from anywhere, a text field included (a chord, not a typed key), and closes it when it is the top layer; Escape and an outside press close it too. A dialog
  owns the keyboard: ⌘K does nothing while one is open. The inspector sheet is not a surface dialog, so the palette
  opens over it and Escape closes the palette alone.
- **Reach.** Every skill by name (→ the inspector, focus on its heading; the sheet below 1200 px), every area
  (Projects returns to the project the world holds, as the sidebar does), Settings and each pane it addresses today
  (GitHub token) through **one** route, `openSettingsPane` (ticket 20 retargets it to the Settings dialog), every
  door verb on the **showing surface's selection** with its count ("Disable 3 selected on all harnesses…" — the
  bulk rail's six verbs), Update all, Add skill…, Configure harnesses…, and the library-wide undeploy "Disable every
  skill on all harnesses…" (D8's select-all → Disable; "remove" finds it — Remove itself is Delete in this app).
- **Order.** Recently used (this session, at most 5, most recent first) · *N selected* · Go to · Actions ·
  Skills. Nothing is highlighted on open — recents come first, so ⌘K → Enter with no query is a no-op and can never
  repeat the last Update all; ↓ highlights the first option, typing highlights the first match. Typing filters every group through the list lens's query rule (`queryMatcher`;
  skills through `filterAndSortSkills` itself), so `*` wildcards work as in My Skills' search.
- **A label ends in … exactly when a dialog follows**: Deploy, Deploy to a project and Remove always; Disable,
  Enable and Update over several skills (one skill acts at once, as from its row).
- **Choosing closes first, then runs.** The palette hands focus back (to the element that opened it, or D21's chain
  when that is gone) and only then runs the choice: a verb goes through `actions.run` without a confirming option, so
  the door opens the same count confirmation, harness pick or Delete dialog as the bulk rail, with the returned element
  as its opener — cancelling it returns focus there. A place moves focus to its heading (the area's `h1`, or the
  main landmark) unless the place took focus itself (Settings → GitHub token focuses the token field).
- **Refused** (D19): while something runs every verb and action stays listed, `aria-disabled` with the lifecycle's
  reason; places and skills never are. A verb that would change nothing says so (the rail's `bulkCounts` reasons).
- **Not the matrix's keymap.** The palette's keys (↑ / ↓ / Enter / Esc) live inside its own popup; it defines
  no grid model, so the Projects matrix keeps its Tab walk (ticket 17 N1 — its spatial keymap is separate work).

## Explore (ticket 22)

Files: `components/skills/{ExplorePage,ExplorePreviewView}.tsx`, the folds in `lib/explorePresentation.ts`. Composed
from primitives and tokens; no variant was added. Tracker rows, not cards (D25, D38's ground).

- **Header** (48 px + hairline): "Explore" (`h1`, the `data-surface-heading="explore"` focus target) · **Add skill**
  (`outline`). Install is the area's action, not the header's.
- **Filter bar** (the area's one list lens, ranked — no sort control): search (`Input sm`, labelled "Search Explore"
  through a Field; ↓ enters the rows, Escape clears it) · Chips **Not in library** (the default) · **Installed** ·
  **Hidden** with counts (`faint`); they partition the candidates — a hidden one is only under Hidden. Under them the
  **search mode line** (`xs muted`): "Featured skills from curated GitHub repositories" (no query), "Filtering featured
  skills" (one character), each with the inline hint "· Type 2 or more characters to search skills.sh too" (`faint`);
  from two characters "Featured skills and skills.sh results for “q”".
- **Source cards** (D38's ground): "Featured" and — while a query searches online — "From skills.sh", each a bordered
  `rounded-lg` card, header (36 px, `card`: `h2` `sm` semibold · count `faint`) over rows on the base `background`.
  A card with nothing under the chip says so in one line (`sm muted`); "Searching skills.sh…" and "Loading featured
  skills…" are `role="status"`.
- **Row** (36 px, `border` hairline, `accent` on hover (ticket 35), instant): name (`sm` medium, the Tab stop; Enter or a click on the
  row opens it; ↑ / ↓ walk the rows, ↑ from the first returns to the search) · source (`font-mono xs faint`) ·
  description (`sm muted`; gone in a pane under 48 rem) · popularity (`xs faint tabular-nums`, ↓ downloads / ☆ stars /
  installs, compact "12.3K", the full figure as its name and title — secondary, never a sort) · **Installed** (`soft
  neutral` Badge with ✓) · overflow (`ghost icon-sm`: Preview / Open in My Skills, **Hide from Explore**). A Hidden
  chip's row carries **Unhide** (`ghost sm`, "Unhide {name}") instead. Hide and Unhide move focus to the next row on
  screen, else the previous, else the search. The row whose preview is open is `aria-current`, `raised` with the 1 px
  inset `ring` edge — the inspector's focused-row cue.
- **An installed candidate is its Managed skill**: its row (and its preview, when it was added from there) opens the
  managed inspector in My Skills (`focus({ kind: "skill" })`) — never a second Install.
- **Preview** in the inspector's slot (`ExplorePreviewView`): an `<aside>` on `card` beside the rows at ≥ 1200 px
  (`max(22.5rem, ⅓)`), a Dialog `sheet` below it. Header: name (`h2` `lg`, `tabIndex=-1`, the arrival focus — from
  its row, or when focus was nowhere; the sheet opens on it) · summary · Close (`ghost icon`). Then **Install**
  (`solid accent`, the area's one primary) with its scope **beside it, always visible**: "Deploys to Cursor, Claude
  Code, Codex +4" — Add's sticky-summary scope, same words ("Not deployed — auto-deploy is off" / "No harnesses — added
  only"); Install opens Add on the repository with the skill checked (ticket 16) and never installs by itself. An
  installed candidate shows **Open in My Skills** (`solid accent`) + the Installed badge. Source (`xs`, the repo as an
  underlined `muted` link) · "Fetched for preview {time}" (`faint`, ⏱) — the clone's acquisition time, never freshness.
  Files (`SkillFiles stacked`). Escape (beside the rows, once a menu or the search has spent theirs) and Close return
  focus to the row — when the row has left the list meanwhile (an Add from the preview takes it out of Not in
  library), to the next row still shown in the order the preview was opened from, else the previous, else the search;
  one resolver (`exploreFocusReturn`) for the aside and the sheet's `finalFocus`, whose Dialog stays mounted. The Add
  dialog opened from Install returns to Install. Installed is the whole library's answer (`exploreInstalledId`, the
  rows' identity rule), never whether the search still lists the candidate.
- **Contrast** (measured 2026-11-02, fixture `rich`, both themes, `.scratch/round19/evidence/22/*-a11y.json`), dark /
  light: Install text on `primary` 4.68 / 5.86 (hover `primary/90` 5.31 / 4.79, token table); Installed badge text
  7.08 / 7.20; name on `background` 15.50 / 17.32; source and popularity (`faint`) 6.12 / 6.09 on `background`, 5.22 /
  5.37 on the focused row's `raised`; description and mode line 8.31 / 8.16; the focused row's `ring` edge on `raised`
  6.01 / 4.72; search edge (`input`) 3.90 / 3.78; scope label / value on the aside's `card` 7.78 / 7.60 and 14.51 /
  16.14 (in the sheet, on `popover`, light 8.36 / 17.76). axe: no violations at 1280 × 800 and 960 × 640, either theme.
  The dark figures are superseded by ticket 35's token tables (2026-11-04): Install 4.68 (hover 5.32); Installed 7.65;
  name 19.26; `faint` 6.00 on `background`, 4.77 on `raised`; description 8.33; `ring` edge on `raised` 6.14; search
  edge 3.94; scope label / value on `card` 8.13 / 18.81 (on `popover` in the sheet 7.65 / 17.71).

## Migration rule

- A surface is **migrated** when its component files reference no class that `App.css` defines. It is then appended
  to `migrated` in `scripts/styles-baseline.json`, never removed.
- The same change lowers `appCssCeiling` by the lines it deleted. `npm run styles:check` (part of `npm run check` and
  CI) fails when:
  - `App.css` grows, or shrinks without the ceiling being lowered;
  - a migrated file, or any file in `components/ui/`, uses a legacy class — read from the TypeScript AST, following
    constants, objects, conditionals, templates and helpers in the file and through named relative / `@/` imports;
  - a new stylesheet appears;
  - `prose.css` leaks outside `.prose` (nesting resolved; `.prose + x` / `.prose ~ x` leak) or holds a colour literal,
    `var()` fallbacks included;
  - a raw colour or palette utility enters `src/**/*.{ts,tsx}` — arbitrary values (`bg-[#…]`, `bg-[red]`,
    `[color:red]`) and colour literals in inline `style` objects included.
- The guard runs its own regression suite first (`scripts/check-styles.test.mjs`): each rule's violations must exit 1
  against a temp copy, the clean copy exit 0.
- `className` on a primitive is layout-only **by convention**; the guard does not enforce it (review S1 note). Review
  call sites for visual overrides.
- **The alias block** at the top of `App.css` maps every legacy custom property (`--bg-app`, `--text-secondary`,
  `--accent-*`, `--status-*`, …) onto these tokens, so un-migrated surfaces already render in the new palette. It
  was the last addition `App.css` received. It is deleted together with the last legacy rule.
- Never add a rule to `App.css`.

## How to add a variant

1. Add the value to the primitive's `cva` map: a `variants` entry plus `compoundVariants` for each
   `variant × tone` it combines with. Use token utilities only. For a soft ground, use `softGround` in `ui/styles.ts`.
2. If it needs a colour that no token expresses, add the token to `theme.css` with both a dark and a light value,
   and register it in `@theme inline`.
3. Measure the new pairs on `/?specimen` in both themes: add the variant to `specimen.tsx`. Record the ratios in this
   file's tables.
4. Document the value in the primitive's table above in the same change, and extend its `*.test.tsx` if it changes
   ARIA state.
