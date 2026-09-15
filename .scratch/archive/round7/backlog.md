# Round-7 backlog (from the round-6 panel; needs-triage)

1. Orphaned `.skills-hub-old-<uuid>` backups have no reclamation path (`temp_cleanup` sweeps only the cache; `artifact_removal`
   `Skill` scope removes `central_path` only). Age-bounded best-effort sweep in `move_old_central_aside`. (Opus S2)
2. Add calls `matching-refs` twice (listing `installer.rs:448` + install `stored_subpath: None`); thread the listing's resolved
   `GitSource` into `install_git_skill_from_selection_with`. Also closes the split-brain window under rate limiting. (Fable F4, Opus P3)
3. Suffix cache pins pre-fix wrong splits forever (record from `tree/feature/x/skills/foo` stored `x/skills/foo`); fall back to
   `matching_refs` when the hinted branch's SHA lookup 404s. (Fable F5)
4. `finalize_install`: upsert failure after `move_into` leaves untracked bytes; needs new-install cleanup. (R1 comments, Fable F6, Astra P2)
5. `App.tsx`: `activeView === "detail"` with no `detailSkill` renders the library but leaves the view state as `detail`; reset via effect. (Fable F7)
6. Extract one GitHub GET helper (`fetch_branch_sha` / `matching_refs_at` duplicate client/UA/Accept/Bearer/check). Name the
   rollback IIFE in `install_finalize.rs`. (Fable F8)
7. `repointDoor` is a tautological alias of `sourceKind`; give it a body that can diverge or drop it. (Opus S1) — note round-7
   Edit V1 may give it a reason to exist (eligibility); decide then.
8. `list_git_skills` now fails Add listing on a settings-read error (`github_token(store)?`); previously token-free. (Fable)
9. `IGNORE_NAMES`/`is_ignored` duplicated in `content_hash.rs` and `skill_files.rs`. (Fable)

## Triage (grilled 2026-09-09, operator rulings)
- Slice for **v1.2.8** = items 1, 3, 4, 5, 8 (safety + regressions). Items 2, 6, 9 = hygiene, ride along only where a slice item touches the same code (6's GET helper under 3). Item 7 deferred until Edit V1 usage is known.
- Sequencing: base the slice on `main` AFTER Edit V1 (v1.2.7) merges — no parallel lanes over R7-B.
- 1: aged sweep of `.skills-hub-old-*` siblings inside `move_old_central_aside`, not a startup walker. NOTE (round-8 review): every central dir shares one parent, so the sweep is library-wide per Update, not per-skill; the backup is mtime-stamped at creation because `rename` preserves mtime.
- 3: on `branch_sha` 404 with a stored-subpath hint, fall back to `matching-refs`; on success rewrite the record's split (self-heal).
- 4: `finalize_install` upsert failure → remove the just-landed directory (best-effort; error names the path if removal fails). No backup.
- 5: derive the view (`detail` with no `detailSkill` renders as `myskills`), no effect.
- 8: all `github_token(store)?` sites degrade to token-free like Refresh; one `github_token_or_none` helper in settings.
- Release: Edit V1 → v1.2.7; this slice → v1.2.8.
- Lanes: two file-disjoint worktrees — Rust safety (1, 3, 4, 8; Astra medium) and frontend (5; Astra low).
- Review: single Fable reviewer (Anthropic) on the integrated diff, not a 3-seat panel.
- 3 honours acquire-first: the corrected split rides in the acquisition result and is persisted by finalize's existing backfill upsert — no pre-finalize `upsert_skill`.
- 1: age bound 7 days, no marker; failed-rollback backups are named in the typed error and rely on age only.

## Added by the round-7 panel (needs-triage)
10. `frontmatter_edit::header_end` / `parse_invocation_mode` / `parse_skill_md_with_reason` treat an indented `---` inside a block scalar
    (`description: |`) as the closing fence; change all three together. (Opus F2)
11. Upstream converging *to* the override's mode still flags a conflict (base ≠ upstream, override == upstream); could auto-resolve. (Fable)

## Triage (round 9, 2026-09-08)
- Shipped on `main` (7ff30a5, unreleased — pending v1.2.9): 2 (H1, listing resolution reused; `GitSourceResolution` DTO), 6 (H2), 7 (H3, `repointDoor` dropped), 9 (H4), 10 (H5, column-0 fence rule shared), 11 (H6, convergence clears conflict). Round-8 cosmetics: S3 retry named; duplicate no-bearer test kept (covers HTTP header behaviour the settings test does not).
- Backlog is now EMPTY. Round-9 panel findings live in `.scratch/round9/panel/` (architecture-review.html + per-seat reports).

## Round-9 review follow-ups (Fable, no Blocking; `.scratch/round9/r9-review-fable.md`)
12. `GitSkillCandidate.resolution`: drop `#[serde(default)]` (Serialize-only DTO → wire is `T | null`) and the `?? null` at `useAddSkillFlow.ts:185,585`; re-export `GitSourceResolution` from `components/skills/types.ts`. → absorbed by round-10 #3.
13. Name the replay IIFE in `skill_edits.rs:172`; `rollback_update`/`roll_back_update` naming pair in `install_finalize.rs`; collapse `install_git_skill_from_listing` to one `install_git_selection_with` call with a named selection type. → #2 / #3.
14. `.skills-hub-manifest-` prefix: `content_hash.rs:19-25` re-inlines it; move into the shared `is_ignored` or document why `skill_files` must not skip it. → #1.
15. CI: a `--release` run of `update_supplies_a_real_hash_to_copy_assignments_and_reconcile_keeps_synced`; replace its 30 s `try_serialized` poll (`tests/propagation.rs:512-525`). → #1.
16. `SkillDetailView.tsx:260-262` has a fourth fence rule; opening fence still `trim()`s. → #5.
(Done in the 1.2.9 release commit: stale hash comments in `propagation.rs`/`project_sync.rs`, CONTEXT.md Edit/Update/Finalize/Refresh clauses, ADR-0004.)

## Round-10 wave-A review follow-ups (3-seat panel, no Blocking after fix; `.scratch/round10/r10a-review-{fable,opus,astra}.md`)
17. `content_identity::read` warn-once uses a process-global `OnceLock<Mutex<HashSet>>` — ambient state in core; replace with a caller-supplied sink or drop the dedupe. (Fable, Opus)
18. `skill_update`: collapse `UpdateBytes::{GitAcquired, RestoreRebuild}` (identical payload/path) and drop the `unreachable!()`; one acquire door (`acquire_local` beside `acquire_update` or dispatch both); make `UpdateRequest` fields private with Edit/Re-point constructors so callers cannot manufacture the admission protocol (`SourceProposal { ref, subpath, type }` instead of a whole record); reuse `installer::ensure_installable_skill_dir` instead of the re-inlined `is_skill_dir` check. (all three)
19. `content_identity::record` is "hash AND upsert the whole row" — rename or document (`finalize_install` depends on the upsert). (Fable)
20. `same_content` trusts the row for the source side: a central copy edited outside the app can make `overwrite_if_same_content` clobber a divergent target. Follows Q2; ADR note or reconcile-driven invalidation in wave B. (Opus)
21. `status.refreshSummarySkipped` copy says "(could not be located)" — wrong for `skipped_acquisition`; reason-neutral summary. (Fable)
22. `refreshOutcome` toast message is a 4–5-deep nested ternary. (Fable, Opus)
(Fixed in the wave-A release commit: batch-of-one double toast; `is_ignored` comment; AGENTS.md reportOutcome invariant + test bullet; acquisition-skip fixtures.)
