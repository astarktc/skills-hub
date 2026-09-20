# 08 Insert-shaped TargetTransition for new sync rows

Status: done — pending parent commit
Lane: D
Source: BACKLOG #13

`src-tauri/src/core/global_sync.rs:134` calls `store.upsert_skill_target(&record)` directly inside `sync_skill_into_root`. Find the existing `TargetTransition` type (rg it; likely in propagation.rs or skill_store.rs) and add an insert-shaped variant so global sync records a new target row through the same transition door as propagation. Do not change behaviour; refactor only. `cargo test global_sync propagation`.

## Done when

No direct upsert in global_sync.rs; existing tests green; the transition enum documents the insert case.

## Comments

- 2026-09-16 (lane D child): added the insert-shaped `TargetTransition::Recorded { skill_id, tool, mode,
  target_path, synced_at }` (`core/skill_store.rs`); `transition_skill_target` handles it as
  `INSERT … ON CONFLICT(skill_id, tool) DO UPDATE` — same SQL semantics as the former upsert (caller's fresh id used
  only on insert, existing id kept on conflict, status `Synced`, `last_error` cleared). `global_sync::sync_skill_into_root`
  now records each `record_tools` row through that door; no `SkillTargetRecord`/`upsert_skill_target` left in
  `global_sync.rs`. With production no longer calling it, `upsert_skill_target` became dead code under
  `clippy -D warnings`, so it is now a `#[cfg(test)]` fixture door (compile-time guarantee that production rows go
  through the transition); `SkillTargetRecord.last_error` gets `#[cfg_attr(not(test), allow(dead_code))]` (no
  production reader — it was only "read" by the upsert's SQL binding). Behaviour and wire unchanged.
  Evidence: new `recorded_transition_inserts_a_synced_row_then_settles_the_existing_one` (tests/skill_store.rs);
  `cargo test global_sync` 18 passed, `propagation` 16 passed, `project_sync` 33 passed, `skill_store` 51 passed;
  `cargo clippy --all-targets -- -D warnings` Finished.

