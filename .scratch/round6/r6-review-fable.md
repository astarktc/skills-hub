# Round-6 review — fable (`95b7893..6b0ec7d`)

Gate run at HEAD: `cargo test --all` 555 passed, `npm test` 236 passed, `npm run build`, `npm run lint`, `npm run version:check` all green; `git status` clean (no binding drift). Worktree safety: `git diff 95b7893..HEAD -- src-tauri/src src/` deletions are only the hunks the tickets replaced (old `remove_dir_all` in `finalize_update`, the old intent/backfill block in `installer.rs`, `detailSkill` state in App, the `kind === "git"` button test) — nothing from `95b7893` reverted.

## Standards

**Hard (documented standard)**

1. `installer.rs` (AGENTS.md worktree-safety "highest-risk shared files"; ticket 03 "Files: `git_acquisition.rs`, `github_api.rs` … their tests"; spec D4 "minimal, additive"). The lane reworked intent selection and dropped the pre-finalize backfill `upsert_skill`. The removal is actually *better* (acquire-first, no DB write before finalize) but it is unticketed scope in a protected file — see Spec (b)/(c) for the behavioural consequence it hides (Re-point hint bug).

2. AGENTS.md "Frontend presentation logic … lives once" / spec D9 "one pure predicate used by both call sites": two Re-point decisions still bypass `repointDoor`:
   - `useSkillLibrary.ts:166` `managed.id === skill.skill_id && sourceKind(managed) === "git"` (offers the git Re-point action);
   - `SkillDetailView.tsx:531` `sourceKind(skill) === "git" && onRepoint` (detail-view Re-point button).
   Judgement: minor, but it is exactly the "decided twice" the ticket closed.

**Judgement calls (baseline smells)**

- *Duplicated Code* — `github_download.rs` `matching_refs_at` re-implements the client build / `User-Agent` / `Accept` / `Bearer` / `check_github_response` sequence of `fetch_branch_sha` line-for-line. Extract one `github_get(url, token) -> Result<Response>`.
- *Middle Man* — `skillPresentation.ts` `repointDoor` is `sourceKind(skill) === "git" ? "git" : "local"`; it adds a name, not a rule. Acceptable because D9 asked for a named door, but the name only pays off if every site uses it (see 2).
- *Duplicated Code* (inherited, now touched) — `IGNORE_NAMES`/`is_ignored` exist in both `content_hash.rs` and `skill_files.rs`; the symlink rule was added to one walker only (correct — the viewer lists files only), but the drift risk is now real.
- `App.tsx` — `activeView === "myskills" || activeView === "detail"` renders the library while `activeView` stays `"detail"` and `detailSkillId` stays set; a *Mysterious State* (view says detail, screen shows list). Cheap fix: `useEffect` in App/hook that calls `setActiveView("myskills")` when `activeView === "detail" && !library.detailSkill`.
- `install_finalize.rs` `rollback_update`: the IIFE closure `(|| -> Result<()> { … })()` is a private-fn-shaped block; a named `fn restore_old_central(central, backup) -> Result<()>` reads better and is unit-testable without the wrapper.

## Spec

**(a) Missing / partial**

- D9 partial — two `sourceKind`-based Re-point decisions remain (Standards 2).
- D4 "call GitHub `git/matching-refs/heads/<seg1>` **once**": Add now calls it twice for the same URL — `list_git_skills` resolves (`installer.rs:448`) and `install_git_skill_from_selection_with` passes `stored_subpath: None` (`installer.rs:573`) so `acquire` resolves again. Unauthenticated budget is 60/h; worth threading the listing's resolved branch into the selection install.
- Ticket 01 "Check whether `finalize_install` has an analogous hole … report": reported in comments only; nothing in code/docs says "an upsert failure after `move_into` leaves untracked bytes under the final name → next Add of that name hits `SkillExists`". Should land as a doc note on `finalize_install` or a round-7 ticket.

**(b) Not asked for**

- `git_acquisition.rs::acquire` intent rewrite: `NamedSkill | NamedSkillOrWholeRepo` with resolved `subpath == "."` → `Subpath(".")` ("An explicit URL root … is a selection"). Not in D4. It makes `tree/feature/x` (branch consumes the whole path) behave differently from `tree/main`: the parser yields `subpath: None` for `tree/main` (name discovery runs), while `resolve_tree_source` yields `Some(".")` and the rewrite forces the repo root. For a multi-skill repo on a slash branch, Refresh of a legacy record, Explore preview (`clone_for_explore_preview`, `NamedSkill(skill_name)`) and Re-point stop discovering by name. Fix: have `resolve_tree_source` set `subpath = None` when the branch consumes the tree path and delete the rewrite.
- `list_git_skills` now calls `settings::github_token(store)?` — a settings-read error newly fails Add listing (previously token-free). Minor.

**(c) Implemented but wrong**

