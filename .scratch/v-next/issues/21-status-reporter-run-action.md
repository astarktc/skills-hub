# 21: Give `StatusReporter` the action lifecycle (`runAction`)

Status: resolved

Type: task
Blocked by: None (can start immediately)

## What to build

`src/hooks/useStatusReporter.ts` (interface at `:19-38` at `943f85c`) exposes five raw setters — `setLoading`, `setLoadingStartAt`, `setActionMessage`, `setError`, `setSuccessToastMessage` — and nothing that owns the lifecycle. So the choreography is copied at every call site: begin (`setLoading(true); setLoadingStartAt(Date.now()); setError(null); setActionMessage(...)`) → try → `setError(formatError(e))` → `finally { setLoading(false); setLoadingStartAt(null) }`, toast on success. Verified: `setLoadingStartAt(Date.now())` appears 13× (7 `useSkillLibrary`, 5 `useAddSkillFlow`, 1 `useExploreState`); `useAddSkillFlow.ts::handleCreateGit` (`:499-628`) must hand-unwind loading on its early-return-to-picker paths — the exact place a missed reset hides. The reporter is shallow: behaviour per unit of interface learned is near zero, and the invariant lives in N copies.

Deepen:

- Add one deep method, e.g. `runAction<T>(opts: { message?: string; successToast?: string }, fn: () => Promise<T>): Promise<T | undefined>` that owns begin/success/error/finally (including `formatError` → `setError`, and the `null` = silent-cancel contract). Consider an explicit way for a flow to "hand off" without toasting/erroring (the picker early-return case) — design it, don't hack it.
- Migrate all 13+ call sites in the three hooks onto it. Demote the raw setters: keep only what a genuinely divergent flow still needs, and document why.
- Tests: the lifecycle invariant is tested once, through the reporter's own interface in `useStatusReporter.test.ts` (loading always resets, error cleared at start, success toast, error path, cancel path). The other hook tests keep passing (adjust only where they asserted the raw choreography).
- Frontend-only; no backend or i18n changes expected. Plain `useState`, no state library (AGENTS.md).

## Acceptance criteria

- [ ] `runAction` exists and owns the full lifecycle; every action handler in `useSkillLibrary`, `useAddSkillFlow`, `useExploreState` uses it (no remaining copy of the begin/finally choreography).
- [ ] Reporter tests cover the invariant through `runAction`; full vitest suite green.
- [ ] `npm run version:check && npm run check` green.

## Answer

Landed green in `c1b513c` (rebased over `8550081`) (Fable 5.1 child, medium thinking; orchestrator-verified; 62 vitest + 248 cargo tests, full gate green on main).

- `runAction<T>(opts, fn)` on `StatusReporter` owns the whole lifecycle: loading on + `loadingStartAt` + stale error cleared + `opts.message` → body → success toast (`string | (value: T) => string`, the function form for `handleUnsyncAll`'s value-dependent copy) / `formatError` → `setError` on throw (null = silent cancel, unchanged) → `finally` resets `loading`, `loadingStartAt`, `actionMessage` always.
- **Hand-off design**: the body receives an `ActionHandle` with two exits that are *returned, never thrown* — `return action.handOff()` (another surface takes over, e.g. candidate picker: no toast, no error, loading still resets) and `return action.fail(message | null)`. Both mint an `ActionExit` (private ctor, static factories); `runAction` dispatches on `instanceof`. Control flow stays visible and type-checked (`Promise<T | ActionExit>`); no control-flow exceptions, no forgettable flag.
- **Demoted**: `setLoading` / `setLoadingStartAt` removed from the interface entirely — the "stuck overlay" bug class is structurally gone. Kept with documented reasons: `setActionMessage` (progress inside an action; sync `Channel`), `setError` (validation before an action; non-overlay hooks; non-fatal warnings), `setSuccessToastMessage` (non-action successes in settings).
- All 13 sites migrated (`useSkillLibrary` 7, `useAddSkillFlow` 5, `useExploreState` 1); zero `setLoadingStartAt(` outside the reporter. `useAddSkillFlow`: picker early-returns → `openGitPicker`/`openLocalPicker` + `handOff()`; `handleCreateGit`'s duplicated install blocks collapsed; `loadPlan` split into `fetchPlan` + `runAction` so `handleImport` no longer nests two lifecycles.
- Tests: invariant tested once through `runAction` in `useStatusReporter.test.ts` (success/toast, function toast, silent, throw, CANCELLED, handOff, fail, fail(null), stale error cleared); hook tests use a small `runAction` stub routing to the same spies; picker tests assert no toast/no error on hand-off.
- Behaviour notes: per-item `showActionErrors` now fires before the success toast; `handleSyncSkillToAllTools`/`syncAllManagedToTools` previously had no `catch` (unhandled rejection) and now toast via `formatError`; a `fetchPlan` failure right after import now surfaces as an error toast instead of being swallowed.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
