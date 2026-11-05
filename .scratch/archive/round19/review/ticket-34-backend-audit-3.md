# Ticket 34 — third backend audit

**Verdict: FAIL.** The ordinary F10–F13 regressions now pass, but the adversarial round found **one P0 and two P1 findings**: directory aliases can make relocation delete the central skill itself (F15); rejected crash adoption leaves a missing source marked synced (F14); and Keep destroys target-local copy edits that exist nowhere else (F16). Do not merge the backend on the strength of the green standard gates.

## Scope and evidence

Reviewed `/Users/alexstark/.t3/worktrees/skills-hub/t3-dd600575`, branch `t3/registry-truth-pass`, uncommitted changes over HEAD `8b42ce5fcd9dd82aadf59c51dd9e98478f6bae93`. Read AGENTS.md first, BACKLOG/newest handoff, audits 1 and 2 in order, the ticket's Result/fix passes/UI handoff, then the affected implementation and regression tests. No child delegation, source fixes, commits, dev app, or live-library access.

**This report is the only worktree file deliberately written.** Authorized binding exports produced identical bytes. Extra Rust tests live only in `/tmp/t34-audit3-scratch/src-tauri/src/core/tests/audit3.rs`, included by a three-line declaration in that scratch copy's `tests/target_relocation.rs`. `diff -rq` confirms those are the only differences between scratch and worktree Rust source. Production functions are byte-identical. The scratch run reused the worktree's Cargo target cache. A byte comparison of `git diff` captured before and after the review confirms no tracked change was altered by this audit.

Executed tests are identified explicitly below; other checks are **code traces**, not claims of executing crashes or Windows. Final scratch run: **10 tests, 6 passed / 4 failed**, exit 101. The four failures support three findings (F15 has two tests). Logs and before/after evidence: `/tmp/t34-audit3-evidence/`; final adversarial output: `adversarial-final.log`. These scratch files supplement the durable reproductions below.

## Audit 2 — status

| Finding | Status | Fresh evidence and boundary |
| --- | --- | --- |
| **F10 — failed cleanup loses the old artifact's owner** | **STILL OPEN at the expanded crash/alias boundary** | All **5 `f10_*` tests pass**. Read `target_relocation.rs:326–408`: materialise → ownership-aware source removal → row settlement; failed removal rolls back an unowned destination, keeps the source row `error` with ownership, and adds `incomplete`. Permission-failure retry, identical destination adoption at both crash boundaries, an owned destination surviving rollback, and first-sync rollback all work. Scratch tests also prove recovery from the seeded failed-rollback state and removal of a destination recorded only by a non-owning marker. But refused adoption after source loss leaves `Synced` at a nonexistent path (F14), and source/destination aliases can delete the surviving destination or the central bytes (F15). The original permission-failure reproduction is closed; the broader safety claim is not. |
| **F11 — Keep → Enable orphans the former deployment** | **CLOSED for the reported ownership sequence; new data-preservation issue F16** | Both **`f11_*` tests pass**, including real removal failure. Read `global_sync.rs:623–668` and `artifact_removal.rs:861–893`: Keep removes/settles the old owned artifact before minting a non-owning disabled destination marker; failure returns `Failed`, retaining old path, `synced_at`, and `error`. The kept foreign destination is untouched. New scratch coverage confirms the app's central Edit survives both link and copy removal. A divergent target-local copy is nevertheless deleted without preserving its unique bytes (F16). |
| **F12 — lexical onboarding keys miss physical aliases** | **CLOSED for the reported cases** | Both **`f12_*` tests pass**, each exercising Selected and Installed scope. `artifact_identity` is canonical parent + leaf; ownership and managed/invalid onboarding exclusion use it. Scan roots are canonicalized separately and merged with their reader roster. Scratch tests prove `/var`/canonical-parent equivalence, distinct leaf links to one central referent remaining distinct, and lexical fallback for a missing parent. Do not mistake this for physical-identity safety throughout relocation: F15 is outside the fixed onboarding consumers. Windows was not executed. |
| **F13 — non-owner Propagation creates a silent second copy** | **CLOSED at the backend seam** | **`f13_*` passes**. Read `propagation.rs:294–367`: an installed owner on a present superseded path blocks only the non-owner first-sync partition with `RelocationPending`; owned partitions still settle in place. Scratch `audit3_pending_propagation_resumes_after_launch_relocation` proves the skip is serialized in the report, no destination write happens, launch completes relocation, and the next Propagation syncs/owns the previously skipped Codex partition. The frontend fold still suppresses skips; that is explicitly handed to the UI half, not fixed by this backend audit. |

