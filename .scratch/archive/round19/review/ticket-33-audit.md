# Ticket 33 — independent audit

- **Commit:** `948f12b584ed5838c8ed6747247f89913193d0ba` against `1cb3d36`
- **Date:** 2026-11-02
- **Reviewer:** OpenAI GPT-6 Astra (`gpt-6-astra`), independent of the implementer
- **Checkout tested:** `3862c751fe427108232360c0f8b331c8674a9ac2`; its only difference from the reviewed commit is the ticket's status line. Production code and tests are identical. Working tree was clean before the audit and after bindings regeneration.

**Verdict: FAIL — the direct Update path is arrival-only, but its confirmed global overwrite retry can still create a previously absent shared-directory sibling target row (F1).**

## 1. Dead-path completeness — PASS

Ran the requested `grep -rniE 'reassert|re-assert|RefreshPolicy|priorGlobalTools|alsoDeployed|reassertFailedTitle' src src-tauri/src`, also including `README.md`, `DESIGN.md` and `docs/adr/`. Exactly one match: the historical comment at `src-tauri/src/core/refresh.rs:24`. No executable re-assert, policy DTO/argument, fixture mirror, old announcement or old error title remains.

Also searched the listed documentation for auto-sync/auto-deploy descriptions. No stale description of the old Refresh re-assert remains in README, DESIGN or ADRs. ADR-0003's auto-sync descriptions concern Onboarding import, not Refresh. CONTEXT's historical and avoid-list mentions are intentional (check 7).

## 2. Creation paths — FAIL (F1)

**The direct Rust operation passes.** The complete write chain is:

1. `refresh.rs:317–318, 473–497` → `skill_update::apply_unlocked`.
2. `skill_update.rs:148–198` re-admits the skill, settles central bytes, then calls Propagation; it does not enumerate selected Tools or create target rows.
3. `propagation.rs:197–206, 236–240` obtains global rows from `list_skill_targets` and forms shared groups by filtering **those rows**, not by adding every registry sharer. Its writes at `:312–322, :342–346` use `SyncCompleted`/`SyncFailed` on existing row IDs. These transitions execute `UPDATE ... WHERE id` (`skill_store.rs:1468–1481, 1539–1559`), not the insert-shaped `Recorded` transition.
4. `propagation.rs:392–401` iterates existing project assignments. `:444–459` sends an existing assignment to `project_sync::sync_assignment_target`; that writes the stored assignment's artifact and transitions its ID (`project_sync.rs:107–163`). No assign/register/fan-out operation is called.
5. The admitted first-deploy exception is row-bound: global `propagation.rs:274–303` tests `!member.owns_artifact()` only inside the existing-row group; project `:435–459` does the same for the existing assignment. Foreign bytes are not force-overwritten without ownership. Failures carry `first_deploy: !owns_artifact()` at `:363` and `:481`.

Fresh `cargo test --all` passed `refresh_all_with_auto_sync_on_creates_no_deployment` (`core/tests/refresh.rs:225–296`). It explicitly installs to Claude Code only, selects Claude/Codex/Cursor with auto-sync on, then asserts the store's entire target list is still exactly `[claude_code]` and neither other artifact exists. This is a store-level absence check, not merely a report assertion, so the conditional request for a replacement Rust test was not needed for the direct path. `a_failed_first_sync_records_an_error_row_that_owns_nothing` and `propagation_never_overwrites_a_path_the_row_does_not_own` also passed.

**The whole frontend Update action fails the stronger claim.** Its Propagation collision is retried through the ordinary global sync batch (`useSyncOrchestration.ts:135–138, 514–522`). That batch derives `record_tools` from **all installed sharers**, not existing rows (`global_sync.rs:313–316`), then inserts/upserts each one (`:138–148`). An existing Amp error row can therefore lead to a new Kimi row after the operator answers Amp's Update collision. The ordinary refresh test stops before this retry.

Executable fixture reproduction and the precise fix are under F1. This is a new **row for the same shared artifact**, not evidence of an unrelated directory being overwritten. No project-row creation path was found.

## 3. D35 gate — FAIL on requested regression coverage; current behavior PASS (F2)

