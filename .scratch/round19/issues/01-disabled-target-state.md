# 01 — `disabled` target state at two grains; Propagation and re-assert skip it; unsync becomes Disable

Status: implemented
Blocked by: —
Spec: `.scratch/round19/spec.md` — D8 (grill Q6, Q28). Absorbs BACKLOG #52's enable/disable half.

## Why

Every unsync today is undone by the next Refresh's `reassert_auto_sync` (`round18/review/flows.md` X6); the toolbar
"Uninstall from tool directories" removes hundreds of targets unconfirmed and impermanently (`critique-*` P0-1/F01).
The operator's "not on this Tool" must be recorded so it survives Refresh, Propagation and re-assert.

## Work (backend, `core/` first; commands are wiring)

- **Schema.** `skill_targets.status` admits `disabled` (a `SyncStatus`/target status variant — read `core/sync_status.rs`
  and the store's status strings first; extend the enum, its serde string, and every match). Skill-level disable is a
  column `skills.disabled_at INTEGER NULL` (additive `ALTER TABLE` in `skill_store`'s migration list, same pattern as
  `imported_from_tool`). Project assignment rows (`project_skill_assignments`) admit `disabled` too (skill-level only).
- **Semantics** (one module, `core/skill_disable.rs`, declared in `core/mod.rs`, under `mutation_guard::serialized` at
  its entry points, composites through `*_unlocked` seams):
  - `disable_targets(scope)` with `DisableScope::{SkillTool { skill, tool }, Skill { skill }, Skills { ids }}` — plans
    through `artifact_removal` (never removes a path itself), and on **successful** removal sets the row `disabled`
    instead of deleting it; on failure ADR-0002 applies (row kept `error`). Skill scope also disables every project
    assignment row of that skill (artifact removed, row kept `disabled`) and sets `disabled_at`.
  - `enable_targets(scope)` clears the marker and re-syncs through the existing batch (`global_sync` /
    `project_sync`) — a first sync again, so `TARGET_EXISTS` can occur and is report data.
  - Answers a `RemovalReport`-shaped report for disable and the sync report for enable (report types cross the wire
    as themselves; no DTO mirrors).
- **Skips.** `propagation.rs`, `refresh.rs`'s `reassert_auto_sync`, `global_sync.rs`'s effective-target fan-out,
  `project_sync.rs` and `onboarding_import.rs`'s first sync all **skip** disabled rows and report `Skipped` with a new
  typed skip reason (`Disabled`), never `Failed`. A disabled skill is never re-materialised by anything but `enable`.
- **Commands.** `unsync_skill_from_tool`, `unsync_skill`, `unsync_all_skills` are **replaced** by
  `disable_skill_targets(scope)` / `enable_skill_targets(scope)` (remove the old three from `collect_commands!`,
  the frontend seam and the fixture). `#[specta::specta]`, listed in `specta_builder()`; `cargo test` regenerates
  `src/bindings/index.ts`; commit it.
- **DTOs.** `SkillTargetDto`/`ManagedSkill` expose the status; `ManagedSkill.disabled_at: number | null`; project view
  cells expose `disabled`. The per-world shims re-export.
- **Frontend minimum** so `npm run build` stays green: `useSyncOrchestration`/`useSkillLibrary` call the new commands
  where the old ones were called (the visual world arrives in ticket 11–13; here the pills keep working — a disabled
  target renders through the existing `error`-style path with an i18n label "Disabled", no red). `reportOutcome`
  learns the `Disabled` skip reason (i18n key). No new UI.
- **Fixture parity** (`src/fixtures/backend.ts`, `model.ts`, `scenarios.ts`): the new handlers, `disabled` rows in
  `failures` (one per grain), re-assert and Refresh honour them; `backend.test.ts` extended.
- **Tests** (`core/tests/`): disable per pair → row `disabled`, artifact gone, Refresh/re-assert/Propagation leave it,
  enable re-syncs; skill scope covers project rows; failed removal keeps `error` not `disabled`; onboarding import
  skips a disabled pair; `export_bindings` regenerates.
- **Docs.** CONTEXT.md: *Disabled* as a term (target state, two grains, the durable "not on this Tool"); AGENTS.md
  § Invariants "Sync-target mutation" gains the two entry points; "Artifact removal is one module" gains the disable
  caller. CHANGELOG line under Unreleased.

## Out of scope

The bulk rail, confirmations, the row switch (ticket 13); the strip's *disabled* mark (ticket 11).

## Review

Astra @ high, adversarial, on the diff: settlement ordering (row flips only after removal succeeds), guard
re-entrancy, skip-vs-fail classification in every fan-out, migration idempotence.

## Result

Uncommitted working tree on `main` @ `553db2a` (two children: the first implemented ~90% and died to a provider
timeout; the second finished, verified and wrote this). Nothing committed.

### What changed

**Backend (`src-tauri/src/`)**
- `core/skill_disable.rs` (new, declared in `core/mod.rs`): `DisableScope::{SkillTool{skill_id,tool}, Skill{skill_id},
  Skills{skill_ids}}` (serde `kind`-tagged, crosses the wire as itself); entry points `disable_targets` /
  `enable_targets` under `mutation_guard::serialized`; everything below is unlocked seams.
  - Disable plans through `artifact_removal` (`SkillTool`; `SkillGlobal` + one `ProjectSkill` per project) and executes
    with the new `artifact_removal::execute_settling_unlocked(.., Settlement::Disable)`. On success, rows flip
    `disabled` only after removal succeeds; on failure they stay `error` (ADR-0002). Rows that are already
    `disabled` are dropped from the plan, so disabling twice settles nothing. A pair with no row gets a
    `disabled` **marker row** (`TargetTransition::DisabledRecorded`, the path where the target *would* be), written
    only when no row exists after execution **and the Tool's shared artifact settled** (review F3: a failed
    shared-dir removal mints no `disabled` sibling), so a failed removal's `error` row is never overwritten. Skill grain
    also sets `skills.disabled_at` (even when a removal failed: intent is recorded, and the failed rows stay
    `error`/retryable).
  - Enable: pair grain clears the `disabled` rows of the Tool's shared-dir group, then first-syncs the pair
    through `global_sync::sync_skills_to_tools_unlocked` (overwrite off, same-content on). Skill grain clears
    `disabled_at` and every `disabled` row, then first-syncs the global rows and the assignments
    (`project_sync::first_sync_assignments_unlocked`, new). A pair Enable of a skill disabled at skill grain answers
    `Skipped{TARGET_DISABLED}` and changes nothing.
