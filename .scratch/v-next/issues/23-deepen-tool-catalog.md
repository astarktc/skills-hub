# 23: Deepen the Tool catalog — one record per Tool, catalog assembly in the module, `supports_symlink`

Status: resolved

Type: task
Blocked by: 19, 20

## What to build

Unanimous review #2 finding (verified; also the old scan's Finding 6). `CONTEXT.md` says Tool, Virtual group, Constituent tools and Shared skills dir group are backend-owned — but at `943f85c` one Tool's facts live in five places, only two compiler-checked:

1. the `ToolAdapter` literal (`core/tool_adapters/mod.rs:138-453`, 44 entries);
2. the project-scope dir in a *separate* `match` on `ToolId` (`:482-528`);
3. virtual-group membership as nine **string keys** (`AGENTS_STANDARD_KEYS`, `:6-16`) duplicating `ToolId` variants;
4. catalog assembly duplicated in the command tier — `get_tool_status` (`commands/mod.rs:68-130`) and `get_project_tool_status` (`:133-207`) both build `Vec<ToolInfoDto>` + `installed`, diverging on virtual-group absorption and `shared_with`; `"agents_skills"` retyped 3× (`:165,:172,:176`) though `ToolId::AgentsStandard.as_key()` exists; `get_tool_status` silently persists `installed_tools_v1` and diffs `newly_installed` inside a getter (`:104-120`); zero tests for any of it (violates AGENTS.md "commands are wiring only");
5. the Cursor no-symlink rule as strings: `sync_engine.rs:159` (`eq_ignore_ascii_case("cursor")`) and re-derived twice in the installer's update propagation (`installer.rs:898,:921` — `t.tool == "cursor"`), bypassing `sync_dir_for_tool_with_overwrite`.

Also: `adapter_by_key` / `adapters_sharing_skills_dir` rebuild all 44 adapters by value per call (the latter inside a 44-iteration loop at `:82`); three `#[allow(dead_code)]` functions (`adapters_sharing_project_skills_dir`, `resolve_project_path`, `supports_project_scope`).

Deepen:

- `ToolAdapter` holds every per-Tool fact: add `project_relative_skills_dir` (replacing the separate match — ticket 19's resolver then reads the field), `group: Option<VirtualGroup>` (replacing the string-key list), and `supports_symlink: bool` (Cursor `false`). `sync_dir_for_tool_with_overwrite` consults the capability; delete all three string checks (installer update propagation goes through the tool-aware entry point).
- Expose the catalog from the module: e.g. `global_tool_entries(home)` and `project_tool_entries()` returning presentation-ready entries with `shared_with` / `constituents` / installedness resolved (uses ticket 20's home seam). The two commands become DTO maps; `get_tool_status`'s side effect becomes an explicit `record_installed_tools` step (keep behaviour). Delete the dead functions; make the registry a static/lazy value if that removes the per-call rebuilds cleanly.
- Tests: catalog against a temp home (virtual-group absorption, shared-dir grouping, installedness), Cursor forced-copy via the capability, project-dir mapping for every Tool (table).
- Update the AGENTS.md "New AI tool adapter" invariant to the smaller shape (one struct literal + README row); do not weaken the README-table check. Keep the "never make Cursor symlink" do-not — now enforced by the field.

## Acceptance criteria

- [ ] No separate `ToolId` match for project dirs; no string list for virtual-group membership; zero `"cursor"` string comparisons outside the adapter table.
- [ ] `get_tool_status` / `get_project_tool_status` contain no policy — only a core call + DTO mapping (+ the explicit persist step).
- [ ] Catalog/capability/project-dir tests exist and pass; the three dead functions are gone.
- [ ] AGENTS.md invariant updated; `npm run version:check && npm run check` green.

## Answer

Landed green in `ec45a24` (Fable 5.1 child, medium thinking; orchestrator-verified; 260 cargo + 62 vitest, full gate green on main).

- `ToolAdapter` is the single record per Tool: gained `project_relative_skills_dir: &'static str` (the separate `ToolId` match deleted; `resolve_project_sync_target` / `gitignore` read the field), `group: Option<VirtualGroup>` (new `enum VirtualGroup { AgentsStandard }`; the 9 constituents carry `Some(..)`, the group entry `None`; `AGENTS_STANDARD_KEYS` deleted; `constituents_of(group)`), and `supports_symlink: bool` (only Cursor `false`). Registry is `static TOOL_ADAPTERS: &[ToolAdapter]`; `default_tool_adapters()` returns the slice (name kept to limit churn); `adapter_by_key` / `adapters_sharing_skills_dir` read the static. Three dead functions deleted.
- New `core/tool_adapters/catalog.rs`: `ToolCatalogEntry { key, label, installed, skills_dir, shared_with, constituents }`, `global_tool_entries(home)`, `project_tool_entries(home)`, `installed_keys`. `get_tool_status` / `get_project_tool_status` are now `home_dir()` → catalog → `ToolInfoDto::from`; the persist is an explicit `record_installed_tools(store, &installed)` (still on raw `get_setting`/`set_setting` for ticket 24 to swap). `ToolInfoDto` wire shape unchanged (doc comment only; binding regenerated).
- `sync_dir_for_tool_with_overwrite(adapter: &ToolAdapter, …)` consults `supports_symlink`; the `eq_ignore_ascii_case("cursor")` check is gone. Installer update propagation: the two `t.tool == "cursor"` checks became `adapter_by_key(..).is_some_and(|a| !a.supports_symlink)`; the actual re-copy deliberately stays a copy (routing it through the tool-aware entry would turn a copy-mode target of a symlink-capable tool into a symlink while the record says `copy`). Zero `"cursor"` string comparisons outside the `as_key` arm.
- Tests: `tests/tool_catalog.rs` (6: global list, shared-dir grouping, installedness, project absorption, group installed iff any constituent, project-dir grouping); `tests/tool_adapters.rs` (+4: project-dir table for all 45 tools with a registry-length assert, AgentsStandard membership, only Cursor lacks symlink, unique keys); `tests/sync_engine.rs` (Cursor forced-copy via capability, symlink-capable gets a link, mutated adapter proves capability-not-identity).
- Docs: AGENTS.md invariant rewritten (one literal + README row + table test) and the Cursor do-not now cites the field; CONTEXT.md gained **Tool capability** and **Tool catalog**.
- Behaviour delta (intentional, unobservable in UI): project-scope `shared_with` now groups by project dir (Trae/Trae CN → `["trae","trae_cn"]`) instead of always `[key]` — the old comment claiming every project entry is its own group was false for Trae. Preserved: the group entry's own `~/.agents` dir does not count as installed (test pins it).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
