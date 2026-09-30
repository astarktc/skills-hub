# 12 — The inspector: ladder, per-tool list + grid toggle, repairs, Edit, project assignment, copyable source, Files, sheet collapse

Status: ready-for-agent
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
