fix-then-ship

# Ticket 04 adversarial review — 2026-09-29

Reviewed the uncommitted ticket-04 implementation on `main` at **`c2df098`** (the brief named `6d7b6f4`; the intervening commit changes the handoff only). Read AGENTS.md in full, ADRs 0001/0002, D8/D9/D12/D20 and tickets 01–04 including their results/comments. Ticket 09's implementation is excluded; its reporter was used as a black-box dependency in the Add cancellation reproduction, not reviewed independently.

**Two blocking findings, three should-fix findings.** The requested tests are green. Green tests do not cover the two reproduced operator failures below. No application source edits, live-library runs, commits, moves or archives were made. Binding regeneration was byte-stable.

## Findings, ranked

### F1 — Blocking: project Enable's ask ignores Keep/Overwrite for a previously deployed assignment; Keep can be followed by destructive resync

**Locations:** `src-tauri/src/core/project_sync.rs:789–790`; `src-tauri/src/core/skill_disable.rs:408–420`; `src-tauri/src/core/project_sync.rs:673–706`. Fixture mirror: `src/fixtures/backend.ts:1583`.

`resolve_collisions` treats **any** `synced_at != NULL` assignment as `AlreadyAssigned`. But this is a historical ownership flag, not proof that the first Enable succeeded. A successful skill Disable retains the assignment and its timestamp (`skill_store.rs:1571–1581`). Project Enable first-syncs that same row without resetting ownership. A new foreign directory makes Enable report `TARGET_EXISTS`, with the row now `error` but still owning according to its historical timestamp. The frontend offers both answers; the new resolver ignores either and returns `already_assigned`.

**Reproduced through the fixture's real handlers, with the same Rust transitions:**

1. Register a project, configure Claude Code, assign a healthy skill successfully.
2. Disable that skill. Its assignment is `disabled`, with `synced_at` still set.
3. Put a foreign folder at the assignment path; Enable the skill → askable `TARGET_EXISTS`.
4. Resolve with **Keep** → `already_assigned`, row remains `error`, not `disabled`.
5. Re-sync the project → `synced`, foreign folder replaced. (`resync_project_unlocked`, line 630, uses `owns_artifact()` as overwrite authorization.)

Overwrite also returns `already_assigned` without replacing the folder. Thus the ask can both lie about success and fail to preserve an explicit Keep.

**Fix:** make admission to project Enable's *first sync* reset the ownership of successfully disabled assignments, analogous to global Enable deleting/recreating its disabled rows. A failed first sync must then remain a non-owner; Keep must settle it `disabled`/`declined`, and a successful overwrite must set `synced_at` anew. Preserve ownership for failed removals (ADR-0002); **do not simply remove the resolver's owned-row safeguard**, which protects genuinely owned stale artifacts.

**Regression:** extend `a_project_collision_resolves_with_overwrite_or_keep` with a *previously synced → successfully disabled → foreign occupant → Enable collision* setup, not only `failed_project_enable`'s never-synced marker. Assert both choices, then Keep → resync/Propagation/Delete preserving the foreign bytes. Assert successful Overwrite owns its new artifact.

### F2 — Blocking: Cancel during Add discovery is discarded and the skill is installed anyway

