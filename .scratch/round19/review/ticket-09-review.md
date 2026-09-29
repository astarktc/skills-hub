# Verdict: **FIX BEFORE ACCEPTANCE** — 3 blocking findings; 6 should-fix findings. The requested gates pass, but do not establish the action-door/lifecycle contracts.

## Ticket 09 adversarial review — 2026-09-29

Reviewed the uncommitted tree on `main`. Actual HEAD was `7e8d8c1c9cff630533b94ef534cd237b83a6c5e2`, not the brief's `6d7b6f4`: the intervening commits are tracker/handoff documentation. Ticket 04 is present in the same tree; findings below identify integration boundaries rather than treating its changes as ticket 09's implementation. No source edits, commits, archive moves, live Tauri runs, or nested delegation.

Read: whole AGENTS.md, BACKLOG/latest handoff, round-19 spec D7/D9/D19–21 and adjoining policies, ticket 09 Result/Comments, round-18 architecture candidates 2/3/6, and results of 02/03/08/10. Skills used: code-review (both standards/spec axes, directly as the assigned review leaf), React best practices, browser routing, shadcn and LSP guidance.

## Ranked findings

### B1 — Blocking: the project world still has mutation paths outside the lifecycle, and does not disable other mutations

**Locations:** `src/components/projects/ProjectsPage.tsx:141–155,200–210,303–316,364,382,401,421`; `src/components/projects/AssignmentMatrix.tsx:346,373,383`.

`handleAddProject`, `handleEditSave` and `handleUpdatePath` call mutating project methods without `runAction`. In particular, Edit's `updateGitignore` writes files and participates in the backend mutation guard. These requests can be sent while another action owns `pending`; they never encounter its refusal. The backend guard may serialize disk work, but that means the second mutation queues in the backend, contrary to D19's frontend refusal rule. The Result's assertion that *every* project mutation runs under `runAction` is false.

Separately, only the active cell/row gets `pendingCells`; there is no global blocked reason passed into the matrix/list. Other cells, row verbs and toolbar actions remain enabled, and project dialogs explicitly receive `loading={false}`. The reporter catches some second clicks, but the contract is disabled control **and** an explainable refusal, not a button that looks usable and does nothing.

**Repro:** hold `toggleProjectSkillAssignment` unresolved. Invoke an Edit save: `updateProjectGitignore` is still sent. In the failures fixture, click `release-train - claude_code`, then `docx - claude_code`: the second control and Sync Project were enabled with no running-reason title; the toast said `Project assignment is running — wait for it to finish`. The guarded second toggle was correctly refused; the unguarded paths are the escape hatch.

**Fix:** route registration, Edit/save and path updates through the same reporter, leaving folder *selection* outside if desired and gating its eventual mutation. Thread global pending/refusal into the project list, matrix and dialogs; preserve per-cell pending solely for the localized mark. Add a hook-level project-action test that holds one mutation and verifies each other mutation sends zero commands. Do not add product-component rendering tests.

### B2 — Blocking: Update/Restore/Re-point/Edit never settle their target collisions through the one overwrite ask

**Locations:** `src/hooks/useSkillActions.ts:276–284,390–398,429–438,486–493`; `src/hooks/useSyncOrchestration.ts:329–368,396–450`; `src/lib/overwriteAsk.ts`.

`refreshOne` replaces the returned catalog and immediately folds `response.report`; Refresh-all immediately folds its report; Edit does the same with its propagation report. None invokes collision settlement. The door's sync dependency does not even expose a Refresh/Propagation resolver. Ticket 04 provides global/project/Enable collision resolvers, but no integration here connects `TARGET_EXISTS` from Refresh re-assert or Update/Edit/Re-point propagation to them.

This is an acceptance gap at the **04/09 integration boundary**, not a demand to make report folds issue commands. AGENTS.md/D12 explicitly includes re-assert and Propagation among the one-ask producers. Currently these paths simply produce target failures and leave the operator without the promised Keep/Overwrite decision.

