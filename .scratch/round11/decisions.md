# Round 11 — v1.2.11 defect round, grilled decisions

Source: operator smoke tests of v1.2.9/v1.2.10 on two machines (2026-09-08), plus code/DB/filesystem
verification by the parent. Wave B (round 10) slides to v1.2.12.

## Rulings (all "as recommended")

- **Q1 Sequencing** — defects ship first as **v1.2.11**; round-10 wave B becomes v1.2.12. Rationale: the
  reassert writes into tool directories the operator never asked for, and wave B refactors the same
  command surface.
- **Q2 Intent beats detection** — `global_selected_tools` (already persisted, `Option<Vec<String>>`,
  `None` = never configured) becomes authoritative for **every** global sync target set. Detection is
  the fallback for a never-configured install, and a hint for the UI. A selected-but-not-installed key
  is a **reported skip**, never a silent expansion. → D1.
- **Q3 Detection rule** — **DROPPED.** The hypothesis (our own `skills/` dir props up a dead tool) was
  falsified: every detect dir on the operator's machine predates our sync and holds the tool's own
  config (`.gemini/settings.json`, `.copilot/config.json`, `.config/opencode/plugins`,
  `.codeium/windsurf/mcp_config.json`). The false positives are genuine leftovers, not self-inflicted.
  Machine hygiene is an operator task, not a code change. Detection stays `dir.exists()`.
- **Q4 Global `.agents/skills`** — list the virtual group in `global_tool_entries` **alongside** its
  constituents (globally it is an independent 31st target, not an aggregate: Cursor reads
  `~/.cursor/skills`, Codex reads `~/.codex/skills`). Project scope is unchanged. → D2.
- **Q4b Constituent roster label** — rule: *the roster renders exactly when checking this box covers
  those tools.* Project scope keeps `.agents/skills (9 tools)` + roster (load-bearing: one folder, nine
  readers). Global scope shows a plain `.agents/skills` with **no** roster (a roster there would be a
  lie that makes users skip the real Cursor checkbox). Requires a per-scope label — `display_name` can
  no longer be one static string. → D2.
- **Q5 Cleanup** — operator unsynced windsurf/gemini_cli/github_copilot/opencode through the UI
  (verified: zero rows, zero artifacts, no `error` residue — `artifact_removal` behaved per ADR-0002).
  Parent removed the five emptied dirs and `~/.agents`. 24 orphaned `cursor` rows deleted from the live
  DB with operator approval (backup `skills_hub.db.bak-20260908-155131`; all 24 `target_path`s verified
  absent from disk first).

## Defects (evidence)

- **D1 target set ignores intent — three sites, one missing rule.**
  1. `core/refresh.rs:517` `reassert_auto_sync_unlocked` → `installed_keys(&global_tool_entries(home))`.
  2. `src/hooks/useSkillLibrary.ts:222` link button → `syncSkillsToTools([skill], installedToolIds)`.
     Operator-observed on the work machine: selection = {Claude Code, Codex, Pi}, link button deployed
     to every detected tool.
  3. `App.tsx:205` `syncAllManagedToTools(relevantNewlyInstalled)` — respects the selection only when
     `scan_selected_tools_only` is true, i.e. a **scan** setting is doubling as a **sync** gate.
  Live DB evidence: `global_selected_tools_v1 = ["cursor","claude_code","pi"]` while `skill_targets`
  held 32 `codex` rows (unselected) and 24 `cursor` rows (selected, uninstalled).
- **D3 hidden selection entries.** `ToolConfigModal.tsx`: `selectedTools` is seeded from the full saved
  selection, but rows are filtered by `detectedOnly` (default **true**), and `handleConfirm` writes
  `Array.from(selectedTools)` back. An undetected-but-selected tool is invisible, un-untickable, and
  **re-persisted on every save**. This is why the operator's UI showed no Cursor while the DB kept it.
- **D4 unreachable targets.** `SkillCard` iterates `installedTools`, so a target row for an undetected
  tool renders no chip and has no per-skill unsync affordance. The only path that reached the 24 Cursor
  rows was `unsync_all_skills` (`RemovalScope::EveryGlobalTarget`), which would also have deleted all 96
  live Claude Code / Codex / Pi targets.

## Round 2 rulings (post-lane findings)

- **D1 has a FOURTH site.** `core/onboarding_import.rs` `sync_imported_unlocked` had the same
  `policy.tools.unwrap_or_else(|| installed_keys(...))` fallback. The `Some` arm is a per-import
  selection (correctly left alone); the `None` fallback now calls `effective_global_tool_targets`.
  Landed as the parent's integration commit `9fc2b71`, with a test proven to fail against the
  unfixed code (`left: ["claude_code", "codex"]` vs `right: ["claude_code"]`).
- **D5 (new, approved into v1.2.11)** — `RemovalScope::SkillTool` planning falls back to the row's own
  stored `target_path` when no member of the shared-dir group is detected. This overturns
  `global_group_keys`' `None` arm and the test `unsync_skill_from_tool_with_an_uninstalled_group_touches_nothing`.
  Justification: the guard's intent ("an uninstalled tool's directory is left alone") protects against
  paths **derived** from the registry for an absent tool; a `skill_targets` row carries the path of an
  artifact **we created and recorded**, so removing it is cleaning up after ourselves.
  `ensure_path_within_tool_dirs`, the single presence rule and ADR-0002's keep-row-on-failure rule are
  all unchanged. Discovered because D4's chip would otherwise render an orphan that cannot be removed
  while `reportOutcome` shows a **success** toast (`reportOutcome.ts:166` — an empty report has
  `failed === 0`). D5 also gives a zero-target removal report its own non-success outcome.
- **Ticket accuracy** — two of three lane children found factual errors in my tickets (backend change
  wrongly declared unnecessary for D4; `i18n` wrongly listed for D2). The "verify this and tell me if
  I'm wrong" instruction earned its place; keep it in every ticket.

## Execution

- Lanes: D1 (backend+frontend, `anthropic/claude-opus-5` thinking medium) ∥ D2 (registry+catalog+UI,
  medium) ∥ D3+D4 (frontend, low). Astra is out of rotation (weekly quota); Opus runs on the
  **Anthropic** provider, not Cortex.
- Review: single Fable seat (round-10 Q11 sizing).
- Gate per lane: `npm run version:check && npm run check`.

## Closure — 2026-09-15

Shipped: v1.2.11 / 9de83e9. Tickets: 4 terminal (4 done), 0 still open (none).
Residue → BACKLOG: #07, #08. Dropped by name: none.
