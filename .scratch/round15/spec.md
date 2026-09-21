# Round 15 — wave C: reports cross the wire as themselves (BACKLOG #02) → 1.2.15

Status: open
Opened: 2026-09-21

Absorbs BACKLOG **#02** (line leaves BACKLOG.md in the opening commit). Origin: round-9 panel, Opus seat #3
(`archive/round9/panel/opus.md:84–113`), deferred by round-10 Q8 to "wave C; ADR-0001 amendment written then".

Baseline: 1.2.14 (`af55d70`) — `release.yml` 35619110341 green on the bumped Actions majors (all five targets,
every `.sig`, `updater.json`; Windows has never carried `.sig`/updater entries), operator smoke passed.

## Problem (verified against `main` @ `9345152`)

Every fan-out report exists twice: a core type and a `…Dto` mirror in `commands/mod.rs`, joined by a hand-written
total-match transcription that also *computes report semantics* (the counters):

| core type | wire mirror | mapper | counters computed in |
|---|---|---|---|
| `global_sync::BatchTargetOutcome` | `SyncTargetResultDto` / `BatchSyncReportDto` | `to_sync_target_result_dto` :531 | command body :507–520 |
| `artifact_removal::RemovalReport` | `RemovalReportDto` (+`RemovalTargetDto`, `RemovalScopeDto`) | `to_removal_report_dto` :590 | core (`removed_rows()`), re-merged by hand in `projects.rs:177–186` |
| `refresh::RefreshReport` (+`propagation::PropagationOutcome`) | `RefreshReportDto` (+5 mirror enums) | `to_refresh_report_dto` :892, `to_propagation_target_dto` :859 | mapper :892–953 |
| `onboarding_import::ImportReport` | `ImportReportDto` (+3 mirror enums) | `to_import_report_dto` :1230 | mapper |
| `skill_edits::InvocationEditOutcome.propagation` | `InvocationEditReportDto` | `from_outcome` :1360 | — |

Adding one propagation skip reason touches four places before the frontend sees it. Two counter rules coexist
(core method vs mapper arithmetic). `commands/tests/commands.rs` carries tests that exist only to test the mirror.
And one removal path still smuggles report data out as an *error*: `remove_skill` renders every failed target's
chain into `SignalError::DeleteCleanupFailed { failures: Vec<String> }` (`artifact_removal.rs:581–583`), prose the
frontend joins with `"\n- "` (`commandError.ts:155`) — the exact shape ADR-0001 retired everywhere else.

## Goal

One representation per report: the core type derives `Serialize + specta::Type` and is what the command returns.
No `*ReportDto`, no mapper, no counter arithmetic at the seam. Per-item failures are typed `CommandError` values
classified **where the row settles**, so the report is wire-complete the moment core finishes building it.

## Decisions (all accepted by the operator 2026-09-21, as recommended)

- **D1 `CommandError` moves into core.** `commands/error.rs` (the enum, `GitCloneFailureKind`, `from_anyhow`,
  `internal`, `From<SignalError>`, `From<GlobalSyncError>`, `Display`) moves to `core/errors.rs` beside
  `SignalError` — one module owning both the internal typed conditions and their wire classification.
  `commands/error.rs` is deleted; `commands::CommandError` stays a re-export so the seam reads unchanged.
  The TS name `CommandError` and every `code` tag are a compat contract and do not change.
  *Alternative rejected:* generic reports `RemovalReport<E>` mapped at the seam — keeps classification at the seam
  but keeps a per-type `map_error` transcription for five nested trees, and leans on specta RC generics.
- **D2 Per-item failures are `CommandError`, classified at settlement.** `RemovalTargetStatus::Failed`,
  `PropagationStatus::Failed`, `SkillRefreshStatus::Failed` / `Refreshed.reassert_error`,
  `BatchTargetStatus::{Skipped, Failed}`, `ImportGroupStatus::Failed`, `OriginalStatus::Failed` hold
  `CommandError` (from `anyhow::Error` / `GlobalSyncError` via the existing conversions) — converted at the one
  place each row is settled, after the `{:#}` chain has been written to the row's `last_error` where that happens
  today. `CommandError::from_anyhow` at the command seam remains for errors that fail a *whole command*; core never
  calls it on an error it then `?`-propagates. This is the ADR-0001 timing amendment Q8 promised (D8).
- **D3 Wire shape follows core; the frontend follows the bindings.** No `#[serde(rename)]` to fake the old wire
  vocabulary (`tool_key` stays `tool_key`; `RemovalReport.targets[].rows[]` stays per-target-with-rows rather than
  the flattened per-row `RemovalTargetDto`; `BatchTargetStatus::Synced { outcome: SyncOutcome }` crosses with
  `mode_used`, `target_path`, `replaced`). Serde tags mirror today's discriminators where the core enum has none yet
  (`tag = "status"` / `"scope"` / `"reason"`, `rename_all = "snake_case"`). `PathBuf` exports as `string` (specta
  built-in). `RemovalReport.scope` (plan input carrying roots) is `#[serde(skip)] #[specta(skip)]` — only `Display`
  reads it. `RowRef` ids cross (opaque, harmless); the fold reads `tool` / `project_id`.
