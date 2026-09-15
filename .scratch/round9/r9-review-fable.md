# Round-9 review — v1.2.9 (`96cd33c` → `0acafd7`)

Gate re-run at HEAD: `version:check` OK (1.2.9); `cargo test --all` 595 passed, bindings unchanged after regen (`git status` clean); `npm test` 237 passed; `npm run build` green. Full `git diff 96cd33c..HEAD -- src-tauri/src src/` read hunk by hunk: every deletion maps to an H/D item; no v1.2.7/v1.2.8 reverts.

## Standards

**Hard (documented rule)**
- AGENTS.md IPC fidelity: "`#[serde(default)]` *input* fields become `field?:`; `Option<T>` is wire-accurate `T | null` (serde always emits the key; never `?`)". `GitSkillCandidate` (`installer.rs:405-412`) is Serialize-only, so `#[serde(default)]` is a serde no-op that only mis-documents the wire as `resolution?:` (`bindings/index.ts:282`) and forces `candidate.resolution ?? null` in `useAddSkillFlow.ts:185,585`. Ticket H1 asked for it, but frontend/backend never skew in a Tauri bundle. Judgement: drop the attribute + `?? null`.
- AGENTS.md "Adding a DTO = … re-export from the shim": `GitSourceResolution` is not re-exported from `components/skills/types.ts`. Nothing imports it today, so cosmetic.
- Stale comments contradicting D1 (files were off-limits to the implementer): `propagation.rs:378` and `project_sync.rs:102` still say the hash is "absent when finalize did not compute one" — after D1 absence means an I/O failure only. `CONTEXT.md:36` ("flagged until the operator re-chooses or clears") predates H6's convergence rule; `:44/:56/:92` say "finalizes, then replays" — replay now runs *inside* finalize's window.

**Judgement (Fowler)**
- *Divergent change / inconsistency*: H2 removed the rollback IIFE, D2 introduced a new one (`skill_edits.rs:172-178`, `let applied = (|| {…})()`). Name it (`apply_replay`).
- *Confusing names*: `rollback_update` now wraps `roll_back_update` (`install_finalize.rs:454/471`) — two spellings of one verb.
- *Data clump / primitive obsession*: `install_git_skill_from_listing` takes `(&str, Option<&GitSourceResolution>)`; and it detours through `install_git_skill_from_selection` for `None` although both branches end in `install_git_selection_with` with an identical `HttpGithubApi` — dead branch.
- *Duplicated predicate* (H4 half-done): `content_hash.rs:19-25` re-inlines the `.skills-hub-manifest-` check the shared `is_ignored` lacks; the two callers still disagree on that prefix.
- Timing-based test: `tests/propagation.rs:512-525` polls `try_serialized` for up to 30 s.
- `commands/mod.rs:363` uses a fully-qualified path instead of importing `GitSourceResolution`.

## Spec

**D2** — all scrutiny points hold at HEAD:
- `previous` is read from the store before `move_old_central_aside` (`install_finalize.rs:283-285`); Re-point's override lives only in the input `record` (`installer.rs:341-346`), so a failed Re-point Update restores the OLD `source_ref`/`source_subpath` (test `finalize_step_failure_restores_persisted_row_even_if_input_and_settlement_changed_it`). `upsert_skill` writes every column incl. `content_hash`, `updated_at` (`skill_store.rs:512-555`) ⇒ byte-for-byte old row (Debug-equality asserts in `failed_update_replay_restores_bytes_skill_and_edit_then_retry_succeeds`).
- Failure after the manifest write but before `record_hash`: replay re-upserts the Edit snapshot, finalize restores bytes + pre-finalize row; nothing half-settled (the `"hash"` fault case covers exactly this).
- Unlocatable Restore: row exists, so `get_skill_by_id` passes; `finalize_update_restores_missing_central_on_success` and `refresh.rs:1620` still green. Only caller is `finalize_and_propagate_unlocked` (`installer.rs:388`). Acquire-first untouched (no upsert before finalize on Add/Update). Double faults named on both sides (`restore pre-finalize skill row…`, `restore pre-replay Edit row…`, tests present).
- Residual (unchanged shape): rollback-failed + row-restored leaves old row over new bytes with the backup named — same as the pre-existing upsert-failure path.

**D1** — env flag + `should_compute_content_hash` gone; AGENTS clause updated; new test drives `refresh_managed_skills` through the real supplier and compares to `hash_dir` (no literal). Partial: the "release-mode" regression is proven only structurally (the `cfg!` branch no longer exists); `cargo test --all` is still debug-only. Stale comments listed above.

**H1** — one `matching-refs` per Add (counting double asserts 1 and a disagreeing second answer); legacy `(subpath, None)` still resolves; Re-point keeps `stored_subpath: None`; `resolution` optional both ways. Note: `resolution.subpath` is never consumed (intent `Subpath(candidate.subpath)` overrides it) and the listing's `TreeSplit` origin is dropped (`Parser` passed) — harmless, since a listing that succeeded already validated the branch.

**H5** — one `header_end` (`skill_discovery.rs:322`), closing fence `trim_end() == "---"`, used by all three parsers + both `frontmatter_edit` sites; test covers block-scalar `  ---`, trailing whitespace on the fence, and the byte-preserving Edit write/restore. Scope note: the opening fence still tolerates leading whitespace (`trim()`), and `SkillDetailView.tsx:260-262` has a fourth, unshared fence rule (`\n---` prefix, assumes exactly `---\n`).

**H6** — `edit.conflict = upstream != override && (edit.conflict || disagrees)`: clears only on convergence; unchanged upstream keeps a set flag; genuine disagreement flags. Both ticket cases tested.

**H7** — `retry_stored_hint` extracted; the `settings_read_failure_sends_no_bearer_to_github` test was kept (defensible: it asserts the missing `Authorization` header at the HTTP seam, which `tests/settings.rs:131` does not), a deviation from the ticket's default that should be stated.

**H2/H3/H4** — as claimed (`repointDoor` gone, 4 sites on `sourceKind`, describe block removed; one `pub(crate) is_ignored`).

## Summary

**Blocking** — none. Every D1/D2 scrutiny question verified at HEAD; gate green.

**Follow-up (round 10)**
1. Fix stale hash comments `propagation.rs:378`, `project_sync.rs:102`; refresh `CONTEXT.md:36,44,56,92` for convergence + replay-inside-finalize.
2. Drop `#[serde(default)]` on `GitSkillCandidate.resolution` (+ `?? null`), re-export `GitSourceResolution` from the shim.
3. Name the replay IIFE; rename `roll_back_update`/`rollback_update` pair; collapse `install_git_skill_from_listing` to one call of `install_git_selection_with` with a named selection type.
4. Move the `.skills-hub-manifest-` prefix into the shared `is_ignored`, or document why `skill_files` must list it.
5. Consider a `--release` run of `update_supplies_a_real_hash_to_copy_assignments_and_reconcile_keeps_synced` in CI; replace its 30 s poll.
6. Unify the frontend `parseFrontmatter` fence rule with `header_end` (H5 residue).
