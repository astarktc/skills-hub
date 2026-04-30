# Quick Task 260429-s9x: Consolidate .agents/skills Harnesses - Research

**Researched:** 2026-04-29
**Domain:** Rust ToolId enum, SQLite migration, Tauri IPC, React project tool UI
**Confidence:** HIGH

## Summary

The task adds a virtual `ToolId::AgentsStandard` variant that consolidates 9 tools sharing `project_relative_skills_dir() == ".agents/skills"` into a single selectable group for project-level tool assignments. The existing architecture cleanly supports this: `adapter_by_key()` is the universal validation gate, `project_relative_skills_dir()` controls sync target resolution, and the frontend `ToolConfigModal` renders from `ToolStatusDto.tools` array (making injection straightforward).

The main complexity is the DB migration: existing `project_tools` and `project_skill_assignments` rows referencing the 9 old keys must collapse into `agents_skills` while handling UNIQUE constraint violations (same project+skill assigned to multiple old tools that all resolve to the same directory).

**Primary recommendation:** Add `ToolId::AgentsStandard` as a first-class enum variant with its own `ToolAdapter` entry (detect_dir = `.agents`, skills_dir = `.agents/skills`), a V8 migration to consolidate DB rows, and inject the virtual group at the front of `ToolStatusDto.tools` in the project-specific tool list.

## User Constraints (from CONTEXT.md)

### Locked Decisions

- DB key: `agents_skills`, Rust variant: `ToolId::AgentsStandard`, `as_key()` returns `"agents_skills"`
- Startup migration in `ensure_schema` path
- `UPDATE project_tools SET tool = 'agents_skills' WHERE tool IN (...)` with dedup handling
- Same UPDATE pattern for `project_skill_assignments`
- `ToolId::AgentsStandard` resolves directly to `.agents/skills` via `project_relative_skills_dir()`
- One symlink per assigned skill -- no expansion to 9 adapters
- `shares_project_skills_dir_with()` continues to work but becomes less critical

### Claude's Discretion

- Display name in Config Tools modal
- Ordering: virtual group appears first
- Whether "(installed)" badge shows if ANY of the 9 tools is detected

### Specific Ideas (non-binding)

- Subtitle listing all 9 harness names
- Place at top of tool picker
- Show "installed" if any of the 9 underlying tools is detected

## Research Findings

### 1. ToolId Enum Pattern

**Location:** `src-tauri/src/core/tool_adapters/mod.rs`

The `ToolId` enum has 44 variants. Adding a new variant requires:

1. **Enum definition** (line ~1-51): Add `AgentsStandard` variant
2. **`as_key()` match** (line ~54-101): Add `ToolId::AgentsStandard => "agents_skills"`
3. **`default_tool_adapters()` vec** (line ~123-430): Add a `ToolAdapter` struct entry
4. **`project_relative_skills_dir()` match** (line ~461-506): Add `ToolId::AgentsStandard => ".agents/skills"`

**Critical: Exhaustive match locations outside mod.rs:**

- `scan_tool_dir()` (line 546): Only checks `ToolId::Codex` specifically -- no exhaustive match, safe.
- `supports_project_scope()` (line 457): Only excludes `HermesAgent` -- implicitly includes new variants. Safe.

**`adapter_by_key()` (line 508):** Iterates `default_tool_adapters()` and matches on `as_key()`. Once the adapter is in the vec, this works automatically.

**The 9 tools being consolidated (all return `.agents/skills` from `project_relative_skills_dir`):**

- `cursor`, `codex`, `amp`, `kimi_cli`, `antigravity`, `cline`, `gemini_cli`, `github_copilot`, `opencode`

[VERIFIED: src-tauri/src/core/tool_adapters/mod.rs direct read]

### 2. DB Migration Pattern

**Location:** `src-tauri/src/core/skill_store.rs`

**Current version:** `SCHEMA_VERSION = 7`

**Mechanism:** `ensure_schema()` reads `PRAGMA user_version`, then:

- If 0: runs full schema + all migrations + sets to current version
- If < current: runs incremental `if user_version < N` blocks
- If > current: errors

**Adding V8 migration:**

