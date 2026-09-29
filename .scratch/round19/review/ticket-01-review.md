fix-then-ship

# Ticket 01 adversarial review — 2026-09-29

Reviewed the **uncommitted** ticket-01 delta on `main` at `553db2a`, including both new `skill_disable` files. Ticket 07 styling changes, including `tauri.conf.json`, are excluded. No source files edited; the explicitly requested binding export reproduced the existing bytes. Review is a terminal leaf, not a further delegation.

**Three blocking findings.** The ordinary disable/enable paths work, but the invariant fails at the boundary between intent-only markers and real artifacts, and after a partial skill-level Disable. Green tests do not cover these combinations.

## Findings, ranked

### 1. Blocking — a never-deployed marker gives Delete ownership of an unmanaged folder

**Locations:** `src-tauri/src/core/skill_disable.rs:170–188`; `src-tauri/src/core/artifact_removal.rs:288–291,543–549`.

`disable_pair` inserts a normal target row pointing at `skills_dir/name` without checking that path or ever having materialised it. Delete then plans **every** target row and removes the recorded path. The marker has invented artifact ownership, not just recorded a preference.

**Failing Rust-test sketch:** install managed `alpha` centrally, never sync it to Claude; independently create `.claude/skills/alpha/SKILL.md` with foreign bytes. Call pair Disable, then `artifact_removal::remove_skill`. Disable returns an empty successful report and records `disabled` while the foreign folder is still there; Delete removes that foreign folder. The same problem arises if a foreign folder arrives after the marker was recorded. Assert the unmanaged bytes survive Delete: current control flow fails that assertion. This is source-traced, not a filesystem reproduction run against the operator's library.

**Fix:** distinguish intent-only targets from artifacts the app owns. Removal planning must discard an intent-only marker without deleting its putative path; only a successful first sync establishes artifact ownership. Do not infer ownership from `status == disabled`, since formerly deployed rows have that status too, or blindly delete the foreign folder during Disable. Pin marker → foreign folder → Delete, and real deployment → Disable → Delete separately. This also makes D9's future “Keep existing records disabled” safe.

### 2. Blocking — project resync and startup relocation ignore `skills.disabled_at` on failed-removal rows

**Locations:** `src-tauri/src/core/project_sync.rs:590–604`; `src-tauri/src/core/target_relocation.rs:76–98`; compare `src-tauri/src/core/skill_disable.rs:200–229`.

Skill Disable deliberately sets `disabled_at` **even when removal fails**, retaining affected rows as `error`. Both readers above gate only on row status `disabled`. Consequently a subsequent project resync can sync that `error` assignment, and startup relocation can materialise an `error` global target in its new directory, while the skill remains “off everywhere until enabled.” Global sync and Propagation correctly consult the skill marker; these two do not.

**Reproduction / test sketches:**
- Assign `alpha` to a project, make the target parent non-writable, disable the skill, then restore permissions and remove the artifact externally. `resync_project` (also reached by resync-all) recreates it and reports `Synced` without Enable. Expect `Skipped { Disabled }`, the failure row preserved, and no artifact. An in-memory fixture reproduction of the same failed-Disable → resync sequence actually returned `synced` with `disabled_at = 1000` (Probe A).
- Seed an Augment row in its former `.augment/rules` directory, force skill Disable's removal to fail, leave the former artifact present, then call relocation. It writes `.augment/skills/alpha` because the row is `error`, not `disabled`. Expect no materialisation while the skill marker exists.

**Fix:** gate project resync and relocation on the skill-level marker as well as row status, before filesystem work. Cache the marker per skill for the batch. Preserve `error` and `last_error` after failed removal; report a disabled skip without falsely settling removal. Also make bulk assignment of an existing `error` row on a disabled skill report the disabled reason rather than `AlreadyAssigned`. Mirror the gate in fixture `resyncOne`.

### 3. Blocking — shared-dir removal failure still creates a `disabled` sibling marker

**Location:** `src-tauri/src/core/skill_disable.rs:170–188` (fixture mirror `src/fixtures/backend.ts:551–569`).

The post-removal guard checks whether the **requested Tool** has a row, not whether the shared artifact was removed. A failure may have left only a sibling's row as `error`.

**Reproduction:** sync `alpha` while only Kimi is installed, then install Amp; both use `.config/agents/skills`. Make that parent non-writable and Disable Amp, which still has no row. Removal fails and keeps Kimi `error`; the requested Amp row remains absent, so the code inserts Amp `disabled` for the same still-present artifact. Probe A reproduces the analogous fixture state: `failed`, then `[kimi_cli:error, amp:disabled]`.

**Impact:** the pill asserts disabled while the Tool can still read the skill. Propagation's shared-dir gate then skips the entire group because it sees the newly inserted marker, suppressing the actual sibling failure on subsequent updates.

**Fix:** make marker admission depend on settlement of the affected shared artifact, not named-row absence alone. On failure do not insert a disabled sibling; retain/report the error state. Add a Rust test with only one existing shared-dir row and the requested sibling absent, including a retry after restoring permissions. Existing shared-dir tests cover a missing sibling with no artifact, not this failure combination.

