# D1 — The global sync target set is the operator's selection, not raw detection

Status: done — 7af11fb

## Problem

Three independent sites each decide "which tools does this write go to", and all three answer
"every **detected** tool". The operator's recorded intent (`global_selected_tools`) is persisted,
surfaced in the Configure Tools modal, and read by **no write path at all**.

Observed twice on real machines:

- Refresh-all with auto-sync on deployed skills to 7 tools when 2 were configured.
- On a second machine, with the selection set to exactly {Claude Code, Codex, Pi}, the per-skill
  **link button** deployed the skill to every detected tool.

Live DB at the time: `global_selected_tools_v1 = ["cursor","claude_code","pi"]`, while `skill_targets`
held 32 `codex` rows (not selected) and 24 `cursor` rows (selected, tool uninstalled). Selection and
reality were fully decoupled.

### The three sites

1. `src-tauri/src/core/refresh.rs:517` — `reassert_auto_sync_unlocked`:
   ```rust
   let missing: Vec<String> = installed_keys(&global_tool_entries(&paths.home))
       .into_iter().filter(|key| !existing.contains(&key.as_str())).collect();
   ```
2. `src/hooks/useSkillLibrary.ts:222` — `handleSyncSkillToAllTools`:
   ```ts
   const report = await syncSkillsToTools([toSyncItem(skill)], installedToolIds);
   ```
3. `src/App.tsx:205` — `syncAllManagedToTools(relevantNewlyInstalled)`: filtered by the selection only
   when `scan_selected_tools_only` is true. A **scan** setting must not gate a **sync**.

## Decision (do not re-decide)

**One rule, defined once, in core:**

```rust
// core/settings.rs
/// The tools a global sync writes to: the operator's recorded selection when
/// they have configured one, otherwise every detected tool. Detection is a
/// fallback for a never-configured install, never an override of intent.
pub fn effective_global_tool_targets(store: &SkillStore, home: &Path) -> Result<Vec<String>>
```

- `global_selected_tools == Some(sel)` → `sel` (**including `Some(vec![])`** — an empty selection means
  "sync nowhere", and is deliberately distinct from `None`; the existing doc comment already says so).
- `global_selected_tools == None` → `installed_keys(&global_tool_entries(home))`.
- The function does **not** intersect with detection. A selected-but-not-installed key stays in the set
  and is reported downstream as a skip — `global_sync` already raises
  `GlobalSyncError::ToolNotInstalled`, which `reassert_auto_sync_unlocked` already maps to
  `PropagationSkip::ToolNotInstalled`. That path must keep working; do not pre-filter it away, or the
  operator loses the only signal that their selection is stale.

**Site 1 (backend)**: `reassert_auto_sync_unlocked` calls `effective_global_tool_targets` instead of
`installed_keys(...)`. It already has `paths.home` and `store`. Do **not** thread a new field through
`RefreshPolicy` or `RefreshPolicyDto` — `reassert_auto_sync` stays a bool, the target set is not a
policy choice the frontend gets to make.

**Sites 2 and 3 (frontend)**: `useSyncOrchestration` gains a derived
`effectiveSyncTargetIds: string[]` (`globalSelectedTools ?? installedToolIds`) exposed on its return.
`useSkillLibrary` takes it through the existing `Pick<>` seam **in place of** `installedToolIds` for
the sync call at line 222 (keep `installedToolIds` if other call sites need it; check before removing).
`App.tsx:205` passes `relevantNewlyInstalled` filtered by the effective set rather than relying on
`scan_selected_tools_only`.

The frontend derivation must mirror the backend rule exactly, including `Some([])` → empty.

## Files

- `src-tauri/src/core/settings.rs` — new `effective_global_tool_targets`.
- `src-tauri/src/core/refresh.rs` — one call site in `reassert_auto_sync_unlocked`; update the module
  doc at the top (lines 21-23) which currently states the invariant as "every installed Tool".
- `src/hooks/useSyncOrchestration.ts` — derive + expose `effectiveSyncTargetIds`.
- `src/hooks/useSkillLibrary.ts` — the `Pick<>` type at line ~37 and the call at line 222.
- `src/App.tsx` — line ~205.
- `CONTEXT.md` — the auto-sync invariant wording, if it repeats "every installed Tool".

**Do not touch**: `core/tool_adapters/` (D2's lane), `src/components/shared/ToolConfigModal.tsx` and
`src/components/skills/SkillCard.tsx` (D3/D4's lane), `core/global_sync.rs` internals,
`core/artifact_removal.rs`.

## Tests

Rust (`src-tauri/src/core/tests/`):
- `effective_global_tool_targets` table test: `None` → installed; `Some(sel)` → sel verbatim;
  `Some(vec![])` → empty; a selected-but-uninstalled key survives into the set.
- Refresh reassert test: selection ⊂ installed ⇒ the reassert creates rows for the selection only, and
  none for the installed-but-unselected tool.
- Refresh reassert test: a selected-but-not-installed tool is reported
  `PropagationSkip::ToolNotInstalled` and creates no row.

Vitest:
- `useSkillLibrary` link-button test: the second argument of `syncSkillsToTools` is the effective set,
  not `installedToolIds`, for a case where the two differ.
- `useSyncOrchestration`: `Some([])` yields an empty effective set (not a fallback to installed).

## Gate

`npm run version:check && npm run check` (add `cargo test --all` when touching Rust).

## Comments

- 2026-09-15 — Status reconciliation: NO STATUS → done — 7af11fb. Evidence: git log v1.2.10..v1.2.11 finds first implementation 7af11fb, then 6f35f7d; verified integration fixes 9fc2b71 and 7dcc4cd, release 9de83e9. src-tauri/src/core/settings.rs:251 and src/hooks/useSyncOrchestration.ts:144 implement selection-or-detection; round11/decisions.md records the import fourth site.
