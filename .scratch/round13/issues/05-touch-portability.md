# 05 Backup-sweep test portability (touch -h)

Status: open
Lane: B
Source: BACKLOG #19

`src-tauri/src/core/tests/install_finalize.rs:373–375` shells out to `touch -h`. Replace with the `filetime` crate's `set_symlink_file_times` as a dev-dependency (check Cargo.toml dev-deps first; if `filetime` is already a transitive dep, still declare it). Keep the test's intent (age symlinks without following). Run `cd src-tauri && cargo test install_finalize`.

## Done when

No `Command::new("touch")` remains; test passes.

## Comments
