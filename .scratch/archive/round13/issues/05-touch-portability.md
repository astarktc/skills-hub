# 05 Backup-sweep test portability (touch -h)

Status: done — 7f9ac17
Lane: B
Source: BACKLOG #19

`src-tauri/src/core/tests/install_finalize.rs:373–375` shells out to `touch -h`. Replace with the `filetime` crate's `set_symlink_file_times` as a dev-dependency (check Cargo.toml dev-deps first; if `filetime` is already a transitive dep, still declare it). Keep the test's intent (age symlinks without following). Run `cd src-tauri && cargo test install_finalize`.

## Done when

No `Command::new("touch")` remains; test passes.

## Comments

- 2026-09-16 (lane B child): Added `filetime = "0.2"` under `[dev-dependencies]` in `src-tauri/Cargo.toml` (it was
  already a transitive dep at 0.2.29, so `Cargo.lock` only gains the edge on `app`). In
  `core/tests/install_finalize.rs::finalize_update_backup_sweep_does_not_follow_symlinks` the `touch -h -t
  200001010000` shell-out is replaced by `filetime::set_symlink_file_times(&link, epoch_2000, epoch_2000)` on the
  aged and dangling links (lutimes — sets the link's own mtime, target untouched), keeping the intent.
  Evidence: `grep 'Command::new("touch")'` → no matches; `cd src-tauri && cargo test install_finalize` → 24 passed
  (incl. `finalize_update_backup_sweep_does_not_follow_symlinks ... ok`); `cargo clippy --all-targets -- -D warnings
  -A dead_code` clean (the only `-D warnings` failure is a `dead_code` field in lane E's in-progress
  `project_ops.rs::ConfigureProjectToolsResult`, not this ticket's files); `cargo fmt --check` clean for this file.


- 2026-09-16 (parent) — closed `done — 7f9ac17`; review fixes in fa4ae9f; released as 1.2.13 (c927683).
