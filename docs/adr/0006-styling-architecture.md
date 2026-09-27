# Styling uses Tailwind v4 tokens, utilities in JSX and a primitives module over shadcn/Base UI; `App.css` ratchets to zero

The frontend's styling was one global stylesheet. `src/App.css` had 3,674 lines at `cc34449` (3,696 at the round-18
baseline) and 530 rule blocks, against about 500 `className` sites. Its selectors are named per surface (`explore-*`
48 families, `matrix-*` 41, `settings-*` 40, `modal-*` 23, …). Nothing links a class to the component that uses it,
so a surface cannot be changed or deleted locally. Deleting a component leaves its rules behind, and changing a rule
can silently restyle another surface. About half of the class sites are six recurring shapes built by hand at each
site: button (`btn*`, 133 tokens), form field, dialog shell, chip/badge, checkbox or switch, and tabs or segmented
control. The one existing primitive, `components/shared/Modal.tsx`, is shallow. It has four `className` escape
hatches, so styling leaks across the seam, and it has no focus trap or focus return. The stylesheet also has
22 `transition: all`, no `@media` rule and no `prefers-reduced-motion`. Tailwind v4 is wired in but used at about
ten sites. See ticket 05, `.scratch/round18/review/architecture.md` candidate 4.

Round 18 (spec D1–D3) replaces the visual world once and then refines every surface into it. **We decided:**

- Tokens live in one place, `@theme` (Tailwind v4).
- Utilities are written in JSX.
- A deep **primitives module**, `src/components/ui/`, has props (`variant`, `tone`, `size`) as its styling
  interface.
- **shadcn** supplies the behavioural primitives (Dialog, Popover, DropdownMenu, Tooltip, Checkbox, Switch,
  Tabs, Command) on **Base UI**. They are copied in and restyled to our tokens.
- `App.css` shrinks to zero surface by surface, behind a CI guard.
- A named prose stylesheet is the only exemption.
- The resolved theme has one owner.

This reverses the AGENTS.md rule "styles live in `App.css` / `index.css`".

## Decision

### 1. Tokens: one file, semantic names, values deferred to the world ticket

Ticket 05 found that the tokens mostly exist already: 38 custom properties in `index.css`, read by 513 `var(--…)`
calls in `App.css`, with only 11 hex and 13 `rgba()` literals left. Moving them into `@theme` is a relocation plus a
renaming pass. The real token work is the new **values** of the world. This ADR fixes the token **set and its
shape**. The round-19 world ticket fixes the values, taking them from the direction contract in
`.impeccable/surfaces/src-components-skills-skillslist-tsx.md`, and records them in `DESIGN.md`.

