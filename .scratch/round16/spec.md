# Round 16 — close the backlog: project-sync report, cross-kind Re-point, bulk unassign → 1.2.16

Status: open
Opened: 2026-09-22

Absorbs every remaining BACKLOG line (`Later` #12 #15 #16 #17 #18 #35 #36; `Parked` #21–#30) — each leaves
BACKLOG.md in the opening commit with its disposition recorded here (§ Dispositions). Two operator additions
join the round: **bulk unassign** (inverse of bulk assign) and **cross-kind Re-point**. Five operator product
ideas are recorded as BACKLOG **Future efforts** #37–#41 (not this round's work).

Baseline: 1.2.15 (`a3cff96`) — release `v1.2.15` published with all five targets, `.sig`s, `updater.json`
(`release.yml` run on tag `v1.2.15` green). Main @ `2145472`.

## Problem (verified against `main` @ `2145472`)

1. **The project world still has two hand-shaped result types** after round 15 made every other fan-out cross
   the wire as its core report:
   - `ToggleOutcome::Assigned` carries nothing (`project_sync.rs:619`); the command answers `report: None`
     (`commands/projects.rs:215`). `assign_skill_to_tools` keeps a sync failure *inside the record* (status
     `error`) and still reports `Assigned`, so toggle-on silently paints a red cell with no toast — while
     toggle-off folds a `RemovalReport` and toasts. (BACKLOG #35)
   - `bulk_assign_skill` maps `AssignTargetStatus` by hand into `BulkAssignResultDto { view, failed:
     Vec<BulkAssignErrorDto> }` and classifies with `CommandError::from_anyhow` *at the seam*
     (`commands/projects.rs:303–347`) — the timing ADR-0001's round-15 amendment retired.
   - `ResyncSummary { synced, failed, errors: Vec<String> }` (`project_sync.rs:345`) renders each failure as
     `"{assignment_id}: {chain}"` — the last prose-on-wire field; `AssignmentMatrix.tsx:110–158` sums counters
     itself instead of using the fold. (BACKLOG #36)
2. **Bulk assign has no inverse.** One click fans a skill out to every configured project Tool
   (`AssignmentMatrix.tsx:438`), but clearing it is one toggle per Tool. No `RemovalScope` covers "one skill ×
   every Tool of one project" (`artifact_removal.rs:63–99`).
3. **Re-point cannot cross provenance.** `repoint_local_skill_source` requires `local` (`unlocatable.rs:172`
   `require_local`) and is only *offered* for the `source_missing` repair; `repoint_git_skill_source` bails
   `GitRepointRequiresGit` (`refresh.rs:228`). Both already run the single-Update pipeline with the new source
   carried only in the acquired record — the guards are the only thing preventing `git ↔ local` and
   `imported → {git, local}`. The operator wants "point this managed skill at a folder I maintain" and "point
   this folder skill at its GitHub home".
4. Small residue: three hand-simulated `bulk_assign_*` tests coexist with the engine suite
   (`core/tests/project_sync.rs:702/768/821` vs `:1057–1179`, #12); `GitSelection.subpath: Option<&str>` lets
   a future caller hand `acquire` a listing-only intent (`git_acquisition.rs:88`, `resolution.rs:62`, #15); the
   Add flow lists a Tool-dir folder as a valid local candidate and refuses it only on Install
   (`installer.rs:54` vs `:296`, #22); the progress-channel factory is named like state (`useSkillLibrary.ts:131`
   `refreshProgress`, #28).

## Goal

The project world answers every mutation with a core report the fold understands; bulk assign has a
one-click inverse; Re-point is one operation over one target enum for every managed skill; the four residue
items close. Released as 1.2.16. BACKLOG leaves the round with only **Future efforts**.

## Decisions (all accepted by the operator 2026-09-22, as recommended)

- **D1 One `ProjectSyncReport`, three producers.** Core `project_sync::ProjectSyncReport { items:
  Vec<ProjectSyncOutcome> }`, `ProjectSyncOutcome { assignment_id: Option<String>, skill_id, skill_name, tool,
  status: ProjectSyncStatus }`, `#[serde(tag = "status", rename_all = "snake_case")] enum ProjectSyncStatus {
  Synced, AlreadyAssigned, Failed { error: CommandError } }`. Derives `Serialize + specta::Type`. Classified
  **where the row settles** (after `last_error` is written), never at the seam. Produced by:
  toggle-on (`ToggleOutcome::Assigned { report }`, a batch of one), `bulk_assign_skill`, `resync_project`,
  `resync_all_projects`. `ResyncSummary`, `BulkAssignResultDto.failed`, `BulkAssignErrorDto` and the
  `synced/failed/errors` counters leave the wire; counters are derived in `src/lib/reportOutcome.ts`.
  `AssignTargetOutcome`/`AssignTargetStatus` (the internal fan-out shape with `anyhow::Error`) either becomes the
  report item or is settled into one — the lane chooses; no `anyhow::Error` crosses.
  *Alternative rejected:* two narrower fixes (`sync_status` on `Assigned`; typed `ResyncSummary.errors`) — same
  work, three shapes survive.
- **D2 Bulk unassign.** `RemovalScope::ProjectSkill { project_id, skill_id }` (every assignment row of one
  skill in one project), planned/executed/settled by `artifact_removal` exactly like `ProjectSkillTool` (row kept
  with status `error` on failed removal, ADR-0002). Command `bulk_unassign_skill(projectId, skillId)` →
  `{ view: ProjectViewDto, report: RemovalReport }`, entry point under `mutation_guard::serialized`. UI: **no
  confirmation** (symmetric with bulk assign; a removed link is re-assignable, not data loss); button shown when
  the skill has **≥1 assignment** in the project; bulk-assign hidden when the row is saturated. Folded by
  `removalOutcome` with a new action key.
- **D3 Re-point is one operation over a target enum.** Core `repoint_skill_source(paths, store, skill_id,
  target: RepointTarget, policy, cancel, now, on_progress) -> RefreshReport` with
  `enum RepointTarget { Git { url: String }, Local { path: PathBuf } }` — the git arm is today's
  `refresh::repoint_git_skill_with` body, the local arm today's `unlocatable::validated_local_repoint` +
  `UpdateRequest::local(…, repoint = true)`. All six transitions `{git, local, imported} → {git, local}` are
  defined; `→ imported` stays `detach_skill_from_source`. Commands `repoint_local_skill_source` and
  `repoint_git_skill_source` are **retired** (leave `collect_commands!`; bindings regenerate) and replaced by one
  `repoint_skill_source(skillId, target: RepointTargetDto, policy)`. `SignalError::GitRepointRequiresGit` /
  `CommandError::GIT_REPOINT_REQUIRES_GIT` and `require_local` are retired with their `describeCommandError`
  branch and i18n keys (the condition no longer exists). Folder validation stays what Add uses (present,
  `SKILL.md`, not inside a Tool's skills dir → `LocalSourceInsideToolDir`); URL validation stays what git
  Re-point uses.
- **D4 Provenance on crossing.** `→ Local`: `source_type = local`, `source_ref = folder`, `source_subpath`
  and `source_revision` cleared. `→ Git`: acquisition records `source_ref`/`source_subpath`/`source_revision`
  as Add does. `imported → *`: the skill **stops being imported** — it gains an external source, becomes
  refreshable, and `imported_from_tool` is cleared. Edit V1 bases replay onto the new bytes inside finalize as
  on any Update (conflicts are report data). The new source is carried only in the acquired record — a failed
  acquisition/finalize changes nothing (unchanged rule). ADR-0003 gets an amendment: *imported is a provenance a
  skill can leave by Re-point; it is re-entered only by Detach.*
- **D5 Change source UI.** One **"Change source…"** action on every managed card (git, local, imported),
  opening one modal with a kind picker (GitHub URL / Local folder), current kind preselected; the
  `source_missing` repair badge opens the same modal with Local preselected. `gitRepointSelection` →
  `repointSelection`; `canRepoint` is true for every managed skill. `GitRepointModal` is the base; the local
  arm reuses the folder picker the Unlocatable path already uses (`@tauri-apps/plugin-dialog` `open`).
- **D6 #22 at listing.** `list_local_skills` marks a candidate whose folder is inside a Tool's skills
  directory `valid: false, reason: "inside_tool_dir"` (same predicate as Install: `tool_holding_path`). The
  command resolves `home` at the seam (core never resolves roots). `LocalPickModal` shows it disabled with
  the mapped reason (a hidden candidate would read as "where did my skill go?").
- **D7 #12** — diff the three `bulk_assign_*` tests against the `fanout_*` suite; delete what is covered, port
  the rest; the ticket records the assertion→test mapping.
- **D8 #15** — `SkillIntent::Selection { subpath: &str, resolution }` non-optional; the listing path builds its
  own private intent; compiler-enforced, no runtime refusal, no new error variant.
- **D9 #28** — rename `refreshProgress` → `newRefreshProgressChannel` (factory, not state). The "decouple
  `update_managed_skill`" half is dropped: AGENTS documents "a single Update is a batch of one" on purpose.
- **D10 Lanes.** Opus 5.5 on Pi (`anthropic/claude-opus-5-5`, thinking medium) implements in worktrees under
  `.scratch/round16/worktrees/` (gitignored): backend-A (01) ∥ backend-B (02), then frontend-A (03) ∥ frontend-B
  (04). The orchestrator merges (rebase, `git diff main...HEAD` deletion review, full gate after each). Known
  collision set: `lib.rs` `collect_commands!`, `i18n/resources.ts` (additive), `src/bindings/index.ts`
  (regenerate, never hand-merge). One Astra review (`openai-codex/gpt-6-astra`, high) over the whole round,
  fix-then-ship.

## Dispositions of absorbed BACKLOG lines

| # | Disposition | Evidence |
|---|---|---|
| #12 | **ticket 05** | `core/tests/project_sync.rs:702/768/821` vs `:1057–1179` |
| #15 | **ticket 06** | `git_acquisition.rs:88`, `resolution.rs:62` |
| #16 | **dropped** — startup removes `.explore-cache` wholesale (`src-tauri/src/lib.rs:179–183`); in-session publication is atomic by rename; the only survivor is a warn-logged failed startup wipe, accepted | |
| #17 | **dropped** — no Cursor on any operator machine; Cursor's own documentation states symlinked skill dirs are discovered since 2.5 (`archive/v-next/assets/research-cursor-symlinks.md`); `supports_symlink` stays the revert lever | |
| #18 | **ticket 08** (facet a: our junction leg has never executed — CI is `ubuntu-latest` only, `try_junction` is reached by no test); facet b (Cursor discovery through a junction) **dropped** with #17's reasoning | `sync_engine.rs:63`, `.github/workflows/ci.yml:17,33` |
| #21 | **dropped** — per-row `last_error` is the durable record; a persisted toast log duplicates it with less structure | |
| #22 | **tickets 01 + 03** (D6) | `installer.rs:54,296` |
| #23 | **dropped** — not hit in practice; a real feature (prefix rewrite + batch) deserves its own effort if demand shows | |
| #24 | **dropped** — explicit round-4 exclusion, no new evidence | |
| #25 | **dropped** — accepted limitation already recorded in CONTEXT.md **Acquisition** | |
| #26 | **kept** as BACKLOG Future efforts | |
| #27 | **dropped** — round 15 made the three unsync commands uniform (all return `RemovalReport`); the residue is three ~10-line wrappers and the distinct toasts remain the reason to keep them | |
| #28 | rename → **ticket 03** (D9); decouple half **dropped** | `useSkillLibrary.ts:131` |
| #29 | **dropped** — `content_identity::record` doc + 2-line body already say it (`content_identity.rs:22–26`) | |
| #30 | **superseded** by BACKLOG Future efforts #37–#41 (operator's ideas recorded 2026-09-22) | |
| #35 | **tickets 01 + 03** (D1) | `project_sync.rs:619`, `commands/projects.rs:215` |
| #36 | **tickets 01 + 03** (D1) | `project_sync.rs:345`, `AssignmentMatrix.tsx:110–158` |

## Tickets

| # | Lane | Scope |
|---|---|---|
| 01 | backend-A (Opus 5.5) | D1 `ProjectSyncReport`; D2 scope + `bulk_unassign_skill`; D6 listing reason; bindings |
| 02 | backend-B (Opus 5.5) | D3 `repoint_skill_source` + `RepointTarget`; D4 provenance rules; old commands + error variant retired; bindings |
| 03 | frontend-A (Opus 5.5, after 01) | fold for `ProjectSyncReport`; `AssignmentMatrix` via the fold; bulk unassign button; D6 disabled candidate; D9 rename; i18n |
| 04 | frontend-B (Opus 5.5, after 02) | D5 Change source modal; hook state; card affordance; i18n |
| 05 | orchestrator | D7 #12 |
| 06 | orchestrator | D8 #15 |
| 07 | orchestrator | CHANGELOG; CONTEXT.md; AGENTS.md; ADR-0003 amendment; version 1.2.16; release |
| 08 | operator, post-release | #18 facet (a): Windows junction smoke on the 1.2.16 build |

Review: `.scratch/round16/review/astra-review.md`, fix-then-ship; fixes as ticket 09 if needed.

## Closure checklist

- `npm run version:check && npm run check` green; `cargo test --all`; bindings committed with no drift.
- Operator smoke (installed 1.2.16): toggle-on toast; bulk assign / bulk unassign toasts; resync toast via the
  fold; Change source on a git skill → local folder and back; Change source on an imported skill → GitHub URL
  (skill becomes refreshable); Add flow lists a `~/.claude/skills/*` folder disabled with a reason.
- Ticket 08 evidence pasted; BACKLOG `Now`/`Later`/`Parked` empty; Future efforts #26 #37–#41; next free #42.
