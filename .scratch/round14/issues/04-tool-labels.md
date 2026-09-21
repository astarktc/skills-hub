# 04 Kept-project toast shows raw tool keys

Status: ready-for-agent
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
