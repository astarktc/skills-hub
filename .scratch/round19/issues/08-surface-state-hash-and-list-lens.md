# 08 — Surface-state module (area · focused subject · selection · dialog), hash addressing, list lens

Status: ready-for-agent
Blocked by: —
Spec: `.scratch/round19/spec.md` — D3, D4, D5 (grill Q3–Q5); `round18/review/architecture.md` candidates 1 and 5.
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

(child fills in)
