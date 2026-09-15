# 27: Type the sync-status lifecycle

Status: resolved

Type: task
Blocked by: 25

## What to build

Review #2 (Opus + Sol, verified). ADR 0001 replaced stringly-typed *errors* with a tagged enum checked by both compilers; the parallel concept — an assignment's/target's lifecycle (`pending` → `synced` / `stale` / `missing` / `error`, plus mode `symlink`/`copy`) — is still raw strings with no owning module. At `943f85c`: the write interface `SkillStore::update_assignment_status(id, status: &str, last_error, synced_at, mode: Option<&str>, content_hash)` (`skill_store.rs:858-866`) — six positional params, legal combinations unwritten, nine callers with three or four `None`s; transitions written from four modules (`project_sync.rs:38/69/84/147/176`, `installer.rs:946/958`, `"ok"` for skill targets at `installer.rs:104/201/883/908/1393` and `global_sync.rs:145`); aggregation by string match in `skill_store.rs:993-1035`; read-side literals in `project_ops.rs:185` and (pre-ticket-25) `commands/mod.rs:1010`; wire DTOs `ProjectSkillAssignmentDto`, `SkillTargetDto`, `ManagedSkillDto`, `ProjectDto.sync_status` all `status: string`, while the newer `SyncTargetStatusDto` is a proper tagged union; `AssignmentMatrix.tsx:419-424` turns the string straight into a CSS class. The staleness loop (`project_sync.rs:217-354`) reads the environment, decides and writes the DB in one pass — no pure decision function.

Decided in grilling: **no DB schema change** — stored strings stay; parse at the store seam (unknown legacy value → explicit handling, never a panic).

Deepen:

- `SyncStatus` (and `SyncMode`) enums in core with `#[derive(TS)]`, serialised as the existing strings; the store parses/serialises at its seam; the six-param setter becomes a small typed transition call.
- A pure `next_status(observation) -> SyncStatus` (source present? target present? mode? hashes?) plus a `reconcile` step that applies transitions — the plan/execute split the staleness loop lacks. Aggregate precedence (error/missing > stale > pending > synced) lives beside it.
- Wire: the DTOs carry the generated union; the frontend switches exhaustively (a `satisfies` guard like `COMMAND_ERROR_CODE_MAP` is welcome); `AssignmentMatrix` renders known statuses only. Bindings regenerated and committed. Consider whether `SkillTargetDto.status` (`"ok"`) shares the enum or gets its own — record the decision.
- Tests: `next_status` table test (no temp dirs, no DB); aggregate precedence table; store round-trip incl. a legacy/unknown value.

## Acceptance criteria

- [ ] No status/mode string literals outside the enum's serialisation; `update_assignment_status`'s six-param signature is gone.
- [ ] Pure decision + aggregate tests exist and pass; DB schema unchanged; legacy values handled.
- [ ] Generated TS unions replace `status: string` on the affected DTOs; frontend compiles with exhaustive handling.
- [ ] `npm run version:check && npm run check` green.

## Answer

Landed green in `80d4348` (Fable 5.1 child, medium thinking; rebase over 26/28 had one CONTEXT.md glossary conflict, both blocks kept; 349 cargo + 74 vitest, full gate green on main).

**Module** (`core/sync_status.rs`): `SyncStatus { Pending, Synced, Stale, Missing, Error }` (snake_case serde/TS, `as_str`/`from_stored`, `has_deployed_artifact()`); `SyncMode { Symlink, Junction, Copy }` moved here from `sync_engine` (dead never-stored `Auto` removed; `can_drift()`); `ProjectSyncStatus { Empty("none"), Error, Stale, Pending, Synced }`. **Pure decision** `next_status(&Observation { source_present, target_present, mode, current, source_hash, recorded_hash }) -> SyncStatus` (old D-04/D-05/D-07 rules in precedence order); **aggregate** `aggregate(iter) -> ProjectSyncStatus` (error/missing > stale > pending > synced; none when empty).

**Store seam**: records carry `mode: SyncMode, status: SyncStatus`; all 6 row-mapping sites parse via one `read_lifecycle`; the six-param `update_assignment_status` is gone, replaced by `transition_assignment(id, AssignmentTransition::{SyncCompleted { mode, synced_at, content_hash } | SyncFailed { error } | Reconciled { status, content_hash }})`. Staleness loop split into `observe_assignment` → `next_status` → `reconcile_assignment` (writes only on change). `skill_removal`/`project_ops` cleanup use `has_deployed_artifact` (cleanup now also attempts `Error` rows — `remove_path_any` tolerates NotFound).

**Decisions**: (1) **Legacy/unknown stored value** → row reads as `Error` with `last_error = "unrecognised stored sync lifecycle (mode: …, status: …)"`; unknown mode → `Copy` + `Error` (so the next update re-syncs rather than assuming a following link); never panics, never healthy at the seam, and the *store* never rewrites on read. (Review #3 clarification: the project listing's reconcile pass — `observe_assignment` → `next_status` → `reconcile_assignment` — may then re-derive the row from disk and write `Synced`/`Stale`/`Missing` canonically, clearing `last_error`; e.g. an unknown-status copy row whose target hash matches becomes `Synced`. That is observation-grounded recovery, identical to base's copy branch which ran for any status, not a coercion of the stored value. CONTEXT.md **Sync status** amended to say so.) (2) **`SkillTargetDto.status` shares `SyncStatus`**: global targets were write-only `"ok"`; new writes are `Synced`, `from_stored("ok")` → `Synced` explicitly (`LEGACY_TARGET_OK`, tested). (3) **`ManagedSkillDto.status` stays `String`** — it is the *skill's* status (`SkillRecord.status`, always `"ok"`), not a sync status; no frontend reader.

**Wire**: `ProjectSkillAssignmentDto.{mode,status}`, `SkillTargetDto.{mode,status}`, `ProjectDto.sync_status: ProjectSyncStatus`, `SyncTargetStatusDto::Synced { mode_used: SyncMode }`; bindings regenerated (+`SyncStatus.ts`, `SyncMode.ts`, `ProjectSyncStatus.ts`). New `src/syncStatus.ts` with `SYNC_STATUS_CLASS` / `PROJECT_SYNC_STATUS_CLASS` `as const satisfies Record<…, string>` — a new Rust variant fails `npm run build` until the UI decides how to render it; `AssignmentMatrix` and `ProjectList` render through the maps.

**Tests**: 21-case `next_status` table + 8-case aggregate table (no DB/temp dirs); 15-combo store round-trip asserting raw column text; legacy `ok`, unknown status, unknown mode; `Reconciled` writes status+hash only. `SCHEMA_VERSION` untouched (8). CONTEXT.md gained **Sync status**, **Sync mode**, **Project sync status**. Residue: `Reconciled { status }` accepts any `SyncStatus` in the type though `next_status` only yields Missing/Stale/Synced (narrowing would need a sub-enum).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
