⚠️ DEGRADED: single-context (one independent critic in the orchestrator’s requested blind review; design assessment followed by a local detector pass, not a nested dual-agent Impeccable run).

# Skills Hub — incumbent design critique · Astra

**Verdict:** the incumbent exposes a capable deployment engine through a browsing interface. It is reasonably easy to read a skill’s instructions; it is unnecessarily difficult to answer **what needs attention, where it is deployed, and what this action will change**. The decided Tracker List / Patch Bay hybrid addresses the right problem. A palette substitution alone would not.

**Top five:** library-wide unsync without confirmation; insufficient skill/target exception hierarchy; keyboard-inaccessible project selection; the project header’s failure at 960×640; unreadable light-theme secondary text. The stacked Add dialogs and clipped Tool pills are also release-gate concerns.

No source edits, commits, staging, other critic’s report, or new server. Only Alex is treated as a validated user. No invented personas, usage statistics, or new visual direction.

## Method, evidence, and limits

Read the requested product, direction, domain and rubric material. Used the already-running `http://localhost:5175` fixture, with new preview tabs `tab_5` and `tab_7`, sequential T3 preview calls, Settings theme controls / system appearance emulation, DOM inspection, and saved screenshots. Reviewed `rich`, `failures`, `first-run`, and `empty`; 1280×800 and 960×640 **CSS** viewports. Some 960px captures are rasterised at a larger pixel size; the evaluated `innerWidth/innerHeight` was 960/640. This is not evidence of a 1280px CSS layout.

The hidden renderer repeatedly reloaded or returned half-painted captures between otherwise successful actions. Repainting and resizing recovered many, not every combination. I inspected the actual saved images, rejected misleading captures, and do **not** treat those automation failures as app defects. The findings below cite usable captures, not merely successful snapshot responses.

**Coverage is substantial but not the requested complete surface × scenario × theme cross-product. Do not close ticket 06’s coverage gate from this report alone.** In particular, a dark/light pair for one shared surface is not proof that every error variant was tested in both themes. Slow-motion `latency=3`, full keyboard task completion, reduced-motion emulation, and screen-reader operation remain unverified. Motion observations below are loaded-browser CSSOM evidence, not a recording-based animation audit.

Two fixture-author captures were inspected as supplementary visual evidence: Add dark and overwrite dark. Only the latter is included in the saved evidence here, explicitly named `reference-author-overwrite-dark.png`, copied from `evidence/app/overwrite-ask-dark.png`. It supplements my own light overwrite capture; it is not an independent live replay. No other critique was consulted.

### Coverage ledger

| Surface | Direct evidence obtained | Limits |
|---|---|---|
| My Skills | Rich grouped and flat List; grouped Auto Grid in dark/light; flat Dense Grid dark; failures flat List light; 960px toolbar | Not every grid/grouping variant in failures or both themes |
| Skill detail | Rich dark `react-perf-audit`; failures light `local-notes` | Other missing-central-copy detail and both-theme cross-product not captured |
| Add / Git picker | Add rich light; Git picker failures light at 960px; rich installation through overwrite | Dark Add inspected in fixture-author reference, not counted as my live capture |
| Local picker | Failures light at 960px; valid and invalid candidates; invalid path followed by Browse recovery | Dark picker / rich-specific pass not captured |
| Projects / matrix | Rich dark normal width; failures dark at 960px; raw-key and project-selection DOM checks | Failed light capture excluded; not a full keyboard test |
| Explore / detail | Rich catalog DOM walkthrough; failures catalog and detail dark/light | Search results, Hide recovery, and network-error rendering not exhaustively exercised |
| Settings | Failures dark/light; rich live DOM/contrast checks | Not every setting’s mutation/failure |
| Onboarding import | First-run dark/light, 12 name-groups represented by 22 found copies; Escape dismissal | Did not complete first-run import or test auto-sync-off copy-removal review |
| Tool configuration | Failures light; detected and selected-but-undetected rows | Dark and expanded full registry not captured |
| Notifications | Failures dark after Refresh; rich light after global unsync | Full failure list not captured in light |
| Overwrite ask | Rich light live; fixture-author dark reference | Failures-specific replay incomplete |
| Delete confirmation | Failures dark | Light attempt reloaded before usable capture |
| Unsync | Global unsync immediately executed in rich and failures; rich light history captured | There was **no global confirmation to capture**; per-skill/shared-dir variants not fully verified |
| Remove Project | Rich light dialog; dark DOM walkthrough | Dark screenshot was half-painted and excluded; kept-error outcome not completed |
| Empty | Empty light at 960px | Dark empty not captured |

### Detector corroboration

After forming the design assessment, ran:

```text
.pi/skills/impeccable/scripts/impeccable detect --json src/components
exit 0; result []
```

Output: `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/detector.json`.

A clean markup detector did **not** catch the measured 2.56:1 text contrast, the 960px header collapse, clipped Tool labels, or the stacked live dialogs. Do not substitute its result for visual/interaction evidence. No live detector overlay was installed or claimed.

## Design specificity and overall impression

The incumbent is not incoherent: repeated source information, recognisable Tool names, Markdown inspection, explicit invalid-candidate reasons, and typed recovery copy all belong to Skills Hub. But the composition is category-generic: broad rounded containers, repeated green success pills, repeated GitHub icons, and a brightly coloured action on nearly every card. The product’s valuable distinction—one managed library with accountable deployment—is not the organising hierarchy.

