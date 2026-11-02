# 31 — My Skills row and group anatomy: card-per-group ground, `--name-col`, Not-deployed mark dropped, projects on the wire, collapsible groups, harness initials; riders #80 #81 #82

Status: ready-for-agent
Blocked by: 30
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

- **Ground**: rows on `background`; hover `card`; the **focused** row a raised rounded block (`raised`, `rounded-md`,
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

## Comments

- 2026-11-02 — opened at the grill; absorbs BACKLOG #80, #81, #82 and the project-count gap from ticket 11.
