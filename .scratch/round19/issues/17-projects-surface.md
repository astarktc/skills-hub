# 17 — Projects: Tool labels, shape + word cell states + legend, keyboard listbox, header collapse, coexistence, Sync All skips missing folders, Assigned/Issues chips, `reconciled:false`

Status: ready-for-agent
Blocked by: 10, 13, 16
Spec: `.scratch/round19/spec.md` — D17, D23, D26 (grill Q17, Q20, Q25). Sources: `flows.md` Flow 4; `critique-astra`
F03, F04, F15; `critique-opus` P1-6; `audit.md` Projects rows (P0: pointer-only project selection).

## Work

- **Project selection** is a real keyboard model: a list of named selection buttons (or a correctly implemented
  single-selection listbox with roving focus and `aria-activedescendant`) with distinct selected vs focused states;
  Configure / Remove live in a per-project overflow, separate from selection. Prove: select an unselected project and
  change an assignment without a pointer.
- **Matrix**: column headers are Tool **labels** (+ "not detected" where the project's configured Tool is not
  installed); cell = assignment checkbox (hit area = the cell, ≥24 px) **plus** a status mark with a word on
  focus/hover and in the accessible name: synced · stale ("copy differs from the skill — Sync to fix") · missing ·
  pending · failed (with the error) · disabled; a **legend**; per-cell **Sync this** on stale/missing/failed.
  **Coexistence** (D17): a cell with a global target is enabled and marked "also global"; bulk and toggle obey the same
  rule. Bulk verbs are named ("Assign to all configured Tools" / "Unassign from all"). Chips **Assigned · Issues**
  through the list lens (ticket 08) — one lens instance per matrix, with the duplicated sort control gone.
- **Header** (D23): at the breakpoint the project list becomes a header select; the project name and path never
  truncate before the toolbar collapses into an overflow; "Last synced" stays one line.
