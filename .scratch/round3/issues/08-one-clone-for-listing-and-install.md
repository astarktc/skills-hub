# 08: Add on a non-GitHub host fetches the repository once

Status: done — f532549

**What to build:** Adding a skill from a git host without GitHub coordinates currently clones twice — once to list candidate skills, once (sparse, under a different git-cache key) to install the chosen one. Both steps resolve to the same git-cache key (same normalised URL and ref) so the install is a cache hit within the TTL. What bytes land, the recorded commit SHA, and the GitHub API fast path are unchanged; the cache's single entry point (`fetch_through_cache`) stays the only way into the cache.

Source: `../spec.md` Q6; `../source-13-review-followups.md` #11; AGENTS.md "Git bytes have two single entry points". Skill: **tdd**.

**Blocked by:** None (can start immediately)

- [x] Core test: listing then install of a non-GitHub URL performs one fetch (observed at the cache's fetch seam), and a second install after TTL expiry fetches again
- [x] Existing git-cache and acquisition tests unchanged and green *(see deviation 1 — two key tests encoded the old rule)*
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — f532549. Evidence: Cited f532549 (integrated as 7fa768e) shares the clone cache by repository/ref; src-tauri/src/core/git_cache.rs:90 lets a full entry serve a subpath; regression at src-tauri/src/core/tests/git_cache.rs:382.

### Diagnosis (before any change)

**Why the two keys differ today.** `git_cache::repo_cache_key` hashes three parts —
`sha256(clone_url \n branch \n subpath)` — and the listing and the install disagree on the third:

- `installer::list_git_skills` requests `FetchRequest { subpath: None, .. }` → key `H(url, branch, ∅)`,
  full shallow clone (`clone_or_pull`).
- `git_acquisition::clone_path` (install, Refresh acquire) requests `subpath: Some(known_subpath)` →
  key `H(url, branch, subpath)`, sparse clone (`clone_or_pull_sparse`).

URL normalisation is *not* the cause: both go through the same `parse_github_url` and pass the same
`clone_url`/`branch`. The subpath is folded into the key **only** because the on-disk checkout shape
(full vs sparse-to-`subpath`) was made a property of the directory name rather than of the entry's
metadata — so a full clone could never satisfy a sparse request even though it is a superset.

**Design.** The key becomes `H(url, branch, ∅)` (subpath leaves `CacheKeyInputs`; the digest scheme
and `explore_preview_key` are untouched, so every legacy *full* entry stays valid at its old name —
legacy sparse dirs are orphaned and reaped by the existing age-based cache cleanup). The checkout
shape moves into `.skills-hub-cache.json` (`checkout: full | sparse{subpaths}`; a legacy record with
no field reads as `full`, which is exactly what lives at that key). The hit rule is "fresh **and**
the entry's shape covers the request" (full covers everything; sparse covers a subpath equal to or
below one of its patterns). An entry is **never narrowed**: a fresh-but-uncovered request *widens*
the checkout in place (`sparse-checkout set` with the union / `sparse-checkout disable`) without
moving HEAD, and a stale one refetches with the widened shape. Never-narrow is what keeps a
parallel Refresh safe now that two skills of one non-GitHub repo share a key: a worker copying
`skills/a` out of the dir cannot have it removed by a sibling widening to `skills/b`.

### Shipped (branch `r3/08-one-clone`)

- `f532549` perf(git): key the clone cache by repository and ref, not subpath — `git_cache.rs` (key =
  `H(url, branch, ∅)`; `Checkout { Full | Sparse { subpaths } }` in `.skills-hub-cache.json` with
  `#[serde(default)]` = `Full`; hit = fresh ∧ covers; fresh-but-uncovered → `reshape_checkout` widen in
  place, fetch time preserved; stale → fetch with the widened shape), `git_fetcher.rs`
  (`clone_or_pull_sparse` takes `subpaths: &[&str]`; new `reshape_checkout(dest, Option<&[&str]>, cancel)`
  = `sparse-checkout set --no-cone …` / `sparse-checkout disable` (only when `core.sparseCheckout` is
  true); a full pull on an existing dir now disables a sparse checkout first), plus the first red test
  `a_full_entry_serves_a_later_subpath_request`.
