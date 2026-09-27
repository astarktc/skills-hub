# 08 — ADR 0006: styling architecture (D3)

Status: implemented
Blocked by: 05
Spec: `.scratch/round18/spec.md` — D3, ticket 08.

## Work

- Read the `shadcn` skill (`.pi/skills/shadcn/SKILL.md`) and decide **Radix vs Base UI** under shadcn for our
  needs (Dialog, Popover, DropdownMenu, Tooltip, Checkbox, Switch, Tabs, Command; origin-aware popovers; focus
  management; React 19; bundle size in a Tauri webview). One line of justification with sources fetched.
- Write `docs/adr/0006-styling-architecture.md` (`domain-modeling` skill format, like 0001–0005): context (3,696-line
  `App.css`, locality and deletion-test failures from ticket 05's findings), decision (D3 verbatim: `@theme`
  tokens, utilities in JSX, `src/components/ui/` primitives as the deep module, shadcn behavioural primitives
  restyled, `App.css` → 0 with a CI guard), consequences, and the migration rule ("a surface is migrated when it
  imports nothing from `App.css`"). Name the CI guard (a script that fails when `App.css` grows or when a
  migrated surface references a legacy class).
- `AGENTS.md`: replace "Styles live in `src/App.css` / `src/index.css`. There are no CSS Modules — don't add the
  pattern" with the new rule and the ADR pointer; add `src/components/ui/` to the Invariants (new primitive =
  variants documented in `DESIGN.md`). `CONTEXT.md`: add *Primitive*, *Token*, *World* if absent.
- No code beyond docs in this ticket.

## Result

2026-09-25. Docs only, uncommitted. Delegated child.

### Deliverables

- `docs/adr/0006-styling-architecture.md` (new).
- `AGENTS.md`, two targeted edits:
  - Ambiguity resolution: the "Styles live in App.css" line is replaced by the ADR-0006 rule.
  - Invariants: a new **UI primitive** entry. Variants are documented in `DESIGN.md`, `className` is for layout
    only, and tokens live only in `theme.css`.
- `CONTEXT.md`, new terms: **World**, **Visual world**, **Theme**, **Design token**, **Primitive**.

### Decision in one line

We use Tailwind v4 `@theme` tokens (one file, shadcn semantic names plus six world extensions, dark first),
utilities in JSX, and a deep `src/components/ui/` primitives module on **shadcn over Base UI**. `App.css` ratchets
to zero under `scripts/check-styles.mjs` (`npm run styles:check`). `src/styles/prose.css` is the one exemption.

**Why Base UI.** It has been shadcn's default and recommended base since 2026-07 and is stable at 1.x (1.8.0). It
matches Radix on focus handling, anchored positioning and React 19. It fits D5/D6 better: focus can depend on the
interaction type, and exits use CSS transitions that can be interrupted. Radix's bundle is smaller by about
100 KB minified / 35 KB gzip, but that doesn't matter in a local webview. I measured this with esbuild; sources are
cited in the ADR.

### D3 corrections adopted (ticket 05 candidate 4)

1. **Tokens already exist.** The ADR fixes the token *set and shape* now:
   - Two tiers with no palette tier. The tokens use shadcn's semantic names, extended by `raised`,
     `border-strong`, `faint`, `ok`, `stale` and `failed`.
   - The file is `src/styles/theme.css`. It uses `:root` (dark) and `:root[data-theme='light']`, then
     `@theme inline`, then `@custom-variant dark` on `[data-theme]`.
   - The legacy names are aliased onto the new tokens inside `App.css` until it is deleted, so un-migrated
     surfaces switch palette the day the world ticket lands.
   - The *values* are deferred to the round-19 world ticket and `DESIGN.md`.
2. **`App.css` will not reach literal zero.** Markdown and syntax-highlighting styles move to
   `src/styles/prose.css`. The guard exempts that file, on two conditions: its selectors are scoped under prose
   roots, and it uses tokens only.
3. **Theme ownership.** The settings world stays the single writer of `data-theme` and exposes
   `resolvedTheme`. Styling reads the theme only through tokens, and reading `data-theme` from the DOM is forbidden.
   The syntax highlighter moves to class output with token colours, which removes the one current JS consumer
   (`SkillDetailView.tsx:389`).
4. **Migration rule, made checkable.** "Imports nothing from App.css" becomes "references no class `App.css`
   defines", because CSS is global and components never import it. The guard has five checks:
   - an `App.css` line ceiling that ratchets down;
   - `migrated` files that must not use legacy classes;
   - no new stylesheets;
   - prose containment;
   - no raw colours in `src/components/**`.

   The baseline lives in `scripts/styles-baseline.json`.

### Open questions for the ticket-09 grill

1. **Primitive tests.** The ADR *proposes* a narrow exception: `src/components/ui/*.test.tsx`, run with RTL on
   jsdom, covering only role and name, ARIA state from props, and Esc plus focus return. The AGENTS.md "no component
   tests" rule is unchanged until the grill accepts it. Contrast, focus visibility and motion stay with the audit
   lane (fixture app plus axe or impeccable `audit`).
2. **⌘K palette.** Should it use `cmdk`, which pulls a Radix Dialog into a Base UI app, or be composed from Base
   UI's `Autocomplete`/`Combobox` plus our Dialog?
3. **Token vocabulary.** Should tokens keep shadcn's semantic names (cheap restyle of copied primitives), or use
   the direction contract's own words (ground, panel, raised, line, iris)? The ADR picks shadcn names, and says
   what would reverse that.
4. **Theme flash on launch.** The theme is applied in an effect, and the Tauri window has its own background.
   Would a pre-paint read break "storage only through `preferences.ts`"?
5. **Toasts.** Keep `sonner`, which the ADR recommends, even though shadcn's Base UI toast is the skill's
   default?
6. **Dormant `Layout.tsx`/`Dashboard.tsx`.** Delete them when `App.css` goes, or leave them unstyled?
7. **Pending AGENTS.md follow-up (outside this ticket's edit scope).** "Confirmations go through the Modal shell"
   needs rewording to "the Dialog primitive" when `Modal` is replaced by the world ticket.
8. **Glossary collision.** "world" already means a *state area* (per-world hooks). CONTEXT.md now separates
   **World** from **Visual world**, and "token" from **Design token** vs the GitHub token (a credential). Spec and
   ticket prose that says "the world ticket" is fine. New docs should say "Visual world" when they mean the look.
