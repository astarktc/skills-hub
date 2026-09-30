# 14 — Notifications grouped by run with actioned rows; header progress; Cancel policy; focus rules applied

Status: ready-for-agent
Blocked by: 03, 09, 12, 13
Spec: `.scratch/round19/spec.md` — D11, D19, D20, D21. Sources: `flows.md` X1–X3, X5, R-X1–R-X3, Flow 2 gaps 1, 3, 7;
`critique-astra` F13; `audit.md` Notifications/Motion rows.

## Work

- **Toasts**: only the composite headline (ticket 03) + **View report**; tone from the fold; sonner themed by tokens
  (ticket 07); a toast never covers the Notifications panel's header while it is open (position or suppress).
- **Notifications panel**: entries **grouped by run** (`runId` from the reporter): the headline is the group row with
  the action's name and time; expand shows items; **rows carry actions** — "Show" (surface state: focus that skill,
  open the inspector, close the panel), Retry, Overwrite… (through the ask), "Open Settings → GitHub" for auth /
  rate-limit, "Change source…" for 404. Per-entry Copy keeps entry-specific accessible names; Copy all and Clear stay.
  Project rows name the project ("in *monorepo*: tdd → Claude Code"). Session-only, as today.
- **Header progress** (D19): the `LoadingOverlay` is deleted; a streamed batch renders in the header region
  ("Refreshing 12/80 · Cancel"), the library stays browsable, per-row pending marks from the reporter's per-subject
  pending; other mutations are disabled with the reason as a tooltip/`aria-description` ("Refresh running"). Small
  actions show inline pending on their control (button `aria-busy`), no overlay. Action-specific copy (Refreshing /
  Adding / Syncing / Importing / Removing), never "Installing Skills…" for everything. Under reduced motion no
  spinner/shimmer — text and counts only.
- **Cancel** (D20): rendered only when `cancellable`; pressing it stops what it can and the report says what happened
  ("Refresh cancelled — nothing was changed" / "Added 2 of 5; 3 cancelled").
- **Passive update affordance** (D8, arch 8): a quiet header item from `useAppUpdate` ("1.3.1 available") opening
  the update dialog; no modal on startup.
- **Focus rules** (D21) applied at every remaining call site: after dialog close → trigger or fallback; after Restore
  → Health row; after Delete → next row; verified by transcript.
- Re-assert additions announced in the Refresh report ("also deployed to Cursor, Pi, Gemini CLI") — the report rows
  exist; render them.
- Migration: `NotificationsModal`, `LoadingOverlay` (deleted), header files join `migrated`; `notif-*` rules leave
  `App.css`.

## Gate (D31)

Astra audit on `failures` with `latency=3`: Refresh runs with the library browsable; another mutation is refused with
a reason; Cancel honoured; the report groups by run and "Show" lands on the skill with focus; keyboard transcript;
reduced-motion emulation via CSS coverage (no `animation` without a `motion-reduce` counterpart). `npm run check`.

## Result

Implemented 2026-09-30 (child, uncommitted over `d191d6a`). Evidence: `.scratch/round19/evidence/14/` (local-only;
drivers `refresh.ts` · `report.ts` · `flows.ts` · `surfaces.ts` · `bar.ts` · `a11y.ts` · `motion-css.mjs`, trusted CDP
keys through `keys-cdp.ts`; `summary.txt` is the readable transcript digest).

**Shipped**

