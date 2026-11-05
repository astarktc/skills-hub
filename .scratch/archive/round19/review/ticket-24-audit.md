# Ticket 24 — Spec review and adversarial audit

**Verdict: FAIL — one P1 focus regression; one P2 density note. No P0 found.**

Reviewed `t3/card-grid` at `936f464ffd292247f0c876d4801197a817d98bca`: the uncommitted diff and new `src/components/skills/SourceGroupHeader.tsx`. Source remained read-only. This report is the only repository file written by this reviewer. Browser evidence used ego-browser, an isolated task space, fixture mode on **5198 only**, and real keyboard/pointer input. No native app or live library was touched. The existing clean main checkout at the same SHA was served briefly on the same port for the baseline comparison, then the worktree server was restored. Both browser space and audit server were closed afterward.

Inputs: ticket 24 Work, Comments and Result; round-19 D21/D31/D32; archived round-18 D2; ticket 31 Result and standards S3; DESIGN.md. Applied the freshly fetched Web Interface Guidelines, with repository conventions taking precedence.

## Spec

### (a) Missing / partial

- Ticket: **“same selection, same keyboard model (`j/k` moves across the grid by reading order)”**. The normal keymap works, but the extra Shift-click suppression leaves focus behind. A first Shift-click leaves `<body>` active; a range clicked from search leaves search active. Thus keyboard continuation is partial: `x` does nothing in the former case and `j` edits the search in the latter. **F1**. This also contradicts D21's **“Focus never lands on `<body>`”** floor and the audit brief's explicit no-body requirement.
- No missing card-model, header-extraction, density, persistence, or token work found. D32 and round-18 D2's **“Both list and card-grid view modes exist in the new world”** are satisfied.

### (b) Scope creep

- None unapproved. Ticket Result explicitly calls out the two additions: **“Shift-press on a checkbox”** and **“`scroll-pt-15` on the list pane.”** The review brief accepts both. The scroll change is correctly confined to the list pane. The Shift-click addition needs the repair below, not removal of its useful text-selection suppression.
- No new token, primitive variant, stylesheet, command, dependency, or matrix behavior was introduced. Grid header initials are deliberately omitted, as accepted in the brief.

### (c) Implemented but wrong

- Result: **“It no longer extends the page's text selection”** is true, but incomplete: the new `preventDefault()` also suppresses native focus acquisition, without replacing it. **F1**.
- Result: **“List unchanged … pixel-identical to main”**: all six headers have equivalent normalized DOM and identical rectangles; both visible headers are pixel-identical at 1280 dark. The full-window captures differed in 64 of 1,024,000 pixels outside the headers, maximum channel delta 24. I therefore corroborate the header-preservation claim, not literal whole-window zero-pixel difference. No visible layout regression was found.
- DESIGN.md matches the rendered grid, including grounds, descriptions, pitches, density floors, footer wrapping and menu names. The focused card uses its existing **1 px border colored `ring`**, rather than the list row's inset box-shadow; this preserves the inner-edge appearance without resizing the card and is accurately documented in the grid section.

## Findings

### F1 — P1: suppress text selection without suppressing checkbox focus

**Location:** `src/components/skills/SkillRow.tsx:404–406`.

```tsx
onMouseDown={(event: MouseEvent) => {
  if (event.shiftKey) event.preventDefault();
}}
```

**Evidence:** `ego-browser nodejs` against `http://localhost:5198/?scenario=rich#/skills`, after a full reload and startup settlement, at 1280×800. Run separately with `list`, `auto-grid`, and `dense-grid`; hold Shift, click `Select artifacts-builder`, release Shift, then press `x`:

```text
list       afterShiftClick { body:true, selected:1, textSelection:"" }
           afterX          { body:true, selected:1 }
auto-grid  afterShiftClick { body:true, selected:1, textSelection:"" }
           afterX          { body:true, selected:1 }
dense-grid afterShiftClick { body:true, selected:1, textSelection:"" }
           afterX          { body:true, selected:1 }
```

