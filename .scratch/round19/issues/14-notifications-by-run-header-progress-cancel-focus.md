# 14 — Notifications grouped by run with actioned rows; header progress; Cancel policy; focus rules applied

Status: implemented
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
- **Dismissed ask fold (11's audit Q4)**: a `TARGET_EXISTS` still in a report is an unresolved collision (the ask was
  dismissed, or the producer raised none — Import does not ask yet, ticket 21) — `HeadlineCounts.collisions`, "N folders to review", conflict tone, a `review` entry with
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

- 2026-09-30 — independent D31 audit of `86bb753`: **FIX-THEN-PASS**, report `.scratch/round19/review/ticket-14-audit.md`, fresh evidence `.scratch/round19/evidence/14-audit/`. Four **P1** findings: **A1** toast View report opens the panel but leaves focus on BODY at 300/2000 ms; **A2** Clear disables its focused button and likewise leaves BODY; **A3** Overwrite… on a dismissed Enable collision incorrectly dispatches ordinary Retry, which skips the still-disabled pair instead of raising the ask; **A4** panel source Retry exposes Cancel only behind the inert modal background. No P0. Core Refresh/browse/refusal/early Cancel/apply withdrawal, bulk Disable/Remove, Show/Settings, menu/Restore/Delete/late focus, grouped reports and reduced motion pass the exercised scope. Eight axe runs: zero violations; contrast minima header 8.31/8.16, panel 6.61/5.32. Full version+check rerun exits 0 (887 frontend / 807 Rust; initial run had one timeout). Six decisions: accept summary replacement; accept unresolved-collision folding but Import does **not** ask today (existing ticket-21 obligation); accept a new ask per Overwrite action but fix Enable routing; accept permanent apply-phase Cancel withdrawal; switches retained focus as SPAN role=switch in both tested layouts (15 Comment exists); 17 Comment exists but **22's Explore-focus Comment is missing**. P2 notes cover project identity on head/copy, one transient resize BODY gap, and documentation/handoff accuracy. Orchestrator should fix A1–A4, add 22's handoff, and use the report's bounded re-check scope; this comment does not grant D31 acceptance. A concurrent docs-only BACKLOG commit `bb64241` changed no audited application files.