The emotional journey is backwards. The quiet daily state is visually busy, while the high-stakes moments are under-explained. Opening a healthy library produces many green marks; discovering a source problem requires scanning down the library or reading notifications; one globally scoped removal executes before Alex has seen its count. The intended visual world should reverse that: **quiet normal state, explicit exception, precise consequence, obvious repair**.

## Heuristic scores by surface

Scores use Impeccable’s Nielsen rubric: 0 absent/broken, 1 weak, 2 partial, 3 good, 4 excellent. These are design judgments over the observed states, not a usability-study score or an accessibility certification. All ten heuristics apply to these Operate surfaces; each denominator is 40. Coverage limitations above constrain confidence.

H1 = status; H2 = real-world/domain match; H3 = control; H4 = consistency; H5 = prevention; H6 = recognition; H7 = efficiency; H8 = minimalism; H9 = recovery; H10 = help.

| Surface | H1 | H2 | H3 | H4 | H5 | H6 | H7 | H8 | H9 | H10 | Total |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| My Skills List | 1 | 3 | 2 | 2 | 1 | 2 | 1 | 1 | 2 | 1 | **16/40 · Poor** |
| My Skills card grids | 1 | 3 | 2 | 2 | 1 | 1 | 1 | 1 | 2 | 1 | **15/40 · Poor** |
| Skill detail | 1 | 3 | 3 | 2 | 2 | 2 | 1 | 2 | 1 | 2 | **19/40 · Poor** |
| Add entry | 2 | 2 | 3 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | **21/40 · Acceptable** |
| Git picker | 2 | 2 | 3 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | **21/40 · Acceptable** |
| Local picker | 2 | 2 | 3 | 3 | 3 | 2 | 2 | 2 | 3 | 2 | **24/40 · Acceptable** |
| Projects + matrix | 2 | 2 | 1 | 2 | 2 | 1 | 2 | 2 | 2 | 1 | **17/40 · Poor** |
| Explore catalog | 2 | 3 | 3 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | **22/40 · Acceptable** |
| Explore detail | 1 | 3 | 3 | 1 | 2 | 2 | 1 | 2 | 2 | 2 | **19/40 · Poor** |
| Settings | 3 | 3 | 3 | 2 | 2 | 2 | 2 | 3 | 2 | 3 | **25/40 · Acceptable** |
| Onboarding import | 2 | 2 | 3 | 2 | 2 | 1 | 2 | 1 | 2 | 2 | **19/40 · Poor** |
| Tool configuration | 2 | 2 | 3 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | **21/40 · Acceptable** |
| Notifications | 3 | 3 | 3 | 2 | 2 | 2 | 1 | 2 | 3 | 2 | **23/40 · Acceptable** |
| Overwrite ask | 3 | 3 | 3 | 2 | 3 | 3 | 2 | 2 | 3 | 2 | **26/40 · Acceptable** |
| Delete confirmation | 3 | 3 | 3 | 2 | 2 | 2 | 2 | 3 | 2 | 2 | **24/40 · Acceptable** |
| Global unsync flow | 3 | 2 | 1 | 1 | 0 | 1 | 2 | 2 | 2 | 1 | **15/40 · Poor** |
| Remove Project dialog | 3 | 3 | 3 | 3 | 3 | 3 | 2 | 3 | 2 | 2 | **27/40 · Acceptable** |
| Empty library | 1 | 3 | 3 | 2 | 2 | 2 | 1 | 2 | 1 | 1 | **18/40 · Poor** |

### Surface reads

**My Skills List.** This is a vertical stack of cards, not an operational list. At 1280×800, the grouped opening shows only two complete skills: one singleton source consumes its own header and broad row before the nine-skill source begins. Four levels—app navigation, view preferences, deployment controls, discovery banner—precede the library. Neither total library size nor an actionable exception summary leads the page. Source grouping is valuable, but repeating the source pill inside every grouped row spends space twice. The accepted 36px tracker rows, singleton fold, Issues chip and source-level Deploy all should be treated as workflow fixes, not cosmetic changes.

**Card grids.** Auto Grid and Dense Grid materially improve the number of names visible, so preserving a grid option is right. They do not yet preserve the meaning of the content: Tool pills are clipped horizontally and `Claude Code` wraps into a fixed-height pill. A single-source singleton leaves most of its row blank. The new-world grid must use the same selection, exception, scope and Tool-state model as List; it must not be the old card compressed until its controls break.

**Skill detail.** The file tree and Markdown reader are useful and legible. The primary hierarchy, however, is “read files”, not “understand and operate this Managed skill”. The failures fixture’s `local-notes` detail presents a normal source path and readable central content without carrying forward “Source folder missing”. A reader can easily mistake readable content for healthy Provenance. Keep the reader, subordinate it to the Health / Upstream / Edits / Last synced ladder, and retain list context in the decided inspector.

**Add entry.** Git Repository and Local Folder are recognisable alternatives; tool selection is explicit. But the optional name and seven Tool choices appear before the app knows whether the source contains one skill or many. “Install” first discovers candidates and opens another modal; the next title says “import”. Alex has to remember the chosen repository and deployment selection through that change of context. One bounded Add flow, with a stable source and deployment summary, is the necessary improvement.

