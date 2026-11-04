# Round 19 — the new world and the workflow features (UI effort phase B) → 1.3.0 (19a), 1.3.x (19b)

Status: open
Opened: 2026-09-27 (grill: `archive/round18/issues/09`, operator + orchestrator, pre-accepted for the session)

Round 18 judged; round 19 builds. This spec records the decisions of the ticket-09 grill over round 18's findings
(`archive/round18/review/{architecture,critique-astra,critique-opus,audit,flows}.md`) and turns them into tickets. It absorbs
BACKLOG **#39 #46 #47 #49 #52** and the §E borrows named in round-18 D8.

Baseline: 1.2.18 (`5218b12`, tag `v1.2.18`). Main @ `b77cd74`. Rounds 16 (ticket 08, Windows) and 17 (closure smoke)
stay live awaiting the operator; nothing here depends on them.

## Ground that is not re-opened

Round-18 D1–D12; the direction contract `.impeccable/surfaces/src-components-skills-skillslist-tsx.md` (shell, palette,
vocabulary, story, rejected list); `docs/adr/0006-styling-architecture.md` **as written, including Base UI under
shadcn** (grill Q1); the D8 exclusions (sectioned app sidebar, Updates page, home/doctor surface, #62 upstream check).

## Problem (what the four reviews agree on)

- **Healthy is loud, broken looks healthy.** A skill whose central copy is missing wears seven green pills; fetch
  failures, Edit conflicts and failed first syncs leave no durable mark; there is no exception summary. There is no
  skill-level health channel at all (`critique-opus` P0-2, `critique-astra` F02, `flows` X1).
- **Removal is hazardous and impermanent.** "Uninstall from tool directories" removes 407 targets with no
  confirmation; per-pill clicks undeploy silently; and every unsync is undone by the next Refresh's re-assert
  (`critique-*` P0-1/F01, `flows` X6/X7).
- **Detail is a file viewer, not an inspector**; there is no multi-skill operating model; Add ends under four stacked
  `aria-modal` dialogs; Tool keys leak; the matrix is colour-only (`critique-*` P1-2/F09, F10, F06, F12, F15).
- **Every D5 clause fails today** (`audit.md`): keyboard-only completion (Add's Tool checkboxes `display:none`, project
  listbox `tabIndex=-1`), focus escaping behind modals, light tertiary text 2.56:1, no `prefers-reduced-motion`, no
  structural collapse at 960×640.
- **The fold and the lifecycle misreport**: conflict outranks five failures, a cancelled Refresh reads "15 failed",
  "Keep existing" is an error toast, Cancel hides an overlay for work that keeps running (`flows` § fold judgement, X2).
- **The frontend has no home for any of this** (`architecture.md`): ~14 `useState` cells across five modules for
  "what is on screen", 11 per-skill callbacks threaded App → list → card, five hand-rolled Outcome unpackers, one
  global `loading`, two update-state machines, a second skill catalog in the project world.

## Decisions (grill, 2026-09-27; Q-numbers are the grill's)

### A · Architecture

- **D1 (Q1) Base UI under shadcn**, ADR-0006 §3 accepted. The palette ticket decides `cmdk` vs Base UI `Combobox`.
- **D2 (Q2) Primitives get role/ARIA tests** — colocated `src/components/ui/*.test.tsx`, `@testing-library/react`
  on jsdom, asserting only our wrapping's contract (role + accessible name, ARIA state from props, Esc closes,
  focus returns). No snapshots, no surfaces. AGENTS.md's "no component tests" rule stays for everything else.
- **D3 (Q3) Detail is an inspector beside the list**, a selection not a route. Files (tree + Markdown) are a section
  of the inspector that expands in place to full width and collapses back without losing the focused row.
- **D4 (Q4) Selection is per surface** (My Skills, matrix, Explore, pickers each own theirs); the palette and the
  bulk rail act on the showing surface's selection. All live in the surface-state module.
- **D5 (Q5) Surface state is hash-addressable for area + focused subject only** (`#/skills/<id>`, `#/projects/<id>`,
  `#/explore`, `#/settings/<pane>`); dialogs, selection and drafts never enter the hash.
