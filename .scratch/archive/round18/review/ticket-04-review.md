# Ticket 04 — adversarial review of `5a72ebe`

Reviewed: `git show 5a72ebe --stat` and `git diff 5a72ebe~1 5a72ebe`, against ticket 04 § Work, round18 D4, AGENTS.md and CONTEXT.md. Source remained untouched; no commits, staging, Tauri launch, or new server. The concurrent ticket-02 work is excluded.

## Verdict

**Fix-then-ship.** Production isolation passes. The fixture is walkable and genuinely typed, but several transitions differ materially from Rust. Fix the should-fix findings before tickets 06/07 use it as evidence of the product's behavior.

## Blocking

**None found on the production-safety axis.** See the fresh build, module-graph and browser evidence below. In particular, the manifest import and Tauri mocks do not enter the normal production bundle.

## Should-fix

### S1 — Refresh silently discards auto-sync reassert failures

**`src/fixtures/backend.ts:521–526`, `548–552`, `932–938`**

`reassert` drops every result, including `TARGET_EXISTS` and `TOOL_NOT_WRITABLE`, and excludes uninstalled selected Tools before generating their skips. Update/Refresh/Re-point return only the preceding Propagation outcomes. Rust `core/refresh.rs:439–522` converts and merges the batch's outcomes into `targets` (unwritable becomes failed; uninstalled becomes skipped).

**Reproduced:** rich → Update `zod-schema-first`, `reassert_auto_sync: true`. Cursor remains absent because the foreign copy blocks it, but the returned report contains no Cursor outcome. The real `refreshOutcome` folds it into `{ toast: { kind: "success", message: "UPDATED" }, errors: [] }`. This also makes the ticket's “All skills refreshed” demonstration falsely reassuring.

**Fix:** return typed Propagation outcomes from reassert, using the same batch/dedupe policy as global sync; merge them for Refresh, Update and Re-point. Add a regression asserting the occupied Cursor row produces `TARGET_EXISTS` in the report and a non-success fold; cover selected-but-absent and unwritable Tools too.

### S2 — Auto-sync import deletes originals even when their takeover fails

**`src/fixtures/backend.ts:709–713`**

Every identical original is removed from `state.foreign` *before* sync. `syncPair` already checks writability and removes an occupant only after authorizing replacement, so this pre-removal bypasses that settlement rule. Rust `core/onboarding_import.rs:304–385` leaves the originals in place and delegates replacement to global sync.

**Reproduced:** failures → import `commit-message` from Codex, auto-sync on, requested Tools `["codex"]`. Amp is forced because it has an identical original. Amp reports `TOOL_NOT_WRITABLE`, yet both foreign originals disappear and the next onboarding plan has no `commit-message`. Successful Codex takeover also incorrectly reports `replaced: false`.

**Why:** critics cannot see the remaining original/recovery state after a partial import; the fixture simulates losing an artifact the backend deliberately preserves.

**Fix:** remove the pre-emptive `dropForeign` loop. Let successful same-content sync replace the occupant; preserve failed/skipped originals. Assert the unwritable Amp original remains discoverable after import.

### S3 — Import bypasses the global target-selection/dedupe policy

**`src/fixtures/backend.ts:705–714`**

Import uses `policy.tools ?? state.installedTools` and a per-tool `flatMap`, unlike Rust's effective selection and `sync_skills_to_tools_unlocked` (`core/onboarding_import.rs:321–382`). Shared Amp/Kimi paths therefore yield two report rows instead of one physical attempt. A null policy ignores a saved selection.

**Reproduced:** first-run → save selection `["claude_code"]`, scan all → import `pdf` with `{ auto_sync: true, tools: null }`. Fixture reports seven targets including both Amp and Kimi; Rust requests Claude Code only (the original is already there). The current import UI supplies explicit Tools, so the null-policy case is a command-contract defect rather than a claim that today's UI over-deploys; the shared-dir duplication affects its ordinary seven-Tool import.

**Fix:** select `policy.tools ?? effectiveTargets()`, append forced identical Tools, and call the existing `syncBatch` without a progress sink. Test saved empty/nonempty selections and shared-dir report cardinality.