- **Sync All** skips a project whose folder is missing with one line ("archived-app: folder not found — Update path
  or Remove") instead of one failure per assignment; `reconciled: false` renders a quiet "statuses may be out of
  date — Refresh view" line.
- Remove Project dialog: says what is removed, what stays (the project's own files), how kept errors will show.
- Assignment from the inspector (ticket 12) and the matrix share the door's verbs; after a mutation the view applies
  the returned `ProjectViewDto` as today.
- Migration: `ProjectsPage`, `AssignmentMatrix`, `ProjectList`, the three project modals join `migrated`; `matrix-*`,
  `project-*` rules leave `App.css`.

## Gate (D31)

Astra audit on `failures`: keyboard-only select `archived-app` → `monorepo` → toggle a cell → All Tools → Sync
Project; every cell state readable in grayscale and by name; 960×640 collapse with the project name intact; Sync
All with a missing folder yields one line; contrast; `npm run check`.

## Result

Implemented 2026-10-01 (child, Opus 5.5). Nothing committed. Evidence: `.scratch/round19/evidence/17/` (drivers
`gate.ts` · `states.ts` · `dialogs.ts` · `a11y.ts` · `longname.ts` over agent-browser 0.31.1 with trusted CDP keys
through `keys-cdp.ts`; JSON transcripts, screenshots, `check.log`, `bindings.diff`).

### Shipped

- **Selection model — named selection buttons** (`ProjectList.tsx`): one `<button data-project-select>` per project,
  named by the project (`aria-labelledby` its name, path + counts as `aria-describedby`); the selected one carries
  `aria-current="true"`, a `raised` ground and a 3 px iris edge, the focused one the inset focus ring — never the same
  cue. ↑ / ↓ / Home / End move focus between projects; Enter / Space select through the surface (`focus` →
  `selectProject`). Each project's own actions (Update path… on a missing folder, Project settings…, Remove project…)
  live in its `DropdownMenu` overflow, separate from selection.
- **Header (D23)** — `ProjectHeader.tsx` (new): name `h2` + path (wrapping only after `/`, `PathText`) + "Last synced"
  (`whitespace-nowrap`) never truncate; the actions take the room beside them. ≥ 1200: Configure Tools… · Sync All ·
  **Sync Project** (the one primary) · overflow. < 1200 (`useMediaQuery(SHELL_WIDE_QUERY)`): the list becomes the
  header's native `Select` (after the "Projects" `h1`, with Add project) and the toolbar collapses to Sync Project +
  the overflow (Configure Tools…, Sync All, the project's actions). A resize that drops focus moves it to the select /
  the selected project.
- **Matrix** (`AssignmentMatrix.tsx`, rewritten): Tool **labels** from the catalog in the column headers (+ "not
  detected" from `projectToolStatus.installed`, loaded when the area opens); cell = a `<label>` spanning the cell
  (hit area = the 36 px row × column) with the Checkbox, the Synced strip's mark (`StripMarkIcon`), ⊕ **also global**
  (D17 — enabled; toggle and bulk obey the same rule) and the state's word on hover/focus (`visibility`); the checkbox's
  name carries skill · Tool · state · meaning · "also global". **Sync this** on stale / missing / failed (pending while
  it runs; once the cell is healthy its focus goes to the cell's checkbox). A **legend** from the same vocabulary. One
  list lens: search + chips **All · Assigned · Issues** (with counts) + Grouped / Flat; the duplicated sort control is
  gone (`LensSortControl.tsx` deleted — it had no other user). Row verbs **Assign to all** ("Assign to all configured
  Tools — {skill}") / **Unassign from all** are the door's `assignToProject` / `unassignFromProject` (the inspector's
  verbs); both stay mounted and are refused with the reason when they would change nothing, so focus never drops.
- **One pure fold** — `src/lib/projectPresentation.ts` (+ test, 17 cases): `matrixCell` (state, mark, word, detail,
  name, `alsoGlobal` from live global targets via `isLiveTarget`, `repairs: ["sync"]`, in-flight), `matrixColumns`,
  `matrixFilters` (Assigned / Issues), `matrixLegend`. `syncStatus.ts` (the legacy class maps) is deleted — the fold's
  `Record<SyncStatus, …>` keeps the compiler's exhaustiveness.
- **Sync All skips a missing folder with one line — decided in core.** `resync_all_projects` reported per assignment
  (and, worse, synced *into* the missing path: `sync_engine` creates parents, so a moved project would grow a stray Tool
  tree). Now `ProjectSyncReport` carries `skipped_projects: Vec<ProjectSkipped { project_id, reason:
  ProjectSkipReason::FolderMissing }>` (specta-derived; the only bindings change besides the new command), produced by
  the one seam `resync_assignments_unlocked` — Sync Project, Sync All and Sync this alike — before any row is touched;
  `project_ops::project_folder_exists` is the one rule behind both `path_exists` and the skip. `projectSyncOutcome`
  folds it into **one** warning entry "archived-app: folder not found — Update path or Remove" (`projectInTitle`, so
  the panel does not prefix it) counted once as skipped. Rust: `resync_all_skips_a_project_whose_folder_is_missing`,
  `resync_assignment_resyncs_one_row`, the wire-shape pin.
- **Sync this** — new command `resync_project_assignment(projectId, skillId, tool) → ResyncProjectResultDto { view,
  report }` (`project_sync::resync_assignment`, serialized, typed `NotFound` for an absent row); the project world's
  `syncAssignment` applies the view, settles a `TARGET_EXISTS` through the one ask, re-reads only on throw / a resolved
  retry. `bulkAssign` / `bulkUnassign` left the world (the door's `assignToProject` / `unassignFromProject` are the
  row verbs now) with their tests ported.
- **`reconciled: false`** renders a quiet `role="status"` line "Statuses may be out of date — Refresh view"; Refresh
  view is a read (`refreshView`, outside the lifecycle) and hands its focus to the project heading once the line
  goes. The fixture seeds it: `state.unreconciledViews` (quartermaster's first view, once).
- **Folds on the count line** (13's comment): bulk assign "2 assigned · 1 failed · 1 skipped" (a disabled skill's
  "not synced" as the detail), resync "7 synced · 3 failed · 2 skipped", bulk unassign "2 unassigned · 1 failed", tool
  configuration "1 removed · 1 failed", remove project "Project kept · 5 removed · 1 failed". **Behaviour change:** a
  disabled skip now tones the count line `skipped` (D10 precedence, as the global batch already did) instead of the
  pre-D10 success tone. Removing a project now **closes its dialog either way** — the kept row shows in the matrix as
  the dialog said it would. 33 project/disabledSkips i18n keys the old sentences used are deleted.
- **Dialogs on primitives** — Add project (Field + Input + **Browse…** in one row, duplicate refused under the field,
  a "Keep deployed skills out of git" FieldSet), Project settings (title now matches its menu item; the shell stays
  mounted, body keyed per project, so focus returns), Remove project (what goes · the path · what stays: the project's
  own files and the library · how a kept copy shows — ADR-0002; Cancel first; Remove `solid danger` pends; focus falls
  back to the area heading when the project is gone), and `ToolConfigModal` gains `progress`. All four show the header
  progress at their foot; `App.tsx`'s `progressInDialog` includes them.
- **Primitive:** Checkbox `focusableWhenDisabled` (Switch's model: read-only + `aria-disabled`, keeps Tab stop and
  focus) for a cell disabled by its own running action; `checkbox.test.tsx` pins it. DESIGN.md: Checkbox, Motion
  (`.cell-spinner` / shimmer gone; the cell spinner joins the spinners row), a **Projects** section with the cell-state
  table and measured ratios.
- **Fixture parity** — `resyncAssignments` (folder-missing skip), `resyncProjectAssignment`, `unreconciledViews`;
  the round-18 "locked project artifact" is walkable: monorepo's `.agents/skills` (`unwritableProjectTools`) keeps
  release-train's row on Remove (fixture test added).
- **Styling (ADR-0006)** — `matrix-*`, `project-*`, `projects-*`, the `form` family (`.form-group`, `.label`, `.input`,
  `.input-row`, `.input-action`, `.field-error`; `.helper-text` stays for `InvocationModeModal`), `delete-*`,
  `.btn-danger-solid`, `.btn-xs`, `gitignore-*`, `.cell-*`, `.skeleton-row`, `group-by-repo-*`, `.repo-group-icon`,
  `.sort-btn` / `.sort-label`, the `shimmer` keyframes and their orphan comments left `App.css`: **1676 → 1108**
  counted lines (raw 1974 → 1297), `appCssCeiling` lowered; `ProjectsPage`, `ProjectList`, `ProjectHeader`,
  `AssignmentMatrix`, `AddProjectModal`, `EditProjectModal`, `RemoveProjectModal` joined `migrated` (36). Motion stays
  instant (no transitions added); no new `.css`; i18n appended at the end of `en` (`projectsSurface`).

### Decisions

- **Selection = named buttons**, not a listbox: each project needs its own overflow beside it, which a listbox option
  cannot contain; buttons + `aria-current` + arrow-key focus movement give the keyboard model without
  `aria-activedescendant` bookkeeping.
- **The skip's home is core** (above): a frontend collapse of N failures would have kept the stray-folder write.
- **R1 deferred** — the matrix does not adopt the triggerless anchor popover (word on hover/focus is CSS; no cell
  popover), so the focus-guard Tab stop fix stays on 19b's list (ticket 23's palette also anchors). Comment added there.
- **No matrix multi-select / keymap** this round: the ticket's flows are cell, row and project verbs; the inspector and
  the bulk rail already assign a selection to a project through the door.

### Verification (fresh, 2026-10-01)

- **Gate transcript** (keyboard only, trusted CDP keys; `gate-*.json`): dark 1280 — Projects → `archived-app` (Enter)
  → ↑ to `monorepo` (`selected-vs-focused-dark-1280.png`: archived-app `aria-current` with the iris edge, monorepo the
  ring) → Enter → Tab to "alias-skill in Claude Code: not assigned, also global" → Space → "…: synced — the copy matches
  the skill, also global" → **Assign to all configured Tools — alias-skill** → row `synced · failed (locked
  .agents/skills) · synced`, focus stays on the verb → Shift+Tab to **Sync Project** → "7 synced · 4 failed". 49 steps,
  focus never `BODY`; **getProjectView reads during the mutations: 0 / 0 / 0** (stack-tagged recorder). Light 960:
  the same through the header select (type-ahead `a` → archived-app, `m` → monorepo), 37 steps, 0 / 0 / 0, never BODY.
- **Cell states by name** (`states-*.json`): synced "release-train in .agents/skills: synced — the copy matches the
  skill"; stale "design-tokens in Claude Code: stale — copy differs from the skill — Sync to fix, also global"; missing
  "docx in Claude Code: missing — the deployed copy is gone — Sync to fix, also global"; pending "release-train in
  Claude Code: pending — assigned, not synced yet, also global"; failed "tdd in .agents/skills: failed — Permission
  denied (os error 13): …/.agents/skills/tdd — Sync to retry"; disabled "retired-playbook in Claude Code: disabled — the
  skill is off everywhere — Enable it to sync". Grayscale: `matrix-grayscale-{dark-1280,light-960}.png` (focused stale
  cell shows its word).
- **Sync this** on design-tokens/Claude Code: → synced, toast "Synced", focus on the cell's checkbox, 0 reads.
  **Chips**: Issues → docx · frontend-design · tdd; Assigned → 5 rows. **Sync All**: toast "7 synced · 3 failed ·
  2 skipped"; the panel's run holds **one** line "archived-app: folder not found — Update path or Remove" (+ "Nothing in
  it was synced; its assignments are unchanged."), archived-app's rows untouched (`missing, missing`).
  **Reconcile line** on quartermaster → Refresh view → line gone, focus on the `h2`.
- **Remove monorepo** (`dialogs-*.json`, `remove-*.png`): the dialog's copy as above, Cancel focused; Remove → foot
  progress "Removing project" → closes; monorepo stays listed with release-train/.agents/skills "failed — Permission
  denied …" (ADR-0002), toast "Project kept · 5 removed · 1 failed", focus back on "More actions for monorepo". Foot
  progress also on Project settings (Save), Configure Tools (Save Tools), Add project (Register). Both themes.
- **960 × 640** (`a11y-light-960.json`, `long-name-light-960.png`): select in the header, no list; inline actions
  Add Project · Sync Project · More actions; the overflow holds Configure Tools… · Sync All · Project settings… ·
  Remove project…; a 64-character name and a 100-character path render whole (name 1 line, path wraps at `/`),
  "Last synced" 1 line, no horizontal scroll.
- **axe 4.10.3, 8 runs** (matrix with a focused stale cell · overflow menu open · Remove dialog · Add dialog × dark 1280
  / light 960): 0 violations except `region` (best-practice, moderate) on the open menu's portal — pre-existing in the
  DropdownMenu primitive (`probe6.ts` reproduces it on My Skills' row menu); incompletes are Base UI's focus guards
  (`aria-hidden-focus`). **Contrast** (`a11y-*.json`): text min 4.68 dark (`primary-foreground` on `primary`) / 5.37
  light, nothing under 4.5; marks dark / light — synced 8.31 / 8.16, stale 10.41 / 5.62, missing & failed 6.91 / 5.50,
  pending 3.90 / 3.78, disabled 6.12 / 6.09, also global 8.31 / 8.16; words 8.31 / 8.16; checkbox edge 3.90 / 3.78.
- **Gate**: `npm run version:check` → "Version OK (1.2.18)"; `npm run check` → EXIT 0: lint clean, styles:check ok
  (1108, 36 migrated), vitest 45 files / 973 tests, build ok, fmt + clippy clean, cargo 809 passed. Bindings diff:
  `ProjectSyncReport.skipped_projects` + `ProjectSkipped` + `ProjectSkipReason` + the `resyncProjectAssignment`
  command (`bindings.diff`).

### Open concerns

- DropdownMenu's portal fails axe `region` while open (all menus; 19b primitive fix — a landmark-aware portal
  container or an accepted exception).
- The matrix is a long Tab walk (3 cells + 2 row verbs per skill); a grid keymap (arrows / j k) on the
  `useSkillListKeymap` pattern would help large libraries — not in this ticket's flows.
- Toggle / bulk assign into a missing folder are refused only in the UI (disabled); core still accepts them.
- The native header select changes on type-ahead; on macOS arrows open the platform menu (by design of `Select`).

## Comments

- 2026-10-01 — independent D31 audit of `c6a47c0`: **FIX-THEN-PASS**, report `.scratch/round19/review/ticket-17-audit.md`, fresh evidence `evidence/17-audit/`. **A1 P1:** first assignment (bulk/toggle), Enable and collision retry can recreate a missing project root; an isolated harness importing unchanged Rust core proves all four, and the inspector's Assign picker still offers archived-app. The fixture currently masks production behavior by failing instead. Enforce the root-presence rule in core before admission/materialization, align the picker/fixture, and recheck the exact scope in Acceptance. Normal keyboard gates pass in dark 1280/light 960 (49/37 steps, no BODY), all four mutation-returned-view paths have zero follow-up reads, states/contrast/reconcile/kept removal/dialog progress pass, 14 full axe runs have only the inherited menu `region` exception, and version + full check + cargo --all pass (973 frontend / 809 Rust). **P2:** 127 Tabs first-to-last matrix cell (no grid required for this gate); an Issues-filter repair falls back to MAIN when its row disappears. Coordinate those with 19b/ticket 23. The named menu portal's landmark warning is a narrow best-practice false positive, not grounds to add a bogus landmark; accept/document the exception. No application source edited or commits made.

- 2026-09-27 — round-18 extract: the fixture's unwritable dirs affect writes only, so ADR-0002's kept row on project removal is not reproducible (`round18/review/flows.md` § Fixture gaps) — seed one locked project artifact so the Remove Project kept-row path is walkable.

- 2026-09-30 — from ticket 12: the inspector's **Assign to project…** assigns to *every* configured Tool of the
  chosen project (`useProjectState.assignToProject` → `bulk_assign_skill`); its picker shows each project's Tools and
  what assigning adds, and leaves per-Tool choice to the matrix (D26). New read `readSkillPlacement(skillId)` reads
  every project's view (no apply) for the inspector's Projects section — N `get_project_view` calls; a per-skill
  backend read would be cheaper if the Projects surface wants the same "where is this skill" view. Project Tool
  labels come from `catalog.loadProjectToolStatus`, which App now also loads when a skill is focused.

- 2026-09-30 — from ticket 13: (1) the project folds (`bulkUnassign`, project removal, `projects.bulkUnassign*`,
  `projects.removeKept`) still toast their old sentences — move them to the count line with the matrix
  (09's handoff). (2) `RemoveProjectModal` is the last user of the legacy `delete-title` / `delete-desc` /
  `delete-warning` / `btn-danger-solid` classes (the skill Delete dialog migrated): migrating it retires those
  `App.css` rules and lowers the ceiling. (3) The matrix's own selection is surface state's `selection.matrix`; the
  keymap pattern (`useSkillListKeymap`: one handler, off while typing or under a layer, clearing from inside a rail
  rescues focus) is reusable there. (4) Assign to project… over a selection (`assignProject` with `skillIds`) assigns
  every skill to all the project's configured Tools through the project world's `assignToProject(projectId, ids)`.

- 2026-09-30 — from ticket 14: the Projects area shows the header progress as a strip above the area
  (`placement="bar"`); resync / resync-all / remove / register set `activity` ("Syncing projects", "Removing
  project", "Adding project"). The project dialogs (add / edit / remove / tool config) get no dialog-foot progress
  yet — add `progress` like the skills dialogs. `projectSyncOutcome` folds an unanswered `TARGET_EXISTS` as "N folders
  to review" (conflict tone, `review` entry with the project in its subject; no overwrite repair from the panel —
  answered from the matrix). The Notifications panel names a project row "in {project}:" from the labels App passes.

- 2026-09-30 — from ticket 15: `Switch` gained `focusableWhenDisabled` (Base UI 1.8's Switch has none; the primitive holds focus
  read-only with `aria-disabled`) — use it for any matrix / project toggle disabled by its own running action. A focused
  `aria-disabled` control draws at 75 % (ring 4.42 dark / 3.49 light). The legacy `.project-item .remove-btn` hover
  reveal, `.matrix-cell` background fade and the matrix tooltip fade lost their `transition` (instant now; motion table
  in DESIGN.md § Motion) — keep them instant when migrating. `.cell-spinner` (0.6 s) and the list skeleton shimmer
  (1.5 s) remain the only legacy loops; the base layer stops them under reduced motion.

- 2026-09-30 — from ticket 16: the `form` family left only partly — `.form-group`, `.label`, `.input`, `.input-row`,
  `.input-action` (and `.helper-text`, also used by `InvocationModeModal`) are still referenced by
  `projects/AddProjectModal.tsx` and `projects/EditProjectModal.tsx` only, so they stay in `App.css` until those dialogs
  migrate here (Field / Input / Button; the Add dialog's local-folder field — Input + "Browse…" Button in one row — is
  the pattern). Lower `appCssCeiling` by what you delete. The one candidate picker (`skills/add/CandidatePicker.tsx`,
  `mode="multiple" | "single"`, search + visible-and-checked rule) is available if a project flow ever lists skills to
  choose from.

- 2026-10-01 — from ticket 16's re-check (R1, P2, inherited — not 16's): the **triggerless `PopoverContent anchor`** (SkillRow's inline Enable confirmation, `SkillRow.tsx` ~:272) exposes a hidden `aria-hidden` focus-guard Tab stop after its last control and stays open (and registered on the layer stack) when focus Tabs out of it; Escape and pointer dismissal recover. Fix in the primitive: give the anchor API a real focus-manager return position (not positioning alone) so forward Tab from the last control reaches the next page control and reverse Tab the prior one, the popup dismisses, zero layer records remain, no focusable hidden guard; primitive + browser regression. Do it here if the Projects matrix adopts the anchor popover for its cells; otherwise leave it on this list for 19b (ticket 23's palette also anchors).

- 2026-10-01 — implementation (child): R1 **deferred** — the matrix uses no anchor popover (the state word is CSS on
  hover/focus), so the triggerless-popover focus-guard fix stays for 19b (noted on ticket 23). The Sync All skip lives
  in core (`ProjectSyncReport.skipped_projects`), not a frontend collapse — the per-assignment path synced into the
  missing folder.
