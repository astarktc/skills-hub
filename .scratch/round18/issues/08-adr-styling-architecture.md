# 08 — ADR 0006: styling architecture (D3)

Status: ready-for-agent
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