### 4. Should-fix — the fixture turns a second failed Disable into success without removal

**Location:** `src/fixtures/backend.ts:401–411`.

Assignment artifact presence is inferred from `synced | stale`. A first failed Disable changes the assignment to `error`; a second Disable now decides the artifact is absent, even with the same locked/unwritable path, and flips the row to `disabled`.

**Executed reproduction:** Probe B calls skill Disable twice on `tdd`, with quartermaster × Claude remaining unwritable throughout. First call reports that assignment failed and retains `error`; second reports removed and stores `disabled`. Rust `symlink_metadata` would still see the artifact and fail again.

**Fix:** model assignment artifact presence independently of lifecycle status, retaining it on failed removal. Repeated Disable must keep reporting failure until that modeled artifact is actually gone/removable. Use the same independent presence model for Delete; global disabled markers currently also inherit an “every row means artifact exists” removal assumption.

### 5. Should-fix — the card offers Deploy after pair-disabling every target, but the handler cannot enable them

**Locations:** `src/components/skills/SkillCard.tsx:127–129,330–341`; `src/hooks/useSkillLibrary.ts:261–283`.

The new `deployed` predicate is false when all target rows are disabled, regardless of grain. The handler takes the Enable branch only when `disabled_at` is non-null. If the operator disabled the individual pills instead, the offered Deploy calls ordinary sync, which correctly skips those pairs; `syncOutcome` then emits the bulk success toast.

**Failing hook-test sketch:** one skill with a disabled Claude row, `disabled_at: null`, effective Tools `[claude_code]`. Invoke the card's deploy handler. Current path calls the sync seam, gets `TARGET_DISABLED`, retains the disabled target, and says sync completed. Expect an explicit Enable action to restore the target, or an affordance that accurately explains that ordinary Sync preserves disabled pairs.

**Fix:** align the card action and its label with the two grains. Route an explicitly offered Enable through `enableSkillTargets`, or retain ordinary Sync but do not present it as enabling already-disabled rows. Add a hook test with a real fold rather than only the invoke→mocked-fold table.

### 6. Should-fix — `importOutcome` still classifies `TARGET_DISABLED` skips as errors

**Location:** `src/lib/reportOutcome.ts:565–580`.

Only `synced` and `TOOL_NOT_INSTALLED` skip this error collector. A `skipped/TARGET_DISABLED` target becomes an `errors.syncFailedTitle` entry and a partial-failure toast, contrary to the new skip contract.

**Executed reproduction:** Probe A feeds one imported group with one `TARGET_DISABLED` skipped target to this pure fold. It returns one error and `partialFailure`.

**Fix:** handle this typed skip here, or share the skip policy with the sync fold; add a table test across sync/install/import/refresh/Edit/enable consumers. This is not blocking today because the current onboarding import finalizes a new skill whose name must be free, so it cannot ordinarily have its own disabled row. That reachability fact does not make the fold exhaustive or the requested “every report kind” coverage pass.

### 7. Should-fix — project bulk assignment counts skips internally but reports “Already assigned” for newly recorded disabled assignments

**Location:** `src/lib/reportOutcome.ts:343–360`.

`counts.skipped` is read only by toggle-on. When bulk assign creates disabled assignment rows, `synced == 0` selects `projects.bulkAssignNothing` (“Already assigned to every configured tool”). The user has just made new assignments, not repeated an existing assignment. Resync summaries likewise omit the skipped count.

**Executed reproduction:** Probe A's one-item skipped project report returns the success toast `projects.bulkAssignNothing`.

**Fix:** surface disabled/skipped counts for bulk and resync, using accurate “assigned but not synced” copy for new disabled assignments. Do not count these as failures. Test all-skipped and mixed synced/skipped/failed reports. The broader composite headline work can remain ticket 03; this minimum fold must not say the newly performed operation was already done.

## Coverage of the nine requested attack areas

