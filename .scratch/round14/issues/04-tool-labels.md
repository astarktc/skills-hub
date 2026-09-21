# 04 Kept-project toast shows raw tool keys

Status: done — 28ea72c
Lane: C
Source: BACKLOG #33 (from `archive/round13/issues/09:comments`)

`src/components/projects/ProjectsPage.tsx:38–43` builds `toolLabelById` from `state.toolStatus?.tools`, which
`useProjectState.loadToolStatus` (`:402`) fills only on the add-project / Configure Tools flow. A Remove Project
reached on a fresh launch folds `projectRemovalOutcome` with an empty map, so the error title
`errors.projectRemovalFailedTitle` interpolates `claude` instead of `Claude Code`.

Fix per round-14 D4 — at the binder, not across worlds: `useSyncOrchestration` already exports `toolLabelById`
(`src/hooks/useSyncOrchestration.ts:91`, i18n-resolved via `t("tools.<key>", { defaultValue })`, loaded at startup).
`src/App.tsx` passes `sync.toolLabelById` to `ProjectsPage` as a prop; `ProjectsPage` uses that for both
`projectRemovalOutcome` calls (`:81`, `:102`) and drops its own `useMemo`. `state.toolStatus` stays for the tool-config
modal (`:287`). Type the prop as `Record<string, string>`; **no** runtime import of the sync hook from the projects
world (a `Pick<>` of its return type is fine for the prop type if you want it named).

Tests: hook- and pure-level only (no JSX tests). `reportOutcome.test.ts` already covers the fold with a label map;
add one case proving a key absent from the map falls back to the raw key (so the contract is explicit), if not
already there. `npm run lint && npm run test && npm run build`.

## Done when

Remove Project / Configure Tools toasts name tools by label regardless of whether the project tool-status has loaded;
projects world imports nothing from `src/hooks/` at runtime; gate green.

## Comments

### 2026-09-21 — lane C (Fable)

Done per D4, at the binder.

- `src/components/projects/ProjectsPage.tsx`: new prop `toolLabelById: Record<string, string>` (doc comment cites D4);
  the `useMemo` over `state.toolStatus?.tools` is gone and `useMemo` is no longer imported. Both `projectRemovalOutcome`
  calls (`handleToolConfigConfirm`, `handleRemoveProject`) read the prop; their dep arrays already listed
  `toolLabelById`, so they are unchanged. `state.toolStatus` still feeds `ToolConfigModal` only.
- `src/App.tsx`: `<ProjectsPage … toolLabelById={sync.toolLabelById} />`.
- No runtime import of any world hook from the projects world: `ProjectsPage`'s `../../hooks/useStatusReporter` import
  is `import type` (pre-existing). Note `AssignmentMatrix.tsx` has a pre-existing runtime import of
  `usePersistedPreference` — a building-block hook, not a world hook; left alone.
- `src/lib/reportOutcome.test.ts`: new case in `describe("removalOutcome")` —
  `"a tool key absent from the label map falls back to the raw key"` — proves an empty map (the fresh-launch case) and a
  populated map lacking the key both surface the raw `claude`, for `removeProject` and `configureTools`, and that the
  same report labels `CLAUDE` once the key is present. The fold itself (`labelFor` at `reportOutcome.ts:42`) needed no
  change.

Evidence: `npm run lint` clean; `npm run test` 15 files / **358 passed** (reportOutcome.test.ts 55, was 54);
`npm run build` (typescript-7) ✓. `lens_diagnostics mode=all`: no errors, only pre-existing style warnings.
Not smoke-tested in `tauri:dev` (forbidden for this lane) — the operator's 1.2.14 smoke should trigger Remove Project on
a fresh launch and confirm the kept-target toast title reads `Claude Code`, not `claude`.

- 2026-09-20 (parent) — closed `done — 28ea72c`.
