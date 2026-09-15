# 10: Smoke-test fixes — "Open log folder" reveals; the history keeps every action error

Status: done — f7d2abc

**What to build:** Two defects the operator found smoke-testing tickets 01/02.

(a) Settings → Diagnostics → "Open log folder" does nothing on macOS. The app log dir is named after the bundle identifier, `~/Library/Logs/com.skillshub.app`, and `/usr/bin/open` treats a directory ending in `.app` as an application bundle ("The application cannot be opened because its executable is missing"); the opener plugin spawns `open` detached, so the failure is swallowed. The command reveals the log **file** in its folder instead (`reveal_item_in_dir` — NSWorkspace select-in-Finder, synchronous, cannot be read as a launch); when no log file exists yet it reveals the directory itself (selected in `~/Library/Logs`), never `open_path` on the dir. The log file name is whatever `tauri-plugin-log`'s `LogDir { file_name: None }` produces (the product name, e.g. `Skills Hub.log`) — resolve it from the app config, don't hardcode.

(b) A batch with many per-target failures (Refresh all: "Update failed: writing-skills — central path not found (+18 more)") records **one** history entry, so the 18 others are unreadable anywhere. `showActionErrors` keeps its one-toast behaviour (head + "+N more") but records **every** visible entry in the notification history, each as its own error notification (title + message), so the panel lists them all and Copy-all captures them. Unread count follows (N entries → N unread).

Source: operator smoke test of v-next `4923f33`, 2026-09-04. Spec Q4/Q5.

**Blocked by:** None (can start immediately)

- [x] `open_log_folder` uses `reveal_item_in_dir` on the log file (or the dir when the file is absent); no `open_path` on a directory anywhere
- [x] Reporter hook test: `showActionErrors` with 3 entries → one error toast, three history entries, `unreadCount === 3`
- [x] `npm run version:check && npm run check` green

## Comments

## Comments

- 2026-09-15 — Status reconciliation: done → done — f7d2abc. Evidence: Cited f7d2abc integrated as d4c1f42; log reveal followed in 973671a. src/hooks/useStatusReporter.ts:320 records every batch entry, and src-tauri/src/commands/mod.rs:227 reveals the selected log item.

Branch `r3/10-smoke-fixes` (worktree `skills-hub-wt-10`), from main `4923f33`.

**Commits**
- `f7d2abc` fix(reporter): record every action error in the history, toast once
- `4d17493` fix(settings): reveal the log file instead of opening the log dir

**(a) `open_log_folder`** — `src-tauri/src/commands/mod.rs`. Confirmed in
`tauri-plugin-log-2.8.0/src/lib.rs` (the locked version): `LogDir { file_name: None }`
resolves to `app_handle.package_info().name` and `RotatingFile::new` builds the path as
`dir.join(&file_name).with_extension("log")`. The command uses the identical expression
(`log_dir.join(&app.package_info().name).with_extension("log")`) — on this machine that is
`~/Library/Logs/com.skillshub.app/Skills Hub.log`, which matches the file present there.
`create_dir_all` kept; target = the file if `is_file()`, else the dir; `opener().reveal_item_in_dir(target)`
(synchronous NSWorkspace select-in-Finder in `reveal_item_in_dir.rs`). `grep open_path src-tauri/src` → 0 hits.
Everything stays at the command seam inside `spawn_blocking`; no `core/` change.
`src/bindings/index.ts` changed too: tauri-specta exports the Rust doc comment into the binding,
so the reworded doc regenerated it on `cargo test` — committed with the fix (CI diff-guards it).

**(b) `showActionErrors`** — `src/hooks/useStatusReporter.ts`. Test-first: added
"toasts once but records every visible entry as its own error" (3 entries → 1 `toast.error`
with `+2 more`, 3 history rows newest-first with plain messages, `unreadCount === 3`);
it failed red (history had 1 row carrying the `+N more` suffix), then went green.
`notify` was split into an internal `record` (the history's only writer) + the existing `showToast`;
`notify`'s public signature/semantics are unchanged. `showActionErrors` calls `showToast` once for the
head (with the `+N more` suffix) and `record` once per visible entry (head included, plain message).
`NotificationsModal` untouched. Existing tests all still pass (26 in the file, 170 total).

**Gate** — `npm run version:check` → `Version OK (1.2.2)`; `npm run check` → exit 0
(vitest 170 passed; cargo test 441 passed; fmt/clippy `-D warnings` clean). The refresh
wall-clock flake did not fire. No version bump; nothing merged/rebased/pushed.

**Deviations** — none. One note: `cargo fmt` reflowed two lines of the new code on the first gate run;
fixed and amended before the final green run.
