# D2 — `.agents/skills` is unreachable in My Skills; the roster label is scope-dependent

Status: done — 14932af

## Problem

The registry already carries a global `.agents/skills` target
(`core/tool_adapters/mod.rs:186`, `ToolId::AgentsStandard`, `relative_skills_dir: ".agents/skills"`,
`relative_detect_dir: ".agents"`), but the two catalog builders disagree
(`core/tool_adapters/catalog.rs`):

- `global_tool_entries` → `.filter(|adapter| !adapter.is_virtual_group())` — the group is **dropped**;
  its 9 constituents are listed individually, each with its own distinct global dir.
- `project_tool_entries` → `.filter(|adapter| adapter.group.is_none())` — the group is **listed**,
  constituents absorbed.

So the consolidation landed project-side only, and `~/.agents/skills` — a real global location — can
never be managed. (It existed and was empty on the operator's machine; the parent removed it during
cleanup, so a fresh run must be able to create it.)

## Decision (do not re-decide)

1. **List the group globally, alongside its constituents.** Globally it is *not* an aggregate: Cursor
   reads `~/.cursor/skills`, Codex reads `~/.codex/skills`. It is an independent additional target that
   several tools also happen to read. Constituents stay individually listed and individually
   selectable; nothing is hidden.
2. **`shared_with` stays honest** — it is derived from `adapters_sharing_skills_dir`, so the group's
   global entry shares with whatever genuinely resolves to `~/.agents/skills` (likely nothing today).
   Do not hand-wire it.
3. **`constituents` is empty for the global entry** and populated for the project entry. The rule:
   *the roster renders exactly when checking this box covers those tools.* Project-side one folder has
   nine readers, so the roster is load-bearing; global-side it would be a lie that makes a user skip
   the real Cursor checkbox.
4. **Per-scope label.** `display_name` is currently the single static string
   `".agents/skills (9 tools)"`, which is only true project-side. Add a per-scope label to the registry
   literal — e.g. `group_label: Option<&'static str>` used by `project_tool_entries` while
   `global_tool_entries` uses `display_name` — so the global row reads `.agents/skills` and the project
   row reads `.agents/skills (9 tools)`. Keep the fact in the registry literal (AGENTS.md invariant:
   every per-tool fact lives in that literal), not in the UI.

`installed` for the global entry is plain `is_installed_in(home, adapter)` (`~/.agents` exists) — the
project-side "installed when any constituent is" rule is project-side only.

## Files

- `src-tauri/src/core/tool_adapters/mod.rs` — the `ToolAdapter` struct + the `AgentsStandard` literal.
- `src-tauri/src/core/tool_adapters/catalog.rs` — `global_tool_entries` filter + label selection.
- `README.md` — supported-tools table: note that the group is an aggregate project-side and a plain
  target global-side. (AGENTS.md: the README table is **not** compiler-enforced — check the row count
  against the `ToolId` variant count.)
- `src/i18n/resources.ts` — **both** `en` and `zh` if any new string is needed.
- `src-tauri/src/core/tests/tool_catalog.rs` and `core/tests/tool_adapters.rs`.

**Do not touch**: `core/refresh.rs`, `core/settings.rs`, `src/hooks/` (D1's lane);
`src/components/shared/ToolConfigModal.tsx`, `src/components/skills/SkillCard.tsx` (D3/D4's lane).

## Tests

- `global_tool_entries` includes an `agents_standard` entry with an **empty** `constituents`, and still
  includes every constituent individually.
- `project_tool_entries` is unchanged: group present with the 9-name roster, constituents absorbed.
- Label test: the global entry's label carries no "(9 tools)" suffix; the project entry's does.
- Extend the existing `project_relative_skills_dir_for_every_tool` table test if the struct changes.

## Gate

`npm run version:check && npm run check`. Note `cargo test` regenerates `src/bindings/index.ts` if any
DTO shape moved — commit it; CI diff-guards it.

## Comments

- 2026-09-15 — Status reconciliation: NO STATUS → done — 14932af. Evidence: git log v1.2.10..v1.2.11 finds 14932af then a0f2c1d, release 9de83e9. src-tauri/src/core/tool_adapters/catalog.rs:43–54 lists the global group with an empty constituent roster.
