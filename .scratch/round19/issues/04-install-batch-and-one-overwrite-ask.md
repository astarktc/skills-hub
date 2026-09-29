# 04 — One install batch command for Add; `TARGET_EXISTS` consolidated across Add, re-assert and import

Status: implemented
Blocked by: 01
Spec: `.scratch/round19/spec.md` — D12, D20 (grill Q10, Q14). Source: `flows.md` Flow 1 gaps 2, 6; `critique-astra` F06.

## Why

The picker installs in a **frontend loop**, one candidate at a time, each raising its own overwrite ask under four
stacked dialogs; bytes have landed before the ask; Cancel does not stop the loop; Refresh re-assert and Onboarding
import collisions get no ask at all. AGENTS.md's "one batch command per operator action" holds for sync but not Add.

## Work

### Backend

- **`install_selection` batch command** (`core/install_batch.rs`, declared in `core/mod.rs`): input = source
  (`Git { url, resolution: GitSourceResolution }` from the listing, or `Local { root }`), `candidates: Vec<{ subpath,
  name_override? }>`, `tools: Vec<ToolId>`, `overwrite_if_same_content`, `overrides: Vec<{ skill_name, tool, decision:
  Overwrite | Keep }>`, `deploy: bool`. Per candidate under the guard: acquire → finalize → first sync through
  `global_sync`'s unlocked seam. Answers **one** report: per-candidate `Installed { skill } | Failed { error } |
  Cancelled`, and per (skill, Tool) target rows including `TargetExists` rows **collected, not asked** — the frontend
  raises one ask and calls **the same command again** with `overrides` for only the collided candidates (already
  installed candidates are re-sent as `already_installed` and skipped, or better: the retry uses
  `sync_skills_to_tools` with overrides, since the skills now exist — choose the one that keeps *one* ask and *one*
  report shape, and record the choice in the Result).
- **Cancel**: honours the `CancelToken` **between candidates**; remaining candidates answer `Cancelled` (ticket 03's
  variant). Progress over a `Channel` ("Adding (i/n) name").
- `install_git_selection` / `install_local_selection` are removed from `collect_commands!` when the frontend no longer
  calls them (ticket 16 does the frontend; **this ticket keeps them registered** and adds the batch beside them so
  the build stays green — note it in the Result so 16 deletes them).
- **Re-assert collisions**: `refresh.rs`'s `reassert_auto_sync` already reports `TARGET_EXISTS` rows as failures with no
  ask; classify them so the report row is distinguishable as **askable** (the row carries the typed `TARGET_EXISTS`
  error already — confirm the frontend can select exactly those rows and retry them through `sync_skills_to_tools`
  with per-pair overrides; if the report lacks the pair identity, add it).
- **Import divergent siblings**: `onboarding_import` reports them today as both a warning and a `TARGET_EXISTS`
  failure (`flows.md` Flow 3 gap 7). One row, `TARGET_EXISTS`, askable the same way; the double report goes.
- Every `TARGET_EXISTS` row carries what the ask displays: `target_path`, the existing folder's `modified_at`, and
  `same_content: bool` (the frontend never re-derives it).

### Frontend minimum

- `useOverwriteConfirmation` learns a list shape (several pairs, per-row Keep/Overwrite, one confirm) while keeping
  its `request()`-resolves pattern; the existing single-pair callers keep working. The Add flow itself is rewired in
  ticket 16 — **do not** rewrite `useAddSkillFlow`/`useCandidatePick` here beyond what the build needs.
- `useSyncOrchestration.syncSkillsToTools` passes `Keep` decisions through (ticket 02's `Keep` override) so declined
  pairs record as disabled.

### Fixture

- `install_selection` handler with the same collision behaviour; the `failures` scenario seeds two colliding
  candidates in one repo; cancelled tail producible with `latency=3`.

### Tests

- Batch of three with one collision → one report, two installed, one `TargetExists` row, no bytes written to the
  collided target; retry with `Overwrite` replaces; retry with `Keep` yields a `disabled` row (via 02); cancel after
  candidate 1 → rows 2–3 `Cancelled`; re-assert collision row is askable; import divergent sibling reported once.

### Docs

AGENTS.md § Invariants "Target fan-out is backend-owned" gains the install batch bullet; CONTEXT.md if a term moves.

## Review

Astra @ high: guard ordering per candidate, no partial-finalize leak on cancel, the retry path's idempotence.

## Result

Implemented 2026-09-29 (child, Opus 5.5), shared tree with ticket 09 (coordinated over intercom: 09 routed Enable
through the new seam in `useSkillActions`, switched the add-flow publish sites to `applyOutcome`, and added
`installSelection` to `CANCELLABLE_COMMANDS` / `cancel: "installSelection"` on the three install actions).

### Backend

- **`core/install_batch.rs`** (new, `core/mod.rs`) + command **`install_selection(request, onProgress)`**
  (`commands/mod.rs`, registered beside `install_git_selection` / `install_local_selection` — **ticket 16 deletes
  those two** once the Add surface stops calling nothing else; today no frontend path calls them).
  - Input `InstallRequest { source: InstallSource::{ git { url, resolution }, local { root } }, candidates:
    [{ subpath, name_override }], tools, overwrite_if_same_content, overrides: [{ skill_name, tool, decision }],
    deploy }` (overrides keyed by skill *name* — no id before install; applied in the first pass).
  - Per candidate: **acquire outside the guard** (`installer::stage_git_selection` / `stage_local_selection` →
    `StagedInstall`, a `StagingDir` RAII — git bytes still only through `git_acquisition::acquire`), then
    **finalize + first sync in one `mutation_guard::serialized` section** via `sync_skills_to_tools_unlocked`
    (same-content policy + the request's decisions). `installer.rs` was split into stage + finalize; the old doors
    call both halves (behaviour unchanged).
  - Report `InstallBatchReport { candidates: [{ subpath, name, status }] }`, `status` = `installed { skill:
    InstallResult, targets: BatchTargetOutcome[] }` | `failed { error }` | `cancelled` (own unit variant, wire
    `{ "status": "cancelled" }`, maps 1:1 onto 03's `InstallSettlement`). `InstallResult` now crosses as itself.
  - **Cancel**: checked before each candidate and again after its acquisition (staged bytes dropped → no partial
    finalize); an acquisition that fails with `CANCELLED` is `cancelled`, not `failed`; a finalized candidate always
    completes its sync. Progress `InstallProgressDto { index, total, name }` → "Adding (i/n) name".
- **Ask facts on every `TARGET_EXISTS`**: `sync_engine::TargetExistsError` gains `occupant: Occupant { modified_at,
  same_content }`, observed where the collision is raised (`Occupant::observe` — mtime + `content_identity::
  same_content(Directory(source), target)`), so every producer carries it by construction. `GlobalSyncError::
  TargetExists` and `CommandError::TargetExists { path, modifiedAt, sameContent }` carry it (global batch, install,
  re-assert, import, Enable, project assign/resync/Enable/Propagation).
- **Retry doors**: `enable_skill_targets(scope, overrides)` (core `enable_targets_with`; `enable_targets` kept as the
  test door) — Keep → `Declined` + `disabled` marker, Overwrite replaces; **`resolve_project_collisions(decisions:
  AssignmentDecision[])`** (`project_sync::resolve_collisions`, guarded entry point) — Overwrite re-syncs with
  overwrite, Keep → row `disabled` (owning nothing) reported new `ProjectSyncOutcomeStatus::Declined`; a gone /
  disabled / owning row answers `Failed NotFound` / `Skipped` / `AlreadyAssigned`.
- **Import double report gone**: a divergent sibling sitting at a target's path is reported once, as that pair's
  `TARGET_EXISTS` row (dropped from `originals`); a divergent copy in a non-target Tool stays `kept_divergent`.
- Re-assert rows needed no pair-identity change (skill from the outcome, Tool from `PropagationScope::Global`).

### Retry decision (recorded)

**The ask's retry is `sync_skills_to_tools` with per-pair overrides**, not a second `install_selection`: after the
first pass every candidate that can collide is already a Managed skill, so the retry is an ordinary sync batch over
the asked skills × Tools; its rows replace the asked ones *inside the install report* (`mergeInstallRetry`), so the
action keeps **one ask and one report shape** and the retry is idempotent (a re-sync of an already-synced pair is a
no-op link check). Enable retries through the same scope + overrides; project rows through
`resolve_project_collisions`.

### Frontend

- `src/lib/overwriteAsk.ts` (new, pure): `OverwriteRow { skillId, skillName, toolKey, toolLabel, path, modifiedAt,
  sameContent, scope: {kind:'global'} | {kind:'project', projectId} }`, `OverwriteChoice`, row builders
  (`globalCollisionRows`, `projectCollisionRows`, `installTargets`), `globalOverrides`, merges (`mergeGlobalRetry`,
  `mergeInstallRetry`, `mergeProjectRetry`), `overwriteRowKey`.
- **Ask data shape** (`useSurfaceState.ts`): `AskRequest { kind: 'overwrite', rows: OverwriteRow[] }`; answers are
  typed per kind — `AskAnswerOf<'overwrite'> = 'confirmed' | 'declined' | 'dismissed' | { choices:
  OverwriteChoice[] }` (row order); `confirmed`/`declined` = all overwrite / all keep. `ask()` is generic.
- `useSyncOrchestration`: `requestOverwriteConfirmation(rows) → OverwriteChoice[] | 'dismissed'`;
  `resolveGlobalCollisions(report, skills, policy)` (the ask + retry half of `syncSkillsToTools`, now per-row);
  `settleProjectCollisions(report) → { report, resolved }`; `enableSkillTargets(scope) → EnableReport` (one ask over
  global + project rows). Dismissed → nothing sent.
- `useAddSkillFlow`: `installBatch(source, picks)` — one `installSelection`, one ask over every candidate's collisions,
  retry, merge, `installBatchSettlements` → `installOutcome` (fold untouched except the new adapter
  `installBatchSettlements(report, { deploy, tools })` in `reportOutcome.ts`); a thrown retry is reported and the
  batch settles as first answered. Single git/local installs go through the same batch. `useCandidatePick`: the
  source adapter is `installSource(ctx, picked)`, the dep `installBatch` (no per-candidate loop, no `deploy`).
- Project world: `useProjectState(deps?: { settleCollisions })` — toggle-on / assign / resync / resync-all pass their
  report through it and re-read the view when a retry ran; App: `useProjectState({ settleCollisions:
  sync.settleProjectCollisions })`. `projectSyncOutcome` folds `declined` as a quiet note (was counted failed).
- `OverwriteModal`: rows show skill → Tool, path, "Modified 3d ago · differs from the skill"; >1 row gets a per-row
  Keep/Overwrite `Segmented`; footer "Keep existing" (all keep) + "Overwrite" / "Apply choices" (mixed). i18n
  `installBatch.step`, `overwriteAsk.*` (end of `en.translation`).

### Fixture

`installSelection` (latency per candidate: git 650 ms / local 400 ms × `latency`; cancel between candidates and after
acquisition), `resolveProjectCollisions`, `enableSkillTargets(scope, overrides)`, every `TARGET_EXISTS` carries
`modifiedAt` (3 days before the clock) + `sameContent`. `failures` seeds repo **`acme/collide-kit`**
(lint-staged-rules · commit-helper · changelog-writer) with Claude Code occupants for two of them. Round-18 gap
(prisma-review → Cursor/Codex re-assert) verified present and now tested. Walked on 5175 `?scenario=failures`: Add →
collide-kit → Install selected → **one ask, two rows** with facts; Keep row 2 + Apply → "3 installed · 3 targets failed
· 1 kept existing" (+ note "changelog-writer: kept your existing Claude Code folder"; the 3 failures are the seeded
unwritable Amp/Kimi dir). Screenshot: `/Users/alexstark/.t3/userdata/browser-artifacts/browser-screenshot-localhost-mumyrurk-be9f2f3c.png`.

### Tests

- Rust `core/tests/install_batch.rs` (9): `a_batch_of_three_with_one_collision_answers_one_report_and_writes_nothing_through_it`,
  `retrying_the_collision_with_overwrite_replaces_the_folder`, `retrying_the_collision_with_keep_records_the_pair_disabled`,
  `a_request_override_settles_the_collision_in_the_first_pass`,
  `cancel_during_the_second_candidate_reports_the_rest_cancelled_and_finalizes_nothing_of_them`,
  `a_failed_candidate_is_report_data_and_the_rest_proceed`, `deploy_off_installs_without_targets`,
  `a_git_source_installs_through_the_listing_resolution`, `the_report_crosses_the_wire_with_cancelled_as_its_own_kind`.
  `core/tests/skill_disable.rs`: `an_enable_collision_is_askable_and_its_overwrite_retry_replaces_the_folder`,
  `an_enable_collision_kept_stays_disabled_and_reads_declined`, `a_project_collision_resolves_with_overwrite_or_keep`,
  `a_reassert_collision_row_is_askable_and_retried_through_the_sync_batch`. Updated: `onboarding_import`
  `auto_sync_on_keeps_a_sibling_that_diverged_after_planning` (reported once, with facts), `global_sync` occupant
  assertions, `skill_health` project collision carries facts.
- Vitest: `lib/overwriteAsk.test.ts` (4); `fixtures/backend.test.ts` "ticket 04" (8: one report / two askable rows,
  retry Overwrite+Keep, request override, cancel tail, failed candidate, re-assert facts, Enable retry, project
  Overwrite/Keep); `useSyncOrchestration.test.ts` "the one overwrite ask across producers" (4: per-row choices,
  project ask + resolve, dismissed project ask, Enable global+project one ask); `useAddSkillFlow.test.ts` (one batch
  request shapes, one ask per Add batch, thrown retry, refusal as a report row); `useCandidatePick.test.ts` (one batch
  call, cancelled tail folded); `useSurfaceState.test.ts` (per-row answer); `reportOutcome.test.ts` (settlements adapter,
  project declined).

### Gate

`npm run version:check` → Version OK (1.2.18). `npm run check` → exit 0 (lint, styles:check, vitest 32 files / 690
tests, build, rustfmt, clippy `-D warnings`, `cargo test` 789 passed; bindings regenerated).

### For 16 (Add surface)

Delete `install_git_selection` / `install_local_selection` (+ `InstallResultDto`, the fixture handlers, `S6` fixture
test) from `collect_commands!`; build on `useAddSkillFlow.installBatch` + `installBatchSettlements` for the stepped
surface (per-candidate rows are already in the report: installed rows ticked off, failed rows retryable by re-sending
those candidates). The ask is already the only nested layer. Ticket 21 decides whether Import review's Replace raises
this ask (the import report's `replaced` `TARGET_EXISTS` rows are askable via `resolveGlobalCollisions`; the import
hook does not route them yet — out of this ticket's minimum). `confirm-dialog.tsx` / ADR-0006 l.98 still name the
deleted `useOverwriteConfirmation` (ui/ and docs out of bounds here).

## Comments

- 2026-09-27 — round-18 extract: the fixture does not produce `TARGET_EXISTS` on re-assert for some seeded occupied targets (`prisma-review` → Cursor/Codex; `round18/review/flows.md` § Fixture gaps) — fix under "Fixture" here.
- 2026-09-29 — from ticket 02: Enable's `TARGET_EXISTS` still bypasses the overwrite ask, and project sync (assign/resync/Enable) has no Keep/Overwrite choice — route both into the consolidated ask here. Overrides now carry `decision: "overwrite" | "keep"` (02); a dismissed ask must send nothing (08 adds the third answer).
