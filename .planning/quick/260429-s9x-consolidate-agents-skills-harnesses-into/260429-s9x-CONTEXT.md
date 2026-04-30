# Quick Task 260429-s9x: Consolidate .agents/skills harnesses into virtual AgentsStandard ToolId for project assignments - Context

**Gathered:** 2026-04-30
**Status:** Ready for planning

<domain>
## Task Boundary

Consolidate the 9 `.agents/skills` harnesses (Amp, Kimi Code CLI, Antigravity, Cline, Codex, Cursor, Gemini CLI, GitHub Copilot, OpenCode) into a single virtual `ToolId::AgentsStandard` variant for project-level tool assignments only. The Config Tools modal replaces the 9 individual checkboxes with one consolidated row at the top. Global sync remains per-tool and untouched.

</domain>

<decisions>
## Implementation Decisions

### DB value & naming

- The virtual group key is `agents_skills` (stored in `project_tools.tool` and `project_skill_assignments.tool` columns)
- Rust enum variant: `ToolId::AgentsStandard` with `as_key()` returning `"agents_skills"`

### Migration strategy

- Startup migration in `ensure_schema` path
- `UPDATE project_tools SET tool = 'agents_skills' WHERE tool IN ('cursor', 'codex', 'amp', 'kimi_cli', 'antigravity', 'cline', 'gemini_cli', 'github_copilot', 'opencode')`
- Same pattern for `project_skill_assignments`
- Handle UNIQUE constraint conflicts with `INSERT OR IGNORE` / `DELETE` for duplicates before UPDATE

### Sync engine behavior

- The virtual `ToolId::AgentsStandard` resolves directly to `.agents/skills` via `project_relative_skills_dir()`
- One symlink per skill assigned to this group — no expansion to 9 adapters
- Existing `shares_project_skills_dir_with()` dedup logic continues to work but becomes less critical since the source of truth is now a single key

### Claude's Discretion

- Display name in the Config Tools modal (e.g. ".agents/skills (Cursor, Claude Code CLI, Codex, ...)")
- Ordering: virtual group appears first in the tool list
- Whether the "(installed)" badge shows if ANY of the 9 constituent tools is detected

</decisions>

<specifics>
## Specific Ideas

- The consolidated row should show a subtitle listing all 9 harness names so users know what's included
- Place the consolidated row at the top of the tool picker list (before individual tools like Claude Code)
- The `detect_tool_status` / installed detection for the virtual group: show as "installed" if any of the 9 underlying tools is detected on the system

</specifics>

<canonical_refs>

## Canonical References

- `src-tauri/src/core/tool_adapters/mod.rs` — ToolId enum, project_relative_skills_dir(), shares_project_skills_dir_with()
- `src-tauri/src/core/skill_store.rs` — project_tools and project_skill_assignments tables, ensure_schema migrations
- `src/components/projects/ToolConfigModal.tsx` — Config Tools modal UI
- `src-tauri/src/commands/mod.rs` — list_project_tools, set_project_tools commands

</canonical_refs>