- **One lifecycle, header progress (D19).** `LoadingOverlay` is deleted (and its `loading-*` / `progress-*` /
  `processing-*` / `notif-*` App.css rules). `shell/HeaderProgress.tsx` reads the reporter's `pending` alone:
  action-specific words (`PendingAction.activity`, set by each caller — "Refreshing", "Syncing", "Disabling",
  "Adding", "Importing", "Removing beta", "Loading preview"…; fallback "{label} running"), `done/total`, a determinate
  bar (`scaleX`, `motion-reduce:transition-none`, hidden below the shell breakpoint), the step line, Cancel only while
  `cancellable`. Three placements: My Skills' header slot (it takes the exception summary's place while it runs — the
  row never wraps, 960 included), a strip above any other area, and the foot of a dialog whose action runs (Add,
  pickers, Import, Re-point — the page behind a modal is inert). `showsHeaderProgress` replaces
  `showsProgressSurface`; a bulk Remove (`skills`) stays the Delete dialog's (one surface). A batch names its skills
  (`{ kind: "batch", ids }`): their rows are `aria-busy` and their overflow button shows `pending`.
- **Refusal with a reason**: header verbs, rail, inspector sections and panel mutations stay focusable while
  disabled (`focusableWhenDisabled`), reason as `title` + `aria-description`.
- **Cancel (D20)** only where honoured: `ProgressTick.cancellable: false` withdraws Cancel for the rest of the action —
  the Refresh apply phase (the token is checked between acquisitions only), not restored by a later `uncancellable`.