## Audit 1 — spot check

The six requested targeted regressions all pass freshly, in addition to both complete suites:

- **F1 still CLOSED:** `the_preference_never_moves_a_target_into_a_dir_a_disabled_sharer_reads`; `codexs_registry_move_waits_while_a_destination_sharer_is_disabled`. Admission remains shared with preview and uses `DisabledGate::blocks_group`; disabled markers are not cleared.
- **F2 still CLOSED for other deployment owners:** `relocation_never_removes_an_artifact_a_project_assignment_owns`; `kimis_relocation_keeps_an_artifact_a_config_agents_project_owns`. `artifact_is_owned_except` still plans every skill's global targets and project assignments, excluding only the moving global row IDs. Non-owning markers do not protect an artifact. Central-byte protection is a separate missing boundary, F15.
- **F3 still CLOSED for the original Update/first-sync orphan sequences:** `an_update_after_a_partial_relocation_updates_each_owned_path_in_place`; `a_first_sync_never_replaces_a_different_owned_target_path`. Owned-path partitions remain distinct; the first sync leaves a present superseded owner for the unlocked relocation seam. F11's original Keep → Enable orphan is also closed. These passing sequences do not waive F14–F16.

## New findings

### F14 — P1: refused crash adoption leaves a missing source permanently marked synced

**Executed:** `audit3_foreign_destination_after_source_removal_is_not_synced`.

Seed the state after source removal but before row settlement: Augment's owned, `Synced` symlink row still names `.augment/rules/skill`, that source is absent, and `.augment/skills/skill` now contains `SKILL.md = "foreign replacement"` (another writer replaced the destination after the crash). Run the public launch entry.

```text
RelocationSummary { relocated: 0, left_in_place: 1, incomplete: [] }
row: target_path=.augment/rules/skill, status=Synced,
     last_error=None, synced_at=Some(1)
source_exists=false
foreign destination bytes unchanged
```

**Code:** `target_relocation.rs:443–451` correctly refuses adoption when the source is absent and destination content is not identical. But `materialise` returns only `None`; `settle_move:341–344` increments `left_in_place` and never settles the row's failure/missing state or supplies an incomplete reason. `skill_catalog.rs:159–184` reads target rows unchanged and `commands/mod.rs:1195–1204` passes their stored status straight to the frontend. This is not repaired by catalog listing. For a recorded symlink, `propagation.rs:421–435` can continue answering `LinkFollowsSource` without checking that the link exists, so a normal Update is not a reliable recovery either.

The safety half **does** hold: foreign destination bytes are neither adopted nor overwritten. The truthful-status half explicitly required by this audit does not.

**Exact fix:** make materialisation refusal distinguish an absent source from a harmless refusal with an intact owned source. For an absent source with a non-adoptable destination, keep the source path for recovery/removal but settle a typed missing/error outcome and include an actionable incomplete row; never leave `Synced`. Propagate that result through first-sync reporting as appropriate. Preserve the foreign destination. Add this seeded crash-state regression, asserting catalog status and report reason as well as bytes; keep the identical-destination crash adoption tests green.

### F15 — P0: relocation's lexical move/fences can delete the destination, including the central skill itself

**Executed, two tests:**

1. `audit3_same_physical_source_destination_must_not_be_removed`: create the ordinary owned Augment link `.augment/rules/skill` → central; make `.augment/skills` a directory symlink to `.augment/rules`; relocate. `artifact_identity(old) == artifact_identity(destination)` is asserted before the call. Actual result:

   ```text
   RelocationSummary { relocated: 1, left_in_place: 0, incomplete: [] }
   row: .augment/skills/skill, status=Synced
   destination_exists=false
   ```

   Materialisation sees the existing identical central link and succeeds. The new write order excludes the moving row from ownership, then unlinks `old`—which is the **same artifact** as destination. The row is settled to the now-missing destination. This applies equally to a preference move with `.agents/skills` aliased to a harness's own skills dir. The older post-settlement all-owner check would have seen the moved row at the alias; the new pre-settlement exception needs an explicit same-artifact check.

