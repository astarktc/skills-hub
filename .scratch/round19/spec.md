# Round 19 — the new world and the workflow features (UI effort phase B) → 1.3.0 (19a), 1.3.x (19b)

Status: open
Opened: 2026-09-27 (grill: `round18/issues/09`, operator + orchestrator, pre-accepted for the session)

Round 18 judged; round 19 builds. This spec records the decisions of the ticket-09 grill over round 18's findings
(`round18/review/{architecture,critique-astra,critique-opus,audit,flows}.md`) and turns them into tickets. It absorbs
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
  world ticket's build, using `round18/review/flows.md`'s per-flow questions; findings feed the surface tickets,
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
| 27 | A refused click says why: held-disabled controls show their reason when activated (from 18; added 2026-10-27) | — | pre-push fix |
| 28 | Executable bits survive the GitHub Contents fast path (UNI-163; added by the operator 2026-10-27) | — | pre-push fix |
| 29 | A toast raised while a modal is open is announced: the toaster stays live under the layer stack (BACKLOG #79; operator "fix first" 2026-10-27) | — | pre-push fix |

### 19b → 1.3.x

| # | Title | Blocked by | Kind |
|---|---|---|---|
| 20 | **Settings panes**; Configure Tools → Settings → Tools; theme radio group; named header controls (D24) | 19 | surface |
| 21 | **Import review**: unchecked conflicts / managed names, Replace/Leave, remove-originals checkbox, counts, variant details, Tool labels, first-run empty state (D13, D14) | 05 19 | surface |
| 22 | **Explore rows**: Not-in-library filter, Preview primary, scope summary, installed → inspector, contrast + search label (D25) | 19 | surface |
| 23 | **⌘K palette** on the surface module and the action door; `cmdk` vs Base UI Combobox decided here | 19 | surface |
| 24 | **Card-grid view mode** designed inside the world (same selection, strip, marks; no clipped labels) | 19 | surface |
| 25 | `App.css` deleted; guard flips to "must not exist"; `Layout.tsx` / `Dashboard.tsx` disposition | 20 21 22 23 24 | closure |
| 26 | CHANGELOG, `version:set 1.3.x`, release; operator smoke (D31) | 25 | release |

## Execution

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
