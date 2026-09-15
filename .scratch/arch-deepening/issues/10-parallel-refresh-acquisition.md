# 10: Parallel acquisition in Refresh (all)

Status: resolved

Type: task
Source: `../spec.md` Q23; CONTEXT.md **Refresh (all)**

**What to build:** The acquire phase of the Refresh batch runs over a bounded pool (4 concurrent) instead of sequentially; progress ticks arrive as each skill's acquisition completes; the finalize-and-propagate phase remains serial under the mutation guard. Cancellation stops issuing new acquisitions and waits for in-flight ones. Per-skill outcomes are unchanged in shape.

**Blocked by:** 02, 08

- [ ] Refreshing N git-sourced skills issues overlapping fetches (test with local repos and a latency stub proves wall-clock < sequential)
- [ ] Two skills from the same repository do not corrupt the cache (same-key serialisation from 08 holds)
- [ ] Cancellation mid-batch produces a report with the completed skills and no partial finalize
- [ ] Progress copy (EN + ZH) reflects completion order, not index order
- [ ] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Implemented on `arch/10-parallel-acquisition` (commit `23dad52`).

- `core/refresh.rs`: phase one runs over `std::thread::scope` with
  `min(4, n)` workers (`ACQUIRE_POOL_SIZE`), a `Mutex<usize>` dispatch cursor
  and an `mpsc` result channel. The coordinating thread drains results, so
  `on_progress` still needs no `Send`. Acquire ticks now carry the running
  completion count (`index` = completions so far), i.e. completion order.
  Phase two is untouched: serial, `mutation_guard::serialized` per skill.
- New crate-internal seam `refresh_managed_skills_with(..., acquire: &AcquireFn)`
  for deterministic latency/cancellation stubs. No new crates (std only).
- Cancellation: workers check the token before taking each job; in-flight jobs
  finish. If cancellation was observed at all, phase two runs for no skill and
  every selected skill is reported as `Failed { SignalError::Cancelled }`
  (decision: reuse `Failed` rather than add a `SkillRefreshStatus` variant —
  no DTO/bindings/frontend churn, and no skill was in fact refreshed). Acquired
  `StagingDir`s are dropped, which deletes them.
- Hoisted per-skill DB reads: `settings::github_token` and
  `settings::git_cache_ttl_ms` are read once per batch;
  `acquire_managed_skill_update_with` now takes `ttl_ms`, and the thin
  `acquire_managed_skill_update` wrapper was removed (the batch is the only
  caller and it injects both). Each acquisition builds its own `HttpGithubApi`,
  so no `&dyn GithubApi` crosses threads and the trait keeps needing no
  `Send + Sync`.
- i18n: `actions.refreshFetchStep` now reads "Fetched (i/total) name ..." /
  "已获取 ...". The `{{index}}` variable name is unchanged, so
  `useSkillLibrary.ts` was not touched (ticket 03 merge safety).
- Tests (`core/tests/refresh.rs`): wall-clock overlap (8 skills × 150 ms
  < 60 % of sequential), completion-order progress (slowest skill dispatched
  first ticks last, indices 1..n), cancellation (no `Applying` tick, no
  finalize, `updated_at` unchanged), and two skills from one local fixture
  repo sharing one cache entry with intact `.skills-hub-cache.json` metadata.
- Bindings unchanged (no DTO change).
