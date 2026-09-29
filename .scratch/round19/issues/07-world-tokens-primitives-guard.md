# 07 — World ticket: `theme.css`, `prose.css`, primitives on Base UI, `styles:check`, `DESIGN.md`, pre-paint theme, min window

Status: ready-for-agent
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

(child fills in)