**Git picker.** Names, descriptions and subpaths are useful, and selected count + Cancel + Install selected are visible. Four candidates are manageable; a real multi-skill source is not served by the absence of search. All four start checked. The final action does not state the number or Tool scope being installed, and neither the chosen repository nor destinations remain in the visible header/footer. D8’s searchable picker and sticky install summary directly address this.

**Local picker.** The picker usefully distinguishes valid candidates from `Missing name in frontmatter` and `Missing SKILL.md`. This is a strength, not noise to delete. It nevertheless shares the Git picker’s “import” terminology and missing destination summary. During recovery from a deliberately invalid path, the old persistent error remained on top after Browse produced valid candidates. Keep invalid rows explainable, make the field error local to the field, and let successful correction retire obsolete feedback without erasing history.

**Projects and assignment matrix.** A skills × Tools matrix is the right representation. The incumbent makes Alex decode it: raw Tool keys, disabled global-covered cells with tiny globes, checked cells with different background colours, and per-row “All Tools” actions without a global legend. There is no quick narrowing to assigned skills or exceptions. At 960px the project name becomes `mon...` and the path `/User...` while the toolbar keeps its width. Project selection is also absent from normal keyboard focus order. This surface needs behavioural work, not just the new palette.

**Explore catalog.** It does provide name, repository, description, popularity, installedness, preview and hiding; it is not an empty marketing gallery. Its main visual question is nevertheless “which Install button should I press?” rather than “which skill is worth inspecting?” Installed badges are conspicuously brighter than much of the content, Hide is repeated everywhere, and broad cards spread the comparison fields apart. The explanatory line mixes featured-source policy, online search behaviour and the minimum query length. In the decided world, use compact comparison, clear search mode, legible Provenance, and preview-first inspection with one clear install decision for the focused candidate. Do not invent quality/trust ratings from popularity.

**Explore detail.** Sharing the Markdown reader with managed detail is sensible, but action semantics are not shared correctly. `docx` is “Installed” in the catalog and “Install” in its preview. In this failure fixture its central copy is missing; the interface does not explain whether Install is intended as repair, duplicate installation, or ordinary acquisition. The preview also changes the timestamp to “just now”, which reads like freshness unless labelled. Preserve catalog query and place on return; distinguish catalog metadata, preview acquisition time, and managed status.

**Settings.** This is one of the calmer incumbent surfaces: a single column, explicit units, sensible controls, and helpful explanations of cache behaviour and keychain storage. The main shortcomings are light-theme contrast, an unnamed header gear, and organisation by accumulated implementation details rather than the operator’s immediate settings question. Appearance, storage, two cache intervals, token, app updates and diagnostics form one long sequence; deployment and discovery policy live elsewhere. Group related settings locally, retain the plain explanatory voice, and disclose persistence consistently. No new sectioned sidebar is needed.

**Onboarding import.** The grouping recognises that several copies can describe one skill, and shows a folder versus a link. But first run calls 22 copies “skills” while presenting 12 name-groups; Conflict groups arrive checked with a variant selected, and the differing content is not visible. Bright amber Match and Found in badges compete with actual conflicts. This is a high-consequence review, not a generic checkbox inventory: name the chosen variant, name what happens to identical originals, state what remains untouched, and use compact counts of groups versus copies.

**Tool configuration.** The compact checklist is familiar and the selected-but-undetected Windsurf entry correctly remains present. Yet “Show detected tools only” is checked while Windsurf says “not detected”; the exception is correct policy but unexplained interface. Every detected Tool repeats a green “Installed” label, and discovery scope is a final checkbox below deployment selection. Explain “selected Tools stay visible”, distinguish deployment selection from scan scope, and show the effective summary on save. Do not drop selected-but-undetected Tools to make the filter look consistent.

**Notifications.** Typed failure copy and session history are real strengths. After Refresh, however, a long modal interleaves summary, skips, Edit conflicts, acquisition failures and Propagation failures. Only some entries have action buttons; others send Alex back to a card or Settings by prose. A persistent toast can cover the top-right of the panel whose purpose is to read that same error. Keep the history and Copy all, but make it the durable-in-session explanation of a concise outcome, not the only usable issue inventory.

**Overwrite ask.** This is well grounded: the skill, Tool and exact path are listed, and Keep existing is an intelligible alternative. The weakness is its environment: four `aria-modal=true` dialogs remain mounted through Add → candidates → progress → overwrite, with no `inert`/`aria-hidden` on the other dialogs in the observed DOM. The replacement action looks like ordinary primary installation. The surface must preserve exact pair-level consent while reducing the stack to one active decision and clearly explaining that existing differing content will be replaced.

**Delete confirmation.** It is appropriately modal, names the skill, uses danger styling, and explains removal from synced Tools plus deletion of the Hub copy. It does not explicitly name project assignments or reassure Alex that an independent local source folder is untouched. “Remove skill?” followed by “Yes, Delete” is also less precise than one stable action name. Keep the confirmation, add scope/counts where known, and promise attempted removal rather than implying that failures cannot leave a kept error row.