```rust
const SCHEMA_VERSION: i32 = 8;

// In ensure_schema(), after the `if user_version < 7` block:
if user_version < 8 {
    // Consolidate .agents/skills tools into agents_skills
    // Step 1: Delete duplicate project_tools rows that would conflict
    conn.execute_batch(
        "DELETE FROM project_tools WHERE id NOT IN (
            SELECT MIN(id) FROM project_tools
            WHERE tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode')
            GROUP BY project_id
        ) AND tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode');
        UPDATE project_tools SET tool = 'agents_skills'
            WHERE tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode');"
    )?;
    // Step 2: For project_skill_assignments, keep one per (project_id, skill_id) and delete duplicates
    conn.execute_batch(
        "DELETE FROM project_skill_assignments WHERE id NOT IN (
            SELECT MIN(id) FROM project_skill_assignments
            WHERE tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode')
            GROUP BY project_id, skill_id
        ) AND tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode');
        UPDATE project_skill_assignments SET tool = 'agents_skills'
            WHERE tool IN ('cursor','codex','amp','kimi_cli','antigravity','cline','gemini_cli','github_copilot','opencode');"
    )?;
}
```

**UNIQUE constraints to handle:**

- `project_tools`: `UNIQUE(project_id, tool)` -- a project with both `cursor` and `codex` configured would produce two `agents_skills` rows. Must dedup first.
- `project_skill_assignments`: `UNIQUE(project_id, skill_id, tool)` -- same skill assigned to same project under both `cursor` and `codex` would collide. Must dedup first.

**Fresh DB path (user_version == 0):** Also needs the V8 constant set but no migration logic needed since fresh installs won't have old keys.

[VERIFIED: src-tauri/src/core/skill_store.rs direct read]

### 3. Frontend ToolStatusDto and ToolConfigModal

**Backend shape (`commands/mod.rs` line 113-125):**

```rust
pub struct ToolInfoDto {
    pub key: String,
    pub label: String,
    pub installed: bool,
    pub skills_dir: String,
}

pub struct ToolStatusDto {
    pub tools: Vec<ToolInfoDto>,
    pub installed: Vec<String>,
    pub newly_installed: Vec<String>,
}
```

**`get_tool_status` (line 129-180):** Iterates `default_tool_adapters()`, checks `is_tool_installed()` for each, and builds the `ToolInfoDto` list. This means adding `AgentsStandard` to `default_tool_adapters()` will automatically include it in the tool status response.

**Frontend consumption (`ToolConfigModal.tsx`):**

- Renders `toolStatus.tools` as a flat list of checkboxes
- Uses `toolStatus.installed` to filter when "detected only" is checked
- No grouping, no special handling -- just iterates and renders

**Strategy for the virtual group:**

- Option A: Add to `default_tool_adapters()` and let it appear naturally in the list
- Option B: Inject it separately in `get_tool_status` at position 0 with special installed-detection logic (OR of 9 adapters)

Option A is simpler but the "installed" detection needs a custom `relative_detect_dir`. Since the virtual group should show as installed if ANY of the 9 underlying tools is detected, the adapter's `is_tool_installed()` check (which checks a single `relative_detect_dir`) won't suffice. Need either:

- A custom override in `get_tool_status` for this specific tool
- Or set `relative_detect_dir` to `.agents` which is likely to exist if any agents-standard tool has been used

**Recommendation:** Add the adapter with `relative_detect_dir: ".agents"` -- this directory is typically created when any `.agents/skills` tool is used. Then in `get_tool_status`, add special-case logic: if `.agents` doesn't exist, check if any of the 9 constituent detect dirs exist and report installed accordingly.

**Frontend ordering:** The `tools` array order in the response determines render order. Place `AgentsStandard` first in `default_tool_adapters()` OR special-case it in `get_tool_status` to be prepended.

[VERIFIED: src-tauri/src/commands/mod.rs and src/components/projects/ToolConfigModal.tsx direct read]

### 4. Project Sync Path Resolution

**Code path:** `project_sync::assign_and_sync()` (project_sync.rs line 30-105)

```
assign_and_sync(store, project, skill, tool_key, now)
  -> adapter = adapter_by_key(tool_key)        // Must return Some for "agents_skills"
  -> project_relative_skills_dir(&adapter)     // Must return ".agents/skills"
  -> resolve_project_sync_target(project_path, relative_dir, skill_name)
  -> sync_engine::sync_dir_for_tool_with_overwrite(tool_key, source, target, false)
```

**All code paths that call `adapter_by_key` with a project tool key:**

1. `commands/projects.rs:84` -- `add_project_tool` validation (MUST pass for virtual group)
2. `project_sync.rs:37` -- `assign_and_sync` (MUST return adapter)
3. `project_sync.rs:124` -- `sync_single_assignment` (MUST return adapter)
4. `project_sync.rs:254` -- `list_assignments_with_staleness` (MUST return adapter for status checks)
5. `project_sync.rs:371` -- `unassign_and_cleanup` (MUST return adapter)
6. `project_ops.rs:130` -- `remove_tool_with_cleanup` orphan handling
7. `commands/projects.rs:418` -- `update_project_gitignore` (uses `adapter.relative_skills_dir` for gitignore pattern)
8. `commands/mod.rs:932` -- `delete_managed_skill` project cleanup

