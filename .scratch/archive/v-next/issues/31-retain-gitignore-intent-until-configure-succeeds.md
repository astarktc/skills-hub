# 31: Retain the gitignore intent until `configureProjectTools` succeeds

Status: resolved

Type: task
Source: review #3 S1 (3/3 reviewers; verified) — [verdict](../assets/review-3/verdict.md)

## What to build

Regression from ticket 28 (minor, user-visible). `useProjectState.configureTools` (`src/components/projects/useProjectState.ts:355-360`) reads `pendingIgnore` and calls `setPendingIgnore(null)` **before** awaiting `invokeTauri("configureProjectTools", …)`. Ticket 28 deliberately keeps the tool-config modal open on failure (`ProjectsPage.tsx:45-51`: `setShowToolConfigModal(false)` runs only after the awaited call). So on any failure — tools-side or ignore-side — pressing Confirm again re-enters `configureTools` with `gitignore: null`: tools are re-persisted, the `.gitignore` block is never written, and nothing tells the user.

Base (`943f85c` `ProjectsPage.tsx:73-83`) also cleared the ref early, but closed the modal and downgraded to `toast.warning`, so no misleading retry affordance existed.

Fix: clear `pendingIgnore` only after the command resolves successfully (keep it on rejection so the retry replays the intent). Consider also clearing it when the modal is dismissed without confirming, so a stale intent can't leak into a later edit of the same project — decide and record.

## Acceptance criteria

- [ ] `useProjectState.test.ts` gains a case: `registerProject` with ignore options → `configureTools` rejects → second `configureTools` call passes the **same** `gitignore` options to the mocked `invokeTauri`.
- [ ] Existing 7 `useProjectState` tests and the T28 convergence test still pass.
- [ ] Decide + document (in this ticket's Answer) whether cancelling the modal discards the intent.
- [ ] `npm run version:check && npm run check` green.

## Answer

Fixed in worktree `~/.worktrees/skills-hub-t31`, branch `t31`, commit `fa5d003`.

- `src/components/projects/useProjectState.ts` — `configureTools` no longer clears the intent before
  the call. `setPendingIgnore(null)` now runs immediately after `invokeTauri("configureProjectTools", …)`
  resolves, inside the `try`. On rejection the intent survives, so the retry the still-open modal invites
  replays the same `gitignore` options. The catch/finally convergence behaviour from T28 is unchanged.
- **Modal cancel: yes, discard.** Dismissing the tool-config modal abandons the registration flow that
  captured the intent, so a new `discardPendingIgnore()` action (added to the `ProjectState` type and the
  returned surface) clears it, and `ProjectsPage.tsx`'s `ToolConfigModal onRequestClose` calls it before
  closing. Reasoning: the intent is scoped to that one add-project flow. Without this, an operator who
  cancels and later edits the same project's tools from the toolbar would get an unexpected `.gitignore`
  write they never re-confirmed in that session — the "stale intent leak" the ticket flags. Clearing on
  cancel costs nothing (re-registering or the Edit dialog's own gitignore toggles remain the way to ask
  for it), while keeping it risks a surprise filesystem write. The existing "consumed once" and
  "dropped by another registration" guards stay as they were.
- Tests: `useProjectState.test.ts` 7 → **9** cases (retry-replays-intent; modal-dismiss-discards).
  Suite total 89 → **91** passed across 8 files. Rust: 317 passed; `src/bindings/index.ts` unchanged.
- Gate: `npm run version:check` (1.1.9 OK) and `npm run check` (lint, vitest, build, rust fmt/clippy/test)
  both green in the worktree.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