- `refresh.rs:412–418` reads typed health through `skill_health::all_skill_health`, not a string comparison at the caller.
- `All`: `:430–449` reports `DeleteIncomplete` before choosing an Unlocatable outcome, but excludes `NotAMember`. Imported non-members remain neither acquired nor reported.
- `Ids`: `:452–464` applies the same health gate even to an explicitly named skill, so Restore cannot bypass it.
- Skipped rows never enter `acquire_all` or phase-two settlement (`:234–243, :325`); they therefore write neither health nor targets.
- Re-point validates the supplied target first, then enters the same selection gate; its local-copy or Git acquisition is inside the undispatched closure (`repoint.rs:132–168`). Parsing/validation is not a network or clone operation. Invalid targets still fail validation rather than producing a skip, as documented in the Result.
- Fresh Rust tests passed: `a_delete_incomplete_skill_is_skipped_by_refresh_all_and_by_a_named_update` (`tests/refresh.rs:316–417`: zero acquisition calls, unchanged global target records, health and central bytes); `repoint_of_a_delete_incomplete_skill_is_skipped` (`tests/repoint.rs:758–813`: local and Git, unchanged record/targets/health/central).
- Precedence is explicitly stated in `refresh.rs:407–411`. The Re-point test starts from a missing local source, so it covers named Re-point with that overlap. **There is no committed test of the `All` branch with both health and Unlocatable, nor of an imported non-member carrying this health.** The normal imported test has no `delete_incomplete` health; the new Refresh D35 test has a present source and central copy.
- Additional in-memory fixture probe run during this audit: give release-train an Unlocatable state and give imported legacy-lint `delete_incomplete`, then Refresh all. Result: exactly one outcome, release-train `skipped { reason: { kind: "delete_incomplete" } }`; legacy-lint absent. This supports current parity, but does not replace committed Rust regression tests.

## 4. Wire contract — FAIL only on missing shim export (F3)

`RefreshSkipReason` derives Serialize and `specta::Type`, with `#[serde(tag = "kind", rename_all = "snake_case")]` (`refresh.rs:117–134`). Generated binding is at `src/bindings/index.ts:1108`; `SkillRefreshStatus` uses it at `:1362`. `PropagationStatus` derives the same wire traits (`propagation.rs:81–99`), and its generated failed arm requires `first_deploy: boolean` (`bindings/index.ts:1081–1088`).

Fresh `cd src-tauri && cargo test export_bindings`: **1 passed**. Immediately following `git status --short`: **empty**. Fresh `refresh_skip_reasons_cross_the_wire_tagged` and `shared_edit_update_report_preserves_classified_failure_on_the_wire` passed in the Rust suite.

No component imports bindings directly outside the per-world shims (searched `src/components`, excluding `types.ts` and tests). The existing report/status/Propagation exports are present in `src/components/skills/types.ts:23–37`, but the newly introduced `RefreshSkipReason` is not re-exported, contrary to AGENTS.md's explicit “Adding a DTO = derive + cargo test + re-export from the shim” rule. Current consumers infer the nested reason through the report, so this is not a runtime/type-check failure.

## 5. Fold correctness — PASS

`src/lib/reportOutcome.ts:320–337` counts a skipped skill under `skipped`, not failure. `:418–436` puts `delete_incomplete` in warnings with the skill subject and `repair: { verb: "delete" }`. `:535–541` chooses the first-deploy title from `first_deploy`; the existing owned-copy title remains. `overwriteAsk.ts:320–322` preserves the asked row's flag when a retry fails.

Fresh `npm run test`: **49 files / 1043 tests passed**, including these exact test names in `src/lib/reportOutcome.test.ts`:

- `announces no deployment: a synced global target is Propagation's, never a new arrival (D34)` (`:401`)
- `a delete_incomplete skip is a skipped item offering Retry the Delete (D35)` (`:413`; asserts warning, no error, delete repair and skipped headline)
- `a failed first deploy reads 'Could not deploy'; an owned copy keeps 'update the synced copy'` (`:433`)

Browser observations below confirm the real translated copy and Delete-dialog action, rather than only translation-key assertions.

## 6. Fixture parity and browser proof — PASS for the requested stock flows