- **Overlay coupling removed (12's note)**: App's `sheetOpenFor` latch, `showsProgressSurface` use and
  `whenSettled` 90-frame wait / 6-frame guard are gone (one placement per arrival, disposed on change); the door's
  overlay wait + 12-frame hold are gone (`restoreFocus` places once when the settled render is on screen, one frame
  of grace for a re-enabled control). The layer-stack hand-over stays. New floor: `actionLifecycle.returnLostFocus`
  (called by `runAction` for every action) returns lost focus (`isFocusLost`: body, main, a layer's popup itself, the
  vanished Cancel) to the triggering control, else the main landmark — never `<body>`. The Dialog primitive's opener
  is the menu's trigger when a dialog opens from a menu item (`openerOf`; `dialog.test.tsx` pins it, fails without).
- **Toasts (D11)**: `applyOutcome` shows ONE toast per run — the composite headline (or the first entry "+N more"),
  no description, "View report" → `openReport(runId)`. While the Notifications panel is open no toast is shown and
  opening it dismisses those on screen (a reporter concern: `useStatusReporter(t, surface)`).
- **Report by run**: `recordRun(headline, entries, label)` is still the one writer; rows now carry data (`subject`,
  `error`, `repair`, `review`) instead of closures (the `OutcomeEntry.action` / `onAction` / `canRepoint` path is
  removed). `lib/notificationRuns.ts` — `groupRuns`, `rowActions`, `rowText`, `historyText` — are the pure folds (9
  tests). `NotificationsModal` (migrated) groups by run (headline · action · time · N items, disclosure), items with
  Show (panel returns focus to the inspector heading / sheet), Retry, Overwrite… (the same re-sync raises the one
  ask), Remove…, Change source… (404), Open Settings → GitHub (auth / rate limit; lands on the token field), a
  project row prefixed "in {project}:", Copy named by its row, Copy all, Clear. The run "View report" asked for is
  expanded and focused, else the newest; the choice is made as the panel opens (a run recorded meanwhile never
  collapses it).
- **Dismissed ask fold (11's audit Q4)**: a `TARGET_EXISTS` still in a report is an unanswered collision (every
  producer asks first) — `HeadlineCounts.collisions`, "N folders to review", conflict tone, a `review` entry with
  an `overwrite` repair, never an error; no retry, no implicit Keep. Across sync, enable, install, import, refresh
  (Propagation/re-assert), Edit and project folds (tests in `reportOutcome.test.ts`, door test with the real sync
  world).
- **Re-assert additions announced**: `refreshOutcome(…, { priorGlobalTools })` — "zod-schema-first also deployed to
  Codex, Pi, Gemini CLI" lines ride the run's headline.
- **Passive update (D8)**: `useAppUpdate` no longer offers a dialog on startup; `announced` + `open()`; a quiet
  sidebar item "1.3.1 available" opens `UpdateDialog`; Don't remind me quiets it.
- **Focus (D21)**: count confirmations close as their verb starts (focus back on the rail button; the run shows in the
  header); verified transcript below.

**Verification (fresh, `failures` + `latency=3`, trusted CDP keys; dark 1280×800 and light 960×640)**

- Refresh all: header "Refreshing 1/19 · Fetched (1/19) tdd … · Cancel", 22 rows `aria-busy`; j/j/k/Enter browsed and
  opened the inspector mid-run; Add skill kept focus, `aria-disabled`, described "Refresh is running — wait for it to
  finish", Enter did nothing; apply phase "Updating (1/19)" offers no Cancel; a fresh run cancelled from the header →
  "Cancelling… 2/19" → toast "Refresh cancelled — nothing was changed", focus back on Refresh all.
- Full run: dismissed re-assert ask → focus Refresh all; toast "13 refreshed · 5 failed · 14 targets failed · 4 skipped
  · 1 Edit to review · 3 folders to review · View report"; panel: one run, 27 items, 0 toasts while open; Show →
  inspector H2 of pr-review (desktop) / sheet H2 (960); Change source… → its dialog, Escape → the row / surface
  heading; Settings → GitHub → `#/settings/github`, token field focused; Overwrite… → the ask above the panel, Escape
  → focus back on Overwrite…; Retry → a new run in the panel, no toast; close → the bell.
- Bulk Disable (rail): header "Disabling", 3 rows pending, rail disabled with reason, focus on "Disable on all
  Tools" throughout; toast "5 disabled". Bulk Remove: header empty, 2 rows pending, the Delete dialog pends; toast
  "2 removed", focus → next row. Row menu → Change source… → Escape → "More actions for alias-skill". Restore → Button
  keeps focus while pending → Health section. Delete → next row (frontend-design). Late completion after navigating
  to Settings keeps focus on Settings. Resize 1280↔960 with Change source open: the child stays top, focused; close →
  sheet H2 / Change source… button. Dismissed Sync ask → focus on the rail button, toast `warning` "4 synced · 1
  skipped · 1 folder to review", no error row. Add listing cancelled from the dialog's foot → focus Install. Explore
  preview: strip above the area ("Loading preview · Cancel"), settles on MAIN. No update dialog at startup; the item
  opens "What's new in 1.3.1".
- Keyboard transcripts: 887 recorded steps, 0 on BODY, all `:focus-visible`.
- axe 4.10.3: 8 runs (header progress running + panel × dark/light × 1280/960) — 0 violations (panel keeps the known
  Base UI `aria-hidden-focus` incomplete). Contrast: header progress min 8.31 (dark) / 8.16 (light); panel rows min
  6.61 / 5.32; 0 pairs < 4.5.
- Reduced motion: emulated — spinner `display:none`, bar `transition-duration:0s`, row spinners `animation none`, 0
  running animations; built CSS carries the base layer's global zeroing plus `motion-safe:animate-spin`,
  `motion-reduce:hidden`, `motion-reduce:transition-none` (`motion-css.json`).

Gate: see the dated comment below.

## Comments

- 2026-09-29 — from tickets 03/09/10: `useNotificationHistory.recordRun(headline, entries)` is called by the reporter's `applyOutcome` (one block per run); the reporter exposes `pending { kind, id? }` and the batch `{ label, done, total, cancellable }` — the `LoadingOverlay` still renders it; replace with the header progress here. `useAppUpdate` feeds the passive title-bar affordance. Focus gap to close: an overwrite ask opened mid-action and then closed does not return focus to the triggering control.

- 2026-09-29 — from ticket 11's audit re-check (Astra, `review/ticket-11-audit.md` § Re-check Q4): an **unanswered `TARGET_EXISTS` row after a dismissed overwrite ask currently folds as a failure** ("Sync failed … Target folder already exists" toast, error tone) — it must fold as **conflict / "to review"** (or a quiet dismissal), never `failed`, per D10/D12 ("a dismissed ask sends nothing"; `declined`/`cancelled` are outcome kinds, never failures). Preserve: no retry, no implicit Keep, the pair stays as it was. The fold lives in `src/lib/reportOutcome.ts` (09's `compositeHeadline` / conflict completion) — do the semantic correction here in 14 since 09 is closed, with a pure-fold unit test; the notification history block then shows the "to review" kind.

