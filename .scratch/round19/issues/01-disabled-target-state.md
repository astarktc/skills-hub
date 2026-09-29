# 01 — `disabled` target state at two grains; Propagation and re-assert skip it; unsync becomes Disable

Status: ready-for-agent
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

(implementing child fills in: sha, what changed, evidence)
