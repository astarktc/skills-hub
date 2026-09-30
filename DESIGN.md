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

"On grounds" lists the ratios against **background / card / popover / raised**, in that order.

### Surfaces

| Token | Dark | Light | Role |
|---|---|---|---|
| `background` | `#0e0f12` | `#fcfcfd` | App ground. |
| `card` | `#15171b` | `#f3f4f6` | Panel: sidebar, list panes, dialog footer, code blocks. |
| `popover` | `#15171b` | `#ffffff` | Floating layers: dialog, popover, menu, toast. |
| `raised` (= `muted`, `accent`) | `#1d2025` | `#eceef1` | Third level: hover/selected rows, solid-neutral controls, inline code. |
| `sidebar*` | = card / muted-foreground / primary / raised / border / ring | same mapping | App sidebar (shadcn names). |

### Text

| Token | Dark | Light | Role | Dark, on grounds | Light, on grounds |
|---|---|---|---|---|---|
| `foreground` | `#e6e7ea` | `#16181d` | Primary text. | 15.50 / 14.51 / 14.51 / 13.21 | 17.32 / 16.14 / 17.76 / 15.28 |
| `muted-foreground` | `#a6abb4` | `#494e57` | Secondary text; the "on" switch. | 8.31 / 7.78 / 7.78 / 7.08 | 8.16 / 7.60 / 8.36 / 7.20 |
| `faint` | `#8c929c` ¹ | `#5b616a` ¹ | Third level: timestamps, helper text, placeholders, sort labels. | 6.12 / 5.73 / 5.73 / 5.22 | 6.09 / 5.67 / 6.24 / 5.37 |
| `ring` (iris as text) | `#9690ff` | `#5a4fdc` | Selection and focus; accent text (ghost/outline accent). | 7.05 / 6.61 / 6.61 / 6.01 | 5.71 / 5.32 / 5.86 / 5.04 |

¹ **Deviation from the contract.** The contract's dark tertiary is `#858b95`, and the first light derivation was
`#646a74`. Both are lifted one step. The legacy hover ground (`--bg-element-hover`, from the App.css alias block)
measured them at 4.20:1 and 4.08:1 under the tertiary "(Disabled)" label of a hovered Tool pill. On that ground they
now measure 4.60:1 and 4.68:1. The step between `faint` and `muted-foreground` stays visible (luminance ratio 1.36
dark, 1.34 light).

### Accent and state

The state colours say state and nothing else; healthy is silent. Colour never carries meaning alone: a word or a
shape always carries it too.

| Token | Dark | Light | Role | Dark, on grounds | Light, on grounds |
|---|---|---|---|---|---|
| `primary` | `#6a61e8` | `#5a4fdc` | Iris fill: the one primary action, checked checkbox. | 4.10 / 3.83 / 3.83 / 3.49 (non-text) | 5.71 / 5.32 / 5.86 / 5.04 |
| `ok` | `#3ddc84` | `#127440` | Synced / success. | 10.74 / 10.06 / 10.06 / 9.15 | 5.69 / 5.30 / 5.84 / 5.02 |
| `stale` | `#f5b400` | `#8a5d00` | Stale (target drift) / warning. | 10.41 / 9.75 / 9.75 / 8.88 | 5.62 / 5.23 / 5.76 / 4.95 |
| `failed` (= `destructive`) | `#ff6b6b` | `#c22e2e` | Failed / destructive. | 6.91 / 6.47 / 6.47 / 5.89 | 5.50 / 5.12 / 5.64 / 4.85 |

Text on the fills:

| Pair | Dark | Light |
|---|---|---|
| `primary-foreground` on `primary` (`#fff`) | 4.68 | 5.86 |
| … on `primary/90` (hover, worst ground) | 5.31 | 4.79 |
| `ok-foreground` on `ok` | 10.74 (`#0e0f12`) | 5.84 (`#fff`) |
| … on `ok/90` | 8.82 | 4.76 |
| `stale-foreground` on `stale` | 10.41 | 5.76 |
| … on `stale/90` | 8.56 | 4.67 |
| `destructive-foreground` on `destructive` | 6.91 | 5.64 |
| … on `destructive/90` | 5.76 | 4.84 |

