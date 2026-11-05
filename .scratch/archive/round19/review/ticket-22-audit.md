# Ticket 22 — independent D31 audit

- **Change:** `git diff 9fa54f3..a2a09bd` (Explore as tracker rows).
- **Audited checkout:** `274970b3d9ea61f25effeedf48e65f501c6394a1`; its only difference from `a2a09bd` is the ticket's status line. Application code is identical.
- **Date:** 2026-11-02.
- **Model:** GPT-6 Astra (`PI_MODEL=gpt-6-astra`), independent of the implementer.
- **Worktree:** `/Users/alexstark/.t3/worktrees/skills-hub/t3-10b7880d` (verified with the first tool call).
- **Verdict: FAIL — F1 P0 and F2 P1.** The prescribed cancel-without-install keyboard path passes, but completing an Add exposes a persistent focus loss; an installed online preview can also revert to Install.

## Method and evidence

Read the ticket's Work, Gate, Result, deliberate leftovers and every Comment; AGENTS.md's surface/list-lens/Add/document/test/styling rules; spec D22/D23/D25/D31/D38/D45; DESIGN.md § Explore; ADR-0006 §6. Reviewed the specified diff, both Explore components, the presentation fold/tests, lens/tests, Explore world and App's wiring, plus the relevant Add and matrix tests.

Used **ego-browser**, one task space, with an independently served fixture:

```text
npx vite --mode fixture --port 5180 --strictPort
http://localhost:5180/?scenario=rich#/explore
http://localhost:5180/?scenario=failures#/explore
http://localhost:5180/?scenario=rich&latency=3#/explore
```

Evidence is under **`.scratch/round19/evidence/22-audit/`** in this worktree (gitignored). No production library, database or Tauri dev window was used. Installation reproductions changed only the fixture's in-memory library. The browser task space is closed and the fixture server is stopped; `lsof -nP -iTCP:5180 -sTCP:LISTEN` returned no listener. No code changes or commits.

One initial keyboard attempt was interrupted by the full gate's generated-bindings write triggering Vite HMR. It was discarded and the affected flows rerun after the gate finished. The final `flow-*` files are uninterrupted runs. Early exploratory `keyboard-1280.json` is not the canonical transcript.

## 1. Keyboard-only flow — FAIL (F1)

**Passing baseline:** four fresh-load runs, `flow-1280-dark.json`, `flow-1280-light.json`, `flow-960-dark.json`, `flow-960-light.json`. Each records every dispatched key and active element, with additional settled-focus observations at asynchronous boundaries. The narrow run is genuinely a Dialog sheet; the wide run an aside.

The common focus transcript, without pointer clicks or programmatic focus:

1. Tab through **My Skills → Explore → Projects → Notifications → Theme: System → Keyboard shortcuts → Settings → Add skill → search**. Search has a native associated label **Search Explore** (`INPUT.labels[0]`), not merely a placeholder. Actual VoiceOver speech was not measured.
2. Type `t`: **Filtering featured skills · Type 2 or more characters to search skills.sh too**. Type `s`: **Featured skills and skills.sh results for “ts”**. The minimum-length hint is intentionally absent once the minimum is met, consistent with DESIGN.md.
3. ↓ **ts-reset** → ↓ **changelog-writer** → ↑ **ts-reset** → ↓ **changelog-writer** → Enter. During acquisition focus stays on the row, then arrives on the preview's **H2 changelog-writer**.
4. Tab **Close preview** → Tab **Install** → Enter. Add lists **acme/changelog-skill**, **1 of 1 checked**, the `changelog-writer` checkbox checked, and **Deploys to Cursor, Claude Code, Codex +4**. No installation occurs from Install alone.
5. Escape → after Add's close transition **Install**. Escape → **changelog-writer row**.
6. Tab **More actions for changelog-writer** → Enter → menu **Preview** → ↓ **Hide from Explore** → Enter → **ts-migration-planner**, the next visible row.
7. Shift-Tab through the preceding row's overflow/name to **Hidden** → Enter → Tab **changelog-writer** → Tab **Unhide changelog-writer** → Enter → **search**, because Hidden is now empty.
8. Tab **Not in library** → Tab **Installed** → Enter → Tab **Hidden** → Tab **docx** → Enter → `#/skills/skill-029-docx`, settled focus **managed inspector H2 docx**.

