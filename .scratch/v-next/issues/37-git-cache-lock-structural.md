# 37: Make the `GIT_CACHE_LOCK` non-reentrancy invariant structural

Status: resolved

Type: task
Source: fog item "GIT_CACHE_LOCK made structural" (map), verified at `369dc9b` after the 1.2.0 release; origin arch-scan §installer (`../assets/arch-scan.md`, deadlock fix `82c36d5`)

## What to build

`core/installer.rs:707` declares `static GIT_CACHE_LOCK: OnceLock<Mutex<()>>`, a process-wide
non-reentrant mutex serialising work on the git cache (`<cache_dir>/skills-hub-git-cache`). It is
acquired at three sites:

1. `clone_to_cache` (`:725`) — guards the TTL probe + clone/pull of one repo cache dir.
2. `clone_to_cache_subpath` (`:804`) — same, sparse variant.
3. `clone_for_explore_preview` (`:1058-1085`) — **borrows the git-cache lock to guard a different
   resource**: the explore cache (`<central_dir>/.explore-cache/<key>`). It holds the lock only for
   the hit-probe + `create_dir_all`, then must drop it before `fetch_skill_files` → `clone_to_cache`
   re-acquires it. The reason is a five-line comment ("std::sync::Mutex is not reentrant"); nothing
   enforces it. This was the self-deadlock fixed in `82c36d5`.

Sites 1 and 2 are siblings that never nest, so the invariant already holds between them. Site 3
is the only reason the "callers must not hold the lock" rule exists.

Deepen so the invariant holds by construction:

- Give the explore cache its own lock (e.g. `EXPLORE_CACHE_LOCK`) for the probe/create section in
  `clone_for_explore_preview`, so `GIT_CACHE_LOCK` is never taken outside the two cache-fetch fns.
  Keep the existing behaviour (hit returns early; miss cleans + creates the dir) — only the lock
  identity changes. Delete the non-reentrancy comment once it is no longer load-bearing.
- Move `GIT_CACHE_LOCK`, `RepoCacheMeta`, `repo_cache_key` and the two `clone_to_cache*` fns into
  a new top-level module `core/git_cache.rs` (declare it in `core/mod.rs`; this matches how
  `git_fetcher.rs`, `install_finalize.rs` etc. were split out) with the static **private** to that
  module. `repo_cache_key` is also used by `clone_for_explore_preview` (`:1055`) — export it
  `pub(crate)` from the new module and import it in `installer.rs`. Then no other code can reach the lock, and the module boundary *is* the invariant.
  Public surface: the two fetch functions with their current signatures (`clone_to_cache(cache_dir,
  store, clone_url, branch, cancel) -> Result<(PathBuf, String)>` and the `_subpath` variant).
  If the two fns share their TTL-probe/meta-write body, dedupe it inside the module — but do not
  change observable behaviour (cache key scheme, meta file name `.skills-hub-cache.json`, TTL
  semantics from `settings::git_cache_ttl_secs`, corrupt-cache retry).
- Do **not** touch `update_managed_skill_from_source`'s direct `clone_to_cache` calls or its
  matcher (that is ticket 29 territory, already resolved differently); do not touch
  `fetch_skill_files` beyond the import path.

Add tests (in the module or `core/tests/`): (a) `clone_for_explore_preview`'s cache-hit path
returns without touching the git cache; (b) a regression that would have deadlocked — call
`clone_for_explore_preview` on a miss for a local `file://` repo (the existing installer tests
already build local git fixtures, e.g. `lists_and_installs_git_skills_without_network`; reuse
that helper) under a short `std::thread` timeout so a deadlock fails rather than hangs.

Constraints: `core/` never resolves roots from the environment — keep `cache_dir`/`central_dir`
parameters explicit. No new env vars. `commands/` untouched. `src/bindings/index.ts` must not
change (no DTO edits) — if `cargo test` regenerates it with a diff, something is wrong.

## Acceptance criteria

- [ ] `rg GIT_CACHE_LOCK src-tauri/src` shows the static declared and used only inside one module, and that module has exactly two acquire sites (the two fetch fns).
- [ ] `clone_for_explore_preview` no longer references the git-cache lock; its probe is guarded by its own lock.
- [ ] The non-reentrancy comment is gone (nothing left to warn about).
- [ ] New tests: explore-preview hit path + miss path under a timeout; all existing installer tests pass unchanged.
- [ ] `git diff --stat` touches only `src-tauri/src/core/**` (+ `core/mod.rs` if a new module) — no bindings, no frontend, no docs beyond a one-line CHANGELOG `[Unreleased]` → `### Changed` entry if you judge it user-relevant (it is not; skip unless behaviour changed).
- [ ] `cd src-tauri && cargo test --all` green; `npm run version:check && npm run check` green.

## Answer

Branch `t37`, commit `9ae710d` ("fix(t37): make the git-cache lock invariant structural"),
worktree `~/.worktrees/skills-hub-t37` (not merged).

What moved where:
- New `src-tauri/src/core/git_cache.rs` (declared in `core/mod.rs`) owns the git clone cache:
  `static GIT_CACHE_LOCK` (private), `struct RepoCacheMeta` (private), `pub(crate) repo_cache_key`,
  `pub(crate) clone_to_cache`, `pub(crate) clone_to_cache_subpath` (signatures unchanged).
  Shared body deduped into private helpers `prepare_repo_dir`, `fresh_head`, `write_meta`,
  `log_cache` — cache key scheme, meta file `.skills-hub-cache.json`, TTL from
  `settings::git_cache_ttl_secs`, corrupt-cache retry and log-line text all byte-identical
  (`log_cache` reproduces the sparse/non-sparse label + optional ` subpath=` field).
- `core/installer.rs` imports the three items from `core::git_cache`; dropped now-unused
  `serde::{Deserialize, Serialize}`, `git_fetcher::{clone_or_pull, clone_or_pull_sparse}` and
  `super::settings` imports. `clone_for_explore_preview` now guards its probe/prepare section with
  a new private `static EXPLORE_CACHE_LOCK` and the non-reentrancy comment is gone.
- Untouched: `update_managed_skill_from_source` call sites/matcher, `fetch_skill_files` (imports
  only), `commands/`, `src/bindings/index.ts` (unchanged), no new env vars, roots still explicit params.

Acceptance checks:
- `rg GIT_CACHE_LOCK src-tauri/src` → 3 hits, all in `core/git_cache.rs`: declaration + exactly two
  acquire sites (`clone_to_cache`, `clone_to_cache_subpath`).
- `git diff --stat <merge-base>..HEAD` → only `src-tauri/src/core/{git_cache.rs,installer.rs,mod.rs,tests/installer.rs}`.

New tests (in `core/tests/installer.rs`):
- `explore_preview_cache_hit_never_touches_git_cache` — pre-seeded explore-cache dir keyed with
  `git_cache::repo_cache_key`, unresolvable URL; asserts `<cache_dir>/skills-hub-git-cache` is never created.
- `explore_preview_cache_miss_does_not_deadlock` — local `file://` git fixture, call on a
  `std::thread` with a 30s `recv_timeout` so the pre-`82c36d5` re-entrant lock would fail, not hang.

Gates (run in the worktree):
- `cd src-tauri && cargo test --all` → 326 passed, 0 failed (lib) + 0 bin + 0 doc-tests.
- `npm run version:check` → Version OK (1.2.0).
- `npm run check` → exit 0: ESLint clean, vitest 9 files / 98 tests passed, tsc+vite build ok,
  `cargo fmt --check` clean, `cargo clippy --all-targets --all-features -D warnings` clean, cargo test 326 passed.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
