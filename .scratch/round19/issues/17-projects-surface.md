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

(child fills in)

## Comments

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