Control case: an ordinary click focuses `Select artifacts-builder`, `body:false`.

A second realistic sequence demonstrates the consequence without starting on body: click the first checkbox, focus the empty search, Shift-click `Select pdf`. All six expected cards are selected, but the active element remains `INPUT[data-skill-search]`; pressing `j` sets the query to `"j"` instead of moving through cards.

Evidence files: `/tmp/t24-f1-focus.json`, `/tmp/t24-f1-list.png`, `/tmp/t24-f1-auto-grid.png`, `/tmp/t24-f1-dense-grid.png`. The screenshots establish the selected state; the JSON and command output establish actual focus, which a screenshot alone cannot prove.

**Exact proposed fix:** retain suppression, but explicitly focus the checkbox's current target on the shifted press. Type the event for `HTMLElement` so `.focus()` is available:

```tsx
onMouseDown={(event: MouseEvent<HTMLElement>) => {
  if (!event.shiftKey) return;
  event.preventDefault();
  event.currentTarget.focus({ preventScroll: true });
}}
```

Keep the existing `onClickCapture` range flag. Do **not** add `select-none` to the card/description. Re-run both reproductions in all three modes: first Shift-click must focus the clicked checkbox and `x` must toggle that card; Shift-click from search must leave focus on the clicked checkbox and `j` must navigate. Retain the six-card range and empty `getSelection()` proof. No source fix was made in this review.

### F2 — P2 note: Dense is usable, but the edited-name allowance is smaller than the headline suggests

**Location:** `src/components/skills/listColumns.ts:28–29`; DESIGN.md “The grid” density bullet.

At 1280×800, Dense renders four **251.5 px** cards per row. The name budget is **138 px** normally and **110 px** with the Edit pencil, not a uniform ~130 px. **13/60 names truncate** in `rich`, versus 0/60 in Grid. Examples: `improve-codebase-architecture` 197→138 px; `finishing-a-development-branch` 202→138 px; `api-contract-review` 123→110 px. All retain full titles and accessible names. At 960 Dense truncates 4/60 names; at 1800 it has six cards and again truncates 13/60.

Evidence: `ego-browser nodejs` measurements in `/tmp/t24-rich-measures.json`; `/tmp/t24-rich-dense-grid-light-1280.png` and `/tmp/t24-rich-dense-grid-dark-1280.png`.

**Judgment / exact proposed disposition:** keep the **15rem** floor; this remains a usable opt-in dense view, with Grid providing the nontruncated alternative. Raising it to 16rem would reduce 1280 to three columns, largely erasing its density advantage there. No layout fix is recommended. Clarify DESIGN.md's name-budget sentence to “At 1280, approximately 138 px normally / 110 px with the Edit pencil; 13 of the 60 rich-fixture names truncate.” Have the operator judge these captures rather than silently widening the accepted density.

## Verification evidence

### 1. One header, one row model

- `rg -n 'data-group-header|groupChip|library.group.deployAll|ChevronDown|ChevronRight|invokeTauri|select-none' src/components/skills/{SkillsList,SourceGroupHeader,SkillRow}.tsx` locates the header renderer/chip/chevrons/Deploy-all only in **SourceGroupHeader**, and no direct IPC call or blanket selection-blocking CSS in the card. `SkillsList.tsx:296–310` renders that component once per group.
- Header interface: `layout: "list" | "grid"` is the sole presentation discriminator; `legend?: ReactNode` is a slot. `collapsed` and `busy` are actual state, not proliferating style flags. Ordering and measurements remain in SkillsList. S3 is fulfilled.
- Six main/worktree header rectangles and normalized DOM trees match (normalization removes generated IDs and sorts class tokens). Both visible headers have **0 differing pixels**. Evidence: `/tmp/t24-list-{main,worktree}-dom.json`, `/tmp/t24-list-{main,worktree}-ax.txt`, `/tmp/t24-list-{main,worktree}-dark-1280.png`, `/tmp/t24-header-comparison.json`.
- Card: same checkbox, name/pencil, `rowMarks`, two-line reserved description with full title, strip/count, scope, switch and overflow. No “Not deployed” row badge; the group summary and strip's accessible state legitimately still use those words. `data-skill-item` (428), `data-skill-row` (172), and conditional `data-skill-health` (213) remain available to lifecycle resolution. Verbs still use `actions.run` / `repair`.
- Actual styles: base `row`; hover `accent`; selection `raised` and **3 px** leading pseudo-element plus checked box; inspected/focused card `raised` and ring-colored border. Groups compute `overflow:clip`, radius **10 px**.