- 2026-09-30 — from ticket 12: while the `LoadingOverlay` lives, the door's `restoreFocus` waits for it and holds the
  D21 target for ~12 frames against the overlay's late focus return (`useSkillActions.ts`); App's arrival/close
  placement does the same (`whenSettled`). Retiring the overlay should let both shrink. A modal sheet (the inspector
  < 1200) must not open *under* the overlay — App latches the sheet until progress clears (`sheetOpenFor`); header
  progress makes that latch unnecessary. The Dialog primitive now returns focus to its opener in Chrome (it was
  judged while its own `inert` was on) — a layer opened with focus nowhere returns nowhere. The Targets section's
  "Removed from {Tool} · Undo" is a 5 s inline affordance (`UNDO_WINDOW_MS`), not a toast; keep it out of the run's
  notification block or record the Undo as its own run.

- 2026-09-30 — from ticket 13: the rail's batch verbs (Disable / Enable / Update / Sync over a selection) run with
  subject `batch`, so the legacy overlay shows over the closing count confirmation (its popup holds focus at 0 ms;
  the confirmation closes beneath it and hands its return chain up — focus lands back on the rail button). With
  header progress, the count line should read as the run's headline ("80 disabled · 3 failed", "5 enabled",
  "2 removed · 1 failed" are already the folds' toasts) and the rail stays visible and disabled-with-reason while it
  runs (its buttons are `focusableWhenDisabled`). A bulk Remove uses the new `{ kind: "skills", ids }` subject:
  rows pend inline and the Delete dialog shows the count ("Removing beta") from `actionMessage` — keep that out of
  the header or mirror it, but never both as competing surfaces. The overlay's title says "Installing Skills…" for
  every batch.

- 2026-09-30 — child result (ticket 14): gate `npm run version:check && npm run check` exit 0 — Version OK (1.2.18);
  styles:check ok, App.css **1882** counted lines (ceiling lowered from 2076), **26** migrated (+ `NotificationsModal.tsx`,
  `shell/HeaderProgress.tsx`); vitest **41 files / 887 tests**; `tsc -b && vite build` ok; cargo fmt / clippy
  `-D warnings` ok; cargo **807** passed. `src/bindings/index.ts` unchanged (no backend change). Decisions for the
  audit: (1) the header progress replaces My Skills' exception summary while a batch runs (the only way the 960 px
  header stays one row; the Issues chip still filters); (2) a `TARGET_EXISTS` reaching any fold is treated as
  unanswered — every producer asks first — so the "to review" rule lives in the folds, not in a flag threaded from the
  ask; (3) "Overwrite…" in the panel is `run("retry", [id], { tool })` — the re-sync raises the one ask again;
  (4) `returnLostFocus` in `runAction` is the lifecycle-wide D21 floor (the door's `focusAfter` still decides Restore
  / Delete / same-control), plus the Dialog primitive's menu-trigger opener; (5) a Refresh's apply phase withdraws
  Cancel (`ProgressTick.cancellable: false`) — the backend honours the token between acquisitions only.
  Not done / open: row Switch controls stay natively disabled while an action runs (focus on a pressed switch drops
  and the door puts it back after — the Switch primitive has no `focusableWhenDisabled`; ticket 15 may add it);
  the Projects world's dialogs (register / edit / remove project) get no dialog-foot progress (ticket 17); the
  Explore preview settles focus on MAIN rather than its heading (ticket 22).

