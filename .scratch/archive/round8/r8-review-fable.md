# Round-8 review — v1.2.8 safety slice (Fable)

Reviewed `git diff 51dcae6...HEAD` (HEAD `3190623`, 5 commits, 11 files) against `issues/01-rust-safety.md` (S1–S4), `issues/02-detail-view.md`, `spec.md`, AGENTS.md, CONTEXT.md, ADRs 0001–0003. Gate re-run at HEAD: `version:check` OK (1.2.7), `cargo test --all` 585 passed, `npm run build`/`lint`/`test` (242) green, `src/bindings/index.ts` clean. Worktree-safety diff read hunk by hunk: every deletion is the S2 doc note, the duplicated GET client in `github_download.rs`, or an `activeView`→`effectiveView` rename — no Edit V1 (`skill_edits.rs`, `frontmatter_edit.rs`) or v1.2.7 code reverted.

## Standards

**Hard (documented-standard breaches)** — none found.
- Core resolves no roots (sweep keys off `central.parent()`; token read stays at the settings seam). No new command/DTO/i18n surface, bindings unchanged.
- Error contract (ADR 0001): S2's double fault is `anyhow` context on the original `rusqlite` source (test downcasts it) — the ticket explicitly excluded a typed variant, and the copy is a diagnostic path string, not user prose.
- Acquire-first (round-6 D4) holds: `stale_split_repair_is_acquire_first_and_persisted_only_by_finalize` uses a `RAISE(ABORT)` trigger on `skills` during acquisition.
- Log prefixes follow the `[install]`/`[acquire]`/`[settings]` convention.

**Judgement (baseline smells)**
1. *Long Method / nesting* — `git_acquisition.rs::acquire_resolved` now carries the repair policy 7 levels deep inside the `Err(failure)` arm, and `let req = original_req;` exists only to be re-shadowed by `resolved_req`. Extracting `repair_stored_split(original_req, failure, api) -> Option<(GitSource, SkillIntent)>` would leave `acquire_resolved` readable and make the "bounded to one" property visible (`split == TreeSplit::StoredHint` guard) instead of implicit in the recursion.
2. *Duplicate test* — `github_download.rs::settings_read_failure_sends_no_bearer_to_github` builds a `SkillStore` inside the HTTP module's tests to prove what `tests/settings.rs::token_read_failure_degrades_only_for_acquisition_not_settings_page` and `tests/installer.rs::settings_read_failure_does_not_block_listing_install_or_preview` already prove. Drop it or keep only the mockito half (token `None` ⇒ no `Authorization`).
3. *Environment-coupled test* — `finalize_update_backup_sweep_does_not_follow_symlinks` shells out to `touch -h` (macOS/GNU; busybox differs). `cfg(unix)`-gated so CI is fine; note it.
4. `TreeSplit` is `pub(crate)` with no doc comment while its sibling enums are documented; `list_git_skills` discards it as `(parsed, _)` — fine, but a one-line doc would help.

## Spec

**S1 — backup sweep.** Parent-local (`read_dir(central.parent())`), exact prefix `.skills-hub-old-`, `symlink_metadata` (links removed as links — unix test proves target bytes survive; `remove_dir_all` on a link root does not follow on macOS/Linux/Windows in std ≥1.58.1), best-effort warn on failure (test), future/unreadable mtimes skipped. Staging dirs are `.skills-hub-staging-*`, so a parallel Refresh's staging is never matched.
- **Scope reality**: every skill's central path shares one parent (the central repo root), so "siblings of that central path" is a *repo-wide* sweep, not per-skill. The ticket's own test line ("a sibling of a different skill's central path is untouched") is satisfied vacuously by `other/.skills-hub-old-aged` — a child of another skill dir, which is not where any backup ever lives. Repo-wide is the only meaningful reading and is acceptable given the prefix, but the spec's "per-skill sweep" wording is inaccurate; record it.
- **Age clock is wrong — Blocking (data safety).** The ticket bounds age by `modified()` and the implementer reads the backup dir's mtime; but `move_old_central_aside` creates the backup with `rename`, which preserves the directory's mtime on every shipped platform (verified: `touch -t 2000… d; mv d .skills-hub-old-x` → mtime still 2000). The backup's mtime is therefore the *previous install/update time of the skill*, not when the backup was made. Any skill last updated >7 days ago (the normal case for Refresh) whose backup lingers after a failed cleanup or failed rollback is sweep-eligible **immediately**, and the sweep runs on the *next Update of any skill* — including the operator's natural retry of the very skill whose `FinalizeRollbackFailed` error just named that backup. The doc comment "Recent backups remain available for manual recovery" and the ticket's "relies on age alone" contract are both false at HEAD. Neither test catches it because both age their fixtures explicitly rather than through a real `finalize_update` backup. Fix: stamp the backup at creation — after the `rename` in `move_old_central_aside`, best-effort `File::open(&backup).and_then(|f| f.set_modified(SystemTime::now()))` (Windows needs `FILE_FLAG_BACKUP_SEMANTICS`, the test helper already shows the flags) — plus a test that runs `finalize_update` on a skill whose central dir mtime is aged, forces the post-success cleanup to fail (or reads the backup straight after a forced rollback failure), and asserts the *next* `finalize_update` keeps it. Pre-existing v1.2.7 orphans keep their old mtimes and will be swept on first use; call that out in the changelog or accept it.