### S4 — Propagation invents automatic recovery of errored links

**`src/fixtures/backend.ts:481–490`, `498–504`; `src/fixtures/scenarios.ts:318–323`**

The fixture skips links only when their status is `synced`; it retries error/missing/pending links. Rust's `needs_new_bytes` is based on mode/capability alone (`core/propagation.rs:99–101`, `234–248`, `378–382`): symlink-capable link rows produce `link_follows_source`, regardless of stored status.

**Reproduced:** rich → Update `using-git-worktrees` with reassert off. Its Amp/Kimi **symlink/error** rows become **symlink/synced**. Rust would skip and leave those rows alone. The existing test at `backend.test.ts:130–137` positively asserts this inaccurate recovery. Conversely, failure-scenario errored links produce propagation failures that Rust would not attempt.

**Fix:** mirror `needs_new_bytes`, not status-based retry. Where the scenario is intended to demonstrate a propagation failure or Refresh repair, seed a copy-mode target/assignment. Test that error-state links remain skipped and copies can recover/fail.

### S5 — Cancellation uses the wrong result contract and is ignored by other cancellable operations

**`src/fixtures/backend.ts:578–585`, `876–891`, `905–943`, `1026–1036`, `1045–1047`**

Refresh cancellation throws a whole-command `CANCELLED`; Rust instead returns a `RefreshReport` containing failed `CANCELLED` rows for selected work plus pre-existing skipped rows, and applies nothing (`core/refresh.rs:217–237`). The fixture also only reads `cancelRequested` in Refresh: Git install, Git Re-point and Explore clone ignore it, though their real commands pass a reset `CancelToken` (`commands/mod.rs:393–424`, `680–704`, `1145–1163`). Resetting after the initial Refresh pause can additionally erase an early cancellation.

**Reproduced:** request cancel on the first acquisition tick → fixture rejects `{ code: "CANCELLED" }`, emits no apply tick, leaves skills unchanged. The no-mutation part is good; the rejection bypasses the actual report fold and its cancellation notifications. Other handlers never consult the flag.

**Fix:** reset at operation entry; model cancellation at the corresponding acquisition boundary. Return the real per-skill cancellation report for Refresh/Update/Re-point (Re-point uses `refresh_managed_skills_with` too), and typed rejection for Git install/Explore clone. Add deterministic injected-sleep tests, including an early cancel and no partial apply.

### S6 — Git Add ignores the operator's custom name

**`src/fixtures/backend.ts:876–888`**

The handler omits `name` and always finalizes `candidate.name`, explicitly acknowledging the divergence in its comment. The generated command takes `(repoUrl, subpath, name, resolution)`; both Add and its picker pass the custom name (`useAddSkillFlow.ts:158–159`, `463–469`). Real finalize honors it.

**Reproduced:** installing `acme/changelog-skill` / `./changelog-writer` with name `my-custom-name` returns and stores `changelog-writer`.

**Why:** reviews of Add/rename/collision recovery will diagnose a fake product defect. TypeScript correctly permits a handler to ignore trailing arguments, so coverage typing does not catch this.

**Fix:** consume `name`, use `name ?? candidate.name` as the local handler does, and test the returned DTO, catalog name and collision check for the explicit name.

### S7 — “Reconciled” project DTOs contradict content-identity rules

**`src/fixtures/model.ts:409–423`; `src/fixtures/backend.ts:398–400`, `434`**

Every assignment gets the current central hash, including stale copies and healthy links; successful link sync also records the hash. Rich's three stale copies all have `assignment.content_hash === skill.contentHash`, yet `getProjectView` claims `reconciled: true`. Rust `sync_status::next_status` makes a present copy with matching hashes **synced**, and `project_sync::sync_assignment_target` records **null** for links (`core/sync_status.rs:155–180`; `core/project_sync.rs:125–140`).

**Reproduced:** `t3-orchestration`, `vitest-migration`, and `tdd` each have identical recorded/current hashes while marked stale. This is target-drift vocabulary with non-drift data, not an “update available” state; the latter is correctly absent.