**Failing-test sketch:** return an otherwise successful `updateManagedSkill` result with a target `Failed { error: TARGET_EXISTS { path, modifiedAt, sameContent } }` (repeat for `refreshManagedSkills` re-assert and Edit propagation). Compose the real sync/surface seams; assert one overwrite ask appears, dismissal sends no retry, and answered rows replace the initial failures. Today no ask is made. The existing Update tests use `targets: []` and cannot detect this.

**Fix:** add a report adapter/resolver for these report shapes, collect global and project collisions together, ask once, execute only the answered retry pairs and fold the merged report. Refresh the affected catalog/view after a retry; the pre-retry catalog envelope is no longer current. Coordinate with 04's owner/reviewer so the same integration defect is not counted as two independent fixes.

### B3 — Blocking: a compound door action can raise multiple asks and publish multiple run blocks

**Locations:** `src/hooks/useSkillActions.ts:311–329,507–514,522–540`; `src/hooks/useSyncOrchestration.ts:363–368,424–450`; `src/components/projects/useProjectState.ts:351–368` (per-skill assignment settlement).

`deployGroup` calls the already-asking Enable seam, executes its Outcome, then calls the already-asking sync seam and executes a second Outcome. A group containing a disabled skill and a never-deployed skill can therefore show **two sequential overwrite dialogs and two report blocks for one click**. `retry` loops over Tools and calls the asking sync seam for each; assigning several skills calls a project verb that settles/asks per skill. Merging reports *after* those calls does not consolidate the asks.

The door was meant to hide batch boundaries and confirmation policy. This implementation instead promotes lower-level seams that each think they own the complete operator action. It violates the brief's `deployGroup = one batch + one ask`, D12, and the one-run report contract.

**Repro/test sketch:** group A is disabled and its old target is occupied; B is undeployed and its selected target is occupied. `run("deployGroup", [A, B])`: resolve A's ask and observe B's second ask. Repeat `retry` for one skill with failed rows on two occupied Tool paths. Assert one ask containing both pairs and one `recordRun`, not merely that both seams were called.

**Fix:** separate acquisition/settlement from asking. Have the operator-level door collect the complete report before invoking the shared ask once and folding once. Where the command API cannot express the selected per-pair/enable+fresh work, add a backend batch surface rather than a frontend fan-out loop. Preserve pair identity: a naïve skills × Tools Cartesian product is not a safe substitute for retrying exactly the failed pairs.

### S4 — Should-fix: mixed Deploy group silently skips work and fails to reload after enabling

**Location:** `src/hooks/useSkillActions.ts:526–538`.

For a group with both `toEnable` and `toSync`, but no effective sync targets, the early no-target warning is bypassed. Enable runs with `{}` handlers because `toSync.length > 0`, and the sync branch that would reload never runs. The enable mutation succeeds, but the catalog remains disabled on screen and the undeployed portion is silently ignored.

**Executed proof:** a read-only `renderHook` harness using the real door/reporter, a disabled skill plus a new skill and `effectiveSyncTargetIds: []` logged `['enable']` — neither `reload` nor `sync` — with `pending = null` afterwards.

**Fix:** determine runnable work before execution, report the no-target portion explicitly, and reload once after any mutating group phase regardless of whether a later phase is runnable. Add this case beside the existing group partition test. Consolidation for B3 should also remove the split-completion trap.

### S5 — Should-fix: the health fold offers an impossible repair for a stale imported/detached target

**Locations:** `src/lib/skillPresentation.ts:785`; `src/hooks/useSkillActions.ts:569–578`; `src-tauri/src/core/skill_update.rs:274–281`.

Every `targets_stale` row gets `repairs: ["update"]`, even for `source_type: "imported", refreshable: false` (including a detached skill). Update is acquisition from upstream; the backend refuses it with `NOT_REFRESHABLE`. The central bytes exist and can be re-deployed, but the fold's advertised repair cannot repair the stale copy. Missing local upstream has the same conceptual problem when central bytes remain usable.

**Test sketch:** extend the corpus with an imported/detached skill and one stale copy target. Assert its repair reaches a target re-sync, not `updateManagedSkill`. Simply changing the repair to `retryTargets` is insufficient today: `retry` only selects `error` and `missing`, and `repair` ignores `row.tools`.