The same search/preview/Add/Escape chain also passed at **960×640 light in `failures`**, with scope **+5** (`flow-failures-960-light.json`).

Supplemental `unhide-neighbours.json` exercises all three removal fallbacks: with Hidden containing ts-reset/changelog-writer/find-skills, unhide first → **next changelog-writer**; unhide last → **previous changelog-writer**; unhide final → **search**.

**Failure after actually adding:** the preview correctly changes to **Open in My Skills**, but its original row leaves the default Not-in-library chip. Escape then leaves focus persistently on **BODY**, in both placements. `f1-keyboard-1280.json` is a fresh-load, entirely keyboard-driven reproduction; `installed-preview-close-sheet.json` reproduces at 960×640. See F1.

**Timing disclosure:** all four baseline transcripts capture a momentary BODY immediately after Enter on the installed row, followed automatically by H2 on the next view commit. Thus the literal “never BODY at any sampled instant” is not proven even on that successful route. This self-recovering navigation frame is not counted as a separate finding; F1 is a settled, persistent BODY state. No settled MAIN stop occurred in the prescribed flow.

Code: `ExplorePage.tsx:179–211`, `ExplorePreviewView.tsx:54–64,155–175`, App's installed-focus callback at `src/App.tsx:516–519`.

## 2. Preview module and cancellation — PASS

- `ExplorePreviewView.tsx:54` calls `useSkillDocument({ root: preview.cachePath, version: preview.fetchedAt })`; the body passes that document to `SkillFiles`.
- `useExploreState.ts:146–154` creates only an `ExplorePreview` (URL, name, summary, cache path, acquisition timestamp). No forged `ManagedSkill` is constructed. The narrow object passed to the pure `skillSourceLine` fold is source presentation data, not a managed record.
- Rendered copy is **Fetched for preview just now**, not updated/fresh. The wire/document comment was corrected in `useSurfaceState.ts:44`.
- With `latency=3`, the header says **Loading preview** and offers **Cancel**, accessible name **Cancel Preview**. Cancelling removes the progress control after settlement, opens no preview and returns focus to **changelog-writer's row**. Evidence: `cancel.json`; implementation: `useExploreState.ts:139–155`, App's header-progress wiring.

## 3. Install intent and installed-preview correctness — FAIL (F2); intent itself PASS

- `src/App.tsx:503–507`: `openAdd({ source: { kind: "git", url: sourceUrl }, preselect: skillName })`.
- `ExplorePage.tsx:458` now supplies **both** `preview.sourceUrl` and `preview.name`; the missing-name issue in the Result is fixed.
- Runtime proof: changelog-writer opens the one-candidate repository with that candidate checked; the online-only `jsdoc-to-ts` opens **mattpocock/skills with 1 of 31 checked**, not all 31. The scope line remains visible. Install never installs by itself; **Add selected** is still required.
- The checked/absent/ambiguous rules are covered by the passing `src/hooks/useAddSkillFlow.test.ts:638–695`: resolved name checks the resolved subpath, `none` records `preselectMissing` and checks nothing, ambiguous checks every returned matching subpath. The production branch at `useAddSkillFlow.ts:425–445` consumes the backend's tagged match. The fixture's ordinary catalog does not provide an ambiguous-name or absent-name Explore row, so those cases were verified through code and the Add tests, not invented fixture data.
- Retained previews correctly become **Open in My Skills** immediately after Add, including an Add with deployment failures. **However**, for online-only rows that installed state is lost when the search changes or clears. See F2 and `online-installed-query-clear.json`.

## 4. List-lens regression — PASS

Fresh targeted command:

