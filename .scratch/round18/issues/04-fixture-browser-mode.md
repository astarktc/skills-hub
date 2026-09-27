# 04 — Fixture-backed browser mode (D4)

Status: implemented
Spec: `.scratch/round18/spec.md` — D4, ticket 04.

## Work

- `src/lib/tauri.ts` is the only backend door (`invokeTauri(name, ...args)` typed over the generated `commands`
  table). Add `src/fixtures/` with a `FixtureBackend` implementing **every** command name as a typed handler over an
  in-memory catalog (`satisfies Record<CommandName, …>` so a new command fails `npm run build`). Mutations mutate the
  catalog and answer the real report/view shapes (`{ report, skills }`, `ProjectViewDto`, `RemovalReport`, …);
  the four `Channel`-streaming commands emit simulated progress events on a timer. `TARGET_EXISTS` rows, failures
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

Implemented in `5a72ebe`. `npm run dev:fixture` serves the whole app on `http://localhost:5175/?scenario=…`
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


## Review fixes

Settles `.scratch/round18/review/ticket-04-review.md` (S1–S7, N1, N2) plus the two fixture gaps the ticket-07 flow
review raised (`.scratch/round18/review/flows.md` § Fixture gaps found: F2, F4). Only `src/fixtures/**` changed. Where a
choice was needed the Rust producer was read and mirrored; it is named per row. Where this section conflicts with the
§ Result prose above ("links skip, copies/errored rows re-materialise", "cancels a running Refresh (`CANCELLED`)"),
this section wins.