**Fix:** seed a distinct recorded hash for stale copies, current hash for synced copies, and null for links/new pending rows. Clear the hash on link sync. Extend scenario invariants beyond enum membership to these relationships.

## Nits

- **N1 — Progress phases overstate work:** `src/fixtures/backend.ts:588–591`, `965–969` emits `applying` for failed acquisitions and refused imports. Rust emits it only after successful acquisition/admission (`core/refresh.rs:251–275`, `core/onboarding_import.rs:188–206`). Shapes and termination are correct, but a reviewer briefly sees “Updating” for work that never reached apply. Separate admission/acquisition results from settlement and emit only the corresponding ticks; test the failure scenario's sequence.
- **N2 — Ticket wording:** `.scratch/round18/issues/04-fixture-browser-mode.md:11`, `28` says five streaming commands and “not committed.” The generated table has **four** Channel-bearing commands, all implemented; commit `5a72ebe` exists. Correct these phrases when recording the fixes so they do not imply missing scope or pending commit work.

## Verified claims

Verification used both the live checkout and an isolated `git archive 5a72ebe` export with the existing dependencies symlinked in. The exact-commit export avoids the other child's HMR/bindings edits. Its parent `5a72ebe~1` was also exported and built for comparison. Local evidence: `.scratch/round18/evidence/ticket04-review/` (ignored), particularly `t04-exact-gates.log`, `t04-baseline-build.log`, `t04-repro.log`, `t04-extra.log`, and `repro.ts`.