- **Where tokens live.** `src/styles/theme.css`, imported by `src/index.css`, is the only file that defines a token.
  It has three parts:
  - raw values as plain custom properties, dark in `:root` and light in `:root[data-theme='light']`;
  - one `@theme inline { --color-background: var(--background); … }` block that registers them as utilities;
  - `@custom-variant dark (&:where([data-theme=dark], [data-theme=dark] *));`.

  This is the pattern shadcn documents for Tailwind v4 (`customization.md` in the shadcn skill), so copied-in
  primitives keep their class vocabulary. `@theme` cannot be nested under a selector, and `inline` makes a utility
  resolve to the switching variable ([Tailwind: theme variables](https://tailwindcss.com/docs/theme),
  [dark mode via a data attribute](https://tailwindcss.com/docs/dark-mode)). Tokens that the prose layer reads only
  through `var()` need `@theme static` or a `:root` definition, because Tailwind emits only the variables it uses
  (same page).
- **Dark first.** `:root` carries the dark values (D1: dark is designed first). Light is the override block, a
  derivation of dark, not a second design.
- **Two tiers, no raw palette tier.** The world is Restrained: about a dozen colours, each with one meaning. A
  palette tier (`gray-50…950`) would only invite raw-colour utilities.
  - The **semantic tier** takes shadcn's names so that restyled primitives change values, not classes:
    `background`, `foreground`, `card`, `popover`, `primary`, `muted`, `accent`, `destructive`, `border`, `input`,
    `ring`, `sidebar-*`, each with `-foreground` where it applies.
  - The **world extension** adds only what the direction contract names and shadcn lacks: `raised` (third surface
    level), `border-strong`, `faint` (third text level), and the three state colours `ok`, `stale`, `failed`.
  - `primary` is the iris accent, used only for selection, focus and the one primary action.
  - Non-colour tokens: radius (4/6/10), type scale (12/13/14/16), row height (36), and motion durations and easings.
    Spacing keeps Tailwind's 4px base.
- **Legacy names do not survive, but they bridge the migration.** `--bg-app`, `--text-secondary`, `--accent-*`,
  `--status-*` and the other legacy names go away. Until `App.css` is gone, it opens with an **alias block** that maps
  each legacy name onto the new token (`--bg-app: var(--background)`). Un-migrated surfaces therefore render in the
  new world's colours from the day the world ticket lands. The app is never half old palette and half new. The alias
  block is deleted together with the last legacy rule.
- **Fonts.** The Google Fonts `@import` goes. The world uses the system UI stack and `ui-monospace` (the direction
  contract and PRODUCT.md say no network fonts).

### 2. Utilities in JSX; the primitives module is the deep module

Surface layout (`explore-*`, `matrix-*`, `detail-*`, `tree-*`, `notif-*`) becomes Tailwind utilities in the
component's JSX, composed with one `cn()` helper (`clsx` + `tailwind-merge`, both already dependencies). The recurring
shapes become primitives in `src/components/ui/`:

- Button;
- field (label, input, select, helper text, error);
- Dialog, including the confirmation dialog;
- Badge/Chip;
- Checkbox and Switch;
- Tabs or segmented control;
- plus Popover, DropdownMenu, Tooltip and Command.

A primitive's **interface is its props**:

- `variant` is visual weight: solid, outline, ghost, …
- `tone` is semantic role: neutral, accent, ok, stale, failed/danger.
- `size` is density.
- Behaviour props come from Base UI.

A primitive's **depth** is what it owns so that call sites do not have to: the focus ring, disabled and pending
semantics (`aria-disabled`, `aria-busy`), reduced motion, ARIA wiring, and the token mapping for every
variant × tone × size. `className` on a primitive is for **layout only**: margin, flex or grid placement, width. It
never recolours or restyles. This follows the shadcn rule "`className` for layout, not styling". A styling need
that props cannot express becomes a new variant, documented in `DESIGN.md`, not an override at the call site.

`Modal.tsx` is replaced by the Dialog primitive. The "ask" hooks keep their shape: `useSharedDirConfirmation` and
`useOverwriteConfirmation` resolve a boolean from `request()`. Ticket 05 rated them model deep modules; they get a new
shell underneath.

### 3. Behavioural primitives: shadcn on Base UI, not Radix

**Base UI, because it is shadcn's default and recommended base for new projects since July 2026 and a stable 1.x
line released monthly. It matches Radix on the behaviour we need (focus trap and return, origin-aware anchored
positioning, React 19). It fits D5/D6 better: `initialFocus`/`finalFocus` can branch on keyboard vs pointer
opening, and enter/exit uses interruptible CSS transitions (`data-starting-style`/`data-ending-style`) rather than
keyframes. Radix's smaller bundle does not matter in a webview that loads from local disk.**

| Criterion | Base UI (`@base-ui/react` 1.8.0) | Radix (`radix-ui` 1.6.7) | Evidence |
|---|---|---|---|
| shadcn support | Default base since 2026-07. "Starting something new? We recommend Base UI." Every component ships for both. | Fully supported, not deprecated. "If your app works, keep shipping." | [shadcn changelog 2026-07](https://ui.shadcn.com/docs/changelog/2026-07-base-ui-default), [2026-01 Base UI docs](https://ui.shadcn.com/docs/changelog/2026-01-base-ui) |
| Maturity, React 19 | 1.0 stable 2025-12-11. Monthly minors to 1.8.0 (2026-09-04). Peers `react ^17 \|\| ^18 \|\| ^19`. | Active. 2026 releases fixed a React 19 infinite re-render loop (Slot/composed refs) and stale Escape/dismiss handlers on React 19.2. | [Base UI releases](https://base-ui.com/react/overview/releases), [Radix releases](https://www.radix-ui.com/primitives/docs/overview/releases) |
| Focus management | Dialog `modal: true \| 'trap-focus'`. `initialFocus`/`finalFocus` take a ref or a function of the interaction type (`mouse`/`touch`/`pen`/`keyboard`). | Focus scope with `onOpenAutoFocus`/`onCloseAutoFocus`. Esc returns focus to the trigger. | [Base UI Dialog](https://base-ui.com/react/components/dialog), [Radix Popover](https://www.radix-ui.com/primitives/docs/components/popover) |
| Origin-aware popovers | Positioner exposes `--transform-origin`, `data-side`/`data-align`, `--available-*`, a custom `anchor`, and collision avoidance (Floating UI). | `--radix-popover-content-transform-origin`, `data-side`/`data-align`, custom anchor, collision handling. | same pages |
| Motion model (D6) | `data-starting-style`/`data-ending-style` drive plain CSS **transitions**, which are interruptible and fit `motion-reduce:` utilities. | `data-state` plus enter/exit keyframe animations (shadcn pairs them with an animation utility package). | same pages |
| Bundle (the 7 non-Command primitives, esbuild 0.28 minified, React included) | 240 KB min / 81 KB gzip | 140 KB min / 45 KB gzip | measured 2026-09-25 in a scratch dir outside the repo |

Caveats:

- **Command stays on `cmdk` under both bases.** shadcn's Command wraps `cmdk` 1.1.1 on the Base UI docs too
  ([shadcn Command (base)](https://ui.shadcn.com/docs/components/base/command)). `cmdk` depends on
  `@radix-ui/react-dialog`, so a Base UI app still ships one Radix dialog (about 23 KB gzip for `cmdk` in total).
  The ⌘K palette ticket decides between this route and composing the palette from Base UI's
  `Autocomplete`/`Combobox` plus our Dialog.
- **Toasts stay on `sonner`.** shadcn's default toast for Base UI projects is Base UI Toast (skill rule
  "Toast follows the project base"). But `useStatusReporter` owns toast lifetime over `sonner`, and toast is not one
  of the eight behavioural primitives. Switching toast libraries is a separate decision, not a consequence of this
  one.
- `components.json`:
  - Base UI is selected by `init -b base`, the default.
  - `tailwind.cssVariables: true`, `tailwind.css: src/index.css`, `iconLibrary: lucide`.
  - `style` cannot be changed after init ([components.json](https://ui.shadcn.com/docs/components-json)). Because
    every primitive is restyled, the world ticket picks the most neutral style.

### 4. The resolved theme has one owner; styling never reads it from JavaScript

Today `useSettingsState` resolves `light | dark | system` against `prefers-color-scheme` and writes `data-theme` and
`color-scheme` on `<html>`. `SkillDetailView.tsx:389` reads `data-theme` back off the DOM during render, so the
syntax highlighter keeps the old theme when the theme changes while the detail is open.

Decided:

- **The settings world stays the single writer of the resolved theme** on `<html>`, and it adds `resolvedTheme`
  to its returned interface.
- **Styling reads the theme only through tokens.** CSS variables switch under `[data-theme]`, so no component
  needs to know the theme in order to look right.
- The rare JavaScript consumer that needs the value (a third-party renderer that only accepts a JS style object)
  receives `resolvedTheme` as data, passed from App like any other world state. That follows the no-Context,
  no-state-library rule.
- **Reading `data-theme` from the DOM is forbidden.**
- The only current JS consumer goes away. The syntax highlighter moves to class output
  (`react-syntax-highlighter` `useInlineStyles={false}`), and its colours become prose-layer rules that read tokens.
  The hard-coded line-number colours (`#636d83`, `#9ca3af`) move with them.

### 5. The prose layer is the one named exemption

`App.css` will not reach literal zero. Rendered Markdown (`markdown-body`, 30 rules) and syntax-highlight colours
style **content we do not author as JSX**, so utilities cannot reach it. They move to **`src/styles/prose.css`**.
The guard exempts that file under two rules:

- every selector is scoped under a prose root class, named by the world ticket;
- it uses tokens only, with no colour literals.

It is not a place for surface styling. Everything else reaches zero. The permanent stylesheets are exactly
`src/index.css` (entry and base layer), `src/styles/theme.css` (tokens) and `src/styles/prose.css`.

### 6. The migration rule and the CI guard

**A surface is migrated when its component files reference no class that `App.css` defines.** CSS is global, so a
component never imports `App.css`. "Imports nothing from `App.css`" in the ticket means this, and it is the only
checkable form.

The guard is **`scripts/check-styles.mjs`**, run as `npm run styles:check`. It is part of `npm run check` and the CI
`web` job, and it reads a committed baseline, `scripts/styles-baseline.json`. It fails when:

1. **`App.css` grows.** Its non-blank, non-comment line count must equal the baseline's `appCssCeiling`. This is a
   ratchet: a commit that shrinks the file lowers the ceiling in the same commit. The world ticket sets the first
   ceiling after it adds the legacy-alias block, which is the last addition `App.css` ever gets. When the file is
   deleted, the check becomes "`App.css` must not exist".
2. **A migrated surface references a legacy class.** Every file listed in the baseline's `migrated` array is scanned
   for class tokens, and any token that matches a class selector in `App.css` fails the check. Files are added to the
   list when their surface migrates and are never removed.
3. **A new stylesheet appears.** Any `*.css` under `src/` other than the three permanent files and `App.css` fails.
   CSS Modules stay unused, now because of the guard rather than prose.
4. **The prose file leaks.** A selector in `prose.css` outside the prose roots, or a colour literal there, fails.
5. **A raw colour enters JSX.** An arbitrary colour value (`bg-[#…]`, `text-[rgb(…)]`) or a Tailwind palette
   utility (`text-red-500`) in `src/components/**` fails the check. The shadcn skill's rule is "semantic colours,
   never raw values".

The round-19 world ticket builds the guard with the tokens, the primitives and the first migrated surface (My
Skills). Surfaces then migrate one ticket at a time, in sequence (D9). Each ticket lowers the ceiling and extends
`migrated`.

### 7. Tests for primitives: a narrow exception is proposed, and ticket 09 decides

AGENTS.md rules out component rendering tests. The reason is that components are thin JSX and behaviour lives in
hooks and pure folds. That reason does not hold for `src/components/ui/`: a primitive's behaviour is the product
there, and D5 makes accessibility a hard gate. **Proposed:** allow colocated `src/components/ui/*.test.tsx`, using
`@testing-library/react` (already a devDependency) on jsdom. These tests assert only **our wrapping's contract**:

- role and accessible name wiring;
- ARIA state that reflects props (`aria-checked`, `aria-disabled`, `aria-busy`);
- Esc closes, and focus returns to the trigger.

There are no snapshots, no visual assertions, no re-testing of Base UI internals, and no test that renders a
surface. jsdom cannot judge contrast, `:focus-visible` or motion. Those stay with the D5 audit lane: impeccable
`audit` or axe over the ticket-04 fixture app in `preview_*`, both themes, keyboard walk. Until ticket 09 accepts
the exception, the AGENTS.md test rule stands unchanged and this ADR's other decisions do not depend on it.

## Considered options

- **Keep `App.css` and add BEM-style discipline.** Rejected: it fails the locality and deletion tests that ticket 05
  measured. Variant logic stays repeated at 133 `btn*` sites, and no rule could be enforced short of a class-ownership
  map nobody maintains.
- **CSS Modules per component.** Rejected: this fixes locality but not depth. Each surface would still hand-build
  its buttons and fields, and the world would be spread across N files instead of one `@theme`.
- **Radix under shadcn.** It is equally capable and has a smaller bundle. Rejected on direction: shadcn now builds
  new work Base UI-first, the motion model fits D6 worse, and the React 19 fixes Radix shipped in 2026 were to
  regressions we would inherit. Reconsider if the palette ticket keeps `cmdk` and the operator prefers one primitive
  family over the leaner-by-default Base UI.
- **React Aria as the base** (`shadcn init -b aria` exists). Not evaluated in depth. Reconsider only if a Base UI
  primitive fails the D5 audit.
- **Base UI directly, without shadcn.** Rejected: shadcn supplies the Tailwind composition, the part structure and
  a CLI diff path (`add --diff`) for upstream fixes, at no lock-in cost, because the code is copied in and ours.
- **Big-bang restyle.** Rejected by D1 (80/20) and D9 (sequential surfaces). The ratchet lets every intermediate
  commit ship.

## Consequences

- AGENTS.md: "Styles live in `App.css` / `index.css`" is replaced by this rule, and a new invariant says a new
  primitive's variants are documented in `DESIGN.md`. `DESIGN.md` does not exist yet; the world ticket creates it.
  The rule "Confirmations go through the Modal shell" becomes "… through the Dialog primitive" when `Modal` is
  replaced.
- New dependencies in round 19: `@base-ui/react`, `class-variance-authority` (shadcn's variant helper), `cmdk` and
  its Radix dialog unless the palette ticket decides otherwise, and the shadcn CLI run through `npx` (no dependency).
  A `components.json` and a `@/` import alias (tsconfig `paths` plus Vite `resolve.alias`) arrive with the world
  ticket.
- Legacy surfaces adopt the new palette the day the world ticket lands, through the alias block, before their own
  migration.
- `SkillDetailView`'s DOM theme read and the highlighter's inline JS style objects go away when the detail surface
  migrates.
- The dormant `Layout.tsx` and `Dashboard.tsx` keep referencing legacy classes. They are never listed as migrated.
  When `App.css` is deleted they render unstyled, which is harmless because nothing imports them. Deleting them
  stays an operator call.

## What would change this

- **Base UI vs Radix.** A Base UI primitive that fails the D5 keyboard or screen-reader audit on the fixture app,
  and that we cannot fix in our copy, reopens section 3.
- **Token shape.** If the world ticket finds that the shadcn semantic names cannot express the direction contract
  without more than a handful of extensions, rename toward the contract's own vocabulary (ground, panel, raised,
  line, iris) and accept restyling the copied primitives' classes.
- **Theme flash.** The theme is applied in an effect after first render, and the Tauri window has its own
  background colour. If a dark-first `:root` still flashes for light users, a pre-paint theme read is needed. That
  collides with "storage access only through `preferences.ts`", so it is a grill decision, not an implementation
  detail.
