# 02 Release-mode regression run for content identity in CI

Status: done — 8cf55c3
Lane: B (first; 03 follows in the same lane)
Source: BACKLOG #10 (from `archive/round7/backlog.md` #15), narrowed by round-14 D2

The 30 s `try_serialized` poll named in the source is already gone
(`src-tauri/src/core/tests/propagation.rs:534` uses `mutation_guard::serialized` directly) — do not look for it.

What remains: `update_supplies_a_real_hash_to_copy_assignments_and_reconcile_keeps_synced`
(`tests/propagation.rs:473`) guards a content-hash bug that surfaced only in release builds; CI runs `cargo test --all`
in debug only (`.github/workflows/ci.yml` rust job, `:53`).

Add a `--release` run to the rust job. Prefer a targeted step after the debug `cargo test --all`:
`cargo test --release --all propagation` (or the single test name) — a full `--release` test build doubles the job's
time for no extra coverage. Keep the bindings-drift check (`:55–61`) after both runs; confirm the `--release` run does
not regenerate `src/bindings/index.ts` differently (the `export_bindings` test is filtered out by the name filter — if
not, say so). `Swatinem/rust-cache` caches both profiles under one key; leave its config alone unless the cache blows
its size limit — check the run log.

Verify locally: `cd src-tauri && cargo test --release --all propagation` passes and the step's timing is acceptable.
You cannot run the workflow; the parent's push exercises it. Do not bump action majors here — that is ticket 03,
which you do next in this lane.

## Done when

`ci.yml` rust job runs the propagation suite (at minimum the named test) under `--release`; local `--release` run green;
step placed before the bindings-drift check; no other workflow touched.

## Comments

### 2026-09-20 — lane B (implementation)

Added one step to `.github/workflows/ci.yml` rust job, between `cargo test --all` and the bindings-drift check:

```yaml
- run: cargo test --release --all propagation
```

(with a 4-line comment explaining why). No other workflow touched; action pins untouched (ticket 03 does those).

Evidence (local, `cd src-tauri && cargo test --release --all propagation`, cold release build sharing the target
dir with lane A):

- `test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 629 filtered out` — includes
  `core::propagation::tests::update_supplies_a_real_hash_to_copy_assignments_and_reconcile_keeps_synced ... ok`,
  plus the whole `core::propagation::tests::*` suite, `mutation_guard::tests::refresh_finalize_and_propagation_is_serialized`,
  `skill_edits::tests::edit_returns_a_failed_copy_target_in_its_propagation_report`,
  `skill_update::tests::every_byte_adapter_settles_and_reports_propagation`,
  `commands::mutation_results::edit_catalog_is_post_propagation_and_includes_unrelated_rows`.
- Wall time 1m09s (user 10m11s) for the cold release build + run on this machine; the test phase itself is 0.40 s.
  In CI the release profile is a second full compile of the crate (Swatinem/rust-cache keys both profiles under one
  key, so a warm cache covers it after the first run) — expect the rust job to grow by roughly the debug build time.
- `export_bindings` lives at `bindings_export::export_bindings` (`src-tauri/src/lib.rs:201`); the `propagation`
  name filter excludes it, and `git status --porcelain -- src/bindings` was empty after the run, so the `--release`
  run cannot regenerate `src/bindings/index.ts` differently. The drift check still sits after both runs.

Parent should watch in the CI run: the new step's duration and the rust-cache post-step's "cache size" line (the
ticket allows re-keying only if the cache blows its size limit; I did not change the cache config).

- 2026-09-20 (parent) — closed `done — 8cf55c3`.