**Fix:** distinguish central-to-target repair from upstream acquisition. Pass the row's affected Tools to a retry/sync path that includes stale targets, or extend the retry selection contract explicitly. Also cover disabled-at-skill + failed target: offering ordinary retry there only meets the disabled gate; repairs must respect the skill's current policy.

### S6 — Should-fix: Delete's “next visible row” is computed from the wrong order in grouped mode

**Locations:** `src/App.tsx:114`; `src/components/skills/SkillsList.tsx:95–120`; `src/lib/actionLifecycle.ts:focusAfter`.

The binder passes `lens.visible.map(...)`, which is the flat sorted order. The grouped renderer walks `lens.groups` and then each group's skills. These orders differ. The pure focus rule is correct for its input, but the input is not the visible order.

**Executed fixture repro:** failures scenario, Name A–Z, Group by repo enabled. The rendered neighbor after `store-hiccup` was `json-canvas`. Delete → Yes, Delete removed the skill and eventually focused `tdd` (`data-skill-row="skill-001-tdd"`), its next *flat alphabetical* neighbor. No source or live library was changed; grouping was restored afterwards.

**Fix:** derive one display-order sequence from `groups ? groups.flatMap(g => g.skills) : visible`, expose/use it consistently for rendering, selection navigation and focus. Add a pure/hook test with interleaved repositories. This is independent of the accepted ticket-11 heading fallback and ticket-14 overwrite-dialog return-focus work.

### S7 — Should-fix: a progress tick replaces “Cancelling…” before the command returns

**Locations:** `src/hooks/useStatusReporter.ts:509–523`; `src/components/skills/LoadingOverlay.tsx`.

`cancelPending` sets the line to Cancelling, but `reportProgress` unconditionally overwrites `actionMessage`, and the public `setActionMessage` used by other channels does too. The `cancelling` flag remains true, so the Cancel button stays disabled, but the progress surface can announce normal work instead of the promised persistent cancellation state.

**Executed proof:** pending Refresh → `cancelPending()` produced `lifecycle.cancelling / cancelling=true`; then `reportProgress({ done: 2, total: 5, message: "Applying beta" })` produced `Applying beta / cancelling=true` while the command was still unresolved.

**Fix:** derive cancellation copy from `pending.cancelling` at presentation time, or guard progress-line updates centrally while preserving numerical progress. Test a tick after cancellation, not only the immediate Cancel click. Keep the slot until the command returns, as it correctly does now.

### S8 — Should-fix: queued reads outlive their owner and replay stale UI intents

**Locations:** `src/hooks/useStatusReporter.ts:347,531–535,571–575`; `src/hooks/useExploreState.ts:handleOpenExploreDetail`; `src/hooks/useAddSkillFlow.ts:handleReviewImport`.

The read queue has no unmount disposal or validity check. When the active body finishes, all waiters wake; they run their captured callback even after the reporter has unmounted. A queued preview/import-review also retains its old “open this surface” callback if the operator has since navigated elsewhere. The current finite queue serializes correctly and I found no ordinary throw/dismiss deadlock; this is a stale-lifetime/intent problem, not evidence of a permanently refused mounted UI.

**Executed proof:** hold a mutation, queue a `read:true` callback, unmount the reporter, then release the mutation: the queued read body ran **once after unmount**. No cleanup rejects/discards it.

**Fix:** dispose/wake waiting reads on unmount without executing their bodies. Give navigational reads a cancellation/generation validity predicate (or let their caller discard stale results before opening a surface). Do not cancel already-settling mutations by merely hiding them. Add mutation→multiple-read ordering, unmount, navigation-change and rejection tests; today's sole queue test is read→read success.

### S9 — Should-fix: Add/picker still interpret Outcome completion themselves

**Locations:** `src/hooks/useAddSkillFlow.ts:finishInstall,364–365,420–425,464–467,537–540`; `src/hooks/useCandidatePick.ts:225–230`.

