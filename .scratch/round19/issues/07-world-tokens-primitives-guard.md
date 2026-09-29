# 07 — World ticket: `theme.css`, `prose.css`, primitives on Base UI, `styles:check`, `DESIGN.md`, pre-paint theme, min window

Status: implemented
Blocked by: —
Spec: `.scratch/round19/spec.md` — D1, D2, D6, D28, § "What the world ticket fixes in values"; ADR-0006 §1–§7 in full;
direction contract `.impeccable/surfaces/src-components-skills-skillslist-tsx.md`. D9: this ticket owns the styling
hotspots; no other child touches `src/styles/`, `src/components/ui/`, `src/index.css`, `src/App.css`, `scripts/check-styles.mjs` while it runs.

## Work

Read ADR-0006 first; it is the design. Read the `shadcn` and `tailwind-design-system` skills before `shadcn init`.

1. **Tokens — `src/styles/theme.css`**, imported by `src/index.css`, the only file that defines a token. Raw values dark
   in `:root`, light in `:root[data-theme='light']`; one `@theme inline { … }` registering the semantic tier (shadcn's
   names: `background foreground card popover primary muted accent destructive border input ring sidebar-*` with
   `-foreground` where it applies) plus the world extension (`raised`, `border-strong`, `faint`, `ok`, `stale`,
   `failed`); `@custom-variant dark (...)`. Non-colour tokens: radius 4/6/10, type 12/13/14/16 (tabular numerals),
   row height 36, motion durations/easings (≤120 ms). Values from the contract (spec § values). **Light is measured**:
   every foreground/background pair in `DESIGN.md`'s table carries its WCAG ratio; nothing under 4.5:1 for text or
   3:1 for non-text ships (the audit's failing pairs are the regression list: tertiary text, synced/failed pills,
   Install/Installed, success toast, placeholder, Match/Conflict badges).
2. **Alias block** at the top of `App.css` mapping every legacy custom property (`--bg-app`, `--text-secondary`,
   `--accent-*`, `--status-*`, … — enumerate from `index.css`) onto the new tokens, so un-migrated surfaces render in
   the new palette from this commit. The Google Fonts `@import` goes; system UI stack + `ui-monospace`. This is the
   last addition `App.css` ever receives.
3. **`prose.css`** (`src/styles/prose.css`): `markdown-*` rules and the syntax-highlighter theme move here, scoped
   under one prose root class, tokens only; `react-syntax-highlighter` switches to `useInlineStyles={false}` and
   the hard-coded line-number colours become token reads. `SkillDetailView`'s `data-theme` DOM read is deleted;
   `useSettingsState` returns `resolvedTheme` (ADR §4) and App passes it where a JS consumer remains (there should
   be none after this).
4. **Primitives — `src/components/ui/`** via `npx shadcn init -b base` (`components.json`: `cssVariables: true`,
   `css: src/index.css`, `iconLibrary: lucide`, most neutral style; `@/` alias in `tsconfig` + Vite `resolve.alias`),
   then `add` and restyle to tokens: **Button** (variant solid/outline/ghost · tone neutral/accent/ok/stale/danger ·
   size), **Field** (label, input, select, helper, error — label always associated), **Dialog** + **ConfirmDialog**
   (focus trap, `initialFocus`/`finalFocus` accepting a fallback function, top-layer-only Escape, `inert` background,
   named by `aria-labelledby`, Close ≥32 px), **Badge/Chip**, **Checkbox**, **Switch**, **Tabs/Segmented**, **Popover**,
   **DropdownMenu**, **Tooltip**. Each owns focus ring (visible in both themes, ≥3:1 against its surroundings, no
   transition delay), `aria-disabled`/`aria-busy`, reduced motion (`motion-reduce:`), target floor (24/32). `className`
   is layout-only; add `class-variance-authority`. Toasts stay on `sonner` but get a token-based theme so the success
   toast passes 4.5:1 in both themes (the audit measured 4.26:1 on a light pair in dark mode).
   **`Modal.tsx` is replaced** by Dialog; `useSharedDirConfirmation` / `useOverwriteConfirmation` keep their `request()`
   shape over the new shell; every current `Modal` caller is switched (mechanical; no visual redesign of their content
   yet). `LoadingOverlay` stays until ticket 14 but is named (`aria-labelledby`) and its spinner honours reduce.
5. **Primitive tests** (D2): `src/components/ui/*.test.tsx`, `@testing-library/react`, jsdom — role/name, ARIA state
   from props, Esc closes only the top layer, focus returns to trigger or fallback. Add `vitest` env config if needed.
