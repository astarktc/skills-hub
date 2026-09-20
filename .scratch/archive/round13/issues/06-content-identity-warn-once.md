# 06 Remove the process-global warn-once in content_identity

Status: done — dd29ecc
Lane: C
Source: BACKLOG #03

`src-tauri/src/core/content_identity.rs:56`: `static WARNED: OnceLock<Mutex<HashSet<String>>>` is ambient state in core. Drop the dedupe: log the warning every time (reconcile flood risk is acceptable; note it in the doc comment) — simplest option; do NOT thread a sink through callers. Remove now-unused imports. `cargo test content_identity` + `cargo clippy`.

## Done when

No OnceLock/static in content_identity.rs; tests green.

## Comments

- 2026-09-16 (lane C child): Removed `static WARNED: OnceLock<Mutex<HashSet<String>>>` from
  `core/content_identity.rs::read`; the read-failure branch now logs `log::warn!` on every failure. Dropped the
  `HashSet`/`Mutex`/`OnceLock` imports (only `std::path::Path` remains). Doc comment on `read` notes the accepted
  reconcile flood risk and why no dedupe state lives in core. No sink threaded through callers.
  Evidence: `cd src-tauri && cargo test content_identity` → `test result: ok. 3 passed` (`internal_symlinks_are_not_content`,
  `hash_changes_with_content_and_ignores_git_dir`, `managed_read_backfills_once_and_trusts_the_row`);
  `rg "OnceLock|static " src-tauri/src/core/content_identity.rs` → no matches.


- 2026-09-16 (parent) — closed `done — dd29ecc`; review fixes in fa4ae9f; released as 1.2.13 (c927683).