**Global unsync.** The global toolbar action executes immediately: one click in `rich` reported **407 tool directory deployments removed**, with no intervening dialog; the light history capture records the outcome. In `failures`, it reported 55 removed and one not removable. Batch reporting is good; the absence of a scope checkpoint for the broadest removal is not. This deserves stronger protection than an individual row action, not a neutral button beside ordinary configuration/search.

**Remove Project.** The dialog is clearer than Delete: it names the Project and distinguishes deployed artifacts from assignment records. Its wording repeats the artifact-removal consequence twice and never explicitly says the project’s source files are retained. The new-world dialog should say what is removed, what stays, and how kept errors will be shown, without adding ceremonial confirmation for safe navigation.

**Empty library.** “No managed skills yet” is accurate, but the full busy-library toolbar remains, including a global uninstall command, alongside a large otherwise blank surface. This makes the empty state feel like absence rather than an available next step. In the same shell, explain Add from GitHub/local and the existing Explore route; when discovered groups exist, make Review the first task. Do not add a home/doctor surface.

## Ranked findings

P0 = prevents task completion entirely; P1 = major difficulty/confusion or hard-gate failure; P2 = meaningful annoyance with a workaround; P3 = polish. No general P0 claim is made from this incomplete keyboard/fixture review.

### F01 · P1 · Global unsync has no scope checkpoint

- **What:** the neutral “Uninstall from tool directories” toolbar button immediately removed 407 deployments in `rich`. It is visually adjacent to Configure Tools and search. No confirmation appeared. This is not the requested hypothetical unsync dialog: the global route did not have one.
- **Why:** H5 error prevention, H3 control; UI/UX `confirmation-dialogs`, `destructive-nav-separation`, `error-recovery`. High impact is independent of whether the fixture operation can be repeated.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-light-unsync-history.png`. Live result: `Removed 407 tool directory deployments`; dialog count after action: 0.
- **Must do inside the world:** move global removal out of the everyday primary-action cluster; confirm the actual global scope, affected skill/Tool counts, what remains managed, and what happens under the current auto-sync policy. Retain per-target failure reporting and kept error rows. Do not invent an undo guarantee the backend does not provide. Preserve the fast single-target path only with legible state/action semantics.

### F02 · P1 · Healthy decoration outranks the library’s problems

- **What:** green Tool pills dominate each row; the first viewport has no library total or exception summary. The strongest banner is “Discovered skills”, not the condition of the managed library. Problems are distributed among tiny Edit marks, inline missing-path text lower down, coloured cells and notifications.
- **Why:** H1, H6, H8; Product principles 1 and 5; D2 healthy-is-silent; cognitive-load **single focus**, **visual hierarchy**, **working memory**.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-list-grouped.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-notifications.png`.
- **Must do:** lead with count + linked exception summary and Issues chip. Put exception marks **after** the skill name. Keep skill-level conditions (unlocatable source, fetch failure, Edit conflict) separate from Sync-target status. Use the agreed shape-encoded Synced strip, near-invisible at N/N, and a clear drill-down. `stale` remains target drift; do not fabricate “update available” or introduce the deferred upstream-check channel.

### F03 · P1 · The project selector is not a keyboard-selectable listbox

- **What:** observed listbox and all three `role=option` project rows have `tabIndex=-1`; the tab-reachable children are Configure Project and Remove Project buttons, not project selection. Assignments depend on first selecting a Project.
- **Why:** D5; UI/UX CRITICAL `keyboard-nav`, `voiceover-sr`, `focus-states`; H3 and H7. This is an inaccessible path, not a request for extra power-user shortcuts.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-matrix.png`. Browser DOM inspection returned listbox `-1`, options `skills-hub/-1`, `quartermaster/-1`, `agent-skills/-1`; their nested controls were only Configure/Remove.
- **Must do:** provide a real keyboard selection model—native focusable navigation or a correctly implemented roving listbox—with a distinct selected versus focused state and predictable entry into the matrix. Keep per-project overflow separate from selection. Prove selecting an initially unselected Project and changing an assignment without a pointer; this inspection alone is not that proof.

### F04 · P1 · At the minimum window, controls win over project identity

- **What:** at 960×640 the active project title becomes `mon...`, the path `/User...`, and “Last synced 5h ago” breaks into a narrow vertical column. Sort, grouping, Configure Tools, Sync Project and Sync All retain their horizontal footprint.
- **Why:** D5; UI/UX HIGH `content-priority`, `compact-label-overflow`, `visual-hierarchy`; H1 and H6. Losing the current Project’s identity next to a scope-changing action increases risk.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-matrix-960.png`.
- **Must do:** implement the agreed structural collapse. Preserve the current Project name and selected-scope context before secondary controls; overflow or regroup actions, retain a legible primary action, and give the matrix an intentional scroll region with stable labels. Do not solve this by shrinking type. The inspector-to-sheet breakpoint is already decided; the project header needs equivalent priority discipline.

### F05 · P1 · Light mode makes actionable explanatory text too faint

