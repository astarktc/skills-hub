# 08: Collapse the git-cache twins, per-key locking, first git-cache tests

Status: resolved

Type: task
Source: `../spec.md` Q21, Q23. Runs in a parallel worktree; independent of the backend chain.

**What to build:** The git cache has one fetch-through-cache entry point (optional subpath selects the sparse fetcher), takes its freshness TTL as a value resolved by the caller instead of a database handle, and writes the corrupt-cache-retry policy once. Locking becomes per cache key so fetches of different repositories no longer serialise — the enabler for parallel Refresh — while the same key still serialises. The module's private-lock invariant from v-next 37 is preserved. The cache-key function gets named, meaningful inputs so a caller cannot put a skill name in the branch slot. The module gets its first tests using local fixture repositories, no network.

**Blocked by:** None (can start immediately; parallel with the backend chain)

- [ ] One public fetch entry; the two previous entry points are gone and all callers migrated with no behaviour change (cache key scheme, meta file, TTL semantics preserved)
- [ ] Two concurrent fetches of different keys proceed concurrently; two of the same key serialise — both with tests
- [ ] Tests cover: cache-key stability, TTL boundary (fresh hit vs stale miss), corrupt cache dir rebuilt once, cancellation
- [ ] No database type appears in the module's interface
- [ ] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Shipped on `arch/08-git-cache` (commit `4047caf`):

- **One entry point.** `clone_to_cache` / `clone_to_cache_subpath` are gone, replaced by
  `fetch_through_cache(cache_dir, &FetchRequest { clone_url, branch, subpath, ttl_ms, cancel })`.
  `subpath.is_some()` picks the sparse fetcher inside a single private `fetch_into`, and the
  corrupt-entry retry policy (remove dir → retry once) is written once.
- **No DB in the interface.** TTL arrives as `ttl_ms: i64`; callers resolve it via the new
  `settings::git_cache_ttl_ms(store)` (= `git_cache_ttl_secs * 1000`). `0 = never fresh` preserved.
- **Per-key locking.** `static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>>`, with
  `key_lock(key)` taking the table only long enough to get/insert the key's mutex. Documented in the
  module header (map lock never held across a fetch). Lock remains private to the module.
- **Typed cache-key inputs.** `repo_cache_key(&CacheKeyInputs { clone_url, branch, subpath })`; the
  explore-cache misuse (`repo_cache_key(source_url, skill_name, None)`) is now the named
  `explore_preview_key(source_url, skill_name)`. Both delegate to one private `hash_key_parts`, so the
  produced strings are byte-identical to what shipped (pinned test).
- **First tests** in `src-tauri/src/core/tests/git_cache.rs` (local fixture repos via the `git` CLI, no
  network): pinned key stability + branch/subpath separation; fresh hit within TTL; `ttl_ms = 0` always
  refetches; aged-meta TTL boundary refetch; corrupt dir rebuilt once; pre-cancelled fetch returns
  `SignalError::Cancelled` and writes no meta; same-key serialisation and different-key
  non-blocking via `key_lock`; two concurrent real fetches of different repos; sparse request gets its
  own entry.

### Deviations

- Added a **pre-fetch cancellation check** and a "don't wipe-and-retry on `Cancelled`" branch in the
  retry policy. Without it a cancelled fetch would delete the cache entry and retry, which is both
  wasteful and contrary to the ticket's "no half-populated cache dir" requirement.
- Added `settings::git_cache_ttl_ms` rather than making every call site write `.saturating_mul(1000)`.
- `explore_preview_key` is a second named key function instead of forcing the explore cache through
  `CacheKeyInputs` — its second component is a skill name, not a branch; that was the misuse.
- Lock table entries are never evicted (bounded, long-lived identities; eviction would let two fetches
  of one key race).

### Follow-ups

- **09 (one acquisition module):** `fetch_through_cache` is the single seam the GitHub-API fast path
  should fall back to; the five `installer.rs` call sites now differ only in `subpath`/`cancel` and are
  ready to collapse into the acquisition module. `settings::git_cache_ttl_ms` is the one DB read to
  hoist there.
- **10 (bounded parallel pool of 4):** per-key locking is in place, so a pool can fan out across
  distinct keys with no cross-repo serialisation; same-key requests still coalesce behind one lock
  (they queue, they do not dedupe — if 10 wants dedupe/single-flight, that is a new behaviour on top of
  `key_lock`).
