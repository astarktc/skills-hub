# Ticket 31 — Spec review and adversarial audit

**Verdict: FAIL** — two reproduced P1 functional gaps (F1, F2); one P2 guard limitation (F3). The row/group implementation otherwise substantially meets the ticket. No source fixes made; no commit made.

Reviewed the uncommitted change plus `src/components/skills/listColumns.ts` in `/Users/alexstark/.t3/worktrees/skills-hub/t3-fc2a282d`, against `e177456`, on 2026-11-04 (local date). Read ticket 31 Work, Proof, Result and all Comments, D38–D42/D44, and tickets 30/35 Results and accepted amendments. Opened both reference HTML pages through my fixture server: `30-colour/index.html?variant=A` and `30-dark-ground/index.html?ground=f`. Used the web-design-guidelines skill and fetched its current rules. Browser: ego-browser, own task space, own Vite on **5178**, in-memory fixture only. Captures are under `/tmp`.

The parallel standards pass/orchestrator changed the on-screen fold during this audit (moved to `skillPresentation.onScreenOrder`, with two tests). I re-read that change and repeated the frontend guards after it: final test count **1146**, not the child's earlier 1144. Neither reproduced functional gap was repaired in the audited tree.

## Spec

### (a) Asked for, missing or partial

- **Project state does not stay current after project actions.** Ticket: “**Scope cell**: globe when a global deployment is live, **and/or** folder + N when N projects hold ≥ 1 live deployment”; and “`skillHealth`: failed project deployments … join `targets_failed` / `targets_missing`.” The backend and initial rendering implement this, but the library's new project data is not reconciled after the project world changes assignments. The result can state a deployment exists when none exists. **F1**.
- **Toast reservation covers one toast, not the expanded stack.** Ticket: “**#80**: the toast stack no longer covers the header controls — reserve space (verify 960 and 1280) without `pointer-events: none` and without dropping hover-pause globally.” A single toast passes, but the supported five-toast stack covers those controls when hovered. **F2**.
- **Reverse i18n guarantee is approximate.** Ticket: “an `en` leaf no source file reads fails the test — grep-based, allow-list for keys built dynamically.” The implementation detects lexical occurrences, not reads, and grants whole dynamic prefixes automatic exemptions. No broken surviving read of any deleted key was found, so this is a **P2 limitation**, not a claim that those deletions are wrong. **F3**.

### (b) Behaviour not asked for / scope

- Ticket's deployment shape says “`deployments: { tool, status }[]`”; the additional `synced_at` is the accepted ownership enabler and is necessary to reject never-synced error markers. **Not scope creep.** Fidelity checked below.
- Ticket says “the longest visible name … clamp …”; allowance for badges/pencil and short row labels are accepted decisions. They preserve the full ladder sentence in the tooltip. **Not scope creep.**
- Ticket says “Harness initials … each with the harness's full name”; GC rather than the prototype's GM follows the accepted algorithm. **Not reopened.**
- Ticket explicitly names six dead keys, while Result deletes 55. The accompanying requirement that every leaf be read justifies deleting the other proven-dead leaves; the independent sweep found no surviving exact reads. **No unjustified deletion found.**
- Ticket/D42 do **not** explicitly specify ⌘A, j/k, shift-click or Delete semantics. The relevant wording is “A collapsed group's selection stays selected; the bulk rail's counts still include it.” Skipping hidden rows for new keyboard selection/focus while retaining existing selection is a coherent reading and is explicitly accepted in this review brief. Verified rather than relitigated.
- The neutral `solid` chip is the orchestrator's accepted correction; it renders on `raised`, not the invisible `muted` header ground.

### (c) Apparently implemented, but wrong

- “per project the skill is deployed to” and “N projects hold ≥ 1 live deployment” are wrong after the matrix removes the last assignment: **rendered N = 1, fresh wire N = 0**. F1.
- “the toast stack no longer covers the header controls” is wrong for five expanded error notices at **both** requested widths. F2. DESIGN.md's absolute “never under it” claim is consequently too strong.
- “an `en` leaf no source file reads fails the test” is not literally true when its full key is an unrelated quoted string or lies under an unrelated template prefix. F3.

## Findings

### F1 — P1: project mutations leave My Skills' project scope and health stale