The one-line `publishOutcome → applyOutcome` conversion changed notification delivery, not completion ownership. Add still branches on `completion.reload`/`closeModal`; the picker resets/closes itself, calls `afterBatch(outcome.completion)`, then invokes the reporter without handlers. Single Add publishes via an unawaited `void applyOutcome(outcome)` *before* `finishInstall` reloads. Thus the claimed removal of hand-unpackers and the reporter's reload → notify/history → close rule are not actually in force for these callers.

**Fix:** pass reload/close handlers to one awaited `applyOutcome` and remove the caller's completion branches/afterBatch-Outcome protocol. Adapt form reset and picker/parent dismissal into those handlers, retaining the deliberate nonfatal reload policy. Coordinate targeted changes with 04. Test observable ordering for each Add entry, not just that a toast eventually appears.

## Coverage against the seven requested areas

| Area | Checked / result |
| --- | --- |
| 1. Pending/refusal under failure | Same-tick mutation refusal and `finally` on throw, explicit failure, cancellation and handoff are implemented/tested. Surface asks settle dismissed on Escape/displacement/unmount; an awaited dismissed ask can therefore finish the action. No normal permanently occupied slot found. B1 exposes bypasses/undisabled controls; S8 exposes queue lifetime. Pair shared-dir confirmation runs *before* `runAction`: a rejected confirmation promise would escape to a `void run(...)` caller without `notifyError`, although the current surface-backed implementation resolves rather than rejects. Add that defensive test. |
| 2. Door semantics | Update/Restore/Re-point and thrown sync reload; thrown Edit does not. Real catalog envelopes are replaced even on failed report rows. Delete/Edit/Re-point confirmation paths and pair shared-dir decline are covered. Failed first sync uses the ordinary sync seam with no blanket overwrite (same-content replacement only). Project verbs route to ticket 10's world. B2/B3/S4 remain. `"all"` means entire catalog, not lens; Refresh and new-Tools use all intentionally. Old toolbar `Uninstall from tool directories` is still reachable and calls unconfirmed `disable("all")`, even under a filtered lens; ticket 13 explicitly records its removal/count confirmation (comment committed in `7e8d8c1`), so not a new blocker here, but not release-ready. |
| 3. Health/source fold | Healthy deployed → zero rows; failed > stale > muted is coherent; disabled has its own kind; stored/live Edit conflict dedupes; import_failed only dismisses; delete_incomplete retries Delete. Source covers GitHub, other git, recorded local path, imported/detached no external copy value. Dismiss Rust core/command/registration/fixture agree; only Import family clears, another newer kind survives. S5 exposes missing cross-product coverage. The fold emits no positive synced health row for central-missing; the legacy card still draws active target pills from raw target status. Ticket 11's strip must eliminate that false positive instead of claiming the current renderer already does. |
| 4. Focus | Pure next/previous/heading table is sound; Delete passes the computed fallback into Dialog rather than implementing another modal focus trap. S6 breaks grouped order. Main-landmark fallback until ticket 11 is acceptable. Restore falls back to the row when its health node disappears, with inspector Health target due in 12. Only Delete has the domain fallback wired today; the generic `dialogFallback` helper is not consumed by all dialogs. Mid-action overwrite focus gap is expressly carried by ticket 14 and is nonblocking for this enabler, not a finished D21 implementation. |
| 5. Cancellation | Set matches **seven** current token-taking operations: installGitSelection, refreshManagedSkills, updateManagedSkill, repointSkillSource, cloneExploreSkill, previewRepointSource, installSelection. cancelCurrentOperation is the control, not another cancellable action. Sync/import/Delete/project operations do not declare Cancel. Slot stays occupied until return; `cancelLoading` is gone. S7 affects the lasting status copy; cancellation request rejection is swallowed and also lacks coverage. |
| 6. Standards | Per-skill callback drill is replaced with the stable door handle; world dependencies are type-only except owned building blocks (candidate picker, shared-dir confirmation, persisted preference, history), not cross-world runtime construction. No new raw native invocation in reviewed components; detail reads use the typed seam. App's bound confirmation handlers are appropriate. S9 means completion ownership is not fully consolidated. `loading` remains derived and threaded into Explore and many dialogs; the claim that all 18 sites disappeared is overstated. Deferred surface work explains retention, not deletion. |
| 7. Tests | 541 requested frontend tests pass, including 53 door tests. The verb table checks `spy.toHaveBeenCalled()` and dispatch `> 0`, not the actual report argument, exact call count or all confirmation sequences. Reload-on-throw compares behavior with exported `RELOAD_ON_THROW` (can agree with an incorrect table). Health tests cover all six kinds individually, not each kind × target state/provenance. Reporter lacks unmount, post-Cancel tick, queued navigation invalidation, rejected confirm and mixed mutation/read adversarial tests. Add the focused cases in each finding; no product-component JSX tests needed. |