- **D6 (Q29) Pre-paint theme.** A ten-line inline script in `index.html` reads the persisted theme key and sets
  `data-theme` before first paint. Documented exception to "storage only through `preferences.ts`"; the key literal
  is pinned by a test in `preferences.test.ts`.
- **D7 (Q32) Architecture candidates in scope**: 1 surface state, 2 skill action door, 3 health/source fold,
  4 tokens + primitives, 5 list lens, 6 action lifecycle, 7 project world composed in the binder (one skill catalog),
  8 one update module, 10 Add flow exposes intents, 11 Tool catalog split from sync orchestration, 12 file browser
  as a module. Candidate 13 (binder growth) is a symptom and gets no ticket. Candidate 9 lands as
  **`src/lib/platform.ts`** beside `lib/tauri.ts` (folder picker, progress Channel, updater handle, zoom), mocked
  the same way; the fixture supplies its second adapter.

### B · Policy

- **D8 (Q6, Q28) Disable is the only durable "not on this Tool".** Every operator-initiated removal of a Sync
  target records a **`disabled`** state at its grain — per (skill, Tool) or per skill: the artifact is removed, the
  row is **kept** with status `disabled`, Propagation and `reassert_auto_sync` report it `Skipped`; enabling
  re-syncs. Skill-level disable covers **global targets and project assignments** ("off everywhere until enabled");
  the per-Tool grain is global only. Deselecting a Tool in Configure Tools stays what it is — a change of the
  effective target set. Unrecorded unsync ceases to exist except as a consequence of Delete. The toolbar's
  "Uninstall from tool directories" is removed; library-wide undeploy is select-all → bulk rail → "Disable on all
  Tools…" with a count-bearing confirmation. The strip shows *disabled* distinct from *failed* and *not synced*.