`backend.ts:946–1049` propagates only existing rows/assignments, computes `first_deploy` before settlement, and has no re-assert. `:1074–1092` applies only Propagation. `:1138–1158` mirrors typed skips and membership/precedence for both selections; Re-point gates before its acquisition at `:1707–1710`. `scenarios.ts:529` retains release-train with `delete_incomplete`.

`lsof -nP -iTCP:5175 -sTCP:LISTEN` found Node PID 17606. The existing server worked; **no server was started or restarted**. Used ego-browser in one audit-owned TaskSpace, then closed it. No tracked file was saved during the flows.

### `?scenario=rich`

- Opened Settings and read the Auto-sync switch's `aria-checked`: **`true`**.
- Captured all **60** skills' seven strip labels before and after Refresh all.
- **Zero previously “not synced” pairs gained a deployment.** No dialog was inserted during the action (MutationObserver), hence no overwrite ask.
- Toast and Notifications headline: **“60 refreshed”**; no “also deployed” item.
- The only strip-state change was the allowed recovery of using-git-worktrees' **existing** Amp and Kimi failed copies to synced. Its Gemini “not synced” stayed unchanged. Thus the ticket Proof's literal “every strip is unchanged” is too strong; the Result correctly records this exception. No finding against the implementation for legitimate Propagation recovery.

### `?scenario=failures`

- release-train's eight strip labels were exactly equal before/after Refresh all.
- Run headline: **“11 refreshed · 5 failed · 5 targets failed · 5 skipped · 1 Edit to review”**.
- Warning item: **“Delete incomplete — finish removing release-train first”**, with the explanatory reason and **Show / Remove…**.
- Clicking **Remove…: release-train** opened **“Remove release-train?”**, the Delete confirmation dialog. I did not confirm removal.
- Run contained both **“Could not deploy json-canvas to Amp”** and **“Could not update the synced copy: tdd -> Amp”** (also their Kimi counterparts). The project first-deploy failure was distinguished too.

Screenshots, captured and inspected, under `.scratch/round19/evidence/33-audit/`:

- `rich-before.png`, `rich-after.png`
- `failures-titles.png`
- `failures-skipped.png`
- `failures-delete-dialog.png`

The separate modified-in-memory fixture reproduction of F1 is not a stock-scenario failure; it exposes a state the stock seeds do not exercise.

## 7. Documentation — PASS for the intended rule, subject to F1's code exception

Reviewed the exact changed sentences in AGENTS.md and CONTEXT.md. They accurately describe the direct update path, new typed skip and removal of the old re-assert. The producer list correctly retains Propagation, because its existing never-owned rows can still collide.

`grep` for `every Tool in the`, `auto-sync invariant`, and `re-assert` in both files finds only `CONTEXT.md:30` (explicit history) and `:32` (avoid-list); no surviving assertion of the removed behavior. Relevant current entries: `CONTEXT.md:142` Disabled, `:154` Propagation, `:166` Re-point, `:170` Refresh. AGENTS.md's affected clauses are at `:194`, `:327`, `:346–358`, `:378`.

The absolute “never creates one” promise is not fully true of the overwrite-retry route identified in F1; fix that route rather than reintroducing old policy prose.

## 8. Arrival regression surface — PASS (unchanged)

- **Add:** `install_batch.rs` has no diff in this commit. `:287` still gates first sync on `request.deploy && !request.tools.is_empty()`. `useAddSkillFlow.ts:282–292` still derives the request's tools/deploy from auto-sync.
- **Onboarding import:** its only production diff is a comment. The auto-sync branch and requested/default Tools remain. `useAddSkillFlow.ts:906–907` still passes auto-sync and the selected scope.
- **Enable:** `skill_disable.rs` has only two comment changes. `:392–420` still selects disabled rows and `:427–452` performs first sync with overwrite off/same-content on. Precision: Enable is an explicit request to restore the selected disabled scope; it is not newly gated on the auto-sync setting and does not automatically deploy to every selected Tool. This ticket did not change that behavior.

Fresh `cargo test --all` passed all **830** tests, including:

