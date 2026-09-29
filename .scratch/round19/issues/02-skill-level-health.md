# 02 — Skill-level health persists on the skill row; failed first sync records an `error` row; declined records `disabled`

Status: ready-for-agent
Blocked by: 01
Spec: `.scratch/round19/spec.md` — D9 (grill Q7). Feeds #49 (issues banner + chip) and the inspector ladder.

## Why

A skill whose fetch failed, whose Edit conflicts, whose first sync was declined or failed, or whose Delete was kept
looks identical to a healthy one after the toast closes (`flows.md` X1, `critique-opus` P0-2, round-18 bake-off
finding (b)). Round 19's rows, chip, banner and ladder need stored data.

## Work

- **Schema** (additive, `skill_store` migration list): `skills.last_outcome_kind TEXT NULL`,
  `skills.last_outcome_error TEXT NULL` (the `CommandError` serialised as JSON — the same tagged shape that crosses
  the wire), `skills.last_outcome_at INTEGER NULL`.
- **Kinds** (`core/skill_health.rs`, declared in `core/mod.rs`): `SkillOutcomeKind::{FetchFailed, EditConflict,
  DeleteIncomplete, RepointFailed, RestoreFailed, ImportFailed}` — an enum with serde + `specta::Type`. One writer
  API: `record_outcome(store, skill_id, kind, &CommandError)` and `clear_outcome(store, skill_id, family)`, where a
  *family* is the set of kinds a success clears (an Update success clears `FetchFailed`/`RepointFailed`/`RestoreFailed`;
  an Edit save clears `EditConflict`; a completed Delete removes the row anyway).
- **Writers, at settlement** (where the row's `last_error` is already written today): `skill_update::apply_unlocked`
  (acquisition failure → `FetchFailed`; Edit replay conflict → `EditConflict`), `repoint.rs`, the Restore path,
  `artifact_removal` for a kept Delete (`DeleteIncomplete`), `onboarding_import` for a group that finalised but
  failed later (`ImportFailed`). Core writes it where the outcome settles — never in `commands/`.
- **Failed first sync records a target row** in `global_sync.rs`: today only `Synced` targets are recorded; a target
  that failed to materialise (any error other than `TARGET_EXISTS`) is recorded with status `error` and `last_error`,
  so the strip can render × and Retry has a row (symmetric with ADR-0002). A **declined** `TARGET_EXISTS` (the
  operator kept the existing folder) is recorded as **`disabled`** for that pair (ticket 01's grain) — the command
  seam gets the decline list from the frontend's ask (extend `sync_skills_to_tools`'s per-pair `overrides` with a
  `Keep` value beside `Overwrite`; the retry batch carries both).
- **Wire.** `ManagedSkill.health: SkillHealthDto | null` = `{ kind, error: CommandError, at }`. `getManagedSkills`
  and every `{ report, skills }` response carry it. Regenerate bindings; re-export through the shims.
- **Frontend minimum**: `skillPresentation.ts` gains a pure `skillHealth(skill)` stub returning the stored outcome
  (the full fold with severity + source line is ticket 09); `SkillCard` shows one plain text line
  ("Update failed · 2 h ago") under the Unlocatable strip so the data is visible before ticket 11 — no new styling.
- **Fixture parity**: `failures` seeds each kind; the fixture's refresh/update/repoint/delete/import handlers write and
  clear the same way; `backend.test.ts` asserts the invariants.
- **Tests**: each writer records the right kind with the row's `CommandError`; the success family clears; a
  declined pair is `disabled`; a failed first sync is an `error` row; a `TARGET_EXISTS` first sync with no decision
  is *not* recorded (it is asked first).
- **Docs.** CONTEXT.md: *Skill health* (distinct from *target* health; the stored last outcome). CHANGELOG.

## Out of scope

Rendering (rows, chip, banner, ladder): tickets 09, 11, 12.

## Review

Astra @ high: every settlement site covered (grep the `last_error` writers), clear-family correctness, no
classification of an error that is then `?`-propagated (ADR-0001 round-15 amendment).

## Result

(child fills in)
