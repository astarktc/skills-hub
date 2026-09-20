# 06 Remove the process-global warn-once in content_identity

Status: open
Lane: C
Source: BACKLOG #03

`src-tauri/src/core/content_identity.rs:56`: `static WARNED: OnceLock<Mutex<HashSet<String>>>` is ambient state in core. Drop the dedupe: log the warning every time (reconcile flood risk is acceptable; note it in the doc comment) — simplest option; do NOT thread a sink through callers. Remove now-unused imports. `cargo test content_identity` + `cargo clippy`.

## Done when

No OnceLock/static in content_identity.rs; tests green.

## Comments