```text
npm run test -- src/hooks/useListLens.test.ts src/lib/skillPresentation.test.ts \
  src/components/projects/useMatrixFocusReconciliation.test.ts \
  src/lib/projectPresentation.test.ts src/hooks/useAddSkillFlow.test.ts
5 files passed; 201 tests passed
```

Evidence: `targeted-tests.log`.

- Existing lens tests still cover query/name/updated sort, repo grouping and singleton folding, visible selection/display order, whole-catalog chip counts, independent `skills-groupByRepo` and `skills-projects-groupByRepo` preferences, and the matrix's list-only view mode.
- Matrix coverage is named explicitly: `projectPresentation.test.ts` **matrixFilters** (Assigned and Issues) and **matrixFocusAfterLeave**, plus `useMatrixFocusReconciliation.test.ts`. There is no separately named “matrix lens” test file; the shared hook's matrix-preference case is in `useListLens.test.ts`.
- The default query/sort path still invokes `filterAndSortSkills`; only surfaces that supply `fold` bypass it (`useListLens.ts:102–112`). Existing My Skills and AssignmentMatrix calls supply no fold. No matrix code changed.
- Fixture My Skills search `ts`: eight rows in Name A–Z order. Grouped yields **mattpocock/skills (6)**, **Individual sources (1)**, **Local (1)**; Flat returns the exact original eight-row sequence. Evidence: `my-skills-lens.json`. This preserves the current behavior, including existing singleton folding; it does not claim ticket 31's future grouping changes have landed.

## 5. Chips, counts and online query treatment — PASS, with explicit interpretation

`EXPLORE_FILTERS` (`explorePresentation.ts:139–143`) partitions candidates into Not in library / Installed / Hidden; a hidden installed candidate belongs only to Hidden. The default is Not in library.

Observed `rich`: initial **6 / 28 / 0**; query `ts` **6 / 33 / 0**, with **3 Featured rows** and no uninstalled online rows. Observed `failures`, query `ts`: **34 / 5 / 0**, while the selected chip shows **10 Featured + 5 From skills.sh rows**. Counts are totals over the **current candidate universe**, not matching-row counts and not the entire managed library. Online results expand that universe, so totals can change with the query even though the count fold itself is unfiltered.

Judgment: the bare chip totals can initially look like match counts, but the separate section counts make the actual result size explicit; this is the documented shared-lens convention, not a gate failure. If clarified later, label these as totals rather than changing My Skills/matrix count semantics incidentally.

Online rows are not re-filtered locally (`exploreQueryFold:150–160`). This is appropriate for provider-ranked/semantic results. In the fixture, every returned online row matched the fixture backend's name/source predicate; no unrelated online row was observed after settlement. The fixture cannot establish the quality of real skills.sh ranking. F2 is a separate error caused by using that query-dependent universe to decide a preview's installedness.

## 6. Rendered contrast — PASS

Measured with **getComputedStyle**, a canvas conversion of CSS colors to sRGB, alpha compositing over ancestor backgrounds, and WCAG relative luminance:

`(max(L1,L2) + 0.05) / (min(L1,L2) + 0.05)`.

Both viewports produce the same requested pairs. Evidence: `a11y-1280-{dark,light}.json`, `a11y-960-{dark,light}.json` (raw/composited channels and ratios included).

| Rendered pair | Dark | Light |
| --- | ---: | ---: |
| Install foreground / primary button | 4.68 | 5.86 |
| Installed badge text / badge ground | 7.08 | 7.20 |
| Source on ordinary row | 6.12 | 6.09 |
| Source on focused raised row | 5.22 | 5.37 |
| Popularity on focused raised row | 5.22 | 5.37 |
| Focused-row ring / raised ground | 6.01 | 5.04 |
| Search border / field ground | 3.90 | 3.78 |

These reproduce the Result's claims. Text exceeds 4.5:1; measured control/focus edges exceed 3:1.

Row/name/chip/search transition and animation durations are **0s**. A separate 960×640 `prefers-reduced-motion: reduce` run confirmed the media query and 0s durations, including the sheet (`reduced-motion-and-guards.json`).

