# 22 — Explore as tracker rows: Not-in-library filter, Preview primary, scope summary, installed → inspector, contrast + search label

Status: done — a2a09bd + 37c8999 (audit fixes)
Blocked by: 19
Spec: `.scratch/round19/spec.md` — D25 (grill Q19); round-18 D10. Sources: `critique-astra` F11, Explore reads;
`critique-opus` P2-1, Q6; `audit.md` Explore row. Phase 19b.

## Work

- Rows through the list lens (search with an associated label; chips **Not in library** (default) · **Installed** ·
  **Hidden**); row = name · source · description · popularity as a secondary numeric · installed mark; row focus opens
  the **preview** in the inspector (the `useSkillDocument` module from ticket 12, fed the Explore document — no forged
  `ManagedSkill`); **Install** as one action with an inline Tool-scope summary that stays visible; an installed
  candidate's row and preview route to the **managed inspector** (surface state `focus({kind:'skill', id})`), never a
  second Install; the preview's acquisition time is labelled as such, never as freshness.
- Search mode line rewritten (featured vs online; minimum query length as inline hint, not a paragraph).
- Hide moves to the overflow; Unhide from the Hidden chip.
- Dark-mode Install/Installed pairs on tokens (the audit's 2.14:1 / 1.82:1 are the regression list).
- Migration: `ExplorePage` joins `migrated`; `explore-*` rules leave `App.css`.

## Gate (D31)

Astra audit: keyboard-only search → focus a row → preview → Install with scope shown; an installed row opens the
managed inspector; contrast both themes; 960×640; `npm run check`.

## Result

**Worktree** `/Users/alexstark/.t3/worktrees/skills-hub/t3-ce69d961` · **branch** `t3/ce69d961` (base `e856b7b`). Nothing
committed — every change is left uncommitted for the orchestrator.

**Gate**: `npm run version:check && npm run check` green (2026-11-02) — lint clean, `styles:check ok — App.css 812
counted lines (ceiling), 37 migrated file(s)`, vitest **50 files / 1064 tests** passed, `tsc -b && vite build` ok,
`cargo fmt --check` + clippy `-D warnings` clean, cargo **837 passed, 0 failed**. `src/bindings/index.ts` unchanged.

**Files changed**
- `src/components/skills/ExplorePage.tsx` — rewritten: rows through the list lens, chips, mode line, two source cards,
  row anatomy, ↑/↓ row walk, Hide in the overflow, Unhide on Hidden rows, preview in the inspector slot, Escape and
  focus return. References no `App.css` class (joined `migrated`).
- `src/components/skills/ExplorePreviewView.tsx` — the preview as the inspector's slot (aside ≥ 1200 px, Dialog
  `sheet` below); heading `tabIndex=-1` is the arrival focus (09-30 note); Install + inline "Deploys to …" scope;
  installed → Open in My Skills; "Fetched for preview {time}" (acquisition, not freshness).
- `src/lib/explorePresentation.ts` (+ `.test.ts`, new) — candidates (featured + online, de-duplicated, installed id,
  hidden), `EXPLORE_FILTERS` (a partition), `exploreQueryFold`, `exploreSearchMode`, `EXPLORE_MIN_ONLINE_QUERY`,
  `exploreHideKey` (the persisted key shape, unchanged), `formatPopularity`.
- `src/hooks/useListLens.ts` (+ tests) — options for a non-skill surface: `fold` (surface query fold, ranked order,
  required when the row is not skill-shaped), `controlledQuery`, `defaultFilter`, `groupByRepoPreference` optional
  (Flat). My Skills and the matrix are unchanged.
- `src/hooks/useExploreState.ts` — the "Show hidden" preference is gone (the Hidden chip replaces it); min query length
  from the fold module. `src/lib/preferences.ts` / `persistedPreference.test.ts` — `showHiddenPreference`
  (`explore-showHidden`) removed with its only reader.