| Area | Verified and evidence | Assessment |
|---|---|---|
| 1. Settlement / shared dirs / marker rows | Traced removal planner → `execute_settling_unlocked` → typed transitions. Ordinary rows settle disabled only after `remove_path_any` succeeds; failure writes diagnostic then `error`. Existing shared rows dedupe by path. Markers record current registry path, capability-derived mode, null `synced_at`; `(skill_id, tool)` uniqueness and Recorded upsert prevent duplicate real-sync rows. | **Findings 1 and 3.** Marker ownership is not equivalent to ordinary target ownership. Group failure cannot be certified by checking the requested row alone. |
| 2. Mutation guard | Both new entry points directly wrap their body in `serialized`. Disable composes read-only planning and unlocked removal. Enable uses `sync_skills_to_tools_unlocked` / `first_sync_assignments_unlocked`, then `sync_single_assignment` / `sync_assignment_target`, none taking the guard. Full tests terminate. | No nested-lock path found. Missing direct guard-contention tests for the two new entry points. |
| 3. Skip vs failure | Read global `DisabledGate`, Propagation's skill/group/assignment gates, reassert mapping, project assign/resync/toggle and relocation. Non-disabled `TOOL_NOT_WRITABLE` batch skips still map to reassert failures. Fixture test pins noisy-linter × Amp separately and passes. | **Finding 2.** Otherwise skips are correctly typed. `TARGET_DISABLED` fits the existing `BatchTargetStatus::Skipped { error: CommandError }` contract; its construction is report data, not a classified error then `?`-propagated. Unknown/uninstalled Tool handling remains a separate pre-existing precedence (an absent Tool gets `TOOL_NOT_INSTALLED`). |
| 4. Skill-level lifecycle | Skill Disable visits global rows and ProjectSkill scopes and records marker; Enable clears marker and first-syncs disabled rows. `SkillRecord` upsert omits the marker, so finalize and rollback cannot clear it. Update's acquired/local/Edit branches all end at gated Propagation; reassert uses the batch gate. | Successful paths covered; partial-disable paths fail finding 2. Delete of true disabled deployments follows normal removal; intent-only markers fail finding 1. Specific disabled Re-point/Restore/Edit/Delete integration tests are absent. |
| 5. Migration / strings / aggregate | Read fresh and incremental schema paths, v9→v10/partial-upgrade test, upsert, legacy DB adoption and startup ordering. Legacy DB is copied before `ensure_schema`. Fresh migration is transactional; next call sees v10. V9 step checks column presence. Typed status roundtrips and aggregate table pass; store aggregate calls the same fold. | No ticket-01 migration defect found. V9 fixture represents 1.2.18 schema; no live operator DB opened. Disabled rows are excluded from status precedence, not assignment/skill counts (appropriate for retained assignments). |
| 6. Wire | Read commands, collect list, generated types and shims. Three old commands removed; new commands annotated and registered. DTO `disabled_at` is required nullable number, target/assignment status union includes disabled. Fixture handler exhaustiveness test passes. | `cargo test export_bindings` passes; hash and diff-stat identical before/after. TS check passes. |
| 7. Frontend minimum | Read pill status/title/click branch, matrix global-deployment predicate, `syncStatus`, all report folds and command-error copy. Disabled pill uses inactive styling, not error/red. Non-null target no longer automatically selects toggle-off. | **Findings 5–7.** Sync/install/refresh/Edit skip handling otherwise does not manufacture failures. No ticket-07 styling/a11y review performed. |
| 8. Fixture parity | Executed fixture probes for failed skill Disable→resync, missing shared sibling, and repeated failed Disable. Read enable and import/reassert implementations; scenario contains both grains. | **Finding 4**, plus fixture mirrors findings 2/3. Project Enable fixture uses ordinary `syncAssignment` with no first-sync collision model; Rust passes overwrite=false and can emit TARGET_EXISTS. Add an occupied project-target fixture case when modeling that presence. |
| 9. Tests | Executed 18 disable tests, full 728 Rust tests, 396 targeted Vitest tests, binding export, TS no-emit check. Read new Rust/fixture/hook test cases and existing removal regression adaptations. | All existing tests pass. Missing adversarial combinations listed below explain why they do not establish the goal. |

### Onboarding-path exclusion judgment

`skill_store.rs:921–935` still returns disabled target paths, and onboarding excludes them. Thus an unmanaged copy appearing at that path is hidden from import review. **Not independently blocking for this enabler**: it neither overwrites nor re-enables it, duplicate-name admission would refuse normal import anyway, and managed-name review belongs to tickets 05/21. However this exclusion is not evidence that the folder is owned or absent; it cannot justify Delete's treatment of a marker (finding 1). Carry the discovery/Replace-or-Leave decision explicitly to those tickets. The new “onboarding import never rematerialises” test proves rejection of a hidden/unowned candidate, not a successful import reporting a Disabled skip.

### Explicitly accepted deferrals

- A failed Enable clears the disabled marker and can leave no **global** target row. This is recorded in the Result and is ticket 02's failed-first-sync/error-row work; do not call it implemented in 01. Project Enable already retains an error assignment.
- Enable's TARGET_EXISTS does not enter the shared overwrite ask. Keep the collision safe (currently it is), then integrate it with the round-19 overwrite work; not a reason to silently overwrite here.
- Bulk confirmation, new disabled visual mark, and obsolete unsync vocabulary are expressly later surface tickets, not blockers invented by this review.

## Missing regression tests (concrete sketches)

