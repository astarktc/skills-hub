> **Disposition (orchestrator, 2026-09-22):** all findings applied — see `../issues/09-review-fixes.md` (`b8ebb35`, `634cef8`).

# Round 16 adversarial review

Reviewed `214547289be8617c61640973b2202bbdeff13e0b..4a721b9c8e52fbf6a74b18ec97ec181ab696d4d1` (25 commits), against AGENTS.md, the round-16 spec, tickets 01–06 including deviations, CONTEXT.md and ADRs 0001–0004. Code remained untouched; no app/dev-window launch.

## Verdict

**fix-then-ship** — one must-fix settlement-contract gap. The ordinary-path implementation meets the round's product requirements, and the full gate passes. Version/release work in ticket 07 and installed-build/operator smoke remain outstanding; the reviewed checkout is still 1.2.15.

## Must-fix

### M1 — Do not return a settled failure when persisting that failure failed (Standards + D1)

**Locations:** `src-tauri/src/core/project_sync.rs:514–528` and `:353–362` (the latter catches failures propagated from `:222–229`).

Resync suppresses the settlement error:

```rust
if let Err(settle) = store.transition_assignment(
    &assignment.id,
    AssignmentTransition::SyncFailed { error: &format!("{:#}", e) },
) {
    log::warn!(...);
}
ProjectSyncOutcomeStatus::Failed {
    error: CommandError::from_anyhow(e),
}
```

The assign path has the same contract hole indirectly: `assign_and_settle` correctly `?`-propagates a failed error-state write, but `assign_skill_to_tools` catches **every** `Err`, looks up whichever row remains, and emits `Failed { error: CommandError::from_anyhow(error) }`. A failed settlement after insertion therefore produces an item with an assignment id while the row is still `pending`, without the promised `last_error`. Resync can leave the previous `synced` state and omit the persistence failure from the wire entirely.

**Why it matters:** D1/ADR-0001 require classification after the row's diagnostic is written. The new report explicitly promises that a failed item naming a row describes an error-state row. Disk-full/read-only/write-failure paths violate that promise. This carries inherited best-effort behavior into the new contract; ordinary sync-failure tests do not test failed settlement itself.

**Smallest correct fix:** propagate the resync settlement failure as `anyhow::Error` before classifying the original sync failure. Also let post-insert persistence failures escape the assign engine to the command boundary; do not catch and reclassify them as settled per-target outcomes. Preserve the original sync chain as context.

**Fuller fix is warranted for assignment:** give the engine a `Result<ProjectSyncReport>` boundary, distinguish expected per-tool refusals (still report data, e.g. unknown tool) from store failures, and eliminate the redundant post-settlement row read when only the already-known assignment id is needed. Do **not** solve this by propagating a preclassified `CommandError` through core; that repeats round 15's rejected fix.

**Regression test:** inject a SQLite trigger rejecting `UPDATE ... status = 'error'`, force a filesystem sync failure, and assert toggle/bulk-assign/resync/resync-all fail at the command-error boundary rather than returning an allegedly settled item. Keep the existing tests proving ordinary target failures remain report data. This fault case was established by control-flow inspection; no new test files were written under the read-only brief.

## Should-fix

### S1 — Surface failure to refresh the matrix after successful resync-all

`src/components/projects/useProjectState.ts:463` now calls `await refreshView(selectedProjectId)` on success, but `:205–210` catches all read failures with `// Silent fallback — state may be stale`. Preserving the sync report is the right deviation; silently leaving the old matrix looking current is not ideal. Return the report **and** expose a view-refresh failure/stale marker, rather than throwing away the report or hiding the failure. Add a hook test for successful `resyncAllProjects` followed by failed `getProjectView`. This is non-blocking: the mutation report itself remains truthful.

## Nits

- `src/i18n/resources.ts:196–197,835`: remove the stale compatibility comments saying the fold still uses the old key. It now uses `changeSource.action`; the comments sit above the permanent `changeSource` block and misleadingly suggest retiring it.
- `src/components/projects/useProjectState.ts:199–202`: `refreshView`'s “Error-path convergence only” / “Success paths never need it” documentation no longer describes resync-all.

## Verified-OK

