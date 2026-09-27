# 04 — Fixture-backed browser mode (D4)

Status: implemented
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

## Result

Implemented (not committed). `npm run dev:fixture` serves the whole app on `http://localhost:5175/?scenario=…`
against an in-memory library; every surface was walked and captured in both themes with the t3 `preview_*` tools.

### What changed

- **`src/fixtures/`** (new, dev-only):
  - `backend.ts` — `createFixtureBackend(state)`: one typed handler per generated command,
    `handlers = { … } satisfies FixtureHandlers` where `FixtureHandlers = { [K in CommandName]: (...Parameters<Commands[K]>) => Promise<Awaited<ReturnType<Commands[K]>>> }`
    — a missing or misspelled command fails `npm run build`. Stateful: sync/unsync/delete/refresh/update/re-point/
    detach/import/Edit and every project mutation mutate the catalog and answer the real shapes (`{ report, skills }`,
    `ProjectViewDto`, `RemovalReport`, `ToggleAssignmentResultDto`, …). Mirrors the real rules: shared-skills-dir
    dedupe (Amp/Kimi), installedness skip (`TOOL_NOT_INSTALLED`), `TOOL_NOT_WRITABLE` as *skipped*, the overwrite rule
    (`TARGET_EXISTS` unless same-content/override), ADR-0002 (failed removal keeps the row with `error`),
    Refresh-all skipping Unlocatable skills, Restore rebuilding a central-missing copy, Propagation (links skip,
    copies/errored rows re-materialise), auto-sync re-assert, onboarding plan scope + import take-over/removal of
    byte-identical originals, project sync-status precedence fold. The four Channel commands
    (`syncSkillsToTools`, `refreshManagedSkills`, `updateManagedSkill`, `importOnboardingSelection`) tick progress
    on a timer (both Refresh phases, both import phases). `cancelCurrentOperation` cancels a running Refresh (`CANCELLED`).
  - `model.ts` — `FixtureState` (wire values stored *as* the generated DTO types + bytes and fault injection beside
    them), path/registry helpers, `makeSkill`/`makeProject` builders, generated `SKILL.md` + files.
  - `scenarios.ts` — `rich` (the BRIEF roster: 60 skills = 28 mattpocock + 9 anthropics + 12 superpowers + 6 vercel +
    5 individual sources, 7 detected Tools, 3 projects), `empty`, `first-run` (no selection, all 7 Tools newly
    detected once, 12-group onboarding plan with conflicts and links), `failures` (see below).
  - `registry.ts` — snapshot of `TOOL_ADAPTERS` (keys, labels, global/project dirs, virtual-group membership),
    generated from `core/tool_adapters/mod.rs`.
  - `index.ts` — `installFixtureBackend()` (Tauri's own `mockIPC` + `mockWindows` so `new Channel()` and plugin calls
    work: dialog picker answers fixture folders, updater = no update, app version, webview zoom) and
    `fixtureInvoke`. `?scenario=` (default `rich`), `?latency=<scale>` (0 = instant, 3 = slow-motion progress).
  - `backend.test.ts` — 16 tests: handler keys == `Object.keys(commands)`; `expectTypeOf` on the handler table;
    per scenario, runtime wire invariants (unique ids/names, provenance rules, registry-known target tools, global
    target rows only `synced|error`, `stale` only on copy-mode assignments, counts consistent); behaviour checks
    (TARGET_EXISTS → override replace, shared-dir dedupe, Refresh phases, toggle round-trip, failures' skip/fail
    kinds, ADR-0002 keep-row, first-run newly-installed-once + import, empty).
- **`src/lib/tauri.ts`** — `const fixtureMode = import.meta.env.VITE_MOCK_BACKEND === "1"`; `isTauri = fixtureMode || <webview check>`;
  `invokeTauri` routes to `import("../fixtures").fixtureInvoke` under the gate. Hooks and their tests are untouched.
- **`src/main.tsx`** — under the gate, dynamic-imports `./fixtures`, installs, then renders; otherwise renders directly.
- **`vite.config.ts`** — function form; `define: { "import.meta.env.VITE_MOCK_BACKEND": mode === "fixture" ? "1" : "0" }`
  (a literal in every mode, so production folds the branches). **`src/vite-env.d.ts`** (new) types the variable.
- **`package.json`** — `"dev:fixture": "vite --mode fixture --port 5175 --strictPort"` (cross-platform, no env prefix).
- **`AGENTS.md`** — Commands: one `npm run dev:fixture` entry.

### Scenario honesty

Only backend-producible states: `stale` appears solely as copy-mode drift on **project assignments** (rich: tdd,
t3-orchestration, vitest-migration); global target rows are `synced` or `error` (propagation/removal failure).
The BRIEF's "stale = update available", "failed on Cursor" and "disabled" were translated: vitest-migration is
5/7 (no Amp/Kimi pair), zod-schema-first is 6/7 because Cursor holds a *different* foreign `zod-schema-first`
(next sync there → `TARGET_EXISTS` → overwrite ask), prisma-review is simply 0/7, using-git-worktrees has two
`error` rows (Kimi/Amp). "Update available" does not exist anywhere.