| Implementer claim | Independent verification / qualification |
| --- | --- |
| Dev-only gate; production contains no fixtures | Exact commit `npm run build` exits 0. Grep of its `dist/` for `skills-hub-fixture-backend`, `fixtureInvoke`, `mockIPC`, `vitest-migration`, `dev:fixture`, `typescript-eslint`, `rust:clippy` returns no match. Control `npx vite build --mode fixture --outDir /tmp/t04-fixture-dist` exits 0 and emits `assets/fixtures-CTzElreQ.js` containing the marker. |
| Gate works in all normal modes; cannot be accidentally enabled by the URL | Vite `resolveConfig`: development/production/test define the literal **`"0"`**, fixture defines **`"1"`**. Scenario selection is downstream of that gate. `tauri.conf.json` uses `beforeBuildCommand: "npm run build"` → `tsc -b && vite build`, with no fixture mode; `beforeDevCommand` is ordinary `npm run dev`. `scripts/tauri-dev.mjs` changes only the port/config and forwards CLI args, not Vite mode. |
| No manifest/mocks leak; normal-browser loud failure preserved | Vite library builds of the actual `src/lib/tauri.ts` entry in production/development/test had **zero emitted module IDs matching fixtures, API mocks, or package.json**. Executed the development output in a fresh browser iframe without Tauri globals: `{ isTauri: false, rejection: "Tauri API is not available", tauriGlobals: false }`. This tests the seam, not a newly started ordinary dev server. |
| Plain-browser fixture boots; Channel/plugin shim works | Existing 5175 rich page loaded in `preview_*`: fixture marker/title, `isTauri: true`, 60 skills and rendered My Skills UI. Instantiated a real Tauri `Channel` in the browser and passed it to a fresh in-memory fixture backend: a sync emitted `{ index:1,total:1,skill_name:"changelog-writer",tool:"claude_code" }` and resolved a real-shaped report. Mock app version answered `1.2.17-fixture`. Plugin switch also covers dialog, updater, name and zoom; those individual UI actions were not all replayed. |
| One typed handler per generated command; real DTO shapes | **45 command names**; exact key equality test passes. `FixtureHandlers` maps `keyof typeof commands` to each command's `Parameters` and awaited `ReturnType`, and the object itself is checked with `satisfies` before the dispatch casts. Missing/renamed keys or incompatible return shapes fail type checking. Imported DTOs/CommandError tags are generated types, not mirrors. Casts at generic dispatch preserve a correlation TS cannot express; they do not suppress the handler-table check. Normal TS assignability still allows ignored parameters (S6), and cannot prove semantic invariants (S7). No temporary source mutation was needed for this type argument. |
| Four scenarios, BRIEF roster and genuine empty/first-run | Fresh calls yield rich **60 skills / 7 detected Tools / 3 projects / 5 onboarding groups**; empty **0/7/0/0**; first-run **0/7/0/12**; failures **20/7/4/8**. Tests check rich's roster/project names and first-run newly-detected-once. Failure data includes the claimed typed acquisition errors, unlocatable states, skips, conflict, kept-removal fault and missing project. |
| Only backend-producible states; stale never means upstream update | Enum strings/provenance/error variants agree with bindings and Rust producers; stale rows are copy-mode project assignments, not global targets or upstream flags. No new “update available” state exists. **Semantic claim only partially holds:** S4 and S7, despite passing enum tests. |
| Stateful mutations; overwrite ask and shared-dir behavior | Passing fixture tests independently exercise `TARGET_EXISTS` → override → replaced target, shared-dir global batch dedupe/record fan-out, two Refresh phases, and project toggle round-trip. Unchanged `useSyncOrchestration` still owns confirmation/retry; no fixture UI branch replaces the real fold. **Refresh/reassert and import parity are partial:** S1–S5. |
| ADR-0002 and project failure reports | Extra probe deletes failures' `release-train`: returns a failed Claude Code removal, retains that target as error and the skill/central copy, removes successful siblings. Assigning `prisma-review` to unwritable monorepo `agents_skills` returns a failed report item with an assignment ID and retained error row. |
| Import on/off and report outcomes | First-run auto-sync-on import test passes. Extra auto-sync-off probe of `commit-message`/`sql-review` returns removed Codex/Claude originals, failed unwritable Amp original, and kept-divergent Gemini original. On-path settlement/policy defects are S2/S3; current tests do not cover them. |
| Tests, hooks unchanged, no raw invoke escape | Exact commit: **17 test files / 426 tests passed**, including 16 fixture tests; `npm run lint` and `npm run build` exit 0. Diff adds only fixture tests, changes no existing hook tests. Search of `src/` core imports finds raw `invoke` only in generated `src/bindings/index.ts`; fixture uses the seam and Tauri's mocks for native plumbing. Type assertions on aliases are modest, but the handler `satisfies` check is substantive; behavioral coverage needs the regressions above. |
| No new build warning | Parent and exact commit both transform **723 modules** and emit the same three warning categories: ineffective dynamic imports for core and plugin-dialog, plus >500 kB chunk. No added production warning found. |
| Script and docs present | `dev:fixture` is `vite --mode fixture --port 5175 --strictPort`; AGENTS.md Commands documents it. `vite-env.d.ts` types the compile-time flag. No new dependency or version change. |
| Required screenshots exist | All **14** requested `{myskills,detail,add,projects,settings,explore,import}-{dark,light}.png` files plus five claimed extras exist under `.scratch/round18/evidence/app/`; image metadata reports 1280×800. This verifies artifact presence/dimensions, not every historical walkthrough or all screenshot pixels. |

## Not verifiable here

- Did not run `tauri build`, `tauri:dev`, cargo tests/clippy/fmt, or touch the operator's real database/library. Tauri release isolation is established from its actual build command and fresh frontend builds, not a packaged release run. The ticket's historical 692 Rust tests/full `npm run check` claim is not independently re-run.
- No new plain `npm run dev` server was started, per instructions. Port 5173 belonged to another app (redirected to `/login`); left it alone. Ordinary-mode behavior was checked using the actual Vite development-configured seam in a clean browser iframe instead.
- Did not repeat the complete historical Add → picker → overwrite modal → Refresh → Projects walk or all 14 theme captures. Browser boot/Channel integration, fixture tests and isolated behavior probes were verified afresh. The old rich Refresh success is not evidence of fidelity because S1 explains its false success.
- No claim that all filesystem/race behavior is simulated. Typed DTO compatibility is necessary, not proof of Rust-equivalent state transitions; the findings above are concrete divergences, not a request to emulate an entire filesystem.
