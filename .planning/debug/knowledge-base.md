# GSD Debug Knowledge Base

Resolved debug sessions. Used by `gsd-debugger` to surface known-pattern hypotheses at the start of new investigations.

---

## remove-block-gitignore-failures — test remove_block used stale index-scanning algorithm that only drained the blank line

- **Date:** 2026-04-09
- **Error patterns:** marker should be removed, roundtrip add+remove, remove_block, gitignore, Skills Hub block remains
- **Root cause:** The test file's `remove_block` was a stale copy of an earlier index-scanning algorithm. When a preceding blank line existed, `start` was set to `i-1`, causing the end-detection condition `i > start.unwrap()` to fire on the marker line itself (which does not start with `/`), so `end` was set immediately and only the blank line was drained.
- **Fix:** Replaced the test's `remove_block` with the correct state-machine algorithm (using `in_block` flag) matching the production code in `commands/projects.rs`.
- **Files changed:** src-tauri/tests/gitignore.rs

---
