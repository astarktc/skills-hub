# 08 Insert-shaped TargetTransition for new sync rows

Status: open
Lane: D
Source: BACKLOG #13

`src-tauri/src/core/global_sync.rs:134` calls `store.upsert_skill_target(&record)` directly inside `sync_skill_into_root`. Find the existing `TargetTransition` type (rg it; likely in propagation.rs or skill_store.rs) and add an insert-shaped variant so global sync records a new target row through the same transition door as propagation. Do not change behaviour; refactor only. `cargo test global_sync propagation`.

## Done when

No direct upsert in global_sync.rs; existing tests green; the transition enum documents the insert case.

## Comments