6. **Guard — `scripts/check-styles.mjs`**, `npm run styles:check`, wired into `npm run check` and the CI `web` job,
   baseline `scripts/styles-baseline.json` (`appCssCeiling` set after the alias block; `migrated: []`). The five
   failure rules from ADR §6 (growth, legacy class in a migrated file, new stylesheet, prose leak, raw colour in JSX).
7. **Pre-paint theme** (D6): inline script in `index.html` reading the persisted theme key (the literal from
   `preferences.ts`; a test pins the two literals equal) and setting `data-theme` + `color-scheme` before first paint.
8. **Min window**: `tauri.conf.json` `minWidth: 960, minHeight: 640`.
9. **`DESIGN.md`** (repo root): token table with values and measured ratios, every primitive's variant/tone/size,
   focus and motion rules, the target floor, the prose exemption, how to add a variant. AGENTS.md § Invariants already
   names it — make the pointer true.
10. **AGENTS.md**: the styling bullet's "(round-19 world ticket lands …)" parenthesis is removed; "Confirmations go
    through the Modal shell" → "through the Dialog primitive"; Commands gains `npm run styles:check`.

## Gate

`npm run version:check && npm run check` green (includes the new guard). Fixture walk in `preview_*`, both themes,
1280×800 and 960×640: the app renders in the new palette everywhere (aliases), no white flash for light users, every
dialog traps and returns focus, the contrast table holds on the rendered page (measure, do not assume).

## Review

Astra @ high on the tokens (ratios re-measured on the page), the Dialog contract (keyboard transcript), the guard's
rules, and that no surface file was restyled beyond the mechanical Modal swap.

## Result

Two children did the work. The first built items 1–8 and ran out of context while measuring contrast. The second
measured contrast on the rendered page, fixed what the measurements found, wrote `DESIGN.md` and the AGENTS.md edits,
walked the fixture and ran the gate. Nothing is committed.

### Files

New:

- `src/styles/theme.css`, `src/styles/prose.css`
- `src/components/ui/`:
  - primitives: `button`, `badge` (Badge + Chip), `field`, `checkbox`, `switch`, `tabs`, `segmented`, `dialog`,
    `confirm-dialog`, `popover`, `dropdown-menu`, `tooltip`, `sonner`;
  - internals: `layer-stack.ts`, `styles.ts`;
  - the dev-only `specimen.tsx`, mounted at `/?specimen` in fixture mode only and absent from `dist/`;
  - six `*.test.tsx`.
- `src/lib/utils.ts`, `src/lib/preferences.test.ts`, `components.json`
- `scripts/check-styles.mjs`, `scripts/styles-baseline.json`
- `DESIGN.md`

Changed:

- `src/index.css` (entry + base layer), `src/App.css` (alias block, plus 12 legacy literals swapped in place), `index.html`
  (pre-paint script)
- `src/main.tsx`, `src/App.tsx` (update dialog → Dialog; token Toaster)
- `src/hooks/useSettingsState.ts` (+ test; `resolvedTheme`)
- `src/components/skills/SkillDetailView.tsx` (class-output highlighter, no DOM theme read),
  `src/components/skills/LoadingOverlay.tsx`
- every former `Modal` caller in `components/skills/modals/`, `components/shared/` and `components/projects/`;
  `src/components/shared/Modal.tsx` is deleted
- `src-tauri/tauri.conf.json` (min 960 × 640)
- `package.json` / `package-lock.json` (`@base-ui/react`, `class-variance-authority`, `@testing-library/*`,
  `styles:check` in `check`), `tsconfig*.json` + `vite.config.ts` (`@/` alias), `.github/workflows/ci.yml`
  (`styles:check` in the web job)
- `AGENTS.md`: Commands, the styling bullet and the Confirmations bullet only

### Tokens

The semantic tier uses shadcn's names: `background foreground card popover primary muted accent destructive border
input ring sidebar-*`, with `-foreground` where it applies. The world extension is `raised border-strong faint ok stale
failed`. Prose-only tokens: `syntax-keyword/-string/-number/-function`, `elevation-sm/-lg`.

Non-colour tokens:

- type 12/13/14/16 with tabular numerals;
- radius 4/6/10;
- `spacing-row` 36;
- `motion-fast` 120 ms with `ease-out` `cubic-bezier(0.16,1,0.3,1)`;
- system UI stack + `ui-monospace`.

The Tailwind palette is removed (`--color-*: initial`).

Deviations from ADR §1 and the contract:

1. **`input` is a third line token.** It is an extension-by-role: `#6b717b` dark, `#7c828b` light, ≥ 3.32 on every
   ground. The contract's lines (`#262a31` / `#343942`) measure 1.1–1.7:1, which is fine for decorative hairlines
   but fails 1.4.11 for a boundary that identifies a control on its own (text field, checkbox, switch).