Soft grounds are the state colour mixed into `popover`, opaque, so the text on them measures the same on every
surface:

| Soft ground | Dark | Light |
|---|---|---|
| Badge `soft` or hover ground: 10 % state into `popover`, state text on it (accent / ok / stale / failed) | 6.07 / 8.53 / 8.32 / 5.71 | 5.08 / 5.07 / 4.99 / 4.89 |
| Toast: 8 % state into `popover`, state text on it (accent / ok / stale / failed) | 6.16 / 8.89 / 8.62 / 5.88 | 5.22 / 5.22 / 5.13 / 5.04 |

### Lines

| Token | Dark | Light | Role | Dark, on grounds | Light, on grounds |
|---|---|---|---|---|---|
| `border` | `#262a31` | `#e1e3e8` | Structural hairline. **Decorative**: never the only cue. | 1.33 / 1.25 / 1.25 / 1.13 | 1.25 / 1.17 / 1.28 / 1.10 |
| `border-strong` | `#343942` | `#cdd1d8` | Edge of a text-bearing control (outline button, segmented group). Decorative, because the text identifies the control. | 1.65 / 1.55 / 1.55 / 1.41 | 1.49 / 1.39 / 1.53 / 1.32 |
| `input` | `#6b717b` | `#7c828b` | Boundary that identifies a control on its own: text field, checkbox, switch track. | 3.90 / 3.65 / 3.65 / 3.32 | 3.78 / 3.52 / 3.87 / 3.33 |
| `ring` | `#9690ff` | `#5a4fdc` | Focus ring (non-text). | 7.05 / 6.61 / 6.61 / 6.01 | 5.71 / 5.32 / 5.86 / 5.04 |

Other non-text pairs, dark / light:

- Switch thumb (`muted-foreground`) on its `raised` track: 7.08 / 7.20.
- Checked thumb (`background`) on the `muted-foreground` track: 8.31 / 8.16.
- Check glyph on `primary`: 4.68 / 5.86.
- Selected segment/tab iris edge on `raised`: 6.01 / 5.04.

### Prose (read only through `var()` by `styles/prose.css`)

| Token | Dark | Light | Worst ground (dark / light) |
|---|---|---|---|
| `syntax-keyword` | `#f29fd8` | `#a1306e` | 8.34 on raised / 5.71 on raised |
| `syntax-string` | `#7fc8dc` | `#16677d` | 8.71 / 5.52 |
| `syntax-number` | `#e8b48a` | `#87460f` | 8.81 / 6.20 |
| `syntax-function` | `#a9b6ff` | `#3a4cb0` | 8.42 / 6.36 |

Comments and line numbers use `faint` (≥ 5.22). Punctuation uses `muted-foreground`.

### The hardest pairs, and the round-18 regression list

Every pair the round-18 audit measured below threshold now passes. These were measured on the rendered page, in the
fixture app, through the alias block:

| Audit pair (round 18) | Was | Dark now | Light now |
|---|---|---|---|
| Light tertiary text (timestamps, inactive nav, helper) | 2.56 | 6.12 (`.skill-source.time`) | 6.09 |
| Synced Tool pill | 3.67 light | 10.06 | 5.30 |
| Failed Tool pill | 4.41 light | 6.39 | 4.94 |
| "+N more" pill | 2.33 light | 5.22 | 5.37 |
| Explore Install | 2.14 dark | 7.05 | 5.71 |
| Explore Installed | 1.82 dark / 3.58 light | 9.72 | 5.09 |
| Success toast ("Tool configuration saved.") | 4.26 both | 8.89 | 5.22 |
| Search placeholder | 3.36 light | 5.73 | 5.67 |
| Import review "Match" / "Conflict" | 3.07 / 4.41 light | 9.41 / 6.39 | 5.03 / 4.94 |

Legacy identifying input boundaries (`.search-input`, `.input` in Add / Re-point, `.settings-input`,
`.settings-select`, `.explore-search-input`) draw their edge in `input`, not the decorative `border` (ticket-07
review B2: they measured 1.33 / 1.25). Rendered edge against its outside ground, dark / light, 2026-09-29: Library
search 3.90 / 3.78, Add `.input` 3.65 / 3.87, Settings select and input 3.90 / 3.78, Explore search 3.90 / 3.78;
focused (the `accent-primary` iris edge) ≥ 6.61 / ≥ 5.71; disabled keeps the same edge.

