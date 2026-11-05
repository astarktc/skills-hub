# Round 19 closure — extract slice A

Slice **A**, walked 2026-11-05 on `main` @ `0f5c2d3`: `.scratch/round19/spec.md` (whole file: D1–D51 incl. the 19b grill, the
D31 amendment, ticket tables, Execution, 19a Closure block), `issues/01`–`06`, `review/ticket-01-review.md` (review +
re-review), `review/ticket-04-review.md` (review + re-review). Every "left" line re-derived from code opened in this walk.

## 1. Carry — real, still open, not in BACKLOG

| id | title | source | current evidence | proposed home | why it matters |
|---|---|---|---|---|---|
| A1 | Disable that only records intent toasts "Nothing was removed" | `issues/01-…:263` ("A Disable that only flipped markers reports no row") | `skill_disable.rs:205–224` writes a `DisabledRecorded` marker but adds nothing to the `RemovalReport`; `artifact_removal.rs:645–655` flips existing non-owner rows `disabled` under `Settlement::Disable` without pushing them to `cleared_markers` (Delete only); `reportOutcome.ts:677–683` → `nothingPlanned` → skipped-tone toast `unsyncNothingPlanned` ("Nothing was removed — no deployment could be located", `resources.ts:396`). Applies to a pair with no row and to a failed-first-sync `error` row (D9). Not driven in the fixture | later | A Disable that worked (row now `disabled`, strip shows it) reads as a skipped no-op |
| A2 | Stale `useOverwriteConfirmation` references after its deletion | `issues/04-…:185` | `src/components/ui/confirm-dialog.tsx:18` and `docs/adr/0006-styling-architecture.md:106` still name `useOverwriteConfirmation` and say it "resolves a boolean"; the hook is gone (`rg` finds no definition), and the ask is now `useSyncOrchestration.requestOverwriteConfirmation` → `OverwriteChoice[] \| "dismissed"` (`useSyncOrchestration.ts:52–62`) | now (doc comment + ADR amendment line) | Agents read the ADR as the contract and will look for a hook that does not exist |
| A3 | Install-batch and collision-resolver test gaps | `review/ticket-04-review.md:89, :93` (§3, §7), `:92` (§6 stale-decision) | `core/tests/install_batch.rs:427–439` `a_git_source_installs_through_the_listing_resolution` passes `resolution: None` (no branch/subpath resolution exercised); no same-content first-pass or extraneous-override batch test (the file's 11 tests, `:123–:460`); `resolve_collisions`' owned-row → `AlreadyAssigned`, gone-row → `NotFound`, disabled → `Skipped` arms (`project_sync.rs` ~:907–947) are tested only via the two collision arrangements (`skill_disable.rs:1374`) and the missing-folder case (`project_sync.rs:1913`); no unknown-mtime/vanished-occupant test (`rg "modified_at: None"` in tests: none) | later | The retry doors are the overwrite path; their refusal guards are what keep a stale decision from writing over an owned folder |
| A4 | Fixture `resolveProjectCollisions` throws on a missing project | `review/ticket-04-review.md:92` | `src/fixtures/backend.ts:1708–1709` calls `findProject(decision.project_id)` (throws) before the `NOT_FOUND` row at `:1711–1719`; Rust answers per decision (`project_sync.rs` ~:907–925 `NotFound` row) | later | Fixture parity (D29): a removed project mid-ask fails the whole fixture call where the app reports a row |

## 2. Covered — already a BACKLOG item

| id | title | source | # | amend? |
|---|---|---|---|---|
| A5 | Four command-API compromises (multi-skill project assign, Enable+sync two commands, dropped answered project row, Enable retry re-runs scope) | `issues/04-…:291`; `review/ticket-04-review.md:254–262` | #64 | no |
| A6 | Overwrite ask without content diff ("a reader feature for #26") | `spec.md:93` (D12) | #70 | no (#70 already asks for content view/diff) |
| A7 | Notification history is session-only | `spec.md:83–87` (D11 "History stays session-only"), D9 "No run-log table" `spec.md:72` | #74 | no |
| A8 | Matrix owns no selection (D4 names the matrix as a selection surface) | `spec.md:48` | #86 | no |
| A9 | Ticket 34 parked: registry truth, AGENTS-dir preference (D47 "waits for #67/#45") | `spec.md:229–231`, ticket table row 34 | #88, #89 | no |
| A10 | Per-harness option home (D47's per-harness rows) for a copy override | `spec.md:229` | #84 | no |

## 3. Closed — fixed since

| id | title | source | closed by | code evidence |
|---|---|---|---|---|
| A11 | Review-01 F1–F7 + re-review F1-R1 + N1 (marker ownership, `disabled_at` gates, shared-dir marker, fixture presence, Deploy/fold fixes) | `review/ticket-01-review.md` F1–F7, F1-R1, N1 | 01 — `79a31bf` | `skill_store::owns_artifact` (`synced_at`); `artifact_removal.rs:645–655` sets non-owners aside; `skill_disable.rs:212` marker admitted only when `failed_rows()==0`; `reportOutcome.ts:677` counts `cleared_markers` |
| A12 | Failed Enable leaves no global row | `issues/01-…:246`; review-01 "accepted deferrals" | 02 — `607ec96` | test `an_enable_meeting_a_folder_in_the_way_keeps_the_pair_disabled`; `TargetTransition::FailedRecorded` for other failures |
| A13 | Enable `TARGET_EXISTS` bypasses the ask; project collisions have no Keep/Overwrite | `issues/01-…:247–248`; `issues/02-…:182`; review-01 deferrals | 04 — `302bbb1` | `commands/mod.rs:613` `enable_skill_targets(scope, overrides)`; `resolve_project_collisions` → `project_sync::resolve_collisions` |
| A14 | Onboarding plan hides an unmanaged copy at a disabled/marker path; rename to "owned" | `issues/01-…:249`; review-01 § Onboarding-path exclusion; `issues/02-…:189` | 02 + 05/21 | `skill_store.rs:1120` `list_owned_skill_target_paths` |
| A15 | Project-sync collision surfaced as `OTHER` | `issues/01-…:260` | 02 | `errors.rs:475` downcasts `sync_engine::TargetExistsError` |
| A16 | Strip has no own *disabled* mark; matrix disabled cell unstyled | `issues/01-…:250` | 11 / 17 | `StripMarkIcon.tsx:9,24,47` (slashed = disabled) |
| A17 | Library-wide Disable unconfirmed; "unsync" copy | `issues/01-…:252–254` | 13, 32 | `rg "Uninstall from tool"` → none; smoke item 6 (count confirm) in issue 19; `resources.ts:323` "Removing {{name}} from {{tool}}", `:396` (keys keep code names per D45) |
| A18 | Fixture had no project first-sync collision model | review-01 coverage row 8 | 01 F1-R1 | `backend.ts` `projectOccupants`; `resolveCollision` `:1708–1730` |
| A19 | Displaced/dropped ask recorded Keep (needs a third answer) | `issues/02-…:180` | 08 / 04 | `useSyncOrchestration.ts:52–62` `OverwriteAnswer = … \| "dismissed"`, `:510` dismissed → no retry |
| A20 | `import_failed` / `delete_incomplete` have no clearing path | `issues/02-…:184` | 09 | `useSkillActions.ts:712` `dismissHealth`; `core/tests/skill_health.rs:199–208` `dismiss_outcome` |
| A21 | `deployAction` counts a failed-first-sync `error` row as deployed | `issues/02-…:186`; review-01 F5 note (`missing`/`pending`) | 09/11 | `skillPresentation.ts:574–577` `isLiveTarget` excludes `disabled` and never-synced `error` |
| A22 | Review-04 F1–F5 (project Enable ownership admission, Cancel during discovery, stored identity in collision facts, fixture double report, cancel while waiting for guard) | `review/ticket-04-review.md` F1–F5 | 04 — `df7cfc8` (→ `302bbb1`) | `AssignmentTransition::EnableAdmitted`; `Occupant::observe(identity: Source)`; `install_batch` in-guard token check; re-review § Finding closure |
| A23 | Cross-product (skills × Tools) collision retry | `review/ticket-04-review.md:88` | 09-B2 | re-review `:250–253`: exact pair batches, retry → catalog → view → fold |
| A24 | Lossy combined Deploy report (09 B3-R1, named in 04's re-review) | `review/ticket-04-review.md:262` | 09 residual fix | `reportOutcome.ts:129–136` `OutcomeTally`, `:1041` enable tally |
| A25 | Delete `install_git_selection` / `install_local_selection` / `InstallResultDto` | `issues/04-…:178–183` | 16 | `rg` in `src-tauri/src/commands` and `src`: none; only `#[cfg(test)] installer.rs:428` helper remains |
| A26 | Import Replace / divergent-sibling rows not routed into the one ask (`handleImport` folds directly) | `issues/04-…:183–185`; review-04 §5 `:91`; `issues/05-…:132–142` | 21 — `e6be057` | `useImportReview.ts:187–201` `settleCollisionParts` over `imported`/`replaced` `TARGET_EXISTS` rows |
| A27 | Import review UI: invalid entries with reasons, Replace/Leave, remove-originals checkbox, counts | `issues/05-…:132–142` | 21 | `useImportReview.ts:75,343` `removeOriginals`; `ImportReviewDialog.tsx:248` `invalidReason` |
| A28 | Refresh re-assert collisions not routed into the ask | spec D12 `spec.md:88–92`; review-04 §5 | 33 — `948f12b` (re-assert deleted, D34) | `rg "reassert_auto_sync\|RefreshPolicy" src-tauri/src` → none |
| A29 | Re-point target cannot carry a picked candidate on a default-branch repo | `issues/06-…:79–80` | 12 | `repoint.rs:64–84` `RepointTarget::Git { url, selection: Option<RepointSelection> }` |
| A30 | Lint warning `useStatusReporter.ts` `idleWaitersRef` cleanup | `issues/04-…:267` | 09 follow-up | `npx eslint src/hooks/useStatusReporter.ts` → no output |
| A31 | D1 palette `cmdk` vs Combobox; D5 `#/settings/<pane>`; D35 `delete_incomplete` skip; D48 add-harness ask; D22 grid toggle; D23 Projects header select | `spec.md:42, :50, :191, :232, D22, :134` | 23, 20, 33, 20, 12, 17 | `shell/CommandPalette.tsx`; Settings is a dialog (AGENTS.md); `refresh.rs:410–415`; `resources.ts:1591`; `inspector/TargetsSection.tsx:156–159`; `ProjectsPage.tsx` uses `SHELL_WIDE_QUERY` |
| A32 | 19a closure: "8 on the work Mac pending" | `spec.md:332` | 26 smoke 2026-11-05 | issue 19 smoke table row 8 "Work Mac ✅ 2026-11-05"; issue 26 Comments 2026-11-05 |
| A33 | D31 operator VoiceOver/reduced-motion smoke per release | `spec.md:162–168` | amended 2026-11-05 (`19e896c`) | `spec.md:166` amendment; AGENTS.md:53–55 |

## 4. Drop candidates

| id | title | source | reason to drop |
|---|---|---|---|
| A34 | `disabled_skill_survives_each_byte_adapter` test (review sketch 6) | `issues/01-…:308`; `review/ticket-01-review.md:116` | Declined in the ticket by name: the gates it would pin (Propagation, global batch) are covered (`a_disabled_skill_is_never_rematerialised_by_any_automatic_pass`, `a_disabled_pair_survives_refresh_reassert_and_propagation`); D34 removed the re-assert path |
| A35 | No Rust test for the `import_failed` settlement writer | `issues/02-…:170` | Shared `note_outcome` path (`onboarding_import.rs:489–492`) is the same writer as tested kinds; fixture test covers it; needs a permission fixture for little gain |
| A36 | Guard tests are timing-based (300 ms / 150 ms hold, no start barrier) | `review/ticket-01-review.md:283`; `review/ticket-04-review.md:244` | Both reviewers judged non-blocking and proved sensitivity; entry points wrap `serialized` directly (`mutation_guard.rs:105–130` helper) |
| A37 | `missing` / `pending` global rows count as deployed | `review/ticket-01-review.md:225` | No writer creates never-synced `missing`/`pending` global rows; a `missing` row was materialised (`synced_at` set), and health shows it |
| A38 | Unknown/uninstalled Tool precedence (`TOOL_NOT_INSTALLED` before Disabled) | `review/ticket-01-review.md:91` | Pre-existing, reviewer recorded as no finding |
| A39 | Binder growth (arch candidate 13; `App.tsx` 981 lines) | `spec.md:55–58` (D7) | Decided by D7: "a symptom, gets no ticket" |
| A40 | Spec D33 model IDs `openai-codex/…` | `spec.md:173–175` | Historical record; AGENTS.md § Delegation is the live routing (and forbids `openai-codex`) |
| A41 | D19 "for 1.3.0 one mutation at a time … not queued" read as a deferred queue | `spec.md:117–121` | Now a standing invariant (AGENTS.md "one mutation at a time (D19) … refused … never queued"; `useStatusReporter.ts:313,342`). See operator decision 1 |
| A42 | Rounds 16 (ticket 08, Windows) and 17 (closure smoke) "stay live awaiting the operator" | `spec.md:10–11` | Not round-19 residue: `.scratch/round16/` and `.scratch/round17/` are still live with `Status: open`; their own closure owns them (flag to orchestrator) |

## 5. Operator decisions

1. Is D19's single-mutation rule permanent, or is a mutation queue a future want? — **Permanent; drop A41 by name.**
   AGENTS.md already states it as an invariant, and refusing with a reason (ticket 27) answered the think-aloud's complaint.
2. Should a Disable that only records intent (A1) read as success ("Pico stays off Cursor") rather than a skipped
   no-op? — **Yes, carry A1 to Later.** The operator's "not on this Tool" got recorded, and a skipped tone says the
   opposite. The fix belongs in the fold, plus a report field for flipped markers.

---
Counts: Carry 4 (A1–A4) · Covered 6 (A5–A10) · Closed 23 (A11–A33) · Drop 9 (A34–A42) · Operator decisions 2.
Dangerous: none open from this slice. Every P0/blocking finding in reviews 01 and 04 (data-loss ownership class
F1/F1-R1/04-F1) is closed with sensitive regressions. The live P0 in this slice's orbit is #89 (lexical launch relocation, via parked ticket 34), and it is already in BACKLOG.
Also flagged: `spec.md` is still `Status: open` with only the 19a closure block. The final closure block, covering 19b and 34-parked, is the orchestrator's.
