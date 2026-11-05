# 08 — Surface-state module (area · focused subject · selection · dialog), hash addressing, list lens

Status: done — 607ec96
Blocked by: —
Spec: `.scratch/round19/spec.md` — D3, D4, D5 (grill Q3–Q5); `archive/round18/review/architecture.md` candidates 1 and 5.
Disjoint from ticket 07's files; touches `App.tsx`, `src/hooks/`, `src/lib/`, `src/components/**` state wiring only —
no styling changes.

## Work

- **`src/hooks/useSurfaceState.ts`** — one discriminated value for "what is on screen":
  `area: 'skills' | 'explore' | 'projects' | 'settings'`, `focused: { kind: 'skill', id } | { kind: 'explore', ref } |
  { kind: 'project', id } | null`, `selection: Record<Surface, Set<id>>` (per surface, D4), `dialog: DialogState | null`
  (one union across the app — the project world's `ProjectDialog` union is the precedent and folds in), plus actions
  `open(area, focused?)`, `focus(subject)`, `select/toggle/clear(surface, ids)`, `openDialog/closeDialog`. Invariants
  hold by construction: one dialog at a time (nested asks are a `stack` inside the state, top-layer-only close), a
  focused id that leaves the catalog clears itself (the `effectiveView` fallback goes), Explore preview stops forging a
  `ManagedSkill` with `id: ""` (candidate 12's document value comes in ticket 10/12; here the focused kind is enough).
- **Hash addressing (D5)**: area + focused subject sync to `location.hash` (`#/skills/<id>`, `#/projects/<id>`,
  `#/explore`, `#/settings/<pane>`); nothing else. Back/forward work; the fixture and critiques can deep-link. Storage
  of preferences stays in `preferences.ts`.
- **List lens — `src/hooks/useListLens.ts`** (candidate 5): query, sort, grouping incl. a **singleton-repo fold**
  ("Individual sources" group when a source has one skill), flat mode, view mode, persisted preferences (through
  `persistedPreference.ts`), and the selection's visible subset. Instantiated by My Skills and the matrix now (the
  matrix's duplicated sort control and `groupByRepo` cell go), by the pickers and the palette later. Pure functions
  stay in `skillPresentation.ts` (extend `groupSkillsByRepo` with the fold).
- The 14 `useState` cells listed in candidate 1 are deleted from `App.tsx`, `useSkillLibrary`, `useAddSkillFlow`,
  `useSyncOrchestration`, `useExploreState`, `useUpdateChecker`, `useProjectState`; their consumers read the module.
  Behaviour is unchanged for the operator — this ticket is a refactor with no visible delta except the hash.
- Tests: `renderHook` suites for both modules (transitions, invariants, hash round-trip via jsdom `location`);
  existing hook tests updated where they asserted the deleted cells.
- AGENTS.md § Ambiguity: the state-management bullet gains the two modules by name (surface state owns "what is on
  screen"; world hooks ask it to open/close; the lens is per list surface).

## Gate

`npm run check` green; fixture walk: every surface still opens, deep-link `#/skills/<id>` opens the detail, Escape
closes one layer.

## Result

Implemented 2026-09-27 (child, Opus 5.5). Refactor with no visible delta except the hash (two deliberate edge deltas below).

**New modules**
- `src/hooks/useSurfaceState.ts` — one `SurfaceValue { route, stack, queue, selection }`:
  - `Route` is a union **per area** (`skills`→`{kind:'skill',id}`, `explore`→`{kind:'explore',ref: ExplorePreview}`,
    `projects`→`{kind:'project',id}`, `settings`→`{kind:'settings',pane}`), so a focused subject can never sit in the
    wrong area. Actions: `open(area)`, `focus(subject)` (shows the subject's area), `unfocus()`, `reconcile(kind, ids)`.
  - `selection: Record<'skills'|'matrix'|'explore', ReadonlySet<id>>` + `select/toggle/clearSelection` (D4; no UI
    consumer yet — ticket 13).
  - `DialogState` — **one union across the app** (addSkill, import, gitPick, localPick, deleteSkill, invocationEdit,
    repoint, toolConfig, newTools, notifications, update, projectAdd/Edit/ToolConfig/Remove — the project world's
    `ProjectDialog` folded in). Stack rules: `openDialog` replaces (one dialog at a time); `pushDialog` layers a nested
    step (the git/local picker over Add); `closeDialog(kind)` is the operator close and acts **only if `kind` is the
    top layer** (so an Escape reaching two Base UI roots closes one); `dismissDialog(kind)` is a flow finishing its own
    dialog wherever it sits; `offerDialog` queues a system announcement until the stack empties (new-tools and update
    at startup can no longer both open); `withdrawDialog`.
  - **Asks** (`ask(request) → Promise<boolean>`, `pendingAsk(kind)` → payload + `resolve`): the shared-dir and overwrite
    confirmations are stack layers. Every ask settles exactly once — answered, displaced by a newer ask of its kind,
    dropped with the stack / closed by the operator (declined), or declined on unmount.
  - `useFocusReconciliation(surface, kind, ids|null)` — layout effect: a focused id, selected id or subject dialog
    whose id is not in the loaded catalog clears itself before paint (history *replaced*); `null` = not loaded, so a
    deep link survives the first empty render. Runs on catalog **and** route/selection change (a stale link clears too).
  - Hash grammar (pure `formatHash` / `parseHash`): `#/skills`, `#/skills/<id>`, `#/projects`, `#/projects/<id>`,
    `#/explore`, `#/settings`, `#/settings/<pane>` (segments URI-encoded). Navigation `pushState`s; reconciliation and
    normalising an empty/unknown hash `replaceState`; `hashchange` + `popstate` feed back in. An Explore preview is not
    addressable (`#/explore`; it needs a clone). Dialogs, selection, drafts never enter the hash.
- `src/hooks/useListLens.ts` — query, sort, Grouped/Flat (`groupByRepo`, `groups: RepoGroup[] | null`), optional
  singleton fold, view mode, persistence via `usePersistedPreference` over each surface's preference from
  `preferences.ts` (keys unchanged: `skills-groupByRepo`, `skills-viewMode`, `skills-projects-groupByRepo`), and
  `visibleSelection` (selected ids the lens shows, sort order). Memoised return, so `FilterBar`/`SkillsList` take it whole.
- `src/lib/skillPresentation.ts` — `groupSkillsByRepo(skills, labels, { foldSingletons })` + `INDIVIDUAL_GROUP_KEY`:
  single-skill repos fold into one "Individual sources" group (after repos, before Local; input order). **Off** in both
  instances today (ticket 11 turns it on for My Skills); i18n key `individualSources` added.
- `src/components/shared/LensSortControl.tsx` — the one sort control (FilterBar + matrix; `compact` for the matrix's
  `btn-sm`). Same classes as before.

**What moved from where** (all `useState` cells deleted from their old homes)
- `App.tsx`: `activeView`, `exploreDetailSkill`, `showNotifications` → surface (`route`, `{kind:'explore'}` focus,
  `notifications` dialog); `effectiveView` fallback deleted (reconciliation clears the focus instead); `searchQuery`,
  `sortBy`, `groupByRepo`, `viewMode`, `visibleSkills` → My Skills lens. Explore lists load in an effect keyed on
  `area === 'explore'` (so Back/links load them too).
- `useSkillLibrary`: `detailSkillId` (+`detailSkill/openDetail/closeDetail`) → surface focus (App resolves the skill);
  `invocationEditSkillId`, `pendingDeleteId`, `repointSelection` → `invocationEdit` / `deleteSkill` / `repoint` layers.
  Catalog is `null` until first load; exposes `managedSkillIds` for reconciliation. The stale-notification
  Change-source path still warns `errors.skillGone` and now also withdraws its layer.
- `useAddSkillFlow`: `showAddModal`, `showImportModal` → `addSkill` / `import` (import *call* untouched — ticket 05).
- `useCandidatePick`: `visible` → `gitPick`/`localPick` layer pushed over Add (`source.dialogKind`, `deps.surface`).
- `useSyncOrchestration`: `showNewToolsModal` (now **offered**), `showToolConfigModal`; `sharedDirPending`/`cancel…`
  and `overwritePending`/`cancel…` gone from its return — App renders both modals from `surface.pendingAsk`.
- `useSharedDirConfirmation`: keeps the decision + label arithmetic; `request` raises `surface.ask`; `pending`/`cancel` gone.
- `useOverwriteConfirmation.ts` (+ test) **deleted**: its displacement/settle-once/unmount semantics are the surface
  ask mechanism (tested generically there). `OverwriteRow` / `OverwritePending` types moved to `useSyncOrchestration.ts`.
- `useUpdateChecker`: the found version stays data; dialog visibility is the offered `update` layer.
- `useProjectState`: `dialog`/`openDialog`/`closeDialog` + `ProjectDialog` deleted. `ProjectsPage` takes `surface`:
  the focused project (hash) drives `selectProject` once the list has loaded; stale ids clear; project dialogs are
  withdrawn when the page unmounts.
- `useExploreState`: no more forged `ManagedSkill { id: "" }` — `onOpenPreview(ExplorePreview)`.
  `SkillDetailView` takes `subject: { kind:'managed', skill } | { kind:'explore', preview }` (`isExplorePreview` prop gone).
- `Header`: `activeArea: Area`, `onViewChange(area)`. `AssignmentMatrix`: own `sortBy` + persisted `groupByRepo` cells
  and hand-built sort control gone → `useListLens` + `LensSortControl`.

**Deliberate edge deltas**: (1) Clicking the Projects tab while already on a project now shows the placeholder (the
tab addresses `#/projects`, no focus) — it used to keep the matrix; consistent with every other tab unfocusing.
(2) New-tools and update announcements at the same startup show one after the other instead of stacked.

**Tests** — `src/hooks/useSurfaceState.test.ts` (24): hash grammar round-trip / preview not addressable / unknown
hashes; opens on My Skills & normalises without a history entry; deep link from the opening hash; focus pushes +
unfocus; follows a hash change; **Back returns to the previous area** (`history.back()` in jsdom); unknown address
replaced; Explore preview kept but not addressed; reconciliation (focus/selection/subject dialog cleared with replace;
other kinds untouched; deep link survives the unloaded render; stale link clears); per-surface selection; dialog stack
(replace, layer + top-only close, dismiss anywhere, offered queue/withdraw); asks (answer once, displacement + stale
resolver, dropped/closed → declined, unmount → declined). `src/hooks/useListLens.test.ts` (4): query/sort flat;
grouping + fold; persistence under each surface's key; visible selection. `skillPresentation.test.ts`: fold case.
Updated: `useSharedDirConfirmation.test`, `useCandidatePick.test`, `useAddSkillFlow.test`, `useSkillLibrary.test`,
`useSyncOrchestration.test` (compose the real surface module the way App does; +3 tests: offered new-tools waits,
tool-config dialog open/close/confirm, stays open while loading), `useProjectState.test` (dialog describe moved to the
surface suite). The `invoke → fold → completion` table now opens only the dialog its action settles (one at a time).
All suites that mount the surface call `afterEach(cleanup)` (no vitest globals → no RTL auto-cleanup; a leftover
surface's window listeners would answer the next test's hash events).

**Fixture walk** (5175, `rich` + `first-run`, t3 preview): list → card name → `#/skills/skill-007-api-contract-review`
detail; Back → `#/skills` list; Forward → detail; Explore tab → `#/explore`, Back → the detail again; Projects →
select → `#/projects/prj-skills-hub` matrix; Back → `#/projects` placeholder; `#/projects/prj-quartermaster` link →
that matrix; `#/projects/nope` → replaced by `#/projects`; matrix lens sort + Group by repo (persists
`skills-projects-groupByRepo`, My Skills key untouched); Settings gear → `#/settings`, Back → Explore; fresh load of
`…#/skills/skill-007-api-contract-review` opens the detail; `#/skills/no-such-skill` → `#/skills`. Dialogs: Add →
git URL → picker **layered over Add**; Escape closes the picker only, second Escape closes Add; Notifications, Configure
Tools, Remove (delete), Change source, Invocation mode, Review & Import each open and close on Escape. Explore Preview →
detail with repo line "anthropics/skills · just now · 2 files", Install, no Change source; Back → Explore.

**Gate**: `npm run version:check` → Version OK (1.2.18); `npm run check` → exit 0 (lint clean, styles:check ok,
vitest 25 files / 562 tests passed, build ok, rust fmt/clippy ok, cargo test 769 passed). Earlier mid-run failures were
all in ticket 02's in-flight files (`src/fixtures/**`, `commandError.ts`, `health` on `ManagedSkillDto`) and cleared.

**Steer (orchestrator, after ticket 02): three-way ask answers.** Ticket 02 made a declined overwrite ask record
`Keep` (pair → `disabled`), but a dismissal must record nothing. The surface ask now resolves
`AskAnswer = 'confirmed' | 'declined' | 'dismissed'`: `confirmed`/`declined` come only from the explicit buttons
(`pendingAsk(kind).answer(...)`); Escape/outside press (the top-layer `closeDialog`), displacement by a newer ask of the
kind, the stack being replaced/reconciled, and unmount all settle `dismissed`. (`useOverwriteConfirmation` no longer
exists — its `request()` is `useSyncOrchestration`'s `requestOverwriteConfirmation(rows) → Promise<OverwriteAnswer>`,
`'overwrite' | 'keep' | 'dismissed'`, still the request-resolves pattern.) `syncSkillsToTools` sends
`decision: 'keep'` only for `'keep'`; on `'dismissed'` it returns the first report as settled — no override, no retry
batch. `OverwriteModal` composes the Dialog primitives itself (Keep existing → `declined`, Overwrite → `confirmed`,
`onOpenChange(false)` → `dismissed`) because `ConfirmDialog` funnels Escape into its cancel callback (ui/ out of
bounds; a ui ticket could give ConfirmDialog an `onDismiss`). `useSharedDirConfirmation` checked: its "no" records
nothing (the toggle just does not happen), so it keeps `request() → boolean` (true only for `confirmed`; Cancel and
dismissal both false) — documented in the hook. Tests: surface suite — confirmed once, explicit `declined`,
displaced → `dismissed` (stale answer no-op), Escape/stack-replaced → `dismissed`, unmount → `dismissed`;
`useSyncOrchestration.test` — confirmed → one retry with `overwrite`; **Keep existing** → retry with `keep`;
`it.each` **dismissed** by Escape / displacing ask / stack replaced → exactly one batch, report as settled;
`useSharedDirConfirmation.test` — Escape resolves false. Fixture (`rich`, zod-schema-first → Cursor, occupied
folder): Escape closes the ask and the next click raises the ask again (nothing recorded); Keep existing → the Cursor
pill reads "Cursor: disabled — Refresh and sync leave it off". Gate re-run: `version:check` OK, `npm run check`
exit 0 (vitest 567 passed, cargo 769 passed).

**Deferred / notes for 09–17**
- `src/components/ui/confirm-dialog.tsx`'s comment still names `useOverwriteConfirmation` (ui/ was out of bounds);
  `docs/adr/0006-styling-architecture.md` (l. 98) and tickets 04/16 also name it — the overwrite ask is now `surface.ask({ kind: 'overwrite', rows })`
  rendered from `surface.pendingAsk('overwrite')`; ticket 04's list shape goes into that `AskRequest` payload.
- Selection has no UI consumer yet (ticket 13: rows, `x`/`Esc`, bulk rail read `surface.selection.skills` and
  `lens.visibleSelection`). The palette/pickers instantiate `useListLens` later (tickets 16, 23).
- `#/settings/<pane>` parses and round-trips but `SettingsPage` has no panes yet (ticket 20 reads `focused.pane`).
- Singleton fold is implemented but off (ticket 11). Explore preview is still the full-page detail (ticket 12's inspector).
- The project world still mounts with its route (candidate 7 / ticket 10); ProjectsPage reconciles its own focus.

## Comments

- 2026-10-29 — **done — 607ec96** (post-rebase SHAs on `origin/main`; the pre-rebase SHAs cited above exist only in the orchestrator's local history). Shipped in 1.3.0 (`v1.3.0`, published 2026-10-28; operator smoke on the main Mac passed 2026-10-29, ticket 19 § Smoke outcome). 19a closure: `round19/spec.md` § Closure — 2026-10-29.