- `install_batch::tests::a_batch_of_three_with_one_collision_answers_one_report_and_writes_nothing_through_it`
- `install_batch::tests::deploy_off_installs_without_targets`
- `onboarding_import::tests::auto_sync_on_overwrites_the_source_tool_in_place_across_its_shared_dir_group`
- `onboarding_import::tests::auto_sync_on_force_includes_a_source_tool_the_policy_deselected`
- `onboarding_import::tests::import_without_a_policy_selection_follows_the_recorded_global_selection`
- `onboarding_import::tests::auto_sync_off_without_remove_originals_leaves_and_reports_every_original`
- `skill_disable::tests::enabling_a_pair_resyncs_it_as_a_first_sync`
- `skill_disable::tests::enabling_a_skill_resyncs_every_disabled_row_and_clears_the_marker`
- `skill_disable::tests::an_enable_collision_is_askable_and_its_overwrite_retry_replaces_the_folder`
- `skill_disable::tests::a_disabled_pair_survives_refresh_and_propagation`

Additionally ran the full definition-of-done command, **`npm run version:check && npm run check`**, successfully: version 1.3.0; lint/styles clean; 1043 frontend tests; TypeScript/build passed; Rust formatting/clippy clean; 830 Rust tests. Build emitted non-blocking dynamic-import/chunk-size notices. A green gate does not cover F1.

## Findings

### F1 — P0: Update's overwrite retry can create an absent global pair row

**Why P0:** the audit rubric defines a false central claim as P0. This falsifies “Update never creates a target row for a pair without a row,” but does **not** demonstrate cross-directory data loss. It requires an operator-confirmed collision retry and a newly detected shared-directory sibling.

**Reachable state:** a first sync to Amp failed while Kimi was not installed, leaving only Amp's never-owned error row. Kimi is subsequently installed. A foreign folder occupies their shared path. Update's Propagation asks about the existing Amp row. Answering Overwrite invokes the general sync batch, whose installed-sharer fan-out inserts Kimi's previously absent row. This occurs inside the Update action; it is not a separate Deploy action. The retry report is then limited to asked pairs (`useSyncOrchestration.ts:527–536`), so that additional row is not announced.

**Evidence:** `useSyncOrchestration.ts:135–138, 514–522`; `global_sync.rs:313–316, 138–148`; insert behavior `skill_store.rs:1444–1466`. The plain Propagation operation correctly avoids the sibling; the retry does not preserve its restriction.

**Reproduction run:** `bun --eval` with the following body, using only the in-memory fixture (no source changes or operator data). The last assertion **failed**, exit 1:

```ts
import { buildScenario } from "./src/fixtures/scenarios.ts";
import { createFixtureBackend } from "./src/fixtures/backend.ts";
const now = Date.UTC(2026, 10, 2);
const state = buildScenario("failures", now);
const skill = state.skills.find(s => s.dto.name === "json-canvas")!;
state.skills = [skill];
state.projects = [];
state.unwritableTools = [];
// Represents an Amp failure predating Kimi's installation.
skill.dto.targets = skill.dto.targets.filter(t => t.tool === "amp");
state.foreign.push({
  tool: "amp", name: skill.dto.name, fingerprint: "foreign", linkTarget: null,
});
const { handlers } = createFixtureBackend(state, { latencyScale: 0, now: () => now });
const sink = { onmessage() {} } as never;
const tools = async () => (await handlers.getManagedSkills())[0].targets.map(t => t.tool);
const before = await tools();
const { report } = await handlers.updateManagedSkill(skill.dto.id, sink);
console.log({ before, report, afterUpdate: await tools() });
// The same command/policy used by settleClaims for this Update collision.
await handlers.syncSkillsToTools(
  [{ skill_id: skill.dto.id, name: skill.dto.name, source_path: skill.dto.central_path }],
  ["amp"],
  { overwrite_if_same_content: true, overrides: [
    { skill_id: skill.dto.id, tool: "amp", decision: "overwrite" },
  ] },
  sink,
);
const after = await tools();
console.log({ afterOverwriteRetry: after, newRows: after.filter(t => !before.includes(t)) });
if (JSON.stringify(before) !== JSON.stringify(after))
  throw new Error("D34: Update collision retry created a previously absent pair row");
```

Observed: **before `[amp]`; after Update `[amp]`, with `TARGET_EXISTS` / `first_deploy: true`; after overwrite retry `[amp, kimi_cli]`.**