The lowest passing pairs anywhere are:

- `faint` on the legacy hover ground `--bg-element-hover`: 4.60 dark, 4.68 light;
- `stale-foreground` on `stale/90`: 4.67 light;
- `primary-foreground` on `primary`: 4.68 dark.

Re-measure them after any change to `faint`, `primary`, `stale` or the grounds.

**Exempt, by rule:**

- Disabled controls.
- `border` / `border-strong` hairlines.
- ~~The legacy decorative `·` separator in the detail header (`.detail-meta-dot`)~~ — gone with the detail (ticket 12).

## Non-colour scale

| Token | Value | Utility |
|---|---|---|
| Type | 12 / 13 / 14 / 16 px (`xs` / `sm` / `base` / `lg`); tabular numerals app-wide (`body`) | `text-xs` … `text-lg` |
| Fonts | System UI stack; `ui-monospace` for identifiers; no network fonts | `font-sans`, `font-mono` |
| Radius | 4 / 6 / 10 px | `rounded-sm` / `rounded-md` / `rounded-lg` (`rounded-full` kept) |
| Row height | 36 px (rows, headers, palette items) | `h-row` |
| Spacing | Tailwind's 4 px base | — |
| Elevation | `shadow-sm`, `shadow-lg` (floating layers only) | — |
| Motion | 120 ms; `ease-out` = `cubic-bezier(0.16, 1, 0.3, 1)` | `duration-(--motion-fast)`, default transition |
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
| | `ghost` | No edge, and a ground only on hover. Toolbars and dense rows. |
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
- `disabled`: not activatable, 50 % opacity (contrast-exempt).
- An icon-only button needs `aria-label`.

### Badge and Chip (`badge.tsx`)

**Badge** is a static label (a count or a state word). Words carry the meaning; colour only reinforces it.

| Prop | Values | Meaning |
|---|---|---|
| `variant` | `soft` (default) | 10 % state ground (opaque, into `popover`) with state text. |
| | `outline` | Tinted edge, transparent ground. |
| | `solid` | State fill with its `-foreground` text. |
| `tone` | `neutral` (default) · `accent` · `ok` · `stale` · `failed` | Semantic role. |
| `size` | `sm` (default) | 20 px. |
| | `md` | 24 px. |

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
- **Switch**: `role="switch"`. `size` is `md` (20 × 36, standalone) or `sm` (16 × 28 with a 24 px hit area, dense
  rows). "On" is **neutral** (`muted-foreground`), not iris: a column of switches must not make the accent mean
  "normal". The thumb slides in 120 ms, and reduced motion removes the slide.

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
  above that it opened (whose opener lay inside it, or is gone): Re-point closed while the progress overlay its own
  action raised is still up hands its chain to the overlay, which returns focus by it when it closes
  (`layer-stack.handOverReturn`).
- **Where the opener can disappear** (a removal: Delete skill, Remove project), the dialog passes `fallbackFocus`.
  Today both removal dialogs pass `mainLandmarkFocus`; the row-level target (the next row, then the list heading) is
  ticket 09's surface lifecycle, which will supply it through the same prop.
- **Background.** Everything behind the top layer is `inert`, not only `aria-hidden`: the app and every lower layer's
  popup.
- **Dismissal.** Escape and outside press dismiss **the top layer only**, and only when `dismissible`. A
  non-dismissible layer also disables its ✕. `interceptEscape` (on `Dialog`) is offered the top layer's Escape
  first; answering `true` spends it (the inspector sheet collapses expanded Files) and the layer stays open.
- **Name.** `<DialogTitle>` names the popup through `aria-labelledby`.
- **Close.** The corner ✕ is 32 × 32 and named.
- **Size.** `sm` 400 · `md` 520 (default) · `lg` 600 · `xl` 720, each capped at viewport − 48.
- **Placement** (`DialogContent placement`): `center` (default) is the card above; `sheet` is a full-height panel on
  the right edge, `min(560px, 100vw − 56px)` wide so the 56 px icon rail stays visible, with a `border-strong` left
  edge. Same focus, inert, dismissal and no-motion rules. The inspector below the shell breakpoint is its one user
  (ticket 12); its width grows to `100vw − 56px` (layout `className`) when its Files expand.
