# Ticket 17 — independent D31 audit of `c6a47c0`

**2026-10-01 (UTC) · FIX-THEN-PASS.** One **P1**: the missing-project-folder safety rule is not enforced on first assignment, Enable or collision retry. No P0. Two P2 usability notes; the menu `region` diagnostic is an accepted popup-specific false positive, not a new accessibility defect.

## Method and evidence

- Audited HEAD `c6a47c0d6e0224c9082b77aa7e7364aec4a55669`, initially clean. Read AGENTS, BACKLOG/latest handoff, ticket Work/Gate/Result/Comments, D17/D19/D21/D23/D26/D31, the signature-interaction contract, ADR-0002, DESIGN's Projects/Checkbox/strip vocabulary, audits 11–16, changed code/tests and related assignment/removal/propagation seams. Standards and spec reviewed here, without further delegation.
- Applied Impeccable audit, freshly fetched Web Interface Guidelines, React/shadcn guidance. Repo desktop minimum, 24px dense targets, EN-only catalog, primitive APIs and limited hash persistence override generic website recommendations.
- **Fixture only**, existing server `http://localhost:5175/?scenario=failures`. One named headless agent-browser session, `r19-17-audit`; trusted keys through the supplied CDP driver, never agent-browser `press`. Dark **1280×800**, light **960×640**. This uses the task's explicit headless/CDP lane rather than claiming a collaborative-preview run or native Tauri/VoiceOver test.
- All evidence below is under **`.scratch/round19/evidence/17-audit/`**. Copied and reviewed the implementer's drivers, changed their output directory/session, and independently executed them. Added adversarial scripts rather than treating the implementer's JSON as fresh evidence.
- `gate-*.json`: uninterrupted keyboard flow, every key's focused element, selected/focused distinction, stack-tagged `getProjectView` recorder. `states-*.json`, `state-words-*.json`: states, focus/hover words, Sync this, chips, Sync All and reconcile. `dialogs-*.json`: four dialog actions/progress and kept removal. `a11y-*.json`, `extra-*.json`: **14 whole-document axe runs**, including list, matrix, menu and all four dialogs in both configurations. `adversarial-*.json`: two additional My Skills menu `region` checks, real pending-checkbox refusal and reduced motion. `final-probes-*.json`: missing-folder picker route and fresh contrast pairs. PNGs include both grayscale matrices, disabled state, selection/focus, removal and long identity.
- `core-probe/` is an evidence-only Cargo binary that **imports the unchanged original core by absolute `#[path]`**. It runs against a temporary database and temporary project/central directories, never the operator's library. `core-probe.log` proves the filesystem gap independently of the browser fixture.
- Excluded contaminated observations: the first dark keyboard attempt overlapped `cargo test export_bindings`/Vite reload and lost focus during reload; both gates were rerun after generation finished. The first latency-12 pending probe caught startup's action rather than a toggle; the accepted `adversarial-*` probes explicitly wait for idle, observe `aria-busy=true`, and record exactly one toggle. No application source was edited.

## Per-clause verdict