**Locations:** `src/hooks/useAddSkillFlow.ts:434–455, 477–528`; `src-tauri/src/commands/mod.rs:455` (batch's reset). The command reset is correct in isolation; the action boundary is not.

Both single-candidate flows declare `cancel: "installSelection"` **before** awaiting `listLocalSkillsCmd` / `listGitSkillsCmd`. Neither listing command takes the token. Cancel while that promise is outstanding sets the token, but the continuation unconditionally calls `installBatch`; `install_selection` resets the token and installs. The operator has explicitly cancelled an action that had not installed anything.

**Reproduced with the real Add hook + real reporter/surface, a deferred listing at the Tauri seam, and the real in-memory fixture install handler:**

```text
before cancel: pending.cancellable = true
cancel flag before listing resolves: true
calls: getOnboardingPlan → listGitSkillsCmd → cancelCurrentOperation → installSelection
cancel flag after: false
installed: ["ts-error-fixer"]
```

The same ordering exists for local Add. This is an integration finding at 04's new install call boundary; it does not ask this review to refactor ticket 09.

**Fix:** expose Cancel only during a token-taking phase, and/or preserve an action-level cancellation decision across preparation so a cancelled discovery continuation cannot issue `installSelection`. Coordinate the lifecycle seam with 09. Do not remove the batch command's reset and leak a prior operation's cancellation into the next action. The overwrite-ask/retry phase likewise must not advertise cancellation of the non-token-taking sync retry.

**Regression:** defer each listing, start single Add, cancel, resolve the listing with one candidate; assert **no install command** is sent if Cancel was offered. Also pin that ordinary batch cancellation still reports a cancelled tail and a fresh later Add works.

### F3 — Should-fix: collision facts re-hash the managed source, violating the stored-identity rule

**Location:** `src-tauri/src/core/sync_engine.rs:35–47` (`Occupant::observe`).

Every collision now calls `content_identity::same_content(Source::Directory(source), target)`. For these managed-skill sync paths that re-reads the central bytes instead of trusting the persisted identity. Global overwrite policy uses `Source::Managed` (`global_sync.rs:124–126`), so the decision and the ask can disagree; it also adds a full source traversal per collision under the mutation guard. This violates AGENTS.md's Content identity invariant and this review's explicit priority 3.

**Failing-test sketch:** install source content A (stored hash A); externally change central bytes to B without recording a new identity; create an unowned target containing B; sync with `overwrite_if_same_content: true`. The stored-identity policy correctly refuses silent replacement, but the error's new `same_content` is currently **true**, not false. `content_identity::tests::managed_read_backfills_once_and_trusts_the_row` already pins the governing policy.

**Fix:** pass the managed source identity from the core caller to collision observation (or enrich the typed collision at core settlement using `Source::Managed`), retaining the target's observed metadata. Keep hashing in `content_identity`; no command-seam or frontend re-derivation. `Directory` remains appropriate for genuinely unmanaged inputs only. Test stored-identity disagreement, unknown identity, and same-content first-pass replacement.

### F4 — Should-fix: the fixture still double-reports divergent import siblings

**Location:** `src/fixtures/backend.ts:1358–1375`; compare Rust `src-tauri/src/core/onboarding_import.rs:546–559`.

The Rust change removes an original when that path already has a `TARGET_EXISTS` target row. Fixture `importGroup` still pushes `kept_divergent` first and retains it after `syncBatch`. This is precisely the duplication this ticket claims to remove, and it makes fixture report counts/review behavior diverge from production.

**Reproduced:** `first-run`, import the Codex variant of `commit-message`, auto-sync to Codex + Gemini CLI. The fixture reports the Gemini path twice: target `failed/TARGET_EXISTS` **and** original `kept_divergent`. Rust's updated `auto_sync_on_keeps_a_sibling_that_diverged_after_planning` expects `originals.is_empty()` for this situation.

**Fix:** apply the same post-sync, collision-path filtering to fixture originals. Add the assertion to the fixture tests; keep a divergent original for a Tool *not* in the sync target set (that warning remains legitimate).

### F5 — Should-fix: cancellation can arrive while waiting for the guard and still finalize a not-yet-finalized candidate

**Location:** `src-tauri/src/core/install_batch.rs:209–218`.

The last cancellation check occurs **before** acquiring the mutation mutex. If another target mutation holds it, cancellation during that wait is not checked when the guard is finally obtained; `settle_unlocked` immediately finalizes and syncs. The documented cutoff is “a candidate already finalized completes its sync,” not “a candidate queued for the guard will install after Cancel.” This is a source-traced scheduling gap, not a claimed runtime reproduction.

**Fix:** re-check the token inside the guarded closure, before consuming `staged` with `finalize`. Return the candidate's `Cancelled` outcome and drop staging there when cancellation won; once finalize starts, finish sync unchanged.

**Regression:** synchronize a test at the post-stage/pre-finalize boundary, hold the mutation guard elsewhere, cancel while the candidate waits, release the guard, and assert no central record/artifact, staging cleaned, current and remaining rows cancelled. Also assert cancellation after finalize begins never interrupts that candidate's first sync.

## Coverage against the seven requested attack areas

| Area | Reviewed / proven | Remaining or failed coverage |
| --- | --- | --- |
| 1. Guard, acquisition, cancellation | `stage_*` outside guard; `StagedInstall` owns `StagingDir` RAII; finalize + global unlocked first sync inside one `serialized`; no nested guarded entry found. Git staging still calls `git_acquisition::acquire`. Command resets token. 9 batch tests pass. | **F2**, **F5**. Existing cancellation test sets the token in the progress callback *before* local staging, then proves staged bytes are dropped; it does not inject cancellation midway through Git acquisition or while waiting for the guard. No new held-guard regression for the batch/resolver. |
| 2. Collision collection/retry/merge | Batch collision leaves foreign bytes and no new global row; Overwrite replaces through sync_engine, Keep records a non-owner disabled marker/declined. Add catches a thrown retry and still folds the original install report. `mergeInstallRetry` preserves candidate rows, including cancelled tail. | Cross-product retry is broader than the collision set: `resolveGlobalCollisions` sends affected skills × affected Tools and drops unasked retry results. Existing tests explicitly allow this. Add a changed/unavailable cross-pair regression before claiming “exactly those pairs” at the mutation level. |
| 3. Facts and classification | Every low-level `TargetExistsError` gets facts; global conversion and `from_anyhow` preserve them for install/reassert/import/Enable/project/Propagation. Pair identity already exists in report rows. `Option<i64>` exports as nullable `number`; `sameContent` as boolean. Typed row errors classified in core settlement, not reconstructed at command seam. | **F3**. No disappearing-folder/unknown-mtime test. Re-assert and import *produce askable data*, which is not the same as having routed their frontend flows through an ask. |
| 4. Project resolution / Enable | Both new doors guarded; composites use unlocked seams. Never-owned project collision's Keep and Overwrite are tested; gone/disabled/skill-disabled/owned guards present. Success path uses the normal assignment sync transition. | **F1**. Missing tests for previously-owned successful Disable, owned stale row refusal, gone/disabled decisions, duplicate/non-collision decisions, and successful overwrite timestamp. |
| 5. Frontend ask / Add | One ask for Add; mixed choices map by row order; combined global+project Enable has one ask. Dismissed makes no retry/Keep; single-row all-Keep/all-Overwrite works. Single local/git installs and picker all call `installSelection`; cancellable set includes it. Project hook settles then refreshes the shown view when retry ran. | **F2**. `handleImport` at lines 406–419 still directly folds its first response; the Result explicitly defers that wiring to 21. No Refresh-report adapter in `overwriteAsk`. These are **uncompleted full-D12 integration**, not a claim that ticket 09's excluded action wiring was reviewed. Ticket 04's narrower frontend-minimum wording makes this a disclosed handoff, not an additional blocker here. |
| 6. Fixture parity | `collide-kit` supplies 3 candidates / 2 collisions; existing tests pin mixed retry choices, ordinary project Keep/Overwrite and Enable global retry. Real `latencyScale: 3` run: candidate 1 installed; cancel 30 ms into candidate 2 → candidates 2–3 cancelled, neither in catalog. | **F4**; **F1** faithfully reproduced by fixture. Fixture's missing-project `resolveCollision` calls `findProject` and throws rather than returning Rust's per-decision missing-assignment row; add a parity case when tightening stale-decision coverage. |
| 7. Tests | Requested Rust filters and Vitest files all pass; extra project/candidate/surface/fold/content-identity suites pass. Bindings export byte-stable. | Tests do not pin F1–F5. Git batch test uses `resolution: None`, despite its name mentioning listing resolution: add a non-default branch/subpath resolution test. Add same-content first-pass batch, pre-finalize cancellation and extraneous override tests. |

### Scope/positive findings

- Per-candidate failures continue the batch; acquisition cancellation becomes its own `cancelled` kind rather than `failed { CANCELLED }`.
- The new batch does not duplicate installer acquisition/finalize policy; the split preserves Git's byte entry point and local Tool-directory refusal.
- Merges replace target outcomes rather than recounting candidates. The install fold's existing cancelled-tail tests and new adapter tests remain green.
- No native app was launched. No D31 visual/axe/keyboard audit is claimed; this was the requested semantic/adversarial enabler review.
- Full D12 completion still needs actual Import/Refresh call-site routing and tests, not just Rust askable-row tests. The ticket's recorded deferral to 21 should remain visible to the orchestrator.

## Exact verification commands and outcomes

Run from the repository root unless the command says otherwise, on 2026-09-29.

```bash
cd src-tauri && cargo test install_batch && cargo test skill_disable && cargo test global_sync && cargo test onboarding_import && shasum ../src/bindings/index.ts && cargo test export_bindings && shasum ../src/bindings/index.ts
```

Exit 0. Test counts respectively **9 / 32 / 18 / 23 / 1**, no failures. Binding SHA-1 before and after:
`69dce91d3e9d47c58de094b7044674268ecd784b`. This means **no regeneration delta relative to the reviewed working file**, not no diff against HEAD (the new bindings are intentionally uncommitted).

```bash
npx vitest run src/lib/overwriteAsk.test.ts src/hooks/useSyncOrchestration.test.ts src/hooks/useAddSkillFlow.test.ts src/fixtures
```

Exit 0: **4 files, 137 tests passed**.

```bash
npx vitest run src/hooks/useCandidatePick.test.ts src/hooks/useSurfaceState.test.ts src/components/projects/useProjectState.test.ts src/lib/reportOutcome.test.ts && cd src-tauri && cargo test project_sync && cargo test content_identity
```

Exit 0: **4 files, 177 Vitest tests**; **36 project_sync-filter Rust tests**; **3 content_identity tests** passed.

```bash
git diff --check -- src-tauri/ src/lib/overwriteAsk.ts src/hooks/useSyncOrchestration.ts src/hooks/useAddSkillFlow.ts src/hooks/useCandidatePick.ts src/hooks/useSurfaceState.ts src/components/projects/useProjectState.ts src/components/skills/modals/OverwriteModal.tsx src/fixtures/backend.ts
```

Exit 0, no whitespace errors. No full release gate is claimed for this read-only review of a shared, in-progress tree.

### In-memory probe commands (no source files written)

F1's decisive Keep → destructive resync reproduction:

```bash
bun run - <<'TS'
import {createFixtureBackend} from './src/fixtures/backend.ts';
import {buildScenario} from './src/fixtures/scenarios.ts';
const {handlers:h,state:s}=createFixtureBackend(buildScenario('rich'),{latencyScale:0});
const skill=s.skills.find(x=>x.dto.name==='zod-schema-first')!.dto;
const {project}=await h.registerProject('/Users/alex/Projects/review-probe');
await h.configureProjectTools(project.id,['claude_code'],null);
await h.bulkAssignSkill(project.id,skill.id);
await h.disableSkillTargets({kind:'skill',skill_id:skill.id});
const path=`${project.path}/.claude/skills/${skill.name}`; s.projectOccupants.push(path);
await h.enableSkillTargets({kind:'skill',skill_id:skill.id},[]);
const kept=await h.resolveProjectCollisions([{project_id:project.id,skill_id:skill.id,tool:'claude_code',decision:'keep'}]);
console.log('Keep returned',JSON.stringify(kept.items[0].status),'folder survives',s.projectOccupants.includes(path));
const resync=await h.resyncProject(project.id);
console.log('Next resync',JSON.stringify(resync.report.items[0].status),'folder survives',s.projectOccupants.includes(path));
TS
```

Exit 0; **Keep returned `already_assigned`, folder survives true; next resync `synced`, folder survives false**. These paths are fixture strings only, not real filesystem operations. A preceding probe also tried Overwrite and got `already_assigned` with the `error` row untouched.

F2, deferred discovery with the real hook/reporter/surface and fixture batch:

```bash
bun run - <<'TS'
import {mock} from 'bun:test';
import {JSDOM} from 'jsdom';
const dom=new JSDOM('<!doctype html><body></body>',{url:'http://localhost/'});
for(const k of ['window','document','navigator','HTMLElement','localStorage','location','history']) Object.defineProperty(globalThis,k,{value:dom.window[k],configurable:true});
globalThis.IS_REACT_ACT_ENVIRONMENT=true;
const {createFixtureBackend}=await import('./src/fixtures/backend.ts');
const {buildScenario}=await import('./src/fixtures/scenarios.ts');
const b=createFixtureBackend(buildScenario('empty'),{latencyScale:0});
let release; const listing=new Promise(r=>release=r); const calls=[];
mock.module('./src/lib/tauri.ts',()=>({isTauri:true,invokeTauri:async(cmd,...args)=>{
 calls.push(cmd); if(cmd==='listGitSkillsCmd') return listing;
 return b.handlers[cmd](...args);
}}));
mock.module('./src/lib/platform.ts',()=>({createProgressChannel:async(onmessage)=>({onmessage}),pickFolder:async()=>[]}));
mock.module('sonner',()=>({toast:Object.assign(()=>{}, {success:()=>{},error:()=>{},warning:()=>{},info:()=>{}})}));
const {renderHook,act,cleanup}=await import('@testing-library/react');
const {useStatusReporter}=await import('./src/hooks/useStatusReporter.ts');
const {useSurfaceState}=await import('./src/hooks/useSurfaceState.ts');
const {useAddSkillFlow}=await import('./src/hooks/useAddSkillFlow.ts');
const t=k=>k;
const {result}=renderHook(()=>{const reporter=useStatusReporter(t);const surface=useSurfaceState();const flow=useAddSkillFlow({t,reporter,surface,sync:{autoSyncEnabled:false,resolveGlobalCollisions:async r=>r,scanScopeRevision:0,syncTargets:{},targetAllInstalled:()=>{}},catalog:{toolLabelById:{},tools:[]},library:{isSkillNameTaken:()=>false,loadManagedSkills:async()=>{}}});return {flow,reporter};});
await act(async()=>{});
const repo=b.state.repos.find(r=>r.candidates.length>0); const candidate=repo.candidates[0];
await act(async()=>result.current.flow.setGitUrl('https://github.com/'+repo.slug));
let task; await act(async()=>{task=result.current.flow.handleCreate();});
console.log('before cancel',result.current.reporter.pending);
await act(async()=>result.current.reporter.cancelPending());
console.log('cancel flag before listing resolves',b.state.cancelRequested);
await act(async()=>{release({candidates:[candidate],target_match:null});await task;});
console.log(JSON.stringify({calls,cancelFlagAfter:b.state.cancelRequested,installed:b.state.skills.map(s=>s.dto.name)}));
cleanup();
TS
```

Exit 0; output quoted in F2. The invoked modules were not changed.

F4, divergent original double-report:

```bash
bun run - <<'TS'
import {createFixtureBackend} from './src/fixtures/backend.ts';
import {buildScenario} from './src/fixtures/scenarios.ts';
const {handlers:h}=createFixtureBackend(buildScenario('first-run'),{latencyScale:0});
const plan=await h.getOnboardingPlan();
console.log(JSON.stringify(plan.groups.map(g=>({name:g.name,managed:g.managed_skill_id,variants:g.variants.map(v=>({tool:v.tool,path:v.path,fp:v.fingerprint}))})),null,2));
const g=plan.groups.find(g=>g.has_conflict && !g.managed_skill_id)!;
const report=await h.importOnboardingSelection([{group_name:g.name,chosen_path:g.variants[0].path,name:null}],{auto_sync:true,tools:g.variants.map(v=>v.tool),remove_originals:true},{onmessage:()=>{}});
console.log('IMPORT',JSON.stringify(report,null,2));
TS
```

Exit 0; `commit-message`'s Gemini CLI path appears in both `targets` (`TARGET_EXISTS`) and `originals` (`kept_divergent`).

Latency=3 cancellation parity:

```bash
bun run - <<'TS'
import {createFixtureBackend} from './src/fixtures/backend.ts';
import {buildScenario} from './src/fixtures/scenarios.ts';
const b=createFixtureBackend(buildScenario('failures'),{latencyScale:3});
const t=performance.now();
const report=await b.handlers.installSelection({source:{kind:'git',url:'https://github.com/acme/collide-kit',resolution:null},candidates:['lint-staged-rules','commit-helper','changelog-writer'].map(n=>({subpath:'skills/'+n,name_override:null})),tools:['claude_code'],overwrite_if_same_content:true,overrides:[],deploy:true},{onmessage:p=>{if(p.index===2)setTimeout(()=>void b.handlers.cancelCurrentOperation(),30);}});
console.log(JSON.stringify({latency:3,elapsedMs:Math.round(performance.now()-t),rows:report.candidates.map(c=>[c.name,c.status.status]),collisions:report.candidates.flatMap(c=>c.status.status==='installed'?c.status.targets:[]),tailPresent:b.state.skills.filter(s=>['commit-helper','changelog-writer'].includes(s.dto.name)).length}));
TS
```

Exit 0, 3905 ms: `installed / cancelled / cancelled`, first candidate's collision preserved, `tailPresent: 0`.
