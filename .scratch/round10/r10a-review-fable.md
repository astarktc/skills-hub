# Round-10 wave-A review — Fable seat

Reviewed `7a7c519..300f89b` (9 commits) at HEAD `300f89b`. Gate re-run here: `version:check` OK (1.2.9); `cargo test --all` 605 passed, tree clean afterwards (no binding drift); vitest 265/265; eslint clean; `tsc -b` exit 0.

## Standards

**Hard (documented rule) — none found.** Checked: new core modules declared in `core/mod.rs`; every `invoke`-crossing type is specta-derived and `src/bindings/index.ts` is committed and drift-free; i18n keys added to both `en` and `zh`; `commands/` only maps (`to_propagation_target_dto`, `UpdateSkipDto`); the only prose `bail!` ("edited skill changed under guard") guards a condition the UI cannot produce — permitted; guard non-reentrancy holds (`set_invocation_override` → `apply_unlocked` → `propagate_unlocked`; `repoint_and_update` validates outside, then the batch takes the guard per skill); `remove_path_any` callers unchanged; `hash_dir` has no caller outside `content_identity.rs`.

**Judgement smells (Fowler ch. 3):**
- `content_identity::record` is really "hash *and upsert the whole row*" — `finalize_install` depends on it being the upsert. The name hides an interface fact; a doc line or `record_and_upsert` would fix it.
- `skill_update::apply_unlocked`: two-level `match` on `UpdateBytes` with an `unreachable!()` arm. `GitAcquired` and `RestoreRebuild` carry identical payloads and take identical paths — the distinction is a hypothetical seam (one adapter). Collapsing to `Staged { staged, revision }` | `EditInPlace` removes the dead arm.
- `UpdateRequest { expected, record, … }` with public fields: the caller must know "only `record`'s three source fields are honoured; everything else comes from the current row". Documented, but it is interface complexity that a `SourceProposal { ref, subpath, type }` field would remove.
- `reportOutcome::refreshOutcome`: ~100 lines, four-deep nested ternary for the toast message (complicated conditional).
- `WARNED` process-global `HashSet` in `read` grows unbounded by distinct error strings — bounded in practice, but global state in core.
- AGENTS.md still says "vitest unit tests (hooks + commandError)"; `reportOutcome.test.ts` is a third kind — one-word doc drift.

**Depth / deletion test:**
- `content_identity` — deep. Delete it and hash rule + backfill + warn-once reappear in six callers (finalize, replay, propagation, project_sync, global_sync, onboarding_import). Three doors, one enum. Passes.
- `skill_update` — deep. Delete it and admission + settle + replay + propagate reappear in `refresh.rs` and `skill_edits.rs`, and the local Re-point ordering bug returns. `acquire_update` is the old installer body moved (not a pass-through — it *builds* the git adapter). Passes; the tail did not just move.
- `reportOutcome` — deep. Nine closures across four hooks collapsed into six pure folds; hooks read no report field (grep confirms only `plan.groups` iteration for selection, not report rendering). Passes. The `ctx.action` string discriminators push some policy variance into the interface, but each is a real second adapter.

## Spec

**B1** — all doors present with the ticket's semantics. `read` backfills exactly once (trigger-counted test); I/O failure → `None` and `next_status` returns `obs.current` (asserted for Synced/Stale/Error). Propagation reads once in `propagate_unlocked`; all three `content_hash: Option<&str>` parameters dropped. The 30 s poll is replaced by `reconcile_listing_unlocked` under `serialized` (backlog 15 ✓). `.skills-hub-manifest-` folded into `is_ignored` — allowed by the ticket, but **no comment says why** (ticket: "fold … *or* document"; folding silently loses the reason). Onboarding migration (`3db3adc`) is forced, not creep: `target_has_same_content` was deleted and `hash_dir` privatised, so `onboarding.rs`/`onboarding_import.rs` had to go through the module's doors.