- `src/App.tsx` (Explore wiring only) — `ExplorePage` always renders for the area with `preview`, `placement` (App's
  deferred inspector placement), `onClosePreview`, `onOpenInstalled` (`focus({ kind: "skill", id })`) and Add's scope
  (`addFlow.add.summary.tools` / `.deploy`); the preview no longer replaces the list and Install no longer unfocuses it
  (Escape chain: Add → Install → row); the preview's Install now passes the skill name (it passed none).
- `src/hooks/useSurfaceState.ts` — `ExplorePreview.fetchedAt` doc: acquisition time.
- `src/App.css` — every `explore-*` rule (296 counted lines) removed; `scripts/styles-baseline.json`:
  **`appCssCeiling` 1108 → 812** (−296), `ExplorePage.tsx` appended to `migrated`.
- `src/i18n/resources.ts` — new `explore.*` family appended at the end of `en`; the keys this change orphaned removed
  (`exploreFilterPlaceholder`, `exploreEmpty`, `exploreLoading`, `exploreSourceHint`, `exploreFeaturedTitle`,
  `exploreOnlineTitle`, `exploreView`, `exploreHide`, `exploreUnhide`, `exploreShowHidden`, `searchLoading`,
  `searchEmpty`, `manualAdd`, `manualAddHint`). Reused: `install`, `navExplore`, `addSkillButton`,
  `library.group.count`, `add.summary.{tools,more,noTools,autoSyncOff}`, `common.listSeparator`, `inspector.files.tree`.
- `DESIGN.md` — new § Explore (ticket 22); the regression-list rows re-measured; the legacy-input note.

**New keys** (`explore.`): `searchLabel`, `searchPlaceholder`, `filterGroup`, `filters.{notInLibrary,installed,hidden}`,
`mode.{featured,short,online,minQueryHint}`, `list`, `sections.{featured,online}`, `loading`, `searching`,
`noFeatured`, `noOnline`, `empty.{notInLibrary,installed,hidden}`, `noMatches`, `clearSearch`,
`row.{installed,openInstalled,openPreview,more,preview,openInLibrary,hide,unhide,unhideLabel}`,
`row.popularity.{downloads,stars,installs}`, `preview.{close,fetched,installedBody,openInLibrary}`. No retired word.

**Contrast** (rendered, canvas-composited, fixture `rich`; dark / light): **Install** (`solid accent`) text on
`primary` **4.68 / 5.86** (was 2.14 dark); **Installed** mark (`soft neutral` Badge) **7.08 / 7.20** (was 1.82 dark /
3.58 light); row name 15.50 / 17.32; source + popularity (`faint`) 6.12 / 6.09 on `background`, 5.22 / 5.37 on the
focused row's `raised`; description + mode line 8.31 / 8.16; focused-row `ring` edge on `raised` 6.01 / 5.04; search
edge 3.90 / 3.78; scope label / value on the aside 7.78 / 7.60 and 14.51 / 16.14 (sheet on `popover`: light 8.36 / 17.76).
The lowest, 4.68 dark, is the documented `primary-foreground` on `primary` floor.