**Exact fix:** preserve the Update/Propagation origin through collision retry and use an existing-row-only backend settlement for that route, under the mutation guard. Restrict both success and failed-first-sync record fan-out to rows admitted for that Update; do not change arrival/explicit Deploy fan-out globally. Preserve shared-directory disable/ownership rules and the existing project's row-only retry. Add a Rust store-level regression for the sequence above (one target before, still one after the confirmed retry) and a frontend/fixture regression that exercises the same retry route. If shared siblings are intentionally exempt from D34, that requires an explicit spec exception and truthful reporting; the present “none” claim cannot simply be marked passed.

### F2 — P2: two stated D35 boundary rules lack committed regression coverage

`tests/refresh.rs:316–417` tests locatable skills; `refresh_all_never_puts_an_imported_skill_in_the_batch` (`:833` onward) does not give the imported skill `delete_incomplete`. `tests/repoint.rs:758` exercises missing-source overlap only through the named path. Neither pins the `All` branch's precedence/membership interaction at `refresh.rs:430–449`.

**Exact fix:** table-test `All` with (a) a refreshable skill having `delete_incomplete` and source missing, (b) central missing, and (c) an imported skill with `delete_incomplete`. Assert typed DeleteIncomplete for (a)/(b), no outcome for (c), zero acquisition calls, and unchanged health/targets/central state. Mirror those boundary cases in the fixture tests. Current code is correct; this is weak coverage, not an observed D35 mutation bug.

### F3 — P2: `RefreshSkipReason` is absent from the skills type shim

`src/components/skills/types.ts:29–31` exports `SkillRefreshStatus` and the old reason-related types, but not the new generated `RefreshSkipReason`.

**Exact fix:** add `export type { RefreshSkipReason } from "../../bindings";` beside `SkillRefreshStatus`. Do not hand-mirror the type. This completes the explicit AGENTS.md new-DTO rule; current consumers remain valid because they infer the nested type.

## Deliberately not checked / boundaries

- No `tauri:dev`, native release smoke, real-library mutation, actual GitHub acquisition, or OS keychain access. All browser actions used the fixture; Rust tests used their temporary roots.
- Did not confirm removal in the fixture Delete dialog; the requested repair-to-dialog behavior was proved without it.
- Did not test simultaneous direct IPC mutations while acquisition is in flight, or the separate Re-point preview command's network behavior. The D35 evidence above concerns skills already marked at selection and the actual mutation commands.
- F1's executable reproduction is the fixture command sequence plus a traced production Rust/hook path, not a newly compiled Rust regression or a browser-driven custom scenario. The existing direct-path Rust absence test was sufficient to satisfy the conditional test request for ordinary Refresh, but does not cover this retry edge.
- No source code changed, no fixes applied, no commits made. This audit is the only authored repository document; screenshots are confined to the requested gitignored evidence directory.

## Re-check — 2026-11-02 (fix 23938db)

**Verdict: PASS-WITH-FINDINGS — F1–F3 closed under the recorded D34 amendment; one documentation-only P2 remains (F4).**

Reviewed `git show --stat 23938db` and `git diff 0946250..23938db`, the last two ticket Comments, D34/amendment and the glossary entries. Tested HEAD `7b05e94` (includes vocabulary-only `60dc80e`). The rejected existing-row-only retry proposal is not the criterion for this re-check.

### 1. F1 under the decision — PASS

- `src-tauri/src/core/propagation.rs:289–328`: `overwrite` means an **installed** member owns the artifact; only its false branch discovers installed sharers lacking a row. Owned-artifact updates add none. Skill Disable returns at `:140–159`; group Disable skips at `:247–258`, before sibling discovery. Existing members and new siblings enter `handled` (`:240–244, :313–317`); Rust also iterates a pre-write snapshot. The fixture's growing array is protected by its equivalent set.
- Success records siblings with `Recorded` and reports each `Synced` (`:361–380`). Failure matches the batch's **new-row rule** literally: `TARGET_EXISTS` creates no sibling; other failures use `FailedRecorded`, null ownership and `first_deploy: true` (`:383–455`). Compare `global_sync.rs:451–469` (collision bypasses `record_failed`), `:496–523` (`FailedRecorded` for each installed `record_tools` member), and `:313–316` (installed-only enumeration). Existing Propagation rows still settle `SyncFailed`; “records nothing” on collision concerns new rows, not erasing/updating existing ones.
- Individually ran `cargo test --all <name>` for all six additions in `core/tests/propagation.rs`; each returned **1 passed / 0 failed**:
  - `a_first_sync_records_every_installed_sharer_of_the_shared_dir` (`:768`)
  - `a_first_sync_records_no_sharer_that_is_not_installed` (`:810`)
  - `an_update_of_an_owned_artifact_records_no_new_sharer` (`:828`)
  - `a_first_sync_collision_records_no_sharer_and_its_overwrite_retry_matches_propagation` (`:853`)
  - `a_failed_first_sync_records_installed_sharers_error_as_the_batch_does` (`:927`)
  - `a_disabled_member_mints_no_sharer_and_skips_the_group` (`:959`)
