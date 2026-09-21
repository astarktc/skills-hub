# 04 — Review fixes: project unassign toggle returns its RemovalReport; carve-out deleted; two nits

Status: done — pending
Blocked by: 01, 02, 03
Source: Opus 5 adversarial review of `8cfe657..b42dccd` (verdict fix-then-ship) — Standards #1 + Spec #1 (both
`should`), Standards #2 and #4 (nits). Review text is in the parent thread; the findings are restated here in full.

## Findings being fixed

- **Standards #1 (should).** `project_sync::unassign_and_remove_artifacts` (`core/project_sync.rs:683–687`)
  re-raises a settled row's `CommandError` inside an `anyhow::Error`. `CommandError::Display` is JSON, so any future
  `.context()` on that chain rendered with `{:#}` would persist `{"code":…}` into a `last_error`/log. It is the
  **only** producer of a `CommandError`-in-anyhow, and the only reason `CommandError::from_anyhow` has the
  "downcast `CommandError` first" carve-out (`core/errors.rs:366–371`).
- **Spec #1 (should).** Toggling a project assignment off with a stuck artifact used to show localized copy
  (`errors.deleteCleanupFailed`) and now shows the raw `OTHER` chain through `ProjectsPage.handleToggleAssignment`'s
  bare `notifyError(err)` (`src/components/projects/ProjectsPage.tsx:144–153`). D6 only sanctioned delete's change.
- **Standards #2 (nit).** `commands/mod.rs:54–55` blanket `#[allow(unused_imports)]` on the
  `CommandError, GitCloneFailureKind` re-export; only `GitCloneFailureKind` (tests-only) needs it.
- **Standards #4 (nit).** `InvocationEditReport` is still defined in `commands/mod.rs:905–910`; the ADR amendment
  says core report types cross as themselves — move it to `core/skill_edits.rs`.

## The fix (the reviewer's "fuller fix", chosen over the frontend-only patch because it removes the root cause)

1. **Core.** `unassign_and_remove_artifacts` returns `Result<RemovalReport>` — it no longer inspects the report or
   raises anything for a kept target (store/plan failures still `?`-propagate as `anyhow`; unknown tool still bails
   `SignalError::UnknownTool`). `toggle_skill_assignment` returns a richer outcome:
   `ToggleOutcome::Assigned` | `ToggleOutcome::Unassigned { report: RemovalReport }` (or an equivalent struct —
   keep the `PartialEq`-based test assertions compiling with the least churn; a kept target is **not** a command
   failure, it is report data, mirroring `unsync_skill_from_tool`).
