# 01 — Backend: core report types cross the wire as themselves

Status: ready-for-agent
Spec: `.scratch/round15/spec.md` — decisions D1–D7, rider R1. Read the spec first; this ticket is the backend half.

## Goal

Delete every `*ReportDto` mirror and mapper in `src-tauri/src/commands/mod.rs` / `commands/projects.rs`. The core
report type derives `Serialize + specta::Type` and is what the command returns. Per-item failures are typed
`CommandError` values classified where the row settles.

## Work

1. **D1 — move `CommandError` into core.** Everything in `src-tauri/src/commands/error.rs` (the enum,
   `GitCloneFailureKind`, `internal`, `from_anyhow`, `From<SignalError>`, `From<GlobalSyncError>`, `Display`)
   moves into `src-tauri/src/core/errors.rs` beside `SignalError`. Delete `commands/error.rs`; keep
   `pub use crate::core::errors::{CommandError, GitCloneFailureKind};` in `commands/mod.rs` so the seam and
   `commands/tests` read unchanged. The TS type name `CommandError` and every `code` tag stay byte-identical
   (verify with the bindings diff — the `CommandError` union must not change).
2. **D2 — per-item failures become `CommandError`.** Change the field type and convert at the one place each row
   is settled, *after* the `{:#}` chain has been written to the row's `last_error` / log line where that happens
   today:
   - `artifact_removal::RemovalTargetStatus::Failed { error }`
   - `propagation::PropagationStatus::Failed { error }`
   - `refresh::SkillRefreshStatus::Failed { error }` and `Refreshed { reassert_error: Option<_> }`
   - `global_sync::BatchTargetStatus::{Skipped, Failed} { error }` (`From<GlobalSyncError>` exists)
   - `onboarding_import::ImportGroupStatus::Failed { error }`, `OriginalStatus::Failed { error }`
   - `skill_update` / `skill_edits` wherever they construct one of the above.
   Rule: core calls `CommandError::from_anyhow` / `CommandError::from` **only** to build report data — never on
   an error it then `?`-propagates. Command seams keep `.map_err(CommandError::from_anyhow)` for whole-command
   failures. Check `refresh.rs:298` (`Cancelled`) and `:581–585` still classify to `CommandError::Cancelled` /
   the right typed variant.
3. **D3 — derive `Serialize + specta::Type` on every type that now crosses**, adding serde tags where the core
   enum has none so the wire discriminators match today's: `#[serde(tag = "status", rename_all = "snake_case")]`
   on the status enums, `tag = "scope"` on `PropagationScope` / `RowRef`, `tag = "reason"` on `PropagationSkip`,
   `rename_all = "snake_case"` on `UpdateSkip`. Do **not** add `#[serde(rename = "...")]` on fields to preserve old
   wire words (`tool_key` stays `tool_key`). `RemovalReport.scope` gets `#[serde(skip)] #[specta(skip)]` (it
   carries plan roots; only `Display` reads it). `SyncOutcome` crosses whole (`mode_used`, `target_path`,
   `replaced`). `PathBuf` fields export as `string` — no conversion. `InvocationEditConflict`, `UnlocatableState`,
   `SyncMode` already derive.
4. **D4 — counters leave the wire.** Delete the `synced/skipped/failed`, `removed/failed`,
   `refreshed/failed/skipped/target_failures`, `imported/failed` fields from the commands' responses (they were
   on the DTOs, which are gone). Core keeps `RemovalReport::removed_rows()` / `failed_rows()` for `Display`/tests.
   Do not add counter fields to core report structs.
5. **D5 — `RemovalReport::merge`.** Add `pub fn merge(reports: impl IntoIterator<Item = RemovalReport>) -> RemovalReport`
   (or `extend`) in `artifact_removal.rs`; `project_ops::configure_project_tools` (or the command) uses it so
   `commands/projects.rs:177–186`'s hand-merge goes. Test it in `core/tests/artifact_removal.rs` (two reports,
   rows and statuses preserved, order stable).