**B2** — admission compares `source_ref`/`source_subpath`/`source_type` of the current row against `expected`, under the guard; `repoint: true` bypasses the stale check and `SkillGone` still applies (ruling Q4 ✓). Staged bytes drop with the request (`StagingDir::drop`; test asserts `!staged.exists()`). No upsert before finalize on any adapter (`validated_local_repoint` no longer writes; `acquire_update` never writes). Local Re-point failure: test covers both staging-fault and settle-fault, asserting row `Debug`-equal and old bytes ✓. Edit: failing copy target lands in `propagation` and the row keeps `Error` status (test ✓); no `log::warn!` remains. D2 tests pass unchanged. `refresh.rs` = pool + guard + reassert + report; `installer.rs` = Add/listing/preview/backfill (605 lines, nothing of the tail). Backlog 13 ✓ (`apply_replay`, `restore_central_bytes`). Tests moved, not duplicated (4 from `tests/installer.rs`, 1 from `tests/refresh.rs`).

**F1/F2** — `managedSkillsRef` gone; actions carry `{skillId, skillName}`; click resolves via `gitRepointSelection` → `managedSkills.find` at render, with a `skillGone` warning effect (hook test proves resolution against a *replaced* row). One hook test per action (12-row table) proves invoke → fold → reporter → completion (reload/close both ways). Deliberate behaviour changes vs v1.2.9, judged:
1. **Single Update/Restore/Re-point failure**: v1.2.9 = one error toast (`action.fail`). HEAD = error-entry toast "Update failed: *name*" **plus** a second warning toast "0 skills refreshed, 1 failed." (`ctx.single` inherits the batch-count copy; `reportOutcome.test.ts:194-222` asserts it). → **regression** (double toast, batch copy on a single action).
2. Refresh-all: conflict now outranks failure in the toast (v1.2.9 was the reverse). Spec-mandated; the failure entries still toast separately. Acceptable.
3. Refresh-all: target/reassert failures now yield `partialFailure` warning instead of "All skills refreshed." Correct.
4. Local Re-point now reports conflicts and reloads on failure. Correct (closes the fork).
5. sync-to-all / auto-sync with failures: success toast suppressed (entries only). Correct.
6. Import `kept_divergent` error→warning. Correct.
7. Candidate batch with every install failed: modal stays open, no reload. Reasonable.
8. `refreshSummarySkipped` copy says "(could not be located)" — wrong for `skipped_acquisition`. Race-only reach; follow-up.

Integration: `ACQUISITION_SKIP_KEY` beside `SKIPPED_REASON_KEY`/`UNLOCATABLE_STATE_KEY` in `skillPresentation.ts` — right home. Worktree safety: `git diff 7a7c519..HEAD -- src-tauri/src src/` touches only in-scope files; `sync_status.rs`, `artifact_removal.rs`, `mutation_guard.rs`, `git_*` untouched; D1/D2 tests intact.

## Summary

**Blocking**
1. `src/lib/reportOutcome.ts` `refreshOutcome` with `ctx.single`: a failed single Update/Restore/Re-point now shows two toasts, the second reading "0 skills refreshed, 1 failed." — operator-visible regression vs v1.2.9's single error toast. Fix: when `ctx.single` is set and the toast would be a count summary (`failed`/`skipped` branch), emit `toast: null` (the error entry already toasts) and adjust the table test's `single.toast` expectation.

**Follow-up**
- `refreshSummarySkipped` / `status.refreshSummary` copy is batch-shaped; `skipped_acquisition` inherits "(could not be located)" — add a reason-neutral skipped summary or drop the parenthetical.
- Add a one-line comment in `skill_files::is_ignored` explaining the `.skills-hub-manifest-` prefix (abandoned atomic-write temp of `frontmatter_edit`).
- Collapse `UpdateBytes::{GitAcquired, RestoreRebuild}` (identical payload and path) and remove the `unreachable!()`; consider a `SourceProposal` field instead of a full `record` in `UpdateRequest`.
- Rename/document `content_identity::record` as the upsert it is.
- AGENTS.md test bullet: "hooks + commandError" → "+ pure folds (`reportOutcome`)".
