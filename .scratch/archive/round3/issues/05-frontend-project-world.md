# 05: Project world — no side effects in state updaters; failure-path refresh after project removal

Status: done — 0868172

**What to build:** Selecting a Project derives its tools, assignments and reconciled flag without doing so inside a `setState` functional updater (which double-fires under StrictMode) — an effect keyed on the selection, or a plain sequence in the select action. When removing a Project fails with `DELETE_CLEANUP_FAILED`, the page refetches the Project view on the failure path exactly as toggle, bulk and configure already do, so the `error` assignment rows ADR-0002 promises are visible immediately rather than after a reselect.

Source: `../spec.md` Q12; `../source-13-review-followups.md` #7, #8; AGENTS.md "Ambiguity resolution" on mutation returns. Skill: **tdd** (hook-level vitest only; no JSX tests).

**Blocked by:** None (can start immediately)

- [x] Hook test: selecting a project under a doubled updater invocation applies the view once
- [x] Hook test: a project removal rejected with `DELETE_CLEANUP_FAILED` triggers a Project-view refetch for the selected project
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: done (awaiting merge) → done — 0868172. Evidence: Cited 0868172 integrated on main as 7ec55fc; pure selection followed in 2edaca8. The old awaiting-merge claim is obsolete (rebased/fast-forward integration, no separate merge commit); src/components/projects/useProjectState.ts:274 refreshes the failed removal.
- 2026-09-15 — Previous status wording (historical, not a current merge/publication claim): done (awaiting merge)

### 2026-09-04 — implementation (branch `r3/05-project-world`)

Shipped:

- `0868172` fix(projects): refetch the Project view when project removal fails — `removeProject` in
  `useProjectState.ts` now catches the command failure, `await refreshView(id)`, rethrows; same shape
  as toggle / bulk-assign / configure. Test: `converges on the backend's view when project removal
  fails` (rejects `DELETE_CLEANUP_FAILED`, asserts `["removeProject", "getProjectView"]`, the project
  still selected, its rows `error`, its row `sync_status: "error"`). Red first, then green.
- `f9b57f6` refactor(projects): keep the selection and its matrix in one pure state value — the four
  states `selectedProjectId` / `tools` / `assignments` / `assignmentsReconciled` become one
  `Selection` value; `applyView` and `removeProject` decide "is this view for the selected project?"
  in a pure updater on that value. The `ProjectState` surface is unchanged (fields destructured from
  the one value), so `ProjectsPage` / `AssignmentMatrix` are untouched. Tests: `applies a late
  mutation result for a deselected project to its row only` (the race the old updater trick was
  protecting — a naive closure-based refactor would fail it) and `derives the matrix once per view
  under StrictMode's doubled updaters` (renderHook with `wrapper: StrictMode`).

Deviations:

- The failure-path refresh lives in the **hook**, not `ProjectsPage.tsx` (orchestrator note allowed
  this; AGENTS.md prefers logic in hooks; the page's `handleRemoveProject` catch/toast is unchanged).
  Consequently it refreshes on **any** removal failure, exactly like toggle/bulk/configure — not only
  `DELETE_CLEANUP_FAILED` — and for the removed project's id whether or not it is the selected one
  (an unselected project only gets its list row updated, which is where its aggregate `error` shows).
- Ticket offered "effect keyed on the selection, or a plain sequence". Neither can decide which view to
  apply when a mutation result lands after the operator navigated away (it needs the *latest*
  selection, not the closure's), so the choice was a single state value (same reasoning as
  `ProjectDialog` in the same file: two states that must agree become one). Satisfies Q12's actual
  rule — no side effects in updaters.
- Honest note on the StrictMode test: the old code's doubled side effects were idempotent
  (`setTools(view.tools)` twice), so this test was not red against HEAD; it pins the contract under
  StrictMode so a non-idempotent regression is caught. The race test is the one that guards the refactor.

For the orchestrator:

- `cargo test` regenerated nothing (no Rust touched, bindings clean).
- One unrelated timing-based Rust test (`refresh::acquisitions_overlap_instead_of_running_one_at_a_time`,
  asserts wall-clock < 60% of sequential) failed once under parallel-worktree CPU load and passed on
  re-run (0.42 s vs 0.72 s budget). Not touched; worth knowing when several gates run at once.
- Pre-existing Prettier drift in both files is outside my hunks and was left alone (Prettier is not in the gate).