1. `intent_only_marker_delete_preserves_unmanaged_bytes`: finding 1's two orderings; also enable collision → declined/disabled → Delete for D9's consumer.
2. `partial_skill_disable_resync_skips_error_assignments`: failing removal, marker set, permissions restored and target removed, resync-one and resync-all; no artifact, typed skipped, original failure row retained. Include a healthy non-disabled skill in the same batch to prove isolation.
3. `partial_skill_disable_relocation_does_not_materialise`: former Augment target, failed removal, `disabled_at` set; relocation leaves current dir absent.
4. `missing_shared_sibling_disable_failure_does_not_mint_disabled`: only Kimi row present, Amp subsequently installed, locked shared artifact; no false disabled row, retry settles all intended rows only after removal succeeds.
5. `disable_enable_entry_points_are_serialized`: hold the existing guard test barrier, call each public entry point on another thread; assert filesystem and row state do not change until release, then complete within a timeout.
6. `disabled_skill_survives_each_byte_adapter`: table over Update, Restore with central absent, Re-point and Edit replay; assert central bytes/metadata change as intended, both grain markers remain, and global/project artifacts stay absent.
7. `disabled_virtual_group_assignment_enables_once`: project `agents_skills` assignment, skill Disable/Enable, correct `.agents/skills` path and registry capability; existing disable tests only use project Claude.
8. `delete_disabled_skill_settles_rows_and_cascade`: formerly owned global + project artifacts removed by Disable, then Delete removes central/records; a reappeared **owned** artifact failure keeps the skill per ADR-0002. Keep intent-only foreign-artifact case separate.
9. Fixture retry-presence and first-sync project collision tests (finding 4 / coverage 8), and real-fold UI tests for findings 5–7. Current hook additions predominantly stub folds and prove dispatch, not semantic correctness.

## Commands and outcomes

All commands ran from repo root unless a subshell/cd says otherwise. No `tauri:dev`, browser mutation, live DB access, source edits, archive, move, or commit.

| Exact verification command | Outcome |
|---|---|
| `cd src-tauri && cargo test skill_disable && cargo test --all` | Disable: **18 passed**. All: **728 passed**, 0 failed; main/doc tests 0. Full output retained by harness at `/var/folders/6p/j8hvzsjx4wjf__r2jbgbff980000gn/T/pi-bash-e11308df4e2d97fb.log`. |
| `npx vitest run src/fixtures src/lib src/hooks` | **15 files / 396 tests passed**. Node emitted the non-fatal localStorage experimental warning. |
| `shasum -a 256 src/bindings/index.ts; git diff --stat -- src/bindings/index.ts; (cd src-tauri && cargo test export_bindings); shasum -a 256 src/bindings/index.ts; git diff --stat -- src/bindings/index.ts` | **1 export test passed**. Both hashes `4c9e7a025c87625a675586f0e5d9e71dd9de1ac028c97c2140c24b8f0c45c45e`; both stats 93 lines, 86 insertions / 7 deletions. |
| `bun x tsc -b --noEmit` (run twice; second followed by `printf 'tsc exit=%s\n' "$?"`) | **exit 0**, no TS errors. |
| `git diff --check -- src-tauri/src src/bindings/index.ts src/fixtures src/hooks/useSkillLibrary.ts src/hooks/useSkillLibrary.test.ts src/lib/reportOutcome.ts src/commandError.ts src/syncStatus.ts src/components/skills/SkillCard.tsx src/components/skills/types.ts src/components/projects/types.ts src/components/projects/AssignmentMatrix.tsx` | Reports 10 trailing-whitespace lines, **all generator-produced `src/bindings/index.ts`**; no hand-edit requested, not a ticket finding. |
| `date '+%Y-%m-%d %H:%M:%S %Z'; git rev-parse --short HEAD; git branch --show-current` | `2026-09-29 09:19:01 CDT`; `553db2a`; `main`. |

LSP tool check on `useSkillLibrary.ts`, `reportOutcome.ts`, `fixtures/backend.ts`, `SkillCard.tsx`, `AssignmentMatrix.tsx`: zero diagnostics but **all five inconclusive** (push-only server), not claimed clean; the successful compiler run is the affirmative type-check evidence.

### Probe A — exact command (in-memory only)

```bash
bun -e '
import { createFixtureBackend } from "./src/fixtures/backend.ts";
import { buildScenario } from "./src/fixtures/scenarios.ts";
import { importOutcome, projectSyncOutcome } from "./src/lib/reportOutcome.ts";
const state = buildScenario("failures", 1000);
const h = createFixtureBackend(state, { latencyScale: 0, now: () => 1000 }).handlers;
const s = state.skills.find(s => s.dto.name === "tdd");
const p = state.projects.find(p => p.id === "prj-quartermaster");
state.unwritableProjectTools.push(`${p.id}:claude_code`);
await h.disableSkillTargets({kind:"skill", skill_id:s.dto.id});
console.log("after failed Disable", s.dto.disabled_at, p.assignments.find(a => a.skill_id === s.dto.id)?.status);
state.unwritableProjectTools = [];
const r = await h.resyncProject(p.id);
console.log("Resync while skill disabled", r.report.items.filter(i => i.skill_id === s.dto.id).map(i => i.status));
const docx = state.skills.find(s => s.dto.name === "docx");
docx.dto.targets = docx.dto.targets.filter(t => t.tool !== "amp");
const d = await h.disableSkillTargets({kind:"skill_tool", skill_id:docx.dto.id, tool:"amp"});
console.log("shared failed removal", d.targets.map(t => t.status.status), docx.dto.targets.filter(t => ["amp","kimi_cli"].includes(t.tool)).map(t => [t.tool,t.status]));
const out = importOutcome({groups:[{group_name:"alpha",status:{status:"imported",skill_id:"s",skill_name:"alpha",forced_tools:[],originals:[],targets:[{skill_id:"s",skill_name:"alpha",tool_key:"pi",status:{status:"skipped",error:{code:"TARGET_DISABLED",tool:"pi"}}}]}}]}, {t:k=>k});
console.log("import disabled skip folded",out.errors,out.toast);
const bulk = projectSyncOutcome({items:[{assignment_id:"a",project_id:"p",skill_id:"s",skill_name:"alpha",tool:"pi",status:{status:"skipped",reason:{reason:"disabled"}}}]}, {action:"bulkAssign",t:k=>k});
console.log("bulk assignment disabled skip",bulk.toast);
'
```