**Fixture proof** (ego-browser, own server on 5177, stopped; evidence gitignored under
`.scratch/round19/evidence/22/`): uninterrupted keyboard transcripts at 1280 × 800 and 960 × 640 in both themes
(`*-transcript.txt`): Tab from the document start → sidebar → Add skill → search; type "ts" (mode line "Featured skills
and skills.sh results for “ts”"); ↓ ↓ to a row; Enter → preview, focus on its `h2` (aside at 1280, sheet at 960);
Tab → Close → Install; Enter → Add on `acme/changelog-skill`, "1 of 1 checked", **Deploys to Cursor, Claude Code, Codex
+4** visible; Escape → Install; Escape → the row. `*-transcript-hide-installed.txt` (rich 1280 dark, failures 960
light): overflow → Hide from Explore → focus on the next row; Hidden chip → Unhide → focus to the search (last row);
Installed chip → row (described "In your library — opens it in My Skills") → Enter → `#/skills/<id>`, focus on the
inspector heading. `1280-dark-transcript-install-to-inspector.txt`: Install → Add selected → the preview turns into
**Open in My Skills** (focus kept on it) → the managed inspector. axe (wcag2a/aa, 21, 22aa, best-practice): **0
violations** on rows, Installed chip and preview open, all four size × theme runs (`*-a11y.json`). Hover is instant:
row, name, chip, search and overflow compute `transition-duration: 0s`; nothing on the surface moves, so reduced motion
changes nothing.

**Left deliberately**
- Popularity is shown, never a sort: rows keep their sources' rank (featured by downloads as curated, skills.sh by
  relevance); Explore has no sort control.
