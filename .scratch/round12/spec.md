# Round 12 — tool-selection integrity

Absorbs BACKLOG #06, #07, #08 (removed from `.scratch/BACKLOG.md` in the commit that opens this effort).

## Problem

The saved global tool selection (`global_selected_tools_v1`, JSON `Vec<String>` of registry keys) drives every
global sync write since round 11 D1 (`settings::effective_global_tool_targets`). Three ways that value can be
wrong are each handled **silently**:

1. **Corrupt** (#06). `read_string_list` (`core/settings.rs:404`) turns a JSON parse failure into `None`, which
   `effective_global_tool_targets` reads as *never configured* → **sync to every detected tool**. A corrupt row
   is indistinguishable from a fresh install; the test at `core/tests/settings.rs:257` pins the wrong behaviour.
2. **Unknown key** (#07). A key the registry no longer knows (tool removed in a later version) is loaded, never
   rendered by the Configure Tools modal (`visibleToolChoices` filters `allTools` — `skillPresentation.ts:370`),
   and re-persisted on every save. In a global sync batch it becomes a `Failed { Other("unknown tool") }` outcome
   the operator cannot fix through the UI.
3. **Selected-but-undetected in the Add flow** (#08). `getSelectedInstalledIds` (`useAddSkillFlow.ts:96`)
   intersects the selection with detection before deploying a freshly installed skill, so the tool is skipped
   with no signal. The global sync batch reports the same case as `Skipped { TOOL_NOT_INSTALLED }` — but the
   frontend fold (`reportOutcome.ts:222` `syncOutcome`) drops every non-`failed` result for `install` and `bulk`,
   so the "reported skip" that round 11 D1 relies on never reaches the operator either.

Round 11 D3 ruled: *do not silently drop undetected keys on save — that discards operator data without telling
them.* This round extends the same principle to the read side.

## Decisions (do not re-decide)

- **D1 Corrupt is a refusal, not a default.** Core reads the selection into a tri-state
  `StoredSelection::{Unconfigured, Configured(Vec<String>), Corrupt { detail }}`. A blank/absent row is
  `Unconfigured` (a blank is "unset", like a blank central-repo override). Any other value that is not a JSON
  string array is `Corrupt`. `effective_global_tool_targets` raises `SignalError::SettingCorrupt { key, detail }`
  → new `CommandError::SETTING_CORRUPT` variant → `describeCommandError` copy that names the setting and tells
  the operator to open Configure Tools and save (a save rewrites the row — that is the repair). Sync never fans
  out on a value it could not read. `load_settings` keeps its "malformed parses to default" contract for
  display (`global_selected_tools: None`) and adds `global_selected_tools_corrupt: bool` so the frontend can warn
  once at startup instead of waiting for the first refused sync.
- **D2 Unknown keys are pruned at the read seam, once.** The single reader filters `Configured` keys through
  `tool_adapters::adapter_by_key` and `log::warn!`s each dropped key. Both `load_settings` and
  `effective_global_tool_targets` see the pruned set, so the modal never renders/re-persists it and a sync never
  plans it. There is nothing to render for a tool that no longer exists, so no UI. The write side
  (`SettingUpdate::GlobalToolConfig`) refuses an unknown key with the existing `SignalError::UnknownTool`
  (the UI cannot produce one; the guard keeps the invariant honest for any future caller).
- **D3 Add/import deploys to the selection and reports skips like everything else.** `deployNewSkill` passes
  the full selection (`syncTargets` on, no detection intersection); `no-targets` is reserved for an empty
  selection. `syncOutcome` surfaces `skipped / TOOL_NOT_INSTALLED` as **warning** entries (not errors — the
  install succeeded and nothing is broken), **deduplicated per tool** with the count of skipped skills, for
  both `install` and `bulk`. `toggle` is unchanged (an explicit per-tool action already treats skips as errors).
  This is the frontend half of round 11 D1's "the skip is the operator's only signal".

## Out of scope

- Repairing a corrupt row automatically (D1 makes the operator's next save the repair — no migration).
- Other settings keys: the `read_bool`/`read_i64` "garbage → default" policy stays; a bool/number default is a
  safe fallback, a tool set is not.
- The project-scope tool selection (`configure_project_tools`) — keys are validated at the command seam already.

## Tickets

- `issues/01-corrupt-selection-refuses.md` — D1 (#06)
- `issues/02-unknown-keys-pruned.md` — D2 (#07)
- `issues/03-add-flow-reports-skips.md` — D3 (#08)

## Verification

`npm run version:check && npm run check` (+ `cargo test --all` to match CI). Each ticket names its tests.