Exit 0; observations: `1000 error` → resync `synced`; shared removal `failed` with Kimi `error` / Amp `disabled`; import emitted `errors.syncFailedTitle` / `errors.targetDisabled` and `partialFailure`; bulk emitted success / `projects.bulkAssignNothing`. These are probes of current behavior, **not passing correctness assertions**, and do not substitute for the proposed Rust filesystem tests.

### Probe B — exact command (in-memory only)

```bash
bun -e '
import {createFixtureBackend} from "./src/fixtures/backend.ts";
import {buildScenario} from "./src/fixtures/scenarios.ts";
const state=buildScenario("failures",1000);
const h=createFixtureBackend(state,{latencyScale:0,now:()=>1000}).handlers;
const s=state.skills.find(s=>s.dto.name==="tdd");
const p=state.projects.find(p=>p.id==="prj-quartermaster");
state.unwritableProjectTools.push(`${p.id}:claude_code`);
for(let i=1;i<=2;i++) {
 const r=await h.disableSkillTargets({kind:"skill",skill_id:s.dto.id});
 console.log("same locked path, Disable",i,r.targets.filter(t=>t.rows.some(r=>r.scope==="assignment")).map(t=>t.status.status),p.assignments.find(a=>a.skill_id===s.dto.id)?.status);
}
'
```

Exit 0. First call assignment outcomes `[removed, removed, removed, failed]`, inspected row `error`; second `[removed]`, inspected row `disabled`, although the same path is still configured unwritable.

### Read-only inspection commands

Used `read`, `read_symbol` and `module_report` for the files named in the coverage table; started with AGENTS.md in full, BACKLOG/latest handoff, both ADRs, round19 spec/ticket Result and CONTEXT. The initial status/stat established the concurrent ticket-07 changes. Shell diff inspections (no writes):

```bash
ls .scratch/handoffs | sort | tail -1
git status --short
git diff --stat -- src-tauri/ src/bindings/index.ts src/fixtures/ src/hooks/useSkillLibrary.ts src/hooks/useSkillLibrary.test.ts src/lib/reportOutcome.ts src/commandError.ts src/syncStatus.ts src/components/skills/SkillCard.tsx src/components/skills/types.ts src/components/projects/types.ts src/components/projects/AssignmentMatrix.tsx CONTEXT.md CHANGELOG.md
git diff -- src-tauri/src/core/artifact_removal.rs src-tauri/src/core/global_sync.rs src-tauri/src/core/project_sync.rs src-tauri/src/core/propagation.rs src-tauri/src/core/refresh.rs src-tauri/src/core/target_relocation.rs
git diff -- src-tauri/src/core/skill_store.rs src-tauri/src/core/sync_status.rs src-tauri/src/core/skill_catalog.rs src-tauri/src/commands/mod.rs src-tauri/src/core/errors.rs src-tauri/src/lib.rs src/bindings/index.ts
git diff -- src/lib/reportOutcome.ts src/hooks/useSkillLibrary.ts src/components/skills/SkillCard.tsx src/components/projects/AssignmentMatrix.tsx src/fixtures/backend.ts src/fixtures/model.ts src/fixtures/scenarios.ts
git diff -- src-tauri/src/core/tests/skill_store.rs src-tauri/src/core/tests/sync_status.rs src-tauri/src/core/tests/target_relocation.rs src/fixtures/backend.test.ts src/hooks/useSkillLibrary.test.ts src/App.tsx src/i18n/resources.ts src/commandError.ts src/syncStatus.ts src/components/skills/types.ts src/components/projects/types.ts
git diff --numstat -- src-tauri/src src/bindings/index.ts src/fixtures src/hooks/useSkillLibrary.ts src/hooks/useSkillLibrary.test.ts src/lib/reportOutcome.ts src/commandError.ts src/syncStatus.ts src/components/skills/SkillCard.tsx src/components/skills/types.ts src/components/projects/types.ts src/components/projects/AssignmentMatrix.tsx CONTEXT.md CHANGELOG.md
git diff -- src-tauri/src/core/tests/artifact_removal.rs AGENTS.md CHANGELOG.md CONTEXT.md
rg -n 'if let \(Some\(adapter\), false\)|if store.get_skill_target|TargetTransition::DisabledRecorded|if assignment.status == SyncStatus::Disabled|if row.status == SyncStatus::Disabled|const present =|const resyncOne|const deployed =|skill.disabled_at !== null|for \(const target of group.status.targets\)|case "bulkAssign"|if \(nothingPlanned\)' src-tauri/src/core/skill_disable.rs src-tauri/src/core/project_sync.rs src-tauri/src/core/target_relocation.rs src/fixtures/backend.ts src/components/skills/SkillCard.tsx src/hooks/useSkillLibrary.ts src/lib/reportOutcome.ts
rg -n 'RemovalScope::Skill \{|builder.push_global|let result = if is_present|const enableSkill|syncAssignment\(project, a\)' src-tauri/src/core/artifact_removal.rs src/fixtures/backend.ts
rg -n -A 2 'bulkAssignNothing:|resyncSuccess:|resyncAllSuccess:|unsyncNothingPlanned:' src/i18n/resources.ts
```