- Chip counts are the whole list's, not the query's (the lens's rule, as in My Skills).
- No per-row pending mark while a preview clones — the header progress says it (with Cancel).
- `D39`'s `--name-col` is not used (ticket 31 owns it); the Explore name caps at 22 rem.
- CHANGELOG entry left for the 1.3.x release ticket (26); `[Unreleased]` is empty in this base.
- For the audit: online results are not re-filtered by the local query (skills.sh matched them); a failed Hide/Unhide
  leaves focus where it was (the world's error toast says why).

## Comments

- 2026-09-30 — from ticket 14 (recorded by the orchestrator; the 14 child noted it only in its Result): the Explore preview leaves focus on the main landmark rather than its own heading when it opens (`ExplorePreviewView.tsx`, which now uses `useSkillDocument`). Move the arrival focus to the preview's heading (`tabindex="-1"`, D21) and return to the row on close, the way the inspector does.

- 2026-09-30 — from ticket 15: `.explore-skill-item`, `.explore-search-input`, `.explore-card`, `.explore-btn-install`,
  `.explore-btn-view` and `.explore-btn-hide` lost their `transition` declarations (the built CSS carries no
  `transition: all`; hover is instant, as in every primitive). Keep hover instant when the rows migrate; DESIGN.md
  § Motion lists what may move.

- 2026-09-30 — from ticket 16: Explore's install is the Add intent now — `addFlow.open({ source: { kind: "git", url },
  preselect: name })` (App's `handleExploreInstall`; the render-cycle trigger effect is gone). It opens Add on the
  repository, listed at once, the named skill checked (an ambiguous name checks every match; a name the listing lacks
  is said and nothing is checked); the operator presses "Add selected" — it never installs by itself, and it no
  longer retargets every installed Tool (`targetAllInstalled` is deleted): the Tool scope in Add's sticky summary is
  D25's "inline Tool-scope summary". The preview's Install takes the same intent.
- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-02 — child (ticket 22): implemented in `t3/ce69d961`, uncommitted; see Result. The 09-30 focus note is
  fixed (heading takes focus, Escape/Close return to the row); the 10-01 instant-hover rule holds (`0s` everywhere on
  the surface); the 16 wiring is kept — the preview's Install now carries the skill name and leaves the preview open
  under Add, so Escape walks back Add → Install → row.

- 2026-11-02 — **Astra D31 audit FAIL** (`round19/review/ticket-22-audit.md`; checks 2–10 pass: contrast, axe 0, lens regression, vocabulary, guard). **F1 (P0)** after Install → Add selected → Open in My Skills → Escape, the row has left "Not in library", the focused button unmounts and focus lands on `body` (side and sheet). **F2 (P1)** the preview's installedness is read from the current `candidates` list, so clearing the search flips an installed online preview back to Install. Fix child dispatched (one focus-return resolver per D21 for both placements; installedness from the full catalog through the shared identity helper in `explorePresentation.ts`); bounded re-check after.

- 2026-11-02 — child (ticket 22 audit fix): fix for audit F1–F2, in worktree
  `/Users/alexstark/.t3/worktrees/skills-hub/t3-6f2df765`, branch `t3/6f2df765` (base `13a677b`), uncommitted.
  **F1 (focus on body after Add-from-preview)**: one pure resolver, `exploreFocusReturn(before, after, left)` in
  `src/lib/explorePresentation.ts` (the Explore twin of `matrixFocusAfterLeave`): the row itself while shown, else the
  next row still shown in the opened-from order, else the previous, else `{ kind: "search" }`. `ExplorePage` saves the
  rendered row order when a row opens its preview (or when a preview arrives from elsewhere) and resolves it in one DOM
  reader (`resolveExploreFocus`: row → search → `data-surface-heading="explore"`), used by the side close path (only
  when focus is lost) **and** the sheet Dialog's `finalFocus`. The sheet's `Dialog` shell now stays mounted in sheet
  placement and toggles `open={preview !== null}` (the primitive's contract); the preview body is a keyed inner
  `PreviewPanel`. Hide/Unhide's neighbour focus goes through the same resolver (same rule, no second copy).
  **F2 (installed preview reverting to Install)**: `exploreInstalledIndex(managed)` + `exploreInstalledId(index, name,
  source)` — the one identity lookup; `exploreCandidates` marks rows through it and the preview resolves
  `previewInstalledId` against the full `managedSkills` through it, never against `candidates`. The online-visibility
  rule moved verbatim into `exploreVisibleOnline(query, loading, results)` so the test exercises it.
  **Tests** (`src/lib/explorePresentation.test.ts`, +13): `exploreInstalledId` — online-only installed candidate stays
  installed across "ts" → "" → "x" and while loading; row and preview agree for featured and online; first Managed
  skill of an identity wins. `exploreVisibleOnline`. `exploreFocusReturn` — row itself, next surviving (the audit's
  changelog-writer → ts-migration-planner), skipping a gone follower, previous, first-shown fallback, search.
  **Fixture proof** (ego-browser, own server on 5181, stopped; gitignored `.scratch/round19/evidence/22-fix/`):
  `f1-side-1280.json` / `f1-sheet-960.json` — rich, keyboard: search "ts" → ↓↓ changelog-writer → Enter → Tab Tab
  Install → Enter → Add selected → preview shows Open in My Skills → Escape → **ts-migration-planner row**, settled at
  +50/+250/+1000 ms, never body, aside and sheet. `regression-{side-1280,sheet-960}.json` — cancel-Add → Escape and
  Close → changelog-writer row; Hide → next row; Unhide last → search; no `inert` left after the sheet closes.
  `f2-online-installed-query-clear.json` — failures 1280: jsdoc-to-ts installed (1 added · 1 deployment failed), then
  query "" → "x" → "ts" (sampled while "Searching skills.sh…") → Installed/Hidden chips → "": the preview keeps
  **Open in My Skills** + Installed throughout; it opens `#/skills/skill-1fhm`, focus on its inspector H2.
  **Gate**: `npm run version:check && npm run check` green — Version OK 1.3.0, lint clean, `styles:check ok — App.css
  812 (ceiling), 38 migrated`, vitest **52 files / 1098 tests**, build ok, fmt + clippy clean, cargo **837 passed**;
  bindings unchanged. `DESIGN.md` § Explore's preview focus sentence synced.

- 2026-11-02 — **done.** Astra re-check (`round19/review/ticket-22-audit.md` § Re-check): PASS — F1 (focus return, both placements, every edge case) and F2 (installedness through query/loading/chips) closed; 1098 tests. Its one noted limitation — a repository URL with a trailing slash did not match — closed by the orchestrator in the closing commit: `normalizeGithubRepo` now strips scheme/www, a trailing slash and `.git` in the right order (the `name|owner/repo` key shape, a persisted contract, is unchanged; a test pins seven spellings to one key).
