# 10 — Tool catalog split, `platform.ts`, one update module, project world composed in the binder

Status: implemented
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

Implemented 2026-09-29 (child, Opus 5.5). Frontend-only; no Rust, bindings, i18n or styles touched.

**The four modules and their interfaces**
- `src/lib/platform.ts` (new) — everything native that is not a command. `pickFolder({ title, multiple? }) →
  Promise<string[]>` (the one rule: a titled directory dialog, `[]` on cancel, at most one path unless `multiple`, a
  picker failure rejects and every caller catches), `createProgressChannel<T>(onMessage) → Promise<Channel<T>>`,
  `checkForUpdate() → Promise<AppUpdateHandle | null>` (the updater handle: `{ version, downloadAndInstall(),
  releaseNotes() }` — the GitHub release-notes fetch moved behind it), `setZoom(factor)`, `appVersion()`. The `isTauri`
  guard lives here: outside Tauri every call rejects `PlatformUnavailableError`, except `setZoom` (best-effort no-op).
  Two adapters: the lazy-importing Tauri adapter and whatever `installPlatformAdapter(adapter)` installs. (Named
  `checkForUpdate` rather than the ticket's `updater()` — one function that returns the handle.)
- `src/hooks/useToolCatalog.ts` (new) — `{ toolStatus, tools, toolLabelById, installedToolIds, installedTools,
  sharedToolIdsByToolId, projectToolStatus, loadProjectToolStatus() }`. Loads `getToolStatus` once at startup; the
  project tool entries (`getProjectToolStatus`) are read on demand and re-read each time the project world asks (its
  labels join `toolLabelById`). Memoised value.
- `src/hooks/useAppUpdate.ts` (new) — `{ status: idle|checking|up-to-date|available|installing|done|error, version,
  notes, error, currentVersion, check(), install(), dismiss(), ignore() }`. Mount: reads the app version and runs a
  **silent** startup check (failure / nothing / an ignored version → `idle`; a found version is adopted and the
  `update` dialog is **offered**). `check()` is the operator's: reports up-to-date / available (ignored version
  included, no dialog) / `error`. A failed install keeps `available` + `error` text and notifies (history). `dismiss`
  closes the notice but the release stays known; `ignore` records the preference and closes.
- Project world in the binder — `useProjectState()` is instantiated in `App.tsx`; its value is memoised and handed to
  `ProjectsPage` as `state`. It no longer loads `getManagedSkills` or `getProjectToolStatus` (`skills`, `toolStatus`,
  `loadToolStatus` removed from `ProjectState`); on mount it calls `listProjects` only. New verbs for ticket 09's door:
  `assignToProject(projectId, skillId) → ProjectSyncReport` and `unassignFromProject(projectId, skillId) →
  RemovalReport` — any project, the view applied (row always, matrix when shown), failure-path `refreshView` then
  rethrow. `bulkAssign` / `bulkUnassign` (selected project, pending cells) now delegate to them.

**What moved from where**
- `useSyncOrchestration`: tool status, `tools`, `toolLabelById`, `installedTools`, shared-dir map → catalog (received
  as `catalog` dep). The two unused members `isInstalled` / `installedToolIds` are gone from its return. It keeps the
  saved selection, `effectiveSyncTargetIds`, the Add modal's `syncTargets`, the seam and both asks. `syncTargets` is now
  derived (`chosenSyncTargets ?? defaults(status, selection)`) instead of set inside the load, since status and settings
  load independently; the defaults and the new-tools offer wait for the settings snapshot to settle (as before).
- `useSkillLibrary` / `useAddSkillFlow`: `toolLabelById` / `tools` come from `catalog`, not `sync`.
- Folder picker ×5 → `pickFolder`: `useAddSkillFlow.handlePickLocalPath`, `useSettingsState.handlePickStoragePath`,
  `useSkillLibrary.pickRepointFolder`, `ProjectsPage.handleUpdatePath` (the former array-accepting static import; now
  titled `projects.updatePath`), `AddProjectModal` Browse → new `onPickFolder` prop fed by
  `ProjectsPage.handlePickProjectFolder` (the unhandled rejection is now caught → `notifyError`).
- `Channel` ×3 → `createProgressChannel` (sync batch, refresh/update, import). Webview zoom ×2 → `setZoom`.
- `useUpdateChecker.ts` **deleted**; `SettingsPage`'s inline `UpdateStatus` machine + `getVersion` load **deleted** —
  it takes `update` (the module) instead. The update modal left `App.tsx` for `src/components/shared/UpdateDialog.tsx`
  (same markup/classes; fed `open` + the module).
