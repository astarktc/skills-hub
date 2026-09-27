# 04 — Fixture-backed browser mode (D4)

Status: ready-for-agent
Spec: `.scratch/round18/spec.md` — D4, ticket 04.

## Work

- `src/lib/tauri.ts` is the only backend door (`invokeTauri(name, ...args)` typed over the generated `commands`
  table). Add `src/fixtures/` with a `FixtureBackend` implementing **every** command name as a typed handler over an
  in-memory catalog (`satisfies Record<CommandName, …>` so a new command fails `npm run build`). Mutations mutate the
  catalog and answer the real report/view shapes (`{ report, skills }`, `ProjectViewDto`, `RemovalReport`, …);
  the five `Channel`-streaming commands emit simulated progress events on a timer. `TARGET_EXISTS` rows, failures
  and skips must be producible so the overwrite ask and report folds fire.
- Scenarios via `?scenario=rich|empty|first-run|failures` (default `rich`), built from `.scratch/round18/prototypes/
  BRIEF.md`'s fixture roster (60 skills, 7 tools, 3 projects) so prototypes and app show the same library.
- Gate: `import.meta.env.VITE_MOCK_BACKEND === "1"` → dynamic-import the fixture module; production build must
  contain none of it (prove by grepping `dist/` for a fixture-only string). `@tauri-apps/api/core` `Channel`
  needs a shim under the gate (`new Channel()` throws outside Tauri — check).
- Scripts: `npm run dev:fixture` (Vite on a fixed port, e.g. 5175, `strictPort`); document in AGENTS.md Commands.
- Verify with the t3 `preview_*` tools: open `http://localhost:5175/?scenario=rich`, walk Add → pick → sync →
  overwrite ask, Refresh all with progress, Projects assign, Settings; capture `.scratch/round18/evidence/app/
  {myskills,detail,add,projects,settings,explore,import}-{dark,light}.png` (local-only) and report their paths.
- Tests: fixture module has a vitest that every `CommandName` has a handler and that `rich` satisfies each DTO's
  invariants at the type level. Existing hook tests keep mocking `src/lib/tauri.ts` — do not change them.