- **Guard/removal boundaries:** changed project mutation entry points take `mutation_guard::serialized`; composites call unlocked seams. Re-point acquires outside the guard and settles through Refresh's guarded apply phase. Bulk unassign plans `ProjectSkill` and uses `execute_unlocked`; no new direct target deletion. Production `remove_path_any` callers remain only artifact removal, onboarding import and sync engine. No newly classified `CommandError` is subsequently `?`-propagated as an anyhow error; M1 concerns failed persistence being swallowed instead.
- **D1 ordinary outcomes:** fresh sync failures retain their assignment ids; toggle is `kind`-tagged; bulk assign and both resyncs return the same core report. `project_id` reaches the resync-all failure fold. No report DTO mirror or wire counter was added. `ResyncSummary`, `BulkAssignErrorDto` and old internal outcome types are absent from active code. The `ProjectSyncOutcomeStatus` name avoids the existing roll-up enum; this deviation is sound. Project-listing failures in resync-all fail the whole command, not a fabricated item.
- **D2:** scope selection is restricted to one project × skill; successful removal deletes rows, failed removal keeps error rows via the established executor. Tests cover another skill/project remaining intact, partial removal, and serialization. UI has no confirmation, hides saturated bulk assign, and gates bulk unassign on ≥1 assignment and >1 configured tool as accepted. Memo comparisons include the new callback.
- **D3/D4:** no provenance guard survives Re-point. All six transitions use the same Update/finalize path. Local proposals clear subpath/revision; git proposals carry the resolved subpath/revision; both clear imported history. `a_failed_local_repoint_of_a_git_skill_changes_nothing` (`core/tests/repoint.rs:1235–1269`) injects a finalize write failure, compares the complete record and explicitly asserts `Some("old-sha")`; it is not merely a source-type test. Acquisition refusal preserves the record/bytes; Edit replay is tested across git→local. Imported→local joins a subsequent real Refresh-all; both imported crossings assert refresh eligibility.
- **D5/D9:** one Change source modal on all managed cards and managed detail views; source-missing repair explicitly selects local; imported defaults local. Folder cancellation invokes no mutation; confirmation uses the typed command and returned catalog/fold completion. `~` expansion uses explicit `paths.home` (not environment lookup), with a passing test. The channel factory is `newRefreshProgressChannel`.
- **D6:** listing resolves home at the command seam and calls the same `tool_holding_path` predicate as installation. Tool-owned root/child candidates stay visible, invalid, and mapped to the disabled reason. Giving this reason precedence over a broken manifest is sound.
- **D7/D8:** inspected the deleted tests and engine assertions; ticket 05's mapping is accurate. `GitSelection.subpath` is `&str`, listing has its private resolution constructor, and both runtime listing guards are gone. Root/prefix/branch resolution behavior is retained.
- **Wire/error plumbing:** new commands have both attributes and are registered in `collect_commands!`; regenerated bindings are unchanged after both cargo runs. `assignment_id` is required `string | null`, not optional. Old Re-point commands and `GIT_REPOINT_REQUIRES_GIT` are absent from Rust/TS/i18n. `commands::tests::git_repoint_refusal_crosses_the_wire_as_a_typed_error` now asserts contextual `InvalidGithubUrl` serializes as `{ code: "INVALID_GITHUB_URL", url: "bad" }`; its singular name remains honest.
- **Frontend/standards:** report interpretation stays in the fold; hooks apply response envelopes. Component/backend access stays through `invokeTauri`; storage remains centralized; no second relative-time/repo-grouping implementation. New copy has EN/ZH parity (493 leaf keys each, no missing keys). No new core filesystem-root resolution or operator-facing backend prose was introduced.
- **Docs/deviations:** source/provenance/removal/project-report docs and CHANGELOG match ordinary behavior. Keeping picker IO in the hook, omitting an id from non-clickable failure entries, skipping a progress channel absent from both old Re-point commands, and hiding one-tool bulk actions are sound. Version bump is deliberately deferred by ticket 07, not a version-desync defect.

## Gate output

Fresh runs on the reviewed HEAD:

```text
npm run version:check && npm run check
Version OK (1.2.15)
ESLint: passed
Vitest: 15 files passed; 392 tests passed
TypeScript-7 build + Vite: passed
cargo fmt --all -- --check: passed
cargo clippy --all-targets --all-features -- -D warnings: passed
cargo test: 659 passed; 0 failed; 0 ignored

cd src-tauri && cargo test --all --quiet
659 passed; 0 failed; 0 ignored
binary tests: 0; doc tests: 0

git diff --exit-code -- src/bindings/index.ts
no drift
```

Vite retains dynamic-import/static-import and >500 kB chunk warnings. `git diff --check 2145472..HEAD` reports trailing whitespace only in generated `src/bindings/index.ts`; do not hand-edit the generator's output. Working tree was clean after the gates, before this report. No installed-build, Windows junction, or interactive UI smoke was performed; those remain the operator's closure checks.