- **What:** settings helper text measured `rgb(161,161,170)` on white, **2.5629:1**. Import paths use the same measured foreground. These are 12px reading tasks, not disabled controls. Bright white grounds expose a hierarchy that was made by fading information instead of ordering it.
- **Why:** D5 WCAG AA; UI/UX CRITICAL `color-contrast`; craft floor ≥4.5:1 normal text. The disabled-control exception cannot excuse explanatory text or selectable variant paths.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-light-settings.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/first-run-light-import.png`.
- **Must do:** use a tested light-theme secondary text token meeting AA on each actual surface. Differentiate metadata through placement/size/weight, not unreadability. Test error, amber, selected and placeholder pairs separately; do not assume that inverting the dark tokens works. No claim that every dark pair was measured is made here.

### F06 · P1 · Add culminates in four simultaneous modal semantics

- **What:** Add, Select skills to import, Installing Skills, and Folder already exists all remained `aria-modal=true`, with no `aria-hidden` or inert ancestor, while the overwrite decision held focus. The screenshot shows visibly stacked panels and multiply dimmed background.
- **Why:** UI/UX CRITICAL `voiceover-sr`, `escape-routes`; HIGH `modal-vs-navigation`; H3/H4; cognitive-load **one thing at a time** and **working memory**. A visible top layer is not a complete accessible modal contract.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-light-overwrite.png`. DOM count: 4 dialogs; all `aria-modal=true`, all `aria-hidden=null`, all without inert ancestors.
- **Must do:** one active Add surface with source → candidates → settlement states and a sticky summary. If overwrite interrupts it, only that decision is interactive/exposed as modal; restore focus and state correctly when it resolves. Keep per-pair consent and Keep existing. A rewrite of acquisition/overwrite business rules is not required.

### F07 · P1 · Grid density is bought by clipping Tool identity

- **What:** Auto Grid and Dense Grid crop Tool pills at the right edge; Claude Code wraps into a 22px-high pill and is visibly cut. The operator cannot reliably read which Tool a control names. List has a `+2 more` disclosure, but the grid’s visible strip can end mid-label.
- **Why:** UI/UX HIGH `compact-label-overflow`, `chip-collection-reflow`, `state-clarity`; H1/H6. This is lost operational information, not a preference for one layout.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-grid-grouped.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-light-grid-grouped.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-grid-flat.png`.
- **Must do:** design the grid inside the accepted world rather than shrinking the old pills. Use the same stable, shape-encoded per-tool strip with accessible full Tool names and visible N/M, plus a keyboard-operable details disclosure. Ensure every mark/control fits; use the inspector for the full grid/list. Retain both list and grid modes.

### F08 · P1 · Onboarding asks for consequential choices without enough comparison

- **What:** a first-run Conflict group is selected and a source variant preselected, but the only differentiator visible is its path/Tool. Match badges and “Found in” badges are amber like exceptions. “Skills found: 22” counts copies while the decision is over 12 groups. The final action has no selected-group or destination summary.
- **Why:** H2/H5/H6; UI/UX `error-prevention` via `confirmation-dialogs`, `field-grouping`, `multi-step-progress`; Product principle 5. The operator must understand takeover of originals, not just choose a checkbox.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/first-run-dark-import.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/first-run-light-import.png`.
- **Must do:** distinguish name-groups from copies, make conflicts the exceptional rows, and reveal meaningful information about the selected variant before import. State what happens to byte-identical originals under the actual auto-sync policy and that divergent siblings remain. Keep a selected-group/Tool-scope summary next to Import. The precise comparison affordance and whether conflict groups require explicit acknowledgement belong in ticket 09, not an invented new backend diff feature.

### F09 · P1 · Detail drops the operational question at the moment Alex asks for more

- **What:** opening `local-notes` in failures produces a normal document reader and Change source action, but no “Source folder missing” explanation, no Sync-target accounting, no Edits/repair hierarchy. Rich detail likewise spends the main pane on files before state.
- **Why:** H1/H6/H9; cognitive-load **context switch** and **memory bridge**; the accepted inspector story explicitly says detail explains why and offers repair.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-light-unlocatable-detail.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-detail.png`. The preceding live row text explicitly included Source folder missing / Re-point / Detach / Remove.
- **Must do:** preserve row identity and selection in the split inspector/sheet. Start with skill-level and target-level conditions, then Health / Upstream / Edits / Last synced; offer the correct repair beside the condition. Keep copyable source and the Markdown/file reader as secondary inspection. Do not conflate readable central content with a refreshable, locatable source.

### F10 · P1 · The primary surface has no efficient multi-skill operating model

- **What:** row actions repeat Update / Change source / unsync / Remove; no selection model or bulk rail is present. Repo headers organise but do not deploy. List rows are roughly 150px high in the visible fixture, despite most of that width being empty. Flat mode removes grouping but does not produce an efficient tracker.
- **Why:** H7/H8; Product principles 2/3; cognitive-load **minimal choices** and **progressive disclosure**; D8 already reserves the remedy.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-list-flat.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-list-grouped.png`.
- **Must do:** one selection model across rows/cards, inspector, source group and bulk rail; deploy/assign/disable/delete in the accepted scope. Keep one primary Add action and secondary Refresh all; put infrequent per-skill operations in overflow. Use the agreed compact row scale, singleton-source fold and sticky source headers. This finding supports the accepted scope; it is not a request for tags, kits or a new overview page.

### F11 · P1 · Explore forgets installedness when entering preview