| Clause | Verdict | Fresh evidence / code check |
| --- | --- | --- |
| Keyboard Projects → archived-app → monorepo → cell Space → named Assign all → Sync Project | **Pass** | Dark 49 key steps; light 37. Zero BODY samples in either accepted transcript. Selected archived-app retains `aria-current=true`/leading edge while ↑ focuses monorepo with its separate inset ring; Enter selects. Native select supplies the narrow path. |
| Apply mutation-returned views, no success refetch tail | **Pass** | Toggle / Assign all / Sync Project recorder counts **0 / 0 / 0** in both themes; Sync this **0** separately. Hook tests and `applyView` code agree. Failure/collision retry, kept Remove and Sync All retain their documented exceptional reads. |
| Tool labels, not-detected column, every state/name/word, legend | **Pass** | Claude Code · .agents/skills · Windsurf **not detected**. Six state words independently visible on focus and CDP hover in both themes; table below. Legend and cells use `StripMarkIcon` and the same pure fold. |
| Grayscale, hit area, repair eligibility | **Pass** | Filled / half-filled / hollow / slashed / × remain identifiable; missing/failed deliberately share × and are distinguished by word/name, not a falsely claimed seventh unique shape. Label hit height **35px** in a 36px row; minimum sampled label width **117.59 dark / 131.72 light**. Sync this appears only for stale/missing/failed. |
| D17 global/project coexistence | **Pass** | Alias-skill/Claude Code is enabled and named “also global”; Space creates its project assignment, then Assign all uses the same rule. The global target does not disable the cell. |
| One lens: All / Assigned / Issues, no duplicate sort | **Pass** | One `useListLens`; name ordering, Grouped/Flat; after repairing design-tokens Issues contains docx/frontend-design/tdd, Assigned five rows. No `LensSortControl` or `syncStatus` source reference remains. |
| D23 collapse / identity / Last synced / overflow | **Pass in tested configurations** | 960 replaces the list with a labelled select; inline toolbar is Sync Project + overflow, containing Configure Tools/Sync All. **64-character name and 110-character path** (stronger than requested 100) render whole; Last synced one line; no document horizontal overflow. The select itself clips its option normally; the full heading beneath does not. |
| Sync All missing-folder skip | **Pass for re-sync; safety incomplete elsewhere — A1** | One leaf report line “archived-app: folder not found — Update path or Remove”; no archived-app item failures, stored fixture statuses stay missing/missing. Core checks before its assignment loop or filesystem work; dedicated tests pass. |
| `reconciled:false` | **Pass** | Quartermaster's first view shows the quiet status line; Refresh view clears it and focuses the project h2. `try_serialized` remains the core reconcile door. |
| Remove locked monorepo, ADR-0002 | **Pass** | Dialog explains removed/staying/kept content, initially focuses Cancel; its action shows foot progress, then closes. Project remains with release-train/.agents/skills failed and reason; toast **“Project kept · 5 removed · 1 failed”**; focus returns to **More actions for monorepo**. Existing non-owning markers can also remain while removal is incomplete, consistently with ADR-0002. |
| Four dialogs, platform seam, D19 | **Pass** | Add/Settings/Tools/Remove use primitives and show action progress at their foot; `App.progressInDialog` includes all four. Escape restores tested idle dialog triggers. Browse uses `pickFolder`. Actual mutation submits/toggles/re-sync/path update are gated by `runAction` and `blockedReason`; opening a dialog and editing its draft are not backend mutations and need not be disabled. |
| Checkbox `focusableWhenDisabled` | **Pass** | Actual toggle: same checkbox remains focused, `aria-disabled=true`, `aria-busy=true`, name changes to “syncing…”. A second trusted Space is refused; **one** backend toggle observed; settled checkbox remains focused/checked. Primitive regression and DESIGN contract present. |
| Architecture / generated API | **Pass except A1** | Pure tested presentation fold; action folds remain in action layer; row verbs use the common skill door; Tool catalog supplied by binder. New `resync_project_assignment` has both macros, registration, spawn-blocking seam, serialized core entry, and `{view,report}` applied by hook. Skip DTOs specta-derived. Generated diff is exactly **27 added lines** for skip types/field + command; fresh export leaves bindings unchanged. |
| Styling / strings | **Pass; documentation count correction** | styles guard **1108 / 36 migrated**, detector `[]`; removed legacy families and deleted modules have no references. EN literal-key test passes. The actual resource diff removes **40 literal leaf keys / 37 logical keys** after plural normalization, not Result's “33”; all removed logical names searched under `src/` with no remaining references. |
| Contrast / axe / reduce | **Pass for measured scope, with menu exception below** | Text ≥4.68 dark / 5.37 light in initial measured surfaces; marks ≥3.324/3.332 on raised. All non-menu axe scopes have zero violations; pending spinner under reduce has animation `none`, duration `0s`, **0 running animations**. |
| Large matrix keyboard efficiency | **P2 note N1** | **127 Tab presses** first cell → last cell, both themes; reachable without a pointer or BODY. A grid keymap is not specified by D23 or this ticket. |
| Filtered repair focus | **P2 note N2** | Issues → Sync design-tokens removes its now-healthy row and falls back to MAIN, not a nearby matrix control. No sampled BODY frame. |