2. `audit3_directory_aliases_must_never_delete_central_bytes`: tested **both** directory-alias directions in one test, collecting both results before asserting:
   - **Source alias:** `.augment/rules` → the central repository directory; the owned copy row at `.augment/rules/skill` physically names the managed skill directory. The new destination is materialised as a link to central, then source cleanup deletes the central directory through the alias.
   - **Destination alias:** start with the ordinary old link into central; `.augment/skills` → the central repository directory. The destination physically names the central skill directory. The same-content check authorizes replacement; sync removes the directory containing the source bytes, then leaves a dangling/self-referential link. Source cleanup and row settlement still report success.

   ```text
   source alias:      relocated=1, left_in_place=0, incomplete=[], central_exists=false
   destination alias: relocated=1, left_in_place=0, incomplete=[], central_exists=false
   expected central/a.txt == "v1"; actual missing in both cases
   ```

   No external writer/race or artificial production failure hook is needed. These are ordinary filesystem symlinks under a temporary home. A user can replace a Tool's skills root with an alias after deployment; a stored owner row is not proof that traversing its parents cannot reach central. This is central-content destruction by an automatic launch operation, not merely a stale strip mark.

**Code:** candidate and move identities are lexical (`target_relocation.rs:187–191,281–297`); `settle_move:341–357` does not compare old/destination identity before removing the source. The cleanup/rollback/Keep fences are also lexical direct-parent checks (`artifact_removal.rs:804–821,833–847,873–882`). The new identity helper is only used to find **other target/assignment owners**; the central source is not one of those rows. On destination aliasing, `materialise:432–460` invokes `sync_engine::hybrid_with_overwrite:172–211`, whose source check occurs **before** destructive replacement and whose removal has no source/target identity guard; `hybrid:112–154` can then create a link to the now-absent source. The sync-engine vulnerability is not new in pass 3, but this migration's automatic writes reach it. It must not ship unguarded merely because that lower-level code predates the ticket.

