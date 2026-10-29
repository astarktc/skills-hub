# Design critique — incumbent Skills Hub, judged against the decided world (Opus)

⚠️ DEGRADED: single-context (delegated critic child). Ticket 06 pairs two blind critics, and that pairing is what
keeps the critique independent, so this file does not split impeccable's Assessment A and B into sub-agents. I
finished the design review first (live fixture app, both themes, `rich` · `failures` · `first-run`, 1440×900 and
960×640). The deterministic detector ran after it, and its results are folded in at § Deterministic scan.

- **Critic:** Opus 5.5 (ticket 06, second critic). It did not read the other critique.
- **Judged against:** `PRODUCT.md`, the direction contract
  (`.impeccable/surfaces/src-components-skills-skillslist-tsx.md`), and `spec.md` D1 · D2 · D5 · D6 · D8 · D10 plus
  its Rejected list. Motion is judged with Emil's Review Checklist. The a11y and interaction rows use ui-ux-pro-max's
  quick-reference priority table.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/` (local and
  gitignored, 44 PNGs). The viewport was 1440×900 CSS px, downscaled in the PNG to 1280×800. The contrast ratios
  come from a WCAG luminance computation run in the page on computed colours. None of them were eyeballed.
- **Fixture caveats:**
  - The preview tab shared `localStorage` with the other critic, so theme and view-mode preferences sometimes
    flipped under me.
  - Frequent HMR reloads reset fixture state. Every capture below was re-verified against the live DOM before it
    was saved.
  - I copied each capture from the exact path the tool returned. Another agent writes into the same artifacts
    folder, so I did not rely on "newest file".

---

## Design specificity verdict

The incumbent could belong to almost any 2025 SaaS dashboard.

- Rounded 150 px cards with an icon tile.
- Rows of green outlined status pills.
- One light-blue primary, a gradient wordmark, and a dashed amber "Discovered skills" banner.
- Fira Sans fetched from Google Fonts.
- Modal after modal.

Almost nothing is authored for *this* product's job, which is reading deployment state across N tools at a glance.
The one strongly product-specific element is the tool pill. It encodes the core fact of the domain (which Tool has
this skill), but it inverts the contract: every healthy target shouts in green, and the rare failure is just another
pill of a different colour, often hidden inside "+2 more".

The world is decided (Tracker List shell and Patch Bay borrows), so the incumbent is **evidence and anti-reference**.
What survives is its copy and several of its state models (see § What to keep). Its look does not survive.

## Overall impression

The backend's honesty is better than the UI that presents it.

- The data model already distinguishes synced / stale / missing / error, source-missing / central-missing, `TARGET_EXISTS`,
  and kept error rows.
- The UI either flattens those states into near-invisible tints or paints the healthy case loudest.
- Two destructive actions fire with no confirmation, and one of them removes every Sync target in the library.
- The skill detail view, where the contract puts "why and the fix", shows a file browser and nothing about
  deployment.

**Biggest opportunity:** make My Skills tell the truth about state. That means exceptions-only marks after the name,
a Synced strip, a header exception summary, and an inspector that explains and fixes. Every other surface then
borrows that grammar.

---

## Scores

### Whole app — Nielsen heuristics (Operate mode, all 10 apply)

| # | Heuristic | Score | Key issue |
|---|---|---|---|
| 1 | Visibility of system status | 1 | Central-missing skill still shows 7 green pills. Fetch-failure and unlocatable-upstream skills look healthy. Search with no match says "No managed skills yet." |
| 2 | Match system / real world | 2 | Raw registry keys (`claude_code`, `CLAUDE_CODE`, `AGENTS_SKILLS`), `os error 13`, and `->` reach the operator. Install / import / create and remove / delete / uninstall drift. |
| 3 | User control and freedom | 2 | No undo anywhere. Tool-pill click and the link icon unsync instantly. Stacked modals all close on one Esc. |
| 4 | Consistency and standards | 2 | Three view modes behave differently. Button order flips between modals. Checkbox styles are mixed. "Manual" opens "Add skill". |
| 5 | Error prevention | 1 | "Uninstall from tool directories" removes the whole library's targets with no confirmation. Post-import leftovers come back pre-selected. |
| 6 | Recognition over recall | 2 | Card actions are icon-only: pin = Change source, link = unsync/deploy toggle. The matrix globe glyph is unexplained, and the matrix header scrolls away. |
| 7 | Flexibility and efficiency | 1 | No selection or bulk actions, no `/`, j/k or ⌘K. Only about 4 skills fit a 900 px viewport. |
| 8 | Aesthetic and minimalist design | 2 | Healthy is loud: 7 pills × 60 rows, the repo pill repeated inside repo groups, a 9-control toolbar across 2–3 rows. |
| 9 | Error recovery | 2 | Unlocatable skills get inline Restore/Re-point, which is good. Failed targets get no Retry. Missing project folder gets no fix. Import says "remove it yourself". |
| 10 | Help and documentation | 2 | The Edit, Re-point and Settings copy is excellent. Everywhere else relies on `title` tooltips. |
| **Total** | | **17/40** | **Poor** (42 %). The core experience (state at a glance, safe deploy/undeploy) is broken. |

### Per surface (0–4 per heuristic; n/a renormalises)

| Heuristic | My Skills list+card | Skill detail | Add flow | Import / onboarding | Projects + matrix | Explore + detail | Settings | Tool config modals | Notifications | Confirmations |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 Status | 1 | 1 | 2 | 2 | 2 | 2 | 3 | 2 | 3 | 2 |
| 2 Real world | 2 | 3 | 2 | 1 | 1 | 2 | 3 | 2 | 2 | 2 |
| 3 Control | 1 | 2 | 2 | 2 | 2 | 2 | 3 | 3 | 3 | 2 |
| 4 Consistency | 2 | 3 | 2 | 2 | 2 | 2 | 3 | 2 | 3 | 2 |
| 5 Prevention | 0 | 3 | 2 | 1 | 2 | 2 | 3 | 3 | n/a | 2 |
| 6 Recognition | 1 | 2 | 3 | 2 | 1 | 3 | 3 | 3 | 3 | 2 |
| 7 Efficiency | 1 | 1 | 2 | 2 | 2 | 1 | 2 | 2 | 2 | n/a |
| 8 Minimalism | 1 | 3 | 2 | 2 | 2 | 2 | 3 | 3 | 3 | 3 |
| 9 Recovery | 2 | 1 | 2 | 1 | 1 | 2 | 3 | 2 | 1 | 2 |
| 10 Help | 2 | 2 | 2 | 2 | 2 | 2 | 3 | 2 | 2 | 2 |
| **Total** | **13/40** Poor | **21/40** Acceptable | **21/40** Acceptable | **17/40** Poor | **17/40** Poor | **20/40** Acceptable | **29/40** Good | **24/40** Acceptable | **22/36** Acceptable | **19/36** Acceptable |

The n/a scores are Prevention for Notifications (a read-only history) and Efficiency for Confirmations (single-decision dialogs).

### Cognitive load (whole app)

Four of the eight checks fail, so cognitive load is **high**.

- **Single focus:** fails. The banner, the toolbar and 7 pills per row compete.
- **Minimal choices:** fails. The toolbar has 9 controls, the Explore card has 3 buttons, and every Explore card has
  its own primary.
- **Visual hierarchy:** fails. Healthy and failed states carry the same weight.
- **Working memory:** fails. The matrix header scrolls away, so the operator has to remember which column is which.

### Emotional journey

- The **first run** lands on a bare "No managed skills yet." under a full toolbar of irrelevant controls
  ([17](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/17-firstrun-light.png)).
- **Import complete**, which D6 names as a delight moment, ends on three stacked error toasts plus a modal that stays
  open and re-offers the leftovers
  ([19](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/19-import-after-light.png)).
- **Install landing** is a 4 s toast, "Selected skills installed.", and nothing in the list shows where the skills
  went.

The peak-end rule is violated at exactly the three moments PRODUCT.md reserves for delight.

---

## Surface reads

**My Skills — list, card, grouped and flat, list / Auto Grid / Dense Grid (13/40)**
([01](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/01-myskills-rich-dark-list.png),
[02](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/02-myskills-rich-dark-grouped.png),
[03](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/03-myskills-rich-dark-grouped-autogrid.png),
[04](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/04-myskills-rich-dark-grouped-densegrid.png),
[05–09](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/05-myskills-failures-dark-list-top.png))

This is the app's primary surface, and it answers the wrong question. It says "here are 60 skills and every tool
each is in", when the operator needs "what is wrong and where".

- Each card is 150 px tall, so about 4 skills fit at 1440×900 and about 1.5 at 960×640.
- The grouped mode repeats the repo pill on every card inside its own repo group, and each singleton repo gets a full
  header (the operator's recorded friction).
- The grid modes break the pill row. "Claude Code" wraps inside a 20 px pill (scrollHeight 25 vs clientHeight 20),
  "Kimi Code CLI" renders as "Code", and "+2 more" is clipped to "+2 m".
- Exceptions are nearly invisible:
  - Failed targets are a red variant of the same pill, possibly hidden in "+N more".
  - A skill whose central copy is missing, so every link dangles, still shows 7 green pills.
  - Skills whose upstream fails to fetch (`alias-skill`, `flaky-upstream`, `deprecated-helper`) look identical to
    healthy ones.
- The toolbar mixes list view options, a global policy toggle (Auto-sync), tool configuration and a library-wide
  destructive action at equal weight.

**Skill detail (21/40)**
([10](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/10-detail-failures-dark-tdd.png))

The detail view is a good file viewer with a tree, byte sizes, rendered `SKILL.md` and frontmatter as a table. It has
**zero deployment information**. `tdd` has two failed targets, and its detail page mentions none of them: no targets,
no projects, no health, no Update/Edit/Disable, only "Change source…". It replaces the list instead of sitting beside
it. Markdown lists lose their bullets because Tailwind preflight's `list-style:none` is not restored in
`.markdown-body`.

**Add flow — GitHub pick and local pick (21/40)**
([13](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/13-add-git-dark.png),
[14](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/14-add-git-pick-dark.png),
[15](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/15-add-overwrite-ask-stacked-dark.png),
[16](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/16-add-local-pick-dark.png))

The model is right: one form for Git or local, a picker for multi-skill sources, and invalid candidates shown
disabled with their reason. The execution stacks dialogs. At the overwrite ask the DOM holds **four `.modal`s at
once**: "Installing Skills… Waiting for your confirmation", "Add skill", "Folder already exists" and "Select skills
to import".

- The picker is titled "Select skills to **import**", although this is Add, not Onboarding import.
- The Git tab's button says "Install" and the local tab's says "Create".
- Every candidate is pre-selected, and there is no picker search (#47).
- There is no already-in-library marker.
- The install summary is not visible while picking (D8 "sticky install summary").

**Import / onboarding (17/40)**
([17](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/17-firstrun-light.png),
[18](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/18-import-review-light.png),
[19](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/19-import-after-light.png))

The grouping model is excellent: one card per name, each variant with its path, Conflict/Match, and a radio to pick
the variant. The presentation leaks and misleads.

- **Raw key leak confirmed** in 31 places: the "Found in claude_code / gemini_cli / pi" pills.
- "Match" (healthy) is painted in warning amber.
- Paths are 2.5:1 in light mode.
- "Skills found: 22" counts variants, not the 12 groups.
- After Import & Sync, the dialog stays open with the 3 divergent leftovers, all **pre-selected** and labelled
  "Match", and "Import & Sync" still armed.
- The notifications report each leftover twice ("Kept a different copy" and "Sync failed: Target folder already
  exists").
- The copy tells the operator to "remove it yourself". That violates PRODUCT.md: "nothing under ~/.claude/skills … is
  ever touched by hand".
- `TARGET_EXISTS` here is never routed to the overwrite ask, the one sanctioned remedy.

**Projects and the assignment matrix (17/40)**
([21](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/21-projects-matrix-rich-dark.png),
[22](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/22-projects-matrix-assigned-rows-dark.png),
[23](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/23-projects-failures-noselection-dark.png),
[24](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/24-projects-matrix-failures-dark.png),
[25](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/25-projects-missing-folder-dark.png),
[37](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/37-projects-960x640-dark.png),
[40](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/40-projects-matrix-failures-light.png))

The sidebar-plus-matrix shape is right. The matrix is a 60-row alphabetical list of every library skill, with the 5
assigned skills scattered through it.

- **Raw key leak confirmed:** the column headers read `CLAUDE_CODE`, `AGENTS_SKILLS`, `PI`, `WINDSURF`, and the cell
  aria-labels read "frontend-design - claude_code".
- The header row is not sticky.
- Cell state is a faint tint only. In light mode stale is `#fffbeb` and missing is `#fff1f2` on white.
- Every globally deployed cell carries the tooltip "Assign it here only to manage it per-project". The same tooltip
  sits on stale and missing cells and on *disabled* cells, which contradicts it.