- The collision test compares `row_set(&f)` with `row_set(&control)`, including harness, status, mode, relative path and ownership—not merely report length.
- Fresh `bun .scratch/round19/evidence/33-fix/repro.ts`: **PASS**. Foreign folder present → Update reports Amp `TARGET_EXISTS / first_deploy: true`, rows remain `[amp]`, confirmed retry yields `[amp, kimi_cli]`. Absent → Update itself reports both Synced and already yields `[amp, kimi_cli]` before retry.

### 2. Additional two-writer consistency sequence — PASS

Ran an independent `bun --eval` fixture comparison, not merely a seeded final-state assertion: remove all target rows and uninstall Kimi; make Amp unwritable; call the global batch and assert its actual failed first sync leaves exactly `[amp: error, owns=false]`. Install Kimi, restore writability, then fork equivalent fixtures between plain Update and the global batch, with no foreign occupant. **Both yield `[amp: synced, kimi_cli: synced]`, the same mode/path/ownership.** Repeated with an unowned Disabled Kimi pair marker inserted after that failure: **both preserve `[amp: error, kimi_cli: disabled]` unchanged**, reporting Disabled skips. Deep equality assertions passed for both comparisons. No differing row set found in these additional sequences.

### 3. F2 / D35 boundary coverage — PASS

`core/tests/refresh.rs:425–564` contains `refresh_all_reports_delete_incomplete_before_unlocatable_and_never_for_a_non_member`; individual run: **1 passed / 0 failed**. Source-missing and central-missing premises first prove Unlocatable, then assert typed DeleteIncomplete; imported non-member is absent. Every case asserts zero acquisitions and unchanged health, targets and central `SKILL.md` state. Fixture mirrors at `src/fixtures/backend.test.ts:600–632` cover both missing states and the imported case; all passed in the fresh suite.

### 4. F3 / shim — PASS

`src/components/skills/types.ts:30` re-exports `SkillRefreshStatus, RefreshSkipReason` directly from `../../bindings`; no hand-mirrored type.

### 5. Documentation — FAIL (F4, P2; documentation only)

AGENTS.md `:327–328`, and CONTEXT.md's Update opening (`:77`) and Propagation (`:150`), carry the first-deploy sibling nuance correctly. However, CONTEXT.md `:81` still independently says Update “never creates one” without that qualification. Auto-deploy (`:145`) says “writing it records a Deployment for every installed harness,” which is broader than the first-sync-only implementation: the passing owned-artifact regression above writes an existing copy without recording the newly installed sharer. The `:81` formulation is in the subsequent vocabulary rewrite, not a logic regression in `23938db`.

**Exact fix:** qualify `:81` with “except installed-sharer records on a first deploy into an already-recorded shared directory; see Auto-deploy.” In Auto-deploy, replace the broad writing rule with: “A first deploy into a shared skills directory records every installed sharer, including when Propagation retries a never-materialised row. An update of an owned artifact adds no sibling records.” Keep the artifact/record distinction and the approved retry behavior; no code change requested.

### 6. Fresh gate — PASS

- `npm run test`: **49 files / 1053 tests passed**, 0 failed.
- `cd src-tauri && cargo test --all`: **837 passed**, 0 failed; main/doc-test targets also passed (0 tests).
- Working tree remained clean after those runs, including binding regeneration. Only this appended section was authored; no code edits, commits, native app runs or live-library operations.