2. **`from_anyhow` carve-out deleted.** Remove the `downcast::<CommandError>()` block in `core/errors.rs` and its
   test `commands/tests/commands.rs` (`settled_report_error_retains_its_classification_when_single_target_command_fails`
   — delete or repurpose). Update the ADR-0001 amendment paragraph in
   `docs/adr/0001-tagged-command-error-contract.md` that mentions the carve-out ("`from_anyhow` also recovers an
   already-classified `CommandError` …") — delete that parenthetical. `impl std::error::Error for CommandError` may
   stay if anything else needs it; otherwise remove it so the type *cannot* enter an anyhow chain again (preferred —
   that is the compiler-enforced version of the reviewer's concern).
3. **Command.** `ToggleAssignmentResultDto { view, assigned, report: Option<RemovalReport> }` — `report` is `Some`
   on the unassign direction, `None` on assign. Wire-accurate `T | null` (never `?`).
4. **Frontend.** `useProjectState.toggleAssignment` resolves to the `RemovalReport | null` (it keeps applying the
   returned `view`; the failure-path `refreshView` stays for thrown errors). `ProjectsPage.handleToggleAssignment`
   folds a non-null report with `removalOutcome(report, { t, toolLabelById, action: "toggle" })` through the page's
   existing `applyOutcome` (which routes `errors` to `showActionErrors` and the toast to `notify`) — the same fold
   and copy the global unsync toggle uses (`errors.unsyncFailedTitle {tool}`, `status.syncDisabled`). Check
   `removalOutcome`'s `toggle` branch semantics fit (its `reload` flag is irrelevant to the projects world — the
   view was applied; do not add a refetch). Thrown errors keep `notifyError(err)`.
5. **Tests.** Core: the stuck-artifact test (`core/tests/project_sync.rs:~290–330`) asserts the returned report
   (`failed_rows() == 1`, the row kept with status `error`, `CommandError::Other { message }` naming the path)
   instead of `expect_err`. Frontend: `useProjectState.test.ts` — toggle resolves with the report and applies the
   view; `reportOutcome.test.ts` already covers `removalOutcome` toggle. Add one hook-level test that a kept target
   on toggle-off surfaces `errors.unsyncFailedTitle` via `showActionErrors` if a page-level seam exists to test it
   at hook level; otherwise the fold test suffices (no component tests — project rule).
6. **Nits.** Split the re-export: `pub use crate::core::errors::CommandError;` plain, and
   `#[cfg(test)] pub(crate) use crate::core::errors::GitCloneFailureKind;` (or `cfg_attr(not(test), allow(...))`).
   Move `InvocationEditReport` to `core/skill_edits.rs` (derive `Serialize + Type` there; command imports it;
   `InvocationEditResultDto::from_outcome` builds it or `skill_edits` gains a small constructor from
   `InvocationEditOutcome`).
7. **Bindings.** `cd src-tauri && cargo test --all` regenerates `src/bindings/index.ts`; commit-ready diff should
   show `ToggleAssignmentResultDto` gaining `report: RemovalReport | null` and nothing else structural.
8. **CHANGELOG `[Unreleased]`** — you MAY edit it for this ticket only: add under **Changed**: "Turning a project
   assignment off whose deployment cannot be removed now reports the kept target the same way unsync does."

## Gate

`npm run version:check && npm run check` green (lint, vitest, build with typescript-7, `cargo fmt --check`,
clippy `-D warnings`, `cargo test --all`); `git status` clean after `cargo test` apart from your edits;
`grep -rn "downcast::<CommandError>\|anyhow::Error::new(error)" src-tauri/src` empty.

## Constraints

- State your approach and continue — no need to wait for confirmation.
- Never commit, never `git stash`. `.scratch/` is tracked: set this ticket `claimed`, append a dated `## Comments`
  entry when done; never archive or `git mv` under `.scratch/`.
- Targeted edits over rewrites; re-read after writing. No refactors beyond the items above.
- Report: files touched, the bindings diff, test counts before/after, and anything here that did not survive contact
  with the code.

## Comments

### 2026-09-21 — implementation complete; awaiting parent review

- Toggle-off returns its settled RemovalReport through core, command and hook; the page uses the existing
  removalOutcome toggle fold and applyOutcome, with no success-path refetch. The stuck-artifact Rust test now
  exercises the public toggle and pins the failed report, kept error row and path. Hook coverage pins report
  return and settled-view application; existing fold coverage pins errors.unsyncFailedTitle (no page hook seam).
- Removed the CommandError downcast carve-out and std::error::Error implementation; nothing else needs it.
  Split the re-export and moved InvocationEditReport into core/skill_edits.rs. Only the allowed ADR parenthetical
  and changelog line changed. Bindings add report: RemovalReport | null plus the corrected assigned doc comment.
- Fresh baseline: cargo 648, vitest 361. After: cargo 647 (obsolete carve-out test removed), vitest 362.
  npm run version:check && npm run check passed; cargo test --all passed twice, with identical bindings SHA-256
  on the repeat run. git diff --check clean. No commits or stash; only ticket-scoped edits on main.
- Necessary shape adjustment: ToggleOutcome lost Copy/Clone/PartialEq/Eq and the command DTO lost Clone because
  RemovalReport does not implement them; two equality assertions became pattern/report assertions.
- Literal grep gate has one pre-existing false positive: src-tauri/src/core/manifest.rs:460 wraps std::io::Error
  using anyhow::Error::new(error), not CommandError. Verified identical in HEAD; left untouched to respect scope.
  No CommandError downcast or CommandError-in-anyhow producer remains.