- **What:** `docx` is Installed in the Explore catalog and has a normal Install button in its preview. In this fixture the managed central copy is missing, but that fact is absent from the preview action. The operator cannot tell whether the button means repair or another install.
- **Why:** H1/H4/H5; UI/UX `state-clarity`, `back-stack-integrity`, `error-recovery`; D10 demands the same scrutiny as My Skills.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-explore.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-explore-detail.png`; light counterpart `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-light-explore-detail.png`.
- **Must do:** carry managed identity and condition into preview. If already managed, offer the managed inspector and the valid repair/action; if intentionally permitting reinstall, name it and its consequences. Maintain a consistent installed/not-managed contract across catalog, preview and settlement. Do not infer update availability from the preview timestamp.

### F12 · P2 · Raw Tool keys leak at three decision boundaries

- **What:** confirmed `Found in claude_code` / `gemini_cli` in Onboarding import; `CLAUDE_CODE`, `AGENTS_SKILLS`, `WINDSURF` matrix headers; raw `kimi_cli` / `agents_skills` inside notification recovery prose. Human names elsewhere make these look like different entities.
- **Why:** H2/H4/H6; UI/UX `nav-label-icon`, `icon-context`, `error-clarity`; canonical Tool vocabulary.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/first-run-light-import.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-matrix-960.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-notifications.png` (notification text below the visible fold confirmed by DOM).
- **Must do:** display Tool-catalog names in headings, selected-source labels, accessible names and recovery text; keep raw keys only in explicitly secondary diagnostics. Preserve the virtual group’s own identity and backend-owned grouping—do not reconstruct it from paths or substitute constituent names independently.

### F13 · P2 · Notifications explain exceptions but rarely take Alex to the repair

- **What:** Refresh leaves many distinct entries. Several literally instruct “Restore or remove it from its card”, “Run Update again”, or “Configure a GitHub Token in Settings”; only some have direct actions. A persistent error toast overlaps the Notifications panel’s header/close area. The history has Clear and Copy all, but no compact per-operation target map.
- **Why:** H9/H6; UI/UX CRITICAL `error-feedback`, `focus-not-obscured`; cognitive-load **working memory**. Do not force Alex to memorise a skill’s name while changing surfaces.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-notifications.png`.
- **Must do:** concise operation outcome linked to the affected skills/Sync targets; actionable rows open the correct inspector/Settings location without dropping history. Keep skips, acquisition failures, Edit conflicts and Propagation failures distinguishable. Suppress or reposition duplicate toasts while their report is open; maintain accessible announcements without stealing focus. No persistence expansion is proposed.

### F14 · P2 · Add and picker language hides the current step and final scope

- **What:** Git acquisition and local creation both open “Select skills to import”, although Onboarding import is a different domain operation. The Git entry says Install before discovering; local says Create. Pickers omit the chosen Tool scope, have no search, and show no selected-item count in the final action. A corrected local path can leave its old error covering the candidate picker.
- **Why:** H2/H4/H6; UI/UX `multi-step-progress`, `error-placement`, `error-recovery`, `redundant-entry`; D8 sticky summary and picker search.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-light-add.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-light-git-picker-960.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-light-local-picker-960.png`.
- **Must do:** reserve Import for Onboarding import; use a visible discover/select/install progression with a stable source + selected-candidates + Tool-scope summary. Implement D8 search and explicitly state how filtering affects installation of checked candidates. Keep invalid candidates disabled with readable reasons. Put path errors by the input, preserve the entered value, and retire resolved visible errors while retaining history.

### F15 · P2 · Matrix status is a colour puzzle, not a diagnosis

- **What:** checked cells can have red or amber grounds without a visible status word; the same table contains disabled unchecked cells with tiny globes. In the 960px capture, docx’s red checked cell and design-tokens’ amber checked cell are not self-explanatory. “All Tools” is an action label that does not name its verb.
- **Why:** UI/UX CRITICAL `color-not-only`, `icon-context`, `voiceover-sr`; H1/H6/H9. A check answers assignment, not health.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-matrix-960.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-matrix.png`.
- **Must do:** separate assignment from Sync status using shape/text and accessible names; explain global coverage without making “disabled” look like a failed control. Make stale/missing/failed details reachable by focus as well as pointer and offer the correct repair. Name bulk verbs (“Assign to all configured Tools”) and clarify selected-Project versus all-Projects scope. Never turn amber into an upstream-update signal.

### F16 · P2 · Configuration communicates policy incompletely

- **What:** Tool configuration says Show detected tools only but includes selected Windsurf marked not detected, without explaining the exception. Detection status repeats in green for every row. Settings itself is a long flat column and its header gear has no accessible name in the observed DOM.
- **Why:** H2/H4/H6; UI/UX CRITICAL `aria-labels`; HIGH `consistency`; domain rule that the Effective global target set preserves operator selection, including selected-but-undetected Tools.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-light-tools.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-light-settings.png`.
- **Must do:** explain that selected Tools stay visible, show a compact effective deployment summary, and visually distinguish deployment selection from Onboarding scan scope. Healthy installedness should recede. Name the Settings button. Group settings within the existing surface and make auto-save versus explicit save behaviour clear; do not introduce the rejected sectioned sidebar.

### F17 · P2 · Removal dialogs need an explicit boundary, not more warning chrome

