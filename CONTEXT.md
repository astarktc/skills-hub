# Skills Hub

Desktop app that installs AI Agent Skills once and syncs them to many AI coding tools. This glossary is the canonical vocabulary; terms crystallise here as design decisions land (see `docs/adr/`).

## Language

**Tool**:
An AI coding tool that Skills Hub syncs skills into (Claude Code, Cursor, …), identified by a stable key. Every fact about a tool (its dirs, group membership, capabilities) lives in its single registry record (`ToolAdapter` in `core/tool_adapters/`).
_Avoid_: adapter (that's the code object serving a tool), client, IDE

**Tool capability**:
A per-tool fact that changes how sync behaves for that tool, recorded on its registry record rather than tested by name in sync code (today: `supports_symlink` — Cursor cannot read a symlinked skills dir, so it is always copied).
_Avoid_: special case, tool quirk, Cursor mode

**Virtual group**:
A tool entry that stands in for several tools sharing one project-scope skills convention (today: the AGENTS standard's `.agents/skills`). It appears in tool lists in place of its constituent tools.
_Avoid_: meta-tool, umbrella tool

**Constituent tools**:
The tools absorbed into a virtual group entry. The group counts as installed when any constituent is; group membership is owned by the backend and only presented by the frontend.
_Avoid_: member tools, sub-tools

**Shared skills dir group**:
Global tools whose skills directories resolve to the same location, so syncing to one member syncs to all. Owned by the backend; the frontend only presents it.
_Avoid_: dir alias, linked tools

**Tool catalog**:
The presentation-ready tool list for one scope (global or project) with installedness, shared-dir groups and virtual-group constituents already resolved against the operator's home. Assembled by the backend (`tool_adapters::global_tool_entries` / `project_tool_entries`); commands only map it to DTOs.
_Avoid_: tool status (that's the DTO carrying a catalog), tool list

**Managed skill**:
A skill installed through Skills Hub and tracked in its database, eligible for sync fan-out to tools.
_Avoid_: installed skill (ambiguous with a tool being installed)

**Sync target**:
One (skill, tool) pair a sync batch attempts; each target resolves to synced, skipped, or failed as report data, never as a command error.
_Avoid_: sync pair, destination
