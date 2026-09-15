# 14: Gitignore lifecycle — removal path + status detection into core

Status: resolved

Type: task
Blocked by: 13

## What to build

Verified review findings (locations as of `be9a74c`):

1. **Self-heal dead-ends at zero tools.** `core/gitignore.rs::update_project_ignore_files` early-returns when `patterns.is_empty()`, and the command derives patterns from the project's *current* tools — so after removing a project's last tool, a stale managed block in `.gitignore`/`.git/info/exclude` can never self-heal or be removed, even toggling explicitly. Ticket 01 promised "stale entries self-heal on any future toggle/edit". Fix: empty patterns (or a toggle set to false) still strips any existing managed block — the write becomes "strip, then append the current block only if there's one to append". Keep the idempotent no-write-when-unchanged behavior.
2. **Marker duplicated in the wiring layer.** `commands/projects.rs::get_project_gitignore_status` re-literalizes `"# Skills Hub"` and hand-rolls detection, the mirror image of core's `apply_to_file`; changing `core/gitignore.rs::MARKER` would silently break the checkbox the UI shows. Move status detection into `core/gitignore.rs` (e.g. a `project_ignore_status(project_path)` fn using `MARKER`), and have the command call it. Also make `managed_block` interpolate `MARKER` instead of re-literalizing the header string.

Add core tests: remove-last-tool-then-toggle strips the block; status detection agrees with the writer; the existing 23-test suite keeps passing.

Note: ticket 13 edits the same file (`commands/projects.rs`) — this ticket is blocked on it to avoid worktree conflicts on those functions.

## Acceptance criteria

- [x] A zero-tool project's stale managed block is stripped on the next toggle/edit (test proves it).
- [x] Status detection lives in core beside the writer; `MARKER` has exactly one literal occurrence.
- [x] Existing gitignore tests green + new coverage.
- [x] `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `3e415c7` (Fable 5 low subagent; orchestrator-verified, rebased, combined-tree gate green). Empty patterns now take the strip path in `apply_to_file` (stale blocks self-heal at zero tools; still creates no files when nothing exists); new `IgnoreStatus` + `project_ignore_status()` in core beside the writer — `get_project_gitignore_status` is DTO conversion only; `managed_block` interpolates `MARKER` (exactly one literal). 25 gitignore tests (23 + 2 new: zero-tool strip; status-agrees-with-writer). Ticket 01's self-heal promise now holds with no dead ends.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