- **What:** Delete names removal from Tools and the Hub copy but not Project assignments or preservation of an independent local source. Remove Project names artifact and assignment deletion but repeats the first consequence instead of saying that project source files remain. Both read more absolutely than a per-target operation that can keep an error row.
- **Why:** H5/H2/H9; UI/UX `confirmation-dialogs`, `error-clarity`; Artifact removal’s kept-row contract.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-delete.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-light-remove-project.png`.
- **Must do:** exact subject, affected scopes/counts, what is retained, and a specific destructive verb. After partial failure, show what remains and its repair; do not toast unconditional removal. Retain the existing restrained danger icon/colour and Cancel route. No new backend deletion semantics.

### F18 · P2 · Empty and discovery states spend attention in the wrong place

- **What:** an empty library retains sorting, grouping, view mode, auto-sync, global uninstall and Refresh, while its content says only “No managed skills yet”. A normal rich library gives the discovery banner the strongest continuous block of colour.
- **Why:** H8/H10; Operate empty-states guidance; cognitive-load **single focus** and **progressive disclosure**.
- **Evidence:** `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/empty-light-960.png`; `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-list-grouped.png`.
- **Must do:** retain the shell and one clear next step; explain GitHub/local Add and offer Explore as the secondary route. With discovered groups, prioritise Review without branding every Match as a warning. Once a library exists, discovery should not drown out actual issues. This is not a proposal for the deferred home/doctor page.

## Motion findings — Emil Before / After format

The direction contract’s ≤120ms limits and reduced-motion removal override generic duration suggestions. These observations use loaded CSSOM; a still image locates the affected surface but cannot prove timing. No frame-rate, interruptibility or reduced-motion playback claim is made.

| Before | After | Why / severity / evidence |
|---|---|---|
| `.skill-card`, `.tool-pill`, `.btn`, `.search-input`, `.explore-card` and several other controls expose `transition: 0.2s`—the shorthand defaults the property to **all**, with default easing. | Frequent row navigation/filtering resolves immediately. Name only necessary properties; use the agreed ≤120ms opacity/transform vocabulary for the permitted palette, bulk rail and switch interactions. Do not animate layout or add card choreography. | **P2**, Emil checklist `transition: all` and frequency-first decision. Broad transitions risk animating future geometry and make ordinary navigation feel embellished. Surface: My Skills/cards. Evidence: `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-dark-grid-flat.png`; browser CSSOM values recorded above in this description. |
| App-owned spinner/shimmer rules remain unconditional in inspected styles; the reduced-motion media rule found belongs to **Sonner**, not the app’s `.spinner`, `.progress-fill`, `.skeleton-row` or switch rules. | Honour reduced motion in app-owned feedback: static progress text/counts and no travel/shimmer; preserve an understandable busy state. Remove the agreed micro-transitions under reduce. | **P1 hard-gate risk**, D5 and UI/UX CRITICAL `reduced-motion`; Emil accessibility guidance. This is a stylesheet-coverage finding requiring emulated playback confirmation, not a claim that every animation was observed under reduce. Evidence surface: `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/rich-light-overwrite.png` (busy/install flow); loaded rules included `.progress-fill: shimmer 1.5s linear infinite`, `.spinner: spin .9s linear infinite`. |
| Loaded toast rule transitions `transform .4s, opacity .4s, height .4s, box-shadow .2s`; persistent error toasts also remain above an open Notifications panel. | Follow the world’s minimal feedback rhythm; avoid height animation in the app’s notification presentation and avoid duplicate moving layers while reading a report. Keep Sonner’s existing reduced-motion handling. Rare completion delight must be brief and must not gate interaction. | **P2**, Emil checklist duration >300ms, transform/opacity performance, and meaningful motion. Evidence: `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/critique-astra/failures-dark-notifications.png`. Do not blindly edit vendor CSS before testing the configured primitive. |

Do **not** add staggered entrances to the library or animate j/k, search, selection, or every keyboard action merely because a generic checklist recommends stagger. Operate frequency and the pinned direction win. The ≤120ms palette allowance is a ceiling, not an obligation to animate it.

## Cognitive load and Alex’s actual task

- **My Skills: high extraneous load.** Fails single focus, hierarchy, minimal choices and progressive disclosure. View preferences, policy changes, destructive global removal, search, refresh and discovery compete before Alex reaches the first skill. The task itself is not this complicated.
- **Add: moderate-to-high memory demand.** The Tool decision is made before candidate selection and then disappears beneath stacked modal surfaces. The four candidate rows are not themselves excessive; losing source/destination context is the problem.
- **Import: high decision load.** Counts describe copies; selection describes groups; a conflict asks Alex to choose a source whose content is not shown. Path recognition is being used as a substitute for evidence.
- **Projects: high decoding load.** Assignment, global coverage, sync mode and Sync status are implicit in checkbox/colour/globe combinations. A legend plus focusable diagnosis reduces necessary domain learning without removing meaningful detail.
- **Explore: moderate load.** Comparison information exists, but every card repeats Install/Preview/Hide with too little hierarchy. Popularity must stay a factual secondary field, not a surrogate recommendation.

For Alex’s short repeated sessions, the painful memory bridge is: **notice an exception → leave the current surface → find the same skill again → discover the repair**. The accepted inspector and action palette should remove that bridge. Keyboard support is not a speculative alternate persona; it is a stated product requirement.

## What to keep

1. **Source grouping, flat mode and grid choice.** These serve actual finding strategies. Fix singleton treatment and density; do not delete the operator’s grid preference.
2. **The real file tree and readable Markdown preview.** Alex can inspect instructions and references before acting. Keep it as a strong secondary inspector capability rather than replacing it with summary-only cards.
3. **Specific unlocatable repairs.** Source folder missing offers Re-point/Detach/Remove where valid; central missing offers Restore/Remove. Carry that precision into detail instead of collapsing everything to “broken”.
4. **Local candidate validity reasons.** Missing `SKILL.md` and invalid frontmatter are distinguished, invalid choices disabled. Keep those reasons legible and associated with the candidate.
5. **Overwrite names the exact skill → Tool → path, with Keep existing.** Preserve pair-level consent and the backend overwrite rule. The problem is layering, not the existence of consent.
6. **Typed, useful error copy.** Rate-limit reset time, a moved-source Change source action, permission paths and a kept-error removal outcome are far better than “Something went wrong”. Reveal diagnostics progressively; do not erase them.
7. **Session notification history and Copy all.** They make recovery and diagnosis possible after a transient toast. Keep history’s in-session scope honest.
8. **A restrained, predictable Settings form.** Explicit cache units, zero-value explanations, and the keychain assurance are worth preserving. Its calmness is a good baseline for the new world once contrast is fixed.
9. **The basic matrix representation.** Skills × configured Tools is the right model; give it readable labels, an assignment/status distinction and keyboard interaction rather than replacing it with another card wall.
10. **Existing modal Escape support.** Escape dismissed the first-run Import dialog in the live check. Do not claim a complete keyboard flow from that one successful test, but retain the behaviour.

## Cross-cutting themes

- **History is mistaken for current status.** Timestamps and past outcomes are available; the current condition and next action are not consistently co-located. A recent timestamp does not mean healthy, and a readable manifest does not mean the source is available.
- **Colour carries too much of the hierarchy.** Green installedness is louder than useful content; amber is used for both Match and conflict-adjacent information; light secondary text is faded below readable contrast. Use the decided semantic tokens, but fix information priority first.
- **Scope is lost across boundaries.** Global versus Project, managed central copy versus independent source, source selection versus final Tool targets: each is individually represented somewhere, but too often absent at the actual commit point.
- **The same entity has multiple languages.** Human Tool names coexist with registry keys; Add uses Import; All Tools lacks a verb. D2’s product language is a behaviour contract, not just a copy pass.
- **Density is currently a compression setting, not a designed structure.** List wastes vertical space; grid clips labels; the matrix shrinks identity. The answer is selective disclosure and stable columns/rows, not universally smaller text.
- **Failure is reported, but the repair is distributed.** Notifications know why; cards know actions; detail knows files. The inspector should join them while keeping skill-level conditions distinct from target conditions.
- **Shared primitives need behavioural acceptance tests.** Dialog isolation, accessible names, focus, contrast and responsive priority are not solved merely by adopting a component package. The detector’s empty result illustrates the gap.

## Open questions for ticket 09’s operator grill

1. **Global removal protection (F01/F17):** should library-wide unsync require **one exact scope/count confirmation**, or live **only in a clearly destructive Settings/overflow entry plus that confirmation**? I recommend the latter. Do not add repeated confirmation to every safe single-target toggle.
2. **Import conflicts (F08):** should a conflict group be **unchecked until explicitly chosen**, or remain **selected but require a compact comparison/acknowledgement before import**? Either is safer than the current selected-path-only review; choose based on Alex’s actual import frequency.
3. **Inspector Tool controls (F02/F09/F15):** should the default be **vertical per-tool status with inline repair**, with the click-to-sync grid behind a toggle, or **the grid immediately visible alongside a compact status explanation**? I recommend the vertical default until Alex explicitly prefers the denser grid.
4. **Explore’s main job (F11/F14):** should the focused-candidate primary action be **Preview, then an explicit install summary**, or **direct Install with the same scope summary revealed inline**? Preserve a fast route for a known skill; do not let speed erase installed/repair semantics.

These are behaviour decisions **within** the settled visual world. No re-grill of the palette, shell, metaphor, grid existence, rejected vocabulary, sectioned sidebar, Updates page, upstream check or home/doctor surface is proposed.

## Acceptance / handoff to the orchestrator

- Use F01–F11 to shape the high-risk round-19 work; F12–F18 should land with their owning surfaces, not as an unrelated polish backlog.
- Validate the unobserved scenario/theme combinations in the coverage ledger before claiming ticket 06 complete. In particular: dark local/Git picker, Tool configuration dark, light matrix, dark Remove Project, both-theme failure variants, and `latency=3` progress/cancellation.
- Complete real keyboard-only paths through project selection, Add, overwrite, import and removal; verify focus isolation of the dialog stack and reduced-motion playback. This report supplies concrete blockers, not a conformance certificate.
- Source semantics, Propagation, artifact removal, mutation serialisation and the typed error contract stay intact. Scope explanations must describe those contracts, not invent simplified behaviour.
- Tooling note: the context launcher reported an available Impeccable v4.4.0 update; it was not run. No project-context repair was performed.

Questions are captured above for **ticket 09**, rather than interrupting this delegated critique with a new operator interview.
