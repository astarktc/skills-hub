# 31 — My Skills row and group anatomy: card-per-group ground, `--name-col`, Not-deployed mark dropped, projects on the wire, collapsible groups, harness initials; riders #80 #81 #82

Status: done — a2eed57
Blocked by: 30 (done 9ca0bc8), 35 (done 03065db — the dark ground — rows on `row` = `background` #000000, hover `accent`, focused/selected `raised`)
Blocks: 25
Source: grill 2026-11-02 (operator, verbatim themes: name column "pretty much 50 % … steals from the skill
descriptions"; "'Not Deployed' repeated in every line … cluttered and repetitive … outright false at the project
level"; "I don't even see the previously scoped project deployment count in the rows"; "should repo groups be
collapsible, and is indentation different enough"; "dark black for skill rows under each repo grouping"). Spec: D38,
D39, D40, D41, D42, D44; D43's pick (ticket 30). BACKLOG #80, #81, #82 absorbed here 2026-11-02. Facts verified:
`SkillRow.tsx` `NAME_GROUP` floors the name at 60 % of the cell; `ManagedSkillDto` carries no project data — ticket 11's
"folder + N projects" shipped as "globe when a global target is live" (`11-my-skills-surface.md` ~:27 vs ~:91).

## Work

### Enabler — projects on the wire (D41)

- `ManagedSkillDto.projects: SkillProjectDto[]` — per project the skill is deployed to: `project_id`, `project_name`,
  `deployments: { tool, status }[]` (status = the assignment row's `SyncStatus`). Backend derives it in the listing
  (one query, no N+1), `specta::Type`, `cargo test` regenerates the binding, the shim re-exports. Counts are derived by
  the fold, never carried. The fixture's `ManagedSkill` seeds gain `projects` consistent with its project scenarios.
- `skillHealth`: failed project deployments (`error` / `missing`) join `targets_failed` / `targets_missing` with the
  project named in the row's tooltip; "N failed" on the row counts them. `deployAction` / `isLiveTarget` unchanged
  (global).

### Row (D38, D39, D40, D41)

- **Ground**: rows on `row` (= `background`); hover **`accent`** (ticket 35 split it from `raised` in dark; 30's brief said `card`); the **focused** row a raised rounded block (`raised`, `rounded-md`,
  inset so the card's border shows); **selected** keeps the 3 px iris edge (ticket 13's rule).
- **Columns**: `32 · var(--name-col) · 1fr · (marks×16+40) · scope · 40 · 32`. The list publishes `--name-col` =
  `clamp(12rem, <longest visible name>ch, 22rem)` the way it publishes `--strip-marks`; `NAME_GROUP`'s 60 % floor is
  deleted; the description truncates with a `title`; below the 48 rem container the description still drops.
- **Marks**: the `not_deployed` row is **never rendered as a row mark** (`SILENT_ROWS` gains it); it stays in the
  inspector ladder, the group state summary and Deploy all's count.
- **Scope cell**: globe when a global deployment is live, **and/or** folder + N when N projects hold ≥ 1 live
  deployment; one `aria-label` / `title` naming the projects ("In 3 projects: monorepo, api, docs"). N/M stays global.

### Groups (D38, D42)

- Each source group is a **card**: `border` + `rounded-lg`, header on `card` (sticky inside the scroll area as today),
  rows inside. Header: chevron (collapse) · identity treatment per ticket 30's pick · source name · count · state
  summary · Deploy all. No extra row indent.
- **Collapse**: chevron button (named "Collapse {source}" / "Expand"), state persisted per source key through the list
  lens (`useListLens` + `preferences.ts`, one new key), ←/→ on a focused header, Collapse all / Expand all in the view
  menu. An active query or filter expands matching groups **for its duration** (collapsed state restored when cleared).
  A collapsed group's selection stays selected; the bulk rail's counts still include it.
- **Grouping on the unfiltered catalog** (#81 (4)): the ≥ 2 rule runs on the whole catalog; a filter that leaves one
  skill of a multi-skill repo keeps it under its repo header. `skillPresentation.ts` fold + test.
- **Harness initials over the strip column**: a 2-letter initials row aligned with the marks, in the group header's
  strip column (per the prototype's placement), each with the harness's full name as `title`/`aria-label` — answers
  #72's "which mark is Cursor".

### Riders (D44)

- **#81**: `index.html` `<title>` → "Skills Hub"; `autoCorrect="off" spellCheck={false} autoCapitalize="off"` on every
  `type="search"` input (`FilterBar.tsx` and the others — grep); Enter / ↓ in the My Skills search moves focus to the
  first visible row (field-level rule; the keymap still ignores keys in an editable).
- **#80**: the toast stack no longer covers the header controls — reserve space (verify 960 and 1280) without
  `pointer-events: none` and without dropping hover-pause globally; DESIGN.md § Toast updated.
- **#82**: delete the dead keys (`invocationEdit.{overrideTooltip,conflictTooltip}`, `detail.{readError,fileCount}`,
  `unsyncSkill`, `unsyncAllComplete`) and add the reverse test (an `en` leaf no source file reads fails the test —
  grep-based, allow-list for keys built dynamically).

## Proof

- `cargo test --all` (binding regenerated, committed); `npm run test` (folds: unfiltered grouping, projects in
  `skillHealth`, `--name-col` clamp, collapse persistence; `resources.test.ts` both directions).
- Fixture `?scenario=rich`, both themes, **960×640, 1280×800 and 1800×1000**: descriptions start on one line; no row
  mark reads "Not deployed"; a project-only skill shows folder + N and no globe; collapse survives reload; a search for
  one skill of a multi-skill repo keeps its header; VoiceOver-equivalent names in the AX tree for chevrons, initials,
  scope. Screenshots under `evidence/31/`.
- D31 child audit (Astra) before `done`.
- `npm run version:check && npm run check` green.

## Result

Implemented 2026-11-04 by a delegated child (uncommitted; the orchestrator reviews and commits). No new tokens.

**Projects on the wire (D41).** `ManagedSkillDto.projects: SkillProjectDto[]`, with
`SkillProjectDto { project_id, project_name, deployments: SkillProjectDeploymentDto[] }` and
`SkillProjectDeploymentDto { tool: string, status: SyncStatus, synced_at: number | null }`. The DTOs are in
`commands/mod.rs`, mapped from `core::skill_catalog::{SkillProject, SkillProjectDeployment}`. Bindings are regenerated,
and the shim re-exports both types. **`synced_at` is beyond the brief's `{ tool, status }`.** Without it the scope fold
could not apply the one ownership rule (`isLiveTarget`): a never-synced `error` row is not live. The project name is
`project_ops::project_name_from_path` (the basename, as on the Projects surface). Projects are ordered by name, then
id, and each project's rows by Tool key.

**The query.** `SkillStore::list_skill_project_deployments(only)` runs one `SELECT … FROM project_skill_assignments a
JOIN projects p ON p.id = a.project_id ORDER BY a.skill_id, p.path, a.tool` for the whole catalog. `skill_catalog`
groups the rows per skill, so there is no N+1. `managed_skill_entry` uses the same method with `WHERE a.skill_id = ?1`.
A failing query fails the listing, as targets already do; a test pins this. In the fixture, `skillsDto()` derives the
projects from the assignment rows at read time.

**Health.** An `error` / `missing` project deployment joins `targets_failed` / `targets_missing`. The row's `count`
spans both scopes. `HealthRow.projects` names each project with its harnesses. `retryTargets` is offered only when the
row has global pairs: a project deployment repairs from the matrix. The header's failed count and the Issues chip count
these deployments too. `deployAction` / `isLiveTarget` are unchanged.

**Row.** `listColumns.ts` holds one template for the row and its header:
`32 · var(--name-col) · 1fr · calc(--strip-marks × 1.25rem + 2.5rem) · 3.5rem · 2.5rem · 2rem`. Below `@max-3xl` the
template is `32 · 1fr · strip · …` and the description is hidden. `NAME_GROUP` is deleted. The description truncates,
and its title is source · description. The strip has a 20 px pitch (`gap-2`).

- `rowMarks` adds `not_deployed` to the silent set. The AX check found 0 "Not deployed" badges at all 6 sizes.
- **Mark words.** Target kinds use the group summary's short form, `ROW_MARK_LABEL_KEY`: "2 failed", "1 missing",
  "1 out of date". The full ladder sentence is the tooltip, followed by `healthRowPlaces`, e.g. "1 deployment failed —
  In quartermaster (.agents/skills)".
- **Ground.** Rows are on the card's `bg-row`, with `hover:bg-accent`. A focused row is `raised`, `rounded-md`,
  `mx-1 px-2`, with a 1 px inset ring. A selected row is `raised` + the 3 px edge. There is no row hairline.
- **Scope cell** (`skillScope`). One `role="img"`, named and titled, e.g. "Deployed globally · In 3 projects:
  agent-skills, quartermaster, skills-hub".

**`--name-col` rule** (`nameColumnWidth`): `clamp(12rem, N ch, 22rem)`. N is the longest on-screen row's name + 2,
plus Σ(mark words + 5), plus 4 for the pencil. The pane sets `text-sm`, so `ch` resolves at 14 px. Collapsed rows do
not count. On the fixture, N = 34 ch = 278 px.

The short mark words came out of this measurement. With the ladder's sentences ("2 deployments failed"), N hit the
22rem cap, and the description at 1280 was 243 px — no better than the old 60 % floor. With "2 failed" it is 317 px.

The description stays on one line at every size; at 960 it is 141 px. Horizontal overflow is false at all 6 sizes.

**Groups.**

- **Card.** Each group is a `rounded-lg` bordered `overflow-clip` card on `row`. The header is sticky, 40 px, on `card`,
  in the same grid as the rows.
- **Header contents.** A chevron (`aria-expanded`, "Collapse/Expand {source}"). The identity chip: `Badge tone="identity"`
  for a GitHub owner (`RepoGroup.owner`, new); otherwise the neutral chip with layers / folder / branch, `aria-hidden`.
  Then the name, the count (hidden below 48 rem), the summary (`shrink-8`), the initials, and Deploy all.
- **Collapse.** Chevron, ←/→ on the header, and Collapse all / Expand all in the view menu (shown while Grouped).
  `useListLens` gains `collapsePreference`, `collapsed`, `setCollapsed`, `toggleCollapsed`, `collapseAll`, `expandAll`
  and `onScreen`. `onScreen` is `displayed` minus collapsed rows; App feeds it to the keymap and the door's focus rule.
- **During a query.** While a query or chip is active, every group is expanded. A toggle made then is transient, and
  the stored state comes back when the query clears.
- **Grouping.** `groupSkillsByRepo(…, { catalog })` decides singletons on the unfiltered catalog. The lens passes `items`.

**Preference key:** `skills-collapsedSources` (`collapsedSourcesPreference`, a JSON string array through the new
`stringListPreference`). It is pinned in `preferences.test.ts` and in `persistedPreference.test.ts`'s key list.

**Initials fold** (`harnessInitials`):

- **Rule.** The first letters of the label's first two words, else its first two letters.
- **Collision.** A pair already taken keeps its first letter and takes the next unused letter of the label (Codex CO →
  CodeBuddy CD → Continue CN).
- **Fixture.** The fixture's eight harnesses read CU · CC · CO · AM · KC · PI · GC · WI: all distinct, no tiebreak.
- **Departs from the brief: Gemini CLI → GC, not GM.** The brief's examples (CC, CU, GM) fit no single rule; GM comes
  from the prototype's hand-written map. The rule above is the brief's sentence ("the first two letters of the harness
  name's words") read literally. **Operator: confirm, or name a different rule.**
- **Labels.** They come from the Harness catalog (`stripTools`). Each initial is a `role="img"` named and titled with
  the full label, inside a group named "Harnesses".

**Riders.**

- **#81:**
  - `<title>Skills Hub`.
  - `autoCorrect="off" autoCapitalize="off" spellCheck={false}` on all 4 `type="search"` fields (FilterBar,
    AssignmentMatrix, ExplorePage, CandidatePicker).
  - Enter or ↓ in the My Skills search focuses the first row. The fixture shows it: Enter after "pptx" → `skill-031-pptx`.
- **#80:** the Toaster is `bottom-right` with `offset={{ bottom: "6rem" }}`. Pointer events and hover-pause are
  untouched (`pointer-events: auto` measured). Measured with the bulk rail up:
  - 960×640: toast y 491–544, rail 599–640.
  - 1280×800: toast y 651–704, rail 759–800.
  - At both sizes no header, filter-bar or rail control intersects the toast.
- **#82:** the reverse test reads every `en` leaf from the source. A key counts as read when it appears as a quoted
  literal, through a plural base, or under a template-literal prefix, which is auto-detected (`` `x.${ `` — 19
  families). `UNREAD_ALLOW_LIST` is empty, and a self-test guards the detector. The test found **55** dead keys, the
  ticket's 6 among them. All 55 were deleted, because the test cannot pass otherwise:
  `subtitle`, `newSkill`, `invocationEdit.overridden`, `invocationEdit.overrideTooltip`, `invocationEdit.conflictTooltip`, `installSuccess`, `filterSort`, `allSkills`, `groupByRepo`, `scanNow`, `skillsTitle`, `toolsLabel`, `activeTools`, `sourceLabel`, `pathLabel`, `updatedLabel`, `statusLabel`, `localImportTitle`, `gitImportTitle`, `deleteTitle`, `deleteBody`, `confirmRemove`, `maintenance`, `storagePath`, `restartNow`, `updatesMaintenance`, `autoUpdateSkills`, `autoUpdateDesc`, `storageCleanup`, `storageCleanupDesc`, `rescanTools`, `changeSource.confirm`, `searchTab`, `exploreTab`, `exploreSource`, `errors.skillNotFoundInRepo`, `delete.confirmPrefix`, `delete.confirmSuffix`, `delete.warningRemoveFromTools`, `delete.warningDeleteFromHub`, `delete.confirmButton`, `detail.files`, `detail.readError`, `detail.fileCount`, `unsyncAllComplete`, `unsyncPartial`, `unsyncSkill`, `projects.assignmentCount`, `projects.toolCount`, `projects.skillCount`, `disabledSkips.deployDisabledPairs`, `repointSource.was`, `repointSource.missingFolder`, `shell.shortcutBack`, `projectsSurface.cell.alsoGlobalTitle`.

**Measured** (`faint` / `muted-foreground` on the new grounds, WCAG, theme.css values):

| Pair | Dark | Light |
|---|---|---|
| Initials (`faint`) on `card` | **5.86** | **5.53** — confirmed |
| Scope (`faint`) on `row` | 6.00 | 6.24 |
| Scope on `accent` (hover) | 5.52 | 5.04 |
| Scope on `raised` (focused / selected) | 4.77 | 5.04 |
| Card `border` on `row` / `card` | 1.17 / 1.14 | 1.42 / 1.25 |

DESIGN.md's strip table still prints light `faint` on `card` as 5.67. That is pre-ticket-30; the measured value is 5.53.
I left that table alone.

**DESIGN.md:**

- § Layout gains a "superseded by ticket 31" bullet with the re-measured widths.
- § My Skills is rewritten: columns / `--name-col`, row, scope cell, group card, header, initials, collapse, grouping,
  and the filter bar's search keys and Groups menu.
- § Deployments strip: the 20 px pitch.
- § Toaster: placement.

**Screenshots** (gitignored, converted to sRGB, fixture `?scenario=rich` on port 5176), in `.scratch/round19/evidence/31/`:
`myskills-{dark,light}-{960,1280,1800}.png`, `collapsed-dark-1280.png` (anthropics + obra collapsed, after a reload),
`search-one-of-repo-dark-1280.png` ("pptx": the anthropics/skills header kept and expanded during the query),
`toast-dark-960.png`, `toast-dark-1280.png`, and `inspector-dark-1280.png` (compact header; the ladder names the
project).

The fixture also proves:

- After a reload the collapse state is `["anthropics/skills","obra/superpowers"]`; clearing the search restores it.
- The AX snapshot carries "Collapse obra/superpowers", group "Harnesses" → image "Claude Code" …, and image "Deployed
  globally · In 1 project: quartermaster".
- react-perf-audit shows a globe + folder 3, and its tooltip names all three projects.
- prisma-review shows folder 1 and no globe.

**Fixture seeds** (rich): react-perf-audit is added to agent-skills (3 projects + global). prisma-review is added to
quartermaster: project-only, since it has no global deployment. writing-plans gets an `error` `agents_skills`
deployment in quartermaster. A fixture test pins all three, and a toggle shows at once.

**Gate:** `npm run version:check && npm run check` exits 0:

- vitest 1144 passed (53 files).
- `cargo test` 839 passed. `cargo test --all` was run as well, and bindings are clean: `src/bindings/index.ts` is
  modified, with nothing untracked.
- clippy `-D warnings`, fmt, lint and build pass.
- styles:check: App.css at 812 counted lines, the ceiling, unchanged (no My Skills rule lived there). 39 migrated
  files: `listColumns.ts` was appended.

**Files:**

- Rust: `src-tauri/src/core/{skill_store,skill_catalog}.rs`, `core/tests/skill_catalog.rs`, `commands/mod.rs`.
- Bindings and shim: `src/bindings/index.ts`, `src/components/skills/types.ts`.
- Surfaces: `src/components/skills/{SkillsList,SkillRow,SyncedStrip,FilterBar}.tsx`, the new `listColumns.ts`,
  `inspector/HealthSection.tsx`, `src/App.tsx`, and the search attributes in `ExplorePage.tsx`, `add/CandidatePicker.tsx`
  and `projects/AssignmentMatrix.tsx`.
- Folds and lens: `src/lib/{skillPresentation,persistedPreference,preferences}.ts` and their tests,
  `src/hooks/useListLens{,.test}.ts`.
- i18n: `src/i18n/resources{,.test}.ts`.
- Fixture: `src/fixtures/{model,backend,scenarios}.ts`, `backend.test.ts`.
- Test factories: `useSkillActions`, `useSkillLibrary`, `bulkCounts`, `commandPalette` (each gains `projects: []`).
- Other: `index.html`, `scripts/styles-baseline.json`, `DESIGN.md`.

**Not proven here:**

- The operator's eyes on the screenshots.
- The Astra audit.
- VoiceOver speech (the AX tree only).
- The `impeccable` / `web-design-guidelines` skills are not installed in this environment (`.pi/skills/` is absent),
  so the work follows DESIGN.md and the prototypes directly.
- **The neutral chip's ground is near-invisible in dark.** DESIGN documents it as "an icon on `muted`", and ticket 35
  made `muted` = `#0e0e0e`, which is 1.06 on `card`. In dark you see the icon alone. Light is fine. I kept the
  documented variant. The prototype drew `raised`; a `solid neutral` chip would match it. That is an operator call.
- At 960 the description is 141 px wide. That is the cost of a fixed name column at the minimum window.

## Comments

- 2026-11-02 — opened at the grill; absorbs BACKLOG #80, #81, #82 and the project-count gap from ticket 11.
- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-02 — operator (prototype review): rows on a **black / near-black** ground, darker than `background` — ticket
  30 mints the token (`--row` ≈ #09090b dark; light stays the proposed `background`); D38's hover `card` / focused
  `raised` / header `card` stand. Identity chip in the group header only (A). Harness initials over the strip at
  **20 px** spacing (16 px runs "GMAM" together) — `--strip-marks` step becomes 1.25rem, the strip column 200 px at 8 marks.
- 2026-11-04 — implemented by delegated child; awaiting code review + Astra audit
- 2026-11-04 — orchestrator, after review. Standards (`round19/review/ticket-31-standards.md`): S1 fixed — the on-screen
  order is a pure fold, `skillPresentation.onScreenOrder`, tested; S2 (the reverse-read i18n scan is text-based: a
  quoted key anywhere counts as a read, concatenation is unseen) noted, allow-list stays empty; S3 (the inline group
  header in `SkillsList` wants a component) carried to ticket 24, which reuses the header for the grid. Astra audit
  (`ticket-31-audit.md`, FAIL → fixed): **F1 (P1)** project mutations left the library's `projects` stale — the project
  world now takes `onDeploymentsChanged` (App passes `library.loadManagedSkills`) and calls it after every mutation
  settles (`settleView`, `refreshView`'s finally, `removeProject`), never on a plain read; hook test added; proven in
  the browser (remove the last project deployment → the scope cell empties). **F2 (P1)** five expanded persistent
  errors climbed to the header at 960×640 — `visibleToasts` 5 → 3 (sonner's default), DESIGN.md § Toaster. **F3 (P2)**
  = S2. The ownerless chip is `solid neutral` (`raised`) — `muted` measured 1.06 on the dark header. Gemini CLI → `GC`
  by the deterministic rule stands; 45 registry labels yield distinct initials (audit).