6. **D6 — delete returns its report; `DeleteCleanupFailed` retired.** `artifact_removal::remove_skill` stops
   bailing with `SignalError::DeleteCleanupFailed` (`:581–583`) and returns the `RemovalReport` (`record_deleted:
   false` + failed targets already say "kept"). `delete_managed_skill` returns `Result<RemovalReport, CommandError>`
   instead of `()`. Remove `SignalError::DeleteCleanupFailed` (`core/errors.rs:64`), the `CommandError` variant, its
   `From` arm, and `RemovalReport::failures()` if nothing else reads it. Rewrite the two tests that assert the
   variant (`core/tests/artifact_removal.rs:813`, `core/tests/project_sync.rs:320`) to assert the report
   (`record_deleted == false`, the failed target's `CommandError` variant, the skill row still present).
   ADR-0002 (`docs/adr/0002-…md:11, :41`) — update the two sentences to say the report carries the kept targets.
   CONTEXT.md:109 — delete the "wire code `DELETE_CLEANUP_FAILED` predates the term" clause.
7. **D7 — tests move with the types.** Delete or rewrite the mirror-only tests in `commands/tests/commands.rs`
   (`removal_report_dto_classifies_a_typed_target_failure_at_the_seam`,
   `acquisition_skips_cross_the_wire_and_count_as_skipped_not_failed`,
   `shared_edit_update_target_mapper_preserves_typed_failure`, and any other `to_*_dto` test) as core tests next to
   the type: assert the `CommandError` variant on the report, and keep the wire-shape pins as
   `serde_json::to_value(&status) == json!({ "status": "skipped_acquisition", "reason": "skill_gone" })`.
   Core test sites that `downcast_ref::<SignalError>()` on a report error (grep `Failed { error }` in
   `core/tests/*.rs` — ~12 sites in `artifact_removal`, `propagation`, `refresh`, `unlocatable`, `project_sync`,
   `installer`) become `matches!(error, CommandError::X { .. })`. A test that inspected `error.root_cause()` text
   asserts the `CommandError::Other { message }` contains it.
8. **R1 — `ResyncSummary` crosses as itself.** Derive on `project_sync::ResyncSummary`; delete `ResyncSummaryDto`
   and `to_resync_summary_dto` (`commands/projects.rs:248–262`). Leave `errors: Vec<String>` alone.
9. **Commands.** `sync_skills_to_tools` returns `Vec<BatchTargetOutcome>` (or a thin `BatchSyncReport { results }`
   defined in `global_sync` if you want a named type — prefer whatever core already returns);
   `unsync_*` / `remove_project` / `configure_project_tools` return `RemovalReport`; `refresh_managed_skills`
   returns `RefreshReport`; `update_managed_skill` / Re-points return `{ report: RefreshReport, skills }`;
   `import_onboarding_selection` returns `ImportReport`; `set_skill_invocation_override` returns `{ report:
   { skill_id, skill_name, propagation: PropagationReport }, skills }` — the `InvocationEditReportDto` wrapper
   may stay as the one struct at the seam *if* its `propagation` is core's type (it composes, it does not mirror);
   otherwise give `skill_edits` an `InvocationEditReport`. Delete every `to_*_dto` mapper and mirror enum.
10. **Bindings.** Run `cd src-tauri && cargo test --all` — the `export_bindings` test regenerates
    `src/bindings/index.ts`. Include the regenerated file in your result. Expect `npm run build` to **fail** after
    this ticket (the frontend still names the old DTOs) — that is ticket 02's job; do **not** touch `src/` beyond
    `src/bindings/index.ts`.

## Gate for this ticket

`cd src-tauri && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test --all` green; the bindings
diff shows no change to the `CommandError` union other than the removed `DELETE_CLEANUP_FAILED` variant;
`grep -rn "ReportDto\|to_.*_dto\|StatusDto\|TargetDto" src-tauri/src` returns nothing.

## Constraints

- State your approach and continue — no need to wait for confirmation.
- Never commit. Never touch `CHANGELOG.md`. `.scratch/` is tracked: you may append dated `## Comments` to this
  ticket; never archive or `git mv` under it.
- Prefer targeted edits over whole-file rewrites; re-read a file after writing it.
- Do not refactor anything the spec does not name.
- Report: files touched, the bindings diff summary (type renames the frontend will have to follow, listed
  old → new), test counts before/after, and anything the spec got wrong (a stale `file:line`, a decision that did
  not survive contact with the code — say so rather than silently deviating).