## 7. axe, both themes and viewports — PASS

**axe-core 4.10.3**, full document with preview open and file content loaded, tags `wcag2a`, `wcag2aa`, `wcag21a`, `wcag21aa`, `wcag22aa`, `best-practice`:

- 1280×800 dark: **0 violations**, 0 incomplete.
- 1280×800 light: **0 violations**, 0 incomplete.
- 960×640 dark: **0 violations**, one incomplete rule.
- 960×640 light: **0 violations**, one incomplete rule.

**Best-practice violations: none.** The narrow runs' manual-review item is **aria-hidden-focus**, targeting two Base UI `span[data-base-ui-focus-guard][data-type="inside"]` sentinels, not a product button. A manual sheet Tab loop cycled inside the dialog without settling on either guard or BODY (`reduced-motion-and-guards.json`). Do not misreport these incomplete results as either axe violations or proof that no manual review was needed. Automated axe does not catch F1/F2.

## 8. Styling guard and legacy-selector retirement — PASS

Fresh full gate reports:

```text
styles:check ok — App.css 812 counted lines (ceiling), 38 migrated file(s),
prose layer scoped, no raw colours; self-test passed.
```

- `appCssCeiling` is **812**, exactly the live nonblank/noncomment count; raw physical line count is not the ratchet's measure.
- Both Explore component files have no legacy class token. The AST-based guard validates migrated files and the literal-class selector cross-check is in `style-scope.txt`.
- **38 distinct removed `explore-*` selectors** were checked against tracked `src/` files. The only remaining text overlaps are `data-explore-list` attributes/selectors, not CSS classes. There are no uses of removed Explore classes.
- `explore-showHidden` / `showHiddenPreference` has **no reader left under src/**; its import/export and obsolete preference-key test were removed.

## 9. Vocabulary and frontend suite — PASS

Screen text and `aria-label` extraction on the Explore surface/preview found no whole-word **Tool / Tools / Sync / Refresh / Assign** in all four axe/contrast runs. Scope says **Deploys to** and Add says **Edit harnesses**. Source-supplied documents are not a promise about all possible upstream prose.

Fresh **`npm run test`: 52 files passed, 1,085 tests passed**. This includes `src/i18n/resources.test.ts:91–100`, the retired-vocabulary assertion and its positive sensitivity test. The Result's older **50 / 1064** figure is not the current checkout's test count.

## 10. Scope hygiene and full gate — PASS

`git diff 9fa54f3..a2a09bd` changes 16 files. All are the ticket's implementation/tests/docs/styles/i18n, the required shared-lens generalization, preference retirement and its test, App's Explore wiring, or the acquisition-time comment in `useSurfaceState.ts`. No unrelated source subtree was changed.

App changes are Explore-only: move preview rendering into ExplorePage, retain the list beneath the inspector, pass placement/preview/scope/close callbacks, route installed candidates through surface focus, and remove the obsolete standalone-preview/header condition. `handleExploreInstall` already used the Add intent; the new preview call now carries the name.

Fresh **`npm run version:check && npm run check` passed** (`full-gate.log`):

- Version **1.3.0** synchronized.
- ESLint and styling guard passed.
- Vitest **52 files / 1085 tests** passed.
- `tsc -b && vite build` passed.
- Rust formatting and clippy with warnings denied passed.
- Rust tests **837 passed, 0 failed**; main/doc-test targets passed.

`git diff --check 9fa54f3..a2a09bd` passed. Working tree was clean before writing this report; no generated binding drift.

## Findings and exact fixes

### F1 — P0: Closing a preview after Add leaves persistent focus on BODY

**Locations:** `src/components/skills/ExplorePage.tsx:179–188`; `src/components/skills/ExplorePreviewView.tsx:155–175`, especially `finalFocus={() => rowControl() ?? true}` at line 170.

**Reproduce:** fresh rich Explore → search `ts` → changelog-writer preview → Install → Add selected → wait for **Open in My Skills** → Escape. The original row has left the default Not-in-library filter. Closing the preview unmounts the focused button. The side effect calls only `rowControl(left)?.focus()`, which now resolves null; the sheet's fallback likewise has no surviving row. Focus stays on BODY, not merely for the next render. Reproduced side and sheet. Evidence: `f1-keyboard-1280.json`, `installed-preview-close.json`, `installed-preview-close-sheet.json`.

**Exact fix:** give both preview placements one surviving focus-return resolver: original row if present, otherwise next surviving row from its saved displayed order, previous if last, finally the Explore search/heading. Use it for side close and the Dialog's `finalFocus`; retain the dialog shell and toggle `open` as the primitive's documented contract requires rather than relying solely on conditional unmount cleanup. Do not move focus if the operator already moved it elsewhere. Add a pure fallback-order test at an appropriate extracted seam and rerun the fixture **successful Add → Escape** path in both placements (not only Add → cancel).

### F2 — P1: Clearing the query makes an installed online preview offer Install again

**Locations:** `src/components/skills/ExplorePage.tsx:109–120,136–139,454`; installed action branch in `ExplorePreviewView.tsx:96–112`.

**Reproduce:** failures Explore, wide layout → search `ts` → online-only **jsdoc-to-ts** → Install → Add selected. Add reports the skill added, with fixture deployment failures. Close Add: preview correctly says **Open in My Skills** and **Installed**. Clear the Explore search while leaving the preview open: it immediately becomes **Install / Deploys to …** again. Evidence: `online-installed-query-clear.json`.

**Cause:** `previewInstalledId` is found in `candidates`, not independently in `managedSkills`. `candidates` drops online rows when the query is short/empty or another online search is loading. A live installed preview therefore becomes “not installed” merely because its discovery result is no longer in the current list.

**Exact fix:** resolve the preview's installed ID directly against the full managed catalog using the same normalized name/repository identity as row matching. Extract/reuse that identity lookup in the pure Explore presentation module so row and preview rules cannot diverge; never use search-result visibility/loading as the installedness authority. Test an online-only installed preview across query clear, query replacement/loading, and chip changes; verify the action remains **Open in My Skills** throughout and opens the correct managed inspector.

## Deliberately not checked

- Native Tauri/WKWebView, real GitHub/skills.sh service responses, credentials/keychain, the operator's library or native filesystem writes. Never ran `tauri:dev`.
- Actual VoiceOver announcements or the release-build minimum-window smoke; those remain D31's operator half. Accessible label association, DOM focus, axe and emulated viewport/motion were checked here.
- Live-provider ambiguous/absent name resolution; covered the existing Add hook tests and production branch instead of modifying fixtures.
- Simulated Hide/Unhide backend failure: the stock fixtures' hide/unhide handlers do not fail. Reviewed their catch path; successful next/previous/search focus was exercised.
- A complete project-matrix interactive audit or every My Skills sort option; scoped regression tests plus the requested My Skills search/grouping fixture proof were run.
- No claim that every upstream document avoids retired words, or that real semantic search results must contain the literal query.
- No fixes applied. F1/F2 must be corrected and independently rechecked before D31 passes.

## Re-check — 2026-11-02 (fix 37c8999)

**PASS — F1 and F2 closed; no new fix-blocking findings.** Independently reviewed `git diff 25a783d..37c8999` at
`37c8999c3853fcec16c4584fe1a5f42918493ba4` in `/Users/alexstark/.t3/worktrees/skills-hub/t3-a46922e4` (first command
verified the worktree). Read the fix child's final Comment, both Explore components, presentation helpers/tests,
AGENTS.md's mounted-dialog contract and `actionLifecycle.focusAfter`. Evidence below is relative to the gitignored
`.scratch/round19/evidence/22-recheck/` in this worktree; ego-browser, independent fixture server on **5182**.

1. **F1 — PASS.** Fresh rich → `ts` → changelog-writer preview → Install → Add selected → Open in My Skills →
   Escape, at **1280×800 aside** and **960×640 sheet**: active element is **ts-migration-planner** at
   **+50 / +250 / +1000 ms**, never BODY/MAIN. Each placement also passes: last shown **ts-migration-planner**
   installed and removed → previous **changelog-writer**; only shown **changelog-writer** installed and removed →
   **Search Explore**; cancel Add then Escape → **changelog-writer**; Close preview → **changelog-writer**.
   All three timed samples pass for every case. After each sheet closes: **0 inert elements**, actual
   `openLayerCount()` **0** (not inferred from absence of dialogs). Evidence: `focus-flows.json`, reproducible driver
   `focus-flows.mjs`. The saved rendered order feeds one DOM resolver; side close guards operator-owned focus,
   sheet `finalFocus` uses that resolver, and its Dialog shell remains mounted with `open={preview !== null}`.
2. **F2 — PASS.** Fresh failures → `ts` → jsdoc-to-ts → Install → Add selected (1 added · 1 deployment failed) →
   close Add → clear → `x` → `ts` → Installed → Hidden → empty. Preview retains **Open in My Skills + Installed**
   at every step. Mutation-observer evidence explicitly captures `loading: true` with both still true, then settled
   results. The Installed-chip row also says **Installed**. Open in My Skills lands on **`#/skills/skill-1g0t`**, active
   **H2 jsdoc-to-ts**. Evidence: `f2.json`, `f2.mjs`. Installedness reads the full managed catalog, not visible results.
3. **Identity — PASS.** Both rows (`exploreCandidates`) and preview call `exploreInstalledId` over
   `exploreInstalledIndex`, keyed by the same `exploreHideKey`. Executed proof: managed **DOCX** at
   `https://github.com/Anthropics/skills.git` and candidate **docx** at
   `https://github.com/anthropics/skills/tree/main/docx` both resolve **case-proof**; literal/case-sensitive URL matching
   would disagree. Evidence: `regressions.json`. Limitation, unchanged by this fix: a repository URL ending in `/`
   is **not** normalized to the slashless form (lookup returns null); rows and preview still agree on that rule.
4. **Regressions — PASS.** Both sizes: Hide changelog-writer → **ts-migration-planner**; Unhide the sole Hidden row
   → **Search Explore**, no inert elements/layers (`regressions.json`). `matrixFocusAfterLeave`
   (`projectPresentation.ts:225`) and skills `focusAfter` (`actionLifecycle.ts:205`) are unchanged; the diff of those
   modules and `useMatrixFocusReconciliation.ts` is empty (`focus-regression.diff`, **0 bytes**).
5. **Tests — PASS.** Fresh `npm run test`: **52 files / 1098 tests passed** (`tests.log`);
   `npm run test -- src/lib/explorePresentation.test.ts`: **1 file / 17 tests passed** (`explore-tests.log`).
   All pure `exploreFocusReturn` branches are pinned: original, next/skipped follower, previous, first surviving new
   row, absent-from-before, empty/search. The DOM resolver's **search-absent → Explore-heading** fallback has no unit
   test; neither does the side effect's **operator moved focus → leave it alone** guard. These are coverage limits,
   not reproduced defects. The original worktree lacked dependencies; installed them without saving manifests before
   rerunning successfully. No production code or lockfile changed.
6. **Scope — PASS.** Exactly six changed paths: `ExplorePage.tsx`, `ExplorePreviewView.tsx`,
   `explorePresentation.ts`, its test, **DESIGN.md § Explore**, and ticket 22. `scope.txt` records the list;
   `git diff --check 25a783d..37c8999` passes. No My Skills/matrix or unrelated changes.

Only this audit section was appended; no code changes or commits. Fixture/browser closed, **no listener on 5182**
(`server-stop.txt`); never ran `tauri:dev`. The earlier audit's native/VoiceOver/live-provider exclusions remain.
An initial automation attempt reused the same URL without reloading and retained fixture installs; it was discarded.
The final focus evidence above explicitly reloads the fixture for each case.
