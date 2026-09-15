# 19: Make the wrong project-scope path family unrepresentable (fixes live orphan-cleanup defect)

Status: resolved

Type: task
Blocked by: None (can start immediately)

## What to build

Architecture review #2 (3-model panel, orchestrator-verified) found a **live defect**: every Tool has two skills-dir facts — the global `ToolAdapter.relative_skills_dir` (under `$HOME`) and `project_relative_skills_dir(&adapter)` (project scope). `core/project_sync.rs::resolve_project_sync_target(project_path, relative_skills_dir: &str, skill_name)` takes a bare `&str`, so every caller must choose the mapping. Project sync and the installer write with the **project** mapping (`project_sync.rs:49/121/251/367`, `installer.rs:939` at `943f85c`), but cleanup uses the **global** one at three sites in `core/project_ops.rs` — `:138` (`remove_tool_with_cleanup` orphan branch, via the helper) and `:191`, `:203` (`remove_project_with_cleanup`, hand-joined, both the normal *and* orphan branches). A fourth site, `commands/mod.rs:1019-1023` (`delete_managed_skill`), bypasses the helper and hand-joins the project mapping.

Consequence: removing a project (or a tool from a project) leaves the synced skill dirs on disk for every Tool whose two mappings differ — Cursor (`.cursor/skills` vs `.agents/skills`), Pi (`.pi/agent/skills` vs `.pi/skills`), Codex, Goose, Augment, Windsurf, OpenClaw, and more. This is the third recurrence of the same trap (`core/gitignore.rs:1-13` documents the first; ticket 01's bonus fix was the second). The interface is the mistake surface; fix the interface, not the callers.

- Change the interface: `resolve_project_sync_target(project_path, &ToolAdapter, skill_name)` chooses `project_relative_skills_dir` **inside**. Delete the `&str` parameter so the wrong mapping cannot be expressed.
- Route all six sites through it (the five correct callers, the three wrong `project_ops` sites, and the `commands/mod.rs` hand-join).
- Regression test in `core/tests/`: for a Tool whose mappings diverge (e.g. Pi or Cursor), assign → then `remove_project_with_cleanup` / `remove_tool_with_cleanup` → the project-scope target is gone; and a test proving the orphan branch (skill record deleted) also cleans the project-scope path.
- Do not touch the global sync path family; do not change wire DTOs.

## Acceptance criteria

- [ ] `resolve_project_sync_target` takes `&ToolAdapter`; zero callers pass a raw dir string; no remaining hand-joins of a skills dir onto a project path outside that helper.
- [ ] New cleanup regression tests (divergent-mapping Tool; normal + orphan branches) pass and fail against the pre-fix code.
- [ ] `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `47744ed` (Fable 5.1 child, low thinking; orchestrator-verified diff + gate on main).

- `resolve_project_sync_target(project_path, &ToolAdapter, skill_name)` now chooses `project_relative_skills_dir` inside; the `&str` parameter is gone, so the global mapping cannot be expressed at any call site. Doc comment records the trap's history.
- All sites routed through it: the four `project_sync.rs` callers, `installer.rs` update propagation, the three wrong `project_ops.rs` cleanup sites (`remove_tool_with_cleanup` orphan branch; `remove_project_with_cleanup` normal + orphan), and the `commands/mod.rs::delete_managed_skill` hand-join. Remaining `join(adapter.relative_skills_dir)` sites are global-`$HOME` joins (correct) plus one dir-level helper in `tool_adapters`.
- Four regression tests in `core/tests/project_ops.rs` using Pi (`.pi/agent/skills` vs `.pi/skills`, precondition-asserted divergent). Three fail against the pre-fix code; the `remove_tool_with_cleanup` normal-branch test passes pre-fix (that branch already delegated to `unassign_and_cleanup`) and is kept as a guard.
- **Finding for later**: `SkillStore` runs with `PRAGMA foreign_keys = ON` and `project_skill_assignments.skill_id` is `ON DELETE CASCADE`, so `delete_skill()` cascades assignments and the orphan branches are only reachable on legacy DBs. The existing `remove_tool_with_cleanup_handles_missing_skill_gracefully` test therefore never exercises the orphan path; the new tests orphan the row via a raw `foreign_keys = OFF` connection. Candidate for ticket 25 (deletion & project fan-out into core) to decide whether the orphan branches stay.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