- 2026-09-30 — fix child for the D31 audit (A1–A4, N1, N3; uncommitted over `743324d`). Evidence
  `.scratch/round19/evidence/14-fix/` (local-only; drivers `panel.ts` · `pointer.ts` · `enable.ts` · `a11y.ts` · the
  audit's `flows.ts`, trusted CDP keys through `keys-cdp.ts`, session `r19-14-fix`, closed).
  - **A1** (View report → BODY): sonner returns focus to the pre-toast element when focus leaves its toaster; the panel
    took focus first, sonner pulled it back onto the page, inert then dropped it. `useStatusReporter.openReport` now
    runs `settleToastFocus()` before it opens the layer — it blurs a focused toast control, so sonner's own return
    completes (and clears its memory) while the page is still usable; the panel then takes initial focus from there
    and returns there on close. No holds, no retries. Seam test `src/hooks/useStatusReporter.toaster.test.ts` mounts the
    real sonner: keyboard and pointer activation, sonner's own toast removal included; fails without the fix (a focus
    lands on the page after the panel took it). Browser: `panel-{dark-1280,light-960}.json`,
    `pointer-{dark-1280,light-960}.json` — samples 0/300/2000 in the panel, never BODY; the only focus outside is
    sonner's return before the panel opens; toast gone at 2000.
  - **A2** (Clear → BODY): Clear and Copy all are `focusableWhenDisabled` (focus stays on Clear, D21 same control); the
    empty state's message is the empty panel's initial focus (`data-run-initial`, so a panel opened empty does not land
    on the disabled Clear). Browser: Clear 0/300/2000 on Clear; Tab → Copy all → Close; Escape → the page.
  - **A3** (Overwrite… after a dismissed Enable ask runs ordinary sync): the repair keeps its route as data —
    `EntryRepair.overwrite.via: "sync" | "enable"` (`reportOutcome.ts`: `enableOutcome` folds its global rows with
    `collisionRoute: "enable"`; `RowAction.overwrite.via`, `notificationRuns.ts`). The panel dispatches `run("enable",
    [id], { tool, answering: "collision" })` for an Enable collision (pair scope → `enable_skill_targets` with its
    disabled-marker admission, the one per-pair ask, overrides on retry) and `run("retry", …)` otherwise. The door checks
    the pair as it is now first (`staleCollision`: skill disabled since / pair disabled since / already deployed /
    Enable's marker gone) and says so (`staleCollision.*` i18n, end of `en`) instead of acting. Real-door tests
    (`useSkillActions.test.ts` "Overwrite… on a dismissed Enable collision"): Overwrite / Keep / dismiss / stale skill /
    stale sync pair; pure tests in `notificationRuns.test.ts`, fold tests in `reportOutcome.test.ts`. Browser
    `enable-{overwrite,keep,dismiss,stale}-{dark-1280,light-960}.json`: one ask each; Overwrite → claude_code `synced`,
    foreign gone, "Enabled and synced."; Keep → pair `disabled`, foreign kept, "kept your existing Claude Code folder";
    dismiss → unchanged, "1 folder to review"; stale (operator switched the skill off meanwhile) → no ask, state
    byte-identical, "Nothing overwritten: retired-playbook has been disabled since this report…".
  - **A4** (panel Retry's Cancel behind the inert page): `NotificationsModal` takes `progress` — App's same
    `dialogProgress` (reporter `pending`, no second state) — at its foot. Browser: source Retry (Update slowed ×8 by a
    harness timer shim, timing only) → foot "Updating pr-review … · Cancel", reached by Shift+Tab, Enter → "pr-review:
    update cancelled — nothing was changed", focus back on Retry: pr-review 0/300/2000; target Retry (tdd, token-less)
    shows no Cancel; the panel opened mid-Refresh shows "Refreshing 6/19 · Cancel", reachable by keyboard.
  - **N1**: one project-aware presentation in `notificationRuns.ts` (`rowProject` / `rowTitle`, `rowText(row,
    inProject)`, `historyText(groups, inProject)`) read by the run heading, the item and both copies; a resync-all
    title that names its project already carries `subject.projectInTitle` and is not prefixed (it was doubled before).
    Regression: a single failed project toggle promoted to the heading keeps "in monorepo:" on screen and in both copies.
  - **N3**: `reportOutcome.ts`'s collision comment now says an unresolved collision (the ask dismissed, or none raised —
    Import does not ask yet, ticket 21), not proof an ask happened; this ticket's Result wording corrected likewise.
  - **N2** re-check (shared focus code touched by A1/A2): `flows-dark-1280-dialogClose,restore,resize,dismissed.json`
    reproduces the same single `key Escape` BODY sample at the same step (Change source closed after the resize
    round trip; next observation the sheet H2) — unchanged; light 960: 172 samples, 0 BODY.
  - axe 4.10.3, 6 runs (panel with progress foot mid-Refresh, full report, emptied after Clear × dark 1280 / light 960):
    0 violations (Base UI `aria-hidden-focus` incomplete as before); contrast minima foot 7.78/7.60, panel 6.61/5.32,
    empty 7.78/8.36; 0 pairs < 4.5.
  - Gate `npm run version:check && npm run check` exit 0 (`gate.txt`): Version OK (1.2.18); eslint ok; styles:check ok,
    App.css 1882 (ceiling), 26 migrated; vitest 42 files / 900 tests; `tsc -b && vite build` ok; cargo fmt / clippy
    ok; cargo 807 passed. Bindings unchanged (no backend change).
  - Re-check focus: the toaster hand-off (a toast whose pre-toast element is gone or inert; VoiceOver); the stale
    rules in `staleCollision` (an Enable pair the operator re-disabled by hand is indistinguishable from the collision's
    marker and is answered by Enable); the empty-state initial focus; the panel foot during an ask above the panel.

- 2026-09-30 — independent bounded re-check of `f9e4e0e`: **PASS for A1–A4**, N1/N3 closed; report `.scratch/round19/review/ticket-14-audit.md` § Re-check (2026-09-30), fresh evidence `.scratch/round19/evidence/14-recheck/`. Both layouts pass keyboard/pointer toast handoff (including real-Sonner entering/stacked stress), Clear/empty landing, reachable panel Cancel and Retry return; Enable Overwrite/Keep/dismiss each raises exactly one ask, preserves unrelated targets, and uses Enable rather than ordinary sync. Stale skill/already-deployed repairs do not run; ordinary sync repair still takes Sync. No non-toast blur, whole-skill admission bypass, or stale closure found. Re-disabling an already-disabled collision marker is indistinguishable in the store (a no-op); a later explicit Overwrite… legitimately requests pair Enable with a fresh ask, not automatic redeployment. **P2 residuals:** N2's transient resize BODY sample unchanged; new N4 stale-report note is not live-announced; new N5 header progress remains visible behind the scrim alongside the active panel foot (one accessible Cancel, not one visual copy). Orchestrator owns explicit disposition of those notes; Import stays ticket 21 and native VoiceOver stays release smoke. Ticket 11 Escape/Keep/Overwrite → switch and Delete → next row regressions pass. Six axe checks: zero violations; contrast/reduced motion pass measured scope. Fresh version+full check exits 0 (**900 frontend / 807 Rust**); affected suite **246 passed**; bindings unchanged. Application source untouched; browser session closed; no commit.

- 2026-09-30 — **D31 gate passed** (orchestrator). Astra's re-check closed A1–A4 and N1/N3 (gate 900 / 807, bindings unchanged, 6 axe runs clean). Concern (a) ruled: an explicit Overwrite… legitimately takes the pair's Enable path with a fresh ask; a hand re-Disable of an already-disabled collision marker records no distinguishable new intent, so nothing to fix. Three P2 notes **deferred to ticket 15** (Comments there): N2 the sub-frame BODY sample during deferred inspector reparenting on resize; N4 the stale-collision note is written to history but not announced live; N5 while the panel is open, progress renders in both the inert header and the panel foot (one Cancel is reachable — decide whether the header copy should hide while a dialog owns the foot).