2. **`faint` is lifted one step** in both themes: dark `#858b95` → `#8c929c`, light `#646a74` → `#5b616a`. The alias
   block's legacy hover ground (`--bg-element-hover`) put the tertiary "(Disabled)" label of a hovered Tool pill at
   4.20 dark and 4.08 light. It is now 4.60 and 4.68.
3. **The light state colours are darkened** relative to the prototype's dot colours, so one token passes as both
   text and mark: ok `#127440`, stale `#8a5d00`, failed `#c22e2e`.

### Primitives and props

All are in DESIGN.md § Primitives.

- **Button**: `variant` solid/outline/ghost · `tone` neutral/accent/ok/stale/danger · `size` sm/md/lg/icon/icon-sm ·
  `pending`.
- **Badge**: `variant` soft/outline/solid · `tone` neutral/accent/ok/stale/failed · `size` sm/md.
- **Chip**: `aria-pressed`, 24 px.
- **Field**: label, Input/Textarea/Select (`size` sm/md), description, error, fieldset/legend, all ARIA-wired.
- **Checkbox**: 16 px box, 24 px hit area.
- **Switch**: `size` sm/md; "on" is neutral.
- **Tabs**: `variant` line/segmented. **Segmented** is a toggle group.
- **Dialog**: `size` sm/md/lg/xl; `DialogBody` `variant` default/flush; `dismissible`, `fallbackFocus`,
  `initialFocus` / `finalFocus`; top-layer-only Escape through `layer-stack.ts`; inert background; 32 px named Close.
- **ConfirmDialog**: `tone` accent/danger, `pending`.
- **Popover**, **DropdownMenu** (item `variant` default/destructive), **Tooltip**.
- **Toaster**: sonner themed entirely by tokens.

Tests: 33 across the six `ui/*.test.tsx` files and `preferences.test.ts`. The whole suite is 520 passing.

### Guard

The five ADR §6 rules:

1. App.css ratchet (non-blank, non-comment lines = `appCssCeiling`);
2. migrated files and `components/ui/` use no legacy class;
3. no stylesheet beyond the permanent three and App.css;
4. `prose.css` stays scoped under `.prose` with no colour literal;
5. no arbitrary colour or palette utility in `src/**/*.{ts,tsx}`.

Baseline: `appCssCeiling: 3038`, `migrated: []`. `rawColourExempt` holds `Layout.tsx` and `Dashboard.tsx`
(dormant; disposition ticket 25). The in-place literal swaps kept the count at 3038.

### Measured contrast

Method: WCAG 2.x luminance with ancestor compositing through a canvas; transitions disabled; every `:hover` rule
forced on for a second pass. Both themes. The pages measured:

- `/?specimen`;
- `?scenario=rich`: My Skills, detail + prose, Explore, Projects + matrix, Settings, and the Add, Change source,
  Configure Tools, Notifications, Remove, Review & Import, Invocation and shared-dir dialogs, plus the success toast;
- `?scenario=failures`: My Skills, Explore, Projects.

The final pass has no text pair under 4.5 and no non-text pair under 3, except the exempt decorative `·` in the detail
header (see Deferred).

The regression list:

| Pair | Round 18 | Dark now | Light now |
|---|---|---|---|
| Light tertiary | 2.56 | 6.12 | 6.09 |
| Synced pill | 3.67 | 10.06 | 5.30 |
| Failed pill | 4.41 | 6.39 | 4.94 |
| +N | 2.33 | 5.22 | 5.37 |
| Install | 2.14 dark | 7.05 | 5.71 |
| Installed | 1.82 / 3.58 | 9.72 | 5.09 |
| Success toast | 4.26 | 8.89 | 5.22 |
| Placeholder | 3.36 | 5.73 | 5.67 |
| Match / Conflict | 3.07 / 4.41 | 9.41 / 6.39 | 5.03 / 4.94 |

The hardest passing pairs:

- `faint` on the legacy hover ground: 4.60 dark / 4.68 light;
- `stale-foreground` on `stale/90`: 4.67 light;
- `primary-foreground` on `primary`: 4.68 dark;
- `primary` as a non-text fill on `raised`: 3.49 dark;
- `input` on `raised`: 3.32 / 3.33.

