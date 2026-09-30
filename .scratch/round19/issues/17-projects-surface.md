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