**Requirement:** “folder + N when N projects hold ≥ 1 live deployment”; failed project deployments join the health rows.

**Evidence / reproduction (actual fixture UI, not an injected catalog):**

1. Open `http://localhost:5178/?scenario=rich`; let the catalog load.
2. My Skills `prisma-review` initially has no live global deployment and one project: quartermaster.
3. Projects → quartermaster → search `prisma` → toggle the deployed Claude Code checkbox off.
4. Wait for the matrix's checkbox name to become `prisma-review in Claude Code: not deployed`.
5. Return to My Skills; search `prisma`.
6. `ego-browser nodejs`, with this `page.evaluate` probe:

```js
const { fixtureInvoke } = await import('/src/fixtures/index.ts');
const fresh = (await fixtureInvoke('getManagedSkills'))
  .find(x => x.name === 'prisma-review');
const row = document.querySelector('[data-skill-item]');
return {
  rowScope: row.children[4].querySelector('[role=img]')?.getAttribute('aria-label'),
  freshProjects: fresh.projects,
  assignmentCount: window.__fixture.state.projects.reduce(
    (n, p) => n + p.assignments.filter(a => a.skill_id === fresh.id).length, 0),
};
```

Output:

```text
MATRIX AFTER [
  'prisma-review in Claude Code: not deployed',
  'prisma-review in .agents/skills: not deployed'
]
{ rowScope: 'In 1 project: quartermaster', freshProjects: [], assignmentCount: 0 }
```

Capture: **`/tmp/ticket31-stale-project-scope.png`**. Repeated the whole sequence from a fresh fixture after the final guard run at **1280×800**: `{rowScope:'In 1 project: quartermaster', freshProjects:[], width:1280}`. It still shows folder `1` after the removal.

**Location/cause:** `src/App.tsx:101` constructs the project world with only the collision seam; the library lens reads `library.managedSkills`. `src/components/projects/useProjectState.ts:279–302` applies returned views only to project/placement state; `:431–457` toggles and applies that view without replacing/updating the library catalog. `src/components/projects/useProjectActions.ts:165–179` executes the outcome without a library convergence step. Newly added `ManagedSkillDto.projects` has therefore created a second presentation snapshot with no update path from ordinary project mutations. Reading the fixture catalog itself is correct; the bug is not its query/seed.

**Impact:** adding/removing deployments can leave the row's project count false, and repaired/new project failures can leave the row mark, Issues filter/count and inspector health false until another action happens to reload the skill catalog. The child Result's “a toggle shows at once” is true of a fresh DTO read, **not** the displayed My Skills catalog.

**Exact proposed fix:** introduce an App-bound convergence seam from settled project changes to the library owner, without runtime imports between worlds. Either apply each authoritative returned project view to the `projects` slice of every managed skill (including clearing skills absent from that view, project rename and project removal), or, more simply, pass `library.loadManagedSkills` as a callback and await it once after project-changing operator actions settle, including partial/failure convergence. Do not refetch the project view on its successful return path. Cover toggle, assign/unassign, configure/remove project, resync/collision retry and path/name change; reconcile reads that change assignment status must converge the library too. Add a regression for the UI sequence above and for repairing a project-only error clearing the library issue mark. Do not fix only navigation by silently refetching on visiting My Skills.

### F2 — P1: hover-expanding five toasts still obscures the header and filter controls

**Requirement:** “the toast stack no longer covers the header controls — reserve space (verify 960 and 1280) … without dropping hover-pause globally.”

**Evidence:** single notices pass (see verification). The adversarial case uses the existing Toaster's supported `visibleToasts={5}` and its normal hover expansion, with persistent error messages of ordinary multi-line length. No CSS or component state was injected. `useStatusReporter.ts:35–84` itself supports description, action, infinite errors and close buttons.

