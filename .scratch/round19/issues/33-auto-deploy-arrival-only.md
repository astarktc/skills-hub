# 33 — Auto-deploy is arrival-only: the Refresh re-assert is deleted, `delete_incomplete` is skipped as report data

Status: ready-for-agent
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

(child writes: what changed, the first-deploy-copy finding, test names, sha)

## Comments