All inspection commands completed successfully. The App diff at review time contained only ticket-07 styling hunks; it supplied no ticket-01 finding.

## Goal-backward verdict

The goal is not just a serialised, typed Disable command: **nothing except Enable may reactivate a disabled skill, and a retained preference must not make someone else's files deletable.** Findings 1–3 violate those goals despite the green suites. Fix them and add their adversarial regressions before shipping ticket 01.

## Re-review — 2026-09-29

**fix-then-ship**

Bounded re-review of the uncommitted tree on `main` at `553db2a`. **One blocker remains: F1's ownership protection is lost when Enable of an intent-only project assignment fails.** F2–F7 are closed for their original cases. No source edits, scratch reverts, additional delegation, live-library operations or commits; only this review and the ticket's Comments were edited. Test sensitivity below is established by reading the tests against their code paths, not by claiming a new executed mutation run.

### Per-finding disposition and test sensitivity

| Finding | Disposition | Evidence and what the regression pins |
|---|---|---|
| **F1 — marker invents ownership** | **OPEN in the project Enable failure case below; direct marker removal fixed** | `skill_store::is_intent_only` recognizes only `disabled + NULL synced_at`. `artifact_removal::TargetBuilder::push_global` and `push_assignments` now set these rows aside; Delete clears them without planning their paths. `intent_only_marker_delete_preserves_unmanaged_bytes` pins both arrival orderings and a **global** Enable collision; bypassing the global marker split would delete the foreign bytes in its first two cases. `an_intent_only_assignment_is_unassigned_without_touching_its_path` pins project marker removal **without first attempting Enable**. `an_owned_disabled_target_is_still_removed_by_delete` prevents an overbroad “skip every disabled row” fix. None pins the project marker → failed Enable → removal transition. |
| **F2 — skill marker ignored after partial Disable** | **CLOSED** | `resync_project_unlocked` caches `disabled_at` per skill and checks it before `sync_single_assignment`; resync-all delegates there. `assign_skill_to_tools` reads the marker before iterating: existing error rows return Disabled without settlement, absent rows are recorded disabled without filesystem work. Toggle-on reaches that same engine; an existing row is toggle-off, not toggle-on. Relocation checks the marker before `materialise` or adding a move. `partial_skill_disable_resync_skips_error_assignments` genuinely fails removal, removes the artifact externally, exercises resync-one/all and bulk assign, and checks no new artifact, unchanged error/diagnostic, and a healthy beta still syncing. Removing the resync marker gate violates its skip/no-artifact assertions. The relocation test exercises a real failed removal and asserts zero moves, the old path retained and no current artifact. Its error-status assertion also catches a removed gate; it does not explicitly compare `last_error`, but the gated branch performs no transition at all. Fixture F2 pins unchanged error/diagnostic and skips for all three actions. |
| **F3 — failed shared artifact mints sibling marker** | **CLOSED** | `disable_pair` requires both `report.failed_rows() == 0` and named-row absence. If the requested Tool's own row fails, the original `has_row` guard prevents admission; `a_failed_removal_keeps_the_row_error_not_disabled` pins it. If only a sibling has a row, `missing_shared_sibling_disable_failure_does_not_mint_disabled` starts with Kimi alone, installs Amp, locks the shared parent, asserts Kimi error/Amp absent, then retries and asserts both disabled only after removal. Dropping the new failed-row check fails the Amp-absent assertion. Fixture F3 covers the same failed/retry sequence. |
| **F4 — fixture retry fabricates removal** | **CLOSED for the reported retry** | The per-path `artifacts` map records presence on failed removal, rather than deriving it afresh from the now-error row. Fixture F4 repeats Disable under the same lock, then Delete, verifies the row/skill remain, and finally unlocks and removes it. Reverting to the old status-only presence test makes the second Disable succeed and fails this test. No claim that the fixture now models project first-sync collisions: `enableSkill` still uses its collision-free `syncAssignment` (the previously noted parity limitation). |
| **F5 — all pair-disabled card offers ineffective Deploy** | **CLOSED** | Card and hook use the same `deployAction`. All existing rows disabled → explicit counted Enable label and `enableSkillTargets(kind: skill)`, not ordinary sync; the real-fold hook test checks both command routing and the Enable toast, and would fail the old route. With disabled pairs plus **absent** targets (no rows for never-synced Tools), it still offers Enable N disabled tools; this intentionally re-enables recorded pairs, not an implicit sync to every absent Tool. With a live row plus disabled rows it offers the existing Disable action. The six-case table pins the live/disabled mixture and no-row case; it does not explicitly name the absent-tool mixture, since `deployAction` receives rows, not the effective Tool set. A stored `missing`/`pending` global row would count as deployed, like an error row; current global first sync does not create these rows, so revisit with ticket 02 rather than inventing another blocker here. |
| **F6 — import makes Disabled an error** | **CLOSED** | `isDisabledSkip` is shared by `syncOutcome` and `importOutcome`. The nine fold cases cover sync bulk/install/toggle, install, import, refresh, Edit, Enable and project resync; import additionally asserts the clean-import success toast. Removing the import check creates an error entry and fails these tests. |
| **F7 — skipped assignments reported already assigned** | **CLOSED** | Bulk branches now distinguish all-skipped, mixed synced/skipped and failed/skipped, with warning copy and the appropriate counts. Resync-one/all include skipped counts. Pure tests assert exact keys/counts for all-skipped, mixed and partial bulk, plus resync and partial resync-all. Removing the skip branches or omitting counts fails those assertions. The generated dynamic resync key resolves to the new EN keys. |