- `sync_status.rs`: `SyncStatus::Disabled` (`"disabled"`), terminal in `next_status` (reconcile never changes it),
  excluded from the project `aggregate` fold (all-disabled folds `Empty`).
- `skill_store.rs`: schema v10 `ALTER TABLE skills ADD COLUMN disabled_at INTEGER NULL`. It is in the fresh-DB
  path and presence-guarded in the incremental path, so it is idempotent and a partial upgrade completes.
  Also `skill_disabled_at` / `set_skill_disabled_at` / `disabled_skills`, `TargetTransition::{Disabled,
  DisabledRecorded}` and `AssignmentTransition::Disabled`.
- Skips (all report data, never `Failed`):
  - `propagation.rs`: `PropagationSkip::Disabled`. It covers a skill disabled at skill grain (every row reported,
    none touched), a shared-dir group with any disabled member, and a disabled assignment.
  - `refresh.rs` re-assert: maps the batch's `TARGET_DISABLED` to `PropagationSkip::Disabled`.
  - `global_sync.rs`: `DisabledGate`, read once per skill. It skips the skill when it is disabled at skill grain,
    or when any row of the Tool's shared-dir group is `disabled`. It reports
    `BatchTargetStatus::Skipped{ CommandError::TargetDisabled{tool} }` (new wire code `TARGET_DISABLED`).
    Onboarding import's first sync gets the gate through this batch.
  - `project_sync.rs`: `ProjectSyncOutcomeStatus::Skipped{ reason: ProjectSyncSkip::Disabled }` in assign and
    resync; a new assignment of a skill disabled at skill grain is recorded `disabled`, not synced.
  - `target_relocation.rs`: leaves `disabled` rows alone.
