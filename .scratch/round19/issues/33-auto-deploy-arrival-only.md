# 33 — Auto-deploy is arrival-only: the Refresh re-assert is deleted, `delete_incomplete` is skipped as report data

Status: claimed (audit F1–F3 fixed; Astra re-check pending)
Blocked by: —
Blocks: 32 (vocabulary rewrites the strings this ticket touches last)
Source: BACKLOG #71 (absorbed here 2026-11-02 — grill D34, D35; `round19/issues/18-operator-think-aloud.md` § Findings
P1·Q3, P2·Q4, P2·Q6, P2·Q10, debrief 4). Spec: D34, D35, D45 (vocabulary — read the new words, write the old ones;
ticket 32 renames).

## Problem

With auto-sync on, every Update / Refresh all / Restore / Re-point also **re-asserts** an invariant — every Managed
skill on every Tool of the effective global target set — through `core/refresh.rs::reassert_auto_sync_unlocked`
(`RefreshPolicy::reassert_auto_sync`, `RefreshPolicyDto` at `commands/mod.rs` ~:600, passed by `useSkillActions.ts`
~:543/560/583/615 from `autoSyncEnabled`). The operator reads a hollow strip mark as *his* choice ("clearly that's the
only tool we synced to"); the next Refresh then deploys the skill to seven more Tools, raises an overwrite ask about
folders Skills Hub never deployed, and on the think-aloud fixture added 16 failed targets on an unwritable shared dir.
Add's per-skill Tool scope ("Edit Tools" in its summary) is silently undone by it: a skill added to Claude Code only
gets the other Tools at the next Refresh, with no `disabled` rows for the ones the operator left out. He concluded
"disabled and not synced is kind of the same thing … redundant". It also spread a skill whose Delete was incomplete
(release-train) to two more Tools. Fact: the operator's real library runs auto-sync **off** — the re-assert only ever
ran on the fixture.

**Decision (D34):** auto-deploy decides where a **new** skill lands — Add, Onboarding import, Enable's first deploy —
and nothing else. Update paths only propagate to deployments that already exist. "Not deployed" is a durable *never
put here*; Disabled is the durable *taken off*; Deploy / Deploy all / Deploy this fill a hollow pair and skip a
disabled one. **(D35):** a skill whose health is `delete_incomplete` is never updated — every update path reports it
skipped with a typed reason; the health row's Retry / Reveal is the only way forward.

## Work

Backend (`src-tauri/`):

- **Delete the re-assert.** Remove `reassert_auto_sync_unlocked`, `RefreshPolicy` (its only field) and every
  `reassert_error` slot on `SkillRefreshStatus::Refreshed` / the Propagation outcome merge (`refresh.rs` ~:21, ~:71,
  ~:115, ~:472–480, ~:553–600). `repoint.rs`, `skill_disable.rs`, `onboarding_import.rs`, `sync_status.rs`,
  `skill_store.rs` carry comments or calls naming it (`grep -rn 'reassert' src-tauri/src`) — each is either deleted
  or reworded to the arrival-only rule; none keeps a code path.
- **Drop the policy from the command seam.** `refresh_managed_skills`, `update_managed_skill`, `repoint_skill_source`
  lose their `policy: RefreshPolicyDto` parameter; delete the DTO. `cargo test` regenerates `src/bindings/index.ts`
  (commit it). Commands stay wiring only.
- **`delete_incomplete` skip (D35).** `select_skills` (`refresh.rs` ~:416) already sets an Unlocatable member aside as
  `SkillRefreshStatus::Skipped { state }`; add the health gate beside it as its **own typed reason**: widen the
  existing skip to a reason enum (`RefreshSkipReason::{Unlocatable(state), DeleteIncomplete}`, serde-tagged, specta
  exported) rather than a second status variant, and rename nothing else. The skip applies to **both**
  `RefreshSelection::All` and `Ids` (unlike Unlocatable, where a named id proceeds: there is no Restore-like repair
  for an incomplete delete — Retry delete is). The gate reads `skills.last_outcome_kind == delete_incomplete` through
  `skill_health`'s read seam, never a string compare at the call site. Re-point of such a skill: refused the same way
  (it is a batch of one), as the typed skip in its report.
- **Tests** (`core/tests/refresh.rs` ~24 `reassert` mentions, `tests/repoint.rs` 13, `tests/skill_disable.rs` 12,
  `commands/tests/mutation_results.rs` 4): delete every test whose subject is the re-assert; replace with (1) Refresh all
  with auto-sync on and a skill on 1 of 3 Tools → the report carries **no** global target for the other two and no row
  is created; (2) a `delete_incomplete` skill in a Refresh all and in a named Update → `skipped` with the typed reason,
  no acquisition, no target touched, health unchanged; (3) Re-point of a `delete_incomplete` skill → the same skip.
  Keep the Disabled-skip tests that don't depend on the re-assert.

Frontend:

- `useSkillActions.ts` (~:543, 560, 583, 615) and `useSyncOrchestration.ts` (4 mentions): drop the policy argument
  and the `autoSyncEnabled` plumbing that only fed it (keep what Add / Enable still read). `App.tsx` (1 mention).
- `src/lib/reportOutcome.ts` (~:390–460): delete the "also deployed to" announcement and its i18n key
  (`runReport.alsoDeployed`, `resources.ts` ~:1133) — nothing produces it now; `reportOutcome.test.ts` (11 mentions),
  `overwriteAsk.ts` / `.test.ts` (the re-assert collision shape — a `TARGET_EXISTS` from Propagation's first sync of an
  `error` row that owns nothing can still occur: keep that path, delete only the re-assert framing),
  `useSkillActions.test.ts` (7).
- **New skip reason in the fold**: the run item for `delete_incomplete` reads "Delete incomplete — finish removing
  {{name}} first" with the health row's repairs (Retry, Reveal); append its keys at the end of `en`.
- **Fixture** (`src/fixtures/backend.ts` ~:1058–1093 mirrors the re-assert; `model.ts`, `scenarios.ts`,
  `backend.test.ts` 33 mentions): delete the mirror, mirror the `delete_incomplete` skip, keep every scenario's seed
  valid (the `failures` scenario's release-train skill is the `delete_incomplete` case — it must now be skipped by
  Refresh all).
- **First-deploy failure copy.** `errors.propagationFailedTitle` ("Could not update the synced copy") was reached by
  re-assert first deploys. After deletion, verify whether Propagation can still *first-deploy* (an `error` row that
  never materialised gets a first sync — `propagation.rs`): if it can, the Propagation outcome carries whether the row
  owned an artifact before (`synced_at` set) and the fold reads "Could not deploy {{name}} to {{tool}}" for a first
  deploy, keeping the "update the synced copy" title only for an owned copy. If it cannot, write that down in
  `## Result` and change nothing.
- **Settings copy**: the auto-sync description (`resources.ts`: "Deploy added, imported and updated skills to your
  selected Tools automatically") drops "and updated" — this ticket; ticket 32 renames the rest.

Docs, targeted lines only (the orchestrator reviews them):

- `CONTEXT.md`: `Refresh (all)` entry — replace the "With auto-sync on it also re-asserts…" sentence with the
  arrival-only rule and the `delete_incomplete` skip; `Disabled` entry — drop "the Refresh re-assert" from the list of
  passes; `Overwrite ask` entry — drop "a Refresh's re-assert" from the producers (4 mentions total).
- `AGENTS.md` (5 mentions): the Update / Refresh bullet's "plus the `reassert_auto_sync` policy"; D12's producer list;
  the Disable bullet's "Propagation and the re-assert"; the Skill-grain reassert mention in the Re-point sentence.
  Reword each to the new rule; add nothing else.

## Proof

- `cd src-tauri && cargo test --all`: the three new tests green; `grep -rn 'reassert' src-tauri/src src` returns
  only comments that describe the *deleted* behaviour as history, or nothing.
- `npm run test`: folds and fixture green; `resources.test.ts` green (new keys present, `alsoDeployed` gone).
- Fixture, `?scenario=rich` (auto-sync on): Refresh all → every skill's strip is unchanged (no new deployment, no
  overwrite ask); the run headline has no "also deployed" item. `?scenario=failures`: Refresh all → release-train
  (delete incomplete) is a skipped item with its reason and its Retry; its targets untouched.
- `npm run version:check && npm run check` green.

## Result

Implemented uncommitted on top of `1cb3d36` (the orchestrator commits). Gate, fresh: `npm run version:check` →
"Version OK (1.3.0)"; lint clean; `styles:check ok`; vitest **49 files / 1043 tests passed**; `npm run build` built;
`rust:fmt:check` clean; `rust:clippy` (`-D warnings`) clean; `cargo test --all` **830 passed, 0 failed**.
`grep -rni 'reassert\|re-assert' src src-tauri/src` → one history line only (`core/refresh.rs:24`, "Before D34 the
apply phase also "re-asserted" every skill…").

**Backend.** `core/refresh.rs`: `RefreshPolicy`, `reassert_auto_sync_unlocked`, `merge_reassert` and
`Refreshed.reassert_error` deleted; the `now` parameter went with them (only the re-assert used it) from
`refresh_managed_skills{,_with}`, `refresh_with_intent`, `repoint_skill_source{,_with}`. `Skipped { state }` widened
to `Skipped { reason: RefreshSkipReason }`, `#[serde(tag = "kind")]` — `{ kind: "unlocatable", state }` /
`{ kind: "delete_incomplete" }`. `select_skills` reads `skill_health::all_skill_health` once and sets a
`DeleteIncomplete` skill aside for **both** `All` (checked before eligibility, imported non-members still unreported)
and `Ids`; Re-point (a batch of one) gets the same skip after target validation, before acquisition.
`skill_update::UpdateOutcome` lost `skill_id`/`name` (read only by the re-assert). `commands/mod.rs`:
`RefreshPolicyDto` and the `policy` parameter of `refresh_managed_skills` / `update_managed_skill` /
`repoint_skill_source` deleted; `src/bindings/index.ts` regenerated by `cargo test`. Comments reworded in
`skill_disable.rs`, `onboarding_import.rs`, `sync_status.rs`, `skill_store.rs`, `repoint.rs`, `commands/mod.rs`.

**First-deploy copy — finding: Propagation CAN still first-deploy.** `propagation.rs` gives a row that never
materialised (`!owns_artifact()`, `synced_at` unset — a failed first sync's `error` row) a first sync, in both the
global arm (`… || !member.owns_artifact()`) and the project arm. So `PropagationStatus::Failed` now carries
`first_deploy: bool` (= the row did not own an artifact before this pass, per row), and `reportOutcome.ts`'s
`propagationEntries` titles it "Could not deploy {{name}} to {{tool}}" (`propagationFirstDeploy.failedTitle`),
keeping "Could not update the synced copy" for an owned copy. `overwriteAsk.mergePropagationRetry` keeps the asked
row's `first_deploy` when its retry fails; the fixture mirrors the flag. Seen live in `?scenario=failures`:
"Could not deploy json-canvas to Amp" beside "Could not update the synced copy: tdd -> Amp".

**Frontend.** `useSkillActions.ts`: policy arguments, `autoSyncEnabled` from its `sync` Pick, and
`priorGlobalTools` deleted (Add / Enable / Settings still read `autoSyncEnabled` from the sync world).
`reportOutcome.ts`: the "also deployed" announcement, `priorGlobalTools` and the `reassert_error` count/entry deleted;
a `delete_incomplete` skip is a warning titled "Delete incomplete — finish removing {{name}} first" with
`repair: { verb: "delete" }`. `useSyncOrchestration.ts`, `overwriteAsk.ts`: re-assert framing removed, the
Propagation `TARGET_EXISTS` path kept. `components/skills/types.ts`: `RefreshPolicyDto` re-export dropped. `App.tsx`:
its one "re-asserted" (a focus comment, unrelated) reworded. i18n: `runReport.alsoDeployed` and
`errors.reassertFailedTitle` deleted; `settingsAutoSyncHint` → "Deploy added and imported skills to your selected
Tools automatically."; appended at the end of `en`: `updateSkip.deleteIncompleteTitle`,
`updateSkip.deleteIncompleteMessage`, `propagationFirstDeploy.failedTitle`.
**Fixture**: the `reassert`/`settleTargets` mirror deleted (`applyRefresh` is Propagation only), the
`delete_incomplete` skip mirrored in `refreshBatch` (both selections) and `repointSkillSource`; the `failures`
scenario's `store-hiccup` skill removed (its only purpose was the re-assert store failure) with model.ts's
`reassertError`.

**Tests.** New Rust: `core::refresh::tests::refresh_all_with_auto_sync_on_creates_no_deployment` (3 Tools
installed + selected, auto-sync on, skill on Claude Code only → report and rows carry `claude_code` only, no codex /
cursor artifact); `…::a_delete_incomplete_skill_is_skipped_by_refresh_all_and_by_a_named_update` (both selections:
`Skipped { DeleteIncomplete }`, zero acquisitions, targets / health / central bytes unchanged);
`core::repoint::tests::repoint_of_a_delete_incomplete_skill_is_skipped` (local and git targets: same skip, record,
targets, health, central untouched); `…::refresh_skip_reasons_cross_the_wire_tagged` (wire shape). First-deploy flag
asserted in `skill_health::a_failed_first_sync_records_an_error_row_that_owns_nothing` (`first_deploy: true`) and
`propagation::…` missing-central test (`false`). Deleted (re-assert subjects): refresh.rs's 4 `reassert_auto_sync_*`
/ `without_the_reassert_policy…` / 2 `merge_reassert` / `…excluded_from_the_reassert`; repoint.rs's
`git_repoint_honours_auto_sync_reassert_policy`, `local_repoint_honours_auto_sync_reassert_policy`; skill_disable.rs's
`a_reassert_collision_row_is_askable_and_retried_through_the_sync_batch`. Reworked to plain Refresh (kept):
`a_disabled_pair_survives_refresh_and_propagation`, `disabling_a_pair_with_no_row_records_a_marker_a_refresh_honours`,
`a_disabled_skill_is_never_rematerialised_by_any_automatic_pass`, the imported / Unlocatable Refresh-all tests, and
`update_and_restore_catalog_include_replayed_override_and_propagated_targets` (deploys first, then propagates).
New TS: `backend.test.ts` "D34: …" (Update / Refresh / Re-point of zod create no deployment and raise no
collision; Refresh all leaves every skill's deployments as they were; prisma deployed nowhere stays nowhere) and
"D35: …" (release-train skipped by Refresh all, named Update and Re-point; unchanged; fold item with the delete
repair); `reportOutcome.test.ts` "announces no deployment…", "a delete_incomplete skip is a skipped item…", "a
failed first deploy reads 'Could not deploy'…". Deleted: the `priorGlobalTools` hook test, the "also deployed" fold
test, the "reassert failure" table row, the fixture's S1 re-assert describe and its prisma collision test.

**Fixture proof** (`.scratch/round19/evidence/33/`). The running `:5175` server answered every page with a Vite
overlay — `Failed to resolve import "@/components/ui/sonner"` although the file exists (a stale server of another
session; left untouched) — so I ran the same `vite --mode fixture` on `:5176` and stopped it afterwards. `rich`
(auto-sync on): Refresh all → toast "60 refreshed", no overwrite ask, run reads "60 refreshed" with no "also
deployed" item; strip diff before/after (`rich-strips-{before,after}.json`, 60 rows) changes **one** row only —
using-git-worktrees' errored Amp / Kimi copies re-materialised by Propagation; every "not synced" pair (e.g. its
Gemini) stays not synced (`rich-01/02/03-*.png`). `failures`: Refresh all → release-train's strip identical before /
after; the run lists "Delete incomplete — finish removing release-train first" with Show + Remove… (the panel's label
for the `delete` repair); Remove… opens the Delete dialog, whose report offers Retry and Reveal in Finder per kept
path (`failures-01…06-*.png`).

**Docs — exact changes.** CONTEXT.md: Overwrite ask "(an Add, a sync, an Enable, a Refresh's re-assert, a project
assign or re-sync)" → "…a Refresh's Propagation…"; Disabled "Propagation, the Refresh re-assert, the global sync
batch" → "Propagation, the global sync batch"; Re-point "replaying Edits, propagating and honouring auto-sync
re-assert, so" → "replaying Edits and propagating to the targets that exist, so"; Refresh (all) "With auto-sync on it
also re-asserts the auto-sync invariant — … not just existing ones refreshed." → "Auto-sync plays no part: it decides
only where a new skill lands, so a Refresh only brings existing Sync targets up to date and never creates one (D34).
A skill whose last Delete was incomplete (`delete_incomplete`) is never updated — reported skipped with that reason,
whether the batch is all skills or names it (Update, Re-point); finishing the Delete is the way forward (D35)."
AGENTS.md: "after settlement/reassert" → "after settlement"; "(finalize + Edit replay + Propagation; ADR-0004), plus
the `reassert_auto_sync` policy." → "(finalize + Edit replay + Propagation; ADR-0004) — existing targets only, never a
new one (D34)."; D12 producers drop "the re-assert,"; the collect-first list "Propagation and the re-assert — and
Edit's Propagation" → "Propagation — and Edit's Propagation"; Disable bullet "Propagation and the re-assert
(`PropagationSkip::Disabled`)" → "Propagation (`PropagationSkip::Disabled`)".

**Deliberately left.** (1) "Reveal" is not a run-item action: the health row's only repair for `delete_incomplete`
is `delete` (`skillPresentation.ts`), and Reveal lives in the Delete dialog's report it opens — I mirrored that
rather than invent a reveal row action; the panel labels the action "Remove…" (its existing `delete` label), the
inspector "Finish removing". (2) CONTEXT.md lines 30/32 (the D45 vocabulary block) already describe the re-assert as
history — untouched. (3) `AGENTS.md` gained no D35 sentence (the brief said reword only). (4) The broken `:5175`
server is not mine to restart. (5) New strings use the old words (Tool, sync) except the ticket-given "Could not
deploy …".

## Comments

- 2026-11-02 (child, ticket 33): claimed and implemented; Result above. Not committed.

- 2026-11-02 — **done.** Orchestrator reviewed the diff (refresh / propagation / repoint / commands hunks, folds, strings, CONTEXT/AGENTS sentences) and re-ran the gate: version OK (1.3.0), vitest 1043, `cargo test --all` 830, clippy `-D warnings` clean, App.css 1108 / 36. Committed with this ticket. Astra audit dispatched on the commit.

- 2026-11-02 — **reopened by the Astra audit** (`round19/review/ticket-33-audit.md`, verdict FAIL): **F1 (P0)** an
  Update's confirmed overwrite retry goes through the global sync batch, whose shared-dir fan-out records a row for an
  installed sharer (Kimi beside Amp) that had none — reproduced on the fixture (`[amp]` → `[amp, kimi_cli]`).
  Propagation's own first sync records only existing rows, so the two writers of one shared dir disagreed; the sharer
  physically reads the artifact either way. **Decision (D34 amendment, spec):** sibling rows are recorded on every path
  that writes a shared dir — Propagation made consistent, never the reverse. **F2 (P2)** D35 boundary tests missing
  (delete_incomplete + source missing / central missing → typed skip; imported + delete_incomplete → unreported).
  **F3 (P2)** `RefreshSkipReason` not re-exported from `src/components/skills/types.ts`. Fix child dispatched; bounded
  Astra re-check after.

- 2026-11-02 (child) — **fix for audit F1–F3** (not committed; gate green: version 1.3.0, vitest 49 files / 1051,
  `cargo test --all` 837, lint / styles / clippy / fmt / build clean).
  **F1** — `core/propagation.rs` global arm: when a group's write is a first sync (no installed member
  `owns_artifact`), every installed Tool in `adapters_sharing_skills_dir` with no row for the skill is a sibling: on
  success it is recorded `TargetTransition::Recorded` (same mode / path / `synced_at`, fresh uuid) and reported as its
  own `Global { tool } → Synced`; a copy-only sibling would drive the write mode like a member. On failure it follows
  the batch's rule exactly — `global_sync::sync_skills_to_planned_tools` (`Err(TargetExists) =>` arm, "Asked first
  (D12): no row until the operator decides") records **nothing** for a `TARGET_EXISTS`, and every other failure goes
  through `record_failed` (`FailedRecorded` `error` row for each installed sharer, owning nothing) — so Propagation
  records no sibling on a collision and an `error` sibling (`first_deploy: true`) on any other failure. An owned
  artifact's update mints nothing; a disabled member / `disabled_at` skips the group before any sibling is computed;
  minted siblings join `handled`. Retry route and batch untouched. Fixture `propagate` mirrors it.
  Rust tests (`core/tests/propagation.rs`): `a_first_sync_records_every_installed_sharer_of_the_shared_dir`,
  `a_first_sync_records_no_sharer_that_is_not_installed`, `an_update_of_an_owned_artifact_records_no_new_sharer`,
  `a_first_sync_collision_records_no_sharer_and_its_overwrite_retry_matches_propagation` (the audit's sequence; the
  batch retry's row set equals Propagation's), `a_failed_first_sync_records_installed_sharers_error_as_the_batch_does`,
  `a_disabled_member_mints_no_sharer_and_skips_the_group` (pair and skill grain) — all pass. Fixture
  (`backend.test.ts`, "D34 amendment …" describe, 5 tests) pass. The audit's repro script, final assertion flipped to
  `[amp, kimi_cli]` and run with and without the foreign folder: PASS (`evidence/33-fix/repro.ts`, `repro.out`).
  **F2** — `core/tests/refresh.rs::refresh_all_reports_delete_incomplete_before_unlocatable_and_never_for_a_non_member`
  (source missing / central missing → `Skipped { DeleteIncomplete }` after a premise run proving each is Unlocatable;
  imported + `delete_incomplete` → absent; zero acquisitions; health / targets / central unchanged) — pass. Fixture
  mirror: "D35 boundary of Refresh all" describe, 3 tests — pass.
  **F3** — `src/components/skills/types.ts` re-exports `RefreshSkipReason` beside `SkillRefreshStatus`.
  Docs: AGENTS.md's Update/Refresh clause and CONTEXT.md's Propagation and Refresh entries now note the sibling record.
