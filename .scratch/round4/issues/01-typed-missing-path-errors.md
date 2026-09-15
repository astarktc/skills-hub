# 01: Missing-path failures are typed errors; the log-reveal rule is a tested core function

Status: done — 0fc6210

**What to build:** When an install, Update or Refresh fails because a source folder, the central copy or a requested repo subpath is not there, the operator reads a message that says which of those it was — with the path shown as a detail — instead of a raw `OTHER` string carrying an absolute cache-internal path. When "Open log folder" fails, the operator reads that failure as its own message. The rule that decides how the log folder is revealed (a directory whose name ends in `.app` must be revealed via its parent, never opened as a bundle) becomes a pure core function with a test.

Source: `../spec.md` Q9; `../source-12-followups.md` #2, #6, #17; ADR-0001.

**Blocked by:** None (can start immediately)

- [x] Test: Update of a `local` skill whose folder is gone surfaces at the command seam as its own code (not `OTHER`), with the path in a structured field and no path in the message
- [x] Test: Update of a skill whose central copy is gone surfaces as its own code
- [x] Test: a git acquisition whose requested subpath is not in the repo surfaces as its own code carrying the *requested subpath*, never the cache-internal absolute path
- [x] Test: the log-reveal target rule returns the parent for a `.app`-suffixed directory and the directory itself otherwise
- [x] `describeCommandError` has a branch and EN + ZH copy for every new code; the binding is regenerated and committed
- [x] Searching core for `"path not found"` finds no remaining `bail!` with an interpolated path
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 0fc6210. Evidence: Cited 0fc6210 types missing-path failures; f30a8d9 adds log policy. src/commandError.ts:159,165 renders the structured codes; src-tauri/src/commands/mod.rs:227 uses the reveal target.

### 2026-09-05 — implementation (branch r4/01-typed-missing-path-errors)

**Shipped** (on top of main `f057190`):
- `0fc6210` feat(errors): type missing-path failures as their own command codes — `SignalError::{SourcePathMissing, CentralPathMissing, SubpathMissing, RevealLogFailed}` → `CommandError` codes `SOURCE_PATH_MISSING` / `CENTRAL_PATH_MISSING` / `SUBPATH_MISSING` / `REVEAL_LOG_FAILED`; all 8 raw bails replaced (`installer.rs` ×6 via a private `source_path_missing(path)` helper + the central check, `central_repo.rs:24`, `git_acquisition.rs:381` now reports the *requested* subpath); regenerated `src/bindings/index.ts`; `describeCommandError` branches (sentence + `\n\n` + detail, same shape as `GIT_CLONE_FAILED`, through one `withDetail` helper); EN + ZH keys `errors.{sourcePathMissing,centralPathMissing,subpathMissing,revealLogFailed}`.
- `f30a8d9` feat(log): pure log-reveal rule in core; open_log_folder failures are typed — new `core/log_reveal.rs` (`log_reveal_target(log_dir, existing_log_file) -> LogRevealTarget { folder, selected }`, declared in `core/mod.rs`) with 4 tests; the command maps `selected: Some` → `reveal_item_in_dir`, `None` → `open_path`, and every failure (dir resolution, create_dir_all, opener) → `RevealLogFailed { detail: "{err:#}" }` via a private `reveal_log_failed` classifier.
- `f04e35c` style: rustfmt.

Tests added: `commands/tests/commands.rs` — `update_of_a_local_skill_whose_source_is_gone_is_typed_source_path_missing`, `update_of_a_skill_whose_central_copy_is_gone_is_typed_central_path_missing` (both drive the real Update = `refresh_managed_skills` batch of one, classify through `CommandError::from_anyhow`, assert wire JSON and `message` absent), `an_open_log_folder_failure_is_typed_with_its_chain_as_detail`, `subpath_missing_serializes_the_requested_subpath_only`; `core/tests/git_acquisition.rs::a_missing_subpath_fails` now asserts the typed value and that the cache dir does not leak; `core/tests/central_repo.rs::move_central_repo_refuses_when_source_missing` rewritten to assert the typed value (was prose-sniffing `"not found"`); `core/tests/log_reveal.rs` ×4; `src/commandError.test.ts` ×4.

Gate: `npm run version:check` OK (1.2.3); `npm run check` green (vitest 176/176, build OK, fmt/clippy clean, cargo 453/453); `cargo test --all` 453/453.

**Deviations**
- Behaviour change in `open_log_folder` on non-macOS: a log dir *without* the `.app` suffix (Linux/Windows) is now opened itself (`open_path`) instead of being selected in its parent — that is what the ticket's "reveal-in-dir vs open" rule asks for; macOS behaviour (the `.app` case) is unchanged. The `.app` check is case-insensitive.
- `git_acquisition::clone_path`: when the resolved subpath is `None` (repo root) and the checkout is somehow absent, the bail is now a plain "repository checkout is missing after fetch" (no path) — it is an infrastructure failure, not an answer about the source; unreachable in practice since the root was just fetched.
- `describeCommandError`'s existing `GIT_CLONE_FAILED` branch was routed through the new `withDetail` helper (identical output, asserted by the existing test) so the "sentence + detail line" rule lives once.

**Notes for the orchestrator**
- Surface for ticket 08 (Add refuses Tool-dir paths): the `source_path_missing` helper in `installer.rs` and the `withDetail` helper in `commandError.ts` are the places to mirror.
- Surface for ticket 09 (Unlocatable): `acquire_managed_skill_update_with` still raises `CentralPathMissing` before staging — ticket 09's "finalize must tolerate a missing central dir" will need to relax that check for the Restore path.
- Rebase risk: `git_acquisition.rs` change is the single `copy_src.exists()` block (ticket 02 edits the same region); `commands/mod.rs` gained two `use` lines + the `open_log_folder` body + one private fn.
- Only Rust-side opener calls are used (`OpenerExt`), so no capability JSON change was needed for `open_path`.