- Commands: `unsync_skill_from_tool` / `unsync_skill` / `unsync_all_skills` are removed (from `commands/mod.rs`,
  `collect_commands!` and `artifact_removal`'s public entry points). They are replaced by
  `disable_skill_targets(scope) -> RemovalReport` and `enable_skill_targets(scope) -> EnableReport { global:
  BatchTargetOutcome[], projects: ProjectSyncReport }`. `ManagedSkillDto.disabled_at: number | null`; the target
  and assignment `status` admits `"disabled"`. `src/bindings/index.ts` is regenerated by `cargo test`.

**Frontend**
- `useSkillLibrary`:
  - The pill toggle is three-state: a disabled target calls Enable, any other target calls Disable at pair grain,
    and no target calls sync.
  - Card unsync is a skill-grain Disable. The toolbar "Uninstall from tool directories" is a `Skills` Disable of
    every managed skill (ticket 13 puts it behind select-all + confirmation).
  - Deploy on a skill disabled at skill grain calls Enable.
- `reportOutcome`: `syncOutcome` treats `TARGET_DISABLED` as a skip (silent in bulk/install, a warning on an explicit
  toggle). There is a new `enableOutcome`. `projectSyncOutcome` counts `skipped`: a toggle-on of a disabled skill
  warns "Assigned, but not synced". `commandError.ts` has a `TARGET_DISABLED` branch.
- UI: `SkillCard` renders a disabled pill `inactive` with a "(Disabled)" label and title, no red; `AssignmentMatrix`
  does not count a disabled global target as a global deployment; `syncStatus.ts` adds the `disabled` class.
- i18n (en): `errors.targetDisabled`, `errors.syncSkippedDisabledTitle`, `status.targetsEnabled`,
  `projects.assignedWhileDisabled`, `syncTarget.disabled`, `syncTarget.disabledTitle`.
- Fixture parity (`src/fixtures/backend.ts`, `model.ts`, `scenarios.ts`) covers the new handlers, the old three
  removed, the batch gate, the propagation / re-assert / project-sync skips, the Disable settlement, and marker rows.
  `failures` holds one row per grain: `noisy-linter` is disabled on Codex + Cursor, and `retired-playbook` is
  disabled at skill grain, including its quartermaster assignment.

**Docs**: CONTEXT.md adds the *Disabled* term, updates *Sync status* (disabled is not re-derived) and makes
*Artifact removal* name Disable as the one keep-row caller. In AGENTS.md § Invariants, "Sync-target mutation" gains
the two entry points, the fan-out list gains the Disable/Enable bullet and `skipped { disabled }`, and "Artifact
removal is one module" gains the `skill_disable` caller and `Settlement::Disable`. The CHANGELOG has a
`[Unreleased] ### Changed` entry.

### Decisions (kept from the first child unless noted)
- **`disabled_at` is not a `SkillRecord` field**; it is read and written only through dedicated store methods. Every
  writer of the skills row upserts a whole record, so a finalize or Re-point would silently re-enable a skill.
  Pinned by `upserting_the_skill_record_keeps_the_disabled_marker`.
- **`ProjectSyncSkip` is a new tagged enum** (`{ reason: "disabled" }`), not a reuse of `PropagationSkip`. Project
  sync had no skip arm; a dedicated enum keeps its reasons its own, like `PropagationSkip`.
- **Global batch skip reason is a `CommandError` variant (`TARGET_DISABLED`)**, because `BatchTargetStatus::Skipped`
  already carries `error: CommandError` (the `TOOL_NOT_INSTALLED` pattern). Propagation and project sync use their
  typed `Disabled` reasons.
- **Shared-dir rule**: one disabled member disables the shared artifact for every sibling, in Propagation and in the
  batch. Writing it for a sibling would re-materialise it where the disabled Tool reads.
- **Marker rows** for pairs that were never synced, so a Tool can be switched off before a re-assert creates it.
  (review 2026-09-29, F1) A marker is **intent only** and owns nothing: see *Review fixes* below.
- **Enable deletes the cleared global rows before the batch** so the batch records them afresh. A failed Enable
  (e.g. `TARGET_EXISTS`) therefore leaves the pair with **no row**, neither `disabled` nor `error`. See ticket 02.
- **`RemovalScope::SkillGlobal` / `EveryGlobalTarget` are kept**: `SkillGlobal` is planned by skill Disable, and
  `EveryGlobalTarget` is no longer planned by any operator action. The removal tests keep their old helper names as
  test-local wrappers over `plan` + `execute_unlocked`.
- (second child) **Onboarding-import test shape**: an import always creates a new skill (the name must be free),
  so its first sync can never meet a disabled row of its own; it gets the gate through the shared batch. The test
  instead pins the one real exposure: an unmanaged copy of a disabled skill reappearing in the Tool dir. Importing
  it fails per group, and the disabled row and the copy are untouched. (Observation: the onboarding plan also hides
  that copy, because `list_all_skill_target_paths` includes `disabled` rows' recorded paths.)
- (second child) Fixed the red fixture test at its cause: the fixture was right and the **assertion** was over-strict.
  noisy-linter's re-assert onto Amp fails `TOOL_NOT_WRITABLE` on the scenario's unwritable shared dir, which the
  Rust re-assert also reports as `Failed`. The test now asserts each disabled Tool is exactly
  `skipped: disabled` and pins that one legitimate failure.
- (second child) Added the 6 i18n keys the frontend already referenced; they were missing, so `resources.test.ts`
  was red.

### Tests
- Rust `core/tests/skill_disable.rs` (18):
  - Pair grain:
    - `disabling_a_pair_removes_the_artifact_and_keeps_the_row_disabled`
    - `a_disabled_pair_survives_refresh_reassert_and_propagation`
    - `disabling_a_pair_with_no_row_records_a_marker_the_reassert_honours`
    - `enabling_a_pair_resyncs_it_as_a_first_sync`
    - `enable_reports_a_directory_in_the_way_as_target_exists_data`
    - `disabling_twice_settles_nothing_new`
    - `a_shared_dir_sibling_is_skipped_while_a_member_is_disabled`
    - `an_unknown_tool_with_no_row_is_a_typed_refusal`
    - `a_failed_removal_keeps_the_row_error_not_disabled`
  - Skill grain:
    - `disabling_a_skill_covers_global_targets_and_project_assignments`
    - `a_disabled_skill_is_never_rematerialised_by_any_automatic_pass`
    - `onboarding_import_never_rematerialises_a_disabled_pair` (new)
    - `a_new_assignment_of_a_disabled_skill_is_recorded_disabled_not_synced`
    - `enabling_a_skill_resyncs_every_disabled_row_and_clears_the_marker`
    - `enabling_a_pair_of_a_disabled_skill_is_skipped`
    - `the_skills_scope_disables_each_named_skill_and_drops_stale_ids`
    - `an_unknown_skill_is_not_found`
    - `scope_and_skip_reasons_cross_the_wire_as_themselves`
- Other Rust tests:
  - `skill_store.rs`: `v10_migration_adds_disabled_at_to_a_v9_database` (idempotent + partial upgrade),
    `upserting_the_skill_record_keeps_the_disabled_marker`, `disabled_target_transitions_flip_or_record_the_row`
  - `sync_status.rs`: Disabled rows in the `next_status` / `aggregate` / serde tables
  - `target_relocation.rs`: `relocation_leaves_a_disabled_row_alone`,
    `a_row_left_in_a_former_dir_can_be_disabled_after_the_tool_is_gone`
  - `export_bindings` regenerated the bindings.
- Vitest:
  - `src/fixtures/backend.test.ts` "Disabled: two grains…" (5 cases).
  - `useSkillLibrary.test.ts`: the invoke→fold table covers `disableSkillTargets` (unsync-all / unsync / toggle-off)
    and `enableSkillTargets` (toggle-enable / enable-skill).

### Gate (2026-09-28, this tree)
- `npm run version:check` OK (1.2.18).
- Green: `npm run test` (24 files / 491 tests), `npm run build`, `rust:fmt:check`, `rust:clippy` (-D warnings) and
  `rust:test` (728 passed).
- `npm run check` is **red only in ticket 07's files** (owned by the concurrent styling child, not touched here):
  - `eslint`: `src/components/ui/dialog.tsx:214:5`, react-hooks/immutability (`opener.current` assigned in
    `useLayoutEffect`).
  - `styles:check`: `src/components/ui/specimen.tsx` references legacy App.css class `sr-only`.
- Visual sanity: `dev:fixture` `?scenario=failures`. Disabled pills render `inactive` with "(Disabled)" and the
  enable title, not red.

### Review fixes (2026-09-29)

- **Ownership (F1, amended by F1-R1).** One status-independent rule, `skill_store::owns_artifact`
  (`SkillTargetRecord` / `ProjectSkillAssignmentRecord::owns_artifact`): a row owns the artifact at its path iff
  `synced_at IS NOT NULL`, whatever its status. Every success writer (`Recorded`, `SyncCompleted` for global first
  sync / Enable, project assign / resync / Enable, Propagation, relocation, and every earlier app version, checked in
  git history) sets `synced_at`; no transition clears it (`Disabled` / `SyncFailed` / `Reconciled` keep it via
  `COALESCE`), so a formerly deployed row stays an owner; the marker writers (`DisabledRecorded`, an assignment
  recorded `disabled` for a disabled skill, D9's future "Keep existing") write NULL, and a failed first sync (Enable's
  collision) leaves it NULL. Unknown ownership (a legacy or partially written row) counts as none: at worst an
  artifact of ours stays on disk, never someone else's is deleted. Readers: `artifact_removal::plan` sets non-owners
  aside (`RemovalPlan.markers`) in every scope; when the whole scope succeeded, Delete clears their rows alone
  (**`RemovalReport.cleared_markers: RowRef[]`**, wire field) and Disable records them `disabled` (row only, still no
  owner, so Enable retries them); until then they keep status and `last_error`. Project re-sync and Propagation
  (both scopes) pass `overwrite = owns_artifact` (a non-owner gets a first sync; the collision is report data), and
  target relocation skips non-owners. Only a successful sync makes a row an owner. `removalOutcome` does not call a
  toggle-off that cleared a marker "nothing planned", and bulk unassign counts cleared markers (N1).
- **Skill marker gates (F2).** `resync_project_unlocked` (so resync-all) reads `disabled_at` once per skill and skips
  every row of a disabled skill before any filesystem work (`Skipped { Disabled }`; an `error` row keeps its status
  and `last_error`). Bulk assign answers `Skipped { Disabled }` for any existing row of a disabled skill, never
  `AlreadyAssigned`. Startup relocation skips a disabled skill's former-dir rows (`left_in_place`).
- **Shared-dir marker admission (F3).** `disable_pair` records the marker only when the removal report has no failed
  row (and the pair still has no row).
- **Fixture (F2–F4).** Artifact presence is a per-path map, independent of row status: set by every write, cleared
  only by a successful removal, left set by a failed one (a repeated Disable / Delete on a locked path keeps
  failing); a path never touched falls back to what the rows imply. `resyncOne` and bulk assign share the skill-marker
  gate (`isDisabledAssignment`); `disablePair` admits a marker only after a clean removal; removal sets intent-only
  rows aside and reports `cleared_markers`.
- **Deploy (F5).** `skillPresentation.deployAction(skill)` is the one predicate: `deployed` (a live target) /
  `enable` (skill marker set, or every target row pair-disabled — with the count) / `sync`. The card labels the
  enable case "Enable N disabled tools and sync" (`disabledSkips.deployDisabledPairs`); the hook routes it to one
  skill-grain `enableSkillTargets` (which re-syncs every disabled row of the skill), never an ordinary sync.
- **Folds (F6, F7).** `isDisabledSkip` is shared by `syncOutcome` and `importOutcome` (a Disabled pair in an import is
  a silent skip). Bulk assign counts disabled skips (`disabledSkips.bulkAssignNotSynced` / `bulkAssignMixed` /
  `bulkAssignPartial`, warning tone) and never says `bulkAssignNothing` for them; resync / resync-all summaries carry
  the skipped count (`disabledSkips.resync*`). New i18n group `disabledSkips` appended at the end of `en`.

### Left for later tickets
- **02**: a failed Enable (first sync `TARGET_EXISTS` / error) leaves no row. 02's "failed first sync records an
  `error` row" should cover Enable's batch too. Also: Enable's `TARGET_EXISTS` is folded as an error entry; it does
  not go through the frontend overwrite-ask seam (`useSyncOrchestration`). Decide whether it should. Also decide
  whether the onboarding plan should keep hiding an unmanaged copy at a disabled row's path.
- **11**: the strip's own *disabled* mark. Today it is the `inactive` pill + label; `.disabled` status class has no
  CSS (App.css/styles are ticket 07's), and the matrix cell for a disabled assignment is unstyled.
- **13**: the library-wide Disable (toolbar "Uninstall from tool directories") is still unconfirmed. It needs
  select-all + a count confirmation, a bulk Enable, and the row switch. Copy still says "unsync" in
  `unsyncSkillTooltip`, `actions.unsyncing` and `unsyncNothingPlanned`.
- Review ran 2026-09-29 (fix-then-ship, `.scratch/round19/review/ticket-01-review.md`); its findings are settled —
  see *Review fixes* above and `## Comments`.
- Review sketch 6 (`disabled_skill_survives_each_byte_adapter`: Update / Restore / Re-point / Edit replay on a
  disabled skill) is not written; the gates it would pin are Propagation's and the batch's, already covered.
- (F1-R1) Ownership is now `synced_at`-only for every status (see *Review fixes*), so ticket 02's failed-first-sync
  `error` rows are non-owners by construction. Left for 02: project sync does not type the collision — a project
  first sync / Enable / resync meeting a folder surfaces `OTHER` ("target already exists"), unlike the global
  batch's `TARGET_EXISTS` (`CommandError::from_anyhow` does not downcast `sync_engine::TargetExistsError`); the
  fixture mirrors that. A Disable that only flipped markers reports no row (the report has no field for it).

## Comments

### 2026-09-29 — adversarial review: fix-then-ship

Review completed against the uncommitted ticket-01 delta on `main` at `553db2a`, excluding ticket 07.
Full findings, reproductions/test sketches, nine-area coverage and command evidence:
`.scratch/round19/review/ticket-01-review.md`.

Three blockers: (1) intent-only marker rows make never-owned foreign folders eligible for Delete;
(2) project resync and startup target relocation ignore `skills.disabled_at` when failed Disable leaves an
`error` row; (3) disabling a shared-dir sibling with no row inserts a `disabled` marker even when the
shared artifact's removal failed. Four should-fix findings cover fixture retry presence and frontend
Deploy/skip-fold behavior. Existing tests do not cover these combinations.

Fresh verification: 18 disable Rust tests and all 728 Rust tests passed; targeted Vitest 15 files / 396 tests
passed; binding export passed with byte-identical output; `bun x tsc -b --noEmit` passed. In-memory probes
confirmed the resync bypass, shared-marker missettlement, false fixture retry success and fold errors.
No source changes, live-library runs, archive operations or commits made by the reviewer.

### 2026-09-29 — review findings settled (fix-then-ship)

Every blocking and should-fix finding of `.scratch/round19/review/ticket-01-review.md` is fixed; design changes are
recorded under `## Result` → *Review fixes*. Per finding (fix → regression test):

- **F1** intent-only markers own nothing → `is_intent_only` + planning sets markers aside + `cleared_markers`.
  Rust `intent_only_marker_delete_preserves_unmanaged_bytes` (marker→foreign, foreign→marker, marker→foreign→Enable
  collision), `an_intent_only_assignment_is_unassigned_without_touching_its_path`,
  `an_owned_disabled_target_is_still_removed_by_delete`; fixture "F1: an intent-only marker gives Delete no path…";
  `reportOutcome` "a report that only cleared a marker row is a success".
- **F2** resync / resync-all / bulk assign / relocation gate on `disabled_at` → Rust
  `partial_skill_disable_resync_skips_error_assignments` (with a healthy skill in the same project),
  `partial_skill_disable_relocation_does_not_materialise`; fixture "F2: a failed skill Disable keeps the row error…".
- **F3** marker admission waits for the shared artifact → Rust
  `missing_shared_sibling_disable_failure_does_not_mint_disabled` (incl. the retry); fixture "F3: …".
- **F4** fixture presence model → fixture "F4: presence is independent of status…" (Probe B, then Delete).
- **F5** deploy of an all-pair-disabled skill enables it → `useSkillLibrary.test.ts` "deploying a skill whose every
  target is pair-disabled enables it" (real `enableOutcome`), `skillPresentation.test.ts` `deployAction` table.
- **F6** `importOutcome` skips `TARGET_DISABLED` → `reportOutcome.test.ts` "Disabled is a skip in every fold"
  (sync bulk/install/toggle, install, import, refresh, Edit, enable, project resync).
- **F7** bulk-assign / resync disabled counts → "projectSyncOutcome counts disabled skips" (all-skipped, mixed,
  partial, resync, resync-all).
- Sketch 5 → `mutation_guard.rs` `disable_enable_entry_points_are_serialized` (asserts disk + row unchanged while the
  guard is held); sketch 7 → `disabled_virtual_group_assignment_enables_once`; sketch 8 →
  `delete_disabled_skill_settles_rows_and_cascade`. Sketch 6 not written (see *Left for later tickets*).

Mutation check: reverting the F1, F2 and F3 lines makes their three Rust tests fail; restored, all pass.
Gate: `npm run version:check` OK (1.2.18); `npm run check` exit 0 — lint, styles:check, vitest 24 files / 517 tests,
build, fmt, clippy `-D warnings`, `cargo test --all` 737 passed.

### 2026-09-29 — bounded re-review: fix-then-ship (F1-R1 remains)

Re-review appended at `.scratch/round19/review/ticket-01-review.md` → `## Re-review — 2026-09-29`.
**F2–F7 closed; F1 only partially closed. One blocker:** an intent-only project assignment
(`disabled`, `synced_at = NULL`) becomes `error`, still NULL, when skill Enable encounters a foreign folder and
reports `TARGET_EXISTS`. `is_intent_only` then returns false, so unassign/Delete/Disable plans and deletes that
never-owned foreign path. This is reachable through 01's new project Enable path today, not just ticket 02's future
global failed-first-sync rows. The added global Enable-collision test is safe because global Enable deletes its
marker; project Enable retains its failed assignment. The project marker-removal test does not attempt Enable.

Keep ownership independent of lifecycle status across failed Enable and reconcile; preserve the error diagnostic
without granting path ownership. Add project marker → foreign folder → failed Enable → unassign/Delete regressions
and mirror the policy in the fixture/docs. The review includes a concrete failing test sketch and source trace.
Nonblocking N1: markers-only bulk unassign succeeds but reports count 0 because `removalCounts` omits cleared markers.

Fresh verification: requested Rust filters **26 / 32 / 13 / 13 passed** (skill_disable / artifact_removal /
target_relocation / mutation_guard); requested Vitest **15 files / 422 tests passed**; additional project-hook suite
**31 passed**, then `bun x tsc -b --noEmit` exit 0. Reviewed timestamp writers/history, all new marker-report consumers,
F2/F3 test sensitivity and the guard's held-lock state assertions. No source edits, live-library runs, archive/move
operations or commits. The earlier “all findings settled” claim is superseded by this re-review for F1.

### 2026-09-29 — F1-R1 + N1 fixed (re-review follow-up)

- **F1-R1 (blocking)** — ownership is independent of lifecycle status: `skill_store::is_intent_only` is replaced by
  `owns_artifact` (`synced_at IS NOT NULL`) on both tables. `artifact_removal::plan` (`push_global`,
  `push_assignments`) sets every non-owner aside in every scope; the settlement clears it (Delete) or flips it
  `disabled` (Disable, row only), both only when the scope had no failure — until then it keeps `error` and its
  `last_error`. The same rule now gates the other paths that could take a foreign folder: project re-sync
  (`resync_project_unlocked`, was `overwrite = true`) and Propagation at both scopes pass `overwrite =
  owns_artifact`, and target relocation skips non-owners. Success writers audited: `Recorded` / `SyncCompleted`
  always write `synced_at`; none needed fixing. Test fixtures that seeded *materialised* rows with `synced_at: None`
  (`tests/artifact_removal.rs::seed_assignment`, `tests/skill_edits.rs::set_propagates_to_copy_fallback_target`)
  now record `Some(1)`. Regressions:
  - Rust `skill_disable::failed_project_enable_never_makes_a_foreign_folder_ours` — the re-review sketch (disable
    skill → assign → foreign folder → Enable fails, row `error` + `synced_at None` + diagnostic), then each of:
    reconcile + bulk unassign, toggle-off, remove project, Disable again (+ Enable retries the collision), Delete,
    resync one, resync all — foreign bytes survive in every case.
  - Rust `skill_disable::propagation_never_overwrites_a_path_the_row_does_not_own` (copy-mode non-owners, global
    and project) and `target_relocation::relocation_leaves_a_row_that_owns_nothing_alone`.
  - Fixture: `enableSkill` / assign / resync / Propagation go through `syncAssignment(.., overwrite)`, which now
    models a project first-sync collision (`FixtureState.projectOccupants`, `OTHER` "target already exists"); markers
    are status-independent (`ownsArtifact`), Disable flips them. `backend.test.ts` "F1-R1: a failed project Enable
    leaves the row error with no owner; no removal or re-sync takes the folder" (same seven acts); the
    `remove_project keeps…` case now expects the never-synced pending/error rows kept (scope failed) with their
    diagnostic. The failures scenario's global `error` rows record `synced_at` (a deployed row whose later sync
    failed, as in Rust).
  - Mutation checks: restoring the old status-dependent rule fails both Rust F1-R1 tests and the fixture F1-R1 test;
    restoring `overwrite = true` in resync fails the Rust test's resync act.
- **N1** — `removalCounts` gains `cleared` (`cleared_markers.length`); bulk unassign's success and partial counts are
  `removed + cleared` (copy "Removed from N tools"; no artifact claimed). `reportOutcome.test.ts` "a markers-only
  bulk unassign counts every cleared assignment" and "a mixed bulk unassign counts removed targets and cleared markers
  together".
- Docs: CONTEXT.md *Disabled* / *Artifact removal* ownership sentences, AGENTS.md "Artifact removal is one module"
  ownership line, `skill_store` / `artifact_removal` / `skill_disable` / `propagation` / `target_relocation` doc
  comments, fixture comments.
