# 32: Import completion must not depend on the post-import plan reload

Status: resolved

Type: task
Source: review #3 S2 (Fable; verified) — [verdict](../assets/review-3/verdict.md)

## What to build

Edge regression from ticket 21. `handleImport` (`src/hooks/useAddSkillFlow.ts:384-390`) now runs `await loadManagedSkills(); await fetchPlan();` **inside** the `runAction` body. `fetchPlan` throws (it no longer has its own try/catch), so a plan-reload failure *after every selected skill was successfully imported* is caught by `runAction` → error toast, the `status.importCompleted` success toast is suppressed, and `setShowImportModal(false)` is skipped — the modal stays open over a completed import.

At `943f85c` the equivalent `loadPlan()` swallowed the failure (`setError` + `return null`, `:153-178`) after the success toast had already fired and the modal closed. The ticket 21 Answer disclosed only "a `fetchPlan` failure right after import now surfaces as an error toast", not the modal/toast consequence.

Fix options (pick the narrower):
1. Split the action: the import loop is the `runAction` body (success toast + close modal on return); the refresh (`loadManagedSkills` + `fetchPlan`) runs after the action, with its own error surfacing via the reporter (`setError`), not by failing the completed action.
2. Wrap the refresh inside the body and convert its failure to a non-fatal collected error shown by `showActionErrors` alongside sync failures.

Check `handleCreateLocal` / git install paths for the same shape (post-install refresh inside the action body) and treat them consistently.

## Acceptance criteria

- [ ] `useAddSkillFlow.test.ts` case: all imports succeed, `getOnboardingPlan` rejects on the reload → success toast shown, modal closed, error surfaced separately.
- [ ] Sync-failure path (`collectedErrors`) behaviour unchanged (existing tests).
- [ ] `npm run version:check && npm run check` green.

## Answer

**Option 2, narrowed** — the refresh stays inside the `runAction` body but can no longer throw; its failure surfaces through the reporter's one-shot `setError` channel rather than as a collected error.

A new `refreshWithoutFailingAction(refresh)` helper in `useAddSkillFlow.ts` wraps the refresh in a try/catch and reports failure via `setError(formatError(err))`. `handleImport` now calls it around `loadManagedSkills()` + `fetchPlan()`; `finishInstall` (the local + git install tail) calls it around `loadManagedSkills()`, so every post-install refresh is non-fatal.

Why not option 1 (refresh after the action): it would force `handleImport`'s body to return a completion sentinel just so the caller could tell completion from `handOff`/`fail`/throw, widening the action's return contract for no user-visible gain. Why not literal option 2 (collected error): `collectedErrors` is exactly what keeps the modal open (`collectedErrors.length > 0` → `showActionErrors`, no close), so feeding a plan-reload failure into it would reproduce the reported bug. `setError` is documented on `StatusReporter` as the channel for "non-fatal warnings inside one" action — a precise fit. Sync-failure semantics are untouched.

Consistency check: the git/local paths shared the shape (post-install refresh inside the body) but not the full symptom — `finishInstall` closes the add modal *before* refreshing, so only the success toast was at risk. Both now share the one helper.

Tests: `src/hooks/useAddSkillFlow.test.ts` gains a `useAddSkillFlow import flow` describe with 2 cases — (a) all imports succeed + `getOnboardingPlan` rejects on the reload → `setSuccessToastMessage("status.importCompleted")`, `showImportModal === false`, `setError("plan reload boom")`, `showActionErrors` not called; (b) a `TARGET_EXISTS` sync failure → `showActionErrors` with the collected entry, modal stays open, `setError` not called (regression guard for the unchanged path). Verified case (a) fails on the pre-fix code.

Counts: `useAddSkillFlow.test.ts` 11 → 13; suite 89 → 91 frontend tests (8 files); Rust 317 passed. `npm run version:check` (1.1.9) and `npm run check` green; `src/bindings/index.ts` unchanged (no Rust touched).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