The focus ring measures ≥ 5.04 on every ground. The full table is in DESIGN.md.

The measurements forced three fixes, none of them a call-site override:

- **The UA placeholder colour** measured 3.33 light and 4.49 dark on the search field. The base layer's
  `::placeholder` now uses `faint`.
- **The highlighter's `<code>` tag** was drawn black at 1.17:1 on dark prose: react-syntax-highlighter builds its
  inline style from the default light Prism theme even with `useInlineStyles={false}`. It now gets
  `style={NO_INLINE_THEME}`.
- **Legacy colour literals.** Twelve literals in App.css (legacy blue focus and button shadows, the blue/purple/orange
  brand gradient, `#60a5fa` in the progress shimmer, red/amber tints) now read aliases or tokens, so no legacy blue
  survives. The dead `.modal-backdrop` rule and a neutral knob shadow are left as they are.

### Pre-paint theme

- `preferences.test.ts` pins the inline script's `getItem` literal equal to `themePreference.key` (`skills-theme`).
- The script runs in `<head>`. In the build it comes before the module script and the stylesheet (`dist/index.html`).
- Run in isolation, it resolves: light pref + dark system → light; dark pref → dark; no or unknown pref → system.
- Observed on a reload of `/?specimen`, which mounts no settings hook, with system appearance emulated dark and
  `skills-theme=light` persisted: `data-theme=light` and `color-scheme: light`, `body` `#fcfcfd`. There is no dark
  flash for a light user.

### Fixture walk transcript

Run at 1280 × 800 and 960 × 640, both themes.

- **Palette.** All three surfaces render in the new palette through the aliases. A detector for legacy-blue and
  white-in-dark found nothing on My Skills, Explore or Projects + matrix. Screenshots were taken at 960 × 640 in dark
  (My Skills) and light (Projects).
- **Overflow.** No horizontal overflow at 960.
- **Opening dialogs.** Add skill, Change source, Configure Tools, Notifications, Review & Import, Invocation and
  Remove each open as `role=dialog`, named through `aria-labelledby`. Focus moves inside:
  - Add → "Local Folder";
  - Remove → "Cancel" (danger);
  - Invocation → the first radio.

  `#root` becomes `inert`, and `.focus()` on an app button behind the dialog is refused. The `data-base-ui-focus-guard`
  pair flanks the popup.
- **Closing dialogs.** Escape closes the dialog, and focus returns to the trigger in all seven.
- **Nested dialogs.** Add skill → toggling Amp raises the shared-dir ConfirmDialog over it. The lower popup gets
  `inert` and focus moves to "Continue".
  - Escape #1 closes **only** the confirm. Add stays open and un-inerted, and focus returns to the element that was
    focused in Add when the confirm opened.
  - Escape #2 closes Add, and focus returns to "Add skill". `#root` is no longer inert.
- **Not exercisable here.** A real Tab keypress in the hidden preview tab: the window never has focus, so focus events
  do not fire and the guards cannot redirect. Tab-wrap is covered by Base UI's modal guards plus `inert`. Verify it
  in the operator's visible pane (D31 smoke).

### Deferred

- **Ticket 11**: the breakpoint number (DESIGN.md § Layout placeholder); the first `migrated` entries and ceiling cut.
- **Ticket 15 (Emil)**: motion tuning of primitives, and the legacy `translateY(-1px)` hover lifts on `.btn-primary` /
  `.btn-danger-solid`.
- **Ticket 12**: the decorative `·` (`.detail-meta-dot`, 1.65 / 1.49) goes with the detail migration.
- **Ticket 16**: Add's Tool toggles use `display:none` inputs (not focusable).
- **Ticket 17**: the Projects header is cramped at 960.
- **Ticket 20**: the header settings icon button has no accessible name.
- **Ticket 19 (AGENTS.md invariants)**: the "Frontend tests … no component rendering tests" bullet should name the
  D2 exception (`src/components/ui/*.test.tsx`). It was left alone because this pass was limited to the
  styling / Modal / Commands lines.

### Gate

- Pass: `version:check`, `lint`, `styles:check`, `test` (520), `build`, `rust:clippy`, and `cargo test --all` (740;
  `src/bindings/index.ts` unchanged).
- Fail: `rust:fmt:check`, **only** in `src-tauri/src/core/tests/skill_disable.rs` (3 hunks) and
  `src-tauri/src/core/tests/target_relocation.rs` (1 hunk). Both files belong to the concurrent ticket-01 child.