`failures` holds: Windsurf selected but not detected (every sync → `TOOL_NOT_INSTALLED` skip), Amp/Kimi shared dir
unwritable (`TOOL_NOT_WRITABLE` skips + propagation failures), occupied targets on Cursor/Codex/Gemini
(`TARGET_EXISTS`), Unlocatable skills (source_missing detachable, source_missing + central gone, central_missing
refreshable → Restore), acquisitions failing `RATE_LIMITED` / `GITHUB_SKILL_NOT_FOUND` / `GIT_CLONE_FAILED` auth+tls /
`SYMLINK_ESCAPES_REPO`, `skipped_acquisition`, an Edit conflict, a `reassert_error`, a locked target whose removal
fails (row kept `error`), imported skills (with/without found-in Tool), an orphan Windsurf row, projects with a
missing folder, `error`/`missing`/`stale`/`pending` assignments and an unwritable project tool, onboarding groups
that fail `SKILL_EXISTS` / `SKILL_INVALID`, a conflict group, and an original that cannot be removed. Add-flow
repos `corp/private-skills` (auth) and `rate/limited` fail listing in this scenario; unknown repos fail
`GIT_CLONE_FAILED notFound` everywhere.

Walkable demo data in every scenario: Add → Git `https://github.com/total-typescript/skills` (4 candidates → picker;
`ts-reset` collides in Claude Code → overwrite ask), `https://github.com/acme/changelog-skill` (single → direct
install), Local `~/Projects/team-skills` (3 valid + 2 invalid → picker) or `~/Projects/pi-lens/skill`; the native
folder picker rotates `~/Projects/team-skills`, `~/Projects/new-service`.

### Tree-shake proof

```
$ npm run build      # (ran inside npm run check)
$ grep -rlE "skills-hub-fixture-backend|fixtureInvoke|mockIPC|vitest-migration" dist/ ; echo "exit=$?"
exit=1               # no match: no fixture code or data in the production bundle
```
Control (proves the grep can hit): `npx vite build --mode fixture --outDir /tmp/fixture-dist` →
`/tmp/fixture-dist/assets/fixtures-*.js` contains `skills-hub-fixture-backend`.

### Screenshots (local-only, gitignored)

`/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/app/` —
`{myskills,detail,add,projects,settings,explore,import}-{dark,light}.png` (14, all inspected; rich scenario,
1280×800), plus extras: `overwrite-ask-dark.png`, `failures-myskills-dark.png`, `failures-refresh-report-dark.png`,
`failures-projects-dark.png`, `first-run-myskills-dark.png`. `fxhelper.js` in the same folder is the in-page driver
used to reach each surface (`await import('/@fs/<abs path>/fxhelper.js')`, then `window.__fx.<surface>()`).

Walk verified in the browser (rich): Add → Git URL → picker (4) → Install selected → sync progress ticks
"Sync (n/6)" (Amp/Kimi deduped) → overwrite ask "ts-reset → Claude Code" → Overwrite → "Selected skills installed.";
Refresh all → "Fetched (n/64)" then "Updating (n/64)" → "All skills refreshed."; Projects → quartermaster → assign
`pdf` × `.agents/skills` → cell synced, skill count 5→6; Settings renders. Failures: Refresh all → "Update failed:
pr-review … rate limit … (+10 more)", 17 notifications.

### Verification

- `npx vitest run src/fixtures` → 16 passed.
- `npm run version:check` → `Version OK (1.2.17)`; `npm run check` → exit 0 (vitest 17 files / 426 tests, build,
  fmt, clippy, cargo test 692 passed).

### CHANGELOG one-liner (for ticket 10)

"Dev: `npm run dev:fixture` runs the app in a plain browser against an in-memory fixture library
(`?scenario=rich|empty|first-run|failures`); dev-only and tree-shaken from release builds."

### Open questions / notes for the orchestrator

1. **Driving `preview_*` from a delegated thread**: the hidden tab's page only advances while a `preview_evaluate`
   is in flight (timers/paint freeze between calls), several tool calls issued in one batch run concurrently and
   disconnect the client, and `preview_click`/`preview_type` sometimes time out and reload the page. What worked:
   one call at a time; drive each flow *inside one awaiting evaluate*; before `preview_snapshot`, force a full
   repaint (the helper's `repaint()`), otherwise captures are partially painted. Worth a line for tickets 06/07.
2. Setting-value flows of ticket 02 (keychain token): the fixture follows the current bindings
   (`github_token_set: boolean`); if ticket 02 changes the `AppSettings` shape again, `model.ts`/`backend.ts`
   `applySetting` need the matching one-line change (the build will say so).
3. Concurrent children's edits hot-reload the fixture app (state resets on full reload); reviewers should expect it
   while other tickets are in flight.
4. Findings spotted incidentally for 06/07 (not fixed — out of scope): the Import modal shows raw tool keys
   ("Found in claude_code"), the Projects matrix header shows raw keys (`CLAUDE_CODE`, `AGENTS_SKILLS`).
5. `global_selected_tools_corrupt` (startup warning + `SETTING_CORRUPT`) is not in any scenario — it would block
   every sync; add a `?corrupt=1` flag if a reviewer needs that state.

