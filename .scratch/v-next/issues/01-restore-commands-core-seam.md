# Restore the commands/core seam

Status: resolved

Type: grilling

## Question

`commands/` is documented as wiring-only, but two command bodies carry real logic that the core test suite cannot reach, and one hides a verified bug. Decide the deepened shape, then implement (per map Notes):

- `sync_skill_to_tool` (`src-tauri/src/commands/mod.rs:562–650`, ~90 lines): writability probing, content-hash compare, shared-dir DB fan-out, error re-mapping. Where in `core/` does this live (`core/global_sync.rs`?), and what is its interface?
- `update_project_gitignore` (`src-tauri/src/commands/projects.rs:394–558`, 164 lines): gitignore block parsing/rewriting. **Verified mismatch**: patterns are built from `adapter.relative_skills_dir` (global, e.g. `.codeium/windsurf/skills`) while project sync writes `project_relative_skills_dir` (e.g. `.windsurf/skills`) — wrong entries for Windsurf/Pi/Goose/Augment-style tools. Decide: the core module shape (`core/gitignore.rs` taking `&[String]` patterns, mapping decided next to `project_relative_skills_dir`?), and the migration story for gitignore entries already written wrong into users' projects (rewrite on next sync? one-time repair? leave stale lines?).
- Minor: `SyncMode → "symlink"/"junction"/"copy"` string match is hand-duplicated 3× in commands/mod.rs (540–546, 625–632, 645–651) — fold into a `SyncMode::as_str()` in core.

Context: scan finding 4 in [arch-scan.md](../assets/arch-scan.md); report card #4. Restores the stated ADR — no conflict. Add core tests for the moved logic (existing suites: `core/tests/project_sync.rs`, 965 lines).

## Answer

Decided (grilled, all recommendations accepted) and implemented in commit `ee4f826`:

- **Sync fan-out → `core/global_sync.rs`** — the global-skills counterpart of `project_sync.rs`, keeping `sync_engine.rs` as pure mechanism. Deep entry point `sync_skill_to_tool_with_records(store, adapter, source, skill_id, name, &OverwritePolicy, now)`; environment probing (installed tools, default paths, shared-dir group) lives in the thin wrapper while the deterministic half (`sync_skill_into_root`, `remove_targets_for_tools`) is driven directly by tests — tests never touch the operator's real home. The unsync fan-out moved too (same family, cheap, symmetric).
- **Error shape** — core returns typed `GlobalSyncError::{ToolNotInstalled, TargetExists, ToolNotWritable, Other}`; the command maps variants to the wire prefixes (`format_global_sync_error`). The prefix strings stay in `commands/` (the documented seam), and ticket 02 inherits a ready-made enum to grow.
- **Gitignore → `core/gitignore.rs`** — pure block rewrite (`set_managed_block`/`remove_managed_block`) + `patterns_for_tools()` built on `project_relative_skills_dir()`, so the dir-mapping decision is made once, next to the mapping project sync writes with. Fixes the verified Windsurf/Pi/Goose/Augment wrong-pattern bug.
- **Migration story: idempotent rewrite, no one-time repair.** The old skip-if-marker write was itself a defect (block never updated when the tool list changed). Now every add-write strips existing Skills Hub block(s) — including the wrong global-dir lines, which all start with `/` — and appends the current block, so stale entries self-heal on any future toggle/edit. Sync deliberately does NOT touch `.gitignore` unprompted (checkbox consent).
- **`SyncMode::as_str()`** on the enum in `sync_engine.rs`; retired `project_sync::sync_mode_to_str` and the 3 hand-matches in `commands/mod.rs`.
- **Bonus fix found during work**: `installer.rs` project re-sync (update propagation) built targets from the *global* `relative_skills_dir` — a third place deciding the mapping, wrongly (e.g. Cursor updates went to `.cursor/skills` while assignments live in `.agents/skills`). Now uses `project_relative_skills_dir`; the installer test that encoded the old behavior was corrected.
- **Tests**: `src-tauri/tests/gitignore.rs` (514 lines that *reimplemented* the command's algorithm) deleted; replaced by `core/tests/gitignore.rs` (mapping, rewrite-migration, roundtrips, file-level orchestration incl. write-skip-when-unchanged) and `core/tests/global_sync.rs` (record fan-out, TargetExists/overwrite policies, cursor copy-mode, unsync). 196 tests green; full gate (`npm run version:check && npm run check`) green.

No AGENTS.md change needed: the "commands/ is wiring only" rule is now true again; the error-contract and DTO invariants are tickets 02/07's business.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
