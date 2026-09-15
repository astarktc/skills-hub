# 02: symlinks inside a skill folder are not content

Status: done — a041ff4

**Lane:** R2  **Files:** `src-tauri/src/core/content_hash.rs`, `src-tauri/src/core/sync_engine.rs`
(`copy_dir_recursive` only), `src-tauri/src/core/onboarding_import.rs` (doc + test), their test modules.

## Problem
`copy_dir_recursive` (`sync_engine.rs:229`) only handles `is_dir`/`is_file` and silently drops symlink entries;
`hash_dir` (`content_hash.rs`) hashes every entry's relative path (including a symlink's name) and only file bytes.
So a copy of a skill with an internal symlink hashes differently from its original: Onboarding import can report the
chosen original `KeptDivergent` against its own central copy, and Propagation's same-content check disagrees with copy.

## Decision (spec D3)
One rule: **a skill's internal symlinks are never part of its identity or its copies.**
- `hash_dir`: skip symlink entries entirely (neither name nor target enters the hash). Use `file_type().is_symlink()` on
  the walkdir entry (walk already has `follow_links(false)`).
- `copy_dir_recursive`: keep dropping them, but make it explicit (a branch with a comment, optionally a `log::debug!`).
- Document the rule in both functions' doc comments and in `onboarding_import.rs`'s header where it claims "the chosen
  variant's own Tool always among them".
Do NOT preserve or follow symlinks (rejected: absolute targets leak across machines; following can pull arbitrary files).

## Tests
- `content_hash`: a dir with a file + a relative symlink hashes equal to the same dir without the symlink; a dangling
  symlink is also ignored.
- `sync_engine`: copy of a dir with a symlink has no symlink at the target, and `hash_dir(src) == hash_dir(dst)`.
- `onboarding_import`: fixture where the chosen original contains an internal symlink — assert it is identified as
  same-content with its central copy (not `KeptDivergent`) and the claim in the header holds.
(`#[cfg(unix)]` for symlink creation is fine.)

## Gate
`npm run version:check && npm run check`. Commit on your branch with a conventional title. Paste `## Comments` in the final message.

## Comments

- 2026-09-15 — Status reconciliation: ready → done — a041ff4. Evidence: git log v1.2.5..v1.2.6 identifies a041ff4; src-tauri/src/core/content_identity.rs:95 excludes symlink entries, and src-tauri/src/core/tests/onboarding_import.rs:569 covers identical chosen originals with internal links.