- The error cell's tooltip is a raw `Permission denied (os error 13): …`.
- Project status dots are colour only, and healthy is shown green.
- "1 tools" is a pluralisation bug.
- A missing folder gets a banner ("Sync is disabled…") but no fix inline.
- At 960 px the project title truncates to "skills…" and "Last synced 5h ago" wraps onto three lines.

**Explore and Explore detail — full critique per D10 (20/40)**
([32](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/32-explore-rich-dark.png),
[33](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/33-explore-search-dark.png),
[34](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/34-explore-detail-installed-offers-install-dark.png),
[42](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/42-explore-light.png))

Explore is a two-column card grid with three buttons per card: Install or Installed, Preview, and Hide.

- In light mode that is 15 filled indigo primaries per viewport.
- In dark mode "Installed" is mint-on-mint at **1.82:1** and "Install" is white on sky at **2.14:1**.
- The metric "730.5K" is unlabelled.
- "+ Manual" opens the generic "Add skill" dialog.
- Most of the featured list is the operator's own library echoed back, with no "not in library" filter and no
  grouping by source.
- **Explore detail does not know the skill is installed.** `docx` shows "Installed" on its card and a primary
  "Install" on its detail page.
- The detail timestamp "just now" is the cache time, not the upstream time.
- In its favour: search splits "Featured" from "Online results", and Back preserves the query.

**Settings (29/40)**
([35](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/35-settings-dark.png),
[44](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/44-settings-light.png))