**S2 — install cleanup.** `remove_dir_all(&central_path)` on upsert failure; `NotFound` treated as clean; double fault names the path with the upsert error as source (both tests). "Known gap" doc note removed. Second Add of the same name succeeds (test). Complete.

**S3 — refs fallback.** Retry only when `split == StoredHint && stage == Sha && 404`; re-resolve with `stored_subpath: None`; retry recurses with `origin == MatchingRefs` so a second 404 falls to `classify_fast_path_failure` (test `…retries_sha_only_once`); non-matching/erroring refs keep typed `GithubSkillNotFound`; hint that resolves makes no refs call (`stored_suffix_resolves_without_discovery…` unchanged); download-stage 404 does not retry (test); `classify_fast_path_failure` untouched; `Subpath` intent rewritten only when equal to the stored hint; Re-point still passes `stored_subpath: None` (`installer.rs:326`). `resolved_subpath` flows to `record.source_subpath` in acquisition, persisted by finalize only (trigger test). GET helper preserves UA/Accept/Bearer/`check_github_response`; existing mockito tests unchanged. Complete. Minor: when the corrected branch consumes the whole tree path, the intent becomes `Subpath(".")` → clone of the whole repo with `resolved_subpath: None` — coherent with the no-hint path, just note it is unexercised.

**S4 — token-free listing.** `github_token_or_none` at all five sites (`grep` shows no remaining `github_token(store)?` outside `load_settings`, which still surfaces the error — test). Warn-logged once per read. Complete.

**Frontend.** `effectiveView` derived once, no effect, every render/nav read replaced; the only raw `activeView` uses left are the `useState` declaration and `setActiveView` writers. `explore-detail` path unchanged (`exploreDetailSkill`). `handleViewChange`'s `closeDetail` branch keys on the requested view, not state — unaffected. Complete.

**Scope creep**: none. **Version**: still 1.2.7 — the v1.2.8 bump (`npm run version:set 1.2.8`) is outstanding before shipping.

## Summary

**Blocking (must fix before v1.2.8)**
1. **S1 age clock** — `rename` preserves mtime, so the 7-day recovery window is measured from the skill's last content change, not from backup creation; a lingering backup of any skill older than a week is deleted on the next Update of any skill, including the retry the `FinalizeRollbackFailed` error invites. Verified at HEAD (`install_finalize.rs:323-334` rename without a timestamp touch; `sweep_old_central_backups` reads `metadata.modified()`). Fix by touching the backup's mtime on creation + a test that goes through a real `finalize_update` backup.

**Follow-up**
- Spec wording: the sweep is central-repo-wide by construction; retitle "per-skill" and fix the vacuous "different skill" test to something that can fail (e.g. a fresh backup left by another skill's failed cleanup is kept).
- Extract the S3 repair block out of `acquire_resolved` (nesting/shadowing smell).
- Remove the duplicate settings test in `github_download.rs`.
- `npm run version:set 1.2.8` before release; CHANGELOG entry for the sweep (including the one-time sweep of pre-1.2.8 orphans).
