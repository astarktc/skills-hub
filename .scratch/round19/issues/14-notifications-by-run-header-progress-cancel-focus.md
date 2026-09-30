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

(child fills in)

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
