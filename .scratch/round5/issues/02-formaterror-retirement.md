# 02: `formatError` leaves the component surface

**What to build:** `ProjectsPage`, `SkillDetailView` and `SettingsPage` no longer receive a `formatError` prop; they call `describeCommandError(err, t)` with the `t` they already hold from `useTranslation`. `App.tsx` stops passing it. Hooks that lack `t` (`useSyncOrchestration`, `useExploreState`, `useSkillLibrary`) keep `formatError` from the reporter as an internal seam — unchanged. AGENTS.md's `skillPresentation` bullet ends with a sharpened sentence: components import pure functions (presentation, `describeCommandError`); the props App passes carry state — `notify`, `runAction`, data, actions — never a function that is only an import with an argument pre-bound. Behaviour is identical.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] No component prop type names `formatError`; `rg formatError src/components src/App.tsx` is empty
- [x] Existing hook tests unchanged and green; `npm run build` type-checks
- [x] AGENTS.md sentence replaced (one clause; do not touch other sections)
- [x] `npm run version:check && npm run check` green

## Orchestrator notes

- Origin of the rule: `e094456` documented arch-deepening ticket 04 (`b664a60`), which removed three *pure* formatter props. Round-3 ticket 12 #9 then directed passing `formatError` — the two collided. Deletion test: `formatError` is `describeCommandError` with `t` pre-bound; components already hold `t`.
- Sites: `ProjectsPage.tsx:25,31,135,149`; `SkillDetailView.tsx:43,425,482,492`; `SettingsPage` (see `App.tsx:257/327/334`). `describeCommandError` returns `string | null` (null = silent cancel) — preserve the `?? …` fallbacks.
- Sibling ownership: ticket 01 adds UI in `SkillCard`/`useSkillLibrary`; ticket 03 edits `skillPresentation.ts`/`resources.ts`. Don't reformat those files.

## Comments

- Shipped in `628b85d` (`refactor(ui): retire formatError component props`): all three components import `describeCommandError`, preserve null fallbacks, and track `t` in callback/effect dependencies; App no longer destructures or passes the formatter. Only the specified AGENTS.md sentence changed. Hook implementations and tests are unchanged.
- Deviation: SettingsPage and SkillDetailView already receive `t` as a prop, rather than obtaining it from `useTranslation` as the ticket assumed. Kept that existing wiring and used that `t`, avoiding an unrelated translation-prop migration.
- Gate: `npm run version:check && npm run check` passed (version 1.2.4; lint; 13 Vitest files / 214 tests; typescript-7 + Vite build; rustfmt; clippy with `-D warnings`; 517 Rust tests). Vite reports its non-blocking large-chunk warning. `npm ci` reported 7 dependency vulnerabilities (1 moderate, 6 high); no dependency changes were made.
- Acceptance evidence: `rg formatError src/components src/App.tsx` has no matches; four edited TSX files have zero primary LSP diagnostics; `git diff --exit-code -- src/hooks src/bindings/index.ts` and `git diff --check` passed. Full gate output: `.scratch/round5/02-gate.log` (local, not committed).
- Follow-ups: none for this ticket; dependency audit and bundle sizing are outside its scope.