## Findings

### A1 — P1: other assignment paths recreate a missing project folder

**Locations:** `src-tauri/src/core/project_sync.rs:107–134,423–430,529–545,761–777,886–905,1133–1153`; `src/components/skills/modals/AssignProjectModal.tsx:104–108,163–180`; fixture divergence at `src/fixtures/backend.ts:646`.

**Reproduce (real core, safe temporary roots):** register an existing temporary project and Claude Code tool, remove its project directory, then call `assign_skill_to_project_tools`; repeat with `toggle_skill_assignment` on an absent assignment. Both report **Synced**, recreate the project root and materialize `.claude/skills/audit-skill`. The evidence binary also reproduces this through Enable's `first_sync_assignments_unlocked` seam and collision Overwrite. See `core-probe.log`: all four routes show `absent_before=true exists_after=true target_exists=true`.

**User-reachable route:** My Skills → local-notes → Assign to project… → archived-app → Assign. The picker offers archived-app **enabled**, without a folder-missing refusal; the matrix's disabled cells do not protect this door. `final-probes-*.json` and `missing-picker-*.png` reproduce it in both configurations. The fixture misleadingly returns **1 failed** and records an error row because `syncAssignment` already rejects `!path_exists`; production Rust instead recreates the folder. Therefore browser-only testing cannot establish this safety invariant.

**Impact:** moving/deleting a registered project, or losing its mount, can cause ordinary assignment/Enable to create a stray project-shaped tree at the old location, then make the next listing consider that location present again. This is the same filesystem correctness bug the new Sync All skip fixes, not simply an alternative disabled-button presentation. Pre-existing in the other paths, but explicitly in the requested audit scope and unsafe to sign off as fixed.

**Concrete fix:** enforce the existing `is_dir` project-root rule in core, under the mutation guard, before first-assignment row admission and before Enable/collision materialization; add a final shared defense at the project target-write seam. Use a typed refusal such as the existing `InvalidPath { reason: "missing" }`, or consistently extend the project-level skip policy—never prose errors or a frontend-only guard. Preserve the re-sync's one skip per project and no assignment mutations. If skips are extended to bulk assignment, **also aggregate/dedupe `skipped_projects` in `useProjectState.assignToProject`** (currently constructs `skipped_projects: []`) and cover Enable/collision folds; do not silently discard the new outcome. Keep the picker informative/disabled for a missing folder, but treat it as UX, not the safety boundary. Bring fixture semantics into parity with the chosen core behavior.

**Boundaries verified:** registration and update-path canonicalize an existing directory; neither deliberately creates a missing project. `gitignore::update_for_project_unlocked:240` refuses a missing directory. `propagation::propagate_one_assignment:423` already skips an absent project path (it uses `exists`, unlike the newer shared `is_dir` rule). The new `resync_assignments_unlocked:675` correctly skips before its loop. Do not accidentally block removal/cleanup of rows just because their folder is gone.

### N1 — P2: 127 Tab presses across the 25-skill matrix

**Location:** `src/components/projects/AssignmentMatrix.tsx:209–256,304–344,388–435`.

**Reproduction/evidence:** `extra-*.json.tabWalk`: from “alias-skill in Claude Code” to “zod-schema-first in Windsurf”, **127** actual trusted Tab presses. This includes 75 cell checkboxes, persistent row verbs (including focusable refused verbs), and repair controls in between. No BODY or unreachable cell.

**Impact/fix:** the native table is accessible but inefficient for repeated large-library work. Search and Assigned/Issues provide a current workaround. Record a 19b keyboard-efficiency follow-up: design spatial/roving navigation with named cell actions and a clear Tab exit, then test it as a complete interaction model. Do not merely add `role=grid` to the current table. D23 only requires structural collapse; the signature j/k/x contract is scoped to library row navigation. **A roving grid is not a D31 requirement for ticket 17.** Coordinate with ticket 23's surface/palette work; it need not delay this fix.

