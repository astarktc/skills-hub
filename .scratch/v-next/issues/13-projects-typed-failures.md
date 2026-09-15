# 13: Projects world — typed failures everywhere

Status: resolved

Type: task
Blocked by: None (can start immediately)

## What to build

Two verified review findings in the projects world (locations as of `be9a74c`):

1. **`NotFound` typed at 2 of 7 sites.** `src-tauri/src/commands/projects.rs` raises typed `SignalError::NotFound` at ~:347/:353 (bulk assign) but raw `anyhow!("project not found: {id}")` prose at :82, :159, :213, :420, :459 — so toggling a cell in the assignment matrix for a stale project shows untranslated dev prose while the bulk button on the same screen shows localized copy. Convert all five to `SignalError::NotFound { kind: "project", id }` (the variant, `describeCommandError` branch, and `projects.notFoundError` copy already exist — five one-line fixes).
2. **`BulkAssignErrorDto.error` is a raw string.** It carries `format!("{:#}", e)` and `ProjectsPage.tsx` (~:171) toasts it verbatim — the one report DTO that bypasses the error contract. Change `error` to the typed `CommandError` (mirror `SyncTargetStatusDto`, which already carries `error: CommandError` per target), regenerate bindings via `cargo test` (commit them), and render each failure through `describeCommandError` in the frontend.

Behavior otherwise unchanged; `projects.*` i18n namespace remains EN-only by design.

## Acceptance criteria

- [x] All 7 project-lookup failures in `commands/projects.rs` produce the localized not-found message via the typed variant.
- [x] `BulkAssignErrorDto` carries `CommandError`; binding regenerated & committed; `ProjectsPage` renders failures via `describeCommandError`.
- [x] `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `787825a` (Fable 5 low subagent; orchestrator-verified, rebased onto main, combined-tree gate green). All 5 raw `project not found` sites now raise `SignalError::NotFound { kind: "project", id }`; `BulkAssignErrorDto.error` is a typed `CommandError` populated via `CommandError::from_anyhow` (mirroring `SyncTargetStatusDto`), binding regenerated; `ProjectsPage` renders each failure through `describeCommandError` (code fallback for the null/cancelled case). Discovered, left as-is (out of scope): sibling `"skill not found"` raw-prose sites in the same file.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