### Remaining blocker — F1-R1: failed project Enable converts non-ownership into ownership

**Locations:** `src-tauri/src/core/skill_store.rs:185–186,1391–1403`; `src-tauri/src/core/skill_disable.rs` `enable_skill`; `src-tauri/src/core/project_sync.rs:662–685`; `src-tauri/src/core/artifact_removal.rs:423–486,571–582`.

**Repro (source-traced; no destructive operator-library run):**

1. Install managed `alpha` centrally, with no project deployment. Disable it at skill grain.
2. Assign it to a project's Claude Tool. The new path in `assign_skill_to_tools` records an intent-only assignment: `status = disabled`, `synced_at = NULL`, no filesystem write.
3. Create a foreign `<project>/.claude/skills/alpha/SKILL.md` (or have it there before step 2).
4. Enable the skill. `enable_skill` clears `disabled_at` and sends that assignment to `first_sync_assignments_unlocked`. Its overwrite=false sync correctly refuses with `TARGET_EXISTS`; **the catch writes `AssignmentTransition::SyncFailed`**, leaving the assignment `error + NULL synced_at`.
5. Unassign it, remove the project, Disable again, or Delete the managed skill. Now `is_intent_only` is **false solely because status changed**. `push_assignments` adds the foreign path as a removal target and execution deletes it. No successful sync occurred between steps 2 and 5.

This is the same data-loss class as F1, reached through this ticket's new intent-only-assignment and Enable paths. The documented leftover is **not only hypothetical ticket-02 global error rows**: global sync does record only successful targets today, but project `assign_and_settle` already persists failed-first-sync assignments, and this ticket's `first_sync_assignments_unlocked` also does. The global collision ordering test passes because global Enable deletes its marker before syncing and leaves no row on failure; that is not the project behavior.

**Required fix:** keep removal ownership independent of the lifecycle status. A never-materialised project row must stay a non-owner across failed Enable and reconcile; preserve its failure diagnostic without promoting it to an artifact owner. A status-independent `synced_at` ownership check can be the conservative implementation after accounting for legacy/partial-write rows, or record ownership explicitly. Do not “fix” this by overwriting the foreign folder, falsely recording a successful timestamp, or suppressing the error. Update the fixture and the ownership documentation to match. This safety invariant must land in 01; ticket 02 can still own creating new global failure rows.

**Required regression (use existing test helpers; not added by this reviewer):** extend the project F1 test with Enable before unassign, and repeat with Delete/reconcile as applicable:

```rust
let f = fixture(&[]);
disable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
let project = project(&f, "claude_code");
assign_skill_to_project_tools(&f.store, &project.id, &f.skill_id, NOW).unwrap();
let path = Path::new(&project.path).join(".claude/skills/alpha");
foreign_folder(&path);
let enabled = enable_targets(&f.store, f.home(), &f.skill(), NOW).unwrap();
assert!(matches!(enabled.projects.items.as_slice(), [item]
    if matches!(item.status, ProjectSyncOutcomeStatus::Failed {
        error: CommandError::TargetExists { .. }
    })));
let row = f.store.get_project_skill_assignment(
    &project.id, &f.skill_id, "claude_code"
).unwrap().unwrap();
assert_eq!(row.synced_at, None);
assert_eq!(row.status, SyncStatus::Error);
unassign_skill_from_project(&f.store, &project.id, &f.skill_id).unwrap();
assert!(foreign_bytes_survive(&path)); // Current path deletes it.
```

### Ownership representation / migration check