- **Re-point applies the old skill's `source_subpath` as the branch hint against the new URL** (`installer.rs:323` `stored_subpath: record.source_subpath.as_deref()` is passed unconditionally, including when `source_override.is_some()`). D4's cache rule exists so "Refresh/Update/Re-point pay nothing *after the first resolution*" — a hint recorded from the *same* URL. On Re-point the hint comes from a different source. Trace at HEAD: record `source_subpath = "old"` (fixture shape `refresh.rs:244`), operator re-points to `https://github.com/owner/repo/tree/main/skills/old` (the maintainer moved the skill into `skills/` — the Re-point use case). `resolve_tree_source`: `tree_path = "main/skills/old"`, `strip_suffix("/old")` → branch **`main/skills`**, subpath `old`. `fast_path_coords` → `branch_sha("main/skills")` → 404; `branch_assumed` is false (`req.source.branch.is_some()`), so `classify_fast_path_failure` raises `GithubSkillNotFound { url: ".../tree/main/skills/old" }` — the typed "not found" for a URL that exists, never retried as a clone (the clone would use the same wrong branch anyway). Existing tests only re-point `old` → `main/new` (no suffix overlap), so the case is uncovered. Fix: `stored_subpath: if source_override.is_some() { None } else { record.source_subpath.as_deref() }` + a `refresh.rs` test for the suffix-overlap Re-point.
- Related trap (follow-up): a pre-fix record installed from `tree/feature/x/skills/foo` stored the parser's wrong split (`x/skills/foo`); the suffix rule now *pins* that split (`explicit_selection_is_not_rewritten_even_when_it_equals_the_parsed_url_path` codifies it). Such records stay 404 until re-pointed; consider a fallback to `matching_refs` when the hinted branch's SHA lookup 404s.

**D1 walk (confirmed sound)** — rename-aside failure before anything moved → old bytes+row intact (test `readonly_parent`); move failure → `.old` restored (`staging_is_missing`); upsert failure → new bytes removed, `.old` restored, real SQLite trigger (no hook); rollback failure → original error is root cause, context names the backup path (`rollback_rename_failure…`); cleanup failure → warn only. Missing-central + upsert failure leaves central missing (Unlocatable, row unchanged) — acceptable, tested. Fresh UUID per attempt → a retained `.skills-hub-old-*` never blocks the next Update; hidden name is skipped by the root/recursive discovery ladder (tested); listing/Unlocatable/`EveryGlobalTarget`/`Skill` scopes are row-driven, onboarding scans Tool dirs only. Crash between rename-aside and `move_into` → `central_missing` with an orphan `.old` sibling: not crash-atomic, as the doc now states; `move_central_repo` leaves such siblings behind (row-driven) — recovery is manual, per D1.

**D3 walk (confirmed)** — `hash_dir` and `copy_dir_recursive` both walk `follow_links(false)` and skip `file_type().is_symlink()` (a directory link is one entry, never descended, in both). A skill root that is itself a symlink (Tool target → central) is still followed: walkdir 2.5 `follow_root_links` defaults to true, so `target_has_same_content` on a link target is unchanged. Hashes of symlink-free skills are unchanged (only the `continue` was added). Onboarding fingerprint (`onboarding.rs:79`) and import identity (`onboarding_import.rs:312/372`) both go through `hash_dir`.

**D5–D8** — `settleSingleReport(action, report) ?? true` is correct: `ActionExit` is an object, so `??` keeps it and `runAction` returns `undefined`; the modal closes only on `true` (both outcomes tested). `handleUpdateManaged`'s return type widened to `Promise<true | undefined>`; its only consumer `void`s it. `errors.skillGone` present in `en` and `zh`. Explore detail keeps its own `exploreDetailSkill` object; the install handoff reads the merged `detailSkill` — works.

## Summary

**Blocking**
1. `installer.rs:323` — Re-point feeds the *old* record's `source_subpath` as the branch hint for the *new* URL; when it is a suffix of the new tree path (skill moved deeper, e.g. `old` → `tree/main/skills/old`) the branch becomes `main/skills` and the operator gets a typed `GithubSkillNotFound` for a valid URL. Pass `None` under `source_override`; add the test.
2. `git_acquisition.rs::acquire` intent rewrite + `resolve_tree_source` `subpath = "."` — unticketed behaviour that makes `tree/<slash-branch>` (no path) skip name discovery, unlike `tree/main`. Yield `None` and drop the rewrite (also removes the `./skills/foo` subpath shape `git_candidates_in` would mint for such listings).

**Follow-up**
3. D9 incomplete: switch `useSkillLibrary.ts:166` and `SkillDetailView.tsx:531` to `repointDoor`.
4. Add makes the `matching-refs` call twice (listing + install); thread the resolved branch through.
5. Suffix-cache pins pre-fix wrong splits forever; fall back to `matching_refs` when the hinted branch 404s on SHA lookup.
6. `finalize_install` upsert-failure hole: record it in code/docs or a ticket (comments-only today).
7. `App.tsx` "detail with no skill" leaves `activeView === "detail"`; reset the view instead of aliasing the render.
8. Extract one GitHub GET helper (`fetch_branch_sha` / `matching_refs_at`); name the rollback closure.