### 2. Keyboard and mutation focus

All ordinary navigation checks below pass; F1 is the explicit exception.

`j` focused names in every dark run: **artifacts-builder → canvas-design → docx → frontend-design → mcp-builder → pdf → pptx → webapp-testing**; `k` returns to **pptx**. Wrap positions vary as expected:

| Window | Grid columns / first wrap | Dense columns / first wrap |
| --- | --- | --- |
| 960×640 | 2 / docx | 3 / frontend-design |
| 1280×800 | 3 / frontend-design | 4 / mcp-builder |
| 1800×1000 | 4 / mcp-builder | 6 / pptx |

Coordinates and focused names: `/tmp/t24-rich-measures.json`. No body focus in those walks.

- `x` selects the focused card. Ordinary first-checkbox click then Shift-click on pdf selects exactly the first six cards in reading order, `getSelection()===""`. Enter opens mcp-builder; Escape first clears the six selections, then closes the inspector and retains row focus. Also verified Enter/Escape through the inspector **sheet at 960**.
- Delete via overflow → Remove… → confirmation: **canvas-design → docx**; last **pi-lens → ui-ux-pro-max**; sole search result **pptx → My Skills heading** (`data-surface-heading="skills"`). No body focus. Captures: `/tmp/t24-delete-{next,previous,heading}.png`.
- Left Arrow on anthropics' chevron removes its nine cards; `j` goes to **api-contract-review**. Collapse persists through reload. Right/expand restores them. No Home/End key is implemented in the existing list, so none is required of the grid.
- View menu: keyboard Enter, ArrowDown through Name A–Z / Most recent / Date added / List / Grid [/ Dense grid], Enter, Escape. Both Grid and Dense work; focus returns to the trigger, whose AX name includes **Grid** or **Dense grid**. Dense survives reload. Radio selection leaves the menu open until Escape; this is existing menu behavior, not lost focus.
- Tab order: checkbox → name → optional pencil → switch → overflow → next checkbox. Keyboard-focused controls compute a **2 px iris outline**. No missing card-control focus ring found.

### 3. Widths, truncation, sticky headers and motion

Measured **every rendered card**, not just the viewport: rich 60 cards and failures 24 cards, each at three sizes × two densities × two themes (24 configurations total).

| Window | Grid columns / width | Dense columns / width |
| --- | --- | --- |
| 960×640 | 2 / 417 px | 3 / 279.33 px |
| 1280×800 | 3 / 332.66 px | 4 / 251.5 px |
| 1800×1000 | 4 / 376.5 px | 6 / 251.66 px |

Every configuration: **0 card `scrollWidth > clientWidth`; no page horizontal overflow; 0 truncated/clamped text elements lacking a title**. Failures' eight-mark strips measured **152 px**, **20 px pitch**, no clipped marks; smallest tested card client width **250 px**. N/M and scope fit/wrap. Data: `/tmp/t24-{rich,failures}-measures.json`.

Captures: `/tmp/t24-{rich,failures}-{auto-grid,dense-grid}-{dark,light}-{960,1280,1800}.png`. Inspected light Dense 1280 and dark failures Dense 960 images directly, as well as the inspector capture.