## Verification — fresh commands and observed outcomes

Run in the repository root unless noted:

```text
npx vitest run src/hooks src/lib src/components/projects
  PASS — 22 files, 541 tests; duration 3.62 s.
  Only Node's existing experimental localStorage warning.

cd src-tauri && cargo test skill_health
  PASS — 19 tests, 770 filtered; main target 0 tests.

cd src-tauri && cargo test export_bindings
  PASS — 1 test, 788 filtered; main target 0 tests.

npx tsc -b --noEmit
  PASS — exit 0, no diagnostics.

shasum -a 256 src/bindings/index.ts   (before and after export_bindings)
  BOTH: 3c1dc65c846f260a88e7907ff4f692279edf2dccb375c1db6b1b6cf20da02c0a
```

The binding already differs from HEAD because this is uncommitted implementation; **regeneration introduced no additional byte change**.

Additional read-only execution: `node <<'NODE' … NODE` in-memory TS transpilation with the installed TypeScript JS API, jsdom and `@testing-library/react` `renderHook`, real `useSkillActions`/`useStatusReporter`, mocked native/notification seams. No test/source files written. Exact observed output:

```text
mixed group / no targets: ["enable"] pending= null
 after Cancel: lifecycle.cancelling true
 after next progress: Applying beta true
 queued read body invocations AFTER unmount: 1
```

The test setup/operations are specified in S4/S7/S8 for conversion into permanent regression tests. Browser verification used the already-running **fixture** `http://localhost:5175/?scenario=failures`, a dedicated T3 preview tab, each flow driven within `preview_evaluate`; inspected grouped Delete's settled focus and a refused second project-cell toggle. Grouping preference restored; review tab closed. No real library access.

**Goal-backward check:** an action door exists and the common happy paths pass, but it does not yet guarantee one mutation admission, one consolidated decision/report per operator action, or usable target-health repairs. Fix B1–B3 before accepting 09; retain explicit ownership for S4–S9 and the already-recorded 11/12/13/14 presentation follow-ups.

## Re-review — 2026-09-29

**Verdict: fix-then-ship — B2 and S4–S9 closed; B1 has a narrower blocked-control/picker remainder; B3's one-ask/one-run mechanics are fixed but its new combined fold loses outcomes (B3-R1).** Two remaining acceptance blockers, detailed below; no recurrence of 04's ownership or cancelled-install defects.

Reviewed **`df7cfc8` against `6d7b6f4`**, not HEAD or ticket 11's concurrent working files. Used a detached `/tmp/r19-recheck`, with shared dependencies/build cache but committed source and worktree-local binding export. All temporary reverts/probe assertions were restored, the clean worktree was removed, and main source was untouched. No nested agents, live Tauri/library run, commit, archive or move.

### B1 — OPEN in part: mutation admission fixed; Add's Browse and disabled reasons are unfinished

**Closed and pinned:** all ten tested project mutations now enter `runAction`; the real reporter/project-world hook tests hold a cell toggle and assert **zero other backend commands**, including registration, Edit ignore write, path update, configure, remove, both resyncs, another cell and row assign/unassign. `handleUpdatePath` also refuses before opening its picker. Matrix/list/Edit wire `blockedReason`; cell memo comparisons include it. This closes the original unsafe mutation bypass. No second mutation escaping the reporter was found.