- **D4 Counters leave the wire.** `synced/skipped/failed`, `removed/failed`, `refreshed/failed/skipped/
  target_failures`, `imported/failed` were the "one drifting counter rule". The fold already walks every item; it
  derives the counts it needs in `reportOutcome.ts` (one helper per report, tested there). Core keeps
  `removed_rows()` / `failed_rows()` for its own consumers (`Display`, tests). Frontend usage outside the fold is
  nil (`AssignmentMatrix` reads `ResyncSummary.synced`, a different object — see R1).
- **D5 `configure_project_tools` merges in core.** `RemovalReport::merge(Vec<RemovalReport>) -> RemovalReport`
  (or `extend`) replaces the hand-merge in `projects.rs:177–186`; the merged report's `scope` is skipped anyway (D3).
- **D6 Delete returns its report; `DELETE_CLEANUP_FAILED` is retired.** `remove_skill` no longer bails with
  `DeleteCleanupFailed`; `delete_managed_skill` returns `RemovalReport` (`record_deleted: false` + failed targets
  say "kept, retry"). `deleteOutcome` folds it like `removalOutcome` (title `errors.deleteCleanupFailed` becomes the
  per-target entry title; toast copy "skill kept, N targets could not be removed" — new EN/ZH keys). `SignalError::
  DeleteCleanupFailed`, the `CommandError` variant, the `describeCommandError` branch and ADR-0002's two mentions
  update. *Why forced, not creep:* with D2 the only alternative is core rendering a `CommandError` to prose for the
  `failures` strings — exactly what ADR-0001 forbids. Delete keeps its refetch (`loadManagedSkills`) as today.
  CONTEXT.md:109's "wire code `DELETE_CLEANUP_FAILED` predates the term" note is deleted with the code.
- **D7 Tests move with the types.** `commands/tests/commands.rs` mirror-only tests
  (`removal_report_dto_classifies_a_typed_target_failure_at_the_seam`, `acquisition_skips_cross_the_wire…`,
  `shared_edit_update_target_mapper_preserves_typed_failure`) are deleted or rewritten as core tests asserting
  the `CommandError` variant on the report (~12 core test sites currently `downcast_ref::<SignalError>()` on a
  report error become `matches!(error, CommandError::X { .. })`). Wire-shape pins (`serde_json::to_value` ==
  `json!({ "status": "skipped_acquisition", "reason": "skill_gone" })`) stay, next to the type.
- **D8 ADR-0001 amendment (parent-authored, after the code lands):** "Report rows classify at settlement" —
  classification-by-downcast now happens in core at the point a fan-out row is settled (chain richest, row's
  `last_error` already written), and at the command seam only for whole-command failures; `CommandError` is a core
  type; there are no wire mirrors of report types; counters are derived by the consumer. AGENTS.md **Error wire
  contract** (path, seam rule) and **Target fan-out** (add the "core report types cross the wire; per-item failures
  are `CommandError` at settlement; no `*ReportDto`" rule) updated in the same commit.
- **D9 Execution shape:** two **sequential** lanes (the frontend needs the regenerated bindings), each a
  `delegate_task` child on Pi, `runtimeMode: full-access`, "state your approach and continue" pre-approved, never
  commits, never touches CHANGELOG `[Unreleased]` or archives under `.scratch/`. Lane A (backend) runs
  `cargo test --all` so `src/bindings/index.ts` regenerates and **includes that diff** in its handoff; parent commits
  A, then dispatches lane B (frontend) against the new bindings. Parent writes D8, runs the gate, adversarial review
  by a model other than the implementer (Standards + Spec), releases **1.2.15**.

## Riders

- **R1 `ResyncSummary` crosses as itself.** `commands/projects.rs:248–262` is a field-for-field mirror; derive on
  the core struct, delete `to_resync_summary_dto`. Its `errors: Vec<String>` (rendered chains) is **not** touched —
  a separate prose-on-wire smell; recorded as BACKLOG residue if not fixed here.

## Out of scope (named)

- `BulkAssignErrorDto` / `AssignTargetStatus` (`projects.rs:329`, `project_sync.rs:224`): a per-tool failure list,
  not a report the fold consumes; `AssignTargetStatus::Assigned { record }` carries a full DB record. Leave.
- `ManagedSkillDto`, `ProjectViewDto`, `InstallResultDto`: catalog/view DTOs, not reports.
- Round-9 Opus #4 (skills world refetch vs apply) — separate item, already partly addressed by round-10 Q7.

## Operator answers (2026-09-21)

- Q1 `core/errors.rs`. Q2 retire `DELETE_CLEANUP_FAILED` now. Q3 counters derived in the fold.
- Q4 routing: lanes A and B = GPT-6 Astra on Pi (`openai-codex/gpt-6-astra`, thinking medium); adversarial review =
  Claude Opus 5 on Pi (`anthropic/claude-opus-5`, thinking high).

## Tickets

01 backend-reports-cross-the-wire (D1–D7, R1) · 02 frontend-follows-bindings (D3, D4, D6 fold + i18n) ·
03 adr-0001-amendment-and-agent-docs (D8, parent)

## Lanes

A: 01 → B: 02 (sequential) · parent: 03, gate, review, release