At 1280 with inspector beside, pane **720 px**, Grid columns **325 + 325 px**. Group clipping is radius-aware; header is sticky with z-index 10. Walking **22 cards/rows forward and 22 back** in each of list/Grid/Dense at 960 produced **0 focused item tops hidden behind their group header**. Evidence: `/tmp/t24-scroll-keys.json`. List scroll padding computes **60 px**; inspector padding remains **auto**. This extra fix does not alter inspector scrolling.

Reduced-motion emulation reports `matchMedia(...reduce).matches === true`. Keyboard view switching has **0s** card/subtree transition durations and **no active animations**. The grid adds no transition in normal mode either.

### 4. Contrast, text selection and empty state

Actual rendered color pairs were sampled, not just assumed from token names:

| Pair | Dark base / hover / selected | Light base / hover / selected |
| --- | --- | --- |
| Description on card ground | 8.33 / 7.65 / 6.61 | 8.36 / 6.75 / 6.75 |
| Scope on card ground | 6.00 / 5.52 / 4.77 | 6.24 / 5.04 / 5.04 |

Focused ground equals selected; the focus edge uses the documented ring token. Evidence: `/tmp/t24-contrast.json` (dark rows) and `/tmp/t24-light-colors.json` (fresh unfocused light baseline; supersedes the first light loop, which retained an inspected hash). All these text pairs pass AA.

The Shift-click fix is **event suppression, not `select-none`**. Description computes `user-select:auto`; a real drag selected “Build multi-component HTML”. Copyable text is not disabled. As with the shared row click handler, the drag's trailing click also opens the inspector; no new text-selection CSS causes that behavior. F1 concerns focus acquisition only.

Both grid preferences with `?scenario=empty` render the unchanged “Your library is empty” onboarding surface, no cards. Captures: `/tmp/t24-empty-{auto-grid,dense-grid}-light-960.png`.

### 5. Persistence and guards

- The unchanged compatibility definition is `src/lib/preferences.ts:19–24`. The byte-identical key assertion is actually in **`src/lib/persistedPreference.test.ts:134`**, not `preferences.test.ts` (the latter tests the theme script). The existing key assertion is green.
- New `useListLens.test.ts` cases cover all three values round-tripping, unknown fallback, identical on-screen order with collapsed groups excluded, and Delete's next-card fold. Browser storage `masonry` renders **List**, independently confirming fallback.
- Matrix remains isolated: `AssignmentMatrix.tsx:85–94` passes its own grouping preference and no view-mode preference; `useListLens.ts:118` uses `LIST_ONLY` in that case. No production lens/matrix code changed.

Fresh permitted commands, all from the worktree:

| Command | Result |
| --- | --- |
| `npx vitest run src` | **exit 0; 56 files, 1250 tests passed** (`/tmp/t24-audit-vitest.log`) |
| `npx tsc --noEmit -p tsconfig.app.json` | **exit 0**, no diagnostics |
| `npm run styles:check` | **exit 0**; App.css **355**, ceiling unchanged; **50 migrated**; scope/raw-color/self-test green |
| `git diff --check` | **exit 0**, no output |
| `npx vitest run src/i18n/resources.test.ts src/hooks/useListLens.test.ts src/lib/persistedPreference.test.ts` | **exit 0; 3 files, 32 tests passed** (`/tmp/t24-targeted-tests.log`), including reverse i18n guard |

The child's full npm/Rust gate claim was **not independently rerun**: this audit honored the permitted command list. No VoiceOver speech, native WKWebView, or automated axe run is claimed (axe-core was not installed in this worktree). These are evidence limits, not invented passes.

## Final verdict

**FAIL.** The grid itself meets the requested model, shared-header architecture, layout, persistence and normal keyboard/focus requirements; the permitted guards are green and the two-theme/six-layout measurements substantiate the child's main claims. The accepted Shift-click addition introduces a reproducible P1 focus regression in both grid and list: fix F1 and recheck the two short focus sequences before advancing to the next ticket. Dense's truncation is an explicit, usable opt-in trade-off, not a reason to widen the accepted 15rem floor automatically.