**Remaining acceptance gap:** the “folder picker not opened while blocked” rule was implemented for Update path, but not Add Project's Browse:

- `src/components/projects/useProjectActions.ts:226–235`: `handlePickProjectFolder` calls `pickFolder` unconditionally.
- `src/components/projects/AddProjectModal.tsx:89–95`: Browse has no `disabled` condition; only Register uses `loading` (`:129`). A cell mutation has no batch overlay, and ProjectList still lets the operator open Add, so this is reachable during the held mutation.
- `ProjectsPage.tsx:234,254,274` passes only `loading` to Add/Tool config/Remove. Their disabled mutation buttons have no running-reason title or description. Edit and the matrix received the reason; these three dialogs did not. This is the remaining D19 “disabled **with the reason**” gap, not another claim of an unguarded disk write.

**Fresh failing probe:** add `["Add Project Browse while blocked", r => r.current.actions.handlePickProjectFolder()]` to the held-toggle table in `useProjectActions.test.ts`. Backend call count stays zero, but the existing `expect(pickFolder).not.toHaveBeenCalled()` fails: **1 call, `{ title: "projects.pathLabel" }`**. The committed ten cases do not exercise this entry point. Scratch-only assertion removed afterwards.

**Fix before acceptance:** gate Add's picker as well as Update path's; disable Browse while blocked. Pass the actual `blockedReason` to the project dialogs and expose it on their disabled submit controls. Preserve read-only browsing and the existing mutation admission rule. Extend the hook test with Browse and inspect the prop wiring (no product-component JSX tests required). This is a narrow finish, not a request to redesign the project world.

### B2 — CLOSED: every reviewed Update-family producer reaches the one ask

`useSkillActions.settleRefresh` routes Update/Restore/Re-point and Refresh-all; Edit passes its Propagation part. `settleCollisionParts` collects their global/project collisions before asking. `settleClaims` retains `(scope, project?, skill, Tool)` identity, uses exact Tool-set sync batches and one project decision list, and merges only answered pairs. Keep becomes a quiet declined result; retry failure stays a failure; dismissal returns the original failures with no retry.

**Tests vs path:** the real-sync-world door tests cover Update/Restore/Re-point with a global re-assert and a project Propagation collision, assert one ask, exact request payloads, catalog and project refresh, no stale target-error entry, one run and cleared pending. Separate tests cover dismissed Update, Refresh-all Keep and Edit. Pure adapters cover the report merges/declined notes. I added temporary call-order assertions to the three parameterized cases: **sync/project retries < `getManagedSkills` < `refreshView("p1")` < `refreshOutcome`**; all **3 passed**. The permanent tests pin the calls but not that full order; retaining those ordering assertions would strengthen them, not fix a current code bug. The shared `settleParts` orders the reads before Edit/Refresh folding too.

The successful project-row elision is judged below, not mistaken for an unhandled collision. If a read/retry throws, `runAction` still frees pending. No normal dismissed-ask/pending leak or queue/`uncancellable` deadlock was found.

### B3 — mechanics CLOSED; B3-R1 OPEN: the consolidation drops Enable's summary and outcome tone/counts

**What is fixed and pinned:** Deploy collects Enable and first-sync reports before one ask; multi-Tool Retry groups exact Tool sets before one ask; multi-skill project assignment collects every per-skill report before calling `settleCollisions` once. The door calls `applyOutcome` once on each of these paths. Real door tests assert one ask and one run for Deploy/Retry; the project-world test pins one merged settlement and one view read, and the door's project case folds/applies that returned report once. The latter test is a mocked settlement seam, not a full native-project-dialog test.

**New blocking regression introduced by this fix:** `src/hooks/useSkillActions.ts:617–621` combines the sync fold first and Enable second. `src/lib/reportOutcome.ts:997–1024` retains **only the first toast's message**, promotes its tone only for another part's errors, and copies only the other part's notes/entries. An Enable success exists only in its toast, so it vanishes. An Enable decline has `toast: null` plus a note; its declined count/tone vanish. An Enable failure retains its error row but is absent from the composite headline's failure count. One `recordRun` is not proof of one **complete** report (D10/D11).