### N2 — P2: repairing the last issue in a filtered row drops matrix context

**Location:** `src/components/projects/AssignmentMatrix.tsx:230–257,365–374`; fallback `src/lib/actionLifecycle.ts:318–337`.

**Reproduce:** monorepo → Issues → Sync design-tokens in Claude Code. Its sole stale assignment becomes synced, so the row leaves Issues. The cell-owned layout effect cannot focus its checkbox because the whole row unmounts. The lifecycle puts focus on **MAIN**. Reproduced in both themes; `extra-*.json.filteredBefore/After/Frames`, `filtered-repair-*.png`; no sampled BODY frame.

**Impact/fix:** the operation completes and the documented safety fallback works, so not a P1 blocker. However the operator loses their position in a workflow meant for sequential repairs. Add matrix-level post-filter focus reconciliation: next surviving row/cell, previous if last, then the active chip/heading if empty. Cover Sync this and unassign-last under Assigned. Keep this with N1's focus model, not a cell-level query for a node that no longer exists.

## Rulings on the implementer's three concerns

1. **Open menu axe `region`: inherited, accepted popup exception—not a demonstrated landmark-navigation defect.** The current and parent `DropdownMenu` files are byte-identical (`cmp`, exit 0); a fresh My Skills row menu reproduces the same violation in both themes. The node is the named, focused `role=menu` portal directly under body, with `aria-labelledby` its trigger. Axe's best-practice rule truthfully finds content outside a landmark, but a transient composite popup is not a document region that needs its own landmark. Do **not** add a bogus region/main role merely to green axe. Treat this as a **P2 audit-policy note**, optionally documented centrally during 19b/ticket 23: allowlist this narrow named-popup case after keyboard/focus verification, not all `region` findings. The triggerless Popover R1 remains separately deferred; this matrix does not use it.
2. **Long Tab walk: N1, P2; no new grid required for acceptance.** Exact measured count and proposed future scope above.
3. **Missing-folder assignment: A1, P1; core fix required.** It is reachable from the inspector/bulk assignment door and proven against actual Rust filesystem operations. UI refusal alone is insufficient, and the fixture currently masks production behavior.

## Cell-state table and measured contrast

Ratios are **dark / light**, normal background. Every cell word is `muted-foreground`: **8.31 / 8.16** (hover raised **7.08 / 7.20**). All six words were independently tested visible on focus **and** hover, not inferred from colour. Names below are DOM `aria-label`s, not imagined screen-reader announcements.

| State | Mark | Word | Actual checkbox name | Mark ratio |
| --- | --- | --- | --- | --- |
| Synced | Filled dot | synced | `release-train in .agents/skills: synced — the copy matches the skill` | **8.31 / 8.16** |
| Stale | Half-filled ring | stale | `design-tokens in Claude Code: stale — copy differs from the skill — Sync to fix, also global` | **10.41 / 5.62** |
| Missing | × | missing | `docx in Claude Code: missing — the deployed copy is gone — Sync to fix, also global` | **6.91 / 5.50** |
| Pending (stored) | Hollow ring | pending | `release-train in Claude Code: pending — assigned, not synced yet, also global` | **3.90 / 3.78** |
| Failed | × | failed | `tdd in .agents/skills: failed — Permission denied (os error 13): /Users/alex/Projects/monorepo/.agents/skills/tdd — Sync to retry` | **6.91 / 5.50** |
| Disabled | Slashed ring | disabled | `retired-playbook in Claude Code: disabled — the skill is off everywhere — Enable it to sync` | **6.12 / 6.09** |
| Also global | Globe, in addition to state mark | legend: also global | e.g. `alias-skill in Claude Code: not assigned, also global` | **8.31 / 8.16** |
| In flight (not stored pending) | LoaderCircle | syncing… | `alias-skill in Claude Code: syncing…, also global`; `aria-busy=true` | muted token **8.31 / 8.16** |
| Not detected | Column text, no invented cell state | not detected | header `Windsurf`, followed by `not detected`; cell still names Windsurf | faint text **6.12 / 6.09** |

