# 25 — `App.css` deleted; the guard flips to "must not exist"; `Layout.tsx` / `Dashboard.tsx` disposition

Status: done — (sha in the tracker commit)
Blocked by: 20, 21, 22, 23, 24
Spec: `.scratch/round19/spec.md` — D32; ADR-0006 §5–§6. Phase 19b closure.

## Work

- Every remaining `App.css` rule is either migrated with its surface (there should be none — list them if any and
  migrate), moved to `prose.css` under the prose root, or deleted with the alias block. `git rm src/App.css`;
  `scripts/check-styles.mjs` rule 1 becomes "`src/App.css` must not exist"; `styles-baseline.json` drops
  `appCssCeiling`.
- `src/components/Layout.tsx` and `src/pages/Dashboard.tsx` (dormant by design) would render unstyled: propose
  deletion to the operator in the Result with the evidence that nothing imports them; delete only on the operator's
  word (AGENTS.md says it is an operator call).
- AGENTS.md: the styling bullet loses every `App.css` sentence; ADR-0006 gains a dated "`App.css` deleted" note.

## Gate

`npm run check` (guard passes on the absence); fixture walk of every surface in both themes shows no unstyled element.

## Result

Implemented by a delegated child on `t3/delete-app-css` (uncommitted, from `771358b`); awaiting the Astra audit.

**What was left in `App.css`** (355 counted lines, 44 class selectors + the alias block + 1 non-class rule +
`@keyframes spin`): 4 surfaces still used 17 of the classes; the other 27 were dead (no `src/` class expression
referenced them; the guard's own AST reader, run over every `src/` file). Every legacy custom property in the alias
block had no reader outside `App.css`. The one live non-class rule, `[data-sonner-toast] [data-description] {
white-space: pre-wrap }`, moved into the Toaster primitive (`descriptionClassName: "whitespace-pre-wrap"`; verified
on a fixture toast: computed `pre-wrap`). `@keyframes spin` duplicated Tailwind's own (still emitted for
`motion-safe:animate-spin`). Nothing moved to `prose.css`. **Deleted: all 355 lines** (`git rm src/App.css`, its
`import "./App.css"` in `App.tsx`). `index.css` still carries the whole base layer (ground, text, tabular numerals,
focus ring, reduced motion, `@theme` chain) — it never depended on `App.css`.

**The four surfaces** (no new variant; DESIGN.md § The last four dialogs; each is a migrated file by construction —
the `migrated` list itself retired, below):

- `InvocationModeModal.tsx` — `FieldSet` + `FieldLegend` of native radios in Import review's described-radio row
  (`accent-primary`, `focusRing`, row `raised` on hover); each radio named by its mode (`aria-labelledby`) and
  described by its explanation (`aria-describedby`; before, the explanation was part of the name). RadioGroup not
  used: it is a row of 2–5 short segments, these five options carry sentences. The conflict banner (a tinted
  `invocation-edit-warning` box) is `stale` text + `TriangleAlert`, `role="status"` kept — the inspector's Edits
  phrasing. Footer: `Button` outline Cancel, `solid accent` Save (`type="submit" form`).
- `OverwriteModal.tsx` — the rows are one bordered list (`border`, `divide-border` hairlines, `rounded-md`,
  `max-h-[40vh]` scroll); pair `foreground`, path `font-mono xs faint` on its own line (before, the `span`s ran
  together on one line — visible in the before shot), facts `xs muted`.
- `UpdateDialog.tsx` — `DialogHeader`/`DialogTitle` (names the popup; the hand-rolled `aria-labelledby` + `useId`
  went), body sentence, notes under `.prose` in a bordered `max-h-52` scroller (same as Settings → Updates), footer
  "Don't remind again" outline + Update now `solid accent`. Update now is **one** button through installing (`pending`,
  "Installing…") and done ("Done") — fixture walk: focus stays on it from click to Done (before: `disabled` dropped it).