- **Focus return is decided while the layer is still open**, when everything behind it carries the stack's own
  `inert`. So the opener (and the main landmark) are judged by `staysInertAfterClose` — only inert that outlives
  the layer disqualifies them (ticket 12: the plain `closest("[inert]")` check sent every opener to the fallback in
  Chrome). A layer opened with focus nowhere (a startup overlay) returns it nowhere. The landmark is focused itself,
  not its first control (Base UI would hand a non-tabbable return element's focus to its first tabbable child).
- **Keep the shell mounted.** A dialog that is mounted only while open never returns focus (Base UI returns it on
  close, and an unmounted popup does not close). Re-point, Invocation, Detach and Assign keep one `Dialog` +
  `DialogContent` for the app's life and key only their body per subject.
- **Body.** `DialogBody` `variant`: `default` (padded) or `flush` (edge-to-edge lists).
- **Motion.** None: dialogs appear and leave instantly.

**ConfirmDialog** asks one question with two answers. Escape or an outside press means "cancel", the safe answer,
and there is no ✕.

- `tone: "danger"` puts the destructive button on the right and initial focus on Cancel.
- `tone: "accent"` (default) puts focus on Confirm.
- `initialFocus: "cancel" | "confirm"` overrides the tone's default. `cancel` is for a large answer that is not
  destructive: the bulk rail's count-bearing confirmations (Disable / Enable / Update / Sync over several skills) open
  on Cancel with an `accent` Confirm (ticket 13).
- `pending` shows Confirm as busy and disables Cancel.

The ask hooks (`useSharedDirConfirmation`, `useOverwriteConfirmation`) resolve their `request()` promise through it.

### Popover, DropdownMenu, Tooltip (`popover.tsx`, `dropdown-menu.tsx`, `tooltip.tsx`)

These are floating surfaces on `popover` with a `border` and `shadow-lg`. All three **appear instantly**.
Escape closes them and returns focus to the trigger.

- **DropdownMenu** items are 32 px tall. `variant="destructive"` both words the action and tints it.
  - The item holding keyboard focus draws the ring **inset** (`insetFocusRing`: 2 px `ring`, offset −2 px, so the
    scrolling popup cannot clip it): 6.01 / 5.04 against the highlight ground, 6.61 / 5.86 against `popover`. The
    highlight ground alone (`accent` on `popover`, 1.10 / 1.16) is never the cue. A pointer highlight shows the
    ground only.
  - `DropdownMenuLabel` names a group and must sit inside a `DropdownMenuGroup` (Base UI throws otherwise).
- **Tooltip** is a hint on an inverted surface. It is never the only carrier of a name or a state.
- **Popover without a trigger** (`PopoverContent anchor`): a control that raises a popover on its own terms — the
  row switch's inline Enable confirmation — renders a controlled Popover with no Trigger, positioned on `anchor`,
  with `initialFocus` on its answer and `finalFocus` back on the control. Escape closes it; focus returns to the
  anchor (ticket 13, `popover.test.tsx`).

### Toaster (`sonner.tsx`)

The toaster is `sonner`, themed entirely by tokens: every colour it draws is one of our variables, so no JavaScript
theme value is needed. State toasts put state-coloured text on an 8 % tint of that state over `popover` (ratios
above). `useStatusReporter` owns toast lifetime; the binder mounts the one `<Toaster>`.

## Focus ring

- One ring everywhere: a **2 px solid `ring` outline, offset 2 px** (1 px for text fields, −2 px inside menu items),
  on `:focus-visible` only.
- **The style is explicit.** Tailwind 4's `outline-none` sets `--tw-outline-style: none` and `outline-2` draws
  `outline-style: var(--tw-outline-style)`, so every ring carries `focus-visible:outline-solid` (`focusRing`,
  `fieldFocusRing`, `insetFocusRing` in `ui/styles.ts`). `ui/focus-ring.test.tsx` resolves each primitive's rendered
  classes against CSS compiled by the installed Tailwind; a trusted-keyboard run measured `solid 2px` on every
  primitive in both themes (ticket 07, 2026-09-29).
- **No transition**: the ring never animates in.
- It measures ≥ 5.04:1 against every ground in both themes.
- Primitives draw it through the ring constants in `ui/styles.ts`. The base layer (`index.css`) gives every other element the
  same ring, legacy surfaces included.
- A legacy rule that removes the outline must replace it with an `accent-primary` (iris) edge. Every current one
  does.

## Motion

- **What moves**: only the ⌘K palette, the bulk rail and the switch thumb. The motion is opacity and transform only,
  **≤ 120 ms**, on `ease-out` (`cubic-bezier(0.16, 1, 0.3, 1)`).
- **The bulk rail** rises on entry only: `@starting-style` (`starting:opacity-0 starting:translate-y-2`) with
  `transition-[opacity,translate] duration-(--motion-fast) ease-out`, so it is interactive from its first frame and
  leaves instantly; `motion-reduce:transition-none` (measured: 0 → 1 opacity in ≤ 112 ms of rAF samples; under
  reduce the first frame is already opacity 1, `transition: none`).
- **What does not move**: dialogs, popovers, menus and tooltips appear and leave instantly.
- **Hover and press** change colour instantly in the primitives, which use no `transition: all`. Legacy `App.css`
  still carries 14 `transition: all` declarations (22 before My Skills migrated; mostly 200 ms) until its surfaces migrate (ticket 15 and the
  surface tickets); reduced motion already zeroes them.
- **Reduced motion.** `prefers-reduced-motion: reduce` removes all motion. The base layer zeroes every transition and
  animation, legacy App.css loops included, and primitives also carry `motion-reduce:` / `motion-safe:`. Spinners
  (`pending`, the progress overlay) stop under reduce.

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

## My Skills (the first migrated surface, ticket 11)

Composed from the primitives and tokens only; no variant was added. Files: `components/shell/AppSidebar.tsx`,
`components/skills/{LibraryHeader,FilterBar,SkillsList,SkillRow,SyncedStrip}.tsx`.

- **Shell.** Sidebar 200 px on `card` (`sidebar` tokens): the areas My Skills · Explore · Projects, then at the
  foot Notifications (unread count as a `solid accent` Badge) · Theme (a menu) · Keyboard shortcuts (a popover) ·
  Settings. Items are 32 px; the current area has `aria-current="page"`, a `raised` ground and an iris icon
  (6.01 dark / 5.04 light on `raised`). In the icon rail every label stays in the accessibility tree (`sr-only`)
  and is the hover title.
- **Header row** (48 px): title (`lg`, the `data-surface-heading` focus target) + count (`faint`) · the
  exception summary · a status region reserved for ticket 14's progress · Configure Tools (`ghost`) · Refresh
  all (`outline`) · **Add skill** (`solid accent`, the one primary). The summary reads the health fold over the
  whole library: "N failed targets" (`failed` text) · "N stale" (`stale` text) · "View N skills with issues"
  (`ghost accent` Button that presses the Issues chip). Zero parts are dropped; a healthy library shows nothing.
  A skill has an issue when its loudest fold row is failed or stale — the Issues chip keeps exactly those.
- **Filter bar** (40 px): search (`Input sm`, labelled "Search skills" through a Field; `/` focuses it, Escape
  clears it) · Chips All · Issues · Edited · Disabled with their counts (`faint`) · Segmented Grouped / Flat · one
  "Sort and view" menu (radio groups). Auto-sync moved to Settings; library-wide Disable is the bulk rail's
  (ticket 13) — nothing library-wide is one click away.
- **Row** (36 px, `h-row`, `border` hairline, `card` on hover, `raised` + a 3 px iris edge when selected): checkbox · name
  (`sm` medium; the Tab stop that opens the skill) + Edit pencil (`ghost icon-sm`, only when overridden) +
  **exception marks** (soft Badges: `failed` with a ⊗ icon, `stale` with a ⚠ icon, `neutral` for "Not
  deployed"; skill-level rows first, then target-level, in the fold's order; Disabled rows stay silent because the
  strip and switch say them; in a list narrower than 48 rem — the inspector beside it — the marks compact to their
  icons (`neutral` gains ⊖) with the words kept as their accessible text and tooltip, so the name keeps its room) · description (`muted-foreground`; it shares the name's track and yields to a long
  name + marks; in Flat and in "Individual sources" preceded by the source in `font-mono xs faint`) ·
  **Synced strip** + N/M · scope (globe when some target is live) · Switch `sm` (= skill-grain Disable / Enable
  through the door) · overflow (`ghost icon-sm` → DropdownMenu:
  a "Fix" group of the fold's repairs, then Update, Change source…, Detach, Edit invocation…, Remove).
  Clicking anywhere but a control opens the skill.
- **Group header** (36 px, sticky, `card`): source icon · name (`h2`, links to the repo; the text inside the link
  truncates, never an ancestor, so the link's ring paints whole) · count · state summary ("1 failed · 2 stale ·
  1 not deployed · 1 disabled", state-coloured words) · **Deploy all** (`ghost sm`; disabled with a reason when
  every skill is deployed). Sources with one skill fold into "Individual sources"; no source pill repeats inside a
  group.
- **Card view** (Auto / Dense grid) renders the same anatomy on a `card` tile with the same strip — a stop-gap
  until round 19b designs the grid (F07's clipping is gone: marks never clip).

### The Synced strip

One 12 px mark per Tool of the effective target set (registry order), then any Tool the skill still has a row
for outside that set. It is **read-only** (D22): each mark is `role="img"` with its own name, and the row opens the
skill. Shape carries the state; colour only reinforces it; the strip was checked in grayscale
(`.scratch/round19/evidence/11/*-grayscale.png`).

| Mark | Shape | Colour | Accessible name | Dark / light on background · card · raised |
|---|---|---|---|---|
| synced | filled dot | `muted-foreground`; `input` when every mark is synced (the quiet N/N strip) | "Cursor: synced" | 8.31 · 7.78 · 7.08 / 8.16 · 7.60 · 7.20; quiet 3.90 · 3.65 · 3.32 / 3.78 · 3.52 · 3.33 |
| not synced | hollow ring | `input` | "Amp: not synced" ("…not synced yet" for a pending row) | 3.90 · 3.65 · 3.32 / 3.78 · 3.52 · 3.33 |
| stale | half-filled ring | `stale` | "Codex: stale — the copy differs from the skill" | 10.41 · 9.75 · 8.88 / 5.62 · 5.23 · 4.95 (token table) |
| failed / missing | × | `failed` | "Amp: failed — the last sync did not complete" ("…first sync…", "…the deployed copy is gone") | 6.91 · 6.47 · 5.89 / 5.50 · 5.12 · 4.85 |
| broken | × | `failed` | "Pi: broken — the central copy is missing" | as failed |
| disabled | slashed ring | `faint` | "Codex: disabled" / "…disabled — the skill is off everywhere" | 6.12 · 5.73 · 5.22 / 6.09 · 5.67 · 5.37 |

- A Tool detection no longer sees is named "Windsurf (not detected): …"; a label is always the Tool catalog's.
- A skill whose **central copy is missing never shows a filled mark**: a synced or stale row is `broken` (its link
  dangles). A skill disabled at skill grain shows every mark disabled and its switch off.
- N/M: N counts synced and stale marks (a deployed copy exists), M every mark; `faint` when complete.
- The strip is near-invisible at N/N by *colour step*, not below 3:1: the quiet marks drop from
  `muted-foreground` to `input`, the lowest non-text token that still identifies a mark on its own.

### Empty and first-run

- **Empty library** (`?scenario=empty`): the shell stays; the header keeps only Configure Tools; the body is one
  heading, one sentence, and the next step — **Add from GitHub** (`solid accent`), Add from a folder
  (`outline`), Browse Explore (`ghost`). No filter bar, no Refresh.
- **Discovered skills** (`?scenario=first-run`): a quiet `card` row at the top of the list (inbox icon, neutral
  words, no amber) — "Skills already in your Tools · 12 skills sit in your Tools' folders…" — with **Review N
  discovered skills**. While the discovery outweighs the library (N ≥ library size) Review is the one
  `solid accent` action and Add skill steps back to `outline`; otherwise Review is `outline accent` and the list's
  real issues lead. In an empty library with discoveries, Review leads the empty state's buttons.
- **The rare delight** (D6; implemented with Import's migration, ticket 21): when an import of ≥ 10 skills
  completes, the imported rows' strips light mark by mark, left to right, 20 ms apart (opacity 0 → 1, each
  ≤ 120 ms, `ease-out`), once, and the header summary reads "12 skills imported · all synced" for one run.
  Nothing else moves; reduced motion shows the settled strip at once.

## The inspector (ticket 12)

The focused skill — surface state's `focused`, a selection not a route (D3). Files:
`components/skills/inspector/{SkillInspector,InspectorSection,HealthSection,TargetsSection,SourceSection,EditsSection,ProjectsSection,FilesSection}.tsx`,
`components/skills/{SkillFiles,StripMarkIcon,ExplorePreviewView}.tsx`, `modals/{RepointModal,DetachModal,AssignProjectModal}.tsx`.
Composed from primitives and tokens; the one new variant is Dialog `placement="sheet"`.

- **Placement.** ≥ 1200 px: an `<aside>` (landmark, `aria-labelledby` its `h2`) on `card`, beside the list below the
  header and filter bar — `max(22.5rem, ⅓)` of My Skills, `border` left edge. < 1200 px: the same content in a Dialog
  `sheet`. Header: name (`lg` semibold, the `data-inspector-heading` focus target) · description (`sm`
  `muted-foreground`, 3 lines) · Close (`ghost icon`, 32 px).
- **The focused row** shows it: `aria-current="true"`, `raised` ground and a 1 px inset `ring` edge (the ≥ 3:1 cue:
  6.01 dark / 5.04 light on `raised`); a selected (checked) row has the ground and a 3 px `ring` edge on its leading
  side (ticket 13) — the two coexist.
- **Sections** (`h3` `sm` semibold, a `border` hairline between): Health · Sync targets · Source · Edits · Projects ·
  Files, then Remove (`outline danger sm`) in a footer. Section controls are `sm` (24 px) Buttons.
- **Health** reads `skillHealth` alone: per row an icon (⊗ `failed`, ⚠ `stale`, ⊖ `faint`), the label (`sm` medium),
  what happened (`xs` `muted-foreground`: the typed error in words, the Tools, when) and its repairs (`outline sm`; the
  first repair of the first row `accent`, removal `danger`). Healthy is one quiet line. The section is Restore's
  focus target (`data-inspector-health`); spanning the scroll area edge to edge, it draws its ring inset
  (`insetFocusRing`) so all four sides paint.
- **Sync targets** (D22): a list by default — the strip's glyph (`StripMarkIcon`, shared with the row), the Tool,
  `link`/`copy` in `faint`, the state in words (`failed` text for failed / missing / broken, else
  `muted-foreground`), inline actions: Retry (`outline`), Disable / Enable / Sync (`ghost`). A row's first action is
  the same element as it changes role, so focus stays on it. Disable shows "Removed from {Tool}" (`role="status"`)
  with **Undo** (`ghost accent`) for 5 s — in the list on the Tool's row, under the grid as one line (the tile keeps
  focus). The grid (Segmented List / Grid, persisted `skills-inspector-targetsView`)
  is `outline md` tiles, glyph + Tool; each names its state and what a press does. A skill off everywhere or with
  its central copy gone offers no per-Tool action — Health carries the repair.
- **Source**: icon · the source line (`sm` underlined link for a repo; a recorded folder in `font-mono xs`) · Copy
  (`ghost icon-sm`) · provenance (`xs`; a missing folder in `stale`) · "Last updated" (`faint`) · Update, Change
  source…, Detach.
- **Edits**: the invocation mode as a term/definition pair; a conflict in `stale` text and the Edit button toned
  `stale` ("Resolve…"). The Edit dialog then leads with the plain choice: **Keep yours ({mode})** (`solid accent`) ·
  **Use the skill's ({mode})** (`outline`).
- **Projects**: each project with its Tools' statuses (`failed` text for failed / missing) and Remove (`ghost sm`);
  **Assign to project…** opens a picker listing every project with its configured Tools (radio rows; a project with
  no Tools, or already on all of them, cannot be chosen).
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
  inspector focuses the new heading; closing returns to the row; Restore lands on Health. A resize never reorders
  layers: while a dialog (or the progress overlay) is open above the inspector it keeps its placement, and the new
  one lands when the last layer closes. The Projects section and the picker read the project world's held views,
  which every assignment's returned view updates — no read follows a successful assignment.

## Selection, the bulk rail and the Delete dialog (ticket 13)

Files: `components/skills/{BulkRail,useSkillListKeymap}.ts(x)`, `modals/{CountConfirmModal,DeleteModal}.tsx`, the
selection in `SkillRow` / `SkillsList`, the counts in `lib/bulkCounts.ts`. Composed from primitives and tokens; the
new primitive options are ConfirmDialog `initialFocus` and PopoverContent `anchor` (above).

- **Selection** is My Skills' surface selection (`useSurfaceState`, D4). A selected row: checked checkbox (`primary`
  fill) + `raised` ground + a 3 px `ring` edge on its leading side — iris's selection use. Measured on `raised`:
  edge 6.01 dark / 5.04 light; checked box 3.49 / 5.04. A group header's checkbox selects its source (`mixed` when
  part of it is selected).