- `86566f9` test(git): `a_sparse_entry_does_not_serve_a_later_full_request`, `widening_does_not_renew_freshness`,
  `a_second_install_after_ttl_expiry_fetches_again`.
- `d3a5462` test(git): `a_second_subpath_widens_the_entry_without_removing_the_first`,
  `a_legacy_record_without_a_checkout_shape_reads_as_full`.
- `c347ccc` test(installer): `add_on_a_non_github_host_fetches_the_repository_once` — the real Add flow
  (`list_git_skills` → `install_git_skill_from_selection`): one clone dir, the recorded `source_revision`
  is the listing's commit although the fixture moved on. Verified red against the pre-change modules.
- `c7a2e4b` docs(agents): the key/never-narrow invariant added to AGENTS.md's git-bytes bullet.

Every new test was mutation-checked (sparse-covers-full, widen-renews-TTL, narrow-instead-of-union,
legacy-reads-as-sparse each kill exactly one test). Gate: `cargo test` 436 passed, vitest 153, lint/build/
fmt/clippy green.

### Deviations

1. **Two existing git-cache tests changed**, not "unchanged": `cache_key_is_pinned_to_the_shipped_scheme`
   pinned a digest *with a subpath* (that key no longer exists; the no-subpath digest is asserted unchanged,
   plus a new independently computed `main` digest) and `cache_key_separates_branch_from_subpath` asserted the
   very behaviour Q6 removes (replaced by `cache_key_separates_branches`). `a_subpath_request_uses_its_own_cache_entry`
   was replaced by its inverse. `git_fetcher`'s sparse test only changed `"skills/a"` → `&["skills/a"]`.
   All acquisition tests are byte-identical.
2. **"Observed at the cache's fetch seam"**: no injectable fetcher seam was added (it would have threaded a
   trait through `git_fetcher`); fetches are observed as the orchestrator allowed — the fixture is advanced
   between calls so a refetch shows as a different head, plus a clone-directory count under the cache root.
3. **Widening rather than "must still fetch"** for sparse-first/full-later: a fresh sparse entry is widened in
   place (`sparse-checkout disable` on the promisor clone lazily fetches the missing blobs) — the listing sees
   the whole tree at the cached head, TTL semantics stay exact. This, and the never-narrow rule, are what make
   sharing one entry safe for a parallel Refresh of two skills from one non-GitHub repo (previously separate
   dirs; now one). Residual: with TTL = 0 ("never fresh") two concurrent workers on different subpaths of one
   repo both refetch, and a `reset --hard` could rewrite files a sibling is copying if upstream moved between
   the two fetches (seconds). Default TTL is 60 s, and 0 is the operator's explicit "never cache".
4. **Wider stale refetch on a full entry**: a stale *full* entry answering a sparse request refetches full
   (never narrow) — slightly more bytes than a sparse pull on Refresh of a repo that was once listed; one dir
   instead of two. GitHub hosts are unaffected (listing clone + API fast path, as before).
5. Old sparse entries at `H(url, branch, subpath)` are orphaned, not migrated; the existing age-based
   `cache_cleanup` reaps them.
6. The libgit2 fallback (`SKILLS_HUB_ALLOW_LIBGIT2_FALLBACK=1`) does not undo a sparse checkout; a sparse
   entry can only have been made by the git CLI, so the mix needs git to vanish between runs.

### For the orchestrator

- `.scratch/` is gitignored (`.gitignore:63`), so this ticket file is **not** in any commit — the
  Comments live only in this worktree's file.
- `AGENTS.md` got one sentence in the git-bytes bullet (ticket 09 / round-3 docs ticket may touch the
  same bullet — trivial to merge).
- Explore preview (`clone_for_explore_preview`) still requests a full clone; it now shares the entry with a
  later install of the same repo too (same benefit, no change needed).
