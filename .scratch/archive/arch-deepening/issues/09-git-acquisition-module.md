# 09: One git acquisition module with the API fast-path enabled for install

Status: resolved

Type: task
Source: `../spec.md` Q21, Q22; CONTEXT.md **Skill discovery**, **Skill candidate**, **Finalize (install)**

**What to build:** "Given a parsed git source and an intent (subpath or skill name, cancel token, fast-path allowed), land the skill's bytes in a directory and tell me the revision and which strategy was used" is answered by one core module. The GitHub API download and the git clone are its two adapters at one seam — fast-path first when applicable, clone as fallback — with typed GitHub outcomes (not found, rate limited) everywhere. Install-from-selection, update (via Propagation's acquire phase) and Explore preview become adapters that only choose the destination and hand the result on; cancellation and sparse fetching therefore reach install and update too. The stale "shared by update and preview" documentation disappears with the duplication.

**Blocked by:** 02, 08

- [x] Install-from-selection with a subpath uses the fast-path when available and records the real commit SHA; falls back to clone on API failure, with tests for both
- [x] Cancelling an install mid-acquisition aborts cleanly (test)
- [x] Typed not-found / rate-limited errors reach the operator from install and update, not only from preview
- [x] Multi-skill name matching and subpath backfill exist once, in the acquisition module
- [x] Existing installer, preview-cache and git-fetcher tests pass; acquisition gets its own test file using local repos and a stubbed HTTP adapter
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

**Shipped** (branch `arch/09-git-acquisition`, commit `69da535`)

New `core/git_acquisition.rs` (538 lines) answers the whole question once:
`acquire(&AcquireRequest, &dyn GithubApi) -> Result<Acquired>` with
`GitSource { clone_url, branch, subpath, api: Option<GithubRepo> }`,
`SkillIntent::{ Subpath, NamedSkill, NamedSkillOrWholeRepo }`,
`Acquired { revision, strategy: GithubApi | GitClone { sparse }, resolved_subpath }`.
`parse_github_url` and its helpers moved here (a `GitSource` is what it parses),
along with `installable_skills_in_repo`, the multi-skill name matching and the
legacy subpath backfill — each existing exactly once now.

Policy: fast path when `allow_fast_path && source.api.is_some()` and the intent
names a real subpath; the branch SHA is fetched **before** the download so a
served fast path always records the real commit (and a bad branch fails before
bytes land). GitHub 404 → `SignalError::GithubSkillNotFound`, 403 →
`SignalError::RateLimited` — never a silent clone; any other API failure logs and
falls back to a clone (sparse when a subpath is known). Cancellation is checked
at entry and between phases and is never treated as a failed strategy.

`installer.rs` lost ~500 lines: `fetch_skill_files`, `find_skill_by_name`,
`installable_skills_in_repo`, `ParsedGitSource`/`parse_github_url` and the
duplicated fast-path + fallback blocks. The three flows are thin adapters that
choose a destination (Staging dir / explore-cache dir) and pass the result on.
The explore cache keeps its own lock/probe — it guards the preview directory,
not acquisition.

**Deviations (with reasons)**

1. `SkillIntent` has three variants, not the ticket's `Subpath / NamedSkill /
   WholeRepo`. `WholeRepo` is `Subpath(".")` (that is exactly how install spells
   it), and the two *named* flows have genuinely different admission rules that
   both had to be preserved: preview/Explore must raise `MultiSkills` when a name
   does not resolve, while the update backfill must fall back to the whole repo
   rather than fail an update that used to work. Naming them
   `NamedSkill(Option<&str>)` (strict) and `NamedSkillOrWholeRepo(&str)`
   (lenient) makes that difference a fact of the type instead of a hidden flag.
2. `GitSource.api` carries `GithubRepo { owner, repo }`, not full API params: the
   branch/subpath coordinates depend on the *intent*, which is only known at
   request time. `github_download::parse_github_api_params` was therefore folded
   into `parse_github_repo(clone_url) -> Option<(owner, repo)>` (its four tests
   became one repo-parsing test plus acquisition-level eligibility tests), and
   `fetch_branch_sha` lost its `#[allow(dead_code)]` — it is live code now.
3. Two `pub(crate)` injection seams added — `install_git_skill_from_selection_with`
   and `acquire_managed_skill_update_with` (each taking `&dyn GithubApi`; the
   public functions delegate with `HttpGithubApi`). Without them the fast path
   through the *real* install/update wiring (staging → finalize → record) could
   only be proven by inspection, because `parse_github_url` on a local fixture
   path yields no GitHub coordinates and a real github.com URL would need the
   network. The stubs prove install records the API's SHA, install surfaces a
   typed 403, and update surfaces a typed 404.
4. `install_git_skill_from_selection` now fetches **sparsely** when the subpath is
   not `"."` (it used to full-clone). The ticket asks for sparse fetching to reach
   install; note it needs the system `git` CLI (`clone_or_pull_sparse` has no
   libgit2 fallback) and uses a different git-cache key than a full clone.
5. `install_git_skill_from_selection` and `acquire_managed_skill_update` /
   `refresh_managed_skills` gained a `cancel: Option<&CancelToken>` parameter; the
   `install_git_selection` and `refresh_managed_skills` **commands** take the
   app's `Arc<CancelToken>` as Tauri `State` and `reset()` it. `State` does not
   cross the wire, so `src/bindings/index.ts` is byte-identical and no frontend
   change was needed (ticket 11's hook is untouched).
6. `ensure_installable_skill_dir` for install now checks the staging dir after
   acquisition instead of the repo dir before the copy — same bytes, same typed
   `SkillInvalid { missing_skill_md }`, and it works for both adapters.
7. Stale doc removed: the `fetch_skill_files` mention in `core/skill_matching.rs`'s
   module doc now names the acquisition module's named intents.

**Verification**

`npm run version:check` (1.2.1 OK) · `npm run check` exit 0 ·
`cd src-tauri && cargo test --all` exit 0, 384 passed / 0 failed
(21 new acquisition tests + 5 new installer tests). `lens_diagnostics mode=all`:
no blocking errors; only pre-existing warnings in files this ticket did not touch.
`git status src/bindings/index.ts` clean after `cargo test` — no wire change.

**Follow-ups for ticket 10 (parallel acquisition)**

- `acquire` is `Send`-friendly: no `Rc`, no thread-locals, inputs shared only as
  `&Path` / `&str` / `&CancelToken`. `git_cache` locking is already per key, so
  workers on different repos do not serialise.
- `GithubApi` deliberately has **no** `Send + Sync` supertrait (the test stub uses
  `RefCell`). A pool should build one `HttpGithubApi` per worker — it is a
  one-field struct over the token — or add the bound then, when a real
  `&dyn GithubApi + Sync` is required.
- The two DB reads acquisition needs (`settings::git_cache_ttl_ms`,
  `settings::github_token`) are still done per skill inside
  `acquire_managed_skill_update`. Hoist both into the batch when `refresh.rs`
  phase one goes parallel: they are pure values on the request.
- `refresh_managed_skills` now takes the cancel token; a pool should check it
  between dispatches so a cancelled batch stops scheduling instead of failing N
  skills one by one.
