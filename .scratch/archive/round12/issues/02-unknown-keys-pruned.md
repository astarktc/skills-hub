# D2 — A selected tool key the registry no longer knows is pruned at the read seam

Status: done — 396374e

Source: BACKLOG #07 (round-11 review C3; `archive/round10/wave-b-ticketing-evidence.md:57`).

## Change

- `core/settings.rs`: `read_tool_selection` filters `Configured` keys through `tool_adapters::adapter_by_key`,
  `log::warn!("[settings] dropping unknown tool key {key:?} from {setting}")` per dropped key. Both
  `load_settings` and `effective_global_tool_targets` consume the pruned set.
- `apply_setting(SettingUpdate::GlobalToolConfig { selected_tools, .. })`: any unknown key →
  `bail!(SignalError::UnknownTool { tool })` before writing (existing variant; no new wire code).
- `src/lib/skillPresentation.ts` `visibleToolChoices` doc comment: note that unknown keys never reach it.

No UI change: a tool that no longer exists has nothing to render; the next save persists the pruned set.

## Tests

- `core/tests/settings.rs`: `["claude_code","ghost"]` → `load_settings().global_selected_tools == Some(["claude_code"])`
  and `effective_global_tool_targets == ["claude_code"]`; `apply_setting` with `["ghost"]` → `Err` downcasting to
  `UnknownTool { tool: "ghost" }` and the stored row unchanged.

## Comments

- 2026-09-16 — done in `396374e`. Evidence: `global_selected_tools_prunes_keys_the_registry_no_longer_knows`, `apply_global_tool_config_refuses_an_unknown_tool_key_and_leaves_the_row` (core/tests/settings.rs).
