# 02 Release-mode regression run for content identity in CI

Status: ready-for-agent
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