| Finding | Change (Rust producer mirrored) | Regression (`src/fixtures/backend.test.ts`) |
| --- | --- | --- |
| **S1** re-assert failures dropped | `reassert` runs the real `syncBatch` over `effectiveTargets()` minus the Tools already in Propagation's global outcomes (not intersected with detection), same-content policy, and converts outcomes: `TOOL_NOT_INSTALLED` → skipped `tool_not_installed`, any other skip (unwritable) → failed, failed → failed. `settleTargets` merges them into Refresh, Update **and** Re-point (`refresh.rs` `reassert_auto_sync_unlocked` / `merge_reassert`). Also closes the flows.md gap "re-assert does not produce `TARGET_EXISTS`" (prisma-review → Cursor). | *S1*: rich Update `zod-schema-first` → Cursor `TARGET_EXISTS` row, real `refreshOutcome` fold is not `success`; failures `prisma-review` → Windsurf skipped, Amp failed `TOOL_NOT_WRITABLE` once (no Kimi row), Cursor `TARGET_EXISTS`; Re-point merges re-assert rows. |
| **S2** import deleted originals before takeover | Pre-emptive `dropForeign` loop removed. Auto-sync on leaves originals in place; the global batch's same-content replacement takes identical ones over (`replaced: true`), a failed/skipped target's original survives. Auto-sync off settles originals per variant in plan order: gone → removed, divergent → kept, unwritable → failed, else removed (`onboarding_import.rs` `sync_imported_unlocked` / `settle_original`). | *keeps an original whose takeover failed*: failures `commit-message` from Codex, tools `["codex"]` → forced `["amp"]`, Codex synced `replaced: true`, Amp skipped `TOOL_NOT_WRITABLE`, next plan still lists the Amp variant. |
| **S3** import bypassed selection/dedupe | Tools = `policy.tools ?? effectiveTargets()` (a saved selection, empty included, is honoured) + forced identical Tools appended, through one `syncBatch` without a progress sink — shared-dir dedupe applies. Admission refusal for an unknown variant is now `NOT_FOUND onboarding_variant` (was `INVALID_PATH`), as `admit` raises it. | *saved non-empty selection* (pdf → `["claude_code"]` only); *saved empty selection* (tdd → only its identical Tools, all forced); *one report row per shared dir* (ego-browser, 7 Tools → 6 rows, Amp not Kimi). |
| **S4** status-based link retry | `needsNewBytes(mode) = mode === "copy"` (every registry Tool supports symlinks) — never the stored status. Global rows handled per shared-dir group: absent members → `tool_not_installed`, no copy in the group → `link_follows_source` for all, else one write settling every member (failure = the sync engine's io error → `OTHER` permission-denied, not the batch's `TOOL_NOT_WRITABLE`; missing central → `INVALID_PATH missing`). Project rows: unknown tool / unavailable project / link skip, else sync (`propagation.rs` `needs_new_bytes`, `propagate_global_rows`, `propagate_one_assignment`). Seeds: rich `using-git-worktrees` Amp/Kimi errored rows are now **copies** (Refresh repairs them — the scenario's intent); failures `tdd` Amp/Kimi errored rows are copies (Propagation retries and fails them); failures `using-git-worktrees`' errored rows stay links. `SkillSpec.copyTools` added. | The old rich Refresh test's "every row synced" assertion is replaced by *re-materialises errored copies on Update; links follow the source* (copies synced, synced links skipped) and *leaves errored links alone and retries (and fails) errored copies* (failures: link rows `link_follows_source` and still `error`; tdd copies failed `OTHER`, still `error`). |
| **S5** cancellation contract | Cancel flag reset at **operation entry** (Refresh/Update, Re-point, Git install, Explore clone — the four commands that `cancel.reset()`). Refresh stops dispatching once a cancel is observed and, if observed, applies nothing and answers every selected skill `failed CANCELLED` followed by the pre-settled skips (`refresh.rs` 217–237). Re-point: cancel observed at its acquisition → `CANCELLED` report row, source untouched (it runs through `refresh_managed_skills_with`). Git install / Explore clone: typed rejection `{ code: "CANCELLED" }`. | *S5* block, deterministic via an injected sleep armed to cancel on its n-th call: entry reset; early cancel (per-skill `CANCELLED`, skips last, no ticks, catalog unchanged); mid-acquisition cancel (two acquiring ticks, no apply, catalog unchanged); Update and Re-point `CANCELLED` rows with the skill unchanged; Git install and Explore clone reject `CANCELLED`, nothing installed. |
| **S6** Git Add ignored the name | `installGitSelection(repoUrl, subpath, name)` finalizes `name ?? candidate.name`; the collision check sees the explicit name. | *finalizes under the explicit name, and collides on it* (`my-custom-name` returned and listed; `My-Custom-Name` → `SKILL_EXISTS`; `null` → manifest name). |
| **S7** hashes contradict reconcile | `recordedHash(mode, status, current)` in `model.ts`: only a copy records a hash; synced/missing copy = current, stale copy = a distinct previous hash, pending/error/link = null. `syncAssignment` records null (every fixture sync lands a symlink) (`sync_status::next_status`, `project_sync::sync_assignment_target`). | `expectRecordedHashes` runs in every scenario's wire-invariant test and again after rich's Refresh-all: link/pending → null, synced copy = current, stale = non-null ≠ current. |
| **N1** progress overstated | Refresh: acquiring ticks on completion; an acquisition failure (incl. `NOT_REFRESHABLE`, `SOURCE_PATH_MISSING`) gets no applying tick; admission skips (`skipped_acquisition`) and conflicts are apply-phase. Import: `admit` (group/variant lookup) before the applying tick; finalize failures (`SKILL_INVALID`, `SKILL_EXISTS`) still tick applying, as in `import_onboarding_selection`. Report order now matches Rust (dispatched outcomes, then skips). | *N1* block: failures Refresh applying ticks = exactly the skills that reached apply; failures import sequence `broken-skill:admitting, :applying, vanished:admitting, sql-review:admitting, :applying`. |
| **N2** ticket wording | "five" → **four** `Channel` commands; "not committed" → implemented in `5a72ebe`. | — |
| **F2** removal ignored unwritability | Artifact removal mirrors `artifact_removal::execute_unlocked`: one presence rule (global rows present; assignments present when `synced`/`stale` in an existing folder — a never-deployed row is removed trivially), one settlement rule — a present artifact under a locked path **or an unwritable parent dir** fails and its rows are kept `error`. Failures `prj-monorepo` gains a deployed `release-train` × `.agents/skills` assignment (the unwritable project tool). | *unsync from a shared unwritable dir* (docx × Amp → one failed target carrying Amp + Kimi rows, both kept `error`); *remove_project keeps the project and the row whose artifact stayed*. |
| **F4** conflict data disagreed | The seed now holds one fact — `acquisition.upstreamInvocation` (what the source's manifest declares); `editConflict` is gone. Update replays the Edit by `skill_edits::replay_unlocked`: conflict iff upstream ≠ base and ≠ Edit; flag = upstream ≠ Edit ∧ (flag ∨ conflict); recorded base := upstream. `design-tokens` seeds an unconflicted Edit (user-only on user-and-model) with upstream now model-only, so card, detail and report all read model-only after the Update. | *the report's upstream becomes the card's base; the flag persists until convergence* (second, unchanged Update reports no conflict, card unchanged). |

Behavioural consequence for demos: rich Refresh-all with re-assert no longer reports "All skills refreshed" — the
Cursor-occupied `zod-schema-first` settles `TARGET_EXISTS` (as Rust would).

Verification: `npx vitest run src/fixtures` → 36 passed (was 16). Gate `npm run lint && npm run test && npm run build`
→ exit 0 (17 files / 449 tests). Tree-shake: `rm -rf dist && npm run build && grep -rlE
"skills-hub-fixture-backend|fixtureInvoke|mockIPC" dist/ ; echo exit=$?` → `exit=1`. The running 5175 server serves
the edited modules (HTTP 200); not re-walked in the browser.