- `ProjectsPage` is a view of the world: props `state`, `skills` (= `library.managedSkills`), `catalog` (labels +
  project tool entries); it keeps the report folds (AGENTS.md: folded in `ProjectsPage`) and its focus→select /
  reconciliation effects. `handleConfigureToolsFromToolbar` now catches a failed tool-status read.
- Fixture: `installFixtureBackend` installs the fixture platform adapter (picker rotates `dialogPicks`, channels are
  plain `onmessage` sinks, `?update=<version>` makes the updater find a release — `&updateFails` fails its install —,
  zoom → CSS zoom, version `…-fixture`). `mockIPC`/`mockWindows` stay installed as a backstop that logs and refuses any
  native call bypassing the seam (the old per-plugin canned answers are gone — nothing reaches them).
- **Navigation delta (reverses ticket 08's edge delta 1 for Projects):** the Projects tab focuses the project the world
  still holds (`#/projects/<id>`, a real push) instead of the bare placeholder; Back to `#/projects` still shows the
  placeholder.
- AGENTS.md § Ambiguity: world list gains the Tool catalog and the update module; every world (project world included)
  is composed in App; the catalog-input rule restated for the project world; the frontend-tests bullet names
  `src/lib/platform.ts` as the native seam/mock point.

**Tests** — new `src/lib/platform.test.ts` (4: picker shape + title/multiple, cancel `[]` / failure rejects, isTauri
guard incl. zoom no-op, installed adapter answers), `src/hooks/useToolCatalog.test.ts` (4: one global load + derived
labels/installedness/shared map, project entries on demand + re-read + labels, failed project read rejects, failed
global load → empty catalog), `src/hooks/useAppUpdate.test.ts` (7: startup offers notice + notes; quiet when
current/ignored/failing; startup release is what Settings shows and survives dismiss; Settings check → up-to-date /
available-without-notice (ignored too) / error; install → done; failed install keeps release + error + notify; ignore).
`useProjectState.test` (+4: loads only `listProjects`; assign/unassign on a non-selected project updates its row only;
on the selected project the matrix takes the view; thrown verb converges via `getProjectView` then rethrows; the
`getManagedSkills` stub case removed). `useSettingsState.test` (+4: picked folder → `central_repo_path` + library
reload; cancel writes nothing; picker failure → error; zoom clamps → `setZoom` + save). `useAddSkillFlow.test` (+1
local picker: fill / cancel / failure). Platform mocks replace `@tauri-apps/api/core` / `plugin-dialog` mocks in
`useSyncOrchestration.test` (now composes the real catalog), `useSkillLibrary.test` (picker test asserts
`{ title }`; the multi-select case is gone — the seam answers one path), `useAddSkillFlow.test`.

**Fixture walk** (5175, t3 preview): `?scenario=rich&update=9.9.9` → startup notice "New version v9.9.9" + fixture
notes; closed; `#/settings` shows "Update available: v9.9.9 · Download & Install" **without a check** (one state);
Download & Install → "Installing..." → "Update installed. Restart the app to finish." Settings Browse → storage path
`/Users/alex/Projects/team-skills`. Projects: select quartermaster → `#/projects/prj-quartermaster`; My Skills →
Explore → Projects tab → `#/projects/prj-quartermaster` with the matrix present on the first frame (no placeholder, no
reload). Add project → Browse Folder fills the path; Configure Tools (toolbar) lists the catalog's project entries.
Add skill → Local Folder → Browse fills `/Users/alex/Projects/team-skills`.

**Gate**: `npm run version:check` → Version OK (1.2.18); `npm run check` → exit 0 (lint clean, styles:check ok, vitest
29 files / 629 passed, build ok, rust fmt/clippy ok, cargo test 775 passed). Mid-run failures seen earlier were all
ticket 03's in-flight copy changes (`reportOutcome*`, composite headline) and cleared.

**Deferred**
- Ticket 09: route the door's project verbs to `projects.assignToProject` / `unassignFromProject` (App passes them);
  their report folds (`projectSyncOutcome` / `removalOutcome`) stay the caller's.
- Ticket 14: the passive title-bar affordance reads `useAppUpdate` (`status === "available"`, `version`) and opens
  `{ kind: "update" }`; `UpdateDialog` already renders from the module.
- Ticket 17: the matrix's Tool labels can read `catalog.toolLabelById` (project entries included once loaded).
- `src/tauri-plugin-dialog.d.ts` (the dialog type shim) is now only consumed by `platform.ts`; left in place.