This is the healthiest surface: labelled fields, helper text under each, and an honest keychain statement for the
token.

- It is one long column with no panes. The operator names Raycast settings panes as a peer.
- Tool selection and scan scope live elsewhere, in the Configure Tools modal.
- "Skills Storage Path — Local copies of Git skills will be stored here" is inaccurate. The path is the central repo
  for every skill.
- Helper text is 2.56:1 in light mode.
- While in Settings no nav item is active, and the gear button has no accessible name.

**Tool configuration modals, global and per project (24/40)**
([27](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/27-project-configure-tools-dark.png),
[28](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/28-global-configure-tools-dark.png))

- **Global:** "Show detected tools only" is checked, yet "Windsurf (not detected)" is listed. It is listed because it
  is selected, and that is the operator's only stale-selection signal (CONTEXT: Effective global target set), but
  nothing explains it. The scan-scope toggle is bolted onto the bottom.
- **Per project:** the virtual group appears as its path, ".agents/skills (9 tools)". Checkbox styles are mixed (a
  native box on the first row, custom boxes on the rest). Every row carries a green "(Installed)", so healthy is not
  silent. The copy "Installed tools are pre-selected" is false: Pi and the group are installed and unselected.

**Notifications (22/36)**
([20](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/20-notifications-light.png))

A centred modal with a readable history, a kind icon, a relative time, "Clear" and "Copy all". Rows are inert: no
Overwrite, no Retry, no Show skill. Each import leftover appears twice. Titles use an ASCII `->`.

**Confirmations: overwrite ask, delete, unsync, remove project (19/36)**
([15](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/15-add-overwrite-ask-stacked-dark.png),
[26](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/26-confirm-remove-project-dark.png),
[29](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/29-confirm-delete-skill-dark.png),
[30](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/30-unsync-no-confirm-dark.png),
[31](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/31-uninstall-all-no-confirm-dark.png))

- **Remove Project** is the best of them: destructive tone, consequences listed, Cancel on the left. Its body and its
  bullets say the same thing twice, and it gives no counts.
- **Delete** mixes three verbs ("Remove skill?" / "delete tdd?" / "Yes, Delete") and says "local copy from Hub" where
  the domain term is central copy. It gives no counts of targets or project assignments.
- **The overwrite ask** names the exact pair and path, which is excellent. It then styles **Overwrite** as the filled
  primary, not the destructive tone.
- **Unsync has no confirmation at all**, in both the card's link icon and the toolbar's library-wide "Uninstall from
  tool directories" (§ P0-1).

---

## Findings, ranked by severity

Each finding gives what is wrong, why (the principle or checklist row), the evidence, and **what the surface must do
inside the decided world**.

### P0-1 — "Uninstall from tool directories" wipes every Sync target in the library with one unguarded click

- **Surface:** My Skills toolbar.
- **What:** a secondary button, placed between "Configure Tools" and search, removed every target of every skill.
  The pill count went from about 60 rows of pills to 1 (a single kept failure).
  - There is no confirmation, no count and no undo.
  - A native `confirm()` was stubbed and never called.
  - "Auto-sync to tool directories" stays **checked** afterwards, a contradiction: the next Refresh re-asserts the
    auto-sync invariant and recreates everything.
  - The only feedback was an error toast about the one target it could *not* remove, rendered in light-theme pink
    over the dark UI and covering the bell and gear.
- **Why:**
  - H5 and H3.
  - ui-ux-pro-max `confirmation-dialogs`, `destructive-emphasis`, `destructive-nav-separation` and `undo-support`.
  - PRODUCT principle 2 (deploy and undeploy are ergonomic, not hazardous).
  - Delete-one-skill *is* confirmed, while undeploy-everything is not. Risk and guard are inverted.