- Every **normally settled successful materialisation** inspected records a non-null timestamp: global first sync (also global Enable) uses `TargetTransition::Recorded { synced_at: now }`; project assign/resync/Enable and project Propagation share `sync_assignment_target` → `AssignmentTransition::SyncCompleted`; global Propagation's copy re-materialisation and relocation use `TargetTransition::SyncCompleted`.
- `Recorded` takes an `i64`, not an `Option`; its INSERT and conflict UPDATE both write it. Both tables' `Disabled` / `SyncFailed` updates use `COALESCE` and preserve an existing timestamp. `DisabledRecorded` inserts NULL but does not clear the timestamp on conflict. Enable deletes/recreates global rows intentionally; it does not null a surviving owned row.
- `synced_at` exists in global V1 and project V4 DDL; none of the incremental migrations shown clears it or introduces it as NULL for every legacy success. Historical global sync in `24c5ae4` wrote `Some(now_ms())`; the old upsert → Recorded conversion in `b8d4a99` preserves `Some(now)` as the required scalar. Project sync's introduction in `86a632a` passes `Some(now)` on success, and `2ed1b76` already used COALESCE for updates. I found no ordinary successfully settled legacy write that omitted the timestamp.
- That is **not** proof every physical artifact has a timestamp: the schema is nullable, failed/pending project rows have existed for years, and historical project copy sync could materialise before a fallible hash/store step. No migration reconstructs ownership for those rows. Treat unknown ownership conservatively; do not use the normal-success history to claim `status != disabled` grants ownership. The concrete blocker above requires neither corruption nor an old schema nor a failed DB write.

### `cleared_markers` consumers and nonblocking follow-up

The required wire field is present in generated `RemovalReport`; merge preserves it, the Rust serialization assertion includes it, the fixture collects it after the whole scope succeeds, and the `useProjectState.test.ts` report builders were updated. Production project hooks pass reports through; removal, delete and project-removal folds do not reinterpret marker paths. The markers-only toggle regression passes and the fold reports success, not “nothing planned.”

**N1 — nonblocking count defect:** `removalCounts` still counts only target rows. A markers-only **bulk unassign** is successful but says `projects.bulkUnassignSuccess { count: 0 }` after actually clearing an assignment; the added test covers only toggle copy, which has no count. Confirmed with an in-memory pure-fold probe. Include cleared assignment markers in the unassignment count (without falsely claiming a filesystem artifact was removed), and test markers-only and mixed bulk reports. This does not block the ownership safety fix.

### Guard and supplementary sketches

`disable_enable_entry_points_are_serialized` is more than a deadlock smoke: its helper holds the **real** mutation guard, spawns the entry point, waits 300 ms, checks noncompletion plus disk/row state **while still holding the guard**, releases, joins, and checks the new disk/row state. Removing either entry-point wrapper permits those observed mutations before release. The test is timing-based rather than a deterministic worker-start barrier, and join has no internal timeout; a start handshake and bounded completion wait would strengthen it, but the actual implementation directly wraps both bodies in `serialized` and this is not a new blocker.

The virtual-group test checks the `.agents/skills` artifact disappears/reappears with exactly one project result; the Delete test checks failed owned-artifact removal keeps the skill/central copy and a retry deletes/cascades. Sketch 6 (all byte adapters) remains explicitly unimplemented, not silently counted as coverage. The permission tests ran as uid 501, not root; their permission probes therefore exercise the normal denial path on this run.

### Fresh commands and outcomes

| Command | Result |
|---|---|
| `cd src-tauri && cargo test skill_disable && cargo test artifact_removal && cargo test target_relocation && cargo test mutation_guard` | All green: **26 / 32 / 13 / 13** tests respectively, zero failures. The first filter includes the relocation test whose name contains `skill_disable`; these are filter totals, not unique-test totals. |
| `npx vitest run src/fixtures src/lib src/hooks` | **15 files / 422 tests passed**; non-fatal localStorage experimental warning. |
| `npx vitest run src/components/projects/useProjectState.test.ts && bun x tsc -b --noEmit` | **31 project-hook tests passed**, followed by TypeScript exit 0 with no output. |
| `bun -e` pure-fold/presentation probe | Markers-only bulk unassign emitted success with count **0** (N1). `deployAction`: no rows → sync; disabled-only → counted Enable; disabled+synced → deployed; disabled+missing/pending → deployed. No disk/library mutation. |
| `git diff --check -- .scratch/round19/issues/01-disabled-target-state.md` | Clean before review append. |
| `id -u; git rev-parse --short HEAD` | `501`; `553db2a`. |

Read-only verification also used the scoped diffs named in the brief, symbol/body reads, `git log main` for store/sync history, and `git show` on `24c5ae4`, `86a632a`, `2ed1b76`, and `b8d4a99`. No full-gate or binding-regeneration rerun is claimed for this bounded re-review; the requested filters do not run `export_bindings`.

**Goal-backward check:** the automatic-pass/failed-Disable gates are repaired, but “only a successful sync makes a marker an owner” is still false for a failed project Enable. Keep ticket 01 at fix-then-ship until F1-R1 is pinned and fixed.
