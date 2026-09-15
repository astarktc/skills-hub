# 02: Acquisition follows upstream in-repo symlinks; one subpath normaliser

Status: done — 95a87eb

**What to build:** A skill whose upstream publishes it as a symlink inside the repository (an aggregation bundle such as `plugins/tanstack-all/skills/<name> → ../../<name>/skills/<name>`) installs from Explore and refreshes with its real content, on both acquisition paths: a sparse clone adds the link's target to the sparse set (following chains to a bounded depth) and copies the resolved directory; the GitHub Contents API path follows a `type: symlink` entry via its `target`. The skill's recorded subpath stays the alias the operator chose — resolution happens at acquire time, every time, and the resolved path is logged as diagnostics only. A target that is absolute or escapes the repository root is refused with a typed error and never read. The cache and the fetcher share one subpath normaliser.

Source: `../spec.md` Q8 and the "Symlink handling today" facts; `../source-12-followups.md` #1, #13.

**Blocked by:** None (can start immediately)

- [x] Test (clone path): a fixture repo where the requested subpath is a symlink to a sibling directory acquires the target's content; the cache entry's coverage includes the target; the record's subpath is unchanged
- [x] Test (clone path): a chain of two symlinks resolves; a chain deeper than the bound is refused
- [x] Test (API path): a `type: symlink` entry is followed via `target` relative to its directory and the skill's files land
- [x] Test: an absolute target and a `../..`-escaping target are each refused with the typed error and nothing is read from outside the checkout
- [x] Test: cache key / fetcher normalise the same subpath identically through the one shared function
- [ ] The five operator rows (`plugins/tanstack-all/skills/*`) refresh in `tauri:dev` — operator-verified after merge
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 95a87eb. Evidence: Cited 95a87eb (integrated as 55ea334) follows clone links; 968aecc/1c9b7e7 follows API leaf links. src-tauri/src/core/git_acquisition.rs:394 and src-tauri/src/core/github_download.rs:170 call the link resolver. Recorded operator smoke remains unverified, not claimed rerun.

### 2026-09-05 — implementation (branch r4/02-acquisition-follows-upstream-symlinks)

**Shipped** (6 commits on top of main `f057190`):

- `620f85e` refactor(git): one subpath normaliser shared by the cache record and the sparse fetcher — new `core/repo_subpath.rs::normalize_subpath`; both local copies (`git_cache.rs:124`, `git_fetcher.rs:249`) deleted. Test: `the_cache_record_and_the_sparse_pattern_normalise_a_subpath_identically` (`/skills/a/` and `skills/./a` are one coverage entry, one sparse pattern, one hit).
- `e26b60c` feat(errors): `SignalError::SymlinkEscapesRepo { subpath, target }` → `CommandError::SymlinkEscapesRepo` (`SYMLINK_ESCAPES_REPO`, bindings regenerated + committed) → `describeCommandError` branch → EN/ZH `errors.symlinkEscapesRepo`. `repo_subpath::LinkChain` (bound `MAX_LINK_DEPTH = 8`): lexical resolution against the link's own directory within the repo root; absolute / escaping / root-landing target refused typed; hop past the bound refused plain. Tests in the module, `commands/tests/commands.rs`, `commandError.test.ts`.
- `95a87eb` feat(acquire): clone path follows upstream links. `git_fetcher::symlink_on_path` asks the **tree** (`git ls-tree -z HEAD -- <prefixes>` + `git cat-file -p <blob>`), not the working tree — a sparse checkout below a component link materialises nothing, and a leaf link is dangling until its target is checked out. `git_acquisition::follow_upstream_links` loops: link → `LinkChain::follow` → `fetch_through_cache(resolved)` (widens the entry, never narrows; `repo_dir`/`revision` follow the last fetch) → repeat. Copies from the resolved dir; `Acquired.resolved_subpath` stays the alias; resolved path logged at `info`.
- `968aecc` feat(acquire): Contents API path. `GET /contents/<path>` for a link answers **one object** with `type: symlink` + `target` (verified live; listing entries of that type carry no `target`). `github_download` parses either shape (`ContentsResponse` untagged enum), follows via the same `LinkChain`, re-requests the resolved directory. Base-URL seam (`download_github_directory_from`) so mockito drives it. `classify_fast_path_failure` now passes **any** `SignalError` through (was: `Cancelled` only) — a typed refusal is the operator's answer, never retried as a clone.
- `99be01f` docs(context): one sentence on the Git acquisition glossary entry.
- `f4b55d5` fix(acquire): ls-tree paths are patterns; a directory prefix lists its children (the bundle's 18 sibling links). Only a record that *is* one of the prefixes counts. Found by an end-to-end run against the real `tanstack-skills/tanstack-skills` (throwaway ignored test, not committed): both adapters now land the 15,509-byte SKILL.md at revision `6f5521e` (the operator's recorded SHA), coverage = alias + `plugins/tanstack-table/skills/tanstack-table`.

Gate: `npm run version:check` OK (1.2.3); `npm run check` green (lint, vitest 173/173, build, fmt, clippy, cargo 463/463); `cargo test --all` 463/463. Worktree clean.

**Deviations**

- The survivor normaliser lives in a **new module** `core/repo_subpath.rs` (declared in `core/mod.rs`) rather than in `git_cache.rs` or `git_fetcher.rs`: the cache already depends on the fetcher, and the link resolver both acquisition adapters share had to live somewhere neither GitHub- nor git-CLI-specific. Same intent as #13 (one function, both import it).
- The fixture commits `120000` entries via `git hash-object` + `update-index --cacheinfo`, so no `#[cfg(unix)]` gating was needed — the tests run on every platform.
- The depth-bound refusal is a plain `anyhow` error (English, names the link subpath, no absolute path), not a second wire code — the spec types only the escape. The API path's typed refusal pass-through was widened from `Cancelled` to every `SignalError` (one-line change in `classify_fast_path_failure`).
- Symlinks **nested inside** a skill directory (a listing entry of type `symlink`, or a file link in the checkout) are still skipped/ignored as before — out of scope; `copy_dir_recursive` walks with `follow_links(false)`.
- Real target resolution differs from the orchestrator note: `../../<name>/skills/<name>` relative to `plugins/tanstack-all/skills/` is `plugins/<name>/skills/<name>` (not `<name>/skills/<name>` at the root). The fixture models the real layout.

**Notes for the orchestrator**

- Ticket 01's bail line `anyhow::bail!("path not found in repo: {:?}", copy_src)` is textually untouched; the variable it reads is still `copy_src`, now built from `checkout_subpath` (the followed path) instead of `resolved_subpath` (the alias). Their `SubpathMissing { subpath }` should report `resolved_subpath` (the alias) — two lines above.
- `core/mod.rs` gained one line (`pub mod repo_subpath;`); trivial to merge if another lane adds a module.
- Intermediate commit `e26b60c` alone has clippy dead-code errors (the resolver is wired in the next commit); every later commit is gate-green.
- One extra `git ls-tree` + (per hop) one `git cat-file` per clone acquisition with a subpath — ~10 ms each, local. With `git_cache_ttl_ms = 0` each hop refetches (the shipped "never fresh" semantics); the revision recorded is the last fetch's, which is the tree the bytes came from.
- Operator smoke test to run after merge: Refresh (all) on the five `plugins/tanstack-all/skills/*` rows; expect five successes, `source_subpath` unchanged, and `[acquire] followed in-repo symlink … = plugins/<name>/skills/<name>` lines in the log.
