# 10 — Tool catalog split, `platform.ts`, one update module, project world composed in the binder

Status: ready-for-agent
Blocked by: 08
Spec: `.scratch/round19/spec.md` — D7 (arch 7, 8, 9, 11).

## Work

- **Tool catalog — `src/hooks/useToolCatalog.ts`** (candidate 11): tool status, labels (`toolLabelById`),
  installedness, `shared_with` map, read by sync orchestration, library, add flow, projects and (later) the strip.
  `useSyncOrchestration` keeps the two target sets and the sync seam; its two unused members go. The project world
  stops loading its own tool status.
- **`src/lib/platform.ts`** (candidate 9): `pickFolder({ title, multiple? })` (one rule, handles rejection, array
  result), `createProgressChannel<T>()`, `updater()` handle, `setZoom()`. The five folder-picker copies, three Channel
  constructions and two updater uses call it; `isTauri` guards live here. Tests mock this module instead of
  `@tauri-apps/plugin-dialog` + `@tauri-apps/api/core`. The fixture's `installFixtureBackend` supplies its adapter
  through the same door (coordinate with `src/fixtures/index.ts`; do not break `mockIPC`).
- **Update module — `src/hooks/useAppUpdate.ts`** (candidate 8): status, version, notes, `check()`, `install()`,
  `ignore()`; `useUpdateChecker` and `SettingsPage`'s inline machine both go; the update modal leaves `App.tsx` for a
  component fed by the module. A third consumer (the passive title-bar affordance) is rendered in ticket 14.
- **Project world in the binder** (candidate 7): `useProjectState` is composed in `App.tsx` like every world; it
  reads the library's catalog (one `ManagedSkill[]`) instead of `getManagedSkills`; `ProjectsPage` becomes a view;
  selected project, matrix and tool status survive navigation. Project assignment verbs are exposed so ticket 09's
  door can call them (`assignToProject` / `unassignFromProject` route here).
- Tests: catalog hook, platform mocks in the affected suites, update module (the previously untestable Settings
  machine), `useProjectState` suite unchanged in intent.
- AGENTS.md § Ambiguity: the world-hook list gains the catalog and update modules; the "no cross-world runtime
  imports" rule restated for the project world's catalog input (passed by App, not imported).

## Gate

`npm run check` green; fixture: Projects keeps its selection across navigation; the folder picker still works in Add,
Settings and Add project; Settings' update check and the startup notice share one state.

## Result

(child fills in)