Fresh `final-probes-*.json` preserves earlier floors: input/hollow mark **3.900/3.777 background**, **3.651/3.519 card**, **3.324/3.332 raised**; faint text **5.217/5.372 raised**; ring/selected edge **6.013/5.037 raised**. Actual dark selected edge measured **6.013**. Light header select: text **17.32**, border **3.777**, measured against its actual ancestor background (the select's own background is transparent).

Whole-surface enabled-text minima in `a11y-*`: matrix **4.68/5.37**, menu **6.47/5.64**, Remove **6.91/5.64**, Add **7.78/8.36**. Disabled-control text is contrast-exempt; it is not silently treated as an enabled pair. Marks do not fade below the settled floor.

**Axe 4.10.3:** 14 full audits: list, matrix, overflow, Remove, Add, Settings, Configure Tools × both configurations. **Only the two overflow runs report `region`**. Two additional region-only My Skills checks reproduce it. Incompletes are retained in JSON: Base UI `aria-hidden-focus` guards (2 on dialogs, 6 on menu), menu `aria-valid-attr-value` (1), and a few contrast nodes (dark matrix/menu/tools). Incomplete is not “pass”; manual measured pairs and trusted focus checks supplement it. No native screen-reader conformance certificate is inferred.

## Gate and source integrity

Fresh independent command output in `gate.log`:

```text
npm run version:check && npm run check
Version OK (1.2.18)
styles:check ok — App.css 1108 counted lines (ceiling), 36 migrated file(s)
Test Files 45 passed (45)
Tests 973 passed (973)
build: TypeScript project check + Vite succeeded
rust fmt / clippy succeeded
test result: ok. 809 passed; 0 failed
GATE_EXIT:0

cd src-tauri && cargo test --all
809 passed; 0 failed
ALL_EXIT:0

cargo test export_bindings
1 passed; 0 failed; 808 filtered out
BINDINGS_EXIT:0
git diff --stat -- src/bindings/index.ts: empty
```

Additional targeted re-sync run (`resync-tests.log`): **8 passed**, including both new skip/one-assignment tests. Impeccable detector `[]`. Deleted CSS families/modules checked; removed i18n logical keys unreferenced, catalog regression passed. No production source changes, no stash/revert/commit. The evidence-only probe intentionally asserts the existing bug; it is not offered as a passing regression for the desired behavior.

## Acceptance — exact re-check scope

**Do not close ticket 17's D31 acceptance until A1 is fixed.** The rest of the measured gate is accepted with N1/N2 and the narrow menu exception recorded. There is no need to redesign the matrix to fix the filesystem invariant.

1. Add temp-root Rust regressions: create/register, delete/move the project root, then **bulk assign, toggle-on, Enable and collision Overwrite**. Assert no root/artifact recreation and the chosen typed refusal/skip semantics, including assignment status/ownership invariants. Keep Sync Project/Sync All/Sync this at one project skip and no per-assignment failures. Check absent-path update/registration/gitignore/propagation remain safe and removal remains allowed.
2. Test a root disappearing **after the UI view/picker loaded**; a frontend guard must not be the proof. Update fixture parity and regression tests. If the report vocabulary changes, export bindings and test every accumulator/fold so project skips cannot vanish or multiply by skill.
3. Recheck My Skills → local-notes → Assign to project → archived-app in both configurations: clear refusal/recovery, no misleading success; verify the ordinary valid-project assignment still applies its returned view with **0 follow-up reads**. Recheck Enable and collision retry with a missing folder at the core/fixture boundary.
4. Rerun the normal ticket keyboard flow and Sync All line after the fix; affected hook/fold/fixture tests, `npm run version:check && npm run check`, `cargo test --all`, binding-drift check. Repeat axe on a changed picker/refusal surface in both themes; repeat contrast only if style/token pairs change. Broader primitive/motion retest only if those owners change.
5. Orchestrator disposition: keep N1/N2 as explicit 19b keyboard/focus follow-ups (ticket 23 coordination), and document/accept the narrow menu axe exception rather than injecting an unnecessary landmark. Operator VoiceOver/reduced-motion/native minimum-window closure remains separate.
