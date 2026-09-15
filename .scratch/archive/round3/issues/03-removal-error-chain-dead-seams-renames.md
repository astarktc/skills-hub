# 03: Artifact removal keeps the error chain; delete dead seams; Artifact-removal names

Status: done — ed2080f

**What to build:** When an Artifact removal target fails with a typed condition (for example the registry's refusal to delete a path outside every Tool skills dir), the operator sees that condition's own error code and copy — today the removal report stringifies the error and the command seam revives it as the generic `OTHER`. The report carries the error value itself, the way the Propagation report does, so `CommandError::from_anyhow` classifies it at the seam. Callerless `*_unlocked` twins of the project-removal, assign and resync-all entry points and the store's blind bulk target deletes (forbidden by ADR-0002) are deleted along with their stale "Phase 2" comments — verify each is callerless first; if one has a caller, leave it and say so in Comments. The Rust-internal entry points still worded with "cleanup" are renamed to the *Artifact removal* vocabulary (the ticket picks one consistent scheme); the wire code `DELETE_CLEANUP_FAILED`, the `SignalError`/`CommandError` variants and the i18n keys do not change.

Source: `../spec.md` Q2, Q8, Q9 and the verified-facts section; `../source-13-review-followups.md` #2, #3, #4; ADR-0001, ADR-0002.

**Blocked by:** None (can start immediately)

- [x] Test: a removal failure that is a typed `SignalError` surfaces at the command seam as its own code, not `OTHER`
- [x] Searching the backend for `_cleanup` finds only the error variants, their mapping, and doc comments that quote the wire code
- [x] No `#[allow(dead_code)]` remains for the deleted store functions; the `*_unlocked` twins named in the source ticket are gone or justified in Comments
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — ed2080f. Evidence: Cited ed2080f (integrated as 4cb641d) preserves typed removal errors; src-tauri/src/commands/mod.rs:602. Dead seams/renames followed in 1c9ef62 and 75caa6a.

### 2026-09-04 — implementation (branch `r3/03-removal-error-chain`)

**Shipped**

- `ed2080f` fix(artifact-removal): `RemovalTargetStatus::Failed { error: anyhow::Error }` (mirrors
  `PropagationStatus::Failed`; `Clone`/`PartialEq` dropped from the status and outcome, `Debug` kept —
  the report is consumed once). `commands/mod.rs::to_removal_report_dto` classifies with
  `CommandError::from_anyhow(error)` once per target and clones the `CommandError` onto every member
  row of a shared skills dir; the `anyhow!("{}", error)` revival is gone. Rows are still settled with
  the `{:#}` rendering as `last_error` (ADR-0002). `failures()` / `failed_rows()` / `removed_rows()`
  keep their signatures. Tests: `removal_report_dto_classifies_a_typed_target_failure_at_the_seam`
  (seam: a `PathOutsideToolDirs` failure reaches the DTO as `PATH_OUTSIDE_TOOL_DIRS`, both rows) and
  `a_failed_removal_reports_the_error_chain_not_its_rendering` (a real EACCES removal keeps a
  downcastable `io::Error` root through its context layers). `00b7e10` is the rustfmt follow-up.
- `1853223` refactor(core): callers verified — `remove_project_with_cleanup_unlocked`,
  `assign_skill_to_project_tools_unlocked`, `resync_all_projects_unlocked` were each called only by
  their own locked entry point; bodies inlined into the entry point (doc notes "no composite composes
  it, so no unlocked seam"). `SkillStore::delete_skill_targets` / `delete_all_skill_targets` were
  reachable only from their own two store tests → both functions and both tests deleted. The two
  `#[allow(dead_code)] // Used in Phase 2` attributes (`get_project_skill_assignment`,
  `list_project_skill_assignments_for_project_tool`) removed — both have callers.
- `b351d73` refactor(core): scheme `<row action>_and_[remove_]artifacts` —
  `remove_project_with_cleanup → remove_project_and_artifacts`,
  `remove_tool_with_cleanup → remove_project_tool_and_artifacts`,
  `unassign_and_cleanup → unassign_and_remove_artifacts`; `mutation_guard.rs` module doc,
  `artifact_removal.rs` scope docs, `commands/projects.rs`, the log line and the tests follow.
  "cleanup"-as-verb doc comments reworded (`sync_status.rs`, `skill_store.rs`, `project_sync.rs`,
  `commands/error.rs`, a test banner). Wire code, variants, i18n keys untouched.
- `d9ebf8a` chore(bindings): `cargo test` regenerated `src/bindings/index.ts` — doc-comment text only
  (`DELETE_CLEANUP_FAILED.failures` and `SyncStatus.error` docs), no shape change.

**Acceptance notes / deviations**

- "Searching the backend for `_cleanup`": the only remaining hit outside the error variants and their
  mapping is the test name `remove_skill_keeps_the_skill_and_raises_delete_cleanup_failed_on_partial_failure`,
  which quotes the wire code. Unrelated `cleanup` uses that are *not* Artifact removal were left alone
  on purpose: the `cache_cleanup` / `temp_cleanup` modules, `cleanup_git_cache_dirs`,
  `settings.rs` "disables cleanup", `install_finalize.rs` `cleanup_err`, `central_repo.rs` context
  string — they describe git-cache / temp-dir housekeeping, not Sync-target removal.
- `resync_all_projects_unlocked`: collapsed by inlining (no private helper left behind), so the
  entry point is the whole batch and `resync_project_unlocked` remains the composed seam.
- CONTEXT.md's *Avoid* line (Q2 wording change) is ticket 06's, not touched here.
- Other `#[allow(dead_code)]` in `skill_store.rs` (`db_path`, `get_skill_target`) are not named by
  the source ticket and are outside scope; left as is.
- Flake observed once during a full `cargo test` while three sibling worktrees were compiling:
  `refresh::tests::acquisitions_overlap_instead_of_running_one_at_a_time` (wall-clock bound
  `elapsed < 0.6 × sequential`). Passes in isolation and in the final gate; untouched by this ticket.

**For ticket 04**: `RemovalReport` public shape is stable except `RemovalTargetStatus::Failed.error`
is now `anyhow::Error` (so `RemovalTargetStatus` / `RemovalTargetOutcome` are no longer `Clone` /
`PartialEq`).

**Gate**: `npm run version:check && npm run check` green — vitest 153 (12 files), cargo 430.