- `NewToolsModal.tsx` — Later outline, Deploy all skills `solid accent` (`disabled={loading}` kept: `loading` is the
  global flag, so `pending` would claim this button's work runs).

**The guard** (`scripts/check-styles.mjs`, ADR-0006 §6 dated note): rule 1 `[legacy]` — `src/App.css` must not
exist, and no source file, stylesheet (`@import`) or `index.html` (`href`/`src`) may import an `App.css` (comments are
not imports). Rule 2 `[retired]` — the `migrated` list meant nothing once every file is migrated, so it became
`retiredClasses`: the 44 class selectors App.css defined at deletion, frozen; any `src/` source file whose class
expressions (same AST reader) use one fails — the stale-branch-merge path AGENTS.md § Worktree safety warns about.
Rule 3 allows only the permanent three. Baseline: `appCssCeiling` and `migrated` dropped, `rawColourExempt` now `[]`
(both entries were the deleted files). `--count` / `countCssLines` / `definedClasses` removed. Self-test: 26 cases
(rule 1: recreated file, TS import with the file absent, CSS `@import`, `index.html` link, a comment naming it is
accepted; rule 2: the five former rule-2 counterexamples now without `markMigrated`, plus a non-class use of
`error`/`status`/`alert` accepted).

**Deleted** (operator 2026-11-04): `src/components/Layout.tsx`, `src/pages/Dashboard.tsx` (`src/pages/` gone), the
`layout.*` (4) and `dashboard.*` (5) `en` leaves (only those two files read them; the reverse guard stays green), and
`react-router-dom` (only `Layout.tsx` imported it; `npm uninstall` also dropped `react-router`, `cookie`,
`set-cookie-parser` from the lockfile). AGENTS.md's "dormant by design" bullet removed.

**Ticket 15 audit A4** (the 2026-10-02 Comment): deleting `Layout.tsx` did **not** drop `.transition` /
`.transition-colors` / `.ease-in-out` — `.scratch/` and docs prose minted them. `src/index.css` now imports
`tailwindcss source(".")` (the app's source only; `index.html` has no classes), and two `src/` comments that named a
utility as a bare word were reworded (`ui/styles.ts`, `ui/sonner.tsx`). Built CSS 62,307 → 54,079 B; the selector
diff is 94 classes removed, 0 added, none of them used by `src/` (a `src/` candidate is still scanned; the DOM audit
below found no undefined class). `.transition-opacity`, `.transition-transform`, `.ease-out` stay (used).

**Docs**: AGENTS.md styling bullet (no `App.css` sentence; "App.css deleted at 1.3.1, ticket 25"; the
`styles:check` line), ADR-0006 dated note + Consequences bullet, DESIGN.md (§ Stylesheets line, § Migration rule
rewritten as complete, § The last four dialogs, Toaster line-break note, the alias-block / `.spinner` mentions in §
Tokens, § hardest pairs, § Motion), CHANGELOG Unreleased "New look" line.

**Fixture walk** (`vite --mode fixture --port 5200`, ego-browser, 1280×800, both themes via
`prefers-color-scheme`; evidence gitignored under `.scratch/round19/evidence/25/`):

- Before/after pairs, dark and light: `invocation-*`, `overwrite-*`, `update-*`, `newtools-*`
  (`{before,after}-{dark,light}.png`), plus `invocation-conflict-after-*` (`failures` → api-style), `update-done-after-dark`,
  `toast-after-dark`. Reached: Invocation from api-contract-review's row menu → Edit invocation…; the overwrite ask
  from zod-schema-first's inspector → Deploy to Cursor (`rich`); Update with `?update=1.4.0` → the sidebar's
  "1.4.0 available"; New harnesses — see below.
- Each of the four: the AX name is the same before and after ("Invocation mode — api-contract-review", "Folder
  already exists", "Update available.", "New harnesses detected"), Escape closes it, focus returns to the opener
  (the row's More actions trigger, Deploy to Cursor, the sidebar item; the startup New harnesses announcement
  returns where it was opened from, the same before and after).
- `walk/` — My Skills, a row menu, the inspector, ⌘K, Add, Explore, Projects, Settings × 5 panes, Import review
  (`first-run`), dark and light (26 shots). On each, a DOM audit listed every class token with no matching selector
  in any loaded stylesheet: only `lucide-*` (icon markers), `language-bash` (Markdown code) and `peer` (a Tailwind
  marker) — no unstyled class.

**Not proven / notes for the auditor**

- **New harnesses is unreachable in the fixture as shipped**: the fixture's `toolStatus()` reports newly detected
  harnesses once and clears them, and dev StrictMode calls it twice, so the announcement never shows (`first-run`
  included). The shots were taken with that line (`state.seenTools = [...installed]`, `src/fixtures/backend.ts`)
  commented out locally, then reverted (the tree has no fixture change; `backend.test.ts` pins "once"). A fixture fix
  is a separate item.
- Focus *into* the Invocation and Update dialogs read as still on the opener in some captures, before and after
  alike — the ego tab runs in the background and Base UI moves focus a frame late; the light Invocation capture shows
  it landing on the checked radio. Not a change of this ticket.
- Saving a tracked doc still reloads an open fixture page after the `source(".")` change (tested), so AGENTS.md's
  fixture note stands.

**Gate**: `npm run styles:check` (self-test 26/26), `npm run test` (56 files / 1250 tests), `npx tsc --noEmit -p
tsconfig.app.json`, `npm run lint`, `git diff --check`, `npm run version:check && npm run check` — all green (see the
final run in the child's report).

## Comments

- 2026-10-01 — from ticket 17: App.css is at **1108** counted lines (raw 1297). Projects took `matrix-*`, `project-*`,
  `projects-*`, the `form` family except `.helper-text` (still `InvocationModeModal`), `delete-*`, `.btn-danger-solid`,
  `.btn-xs`, `gitignore-*`, `.cell-*`, `.skeleton-row` + the `shimmer` keyframes, `group-by-repo-*`, `.sort-*`. A scan
  for App.css classes no `src/` file references found many already-dead rules outside this ticket (`explore-*` list
  rules, `modal-*`, `settings-*` list rules, `alert-*`, `brand-subtitle`, `btn-danger`, `btn-full`, `card-btn`,
  `icon-btn`, `space-between`, `status-row`, `update-modal-close`) — deletable with no migration.

- 2026-10-02 — (orchestrator, recorded late) from ticket 15's audit **A4 (P2)**: Tailwind generates `.transition`,
  `.transition-colors` and `.ease-in-out` from source scans (a `styles.ts` comment, dormant `Layout.tsx:15`, docs).
  Nothing live applies them: the audited DOM has 0 uses. They're harmless (120 ms, no `all`). Ticket 15's gate
  Comment left them with this ticket, but that entry never landed. When this ticket decides `Layout.tsx`'s
  disposition, narrow irrelevant source scanning (`@source not`) or let the dormant file's removal drop them, and
  keep useful utilities. Keep claims precise: dormant source is not the same as no reference. Source:
  `round19/review/ticket-15-audit.md` § A4.
- 2026-11-02 — Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update — ticket 32.
- 2026-11-04 — operator: **delete `src/components/Layout.tsx` and `src/pages/Dashboard.tsx`** (dead scaffold from `24c5ae4`, never rendered, palette classes that no longer exist). The child deletes them with `App.css`, drops their `dashboard.*` i18n leaves if nothing else reads them, and removes AGENTS.md's "dormant by design" bullet.
- 2026-11-04 — implemented by delegated child; awaiting Astra audit.
- 2026-11-04 — orchestrator: Astra audit PASS-with-findings (`round19/review/ticket-25-audit.md`): F1 (the guard's import
  detection is regex, comments/strings can fool it) and F2 (the retired-class scan inherits the raw-colour exclusions —
  tests, fixtures, bindings — while the docs say every source file) are P2 notes on the guard, left as documented
  limits; F3 (the New harnesses dialog is unreachable in the fixture under StrictMode — the fixture reports newly
  detected harnesses once and the startup effect runs twice) → BACKLOG #90. The dialog itself renders correctly
  (the auditor mounted it directly).