- **D9 (Q7) Skill-level health persists on the skill row**: `last_outcome_kind` (`fetch_failed` · `edit_conflict` ·
  `delete_incomplete` · `repoint_failed` · …), `last_outcome_error` (the typed `CommandError`, JSON),
  `last_outcome_at`; written where the outcome settles, cleared by the next success of the same family. A **failed
  first sync records a target row with status `error`** (symmetric with ADR-0002). A **declined** target ("Keep
  existing") records as **`disabled`** for that pair under D8. `ManagedSkill` carries a `health` DTO; the frontend
  fold (D7 candidate 3) is the only reader. No run-log table.
- **D10 (Q8) Outcome kinds and the headline.** `declined` and `cancelled` are first-class outcome kinds, never
  failures. The single-headline precedence is replaced by a **composite count line** ("12 refreshed · 5 failed ·
  4 skipped · 1 Edit to review"); precedence survives only for the toast **tone** (failure › conflict › skipped ›
  success). Edit with target drift reads "saved · 2 targets failed". A cancelled Refresh returns rows of kind
  `Cancelled` (its own variant, not `Failed { Cancelled }`) and reads "Refresh cancelled — nothing was changed".
- **D11 (Q9, Q27) A run's report is the Notifications panel grouped by run**: the composite headline is the group
  row, items expand, and rows carry the skill action — "Show" (inspector focused on that skill), Retry,
  Overwrite…, Open Settings → GitHub for auth/rate-limit. Toasts carry only the headline + "View report". "What is
  wrong now" is #49's issues banner + Issues chip + the inspector ladder, all reading D9. History stays
  session-only.
- **D12 (Q10) One consolidated overwrite ask per operator action.** The picker's install becomes **one backend
  batch command** (the "one batch per operator action" rule finally holding for Add); its report collects every
  `TARGET_EXISTS` pair and the frontend raises **one** ask with per-row Keep/Overwrite and one retry batch with
  per-pair overrides. Refresh re-assert `TARGET_EXISTS` rows and Onboarding import's divergent siblings route into
  the same ask. The ask shows skill → Tool → path, the existing folder's modified time and "differs from the skill";
  **no content diff** in this round (a reader feature for #26).
- **D13 (Q11) Import with auto-sync off**: "Remove my byte-identical originals from *Cursor, Claude Code, …*" is a
  **per-run checkbox** in the review, defaulting to the current policy for the current mode; the button names what
  it will do ("Import and remove 9 originals" / "Import and sync"); the report lists removed originals per Tool.
- **D14 (Q12) Import review defaults.** Conflict groups and groups whose name is already managed start
  **unchecked**; a managed-name group offers "Replace this Tool's copy with the managed skill" (D12's ask) or
  "Leave"; conflict variants show description and modified time before the radio. The plan carries
  `name_is_managed`. Counts state name-groups and copies separately ("8 skills · 11 copies in 8 tools"). Invalid
  manifests are listed disabled with the reason in words (#46's `SKILL.md`-gated scan).
- **D15 (Q15) Re-point previews.** The target is resolved first and shown — "replaces *deprecated-helper*'s bytes with
  **ts-error-fixer** (`skills/engineering/ts-error-fixer` @ main)"; a name mismatch is highlighted and needs an
  explicit continue (never a refusal); multi-skill repos open the Add picker; the field is pre-filled with the
  current source; errors render inline by the field. Local skills show their recorded `source_ref`, copyable.
- **D16 (Q16) Delete confirms with scope, no soft delete.** The confirm lists the Tools, project assignments and
  Edits that go, and says "Skills Hub's copy is the only copy" for imported/detached skills. After a partial delete
  the same dialog becomes the report: removed from X · still in Y at *path* — Reveal in Finder · Retry · Close; the
  skill shows `delete_incomplete` health (D9).
- **D17 (Q17) A global target and a project assignment may coexist on one Tool.** The matrix cell is enabled and
  marked "also global"; bulk and toggle obey one rule.
- **D18 (Q30) Imported provenance lives in the inspector's Upstream step** ("Imported from Cursor · no upstream —
  Refresh skips it"); the row shows nothing, because imported is healthy.

### C · Action lifecycle

- **D19 (Q13) Non-blocking progress.** Streamed batches (Refresh, sync, import, install) show progress in the header
  ("Refreshing 12/80 · Cancel") with per-row pending marks; the library stays browsable and the inspector readable.
  For 1.3.0 the frontend keeps **one mutation at a time**: other mutations are disabled with the reason ("Refresh
  running"), not queued. The `LoadingOverlay` goes.
- **D20 (Q14) Cancel only where honoured**: the five token-taking commands plus the new install batch (between
  candidates, remaining candidates reported `cancelled`). Sync, import, delete and project operations show no Cancel.
- **D21 (Q26) Post-mutation focus is a rule of the lifecycle module and the Dialog primitive's `finalFocus`**, never
  a call site's: after Delete → next row (previous if last; list heading if empty); after Disable/Enable/Retry → the
  same control; after Restore → the inspector's Health row; after any dialog closes → its trigger, fallback chain
  trigger → focused row → surface heading. Focus never lands on `<body>`.

### D · Surfaces (inside the decided world)

- **D22 (Q24) The row strip is read-only**; clicking a row focuses it and opens the inspector. The inspector's
  per-tool section is a **vertical list by default** (mark · Tool label · sync mode · failure text in product words ·
  inline Retry / Overwrite and retry / Disable); the click-to-sync **grid** is behind a toggle (the contract's
  Unresolved item, resolved).
- **D23 (Q25) One breakpoint, three things move.** Below **~1200 px** (exact value measured in ticket 11 with the
  36 px row and the strip): the inspector becomes a sheet, the app sidebar becomes an icon rail, the Projects list
  becomes a header select. `tauri.conf.json` enforces `minWidth: 960, minHeight: 640`. If a resize reparents a
  focused inspector into a sheet, focus goes to the sheet's heading; if the sheet closes, to the focused row.
- **D24 (Q18) Settings gets panes** — General · Tools · Storage & cache · GitHub · Updates · Diagnostics — inside one
  Settings surface (not a second navigation). **Configure Tools moves into Settings → Tools**, still reachable from
  the My Skills header and ⌘K; scan scope is shown as a distinct thing from deployment selection.
- **D25 (Q19) Explore is tracker rows**, "Not in library" the default filter, **Preview** the primary action for the
  focused candidate, Install with an inline Tool-scope summary that stays visible; an installed candidate routes to
  the managed inspector (no second Install). Popularity stays a secondary fact.
- **D26 (Q20) The matrix remains the per-project surface**, showing every skill with **Assigned** and **Issues**
  chips; assignment from the inspector serves "put this skill in three projects".
- **D27 (Q21) Add is one surface with steps** — source → candidates → settlement — with a sticky summary (source ·
  selected candidates · Tool scope) through every step; D12's ask is the only nested layer; on partial failure the
  candidates step stays with installed rows ticked off and failed rows retryable. Vocabulary: the "Add" family;
  "Import" is reserved for Onboarding import. Picker search (#47) filters the list; only checked *and* visible
  candidates install, and the summary says so.
- **D28 (Q31) Target-size floor** owned by the primitives: 24×24 CSS px for dense controls (strip marks, matrix
  checkboxes with the hit area extended to the cell), 32×32 for standalone icon buttons and dialog Close.

### E · Process

- **D29 (Q22) Backend enablers are their own tickets** (01–06), scheduled in parallel with the world ticket; each
  keeps the fixture in parity (the fixture is a review input — `round18` handoff).
- **D30 (Q23) A 45-minute think-aloud** on the fixture (`failures` + `first-run`) with the operator during the
  world ticket's build, using `archive/round18/review/flows.md`'s per-flow questions; findings feed the surface tickets,
  the spec does not wait.
- **D31 (Q33) The a11y gate has two halves.** Every ticket that touches a surface ends with a **child-run fixture
  audit** in `preview_*`: an uninterrupted keyboard transcript of the ticket's flows, axe, measured contrast of the
  actual token pairs, both themes, 960×640 and 1280×800, reduced-motion CSS coverage — recorded under the ticket.
  The **operator** does one VoiceOver + `prefers-reduced-motion: reduce` + native-minimum-window pass on the
  release build over the six core flows as each release's closure smoke.
- **D32 (Q34) Two phases.** **19a → 1.3.0**: enablers ∥ world ticket → frontend modules → My Skills (list, inspector,
  selection + verbs, notifications + progress) → Emil pass → Add → Projects → release. **19b → 1.3.x**: Settings
  panes, Import review, Explore, ⌘K palette, card-grid view mode, `App.css` deleted. D9 (sequential on styling
  hotspots) holds inside each phase; the alias block guarantees no half-old palette between them.
- **D33 (Q35) Children on Pi**: implementation **Opus 5.5 @ high** (`anthropic/claude-opus-5-5`, `thinking: high`);
  adversarial review and audits **Astra @ high** (`openai-codex/gpt-6-astra`); mechanical single-surface class
  migrations **GPT-6 Luna @ high** (`openai-codex/gpt-6-luna`). `runtimeMode: full-access`. Reviewer ≠ implementer.

### 19b decisions (grill, 2026-11-02; Q-numbers are that grill's, two rounds)

- **D34 (Q1) Auto-deploy is arrival-only.** The setting decides where a **new** skill lands (Add, Onboarding import,
  Enable's first deploy) and nothing else. Update / Update all / Restore / Change source only propagate to deployments
  that exist; **the re-assert is deleted** (`refresh.rs::reassert_auto_sync_unlocked`, `RefreshPolicy`). "Not
  deployed" is a durable *never put here*; the difference from Disabled is crisp: Deploy / Deploy all / Deploy this fill
  a hollow pair and skip a disabled one; nothing fills either on its own. Add's per-skill harness scope is therefore
  truthful. Fact behind it: the operator's real library runs auto-deploy **off**; the re-assert only ever ran on the
  fixture. Closes BACKLOG #71 → ticket 33.
  **Amendment (ticket 33 audit F1, 2026-11-02):** a shared skills dir is one artifact, so a deployment into it is
  recorded for every *installed* sharer on every path that writes it — the global batch (already) and Propagation's
  first sync of a never-materialised row (made consistent by the fix). An Update may therefore add a sibling **row**
  for a Tool that shares the written folder (Kimi beside Amp); it never adds an **artifact** anywhere the skill was not.
  The strip then tells the truth for the sharer. The run reports the write once, on the asked pair.
- **D35 (Q2) A `delete_incomplete` skill is never updated**: every update path reports it skipped (typed reason) and the
  health row's Retry / Reveal is the only way forward. Ticket 33.
- **D36 (Q3) #78 becomes three tickets**: 30 (tokens and colour, both themes), 31 (My Skills row and group anatomy),
  and the settings-as-dialog part of 20. Order 32 → 33 ∥ … see D51.
- **D37 (Q4) Settings is a dialog**, not an area: a large modal on the layer stack with pane tabs on the left, opened from
  the sidebar item, Escape closes, focus returns there. Configure Tools becomes its Harnesses pane. `#/settings/<pane>`
  leaves the hash (dialogs never reach it). Keyboard shortcuts adopts the same frame. Ticket 20.
- **D38 (Q5) Row ground.** Each source group is a bordered rounded card: header on `card`, rows on the base
  `background`, hover lifts a row to `card`, the **focused** row (inspector) is a raised rounded block, **selected** rows
  keep the 3 px iris edge; no indent beyond the card inset. Ticket 31.
- **D39 (Q6) Name column** is `--name-col`, published by the list like `--strip-marks`: the longest visible name in `ch`,
  clamped to `[12rem, 22rem]` (≈ 45 characters; the operator's longest real name is 40). The 60 % floor goes; the
  description takes the rest; the list keeps no max width. Ticket 31.
- **D40 (Q7) The "Not deployed" row mark is dropped**; the kind stays for the inspector ladder, the group state summary and
  Deploy all's count. Ticket 31.
- **D41 (Q8) Projects on the wire.** `ManagedSkillDto.projects` (per project: id, name, per-harness deployment status);
  counts derived by the fold. Row scope cell: globe (a live global deployment) and/or folder + N (projects with ≥ 1 live
  deployment), tooltip naming them; N/M stays global; failed project deployments join the row's "N failed" mark.
  Ticket 31.
- **D42 (Q9) Groups collapse**: chevron, state persisted per source key in the list lens, an active search/filter
  expands matching groups for its duration, ←/→ on a focused header, Collapse all / Expand all in the view menu.
  **Grouping is decided on the unfiltered catalog** (the ≥ 2 rule never demotes a filtered singleton to "Individual
  sources" — BACKLOG #81 (4)). Ticket 31.
- **D43 (Q10, Q22) Colour.** State colours are semantic (failed red, out-of-date amber — new token — disabled muted;
  deployed stays a neutral filled mark, never green); the iris accent does not move. **Identity colour is decided by a
  prototype** (three treatments: 2-letter source chips · a coloured source edge on the group card · none), each with a
  harness-initials row over the strip column, both themes; the operator picks from screenshots and the pick is ticket
  30's spec. Light theme gets stronger ground steps. Ticket 30.
- **D44 (Q11) #80, #81, #82 ride with ticket 31** (same files).
- **D45 (Q12–Q16) Vocabulary.** **Harness** replaces Tool ("tool" is what the operator's users call a model-callable
  function; "agent" collides with sub-agents); **Deploy / Deployed / Deployment** replace Sync / Synced / Sync target
  (global or project; "Deploy to project" replaces Assign, "Re-deploy" replaces Resync, **Auto-deploy** replaces
  Auto-sync); **Update / Update all** replace Update / Refresh (all) ("App updates" keeps its name); states: Deployed ·
  Not deployed · Not deployed yet · Out of date copy · Copy missing · Failed · Disabled; **Library** / **Library folder**
  replace "Skills Storage Path"; Global and Project stay the scope words. Code identifiers, DB columns and error codes
  keep `Tool` / `sync` / `refresh`; the glossary records the code name per entry. Ticket 32.
- **D46 (Q17) Vocabulary is ticket 32 and runs before every surface ticket**, with a catalog test that fails on the
  retired words.
- **D47 (Q18) Settings expansion.** Ticket 20's Harnesses pane is per-harness expandable rows (detected or not, dir in
  use, deploy mode in use, remove from selection) so per-harness options have a home; nothing speculative ships. The
  AGENTS-standard-dir preference (BACKLOG #83) waits for the #67/#45 research (dispatched 2026-11-02).
- **D48 (Q19) Adding a harness** with auto-deploy on asks once — "Deploy your N skills to Cursor now?" — one batch,
  one overwrite ask; declined or auto-deploy off → nothing, Deploy all per group fills the hollow marks. Never
  automatic. Ticket 20.
- **D49 (Q20) Auto-deploy shows where it acts**: Add's summary, the Import review, the Harnesses pane — not My Skills.
- **D50 (Q21) Settings dialog shape**: max ≈ 880 × 640, capped at 90 vw / 85 vh, pane list left (General · Harnesses ·
  Appearance · Updates · Diagnostics), not resizable.
- **D51 (Q23) 19b order**: **33 ∥ research ∥ prototype → 32 → 30 → 31 → 20 ∥ 22 ∥ 23 → 21 → 24 → 25 → 26.**
  Amended 2026-11-02 evening: 33, 32, 23, 22 done; prototype pick **A** (D43 settled — header chip only, rows on a
  near-black ground); ticket **34** (registry truth, operator-approved) follows 20. Remaining: **30 → 31 → 20 ∥ 21 → 34
  → 24 → 25 → 26**.

## What the world ticket fixes in values (from the direction contract)

Ground `#0e0f12`, panel `#15171b`, raised `#1d2025`, lines `#262a31`/`#343942`, text `#e6e7ea`/`#a6abb4`/`#858b95`;
iris `#9690ff` (light `#5a4fdc`, fill `#6a61e8`) for selection, focus and the one primary action; ok `#3ddc84` ·
stale `#f5b400` · failed `#ff6b6b` as state only, healthy silent; light mode a derivation on a white-family ground;
system UI stack + `ui-monospace`; 12/13/14/16, tabular numerals; 4 px base; 36 px rows; radii 4/6/10; motion
≤120 ms opacity/transform on palette, bulk rail and switch only, removed under reduced motion. **Every light pair is
measured** (the audit's 2.56:1 tertiary is the reason light is "a derivation", not an inversion).

## Tickets

Order is dependency order; `Blocked by` is the edge list. Every surface ticket carries the D31 gate.

### 19a → 1.3.0

| # | Title | Blocked by | Kind |
|---|---|---|---|
| 01 | `disabled` target state, two grains; Propagation/re-assert skip; unsync commands become Disable (D8) | — | enabler |
| 02 | Skill-level health on the row; failed first sync records an `error` row; declined records `disabled` (D9) | 01 | enabler |
| 03 | Outcome kinds `declined` / `cancelled`; composite headline in the fold (D10) | 01 | enabler |
| 04 | One install batch command for Add; consolidated `TARGET_EXISTS` across Add / re-assert / import (D12, D20) | 01 | enabler |
| 05 | Onboarding plan `name_is_managed`; `remove_originals` per run; `SKILL.md`-gated scan (#46) (D13, D14) | — | enabler |
| 06 | Re-point preview command; `source_ref` for local skills on the wire (D15) | — | enabler |
| 07 | **World ticket**: `theme.css`, `prose.css`, primitives on Base UI, `styles:check`, `DESIGN.md`, pre-paint theme, min window (D1, D2, D6, D28) | — | world |
| 08 | Surface-state module + hash addressing + list lens (D3, D4, D5; arch 1, 5) | — | frontend module |
| 09 | Skill action door + action lifecycle + health/source fold (D7 arch 2, 3, 6; D19–D21) | 02 03 08 | frontend module |
| 10 | Tool catalog split, `platform.ts`, one update module, project world in the binder (arch 7, 8, 9, 11) | 08 | frontend module |
| 11 | **My Skills**: shell, header + exception summary, filter bar + Issues chip (#49), rows + strip, group headers + Deploy all (#39), singleton fold, empty state (D22, D23) | 02 07 08 10 | surface |
| 12 | **Inspector**: ladder, per-tool list + grid toggle, repairs (Retry, Overwrite and retry, Restore, Re-point preview, Detach confirm), Edit, project assignment, copyable source, Files section, sheet collapse (D3, D15, D18, D22, D23) | 06 09 11 | surface |
| 13 | **Selection, bulk rail and verbs**: per-surface selection, `x`/`Esc`, bulk Disable/Enable/Sync/Assign/Delete with count confirmations, Delete dialog → report, keyboard rows `j/k` (#52 bulk, D8, D16) | 01 09 11 | surface |
| 14 | **Notifications grouped by run + header progress + Cancel policy + focus rules** (D11, D19, D20, D21) | 03 09 12 13 | surface |
| 15 | **Emil pass** on primitives + My Skills (D6 motion, details) | 11 12 13 14 | review + fixes |
| 16 | **Add**: one stepped surface, sticky summary, picker search (#47), one ask, retryable partial failure (D12, D27) | 04 09 15 | surface |
| 17 | **Projects**: Tool labels, shape+word cell states + legend, keyboard listbox, header collapse, coexistence (D17), Sync All skips missing folders, Assigned/Issues chips, `reconciled:false` line (D23, D26) | 10 13 16 | surface |
| 18 | Operator think-aloud on the fixture (D30) | 11 | ready-for-human |
| 19 | Docs, CHANGELOG, AGENTS.md invariants, `version:set 1.3.0`, release; operator VoiceOver smoke (D31, D32) | 15 16 17 27 28 29 | release |
| 27 | A refused click says why: held-disabled controls show their reason when activated (from 18; added 2026-10-27) | — | pre-push fix — done 1c7d031 |
| 28 | Executable bits survive the GitHub Contents fast path (UNI-163; added by the operator 2026-10-27) | — | pre-push fix — done cd33e1e |
| 29 | A toast raised while a modal is open is announced: the toaster stays live under the layer stack (BACKLOG #79; operator "fix first" 2026-10-27) | — | pre-push fix — done 27c0862 |

### 19b → 1.3.x

| # | Title | Blocked by | Kind |
|---|---|---|---|
| 20 | **Settings panes**; Configure Tools → Settings → Tools; theme radio group; named header controls (D24) | 19 | surface |
| 21 | **Import review**: unchecked conflicts / managed names, Replace/Leave, remove-originals checkbox, counts, variant details, Tool labels, first-run empty state (D13, D14) | 05 19 | surface |
| 22 | **Explore rows**: Not-in-library filter, Preview primary, scope summary, installed → inspector, contrast + search label (D25) | 19 | surface — done a2a09bd + 37c8999 |
| 23 | **⌘K palette** on the surface module and the action door; `cmdk` vs Base UI Combobox decided here | 19 | surface — done 359bb3b + 13a677b |
| 24 | **Card-grid view mode** designed inside the world (same selection, strip, marks; no clipped labels) | 19 | surface |
| 25 | `App.css` deleted; guard flips to "must not exist"; `Layout.tsx` / `Dashboard.tsx` disposition | 20 21 22 23 24 | closure |
| 26 | CHANGELOG, `version:set 1.3.x`, release; operator smoke (D31) | 25 | release |
| 30 | **Colour and identity tokens**: state hues, identity treatment per the prototype's pick, light-theme ground steps, DESIGN.md contrast table (D43; BACKLOG #78) | 32, prototype pick | world — done 9ca0bc8 |
| 31 | **My Skills row and group anatomy**: card-per-group ground, `--name-col`, Not-deployed mark dropped, projects on the wire + scope cell, collapsible groups, unfiltered grouping, harness initials over the strip; riders #80 #81 #82 (D38–D42, D44) | 30 | surface + enabler |
| 35 | **Dark ground = T3's two-tone structure**: true black `background`/`row`, `card` #040404 pane, `popover`/`accent` #0e0e0e panel + hover (split from `raised` #1e1e1e), neutral greys, selection edge kept; every dark surface, DESIGN.md dark columns re-measured (prototype `30-dark-ground` pick d/f, operator 2026-11-04) | 30 | world — done 03065db |
| 32 | **Vocabulary**: Harness · Deploy · Update · state words · Library, across CONTEXT.md, `resources.ts`, README, DESIGN.md, fixture, tickets; retired-word catalog test (D45, D46) | 33 | docs + strings — done 60dc80e |
| 34 | **Registry truth pass**: rows corrected to vendor docs (Codex → AGENTS-standard group, Antigravity, Kimi (Amp hazard), Kilo, Xum, 4 new rows), `reads_agents_standard_globally` + `also_reads` facts, cross-read warning, the "prefer the AGENTS standard dir" preference in the Harnesses pane (BACKLOG #67 #45 #83, #40's detection half) | 20 | enabler + surface |
| 33 | **Auto-deploy is arrival-only**: re-assert deleted, `delete_incomplete` skipped as report data, policy DTO gone, first-deploy failure copy (D34, D35; BACKLOG #71) | — | enabler — done 948f12b + 23938db |
## Execution

- **19b order (D51)**: 33 ∥ research (#67/#45) ∥ colour prototype → 32 → 30 → **35** → 31 → 20 ∥ 22 ∥ 23 → 21 → 24 → 25 → 26 (35 inserted 2026-11-04: the dark-ground prototype's pick).
  Tickets 20–24 are read in the D45 vocabulary even where their text still says Tool / Sync / Refresh.
- Children per D33. Every brief: "state your approach and continue" (Workflow step 1 pre-approved); `.scratch/` is
  tracked; `docs/agents/issue-tracker.md` § Delegated children applies (edit tickets, write handoffs; never archive,
  `git mv` or commit under `.scratch/`); `runtimeMode: full-access`; prefer targeted edits; re-read after writing;
  prove with fresh command output; run `npm run version:check && npm run check` before reporting.
- **One working tree.** Enablers 01–06 and modules 08/10 are disjoint from world ticket 07 (`src/styles/`,
  `src/components/ui/`, `index.css`, `App.css` alias block, `scripts/`) and may run in parallel; `cargo test`
  regenerates `src/bindings/index.ts` and hot-reloads every fixture preview — a child that captures screenshots is
  told to expect it. The orchestrator runs the single fixture server on 5175.
- Every surface ticket ends with the D31 child audit (Astra) before it is marked implemented; a P0/P1 finding
  reopens the ticket.
- Reviews: Astra reviews 01–04 (they touch settlement, Propagation and removal), 07 (tokens + primitives, with the
  contrast table), 09 (lifecycle) and 16 (Add batch). Fix-then-ship with a bounded re-review.

## Closure checklist (19a)

- 1.3.0 published with five targets; operator smoke: the library opens in the new world in both themes; no skill
  wears a healthy mark; `docx`-class skills show their health; Disable survives Refresh; select-all → Disable on
  all Tools asks with a count; Add of a multi-skill repo with two collisions raises one ask; Projects selectable by
  keyboard; window refuses to shrink below 960×640; VoiceOver completes the six flows.
- `npm run styles:check` in `npm run check` and CI; `DESIGN.md` documents every primitive's variants.
- Tickets 01–19 terminal; `## Closure — <date>` block here; BACKLOG re-verified; 19b tickets remain live under this
  same effort.

## Closure — 2026-10-29 (19a)

**Shipped:** 1.3.0 — tag `v1.3.0` = `457085e`, GitHub Release published 2026-10-28 12:23 UTC (`release.yml` run
40592085719: macOS ×2, Windows ×2, Linux, `updater.json`), CI green on `main` (40586993850, 40592082973). The push
`700dc9e..b0e9d4b` carried round 19's 44 commits plus 27 (`1c7d031` + `bfbc1cb`), 28 (`cd33e1e` + `3905da3`) and 29
(`27c0862`). Operator smoke on the main Mac 2026-10-29: every item passed (7 by fixture proof; 8 on the work Mac pending —
ticket 19 § Smoke outcome). Tickets 01–19, 27, 28, 29 are `done — <sha>`.

**Checklist (above):** five targets ✅; both themes, no healthy mark, Disable survives Refresh, select-all count + Cancel,
960×640 refusal, VoiceOver through the flows ✅ (operator); the collision ask by ticket 16's audit; keyboard project
selection by ticket 17's audit. `styles:check` in `check` and CI ✅; DESIGN.md documents every primitive ✅.

**Residue, all recorded:** BACKLOG #68–#78 (round-19 and think-aloud carries, 10-02 and 10-27), #80 (toast stack over
the header, from 27's audit), #81 (live-smoke findings: `<title>`, search autocorrect, 10 Tabs to the first result,
one-result group label), #82 (dead i18n keys). #79 became ticket 29 and shipped. 19b (tickets 20–26) stays live under this
effort; 20 and 21 carry binding think-aloud comments; #71 needs a grill before 19b builds near it.

**Dropped by name:** nothing. Release-process lessons live on ticket 19 § Release run ids (idle-fork sweep; macOS runner
scarcity on Free; `assemble-updater-json` moved to `ubuntu-latest` in `457085e`).

**Archived with this closure:** `archive/round18/` → `archive/round18/` (its closure block dated 2026-09-27 in `753e895`).
