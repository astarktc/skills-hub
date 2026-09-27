# Frontend architecture review — `src/` @ `e4639fe`

Round 18, ticket 05. Skill: `improve-codebase-architecture` with the `codebase-design` vocabulary (module, interface,
implementation, depth, seam, adapter, leverage, locality, deletion test). Scope: `src/` only; Rust core, ADRs 0001–0005,
the ZH locale (ticket 03) and fixture mode (ticket 04) are out of scope. Everything was read from the commit
(`git archive e4639fe`), not from the working tree, which other children are editing.

Companion visual report: `.scratch/round18/review/architecture.html` (same candidates in the same order).

**Ranking rule.** Leverage against the round-19 list (spec D8): multi-select bulk (#52), group Deploy all (#39), per-row
and inspector project assignment, the issues banner and chip (#49), the "Synced N/M" strip, the detail health ladder
with a copyable source, the sticky install summary, picker search (#47), the `SKILL.md`-gated scan (#46), the passive
title-bar update affordance, singleton fold with a flat mode, and the ⌘K palette. Plus the D1/D2 world: a Tracker
List shell with a left sidebar and an inspector.

**This review stops before interfaces.** Each "Solution" says what moves and where it lives. Ticket 09 (the grill)
picks candidates and designs their interfaces.

## Measurements at `e4639fe`

| Fact | Value |
|---|---|
| `className=` sites | 522 in total, 500 outside the dormant `Layout.tsx` and `Dashboard.tsx` (the spec says 482; the count depends on how you count) |
| Most class sites | `SettingsPage` 63, `ExplorePage` 45, `SkillDetailView` 40, `AssignmentMatrix` 37, `SkillCard` 34, `AddSkillModal` 26 |
| Class-token families at sites | `btn*` 133 · `settings-*` 55 · `explore-*` 45 · `matrix-*` 27 · `pick-*` 26 · `detail-*` 25 · form (`input`/`label`/`form-group`/`helper-text`) ≈ 45 |
| `App.css` rule families | `explore` 48 · `matrix` 41 · `settings` 40 · `markdown` 30 · `modal` 23 · `tool` 23 · `notif` 22 · `project` 22 |
| Tokens | 38 CSS custom properties in `index.css` `:root` / `[data-theme='dark']`, used by 518 `var(--…)` reads in `App.css`. Only 11 hex and 13 `rgba()` literals remain in `App.css` |
| World-hook interface widths (returned members) | `useSkillLibrary` 31 · `useSyncOrchestration` 30 · `useAddSkillFlow` 30 · `useProjectState` 29 · `useStatusReporter` 18 · `useSettingsState` 15 · `useExploreState` 14 · `useUpdateChecker` 7 |
| Places where modal or route state lives | App (`activeView`, `exploreDetailSkill`, `showNotifications`), `useSkillLibrary` (detail, invocation edit, delete, re-point), `useAddSkillFlow` (add, import, 2 pickers), `useSyncOrchestration` (tool config, new tools, 2 confirmations), `useUpdateChecker`, and `useProjectState` (the `ProjectDialog` union) |
| `loading` threaded as a prop | 18 `loading={…}` pass sites into 19 components declaring `loading: boolean` |
| `t={t}` in `App.tsx` | 19 |

## Ranked candidates

### 1. Deepen surface state into one module: route, selection, inspector target and dialog — **Strong**

- **Files:** `App.tsx` 57–69 (`activeView`, `exploreDetailSkill`, `showNotifications`), 84–86 (the `effectiveView`
  fallback: `detail` without a `detailSkill` quietly becomes `myskills`), 128–200 (open/close/back handlers);
  `hooks/useSkillLibrary.ts` 79–94 (`detailSkillId`, `invocationEditSkillId`, `pendingDeleteId`,
  `repointSelection`); `hooks/useAddSkillFlow.ts` 82–83 (`showAddModal`, `showImportModal`) plus the `git`/`local`
  pick visibility; `hooks/useSyncOrchestration.ts` 120–121 (`showNewToolsModal`, `showToolConfigModal`);
  `hooks/useExploreState.ts` 124–150 (builds a fake `ManagedSkill` with `id: ""` so it can be shown as a detail);
  precedent: `components/projects/useProjectState.ts` 32–36 (`ProjectDialog` union).
- **Problem:** "What is on screen" is spread across about 14 independent `useState` cells in 5 modules. No invariant
  holds them together: two dialogs can both be open, and a detail id can outlive its skill, which is why
  `effectiveView` exists. The skills world keeps a flag and a target per modal. The project world already fixed this
  shape with a single `ProjectDialog` union (its comment reads "the two can never disagree"), but the fix never
  crossed to the skills world. Nothing represents "the selected skills".
- **Solution:** Put one in-process module in charge of the surface: which top-level area is showing, which skill (or
  Explore preview) the inspector shows, which skills are selected, and which dialog is open, as one discriminated
  value. World hooks keep their data and actions and ask this module to open or close surfaces, the way
  `ProjectsPage` calls `state.openDialog`. Explore preview would stop pretending to be a `ManagedSkill`.
- **Benefits:**
  - *Leverage:* the ⌘K palette, the bulk rail, the inspector and the sidebar shell (all D8/D2) need one door that
    can open any surface by name. Today no such door exists.
  - *Locality:* the "can two dialogs be open?" and "stale detail id" bugs live in one place.
  - *Tests:* `renderHook` tests of surface transitions replace ad-hoc open/close assertions in four hook suites.
- **Dependency category:** in-process.
- **Deletion test:** today's state is not a module, so there is nothing to delete. The complexity is the ~14 cells
  plus `effectiveView`, and it concentrates if they are merged.

### 2. Deepen the skill action set so every verb acts on N skills through one door — **Strong**

- **Files:** `hooks/useSkillLibrary.ts` (whole file; 31-member interface at 459–494); `App.tsx` 293–335 (11
  callbacks threaded into `SkillsList`); `components/skills/SkillsList.tsx` 9–33; `components/skills/SkillCard.tsx`
  36–60 and 101–116 (the repair-handler map); `components/skills/SkillDetailView.tsx` 35–46 (the detail reaches only
  `onRepoint`).
- **Problem:** Every per-skill action is its own callback. They are threaded App → `SkillsList` → `SkillCard` with
  mixed argument shapes: `onDelete(skillId)`, `onUnsync(skillId)` and `onInvocationClick(skillId)` take an id, while
  the other callbacks take a `ManagedSkill`. Each action handles exactly one skill, even though the sync seam already
  takes arrays (`syncSkillsToTools(skills[], tools[])`, `useSyncOrchestration.ts` 325–367). The interface is as wide
  as the implementation: each `handleX` is a thin `runAction` plus `invokeTauri` plus `applyOutcome`.
- **Solution:** Put the skills world's verbs (sync to targets, unsync, update/restore, re-point, detach, delete,
  invocation edit, assign to project) behind one action door that takes a verb and a set of skills. The door decides
  whether the verb is one batch or one skill at a time, where confirmation happens, and which fold settles the result.
  Card, inspector, bulk rail and palette all become callers of that door.
- **Benefits:**
  - *Leverage:* #52 bulk and #39 group Deploy all become new callers of the existing door. They need no new plumbing.
  - *Locality:* the "a thrown sync also reloads" rule (`syncOrReload`, 236–247) and the shared-dir ask stop being
    per-handler knowledge.
  - The 11-prop drill collapses to one handle.
  - *Tests:* one parametrised suite over verbs replaces about 12 near-duplicate handler tests.
- **Dependency category:** in-process (backend reached through the existing `invokeTauri` seam).
- **Depends on:** candidate 1, which owns the selection that bulk verbs act on.

### 3. Deepen skill health and the source line into one pure fold — **Strong**

- **Files:** `lib/skillPresentation.ts` 69–145 (`sourceKind`, `importedSourceLine`, `skillSourceLabel`, `repoInfo`)
  and 270–384 (`unlocatableRepairs`, `skillToolChips`); `components/skills/SkillCard.tsx` 81–125 (source line plus
  copy value plus repairs plus chip split); `components/skills/SkillDetailView.tsx` 461–475 (a second, different
  source-line rule, with no copy); `components/skills/InvocationModeBadge.tsx` 30–36 (Edit conflict shown only as a
  badge class); `syncStatus.ts`.
- **Problem:** There is no single answer to "is this skill healthy, and why not?". The card composes Unlocatable
  state, Edit conflict and per-target status inline. The detail view shows none of them and re-derives the source
  line with its own ternary. The round-18 bake-off found that neither prototype had a skill-level health channel
  (Unlocatable, fetch failure, Edit conflict) separate from target health. The frontend has no module where that
  channel could live.
- **Solution:** Add a pure fold in the presentation module that takes a Managed skill (and the Tool catalog) and
  answers its health rows at skill level and target level, with severity, plus its source line (label, href, copy
  value, icon kind). Card, inspector ladder, issues banner and "Synced N/M" strip all read that one answer.
- **Benefits:**
  - *Leverage:* four D8 items read the same fold: the health ladder, the #49 banner and chip, the Synced N/M strip and
    the copyable source.
  - *Locality:* one precedence rule for severity. This is the frontend twin of `ProjectSyncStatus`'s fold.
  - *Tests:* the fold extends `skillPresentation.test.ts`, which is already the pattern (pure plus corpus).
- **Dependency category:** in-process, and the cheapest candidate on the list.

### 4. Styling: tokens plus a primitives module (D3, evaluated) — **Strong, with three corrections to the spec's premise**

- **Files:** `index.css` 1–95 (Google Fonts `@import`, 38 tokens, dark overrides); `App.css` (3,696 lines);
  `components/shared/Modal.tsx` (the only existing primitive); highest-density sites: `SettingsPage.tsx` (63),
  `ExplorePage.tsx` (45), `SkillDetailView.tsx` (40), `AssignmentMatrix.tsx` (37), `SkillCard.tsx` (34);
  `SkillDetailView.tsx` 389 (reads `data-theme` from the DOM) and 319–365 (syntax-highlighter colours hard-coded
  outside the tokens); `FilterBar.tsx` and `AssignmentMatrix.tsx` 118–135 (the same sort button hand-built twice).
- **Where the class sites cluster:** about 45–50% of the ~500 sites resolve to six recurring shapes: Button (the
  `btn*` family alone is 133 tokens), form field (label, input, select, helper text; about 70 counting `settings-*`
  fields), dialog shell, chip/pill/badge (`tool-pill`, `repo-pill`, `invocation-badge`, status dots), checkbox or
  switch, and tabs or segmented control. The other half is surface layout (`explore-*`, `matrix-*`, `detail-*`,
  `tree-*`, `notif-*`), which should become utilities in JSX, not primitives.
- **Verdict on D3:** a primitives module (`src/components/ui/`) is the right deep module for those six shapes.
  - *Deletion test:* deleting a Button primitive would put variant logic back at 133 sites, so it earns its keep.
  - *Depth:* comes from the primitive owning focus ring, disabled, pending, reduced motion and ARIA, not from
    `variant`/`tone`/`size` alone.
  - *The existing precedent is shallow in styling:* `Modal` takes 4 className escape hatches (`className`,
    `backdropClassName`, `bodyClassName`, `footerClassName`), and styling leaks across that seam. It also lacks focus
    trap and focus return, which is a D5 gap. A shadcn Dialog would absorb both problems.
- **Corrections to the spec's premise:**
  1. **Tokens already exist.** 518 `var()` reads against 38 properties and only 24 literals. Moving them into
     `@theme` relocates tokens that are already consistent. The real token work is a new set of values for the
     world, not consolidation.
  2. **`App.css` will not reach literal zero.** `markdown-*` (30 rules, rendered Markdown) and the syntax-highlighter
     theme are a prose layer, not surface styling. The CI guard needs an explicit exemption or a named prose
     stylesheet.
  3. **Theme resolution is not a module.** `useSettingsState` 130–156 writes `data-theme`; `SkillDetailView` reads
     it back from the DOM on each render, so the value is stale when the theme changes while the detail is open. The
     world module needs to expose the resolved theme.
- **Open conflict with AGENTS.md:** "components get no JSX tests". A primitives module whose interface is props can
  then be verified only through types and visual review. Ticket 08 or 09 should decide whether primitives get
  role/ARIA tests, since D5 makes accessibility a hard gate.
- **Dependency category:** in-process. Sequencing (D9) puts this first in execution regardless of rank.

### 5. Deepen the library lens: query, sort, group, fold, view mode and selection as one stateful module — **Strong**

- **Files:** `App.tsx` 68–72 and 129–136 (`searchQuery`, `sortBy`, `groupByRepo`, `viewMode`, `visibleSkills`);
  `components/skills/FilterBar.tsx` 12–30; `components/skills/SkillsList.tsx` 56–63;
  `components/projects/AssignmentMatrix.tsx` 73–89 (its own `sortBy`, its own persisted `groupByRepo`, the same
  pure calls) and 118–135 (duplicated sort control); `lib/skillPresentation.ts` 182–245 (`groupSkillsByRepo`,
  `filterAndSortSkills`, which are deep and pure); the pick modals, which have no search (#47).
- **Problem:** Two surfaces, the My Skills list and the assignment matrix, each re-compose the same pure functions
  with their own state cells and their own hand-built sort control. The pickers and a future palette need the same
  lens but have none. Singleton fold and flat mode (D8) would be a third copy.
- **Solution:** Add one lens module per list surface that owns query, sort, grouping (including a singleton-repo
  fold), view mode, persisted preferences and the selection's visible subset. It is instantiated by My Skills, the
  matrix, the pickers and the palette.
- **Benefits:**
  - *Leverage:* the lens has two callers today, which is a real seam, and D8 adds three more: #47, fold/flat mode,
    and ⌘K.
  - *Locality:* "which skills am I looking at" is answered in one place, and "Deploy all in this group" (#39) reads
    it.
  - *Tests:* one `renderHook` suite.
- **Dependency category:** in-process.

### 6. Deepen the action lifecycle: one Outcome executor, per-subject pending, presentation chosen by scale — **Strong**

- **Files:** `hooks/useStatusReporter.ts` 185–260 (18-member interface: `notify`, `notifyError`, `setError`,
  `setSuccessToastMessage`, `formatError`, `showActionErrors`, `showActionWarnings`, `setActionMessage`, …) and
  392–427 (`runAction`); `components/skills/LoadingOverlay.tsx` (a full-screen modal for any action);
  Outcome unpacked by hand at `useSkillLibrary.ts` 181–193, `useAddSkillFlow.ts` 140–143, 302–306 and 358–363,
  `useCandidatePick.ts` 220–227, and `ProjectsPage.tsx` 52–60 (which never calls `showActionWarnings`);
  `useProjectState.ts` `pendingCells` (the project world's own per-cell pending, bypassing `runAction`).
- **Problem:**
  - The reporter offers two failure dialects: `setError(formatError(err))` at 17 sites and `notifyError(err)` at 12.
  - Every caller unpacks `Outcome` (errors, warnings, toast, completion) itself. That is five copies, and the project
    copy silently drops warnings, so it only works because today's project folds emit none.
  - `runAction`'s single global `loading` flag raises a full-screen overlay even for a one-pill toggle, and `loading`
    is threaded through 18 pass sites into 19 components.
  - The project world avoided the overlay by building its own per-cell pending state. That leaves two lifecycles.
- **Solution:** Have the reporter run an `Outcome` itself, so callers hand it the fold's answer and nothing else. Make
  the action lifecycle know what the action is about (one skill, one cell, a batch), so a small action shows inline
  pending and a streamed batch shows the progress surface. The backend mutation guard already serialises, so a
  frontend "one at a time" rule can stay. Only its presentation moves.
- **Benefits:**
  - *Locality:* "how an action settles" lives in one module.
  - *Leverage:* D6 (crisp, near-invisible feedback on frequent actions) and D8 bulk progress both need it.
  - The five Outcome copies and the `loading` prop drill go away.
  - *Tests:* reporter tests cover Outcome execution once. Hook tests assert the fold call, not the toast.
- **Dependency category:** in-process.
- **ADR note:** consistent with ADR-0001. Classification and copy are untouched; only frontend delivery moves.

### 7. Give the project world an app lifetime and stop keeping a second skill catalog — **Worth exploring (Strong if the inspector gets project assignment)**

- **Files:** `components/projects/ProjectsPage.tsx` 41–48 (`useProjectState()` is called inside the page);
  `components/projects/useProjectState.ts` 152 and 238–255 (its own `ManagedSkill[]` loaded from `getManagedSkills`,
  with failure swallowed); `App.tsx` 359–365 (`ProjectsPage` is mounted only on its route).
- **Problem:** The project world is the one world hook that App does not compose. Because it mounts with its route,
  the selected project, loaded matrix and tool status are lost on every navigation and refetched on return. It holds
  a second copy of the skill catalog, which goes stale after an Update in My Skills until the page remounts. D8's
  per-row and inspector project assignment needs assignments from inside the skills world, where today they cannot
  be reached.
- **Solution:** Compose the project world in the binder like every other world, and make it read the library's
  catalog instead of fetching its own. `ProjectsPage` becomes a view of that world. Project assignment actions then
  become verbs in candidate 2's door.
- **Benefits:**
  - *Locality:* one skill catalog.
  - *Leverage:* the inspector's "assign to project" reuses the matrix's settled `ProjectViewDto` path.
  - *Tests:* the existing `useProjectState` suite is unaffected.
- **Dependency category:** in-process.
- **Cost:** a global project list load at startup, which is small.

### 8. One app-update module — **Strong (small)**

- **Files:** `hooks/useUpdateChecker.ts` (startup check, release notes, install); `components/skills/SettingsPage.tsx`
  8–16 and 76–140 (a second, independent `UpdateStatus` state machine with its own `check()` and
  `downloadAndInstall()`); `App.tsx` 541–609 (the update modal inlined in the binder).
- **Problem:** Two implementations of one state machine exist, and neither knows about the other. A version found in
  Settings does not reach the startup modal. A third consumer is planned: the passive title-bar affordance (D8).
- **Solution:** Add one update module (status, version, notes, check, install, ignore) that is read by Settings, the
  title bar and the notice.
- **Benefits:**
  - *Locality:* one updater lifecycle.
  - *Leverage:* three callers.
  - *Tests:* the inline Settings machine, untestable today because it lives in a component, becomes hook-tested.
  - About 60 lines leave the binder.
- **Dependency category:** true external (the Tauri updater plugin). Tests substitute a mock adapter, which ties in
  with candidate 9.

### 9. Native platform seam beside `invokeTauri` (folder picker, updater, Channel, webview) — **Worth exploring**

- **Files:** folder picker implemented 5×: `useAddSkillFlow.ts` 255–271, `useSkillLibrary.ts` 397–411,
  `useSettingsState.ts` 196–213, `components/projects/AddProjectModal.tsx` 35–43 (no `catch`, so a rejection goes
  unhandled), and `components/projects/ProjectsPage.tsx` 3 and 218 (a static import, and the only caller that accepts
  an array result). `Channel` is constructed in 3 hooks; the updater is used in 2 places (candidate 8); webview zoom
  is used 2× in `useSettingsState`.
- **Problem:** The typed `invokeTauri` seam covers commands only. Plugin calls cross into Tauri directly, each copy
  with its own title, error handling and `isTauri` guard. Any second adapter (a fixture, a test, a browser) has to
  intercept dynamic imports at 12 places.
- **Solution:** Put a small platform module next to `lib/tauri.ts` that owns `pickFolder`, progress-channel creation,
  the updater handle and zoom. Commands and plugins would then cross into Tauri at the same place.
- **Benefits:**
  - *Locality:* one folder-picker rule, fixing the unhandled rejection and the array result.
  - *Tests:* hooks mock one module instead of `@tauri-apps/plugin-dialog` plus `@tauri-apps/api/core`.
- **Dependency category:** ports and adapters. The Tauri adapter already exists and ticket 04's fixture door is the
  second, so the seam is real.
- **Scope note:** this touches ticket 04's file. It is flagged for the grill, not asserted as missing work.

### 10. Add flow: expose intents, keep form state internal — **Worth exploring**

- **Files:** `hooks/useAddSkillFlow.ts` 503–533 (a 30-member interface: `localPath`/`setLocalPath`,
  `localName`/`setLocalName`, `gitUrl`/`setGitUrl`, `gitName`/`setGitName` and `addModalTab`/`setAddModalTab` are
  raw state pairs) and 484–501 (Explore install: `setGitUrl` plus a ref plus a counter plus an effect with
  `exhaustive-deps` disabled, so that `handleCreateGit` runs after the state lands); `App.tsx` 389–430 (18 props into
  `AddSkillModal`, 11 into `ImportModal`).
- **Problem:** The hook exposes its form as ten state/setter members, which is the interface-as-wide-as-body smell.
  The one real cross-world intent ("install this repo URL, preselect this skill") is done through a render-cycle
  trigger instead of a call.
- **Solution:** Make the Add flow's interface its intents: open with a source, install a selection, review the
  import. The form draft becomes internal state of the flow, or of the modal.
- **Benefits:**
  - *Leverage:* the sticky install summary, the #46 `SKILL.md`-gated scan and #47 picker search land behind the flow
    and do not add more props.
  - The trigger effect goes away.
  - *Tests:* tests drive intents instead of setters.
- **Dependency category:** in-process.

### 11. Split the Tool catalog out of sync orchestration — **Worth exploring**

- **Files:** `hooks/useSyncOrchestration.ts` 118–240 and 435–457 (tool status, labels, installedness, shared-dir map, the
  effective target set, the Add modal's `syncTargets`) and 459–494 (a 30-member interface; `isInstalled` and
  `installedToolIds` have no caller outside the hook).
- **Problem:** One hook mixes three concepts:
  - the **Tool catalog** (a CONTEXT term);
  - two different target sets: the **Effective global target set** and the Add modal's per-install `syncTargets`,
    whose defaults and reset rules differ;
  - the sync seam, with its overwrite and shared-dir asks.

  `ProjectsPage` gets only `toolLabelById` from it, and the project world loads its own tool status.
- **Solution:** Make the Tool catalog its own module, read by sync, library, add flow, projects and the "Synced N/M"
  strip. Sync orchestration keeps target sets and the seam.
- **Benefits:**
  - *Locality:* the CONTEXT term maps to one module.
  - *Leverage:* #52 enable/disable and the per-tool strip read the catalog directly.
  - Two unused members are deleted.
- **Dependency category:** in-process.

### 12. Detail file browser as a module fed a skill document, not a `ManagedSkill` — **Speculative**

- **Files:** `components/skills/SkillDetailView.tsx` 56–175 (`formatSize`, a 40-entry `EXT_LANG` table, `getLang`,
  `buildTree`, all pure and all untested because they live in a component file) and 371–445 (two fetch effects;
  `invokeTauri` passed in as a prop); `hooks/useExploreState.ts` 124–150 (a forged `ManagedSkill` for the Explore
  preview).
- **Problem:** The pure tree and language logic is locked where AGENTS forbids tests. The view's interface demands a
  whole Managed skill when it needs a name, a description, a source line and a path.
- **Solution:** Move the file browser (tree, active file, content load) into a hook plus pure module. The detail and
  the inspector take a small skill-document value.
- **Benefits:**
  - *Tests:* `buildTree` and `getLang` become testable.
  - The forged `ManagedSkill` goes away.
- **Why Speculative:** it has one caller, and the inspector may not show files at all.

### 13. The `App.tsx` binder's growth (evaluated) — **Speculative as a standalone candidate; it is a symptom**

- **Files:** `App.tsx` 1–613: 245–609 is JSX, about 60% of the file and mostly one-to-one prop threading; 19
  `t={t}`; `SettingsPage` receives 15 members of `useSettingsState` one-to-one (336–358).
- **Finding:** The binder principle in AGENTS.md is sound, and its real composition is small:
  - `handleSyncAllNewTools` (207–235);
  - `handleExploreInstallFromDetail` (190–198);
  - view switching.

  The deletion test finds that the prop threading is a pass-through, so deleting it concentrates nothing. The growth
  comes from candidates 1 (view and modal state), 6 (reporter destructuring, `loading`), 8 (the inlined update modal)
  and 10 (form props) leaking into the binder. Fix those and App shrinks without a candidate of its own. A separate
  "split App" refactor would only move lines around.
- **Side note:** `t` threading (27 components take `t: TFunction`; 3 call `useTranslation`) is a convention choice,
  not depth. Only change it if the primitives work (candidate 4) wants it.

## Evaluated and not candidates

- **`lib/reportOutcome.ts` (639 lines):** deep and well tested (a 1,218-line suite). The friction is at its consumers
  (candidate 6), not inside it.
- **`lib/skillPresentation.ts` (417 lines):** deep and pure. It is where candidates 3 and 5 grow.
- **`useOverwriteConfirmation` / `useSharedDirConfirmation`:** model deep modules (`request()` resolves to a boolean;
  3–5 member interfaces). Keep this "ask" pattern when shadcn Dialog replaces `Modal`.
- **`commandError.ts`:** ADR-0001's single copy module, which works as intended.
- **`SettingsPage` (449 lines) and `AssignmentMatrix` (449 lines):** their size is markup, which candidate 4 absorbs.
  Beyond that, `SettingsPage` needs only candidate 8, and `AssignmentMatrix` needs only candidate 5.

## Top recommendation

Start with **candidate 1 (surface state)**, grilled together with **candidate 2 (the action set)**:

- The round-19 world (sidebar shell, inspector beside the list, bulk rail, ⌘K) is defined by those two modules.
- Every D8 feature is a caller of one or both.
- Neither has a home today.

Candidate 3 is the cheapest high-leverage win and can run in parallel, because it is pure. Candidate 4 is already
sequenced first by D9, and it does not conflict with candidates 1 and 2: primitives are how surfaces look, and
candidates 1 and 2 decide what is on screen and what it can do.

## Open questions for the grill (ticket 09)

1. Is the Tracker List detail an **inspector beside the list** (a selection), or still a **route**? This decides the
   shape of candidate 1.
2. Is there **one selection** shared by My Skills, the matrix and the palette, or one per surface?
3. Should surface state be **addressable** (a hash route), for example so the ticket-04 fixture mode and critiques
   can deep-link, or in memory only?
4. Should primitives get **role/ARIA tests** despite the "no component tests" rule, given that D5 is a hard gate?
5. Should the **prose layer** (Markdown plus syntax highlighting) be exempt from the `App.css`-to-zero CI guard?
6. Should the frontend keep its **one-action-at-a-time** rule (so candidate 6 changes only the presentation), or allow
   concurrent inline actions, which the backend guard would serialise anyway?
7. Candidate 7: does **project assignment from My Skills** ship in round 19 (D8 says yes)? If it does, candidate 7
   becomes Strong.
8. Candidate 9 overlaps ticket 04's `lib/tauri.ts` door. Should the **platform seam** fold into that door or sit
   beside it?