**Executed reproduction using the real pure folds** (the same composition as the door): fresh sync has one synced target; Enable has one target whose status varies. `combineOutcomes(syncOutcome(fresh, { action: "bulk", t }), enableOutcome(enabled, { t }))` produced:

| Enable result | Actual combined headline / tone | Missing information |
| --- | --- | --- |
| one synced target | `1 synced`, **success**, no detail | The enabled target/success summary disappears entirely. |
| one `declined` target | `1 synced`, **success**, kept-folder note only in detail | No declined count; success tone instead of declined. |
| one failed target | `1 synced`, failure/warning; error row retained | No failure count in the composite headline. |

This also affects the existing two-collision Deploy test: its first-pass sync covers two targets and Enable covers one, but asserting only one run/no errors cannot detect the missing Enable result.

**Fresh failing regression probe:** in `reportOutcome.test.ts`, compose a one-target successful `syncOutcome` and a one-target declined `enableOutcome`; assert `combined.toast.tone === "declined"` and that its headline includes the declined count. The first assertion fails: **expected `declined`, received `success`**. The committed `combineOutcomes` tests only pin “first toast wins, errors retone, notes survive”; they encode the lossy strategy rather than the complete operator-action contract. Scratch probe removed afterwards.

**Fix before acceptance:** fold the full Deploy action once from typed settled reports/counts (or retain structured counts/tone in Outcomes before combining), preserving units for global/project targets and Enable. Emit one composite headline with all done/failed/skipped/declined contributions, then choose tone across all contributions; keep all errors/warnings/notes and one run. Do not parse localized toast strings or merely append a second success toast. Add pure tests for Enable success/decline/skip/failure + fresh-sync success, and strengthen the real Deploy test to assert report contents/counts, not only the run-id cardinality.

**04's changed retry test is sound:** it now requires exact `s1/cursor` and `s2/claude` batches rather than replaying the four-pair product, while a separate test preserves one batch for identical Tool sets. That is a safety improvement, not weakening the one-ask/one-run guarantee. The additional IPCs are an explicit API compromise, not invisible fan-out correctness.

### Should-fix closure

| Finding | Rechecked result and evidence |
| --- | --- |
| **S4 — CLOSED** | Mixed group/no targets computes runnable work first, enables the disabled part, records the undeployed warning and reloads. Dedicated door test asserts Enable, warning and one reload. B3-R1 separately covers truthful aggregation when both parts run. |
| **S5 — CLOSED** | Failed/missing/stale repairs map to `retryTargets`, pass `row.tools`, and Retry includes stale. Imported/detached/local/git corpus and the real-sync imported-stale test pin central-to-target sync, zero upstream Update calls. Skill-grain Disable offers finishing Disable, not futile ordinary retry. |
| **S6 — CLOSED** | `displayOrder` → `lens.displayed` → App's `order`; grouped interleaved-repo test pins alpha → gamma, not beta. Verified against the commit, not ticket 11's working renderer. |
| **S7 — CLOSED** | Reporter exposes cancellation copy from `pending.cancelling`; both streamed progress and direct `setActionMessage` leave it intact while the numerical count advances. Dedicated real-reporter test passes. |
| **S8 — CLOSED for the reported lifetime/navigation failures** | Cleanup wakes queued reads without running their bodies; `stillWanted` is checked on admission. Four reporter tests cover ordered mutation→reads, unmount, invalid intent and rejection; Explore's tests cover generation/area before and after the request. App supplies live area predicates to Explore/import review. `uncancellable` does not release the mutation slot or wake the queue. |
| **S9 — CLOSED** | Add/picker/import now await `applyOutcome` with handlers; picker completion no longer goes through `afterBatch`. Tests assert completion true/false and reload→notification→close ordering. The real reporter's executor tests pin its half independently of the Add test doubles. |

The rejected shared-dir confirmation is also caught, reported and treated as “no”; its new test asserts zero mutation commands. 04-F2's real-reporter cancellation tests are green and failed when the action-boundary checks were removed in the scratch worktree. 04-F1's admission test likewise fails without its fix; the failed-removal ownership guard remains intact.

