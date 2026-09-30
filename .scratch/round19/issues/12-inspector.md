# 12 — The inspector: ladder, per-tool list + grid toggle, repairs, Edit, project assignment, copyable source, Files, sheet collapse

Status: implemented
Blocked by: 06, 09, 11
Spec: `.scratch/round19/spec.md` — D3, D15, D18, D22, D23. Sources: `critique-astra` F09, F03/Q3; `critique-opus` P1-2;
`flows.md` Flow 5; `audit.md` Detail rows. Absorbs D8's "detail health ladder + copyable source" and "per-row /
inspector project assignment".

## Work

- **Placement**: right third of My Skills for the focused row (surface state's `focused`); below the breakpoint
  (ticket 11's number) a **sheet** over the list; focus rules per D23/D21. `SkillDetailView.tsx` is replaced; its pure
  helpers (`buildTree`, `getLang`, `formatSize`) move to `src/lib/skillFiles.ts` with tests (arch candidate 12) and
  the file loading into `useSkillDocument(skillRef)` — fed a small document value, so Explore's preview (19b) can reuse
  it without forging a `ManagedSkill`.
- **Order of sections** (the operational question first):
  1. **Health** — every row of the fold with its repair inline: Restore (central missing) · Re-point / Detach (source
     missing) · Retry (fetch failed) · resolve Edit conflict (opens Edit with the plain choice "Keep yours (User only)"
     / "Use the skill's (User & model)") · finish Delete (D16's report dialog).
  2. **Targets** — vertical per-tool list (D22): mark · Tool label · sync mode (link/copy) · failure text in product
     words · inline **Retry failed** / **Overwrite and retry** (`TARGET_EXISTS`, through the ask) / **Disable** /
     **Enable**; the click-to-sync **grid** behind a toggle (persisted preference). A toggle-off shows "Removed from
     Pi · Undo" for ~5 s (undo = enable).
  3. **Upstream** — source line from the fold (label · href · **copy** button; local skills show the recorded folder,
     D15), provenance (imported: "Imported from Cursor · no upstream — Refresh skips it", D18), recorded ref/SHA,
     "Last updated"; **Update** and **Change source…** here.
  4. **Edits** — invocation mode state, conflict if any, Edit.
  5. **Projects** — assignments with status; **Assign to project…** (door verb `assignToProject`, project picker with
     Tool choice honouring the project's configured Tools; the matrix stays the per-project view).
  6. **Files** — tree + Markdown/code reader (`prose.css`), **expand** to full width in place and collapse back
     without losing the focused row; `list-style` restored in prose.
  7. Footer: **Remove** (danger, D16 dialog).
- **Re-point modal** (D15): pre-filled with the current source; on submit calls `preview_repoint_source` first and
  shows "replaces *X*'s bytes with **Y** (subpath @ ref)" with a name mismatch highlighted and an explicit Continue;
  multi-skill target opens the Add picker (ticket 16's picker component if landed, else the existing one); errors
  inline by the field, never a toast over the modal.
- **Detach** gets a consequence-stating confirm ("stops following ~/…; Skills Hub's copy becomes the source; Refresh
  will skip it"); the provenance line afterwards reads "Detached from ~/…" (if the backend lacks the field, record it
  in the Result as a follow-up for BACKLOG — do not invent it).
- Every control keyboard-reachable; the inspector is a landmark (`aria-labelledby` the skill name); Tab enters after
  the row.
- Migration rule: files join `migrated`; `detail-*` and `tree-*` rules leave `App.css`; ceiling lowered.

## Gate (D31)

Astra audit: keyboard transcript row → inspector → repair → back to row; sheet at 960×640 with focus rules; contrast;
`local-notes` shows its missing folder and Re-point pre-filled; `docx` shows Restore under Health and no filled marks;
Re-point to a mismatched name requires Continue. `npm run check` green.

## Result

Implemented 2026-09-30 (child, Opus 5.5). Nothing committed. Evidence: `.scratch/round19/evidence/12/` (gitignored).

### Files

- **New** — `components/skills/inspector/`: `SkillInspector.tsx` (aside ≥ 1200 / Dialog sheet < 1200, header, sections,
  footer), `InspectorSection.tsx`, `HealthSection.tsx`, `TargetsSection.tsx`, `SourceSection.tsx`, `EditsSection.tsx`,
  `ProjectsSection.tsx`, `FilesSection.tsx`, `useInspectorTabOrder.ts`. `components/skills/`: `SkillFiles.tsx` (tree +
  reader), `StripMarkIcon.tsx` (the strip glyph, shared by row and inspector), `ExplorePreviewView.tsx` (Explore's
  preview on the same document hook). `modals/`: `RepointModal.tsx`, `DetachModal.tsx`, `AssignProjectModal.tsx`.
  `lib/skillFiles.ts` (+ test: `buildTree`, `getLang`, `formatSize`, `isMarkdown`, `defaultFile`, `ancestorDirs`),
  `hooks/useSkillDocument.ts` (+ test; fed `{ root, version }`, no `ManagedSkill`), `hooks/useMediaQuery.ts` (+ test;
  `SHELL_WIDE_QUERY` = 75rem), `ui/layer-stack.test.ts`.
- **Deleted** — `SkillDetailView.tsx`, `modals/ChangeSourceModal.tsx`.
- **Changed** — `App.tsx` (layout: header + filters span My Skills, the inspector sits beside the list; focus rules;
  the four skill dialogs), `SkillsList.tsx`/`SkillRow.tsx` (focused row: `aria-current`, `raised` + 1 px inset ring;
  `data-skill-list`), `SyncedStrip.tsx` (glyph → `StripMarkIcon`, `stripMarkName` → `skillPresentation.ts`),
  `InvocationModeModal.tsx` (stable shell; conflict leads with the plain choice), `SettingsPage.tsx` (its header left
  the `detail-*` classes for utilities + Button), `useSkillActions.ts` (Detach confirms; `assignToProject` without
  `projectId` opens the picker; `restoreFocus` survives the progress overlay and works inside the sheet),
  `useSkillLibrary.ts` (`previewRepoint` as a cancellable `read: true` action answering a value; Detach / Assign
  dialog subjects), `useSurfaceState.ts` (`detachSkill`, `assignProject` kinds), `useProjectState.ts`
  (`readSkillPlacement`), `actionLifecycle.ts` (health target prefers `data-inspector-health`), `preferences.ts`
  (`skills-inspector-targetsView`, pinned in `preferences.test.ts`), `ui/dialog.tsx` + `ui/layer-stack.ts` +
  `ui/focus-fallback.ts` (sheet placement; focus-return fix, below), `i18n/resources.ts` (`inspector.*`, `repoint.*`,
  `detach.*`, `assignProject.*`, `invocationConflict.*` appended at the end of `en`), `fixtures/backend.ts` (selection
  parity), `DESIGN.md` (Dialog placement + focus-return notes, § The inspector, Layout), `App.css` (−240 lines: every
  `detail-*`, `tree-*`, `file-*` rule, `.change-source-kind`, `.settings-page > .detail-header`),
  `scripts/styles-baseline.json` (14 files appended, ceiling 2316 → 2076).
- **Rust** — `core/repoint.rs` (`RepointTarget::Git { url, selection? }`, `RepointSelection { subpath, resolution }`;
  preview and Re-point acquire a selection as `SkillIntent::Selection`), `core/skill_update.rs`
  (`acquire_git_repoint(.., selection)`), tests (`core/tests/repoint.rs` +2: preview of a picked candidate on a
  default-branch repo, Re-point to it records url + subpath and lands its bytes; literals updated in
  `commands/tests/mutation_results.rs`), `src/bindings/index.ts` regenerated (`selection?: RepointSelection | null`).

### Per section (all verbs through `actions.run` / `actions.repair`; rows and source line only from the fold)

1. **Health** — every `skillHealth` row: icon + label + what happened (typed error in words, Tools, when) + its repairs
   inline (Restore · Re-point · Detach · Retry update · Review invocation · Finish removing · Retry · Enable · Deploy ·
   Finish disabling · Remove). Retry covers failed first-sync `error` rows (ticket 02; a first sync, `TARGET_EXISTS`
   → the one ask). `data-inspector-health` = Restore's D21 target.
2. **Sync targets** — list (default): glyph · Tool · link/copy · state in words · Retry (`outline`) / Disable /
   Enable / Sync; the first action is one element across role changes so focus stays (Disable → Undo → Enable).
   "Removed from {Tool} · Undo" 5 s (`UNDO_WINDOW_MS`), Undo = pair Enable. Grid behind a persisted Segmented toggle.
   No per-Tool action for a skill off everywhere or with its central copy gone (Health carries it).
3. **Source** — fold's source line (link, Copy; a local skill's recorded folder, D15) · provenance ("Imported from
   Cursor · no upstream — Refresh skips it", D18; "The folder is gone — …" in `stale` for `source_missing`) · Last
   updated · Update / Change source… / Detach.
4. **Edits** — invocation mode, the conflict line, Edit / Resolve…; the Edit dialog leads with **Keep yours (User
   only)** / **Use the skill's (Model only)**. Fixture consistency (round-18 comment): the override's `base_mode` is
   the upstream after a replay (Rust `skill_edits::replay_unlocked` re-bases it), and `api-style`'s stored error says
   `upstreamMode: model-only` — Health, Edits and the dialog all read **Model only**; `design-tokens`' comment already
   records the Refresh path agreeing. No fixture change was needed.
5. **Projects** — each project with its assigned Tools' statuses and Remove (`unassignFromProject`); **Assign to
   project…** = `run("assignToProject", [skill])` → picker (every project with its configured Tools; "already on every
   Tool" / "adds it to Pi" / "no Tools configured") → `run(.., { projectId })` → the project world's
   `assignToProject` (all configured Tools; the collision ask as before).
6. **Files** — `useSkillDocument` + `SkillFiles`: opens on `SKILL.md`, tree above a 28 rem reader; **Expand** covers
   My Skills in place (list mounted + `inert`, scroll and focused row kept), Escape collapses first; lists render
   with markers (`.prose ul/ol` already set `list-style`; verified `disc`/`decimal` rendered).
7. **Footer** — Remove skill… (`outline danger`, the D16 dialog through the door).

### Decisions

- **Re-point multi-candidate: Rust extension, not URL composition.** The preview's `ref` **is null** for the
  multi-skill case (`preview_git` answers `git_ref: None` with the candidates), and a candidate's own
  `resolution.branch` is null on a default-branch repo — no `/tree/<ref>/<subpath>` URL can name it. Per 06's
  fallback, `RepointTarget::Git` gained `#[serde(default)] selection: Option<RepointSelection>` (the Add batch's
  `GitSelection` shape); the stored record is url + resolved subpath, exactly as Add records. Two Rust tests.
- **The picker for Re-point's candidates is a step inside the Re-point dialog** (radio rows), not the Add world's
  `GitPickModal`: that modal is a multi-select install surface bound to `useAddSkillFlow`'s state. Handoff to 16.
- **Mismatch needs Continue**: Check source always previews first; the summary takes focus (not the button) and the
  confirm reads "Continue with {name}". Refusals render as `FieldError` under the field.
- **Assign to project… assigns to all of the project's configured Tools** (`bulk_assign_skill`); per-Tool choice stays
  in the matrix (D26) — the picker shows each project's Tools and what assigning adds. Handoff to 17.
- **"Overwrite and retry"** is not a separate label: `SkillTargetDto` carries no `last_error`, so a row cannot know it
  is `TARGET_EXISTS`; its Retry raises the one overwrite ask when a folder is in the way (hint text says so).
- **Tab enters after the row** (`useInspectorTabOrder`): opening from a row keeps focus on it; Tab from the row's last
  control → inspector heading; Tab from the inspector's last control → what followed the row; Shift+Tab mirrors.
- **Primitive fix — dialogs now return focus to their opener.** In Chrome `DialogContent` decided the return target
  while its own `inert` was still on the page, so every opener failed `closest("[inert]")` and focus went to the
  fallback (usually `<main>`, whose first tabbable Base UI then focused). `layer-stack.staysInertAfterClose` ignores
  the closing layer's own inert; a layer opened with focus nowhere returns nowhere; the landmark is focused itself.
  Skill dialogs keep one mounted shell (Re-point, Invocation, Detach, Assign) — a conditionally mounted dialog never
  returns focus. Unit test in `layer-stack.test.ts`; the jsdom dialog tests could not reproduce the Chrome timing.
- **Sheet vs the progress overlay**: a streamed action's overlay (ticket 14 retires it) sits above the sheet; the
  door's `restoreFocus` waits for it and holds the D21 target against its late focus return.

### Deferrals / follow-ups for BACKLOG (not invented)

- **"Detached from ~/…"**: `detach_from_source` clears `source_ref` and `imported_from_tool`; nothing records the
  former folder, so after Detach the Source section reads "Managed here · No upstream — Refresh skips it". Needs a
  stored `detached_from` (+ DTO field).
- **Recorded ref / SHA**: `skills.source_subpath` and `source_revision` exist in the store but not on
  `ManagedSkillDto`; the Source section shows the recorded URL/folder and "Last updated" only.
- **Target `last_error`**: `SkillTargetDto` has no typed error, so target rows speak the strip's product words and
  cannot distinguish `TARGET_EXISTS` for an "Overwrite and retry" label.
- Unused i18n keys left for a sweep: `invocationEdit.{overridden,overrideTooltip,conflictTooltip}`,
  `disabledSkips.deployDisabledPairs`, `detail.{files,readError,fileCount}`.

### Verification (fresh, fixture 5175, headless `agent-browser`, trusted keys)

- **Keyboard transcripts** `kt-{dark,light}-{1280x800,960x640}.txt` (script `r19-12-run.sh` + `r19-12-active.js`
  alongside): 36 / 29 stops per run, **0 on BODY, every stop `solid 2px`**. 1280: `/` → type docx → Tab ×9 → docx name
  → Enter (focus stays, `#/skills/skill-004-docx`) → Tab switch → overflow → **inspector h2** → Close → Restore → Enter
  (overlay) → **Health section** → Shift+Tab ×3 → the row's overflow → Escape → **docx name**. 960: Enter → **sheet
  h2** → … Restore → Health → Escape → docx name. Re-point: pre-filled `https://github.com/mattpocock/skills` →
  grill-me URL → Enter → focus on the summary "Replaces deprecated-helper's bytes with grill-me
  (skills/engineering/grill-me @ main)", "Different name", **Continue with grill-me** → Escape → Re-point button.
- Also driven: resize 1280↔960 with focus in the inspector → the new place's heading both ways; deep link arrival →
  heading (side and sheet); local-notes Re-point opens on Local with `/Users/alex/Projects/notes-skill` and shows
  `SOURCE_PATH_MISSING` under the field; multi-skill `total-typescript/skills` → pick step → ts-reset → mismatch;
  Continue → overlay → focus to the row (Re-point button gone, D21 chain); Disable → Undo focused → 5 s → Enable
  focused; Detach confirm → Cancel returns to Detach, Detach → row; Assign picker → Windsurf added, focus back on
  Assign to project…; Files Expand/Collapse keeps focus on the toggle, Escape collapses then closes; Explore preview.
- **docx**: Health "Central copy missing" + Restore/Remove; the Targets list shows 7 × "Broken" + Windsurf not
  synced, **no filled mark, no per-Tool action** (`docx-inspector-*.png`).
- **Contrast** (`contrast-*.txt`, rendered + composited, every text node in the inspector / sheet / Re-point dialog;
  docx, local-notes, api-style, deprecated-helper, mismatch × 2 themes × 2 sizes = 20 runs): **0 pairs < 4.5**;
  lowest 4.68 (white on `primary`, dark — the known token floor), then 5.22 (`faint` on `card`). Non-text: focused-row
  ring = `ring` on `raised` (6.01 / 5.04); glyphs = the strip table on `card`. No new colour pairing.
- Screenshots: `{docx,local-notes,api-style,deprecated-helper}-inspector-{dark,light}-{1280x800,960x640}.png`,
  `repoint-mismatch-*`, `repoint-pick-dark-1280.png`, `targets-{grid,undo}-dark-1280.png`,
  `files-expanded-dark-1280.png`, `projects-dark-1280.png`, `assign-picker-dark-1280.png`,
  `detach-confirm-dark-1280.png`, `edits-conflict-dark-1280.png`, `explore-preview-dark-1280.png`.
- Not run here: axe, reduced-motion sweep (nothing new moves; spinners are `motion-safe:`) — the Astra half.
- **Gate**: `npm run version:check` → Version OK (1.2.18); `npm run check` → exit 0: lint clean; styles:check "App.css
  2076 counted lines (ceiling), 21 migrated file(s)"; vitest 38 files / 801 tests; build ok; rustfmt, clippy clean;
  cargo test 794 passed.

## Comments

- 2026-09-27 — round-18 extract: the fixture's invocation-conflict data differs between the card ("upstream … User & model") and the Refresh report ("Model only"; `round18/review/flows.md` § Fixture gaps) — make it consistent when the Edits section renders the conflict.
- 2026-09-29 — from ticket 02: the Health section's Retry must cover failed first-sync `error` rows (`synced_at` NULL — they own nothing at the path; a retry is a first sync, so `TARGET_EXISTS` can occur and routes to the ask).
- 2026-09-29 — from ticket 06: `commands.previewRepointSource(skillId, target)` answers `{ resolved_name, ref, subpath, name_matches, candidates | null }` and writes nothing. Open decision here: `RepointTarget::Git` takes only a URL, so a candidate picked from a repo on its *default* branch (no branch name known) has no URL to send back — either compose a URL with the resolved branch (`…/tree/<ref>/<subpath>`, using the preview's `ref`) or extend the git target with `subpath`/`resolution`. Recommended: compose from the preview's `ref` + candidate `subpath` (no wire change); fall back to extending the target only if the preview's `ref` can be null for a resolved candidate.

- 2026-09-30 — implementation child: see § Result. For the D31 audit: (1) the Dialog primitive's focus return was
  broken in Chrome for every dialog (opener judged while its own `inert` was on) — fixed in `ui/dialog.tsx` /
  `ui/layer-stack.ts`; please re-verify ticket 11's flows that relied on the door's restore (overwrite ask, switch)
  and the Delete dialog's next-row fallback at default latency. (2) The startup progress overlay + a deep-linked sheet
  and the overwrite ask above the sheet were driven headless only. (3) `agent-browser` tabs went `hidden` mid-session
  once (timers throttled, the fixture's overlay looked stuck) — a fresh session fixed it; not an app defect.

- 2026-09-30 — independent D31 auditor: **fix-then-pass; not accepted yet**. Full report `.scratch/round19/review/ticket-12-audit.md`, evidence `.scratch/round19/evidence/12-audit/`. **P1 I1–I5:** successful Detach/Re-point/conflict Edit from the sheet leave focus on BODY; narrowing with Re-point open puts the sheet above its child modal; at 1200 with inspector open scratchpad's name is 0px wide; a final Re-point command rejection appears as a toast over the still-open modal instead of inline; successful assignment triggers four all-project view reads despite applyView. **P2 I6–I9:** sheet Escape closes expanded Files instead of collapsing; grid Disable lacks Removed/Undo; Restore's Health ring is clipped; target-error/ref-SHA/detached-origin metadata require explicit wire-scope dispositions. Ticket 11's overwrite Escape/Keep/Overwrite paths pass at default latency; axe is zero violations in eight theme/size/scenario runs, measured contrast passes, reduced motion (including pending) has zero active duration, bindings regenerate byte-identically. Fresh version+check exit 0: 38 frontend files / 801 tests, 794 Rust tests; cargo --all also passes. The report gives exact fixes and re-check scope, including the ticket-14 overlay-removal note. No application source changed.

- 2026-09-30 — fix child (I1–I8 of the D31 audit; I9 is BACKLOG #65, untouched; nothing committed). Evidence
  `.scratch/round19/evidence/12-fix/` (gitignored; drivers beside their JSON; headless `agent-browser`, trusted keys, no
  `.focus()` in primary transcripts, default latency).
  - **I1** — root cause: the Dialog primitive validated only the opener, not `fallbackFocus`'s answer, so the row behind
    a sheet that stays open was returned (inert → BODY); and a dialog closing *beneath* the progress overlay its own
    action raised aimed focus into an inert page. Fix: every return target passes the closing layer's `usable` test
    (`FocusFallback = (usable) => …`); a layer closing beneath a layer it opened moves no focus and hands its return
    chain to it (`layer-stack.handOverReturn` / `returnChains`); the skill dialogs' D21 chain is row → inspector heading
    → surface heading (`actionLifecycle.dialogFallback`, new `inspector` target, `resolveFirst(…, usable)`); the
    Invocation conflict buttons stay focusable while pending. Files: `ui/dialog.tsx`, `ui/layer-stack.ts`,
    `ui/confirm-dialog.tsx`, `lib/actionLifecycle.ts`, `App.tsx`, `modals/{InvocationMode,Detach,Delete,Repoint}Modal.tsx`,
    `projects/RemoveProjectModal.tsx`. Evidence `i1-{dark,light}-{960,1280}.json` + png: 12 flows × 4 samples
    (0/300/2000/5000 ms), **0 BODY**; 960 → the sheet's heading, Tab → Close details; 1280 → the row, Tab → its switch.
    (Re-point's settle is a partial one — the fold reports Amp failed — so partial settlement is covered.)
  - **I2** — root cause: placement followed the media query at once, remounting the inspector (a new sheet layer
    registered above the open child) and scheduled focus work was never disposed. Fix: App latches the placement
    while any surface dialog or the progress overlay is open, and applies it one task after the last one closes
    (after its focus return); the arrival/row-return rAF chains and the reparent timer are disposed on
    subject/placement/area change. Files: `App.tsx`. Evidence `i2-{repoint,assign,edit,ask}.json`,
    `i2-ask-960to1280.json` + png: both directions, and back again, with Re-point / Assign / Edit / the overwrite ask
    open: the child stays top, keeps focus, the sheet stays inert beneath it; Escape closes the child, then the new
    placement's heading takes focus.
  - **I3** — root cause: exception badges never shrink; with the inspector beside it the list (640 px at 1200) left the
    name 0 px. Fix: in a list narrower than 48 rem (where the description already collapses) the marks compact to
    their icons (neutral gains ⊖), the words kept as sr-only text + tooltip, the marks wrapper positioned (A4). Files:
    `SkillRow.tsx`, `DESIGN.md`. Evidence `i3.json` + png: scratchpad 69.781 px, scrollWidth 70 = clientWidth 70, no
    truncated name, no document overflow at 1200 / 1280 / 960 × dark/light with the inspector open; ticket 11's
    full-width 960 number unchanged (69.781).
  - **I4** — root cause: the door ran the final `repoint_skill_source` through `runAction`, whose catch toasts. Fix:
    `run` is typed per verb (`SkillVerbAnswer`): Re-point answers its final command's whole-command refusal (still one
    `runAction`, reload per `RELOAD_ON_THROW`, a cancel stays silent, report failures stay the fold's); the modal shows
    it with `describeCommandError` in a `FieldError` under the summary, target kept, Continue `aria-describedby` it;
    `CommandError` now comes through the skills shim. Files: `useSkillActions.ts` (+ test), `RepointModal.tsx`,
    `skills/types.ts`, `App.tsx`. Evidence `i4-i5.json`, `i4-repoint-final-error-*.png` (auditor's one-shot injection):
    inline "Cannot access this repository…", 0 toasts, focus on Continue; a retry then settles.
  - **I5** — root cause: `readSkillPlacement` was a read whose identity changed with every applied view. Fix: the project
    world holds `placementViews` (explicit `loadPlacementViews`, kept current by `applyView`; a view applied during a
    read outranks it) and `placementOf(skillId)` (pure `skillPlacement`); inspector and picker consume the value.
    Files: `useProjectState.ts` (+ 6 hook tests incl. assign/unassign with **no** `getProjectView`),
    `ProjectsSection.tsx`, `AssignProjectModal.tsx`, `SkillInspector.tsx`, `App.tsx`. Evidence: recorder **0** reads
    after a successful Assign at 1280 and 960; positive control 4 reads on opening the picker (`i5-control.json`).
  - **I6** — `Dialog interceptEscape` (primitive, tested, DESIGN.md): the sheet spends Escape collapsing Files, focus to
    Expand; the side path uses the same `collapseFiles` (flushSync, no timer). Evidence `i678-i6.json` (960: Esc →
    collapsed + Expand focused, Esc → closed, row focused), `i6-1280-t3preview.json` (1280, see concerns).
  - **I7** — the grid shows the same "Removed from {Tool}" (persistent `role=status`) + Undo line; Undo from the grid
    starts from the Tool's tile so focus stays on it. File: `TargetsSection.tsx`. Evidence `i678-i7-{1280,960}.json`,
    `i7-list.json`: tile keeps focus, status + Undo shown, gone after 5.6 s, Undo → tile "Pi: synced — Disable", grid
    persists across reload; list Undo unchanged.
  - **I8** — a focusable `InspectorSection` draws `insetFocusRing`. Evidence `i678-i8.json` + png: outline −2 px offset,
    box inside the clipping scroller on all four sides, both themes/sizes.
  - Regression guards: ticket 11 ask Escape/Keep/Overwrite (dark 1280, light 960) → switch, root not inert, 0 dialogs,
    Tab → overflow (`ask-*.json`); Delete from the inspector → next row api-style both placements
    (`delete-unfiltered.json`); keyboard matrix `keyboard-{dark,light}-{1280,960}.json` 20/17 stops, 0 BODY, all
    `:focus-visible` solid 2 px, Restore → Health, Escape → docx row; axe 8/8 **0 violations** (sheet runs keep the
    known Base UI `aria-hidden-focus` incomplete); measured text/mark contrast 0 failures; reduced motion 0 running.
  - Gate (`gate.txt`): Version OK (1.2.18); styles:check 2076 (ceiling) / 21 migrated; vitest 38 files / 811 tests;
    build ok; rustfmt + clippy clean; cargo 794 passed; exit 0.
  - For Astra's re-check: the hand-over path (Re-point success/partial under the overlay, both placements); the
    placement latch (resize with each child open, then immediate second action); `placementViews` staleness after
    project-page mutations that answer no view (resync-all) — the inspector re-reads on mount; compact marks at
    list widths just under/over 48 rem. Open: agent-browser 0.31.1 replaces the page with `about:blank` on some later
    Escape presses in one session (not reproducible in the T3 preview, where the same sequence keeps the page), so
    I6's 1280 second Escape was driven there; the legacy Delete dialog's disabled Confirm still drops focus to BODY
    for the ~300 ms the delete runs (pre-existing; ticket 13's D16 dialog).

- 2026-09-30 — independent bounded re-check of the uncommitted fixes over `b53fd82`: **fix-then-pass, only residual I5 blocks**. Appended `.scratch/round19/review/ticket-12-audit.md` § Re-check; fresh evidence `.scratch/round19/evidence/12-recheck/`. I1–I4 and I6–I8 closed: 24 settlement samples with no BODY, correct sheet/row returns, both-direction child-dialog resize holds, scratchpad 70/70 scroll/client width, final Re-point refusal inline with zero toasts, Files Escape twice, grid Undo/expiry/persistence, four-sided Health ring. Assign/Unassign now have zero success-tail reads (picker opening explicitly reads four views), but a failed later read still hides last-good quartermaster placements: preserve those rows with stale/error indication and test good→failed→recovered plus mutation racing failed read. Nonblocking P2 R2: Re-point's generic answer promises null but its open/no-op branches return undefined; normalize or widen the type. Six overwrite regressions, four-condition keyboard matrix (74 stops, zero BODY), eight axe runs (zero violations), pending reduced-motion and byte-identical bindings pass. Fresh full version+check: 38 frontend files / 811 tests, 794 Rust, exit 0; targeted five files / 155 tests pass. Retain legacy Delete's pending BODY interval under ticket 13 and the existing ticket-14 overlay-removal note; I9 remains accepted BACKLOG #65. No application source changed, no commit.

- 2026-09-30 — **D31 gate passed** (orchestrator). Astra's re-check closed I1–I4 and I6–I8 and left R1 (a failed placement re-read hid last-good rows) and R2 (Re-point exits answering `undefined`); both fixed directly by the orchestrator — `PlacementViews.loaded`, last-good rows kept under a `inspector.projects.stale` note in the section and the picker, `null` on every Re-point exit — with hook tests; fresh gate exit 0 (vitest 814, cargo 794, App.css 2076 / 21 migrated). Carried forward: 13 owns the legacy Delete dialog's ~300 ms BODY focus while the delete runs (and its D16 replacement); 14 owns the overlay removal per the audit's concern-2 note (the layer-stack return-chain hand-over is what it relies on); BACKLOG #65 the wire metadata (I9).