**Exact fix:** before any materialisation or cleanup, resolve artifact identities with canonical parent + leaf (not canonicalizing an ordinary deployment link's referent):

- If old and destination identify the same deployment entry, treat this as a path/row normalization, never a physical move/delete; re-key/settle only with truthful presence/content checks.
- Refuse or safely special-case a move whose source or destination entry physically identifies a central skill directory. No target cleanup, rollback, or Keep seam may remove central bytes through an aliased parent. Protect all managed central paths, not merely another global/project row.
- Harden the sync write seam against destructive source/target identity overlap before overwrite so other callers cannot delete their own source. Preserve normal per-skill symlinks into central; blindly canonicalizing the leaf and rejecting every such link would be wrong.
- Test aliased old/current roots, both central-root alias directions, and first-sync/Keep cleanup against aliases. Assert central bytes, deployment presence, ownership, and truthful report status—not just relocated counts.

### F16 — P1: Keep deletes the only copy of target-local operator edits

**Executed:** `audit3_keep_preserves_only_copy_of_local_operator_bytes`.

Cursor owns a real **copy** at `.cursor/skills/skill` (mode Copy, `synced_at` set). Add `operator-only.txt = "only local edits"` inside that copy, absent from central. The preferred `.agents/skills/skill` contains foreign content. With preference on, answer the destination collision **Keep**.

```text
outcome: Declined
old owned directory: removed
Cursor row: disabled at .agents/skills/skill, synced_at=None
foreign destination: unchanged
operator-only.txt: absent from central and old copy
```

**Code:** `global_sync.rs:631–642` admits the old owner for cleanup based on location/ownership/presence; `artifact_removal.rs:873–882` checks its fence and other ownership, then unconditionally `remove_path_any`. Neither compares the old copy with central nor preserves divergent bytes. This is the requested distinction between ownership and byte preservation: the row transitions are now correct, but the only copy of the user's edits is gone.

**What “Edit” actually stores:** the app's Edit is an **invocation override**, not a generic target-local editor. `skill_edits.rs:61–74` locates the central `SKILL.md`; `:94–125` persists a `SkillEditRecord` (mode + base invocation lines) and calls Update; `:138–159` writes/restores those lines in the **central** manifest before Propagation. Scratch `audit3_keep_preserves_central_app_edit_for_link_and_copy` passes for both an ordinary link and a copied deployment: Keep removes the old deployment, but the central manifest bytes and durable invocation override remain unchanged. Thus app-managed Edit is safe. A manual/editor/harness modification made only to a deployment **copy** is not that central Edit and is not backed up by it. The failing test deliberately distinguishes the two; it does not claim app Edit stores bytes in targets.

**Exact fix:** before this new Keep cleanup deletes a real owned copy, prove it is byte-identical to central (including legitimate central Edit), or preserve its divergent bytes durably / require an explicit destructive choice that identifies the old path and local changes. A minimal safe backend response is a typed failed/refused removal with the old row/path still owned and the foreign destination untouched; do not report `Declined`/disabled until preservation or authorized removal settles. Add both the target-local divergence regression and the passing central-Edit link/copy cases. Ownership alone is sufficient for an explicit Remove policy, but the destination's **Keep** answer must not silently discard the only local edits.

## Adversarial round — coverage and traces

### 1. Write order and crash boundaries

- **After destination creation:** production `f10_a_crash_after_materialising_is_completed_next_launch` passes. The old row/artifact remains; identical destination is reused/replaced, source removed, same row settled. No duplicate row.
- **After source removal, identical destination:** production `f10_a_crash_after_source_removal_is_completed_next_launch` passes. Adoption recovers the one row.
- **After source removal, foreign destination:** executed F14. Refusal preserves foreign bytes but leaves a nonexistent source synced, with no incomplete reason.
- **After rollback destination-removal failure:** scratch `audit3_failed_rollback_intermediate_state_retries_once` seeds exactly the resulting state—source and identical destination present, source row `error` and owned. The public launch moves once; the next launch returns the default empty summary; one row remains at destination and source is absent. This is a seeded boundary test, **not** a real crash or a dynamically injected second unlink failure. A real source-removal failure is executed in the production F10/F11 tests.
- There is no durable pending-move record. Recovery is inferred from source path plus destination content. That explains why identical-byte recovery works and why an unclassified refusal can lose truthful state. Source/destination physical aliasing is F15.

### 2. Rollback exceptions

Production `f10_rollback_keeps_a_destination_another_row_owns` seeds Codex ownership at `.agents/skills/skill` and denies Cursor's old-parent unlink with mode 0500. Destination survives; Cursor remains owned/error at source. Scratch `audit3_rollback_removes_destination_recorded_by_nonowning_marker` changes the destination row to never-synced `error` (not disabled, so it does not block admission): the destination is removed, marker still owns nothing, and Cursor remains owned/error. A disabled marker instead blocks relocation before writing, as F1 requires. The rule is **owned**, not merely recorded.

### 3. Keep, links, copies and failures

Production F11 executes refused relocation → Keep → Enable → Delete, with foreign bytes retained, old link removed and marker non-owning. Its 0500 test executes removal failure → `Failed`, old path owned/error, not Declined. Scratch distinguishes central app Edit (link and copy safe) from divergent target-only copy edits (F16). Another owner's old artifact is deliberately retained by `artifact_is_owned_except`; this is not a claim that Keep can physically stop a harness from reading an artifact legitimately retained for another owner/cross-reader.

### 4. RelocationPending and its report/wire path

`propagation.rs:348–366` appends `Skipped { reason: RelocationPending { error } }` to each blocked non-owner member and continues **before** sibling recording or `ArtifactWrite::settle`. Owned partitions still run. The recovery scratch test passes and checks serialized JSON contains:

```text
status.status = "skipped"
status.reason.reason = "relocation_pending"
status.reason.error.code = "RELOCATION_PENDING"
```

`CommandError` includes the tagged variant; generated `src/bindings/index.ts` includes it and the skip reason. `src/commandError.ts:113–116` handles it using `errors.relocationPending` and the translated harness label; the new `commandError.test.ts` case passes in the full frontend suite. **It reaches the backend report, but is swallowed by today's frontend notification fold**: `reportOutcome.ts:522` skips all statuses other than `failed`. The Result/UI handoff explicitly assigns surfacing this skip and `relocation.incomplete` to the UI half. Backend closure is not end-to-end UX closure.

### 5. Artifact identity

`artifact_removal.rs:475–481` canonicalizes only the parent and appends the leaf unchanged. Executed scratch tests confirm:

- `/var/...` and the `canonicalize(home)` spelling identify the same entry on this macOS host.
- Two distinct deployment links to the same central directory remain distinct artifact identities.
- A missing parent returns `path.components().collect()` as documented, not a physically resolved identity.

Production F12 tests cover a Tool skills dir symlinked to another Tool's dir under both scan scopes. `onboarding.rs:322–324` uses the helper for managed exclusion; invalid entries pass through the same `is_unmanaged` rule. Scan-root dedupe at `:169` **does not literally call `artifact_identity`**: it canonicalizes the whole directory, correctly following a directory alias, then merges `relative_dirs`; reader union happens at `:261–279`. These are different objects (directory root vs leaf artifact), not a second lexical-only exclusion implementation. The helper's comment saying scan-root dedupe “reads it” is imprecise.

Requested grep for `components().collect()` finds only:

- `artifact_removal.rs:481`: the documented missing-parent fallback;
- `onboarding.rs:169`: the canonicalized scan-root's fallback;
- `log_reveal.rs:99,107`: separate lexical admission normalization, not onboarding/removal ownership equality.

No old lexical-only `managed_target_key` remains. Physical identity still needs to govern the destructive relocation decisions in F15.

### 6. Operator-like launch idempotence

Scratch `audit3_operator_like_launch_twice` runs the **public launch pass twice** on six owned deployments: Cursor, Pi, Codex in deprecated `.codex/skills`, Gemini CLI, Amp, Kimi. With Kimi already in `.kimi-code/skills`: first `{relocated:1,left_in_place:0,incomplete:[]}` (Codex only); second `{0,0,[]}`; six rows, other paths unchanged; `agents_standard_offer_pending=true`.

Also tested Kimi at its **old registry path** `.config/agents/skills` beside Amp: first relocates **two**, Codex and Kimi, second `{0,0,[]}`, offer still pending, Amp's shared original intact. This is the authorized Kimi registry correction, not a preference-on default. “Nothing but Codex moved” is true only for the current-Kimi-path seed. No real-machine seed was inspected.

### 7. Re-entrancy — code trace, backed by public-entry tests

No `serialized` inside `serialized` found on these paths:

| Entry | One lock → internal path |
| --- | --- |
| Launch | `lib.rs:183` → `relocate_former_global_targets` (`target_relocation.rs:111`, one guard) → `relocate_unlocked` → `relocate_skill_unlocked` → `settle_move`. |
| Preference save | `apply_global_layout` (`:129`, one guard) → setting write + same unlocked relocation. |
| Global batch / Keep | `global_sync.rs:699` guard → `sync_skills_to_tools_unlocked` → planned batch; Keep → `record_kept` → `settle_superseded_owner_unlocked`; successful sync → `relocate_synced_skills` → `relocate_skill_unlocked`. |
| Enable | `skill_disable.rs:144` guard → `enable_unlocked` → pair/skill → `first_sync_global:452` → global batch **unlocked**, then same relocation. The test convenience `enable_targets` only delegates to this guarded entry. |
| Disable / Delete | Disable's guard (`skill_disable.rs:110`) → removal planning + `execute_settling_unlocked:285`; Delete's guard (`artifact_removal.rs:914`) → plan + `execute_unlocked`. |
| Update / Edit / Propagation | Refresh apply guard (`refresh.rs:300,318`) or Edit guard (`skill_edits.rs:94`) → `skill_update::apply_unlocked:192` → `propagate_unlocked`. Propagation calls no relocation entry and acquires no guard. |
| Install / Import | Their existing per-settlement guards (`install_batch.rs:222`, `onboarding_import.rs:242,248`) → global batch unlocked → unlocked relocation. |
| Relocation removal / rollback | `artifact_is_owned_except` calls read-only removal planning; `remove_superseded_artifact_unlocked` / `remove_relocation_destination_unlocked` call low-level removal, no guarded public removal entry. |

Preview remains read-only and outside the guard; its count is an attempted-move estimate, not a guarantee of successful moves.

### 8. PartialEq, serialization, and the contention test

`errors.rs` changes the derive to add only `PartialEq, Eq`, plus the new variant. Equality derives add Rust comparisons, not serde fields or specta shape. The existing serde tagging is unchanged. Both explicit `cargo test export_bindings` runs pass and produce byte-identical bindings; before tests, after exports and after scratch verification all hash to:

```text
5f0dc0cc09382965be02cdebad3c3caa78799aecd99c329896a7341de5bc4bf2
```

The requested flake **did not reproduce**: both full Rust runs and all three `cargo test mutation_guard` runs pass. Read `tests/mutation_guard.rs:466–480`: 100 attempts with 50 ms sleeps to acquire the process-global try-lock. The same file's HOLD tests occupy that guard for 300 ms per window. `git diff --exit-code HEAD -- src-tauri/src/core/tests/mutation_guard.rs` is empty/exit 0. The previously reported failure is consistent with the unchanged bounded poll budget under parallel guard contention; new guarded tests increase load, but this run provides no evidence of a new deadlock. Do not report a new reproduced P2 here. The existing recommendation stands: isolate the no-contention case or provide a deterministic test synchronization seam, rather than changing the production reader to block or merely hiding failures with retries.

### 9. Documentation after pass 3

README still has **47 table rows / 47 registry literals**; registry corrections, retained OpenClaw, fresh vs existing defaults, Kimi/Amp caveat and cross-read qualification agree with the reviewed implementation. No new registry-fact discrepancy was identified. Kilo/Xum project paths remain the explicitly recorded deferral, not silently implemented here.

CHANGELOG/CONTEXT describe the intended write order, rollback, central content source, Keep removal and pending skip, and correctly separate registry corrections from the preference. They are **not reliable unconditional safety guarantees yet**: F14/F15 disprove healthy crash recovery and physical-path safety, and F16 exposes the byte-loss consequence of unqualified “Keep removes the old copy.” A failed rollback may also leave the destination until a later successful retry, so “the move is undone” needs that qualification. Keep those descriptions aligned with the actual fix, rather than treating prose as proof.

**Carried P2, not a new finding:** audit 2's F9 stale header remains at `tool_adapters/mod.rs:341` (“a registry edit without a brief edit fails”). The citation-bound hand-transcribed snapshot does not enforce that stronger claim. Change it to “without a snapshot edit” or implement revision/content coupling. `artifact_removal.rs:793–798` also still describes cleanup as happening *after* row settlement, contrary to pass 3's order; update that comment when fixing the relocation seam. No source comments were changed by this reviewer.

## Verification

All commands below ran in the specified worktree unless marked scratch. Exit codes are observed, not inferred from assertion counts. Node: `v26.10.0`.

| Command | Fresh result |
| --- | --- |
| `cd src-tauri && cargo test --all` — run 1 | **0** — 897 passed, 0 failed, 16.72 s; binary/doc targets also pass. |
| Same — run 2 | **0** — 897 passed, 0 failed, 14.14 s; binary/doc targets also pass. |
| `cargo test mutation_guard` ×3 | **0 / 0 / 0**, 13 passed each (3.28 / 3.31 / 3.31 s). |
| `cargo test f10_` | **0**, 5 passed. |
| `cargo test f11_` | **0**, 2 passed. |
| `cargo test f12_` | **0**, 2 passed. |
| `cargo test f13_` | **0**, 1 passed. |
| Six targeted F1–F3 filters listed above | **0** each, 1 matched test each. |
| `cargo test export_bindings` ×2 | **0 / 0**, 1 passed each; byte-identical bindings. |
| `cargo clippy --all-targets -- -D warnings` | **0**. |
| `cargo fmt --check` | **0**. |
| `npx vitest run src` | **0**, 53 files / 1,153 tests; no unhandled teardown errors. Only Node's experimental localStorage warning. |
| `npx tsc --noEmit -p tsconfig.app.json` | **0**. |
| `git diff --check`, including final repeat | **0**. |
| Scratch: `CARGO_TARGET_DIR=/Users/alexstark/.t3/worktrees/skills-hub/t3-dd600575/src-tauri/target cargo test audit3 -- --nocapture` | **101**, **6 passed / 4 failed**, 897 original tests filtered. Failures are F14, both F15 tests, F16. |
| `diff -rq src-tauri/src /tmp/t34-audit3-scratch/src-tauri/src` | **1 as expected**: only added `core/tests/audit3.rs` and the scratch test-module declaration differ. No production difference. |
| `cmp` of before/after `git diff` captures | **0**: existing tracked changes untouched. |

Scratch test setup initially failed to compile because its `load_settings` call omitted credentials/root; corrected the **scratch fixture** to use `MemoryStore` and explicit temp central root, then executed the final ten-test run above. No production patch was used to manufacture these results. Both crash-boundary recoveries and the rollback-failure state are seeded intermediate states, not hard process-kill tests. Windows and operator release smoke remain unexecuted.

Final pi-lens cache check was inconclusive; active probing confirmed the generated TS binding clean, while the two scratch Rust test files had no language-server answer. Actual Cargo compilation and execution supply their verification; zero LSP diagnostics is not represented as a clean Rust verdict.

## Final verdict

**FAIL.** The F1–F3 spot checks, ordinary rollback/Keep fixes, physical onboarding exclusion, typed pending Propagation skip, next-launch recovery for identical bytes, settings idempotence and all requested standard gates pass. However, the ten-test adversarial extension proves a central-data deletion path through aliased directories, a false-synced missing source after refused crash adoption, and loss of target-local copy edits on Keep. Fix F14–F16 and retain their regression evidence before merging; the UI half must still expose pending/incomplete outcomes, and no live-library smoke or Windows safety claim follows from this audit.