### Four command-API compromises — release judgments

These are **acceptable for 1.3.0 with a BACKLOG note**, with the restrictions below; none excuses B1 or B3-R1. The queue is owned by the parent, so this review records the proposed follow-up here/in the ticket rather than editing a fifth file.

1. **Per-skill assign/unassign commands — accept bounded debt.** The outer action admission spans the loop; target fan-out remains backend-owned inside each command; asks/folds are consolidated. A later multi-skill command should carry settled partial reports even if a later skill has a whole-command/store failure. Do not claim the one-command-per-action ideal is currently satisfied.
2. **Enable + sync as two commands — accept the API compromise.** Partitions are disjoint and remain in one lifecycle action; no atomic cross-partition transaction was promised. **Block the current lossy fold**, not shipment on a new combined command. Record a follow-up for the combined backend surface.
3. **Successful answered project row elided from Update/Edit report — accept temporarily.** `resolveProjectCollisions` does not return a sync mode; synthesizing one would be false. These current folds count target failures, while actual deployment/ownership comes from the re-read catalog/project views. Failed retries and explicit Keeps are retained as error/declined data. Follow up with a lossless typed settled-target representation before any consumer treats the shortened array as a complete target audit.
4. **Enable retries the same scope — accept with a narrowing follow-up.** `enable_skill` reselects remaining **disabled** rows. First-pass successes are synced; project collisions are now error/non-owner and excluded from that retry, then resolved by exact project decisions. A pair-grain retry is already its one Tool/shared-directory scope. This does not reproduce the old foreign-folder ownership bug. Prefer a dedicated answered-pair Enable API eventually; preserve tests that unrelated completed pairs stay untouched.

### Fresh verification and cleanup

From `/tmp/r19-recheck` at `df7cfc8`, run initially and again **after restoring all scratch changes**:

```text
cd src-tauri && cargo test install_batch && cargo test skill_disable && cargo test project_sync && cargo test sync_engine && cargo test export_bindings
  PASS — 10 / 33 / 36 / 8 / 1 tests.
npx vitest run src/hooks src/lib src/components/projects src/fixtures
  PASS — 25 files / 664 tests (final run 4.35 s).
```

Rust used `CARGO_TARGET_DIR=/Users/alexstark/Projects/skills-hub/src-tauri/target`; the node_modules symlink was dependency reuse only. Extra commands:

```text
cd src-tauri && cargo test the_collision_fact_reads_the_stored_identity_like_the_policy && cargo test target_relocation
  PASS — 1 / 14 tests.
npx vitest run src/hooks/useSkillActions.test.ts -t 'answered pairs are retried'
  PASS — 3 tests with temporary retry/read/fold call-order assertions.
npx vitest run src/components/projects/useProjectActions.test.ts src/lib/reportOutcome.test.ts -t RECHECK
  EXPECTED FAIL — the two added residual probes above: picker called once; success instead of declined tone.
bun run - <<'TS' … TS
  Real syncOutcome/enableOutcome/combineOutcomes probe: the three results in B3-R1's table.
```

Sensitivity commands/results for 04's fixes are recorded in `ticket-04-review.md` § Re-review: **F1 and F5 each fail their committed Rust regression without the fix; F2 fails both git/local tests without its boundary checks**. These are expected negative-control failures, distinct from the two residual probes which fail on the unmodified reviewed implementation.

`git diff --exit-code df7cfc8 -- src/bindings/index.ts` passed after export. SHA-256: `3c1dc65c846f260a88e7907ff4f692279edf2dccb375c1db6b1b6cf20da02c0a`. Scratch worktree diff/status were clean before removal; `git worktree list` afterwards lists only the main checkout. No full release gate or new visual audit claimed.

**Goal-backward check:** one mutation admission and one consolidated ask are now real. Finish the blocked picker/reason contract and preserve every Deploy outcome in its single report before accepting ticket 09. No passed area needs reopening for those fixes.