**All these work automatically** once `adapter_by_key("agents_skills")` returns a valid `ToolAdapter` with correct `relative_skills_dir`.

**Gitignore concern:** `update_project_gitignore` (line 418-419) uses `adapter.relative_skills_dir` (the GLOBAL dir) for the gitignore pattern. For the virtual group, we want the gitignore to contain `/.agents/skills/`. Set the adapter's `relative_skills_dir` to `.agents/skills` so it works correctly. Note: this is the same value as `project_relative_skills_dir` for this virtual tool, which makes sense since the virtual tool only exists for project scope.

[VERIFIED: src-tauri/src/core/project_sync.rs, src-tauri/src/commands/projects.rs direct read]

### 5. `adapters_sharing_project_skills_dir()` and dedup

**Location:** `tool_adapters/mod.rs` line 443-449

```rust
pub fn adapters_sharing_project_skills_dir(adapter: &ToolAdapter) -> Vec<ToolAdapter> {
    let relative = project_relative_skills_dir(adapter);
    default_tool_adapters()
        .into_iter()
        .filter(|a| project_relative_skills_dir(a) == relative)
        .collect()
}
```

**Current usage:** Only declared `#[allow(dead_code)]` -- not actively called anywhere in production code. It's prepared for future dedup but currently unused.

**Impact of adding AgentsStandard:** This function would return the virtual group + all 9 original tools. Since it's unused, no immediate concern. However, if it's later used for dedup, the logic might need adjustment to treat `AgentsStandard` as the canonical representative.

**`adapters_sharing_skills_dir()` (global, line 435-440):** Used in `unsync_skill_from_tool` (commands/mod.rs:597) for GLOBAL sync. This checks `adapter.relative_skills_dir`. If the virtual group has `relative_skills_dir = ".agents/skills"`, it won't collide with any existing global adapter (Amp/KimiCli use `.config/agents/skills`, Cursor uses `.cursor/skills`, etc.). Safe.

[VERIFIED: src-tauri/src/core/tool_adapters/mod.rs direct read]

## Architecture Patterns

### ToolAdapter for the Virtual Group

```rust
ToolAdapter {
    id: ToolId::AgentsStandard,
    display_name: ".agents/skills",  // or "Agents Standard"
    relative_skills_dir: ".agents/skills",  // Used for gitignore + global (though global sync is N/A)
    relative_detect_dir: ".agents",         // Best-effort detection
}
```

### Migration Safety Pattern

The migration must handle the dedup-before-update pattern since SQLite doesn't support `UPDATE ... ON CONFLICT IGNORE`:

```sql
-- Step 1: Identify which rows to keep (one per project_id for project_tools)
-- Step 2: Delete the duplicates
-- Step 3: Update remaining rows
```

Alternatively, use a transaction-safe approach:

```sql
-- Delete all but one per group, then update the survivors
DELETE FROM project_tools
  WHERE rowid NOT IN (
    SELECT MIN(rowid) FROM project_tools
    WHERE tool IN (...) GROUP BY project_id
  ) AND tool IN (...);
UPDATE project_tools SET tool = 'agents_skills' WHERE tool IN (...);
```

### Frontend Virtual Group Injection

The virtual group should appear in `get_tool_status` with special installed-detection logic. Two approaches:

1. **Add to `default_tool_adapters()` as first entry** -- simple, but `is_tool_installed` only checks `.agents` directory
2. **Special-case in `get_tool_status`** -- inject at position 0 with `installed = true` if any of 9 detect dirs exist

Recommendation: Use approach 1 (add to adapter list) + override installed detection in `get_tool_status` for this specific key.

## Common Pitfalls

### Pitfall 1: UNIQUE Constraint Violation During Migration

**What goes wrong:** `UPDATE project_tools SET tool = 'agents_skills' WHERE tool IN (...)` fails if a project has multiple of the 9 tools configured.
**How to avoid:** DELETE duplicates BEFORE the UPDATE. Use `MIN(id)` or `MIN(rowid)` to keep one representative row per project.

### Pitfall 2: Global Sync Interference

**What goes wrong:** Adding `AgentsStandard` to `default_tool_adapters()` means global sync (`sync_skill_to_tool`) could target it, creating spurious symlinks in `~/.agents/skills/`.
**How to avoid:** Either exclude it from global sync logic in `sync_skill_to_tool`, or ensure the global `get_tool_status` does not present it as selectable for global sync. The task scope says "project-level tool assignments only" -- need guardrails.