- **Keys** (one keymap, `useSkillListKeymap`; listed in the sidebar's Keyboard shortcuts): `/` search · `j` / `k`
  next / previous row · Enter open · `x` select the row holding focus · ⇧-click a range from the last row toggled ·
  ⌘A every row the lens shows (the visible subset) · Esc clears the selection, then collapses Files, then closes the
  inspector. None fires while typing or while a dialog, sheet, popover or menu is open. Clearing from inside the rail
  hands focus to the row last toggled (the rail goes with the selection).
- **Bulk rail** (`card`, `border-strong` top edge, at the foot of the list; first in the DOM, so `/` then Tab reaches
  it): "N selected" (`sm` semibold, a `role="status"`) · the `x` / `Esc` hints (kbd, hidden below the shell
  breakpoint) · `outline sm` Buttons Sync to Tools… · Assign to project… · Disable on all Tools · Enable · Update ·
  Remove… (`danger`) · Clear (`ghost icon-sm`). A verb that would change nothing is disabled with the reason as its
  title; every rail Button keeps focus while disabled (`focusableWhenDisabled`), so D21's "same control" holds after
  it ran. Text on the rail: ≥ 5.22 dark / 4.85 light.
- **Count-bearing confirmation** (`confirmCount`): ConfirmDialog, focus on Cancel, one sentence of counts from
  `bulkCounts` ("Disable 25 skills on 8 Tools — 80 Sync targets are removed and kept disabled; 12 project assignments
  for these skills are disabled too. Refresh will not re-add them."), a `muted-foreground` second line for what
  changes nothing. Sync to Tools carries its Tool pick (a two-column checkbox FieldSet) and the counts follow the pick.
- **Row switch**: Disable is never asked; Enable that will sync the skill to Tools again asks in a quiet anchored
  Popover ("Enable retired-playbook and sync it to 2 Tools again?" · Not now `ghost` · Enable `outline accent`).
- **Delete dialog** (D16), one Dialog with two states. Confirm: the Tools it is removed from, the projects that carry
  it, the Edit lost, "Skills Hub's copy is deleted" (`muted-foreground`), and for a skill with no external source
  "Skills Hub's copy is the only copy — this cannot be undone" (`failed` text with ⚠); several skills read as counts
  plus their names (six, then "and N more"). Footer: Cancel (initial focus) · Remove (`solid danger`, `pending` while
  it runs — it keeps focus, so focus never reaches `<body>`). Report: the heading takes focus ("release-train was not
  deleted" / "1 of 3 skills was not deleted"); per kept skill "Removed from Codex, Pi." then per kept path a `card`
  block — "Still in Claude Code:", the path (`font-mono xs`), the reason (`failed xs`, `describeCommandError`),
  **Reveal in Finder** (`outline sm`); footer Close · Retry (`solid danger`, the kept skills only). The heading is
  `tabIndex=-1` (focusable, never a Tab stop).

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