- **Evidence:** [31-uninstall-all-no-confirm-dark.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/31-uninstall-all-no-confirm-dark.png)
- **In the world:**
  - Remove the action from the toolbar.
  - Library-wide undeploy becomes select-all (`x` on a group header, or ⌘A) followed by the bulk rail's "Remove
    from tools…".
  - That opens a confirmation stating counts ("Remove 412 Sync targets across 7 Tools for 60 skills. Project
    assignments are untouched.") with destructive tone and focus on Cancel.
  - When auto-sync is on, the dialog says so and offers "Also turn off auto-sync". Otherwise the next Refresh undoes
    the removal.
  - Also reachable from ⌘K ("Remove all skills from tools…"), same confirmation.

### P0-2 — My Skills misreports state: healthy is loud, broken looks healthy

- **Surface:** My Skills list and card (all view modes).
- **What:**
  1. `docx` shows "Central copy missing". By CONTEXT's definition every Tool's link for it is dangling, yet it still
     wears 7 green "synced" pills.
  2. `alias-skill` (upstream path escapes the repo), `flaky-upstream` and `deprecated-helper` (removed upstream) are
     indistinguishable from healthy skills. There is no skill-level health channel.
  3. Failed targets are the same pill in red. The only differentiator is colour; the dot is identical.
  4. Failed targets can be hidden in "+2 more". `using-git-worktrees` shows 2 of its failures and hides the rest.
  5. `prisma-review` has zero targets and says nothing about it.
  6. There is no exception summary anywhere.
- **Why:**
  - PRODUCT principle 1 (state at a glance) and principle 5 (every failure shown, never a silent success).
  - The contract: "healthy is silent; only exceptions carry a mark, after the name".
  - H1.
  - ui-ux-pro-max `color-not-only`, `color-not-decorative-only` and `compact-label-overflow` (a `+n` summary must not
    hide essential values).
  - The spec's own bake-off finding (b): the product needs a skill-level health channel distinct from target
    health.
- **Evidence:**
  - [06-myskills-failures-dark-unlocatable.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/06-myskills-failures-dark-unlocatable.png) (docx: "Central copy missing" plus 7 green pills)
  - [05-myskills-failures-dark-list-top.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/05-myskills-failures-dark-list-top.png) (fetch-failure skills look healthy)
  - [08-myskills-failures-dark-failed-targets.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/08-myskills-failures-dark-failed-targets.png)
  - [09-myskills-failures-light-failed-targets.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/09-myskills-failures-light-failed-targets.png)
- **In the world:**
  - **Row.** Name, then *exception marks only*: skill-level (source missing / central missing / fetch failed / Edit
    conflict) and target-level (N failed / N stale). Then the **Synced strip**: one mark per installed Tool, hollow
    for not synced, filled for synced, × for failed; near-invisible at N/M = all. Then "N/M".
  - **Central missing.** Every strip mark renders as dangling (×-family or hollow-with-warning), never filled.
  - **Header row.** An exception summary in the contract's words, for example "3 failed targets · 3 stale · View 5
    skills with issues", plus an **Issues** chip in the filter bar.
  - **Colour is never alone.** Shape, then colour, then a text label on hover/focus and in the SR name ("Amp: failed —
    permission denied").
  - **Stale** means target drift only. "Update available" is a separate channel (#62) and must not reuse amber on
    marks (Rejected list: "amber-on-jacks for stale").

### P1-1 — One click on a status pill or the link icon silently undeploys

- **Surface:** My Skills card.
- **What:**
  - Tool pills are `<button>`s whose only affordance text is "Cursor (symlink)". Clicking "Pi" on `release-train`
    unsynced Pi: the pill vanished with no confirmation, the toast said only "Sync disabled.", and there was no undo.
    Because the pill disappears it cannot be clicked back, and re-adding requires the deploy-to-all icon.
  - The link icon means "Remove this skill from all tool directories" when synced and "Deploy this skill to your
    selected tools" when not. It is a state-as-icon toggle that removed all 7 targets of `tdd` with no confirmation
    ([30](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/30-unsync-no-confirm-dark.png)).
- **Why:**
  - H3 and H5.
  - ui-ux-pro-max `hover-vs-tap`, `state-clarity` and `undo-support`.
  - The contract: the Synced strip is *read* state in the row, and click-to-sync lives in the **inspector's per-tool
    grid**.
- **Evidence:** [11-myskills-pill-click-unsyncs.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/11-myskills-pill-click-unsyncs.png), [30-unsync-no-confirm-dark.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/30-unsync-no-confirm-dark.png)
- **In the world:**
  - The row strip is not clickable. Clicking the row focuses it and opens the inspector.
  - The inspector's per-tool grid toggles one target. A toggle-off that removes an artifact shows an inline "Removed
    from Pi · Undo" for about 5 s. A re-sync is the undo, so it is cheap.
  - "Remove from all tools" is a labelled overflow and bulk-rail action with a count-bearing confirmation.
  - The row switch in the contract means *Disable* (#52), which is not unsync. Keep those two verbs distinct in copy.

### P1-2 — Skill detail carries no deployment truth and replaces the list

- **Surface:** Skill detail.
- **What:** `tdd` (2 failed targets) opens a full-page file browser.
  - It shows no targets, projects, health, last-synced time, Provenance ladder or Edit state.
  - Its only action is "Change source…".
  - Update, Edit (invocation mode lives as a tiny glyph on the card), Disable and Delete are absent.
  - Back is the only way out, so list context and scroll are lost.
- **Why:**
  - The contract STORY: "the inspector says why and offers the fix inline (Retry failed · Overwrite and retry ·
    Update · Edit · Disable) with a Health / Upstream / Edits / Last synced ladder".
  - H1, H9 and H6.
  - ui-ux-pro-max `state-preservation`.
- **Evidence:** [10-detail-failures-dark-tdd.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/10-detail-failures-dark-tdd.png)
- **In the world:**
  - The right-third inspector for the focused row (a sheet below about 1200 px) has these parts:
    1. the ladder (Health / Upstream / Edits / Last synced);
    2. a vertical per-tool target list with sync mode and the failure text in product words;
    3. inline fixes (Retry failed, Overwrite and retry for `TARGET_EXISTS`);
    4. project assignments, with assign-to-project from here (D8);
    5. copyable source (D8).
  - The file tree and `SKILL.md` render stay as the inspector's "Files" section or an expand-to-full view.
  - Restore `list-style` in `.markdown-body` / `prose.css`.

### P1-3 — Raw registry keys and backend strings reach the operator (both leaks confirmed)

- **Surface:** Import, Projects, notifications, a11y names.
- **What:**
  - Import review shows "Found in `claude_code`", "`gemini_cli`", "`pi`", "`codex`", "`amp`", "`cursor`" (31
    occurrences) ([18](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/18-import-review-light.png)).
  - The matrix column headers are `CLAUDE_CODE` · `AGENTS_SKILLS` · `PI` · `WINDSURF`
    ([21](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/21-projects-matrix-rich-dark.png),
    [24](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/24-projects-matrix-failures-dark.png)).
  - The matrix checkbox aria-labels are "frontend-design - claude_code".
  - The per-project tool config names the virtual group by its path, ".agents/skills (9 tools)"
    ([27](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/27-project-configure-tools-dark.png)).
  - The matrix error tooltip and a toast read "Permission denied (os error 13): …"
    ([31](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/31-uninstall-all-no-confirm-dark.png)).
  - Notification titles use `->`.
- **Why:**
  - H2.
  - The ADR-0001 contract: user-facing copy lives in `describeCommandError`, and the backend composes no prose.
  - CONTEXT: the Tool catalog is presentation-ready.
- **In the world:**
  - Every Tool label comes from the catalog display name.
  - The virtual group reads "AGENTS standard" with a muted `.agents/skills` and its constituents listed on hover.
  - OS errors are classified into product sentences ("Claude Code's skills folder isn't writable"), with the raw
    detail in a copyable "Details" disclosure.
  - Use `→` or "to".

### P1-4 — Modal as first thought: stacked dialogs, blocking progress, Esc closes everything

- **Surface:** Add, Refresh all, Import, notifications, every dialog.
- **What:**
  - At the overwrite ask, 4 `.modal`s are live at once
    ([15](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/15-add-overwrite-ask-stacked-dark.png)).
  - Refresh (all) blocks the app with a modal titled "**Installing Skills…**", with a spinner *and* a bar
    ([39](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/39-refresh-all-blocking-modal-dark.png)).
  - Notifications, a history log, is a centred modal.
  - The code has more problems (`src/components/shared/Modal.tsx`):
    - Every mounted Modal registers its own `document` keydown Escape handler, so **one Esc closes every stacked
      dialog at once**. An Esc on the overwrite ask also discards the picker and the Add form.
    - There is no focus trap and no focus return to the trigger.
    - There is no enter or exit motion.
  - The dark-theme backdrop is `rgba(17,24,39,.45)` over a `#0b0f1a` ground, so the page behind barely dims
    ([12](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/12-edit-invocation-modal-dark.png)).
- **Why:**
  - operate.md: "Modal as first thought… Exhaust inline/progressive alternatives first".
  - craft-floor: "A modal for a task that needs neither interruption nor protected focus".
  - H3 and H4.
  - ui-ux-pro-max `escape-routes`, `focus-management`, `modal-vs-navigation` and `no-blocking-animation`.
- **In the world:**
  - **Add** is one sheet (right-anchored, or the inspector slot) with three steps in place: source, pick, targets.
    The *sticky install summary* (D8) stays visible throughout, and the overwrite ask is an inline section of that
    summary ("1 target already exists with different content · Overwrite / Keep"), not a fourth dialog.
  - **Refresh (all)** runs in the header ("Refreshing 24/60 · Cancel"). Rows update in place, and the list stays
    usable. The mutation guard already serialises writers.
  - **Notifications** is a popover anchored to the bell.
  - Dialogs are reserved for destructive confirmations. They get one shared primitive (Dialog, D3) with a focus trap,
    focus return, a top-most-only Esc, and motion per the table below.

### P1-5 — Onboarding import's ending contradicts the product's promise

- **Surface:** Import (first run).
- **What:**
  - After Import & Sync, the dialog stays open with the three divergent originals re-offered, **pre-selected**, each
    badged "Match" in warning amber. Pressing Import again would try to import a second variant of an already-managed
    name.
  - The toasts and history carry six rows for three facts.
  - The copy says "Review it and remove it yourself".
  - "Match" and "Found in" pills are amber, so every healthy row looks like a warning.
  - "Skills found: 22" counts variants; the operator thinks in 12 groups.
- **Why:**
  - PRODUCT success: "nothing under `~/.claude/skills` … is ever touched by hand".
  - CONTEXT (Sync target): `TARGET_EXISTS` settles through the **overwrite ask**.
  - D6 (a large import completing is a delight moment).
  - H5 and H9.
  - The contract ("healthy is silent").
- **Evidence:** [18-import-review-light.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/18-import-review-light.png), [19-import-after-light.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/19-import-after-light.png), [20-notifications-light.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/20-notifications-light.png)
- **In the world:**
  - Review rows: a group name, the variant count, and a mark only for Conflict. Match is silent.
  - Tool display names.
  - The count says "12 skills (22 copies) in 7 tools".
  - On completion, the flow shows an in-place result: "9 imported and synced · 3 tools hold a different copy". Each
    of the 3 lists the pair and path with **Overwrite with imported / Keep theirs**, which is the same overwrite ask
    used elsewhere.
  - Then the list is revealed with the new rows marked once (motion table). No lingering review dialog, no duplicate
    notifications.

### P1-6 — The Projects matrix hides what is assigned and encodes state as invisible tint

- **Surface:** Projects and the matrix.
- **What:**
  - The 5 assigned skills are spread among 60 alphabetical rows, and the header row scrolls away
    ([22](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/22-projects-matrix-assigned-rows-dark.png)).
  - Stale, missing and pending differ only by a background tint: `#fffbeb` vs `#fff1f2` on white in light mode, dark
    olive vs dark maroon in dark mode.
  - Only error cells have an sr-only reason.
  - Stale and missing cells carry the *global* tooltip, not their own status.
  - Globe glyphs overlap the checkbox corners with no legend.
  - Some global cells are disabled while their tooltip invites assignment.
  - "All Tools" is repeated on every row, and "Unassign All" appears only on assigned rows, so the column jitters.
  - Project dots are colour only, and healthy shows a green dot.
  - At 960 px the header crushes the project identity
    ([37](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/37-projects-960x640-dark.png)).
- **Why:**
  - H6 and H1.
  - ui-ux-pro-max `color-not-only`, `pattern-texture`, `sortable-table`, `focus-not-obscured` and the sticky-header
    working-memory rule.
  - D5 (960×640).
  - D8 (per-row and inspector project assignment).
- **Evidence:**
  - [21-projects-matrix-rich-dark.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/21-projects-matrix-rich-dark.png)
  - [24-projects-matrix-failures-dark.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/24-projects-matrix-failures-dark.png)
  - [40-projects-matrix-failures-light.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/40-projects-matrix-failures-light.png)
  - [25-projects-missing-folder-dark.png](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/25-projects-missing-folder-dark.png)
- **In the world:**
  - The matrix uses the tracker grammar: 36 px rows, a sticky header and a sticky name column.
  - The default filter is **Assigned** (N), with a chip for All skills and search via `/`.
  - Cells use the Synced-strip shape vocabulary (filled / hollow / ×, plus a distinct stale glyph that is not amber).
    Each cell's tooltip and SR name state its own status in product words.
  - The globally-deployed hint becomes a muted column-level note, not a per-cell glyph.
  - The sidebar shows exceptions only ("2 failed", "folder missing" after the name).
  - The missing-folder banner carries its fix: **Locate folder… · Remove project**.
  - The bulk rail assigns and unassigns selected rows.

### P1-7 — Density and toolbar structure fail the 960×640 gate and the "grouping serves finding" principle

- **Surface:** My Skills and Projects.
- **What:**
  - At 1440×900 a card is 150 px and about 4 skills show.
  - At 960×640 the toolbar wraps to three rows (208 px), and with the banner about 1.5 skills are visible
    ([36](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/36-myskills-960x640-dark.png)).
  - The toolbar holds 9 peer controls: Sort, Group by repo, View, Add, Auto-sync, Configure Tools, Uninstall, Search
    and Refresh. That exceeds the ≤4 working-memory rule.
  - Grouped mode puts `alexstark/handoff` (1 skill) and `anthropics/skills` (9) under the same full-height group
    header, and repeats the repo pill on every child
    ([02](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/02-myskills-rich-dark-grouped.png)).
  - The grid modes clip pill text
    ([03](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/03-myskills-rich-dark-grouped-autogrid.png),
    [04](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/04-myskills-rich-dark-grouped-densegrid.png)).
- **Why:**
  - PRODUCT principle 3 and the recorded frictions.
  - D5.
  - Cognitive load: minimal choices and single focus.
  - ui-ux-pro-max `chip-collection-reflow` and `compact-label-overflow`.
- **In the world:**
  - Header row: title and count, exception summary, Refresh all, and the one primary Add skill.
  - Filter bar: search (`/`), chips All · Issues · Edited · Disabled, and a Grouped/Flat toggle.
  - Auto-sync and tool configuration move to Settings → Tools (they are policy, not view).
  - Sort lives in a column-header affordance or the palette.
  - Sticky group headers show name, count, state summary and Deploy all.
  - Singletons fold into "Individual sources" with a muted inline source label, and the source never repeats inside
    its own group.
  - The one structural collapse at ≤1200 is the inspector becoming a sheet.
  - The grid is round 19's to design (spec D2). Until then, do not ship the broken Auto/Dense grids as they are.

### P1-8 — Light theme is not a first-class derivation (computed AA failures)

- **Surface:** all, both themes.
- **What:** the computed fails at 12–13 px text, which needs 4.5:1, are listed below.

  | Theme | Element | Colours | Ratio |
  |---|---|---|---|
  | Light | Relative time "39d ago" | `#a1a1aa` on white | 2.56:1 |
  | Light | "+2 more" | | 2.33:1 |
  | Light | Synced pill | `#059669` on `#fcfcfc` | 3.67:1 |
  | Light | Failed pill | | 4.41:1 |
  | Light | "Central copy missing" badge | | 3.07:1 |
  | Light | "Review & Import" | white on `#d97706` | 3.19:1 |
  | Light | Import paths | | 2.50:1 |
  | Light | "Found in" pills | | 3.07:1 |
  | Light | Explore author / source label | | 2.56:1 |
  | Light | Settings helper text | | 2.56:1 |
  | Light | Matrix "Sort:" | | 2.50:1 |
  | Dark | Explore "Installed" | | 1.82:1 |
  | Dark | Explore "Install" | | 2.14:1 |

  Toasts (Sonner) render their light palette over the dark UI. My Skills dark passes AA for text.
- **Why:**
  - D5 (WCAG AA in both themes is a hard gate).
  - PRODUCT ("light is a first-class derivation").
  - craft-floor contrast.
  - ui-ux-pro-max `color-contrast` and `color-accessible-pairs`.
- **Evidence:** [09](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/09-myskills-failures-light-failed-targets.png), [18](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/18-import-review-light.png), [32](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/32-explore-rich-dark.png), [42](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/42-explore-light.png), [44](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/44-settings-light.png)
- **In the world:**
  - The contract's three text tokens (`#e6e7ea` / `#a6abb4` / `#858b95` and their light pair) are the only text
    colours.
  - Status colours are used as marks, not as 11 px text.
  - The theme-token pipeline covers the toaster.
  - A CI contrast check on token pairs is part of the `styles:check` guard.

### P2-1 — Explore (D10): one decision per card is three buttons, installed-state lies in detail

- **Surface:** Explore and Explore detail.
- **What:**
  - Each card carries its own primary (Install) plus Preview and Hide. In light mode that is 15 filled primaries per
    viewport ([42](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/42-explore-light.png)).
  - "Installed" is styled as a disabled button.
  - The Preview button floats mid-card.
  - The detail of an installed skill offers **Install**
    ([34](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/34-explore-detail-installed-offers-install-dark.png)).
  - "730.5K" has no unit.
  - "+ Manual" opens a dialog titled "Add skill".
  - Hide gives no feedback and has no undo.
  - The featured list is dominated by skills already in the library, and there is no source grouping, even though
    "tailored groupings from multiple source repos" is a stated success.
- **Why:**
  - ui-ux-pro-max `primary-action` (one per screen).
  - H1, H4 and H8.
  - D10.
- **In the world:**
  - Explore is the same tracker list: name · source · description · installs, with the unit named.
  - Rows already in the library carry a quiet "In library" mark after the name, and a **Not in library** chip is the
    default filter.
  - The inspector is the preview (the same component as Skill detail) and holds the single primary, Install, which
    reads "Open in My Skills" when already installed.
  - Multi-select plus the bulk rail installs several at once through the Add sheet's summary.
  - "+ Manual" becomes "Add from URL or folder…".
  - Hide is an overflow action with Undo.

### P2-2 — Empty and loading states say the wrong thing

- **Surface:** My Skills and first run.
- **What:**
  - "No managed skills yet." renders whenever `visibleSkills.length === 0`
    (`src/components/skills/SkillsList.tsx:116`). That includes a search with no match ("zzzz" on a 60-skill library
    ([38](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/38-search-nomatch-says-no-skills.png)))
    and the moment before data arrives. One capture caught that false-empty flash on a reload of `failures`, which is
    why the first attempt at 05 had to be retaken.
  - First run shows the full 9-control toolbar above a one-line empty box
    ([17](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/17-firstrun-light.png)).
- **Why:**
  - operate.md: "Empty states that teach the interface"; "Skeleton states for loading".
  - H1.
  - ui-ux-pro-max `empty-states` and `progressive-loading`.
  - D6 (first run).
- **In the world:**
  - Loading is 36 px skeleton rows.
  - No match reads "No skills match 'zzzz' · Clear search (Esc)".
  - A true first run leads with Onboarding import as the only primary: "12 skills found in 7 tools · Review and
    import". Add from URL or folder and Explore sit as secondaries, and the list chrome stays hidden until there is a
    list.

### P2-3 — Card actions are icon riddles, and Update is emphasised where nothing is available

- **Surface:** My Skills card.
- **What:**
  - Four icon-only actions per card:
    - a *filled/tinted* Update (styled as primary on every one of 60 cards, with no upstream check behind it);
    - a map-pin meaning "Change source…";
    - a link glyph meaning unsync-all or deploy depending on state;
    - a trash can.
  - Unlocatable cards add a second "Remove" chip beside the trash.
  - The invocation-mode Edit is a 21×18 px glyph after the name.
  - The repo pill copies on click and is labelled only "Copy".
- **Why:**
  - H6.
  - ui-ux-pro-max `aria-labels`, `icon-context`, `primary-action` and `web-target-size` (the 21×18 Edit glyph is
    under 24×24).
  - The contract: ⌘K reaches every action, and the row carries a switch plus an overflow.
- **In the world:**
  - The row holds switch (Disable) and overflow (⋯) with labelled items: Update, Change source…, Edit invocation…,
    Remove from tools…, Delete….
  - Edited state is the contract's Edit pencil after the name, and it appears only when an Edit exists.
  - "Update available" appears only when the separate upstream channel (#62) says so.
  - Everything is also in ⌘K.

### P2-4 — Focus is invisible on inputs and undesigned elsewhere; no keyboard layer

- **Surface:** global.
- **What:**
  - The search, `.input` and settings fields set `outline:none` and substitute a 2 px ring at `rgba(37,99,235,0.1)`
    or `0.12`, which is effectively invisible.
  - Buttons, pills, nav tabs and cards have no `:focus-visible` rule and fall back to the UA default.
  - The gear button has no accessible name.
  - The only keyboard handlers in `src/` are modal Esc and the zoom hotkeys. There is no `/`, j/k, `x` or ⌘K.
- **Why:**
  - D5 (keyboard completion and visible focus).
  - ui-ux-pro-max `focus-states`, `focus-appearance`, `keyboard-nav` and `aria-labels`.
  - The contract's signature interaction.
  - The accessibility audit (ticket 06, `audit.md`) owns the full tab-order walk. These are the findings visible
    from a design seat.
- **In the world:**
  - One focus token: a 2 px iris `#9690ff` ring (`#5a4fdc` in light) with 2 px offset, on every primitive.
  - Row navigation (j/k · x · Enter · Esc), `/` for search, and ⌘K with no motion on keyboard-driven moves.

### P2-5 — Confirmation copy and tone are inconsistent, and counts are missing

- **Surface:** the delete, overwrite and remove-project confirmations.
- **What:**
  - **Delete** says "Remove skill?", then "delete tdd?", then "Yes, Delete", then "Delete local copy from Hub": three
    verbs, and the wrong noun (central copy).
  - **The overwrite ask** gives "Overwrite" the filled accent primary and "Keep existing" the secondary, with no hint
    of what differs (mtime, size, whether the folder holds a `SKILL.md` with the same name).
  - **Remove Project** repeats its body in its bullets and says "symlinks/copies" twice.
  - None of the three states the scale of the change.
- **Why:**
  - H2 and H4.
  - ui-ux-pro-max `destructive-emphasis` and `confirmation-dialogs`.
  - PRODUCT voice ("plain, precise").
- **Evidence:** [29](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/29-confirm-delete-skill-dark.png), [15](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/15-add-overwrite-ask-stacked-dark.png), [26](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/26-confirm-remove-project-dark.png)
- **In the world:**
  - Title is verb plus object ("Delete tdd?").
  - The body states counts from the plan ("Removes 7 Sync targets and 2 project assignments, then deletes the central
    copy.").
  - The confirm button repeats the title verb in destructive tone ("Delete skill"), and focus starts on Cancel.
  - The overwrite ask uses caution tone and shows one line of difference evidence per pair. Where one confirmation
    lists several pairs, it gets "Overwrite all" and "Choose…".
  - The vocabulary follows CONTEXT: delete for the skill, remove from tools (undeploy), unassign for projects. Avoid
    "uninstall".

### P2-6 — Tool configuration contradicts itself

- **Surface:** the global and per-project Configure Tools dialogs.
- **What:**
  - "Show detected tools only" is checked while "Windsurf (not detected)" is listed.
  - The per-project copy "Installed tools are pre-selected" is false.
  - Every row wears a green "(Installed)".
  - The virtual group is shown by path.
  - Checkbox styles are mixed.
  - The scan-scope setting is bolted onto a dialog about sync targets.
- **Why:**
  - H4.
  - CONTEXT (Effective global target set: a selected-but-uninstalled Tool is "the operator's only signal that the
    selection has gone stale").
- **Evidence:** [27](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/27-project-configure-tools-dark.png), [28](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/28-global-configure-tools-dark.png)
- **In the world:**
  - Settings → Tools pane: one list with a silent healthy state and exceptions after the name ("not detected — still
    selected · Remove from selection").
  - The scan scope has its own labelled row.
  - Per-project tools live in the project's header as a Tools section, not a modal.

### P3 — Polish

- **The gradient wordmark** (`App.css:100`, `.brand-text`, blue → purple → orange) is a craft-floor ban and appears
  in every screenshot.
- **Google Fonts are fetched at launch** (`index.css:1`, Fira Sans and Fira Code). The contract says "no network
  fonts" and uses the system UI stack plus `ui-monospace`.
- **Unicode `✕` close glyphs** (`.modal-close`) break the one-icon-system rule. Use Lucide `X`.
- **"Choose folder…" is clipped** to "Choose folder." in the Re-point dialog
  ([43](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/43-repoint-modal-light.png)).
- **"Installing Skills..."** uses three dots, and its title is wrong for Refresh.
- **"1 tools" and "Found 7 skills ready for review"** need pluralisation and a better noun ("7 skills found in your
  tools").
- **The Discovered-skills banner** uses a dashed amber border and an amber "Review & Import" button, a warning tone
  for good news. In the world it is a calm neutral line in the header exception area, and silent once reviewed.
- **The Settings nav** has no active state while in Settings. In the world the sidebar foot holds Settings with an
  active state.

---

## Motion — Emil's Review Checklist, Before/After

| Element / checklist row | Before (measured) | After (inside D6 and the contract's ≤120 ms budget) | Why |
|---|---|---|---|
| `transition: all` on `.btn`, `.tool-pill`, `.skill-card`, `.card-btn`, `.search-input`, `.repo-pill` and more (22 rules, computed `transition-property: all 0.2s ease`) | Every property animates, including layout-affecting ones, at 200 ms `ease`. | Name the properties: `background-color, border-color, color 120ms cubic-bezier(.2,.8,.2,1)`. Never `all`. | Emil: "`transition: all` → specify exact properties". Operate: 150–250 ms max, and the contract caps the world at 120 ms. |
| Hover animation without media query | No `@media (hover:hover) and (pointer:fine)` exists in any stylesheet. | Gate every hover transition behind it. | Emil row "Hover animation without media query". |
| Modal enter/exit | Instant snap: `document.getAnimations()` is empty on open, and the backdrop barely dims. | Enter: opacity 0→1 plus `scale(.97)→1` at 140 ms ease-out, origin centre (modals exempt). Exit: 90 ms opacity only (exit faster than enter). Reduced motion: opacity only. | Emil: "Same enter/exit speed → make exit faster". Dialogs are the one place state change needs spatial continuity. |
| Reduced motion | The only `prefers-reduced-motion` rule is Sonner's own. Spinners (0.6–0.9 s infinite), shimmer (1.5 s) and card transitions ignore it. | A global reduced-motion layer: transitions → 0 ms, spinners → static "Refreshing 24/60", shimmer → a static skeleton tone. | D5, ui-ux-pro-max `reduced-motion`, and the contract ("reduced-motion removes it"). |
| Refresh progress | A modal with an infinite spinner **and** a shimmer bar (1.5 s linear) at once. | One determinate header bar driven by `transform: scaleX()`, one 120 ms linear step per tick, no spinner. Rows update in place. | Emil "Keyframes on rapidly-triggered element → transitions". ui-ux-pro-max `loading-states` (don't double indicators). |
| Pill click / target toggle | The pill disappears and the row reflows (layout shift). | The strip mark crossfades filled→hollow in 120 ms opacity with no reflow (fixed mark slots). | ui-ux-pro-max `layout-shift-avoid`. Contract: strip marks are fixed per installed Tool. |
| Synced strip on sync completion (signature) | None. The operator cannot see which targets just changed. | Marks that changed flash once: opacity .4→1 in ≤120 ms. No motion on marks that did not change. | The contract's signature interaction. D6 "crisp and near-invisible on frequent actions". |
| Install landing / large import completing (rare, D6 delight) | A 4 s toast, "Selected skills installed.", and the list does not move. | The list scrolls to the first new row. New rows hold an iris-soft background that fades to transparent over 600 ms ease-out, once, after arrival. Reduced motion: a static highlight until the next interaction. | D6 reserves delight for exactly these moments. Motion conveys *where* the new things went. |
| Keyboard-driven moves (j/k, x, Enter) | n/a (no keyboard layer). | Zero animation on focus movement and selection toggles triggered by keys. The bulk rail may rise in 120 ms on the *first* selection only. | Emil: "Animation on keyboard action → remove entirely". |
| List first paint | No stagger (correct). | Keep it that way: no staggered row entrances. | Emil suggests a 30–80 ms stagger, but operate.md overrides it: "No orchestrated page-load sequences". Recorded so round 19 does not add one. |
| Toasts | Sonner defaults in the light palette over the dark theme. The error toast covers the bell and gear. | Themed tokens. Anchored bottom-right, away from header controls. Exit 150 ms, enter 200 ms. Errors persist (already true). | Emil "exit faster than enter". ui-ux-pro-max `focus-not-obscured`. |

---

## What to keep (the incumbent's real strengths)

1. **Unlocatable-skill repair inline.** "⚠ Central copy missing · Restore · Remove" and "⚠ Source folder missing ·
   Re-point · Detach · Remove" put the diagnosis *and* the fix on the row, in the domain's exact vocabulary. This is
   the contract's "inspector offers the fix inline" in miniature. Keep the words, the order, and Detach appearing only
   when valid ([06](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/06-myskills-failures-dark-unlocatable.png)).
2. **The invocation-mode (Edit) copy.** It explains each mode in the operator's terms, names the real frontmatter
   keys (`disable-model-invocation: true`), states "Only Claude Code honours these keys today; other tools ignore
   them", and offers "Follow the skill's own setting (currently: …)". It is honest, precise and reusable as the
   inspector's Edits step ([12](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/12-edit-invocation-modal-dark.png)).
3. **The Re-point reassurance line.** "Your source changes only after the update succeeds; existing sync targets and
   project assignments are preserved." It is exactly the reassurance a high-stakes moment needs; keep it verbatim
   ([43](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/43-repoint-modal-light.png)).
4. **One detail component for My Skills and Explore.** It has a file tree with byte sizes, monospace filenames, and
   frontmatter rendered as a table above the body. It becomes the inspector's Files section
   ([10](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/10-detail-failures-dark-tdd.png),
   [34](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/34-explore-detail-installed-offers-install-dark.png)).
5. **Discovery validity is shown, not hidden.** The local picker lists `draft-notes — Invalid: Missing name in
   frontmatter` and `scratch — Invalid: Missing SKILL.md` as disabled rows with the reason
   ([16](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/16-add-local-pick-dark.png)).
6. **The overwrite ask names the exact pair and path.** "ts-reset → Claude Code /Users/alex/.claude/skills/ts-reset",
   with "Keep existing" as the safe verb. Keep the content and fix the tone and placement
   ([15](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/15-add-overwrite-ask-stacked-dark.png)).
7. **The import review's data model.** One group per name, variants with paths, Conflict vs Match, a radio for the
   variant, and link variants shown as `link → …`. It is the right model with the wrong paint
   ([18](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/18-import-review-light.png)).
8. **Per-target explanations exist.** The pill `title` carries the sync mode ("Cursor (symlink)"). Error pills explain
   themselves ("this sync target is in error — its artifact could not be updated or removed"). Matrix error cells
   carry sr-only text. Promote all of this from `title` to visible inspector text.
9. **The stale-selection signal exists.** "Windsurf (not detected)" on the card and in Configure Tools is the only
   place the Effective global target set's staleness surfaces. Keep the concept and make it an exception mark with a
   fix.
10. **The notification history.** It has kinds, relative time, errors that persist until closed, and "Copy all" for
    forensics ([20](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/20-notifications-light.png)).
11. **Remove Project's confirmation shape.** Destructive tone, consequences listed, Cancel left and confirm right
    ([26](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/26-confirm-remove-project-dark.png)).
12. **Settings helper text** under every field, and the keychain sentence for the token
    ([35](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/35-settings-dark.png)).
13. **Explore's Back preserves the search query**, and search separates Featured from Online results
    ([33](/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-opus/33-explore-search-dark.png)).
14. **The dark ground is already near the world's.** Page `#0b0f1a` against the contract's `#0e0f12`, and My Skills
    dark text passes AA. The migration is a token swap, not a re-light.

---

## Cross-cutting themes (what every surface gets wrong the same way)

1. **Inverted salience.** Healthy states get the colour and the space: green pills, green "(Installed)", green
   project dots, amber "Match". Exceptions get a tint or a title attribute. The contract's rule, "healthy is silent,
   exceptions after the name", fixes every surface at once.
2. **Colour-only state.** The red and green pills, matrix tints, project dots and Conflict/Match badges all rely on
   colour. The world's shape vocabulary (filled / hollow / × plus a distinct stale glyph) must be the one grammar
   across the strip, matrix and sidebar.
3. **Guards in the wrong places.** Deleting one skill asks first. Undeploying one skill, one tool, or the whole
   library does not. Guard by blast radius (counts), and give cheap reversible actions an Undo instead of a question.
4. **Modal as first thought.** Add, pick, overwrite, progress, notifications and tool configuration are all dialogs,
   and they stack. The world has an inspector and sheets. Dialogs are for destructive confirmation only.
5. **Leaky seams.** Registry keys, OS error strings and ASCII arrows cross into the UI. Every label must come from
   the Tool catalog or `describeCommandError`.
6. **Vocabulary drift.**

   | Concept | Words the UI uses | Should be |
   |---|---|---|
   | Adding a skill | install / import / create / "Manual" | install (Add) |
   | Undeploying and deleting | remove / delete / uninstall / "Sync disabled" | remove from tools; delete |
   | The stored copy | "local copy" | central copy |
   | Refresh | "Installing Skills…" | Refreshing |
   | Onboarding import | Review & Import | (keep) |

   Put a glossary check on the i18n catalog in round 19.
7. **No keyboard layer and invisible focus.** No `/`, j/k, `x` or ⌘K, and input focus rings at 10 % alpha. D5
   blocks round 19 on this.
8. **Light is second-class.** Every AA failure except Explore's buttons is light-only. The toaster is not themed.
9. **Rare moments end on toasts.** First run, import complete and install landing, the three D6 delight moments, are
   the weakest-designed transitions in the app.
10. **Every page reinvents list chrome.** Sort and "Group by repo" appear on Projects too, where they mean
    something else. The world needs one filter-bar primitive with per-surface chips.

---

## Deterministic scan (run after the design review)

`impeccable detect --json src/components src/App.tsx` found **0**. The JSX carries no inline style tells.
`impeccable detect --json src/App.css src/index.css` found **2**:

- **`gradient-text`** at `src/App.css:100` (`.brand-text`). **Agree.** It is the P3 wordmark finding.
- **`side-tab`** at `src/App.css:2813` (`.markdown-body blockquote { border-left: 3px }`). **False positive.** A
  blockquote rule in rendered Markdown is a document convention, not a card accent. Keep it, but thin it to the
  world's 2 px line token.

The detector misses the substantive problems: the missing confirmations, colour-only state, raw keys, stacked
modals, contrast and the Google Fonts fetch. That is expected, because they are behavioural or computed. No browser
overlay was injected, so no user-visible overlay exists.

## Persona red flags (validated operator only: Alex; plus the D5 accessibility gate)

- **Alex (the operator, a power user at a dark desk):**
  - He cannot answer "what's broken?" without scrolling 60 cards and hovering pills.
  - He cannot select several skills and assign them to a project; there are no bulk actions (a recorded friction).
  - There is no `/` and no ⌘K.
  - A mis-click on "Uninstall from tool directories", or on a pill, silently undeploys.
  - Refresh all blocks the app for the duration.
  - Every Add goes through up to 4 dialogs.
- **The D5 accessibility gate:**
  - State is conveyed by colour alone in 5 places.
  - Input focus rings are at 10 % alpha.
  - The gear button has no name.
  - Stacked modals have no focus trap or focus return, and one Esc closes all of them.
  - The Edit glyph target is 21×18 px, under 24×24.
  - There are 13 computed contrast failures.
  - `audit.md` carries the per-check table.

---

## Open questions for the operator's grill (ticket 09)

1. **Library-wide undeploy.** Does it survive as an action at all, given auto-sync on re-asserts every target on the
   next Refresh? The options:
   - (a) only via select-all plus the bulk rail with a count confirmation;
   - (b) couple it to turning auto-sync off;
   - (c) drop it and let Tool deselection in Settings → Tools do the job.
2. **Row strip interactivity.** Is the Synced strip strictly read-only in rows, with click-to-sync only in the
   inspector's per-tool grid? The contract lists that grid as "an option", and the Unresolved section asks whether it
   is default-on. I recommend default-on in the inspector and read-only in the row.
3. **Import leftovers.** Should divergent originals after Onboarding import go straight into the overwrite ask inside
   the import result, or stay report-only with a "Review 3 differing copies" entry?
4. **The Projects page's future.** Once rows and the inspector can assign to projects (D8), is the per-project matrix
   still the primary assignment surface, or a review surface filtered to Assigned by default?
5. **Refresh (all) while running.** Can the operator browse, Edit or queue while the header shows progress, given the
   mutation guard serialises writers? Or does the list go read-only for the duration?
6. **Explore's shape.** Tracker rows like My Skills, or the round-19 card grid? And should "Not in library" be the
   default filter?
7. **Notifications.** Should rows carry actions (Retry, Overwrite…, Show skill), which turns history into a work
   queue, or should actions live only on the row and inspector, with notifications as pure history?
8. **Settings panes.** Adopt Raycast-style panes (General · Tools · Storage & cache · GitHub · Updates ·
   Diagnostics), moving Configure Tools and scan scope into Settings → Tools?
9. **Provenance labels.** For imported skills, is "Managed here · Imported from Cursor" the wording, or should the row
   show nothing, since imported is a healthy state, and keep provenance to the inspector's Upstream step?
10. **Delete safety.** A confirmation with counts (today's model), or a short-lived soft delete with Undo (a new
    backend capability)?