### Pitfall 3: Orphaned Old-Key Assignments After Migration

**What goes wrong:** If migration runs but the user had assignments to old keys that weren't migrated (edge case: assignment added between schema check and migration), new assignments would fail validation.
**How to avoid:** The migration runs in `ensure_schema` before any commands execute, so this is safe by design.

### Pitfall 4: Gitignore Pattern Uses Wrong Dir

**What goes wrong:** `update_project_gitignore` uses `adapter.relative_skills_dir` (not `project_relative_skills_dir()`). For existing tools this means it adds the global dir to gitignore. For the virtual group, we want `.agents/skills` in the gitignore.
**How to avoid:** Set `adapter.relative_skills_dir = ".agents/skills"` for the virtual group. This aligns both global and project paths, which is correct since this tool only operates at project scope.

## Don't Hand-Roll

| Problem                  | Don't Build                         | Use Instead                                                          | Why                                                |
| ------------------------ | ----------------------------------- | -------------------------------------------------------------------- | -------------------------------------------------- |
| Dedup before migration   | Manual row-by-row iteration in Rust | Single SQL DELETE + UPDATE in execute_batch                          | SQLite handles atomicity, faster, less error-prone |
| Tool detection for group | Custom function checking 9 paths    | Reuse existing `resolve_detect_path` + `is_tool_installed` in a loop | Pattern already established                        |

## Key Code Locations Summary

| File                                          | What to Change                                                                                               |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `src-tauri/src/core/tool_adapters/mod.rs`     | Add enum variant, `as_key()`, adapter entry, `project_relative_skills_dir()` match arm                       |
| `src-tauri/src/core/skill_store.rs`           | Bump SCHEMA_VERSION to 8, add V8 migration block                                                             |
| `src-tauri/src/commands/mod.rs`               | Modify `get_tool_status` to handle virtual group installed detection, possibly exclude from global tool list |
| `src-tauri/src/commands/projects.rs`          | No changes needed -- `adapter_by_key` validation passes automatically                                        |
| `src/components/projects/ToolConfigModal.tsx` | Optionally add subtitle rendering for the virtual group entry                                                |
| `src-tauri/src/core/project_sync.rs`          | No changes needed -- `adapter_by_key` + `project_relative_skills_dir` work automatically                     |

## Assumptions Log

| #   | Claim                                                                            | Section   | Risk if Wrong                                                                        |
| --- | -------------------------------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------ |
| A1  | `.agents` directory exists if any .agents/skills tool has been used in a project | Section 3 | installed detection shows false negative; low risk since it's a display concern only |
| A2  | No existing users have `agents_skills` as a tool key in their DB                 | Section 2 | Migration would skip those rows; extremely unlikely                                  |

## Open Questions

1. **Should `AgentsStandard` appear in the global tool sync list?**
   - The task says "project-level tool assignments only"
   - If added to `default_tool_adapters()`, it will appear in global `get_tool_status`
   - Recommendation: Filter it out of the global tool list in `get_tool_status` by checking a flag, or only inject it in the project-specific tool response

2. **Should old individual tool keys still work for project assignments after migration?**
   - After migration, `adapter_by_key("cursor")` still returns a valid adapter
   - But `project_tools` won't have those keys anymore (migrated to `agents_skills`)
   - If a user tries to add "cursor" as a project tool, it would create a new `project_tools` row with key "cursor" -- should this be blocked?
   - Recommendation: Block it in `add_project_tool` by rejecting keys that belong to the agents-standard group for project scope

## Sources

### Primary (HIGH confidence)

- `src-tauri/src/core/tool_adapters/mod.rs` - ToolId enum, all match arms, adapter list, project_relative_skills_dir
- `src-tauri/src/core/skill_store.rs` - Schema versioning, migration pattern, UNIQUE constraints
- `src-tauri/src/commands/mod.rs` - ToolStatusDto, get_tool_status command
- `src-tauri/src/commands/projects.rs` - add_project_tool validation, update_project_gitignore
- `src-tauri/src/core/project_sync.rs` - assign_and_sync, sync_single_assignment, unassign_and_cleanup
- `src/components/projects/ToolConfigModal.tsx` - Frontend rendering of tool list

## Metadata

**Confidence breakdown:**

- ToolId enum pattern: HIGH - exhaustively traced all match arms
- DB migration: HIGH - schema mechanism fully understood, UNIQUE conflicts identified
- Frontend impact: HIGH - simple injection into existing array
- Sync resolution: HIGH - traced full code path from assignment to symlink

**Research date:** 2026-04-29
**Valid until:** 2026-05-29 (stable codebase, no upstream changes expected)