Reproduction on `rich`, via `ego-browser nodejs` → `page.evaluate` (use the current Vite-versioned Sonner import from `/src/hooks/useStatusReporter.ts`; the audited server's hash is below):

```js
const { toast } = await import('/node_modules/.vite/deps/sonner.js?v=91a40565');
for (let i = 0; i < 5; i++) toast.error(`${i + 1} deployments failed`, {
  id: `stack${i}`,
  description: 'Could not deploy frontend-design to Claude Code. Permission denied. Check the folder and try again.',
  duration: Infinity,
  closeButton: true,
  action: { label: 'View report', onClick: () => {} },
});
```

Set viewport, hover `[data-sonner-toast][data-front=true]`, wait for `data-expanded=true` and the 120 ms movement to settle. Intersect each toast rect with non-list `button,input` rects.

```text
960×640, toast y intervals (newest first):
415.71875–544; 273.4375–401.71875; 131.15625–259.4375;
-11.125–117.15625; -153.40625–-25.125.
Overlaps: Configure harnesses, Update all, Add skill (y40),
Disabled, Grouped, Flat (y93), Name A–Z (y89).

1280×800, after re-hovering at the resized viewport:
575.71875–704; 433.4375–561.71875; 291.15625–419.4375;
148.875–277.15625; 6.59375–134.875.
Overlaps: Configure harnesses, Update all, Add skill (y8),
Grouped, Flat (y61), Name A–Z (y57).
```

Captures: **`/tmp/ticket31-toast-stack-960.png`**, **`/tmp/ticket31-toast-stack-1280.png`**. At 960, one notice is entirely above the viewport and another is clipped at the top. At 1280, the oldest spans the full header/filter area. The 1280 capture was re-taken after re-hovering; merely resizing an expanded stack moves the pointer out and collapses it.

**Location:** `src/App.tsx:594–602`, especially the fixed `offset={{ bottom: '6rem' }}` with five visible notices; `DESIGN.md` § Toaster placement. Bottom clearance is not a top bound.

**Exact proposed fix:** constrain the expanded stack to a reserved vertical region between the header/filter area and the bottom rail/foot, with overflow accessible through scrolling or a bounded visible-stack/history affordance. Implement the reservation in the Toaster/layout interface, document it, and preserve pointer actions, hover-pause and access to every persistent error. Merely moving the fixed offset or disabling pointer events/expansion is not sufficient. Add the five-multiline-error hover case at both viewports to the browser proof and revise DESIGN.md's absolute claim to the actually verified reservation.

### F3 — P2: reverse i18n scan is a lexical liveness approximation, not a read detector

**Requirement:** “an `en` leaf no source file reads fails the test — grep-based, allow-list for keys built dynamically.”

**Evidence:** ran the actual `leaves` and `unreadLeaves` bodies from `src/i18n/resources.test.ts:99–118`, transpiled in memory with the installed TypeScript API (`node` script; no source/test file written), against `{a:{b:'x'}}`:

```text
const k = "a.b"; t(k)       => unread []       (variable read found)
t("a." + x)                => unread ['a.b']  (concatenated read missed)
console.log("a.b")          => unread []       (unrelated string treated as read)
const unused = `a.${x}`     => unread []       (unrelated template exempts family)
```

The first result is useful, but does not represent variable tracking: it passes because the quoted key exists anywhere. Auto-detected template prefixes have no proof that they reach `t`. A future dead leaf under such a prefix is exempt even with no reachable read.

**Exact proposed fix:** add these self-tests and state the lexical limitation explicitly; use an explicit, reasoned allow-list for concatenated/dynamic families rather than silently granting every template prefix. Narrow recognized static references to translation calls and the project's translation-key maps (or a small syntax-aware extractor if chosen). Do not claim full read coverage while accepting arbitrary quoted strings. **No current deleted-key regression demonstrated:** all 55 were swept independently below, so this remains P2 rather than blocking on hypothetical future code.

## Verification ledger

### 1. Wire / Rust

- `commands/mod.rs:957–980`: `ManagedSkillDto.projects` is required; both new DTOs derive `Serialize + Type`; deployment `synced_at: Option<i64>`.
- Generated `src/bindings/index.ts:792,1350–1366`: `projects: SkillProjectDto[]`; `status: SyncStatus`; `synced_at: number | null`, not optional, not bigint. `components/skills/types.ts:15` re-exports both DTOs.
- `skill_catalog.rs:120–137`: **one project query before mapping the skill list**, `list_skill_project_deployments(None)`. The per-skill builder receives the grouped rows; no per-skill project query in the catalog mapping. Existing per-skill **global target** reads remain, unrelated to this new project query.
- `skill_catalog.rs:141–156`: single `managed_skill_entry` uses `Some(skill.id)` and produces the same project result. Tested.
- `skill_store.rs:1389–1434`: one JOIN, no disabled exclusion, deterministic `skill_id, project path, tool` ordering. Core fold sorts project names case-insensitively then project id; deployments inherit deterministic tool ordering. `project_name_from_path` is the same helper used at `project_ops.rs:85` for the Projects world.
- Fresh commands:

```text
cd src-tauri && cargo test export_bindings
  1 passed; 838 filtered out
cargo test --manifest-path src-tauri/Cargo.toml skill_catalog
  9 passed; 830 filtered out
```

The first attempted filter `core::tests::skill_catalog` matched **0** tests; I corrected it to `skill_catalog` and obtained the nine-test run above. The new catalog test covers one skill in two projects, multiple harness rows, disabled, never-synced error, no assignments, and the single-entry path; the other new test proves query failure fails the catalog.

Binding SHA-1 before/after regeneration: **`90f64cf80e2104a07d8acf625e615ec349392964`**, identical. `git status --short src/bindings` prints **` M src/bindings/index.ts`** because this is an uncommitted feature diff. It is not literally clean relative to HEAD; there is **zero regeneration drift relative to the supplied change** and no untracked binding. That is the correct interpretation of the requested regeneration check for this worktree.

### 2. Folds, initials and grouping

- Constructed a skill with no global targets, one project-only `error` with `synced_at:null`, and another project with only a disabled deployment. Actual imported production folds returned:

```text
scope = {global:false, projects:[]}
health.targets_failed = {count:1, tools:[], projects:[{id:'p1',name:'api',tools:['pi']}], repairs:[]}
rowMarks = [{count:1,key:'library.row.marks.failed',repairs:[]}]
```

Thus a project-only failure still renders **“1 failed”**, but never offers a no-op global retry. The fixture's `writing-plans` proves the rendered form and tooltip: `1 deployment failed — In quartermaster (.agents/skills)`. Inspector shows `1 deployment failed` and names quartermaster without Retry.
- `skillScope` uses `isLiveTarget`; disabled and never-synced error rows do not count. Existing `deployAction` / `isLiveTarget` global behaviour remains unchanged. Tests cover mixed global+projects, project-only, non-live project rows and disabled skills.
- `rowMarks` excludes all three silent kinds; full-page AX snapshot plus DOM scan found **0** “Not deployed” health badges. Per-harness hollow marks still correctly announce “not deployed”; that is not the forbidden repeated row badge.
- `nameColumnWidth([])` → `clamp(12rem, 0ch, 22rem)`; a 100-character name with a failed badge and pencil → `clamp(12rem, 119ch, 22rem)`. Rendered CSS clamps at 352 px in the failures fixture. `SkillsList.tsx:118–133` supplies only `onScreen` rows, including mark/pencil allowance; hidden collapsed rows are excluded.
- Unfiltered singleton grouping is pinned by the fold and lens tests. Browser search `pptx` keeps the `anthropics/skills` header while showing one skill. Truly singleton sources still live in Individual sources.
- Ran **the actual `harnessInitials` body** over every `display_name` extracted from `core/tool_adapters/mod.rs`, in registry order: **45 entries, 45 distinct marks, collisions `[]`**. This includes the virtual `.agents/skills` entry. The eight fixture harnesses are independently tested as `CU CC CO AM KC PI GC WI`; the rich page's effective selection is seven (Windsurf excluded), not eight active columns.

Full registry result (important collision cases included, no hand-written override):

```text
.agents/skills AG; Cursor CU; Claude Code CC; Codex CO; OpenCode OP;
Antigravity AN; Amp AM; Kimi Code CLI KC; Augment AU; OpenClaw OE;
Copaw CP; Cline CL; CodeBuddy CD; Command Code CM; Continue CN;
Crush CR; Junie JU; iFlow CLI IC; Kiro CLI KI; Kode KO; MCPJam MC;
Mistral Vibe MV; Mux MU; OpenClaude IDE OI; OpenHands ON; Pi PI;
Qoder QO; QoderWork QD; Qwen Code QC; Trae TR; Trae CN TC;
Zencoder ZE; Neovate NE; Pochi PO; AdaL AD; Kilo Code KL;
Roo Code RC; Goose GO; Gemini CLI GC; GitHub Copilot GI;
Clawdbot CA; Droid DR; Windsurf WI; MoltBot MO; Hermes Agent HA.
```

### 3. Collapse, bulk counts and keyboard

- `preferences.test.ts:34–36` pins **`skills-collapsedSources`**. Hook tests cover storage/readback, query and chip expansion/restoration, no-collapse surfaces, and all/expand actions. `onScreenOrder` tests cover filtering and Flat behaviour.
- Browser: collapse anthropics with no query → storage `["anthropics/skills"]`; search `pptx` → expanded; clear → collapsed again. Collapse all while searching `pptx` collapses **only anthropics**, does not write stored `[]`, and clearing restores all groups. Menu contains both Collapse all / Expand all; Collapse all with no query produced zero row elements and only the six shown group keys in storage. Independent reload proof: collapse mattpocock then reload → stored `["mattpocock/skills"]`, **32** rendered rows, **0** rows in that group.
- **Bulk selection actually survives:** selected `artifacts-builder`, collapsed anthropics → `1 selected`, 51 row elements, stored collapsed key. Then selected all nine anthropics rows, collapsed them and invoked the bulk Disable confirmation. It read:

  > Disable 9 skills on 7 harnesses — 56 global deployments are removed and kept disabled; 2 project deployments for these skills are disabled too. Updates will not re-add them. 1 of them is already off everywhere.

  The one already-disabled skill was intentionally disabled earlier in this fixture session. All nine hidden selected skills remained counted, including its already-disabled count. The confirmation was cancelled. Source chain: `App.tsx:458–461` resolves selection against the **full library**, `BulkRail.tsx:39–43` passes that list to `disableCounts` / `enableCounts` / `updateCounts`; no `onScreen` filter enters the bulk counts.
- **⌘A:** with anthropics collapsed, search Enter focuses the first on-screen row; ⌘A returned **51 selected / 51 on-screen**. This matches the accepted visible-row interpretation, while an existing hidden selection is retained until deliberately cleared.
- **j/k across hidden groups:** collapse mattpocock; focus xlsx (`skill-032-xlsx`); `j` → brainstorming (`skill-038-brainstorming`), `k` → xlsx. It skips all 28 collapsed rows.
- **Delete focus, actual action:** with mattpocock collapsed, removed xlsx through its menu and confirmation. After the fixture backend settled, `document.activeElement.dataset.skillRow` was **`skill-038-brainstorming`**, not a collapsed mattpocock row and not body.
- **Trusted shift-click:** with mattpocock collapsed, click the xlsx checkbox, hold Shift with `page.keyboard.down('Shift')`, click brainstorming's checkbox, release Shift. Browser result: selected ids **`['skill-032-xlsx','skill-038-brainstorming']`**, rail **2 selected**, not the intervening 28 hidden rows. This exercises the `visibleOrder = lens.onScreen` passed into the existing tested keymap.

### 4. Browser AX / focus / sticky header

At dark 1280×800:

- Header Tab order observed: **group checkbox → chevron → source link → first row checkbox** when Deploy all is disabled. Deploy all has a native disabled state and is skipped, correctly. After waiting for startup scanning to finish, the enabled mattpocock sequence was independently walked: **checkbox → chevron → source link → Deploy all → first row checkbox**.
- Checkbox, chevron and source link each matched `:focus-visible` with **2 px solid `rgb(150,144,255)` outline**. Capture `/tmp/ticket31-chevron-focus.png`.
- Focused chevron ← → output: `Expand anthropics/skills`, `aria-expanded=false`, 0 rows; then `Collapse anthropics/skills`, `aria-expanded=true`. This also exercises the header's bubbled arrow handler.
- My Skills search: Enter after `pptx` focused **`skill-031-pptx`**; ↓ did the same. After clearing with anthropics stored collapsed, Enter picked the first mattpocock row, not hidden anthropics.
- AX snapshot carries `image "Deployed globally · In 3 projects: agent-skills, quartermaster, skills-hub"` for react-perf-audit and `image "In 1 project: quartermaster"` for project-only prisma-review. One labelled scope image, no duplicate child icon names.
- Header initials are labelled images inside group `Harnesses`, with full catalog titles/names. The eight-tool failures page includes Windsurf's WI too.
- Sticky proof: scrolling the long mattpocock group to `scrollTop=738` gave **pane top = 98, header top = 98, position = sticky**. Capture `/tmp/ticket31-sticky.png`. `overflow:clip` on the card does not break its sticky ancestor.
- Inspector opened and its health ladder named the failed project (`/tmp/ticket31-inspector.png`); Projects matrix opened and operated (`/tmp/ticket31-projects.png`); Explore loaded seven featured rows after waiting for loading to finish (`/tmp/ticket31-explore.png`). No component rendering tests were added or run ad hoc.

### 5. Layout and accepted grounds

Six fresh captures: `/tmp/ticket31-{dark,light}-{960,1280,1800}.png`. Sizes: 960×640, 1280×800, 1800×1000. Light capture includes the theme menu, but measurements are of the actual unobscured row grid.

| Viewport | Pane width | Name | Description | Strip | Horizontal overflow |
|---|---:|---:|---:|---:|---|
| 960 | 889 | 278.406 | 140.594 | 180 | no |
| 1280 | 1065 | 278.406 | 316.594 | 180 | no |
| 1800 | 1585 | 278.406 | 836.594 | 180 | no |

Identical dimensions in both themes. Seven effective marks give **7×20+40 = 180**. The eight-mark failures fixture gives **200**. Names and descriptions share aligned column starts; descriptions are one-line truncations, not wrapping. Name title is always the complete `skill.name`; description title is source + full description. CSS max is 22 rem / 352 px.

**960 judgment:** D39 says the longest visible name is clamped to 12–22 rem and “the description takes the rest; the list keeps no max width.” Ticket 31 further says “below the 48 rem container the description still drops.” At 960 the pane is 889 px, above that threshold. **141 px is cramped but conforms**; no evidence-based requirement says to hide it earlier. The adversarial failures fixture reaches the cap (352 px), leaving **47 px** of description at 960, with no overflow (`/tmp/ticket31-failures-960.png`). That is a useful future responsive-design consideration, not grounds to silently change the accepted breakpoint in this ticket. With the inspector open the pane is 705 px and description display is `none`, as specified.

Computed styles / captures confirm:

- Cards `overflow:clip`, rounded corners and border; header dark **rgb(4,4,4)**, rows **rgb(0,0,0)**; light header **rgb(240,241,244)**, rows white.
- Hover **rgb(14,14,14)** (`accent`).
- Focused row **rgb(30,30,30)** (`raised`), radius **6 px**, margin-left **4 px**, **1 px inset iris ring**.
- Selected row **rgb(30,30,30)** with **3 px** `rgb(150,144,255)` edge.
- Header-only identity chips visibly read; no per-row chip added. Ownerless chips compute to **rgb(30,30,30)** with **rgb(245,245,245)** text/icon in dark; visible in the failures capture. The accepted solid-neutral adjustment is applied.
- Reference captures: `/tmp/ticket31-reference-A.png`, `/tmp/ticket31-reference-f.png`; structure matches the accepted card/header/row anatomy, not the discarded per-row-chip option.

### 6. Riders

**#81:** `index.html:7` is `<title>Skills Hub</title>`. Fixture deliberately overrides it to `Skills Hub · fixture: rich` (`fixtures/index.ts:153`), not a production regression. Source sweep found four search inputs, all with autocorrect off, autocapitalize off and spellcheck false. Browser verified these attributes on My Skills, Projects, Explore and the Add flow's CandidatePicker (opened Source → Continue, did not install). Enter/↓ proof above.

**#80 single-toast baseline:**

- 960×640, notice with description/action: **y470.305–544**, rail **599–640**, pointer-events **auto**. Only scrolling row controls intersected, no header/filter/rail controls. Capture `/tmp/ticket31-toast-hover-960.png`.
- 1280×800, single action notice: approximately **y646–704**, rail **759–800**. Capture `/tmp/ticket31-toast-1280.png`.
- Real hover-pause tested with `duration:2000`: hovered for **2.6 seconds**, notice still present; moved pointer away, after **2.6 seconds** zero notices. This proves actual timer behaviour, not just an unchanged prop.
- Expanded stack fails: F2.

**#82 deleted-key sweep:** scanned every one of the **55 exact key strings** listed in Result across all `src/**/*.ts,tsx` (including tests), using a Node file walk and per-line substring search. No surviving exact translation read found. Non-empty lexical hits were only:

| Deleted root key | Surviving hits (not reads of the deleted key) |
|---|---|
| `groupByRepo` | preference/property identifiers in App, lens, preferences/tests, AssignmentMatrix and FilterBar; storage names `skills-groupByRepo`, `skills-projects-groupByRepo` |
| `toolsLabel` | `toolsLabelText` prop in `App.tsx:857`, `modals/NewToolsModal.tsx:15,24,42` |
| `sourceLabel` | presentation/component variable/property names in `useAddSkillFlow`, `explorePresentation`, ExplorePage, SkillRow and SkillsList |
| `pathLabel` | distinct live keys **`projects.pathLabel`** and **`changeSource.pathLabel`** in catalog, Add/EditProject, useProjectActions/tests and RepointModal |
| `storagePath` | Settings state/prop/test identifiers in App, useSettingsState and SettingsPage |

All other 50 searched strings had **zero substring hits**:

```text
subtitle
newSkill
invocationEdit.overridden
invocationEdit.overrideTooltip
invocationEdit.conflictTooltip
installSuccess
filterSort
allSkills
scanNow
skillsTitle
activeTools
updatedLabel
statusLabel
localImportTitle
gitImportTitle
deleteTitle
deleteBody
confirmRemove
maintenance
restartNow
updatesMaintenance
autoUpdateSkills
autoUpdateDesc
storageCleanup
storageCleanupDesc
rescanTools
changeSource.confirm
searchTab
exploreTab
exploreSource
errors.skillNotFoundInRepo
delete.confirmPrefix
delete.confirmSuffix
delete.warningRemoveFromTools
delete.warningDeleteFromHub
delete.confirmButton
detail.files
detail.readError
detail.fileCount
unsyncAllComplete
unsyncPartial
unsyncSkill
projects.assignmentCount
projects.toolCount
projects.skillCount
disabledSkips.deployDisabledPairs
repointSource.was
repointSource.missingFolder
shell.shortcutBack
projectsSurface.cell.alsoGlobalTitle
```

The five non-empty lexical-hit groups are classified in the table, rather than treated as reads of the deleted keys. The matcher counterexamples and their disposition are F3. Both directions of the current i18n test are green.

### 7. Regression and styling guards

Final fresh command sequence, all exit 0:

```sh
npx vitest run src
npx tsc --noEmit -p tsconfig.app.json
npm run styles:check
git diff --check
```

```text
Test Files 53 passed (53)
Tests 1146 passed (1146)
styles:check ok — App.css 812 counted lines (ceiling), 39 migrated file(s),
prose layer scoped, no raw colours; self-test passed.
```

This includes inspector hook tests, `useSkillActions`, `useSkillListKeymap`, `useListLens`, `bulkCounts`, `actionLifecycle`, project state/actions/focus/presentation, Explore state/presentation, fixture backend and the bidirectional resources guard. No full Rust gate was repeated; the supplied full gate is not substituted for the fresh nine-test catalog and binding checks above.

`styles-baseline.json` appends **only `src/components/skills/listColumns.ts`** to migrated; ceiling remains **812**. `rg -n 'skill-row|group-header|synced-strip' src/App.css` finds **no rules**, so this change did not orphan such legacy selectors. Matches elsewhere are data attributes, tests, a theme comment and focus selectors, not residual App.css consumers. `App.css` itself is unchanged.

DESIGN.md § My Skills agrees with rendered pitch, columns, header-only chip, initials, scope and collapse semantics; its compact-list rule matches the 705 px inspector pane. The only material documentation contradiction found is the unconditional toast-clearance assertion (F2).

## Goal-backward conclusion

**FAIL.** The initial My Skills screen now has the requested card/group anatomy, accurate initial wire data, distinct harness initials, proper collapse/selection behaviour, and substantially better aligned descriptions, with all fresh structural guards green. However, the user's goal is a trustworthy deployment overview during real use, not just after launch: the new project scope becomes false after an ordinary matrix edit, and the supposedly reserved toast stack again covers the next-action controls when expanded. Fix F1 and F2 and repeat those exact browser sequences before calling ticket 31 complete; F3 can remain a documented P2 only if the reverse test's guarantee is stated honestly.
