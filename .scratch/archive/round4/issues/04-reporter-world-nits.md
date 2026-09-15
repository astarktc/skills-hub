# 04: Reporter world: notification ring hook, one clipboard helper, error-format seam, batch order, visible toasts

Status: done — 9828706

**What to build:** Batch error entries read top-down in the notification panel in the order they happened; copying a skill's path shows a toast but is not a Notification (never recorded in the history); a fourth open error toast is visible without hovering; project components hand errors to the reporter's own formatter instead of repeating the `describeCommandError` + `if (msg)` dance; the notification ring (entries, unread, clear) lives in its own building-block hook composed by the reporter; the binder imports the Toaster directly rather than through a hook-module re-export; the one hardcoded colour in the badge uses a token.

Source: `../spec.md` Q10; `../source-12-followups.md` #5, #7, #8, #9, #14, #15, #18.

**Blocked by:** None (can start immediately)

- [x] Test: `showActionErrors` with three entries records them so the panel (newest-first) shows the batch's first error at the top of its block
- [x] Test: the clipboard helper's success toasts and does not add a history entry; its failure is an error Notification
- [x] Test: `useNotificationHistory` — bounded ring of 100, unread counts errors/warnings until opened, clear empties
- [x] `describeCommandError` is imported by the reporter (and `commandError.ts` tests) only — not by `ProjectsPage` / `AssignmentMatrix`
- [x] The Toaster carries `visibleToasts` set once; `useStatusReporter.ts` no longer re-exports it
- [x] No hex colour literal remains in `App.css` for the badge
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 9828706. Evidence: Cited 9828706 integrated as 3fa299a; remaining slices 4fe97ab,5569dac,6cf62db,0d6b86a shipped v1.2.4. src/hooks/useStatusReporter.ts:320,348 preserves ordering/clipboard and src/App.tsx:248 sets visibleToasts=5. Later formatError retirement is intentional (round5 Q1).

### 2026-09-05 — implementation (branch r4/04-reporter-world-nits)

**Shipped** (5 commits on top of f057190):
- `9828706` refactor(reporter): extract the notification ring into `useNotificationHistory` — new `src/hooks/useNotificationHistory.ts` (+ test, 5 cases: newest-first/ids, ring of 100, unread = errors+warnings, markAllRead, clear). `useStatusReporter` composes it; `Notification`/`NotificationKind` types stay exported from the reporter so no importer changed.
- `9c77b57` fix(reporter): `showActionErrors` records last-to-first so the newest-first panel shows the batch's first error at the top of its block (existing test flipped red → green).
- `b45f0e2` refactor(reporter): one clipboard helper — **a reporter method** `copyToClipboard(text): Promise<boolean>` (type `CopyToClipboardFn`), chosen over `src/lib/clipboard.ts` because "toast without recording" needs the reporter's private `showToast`. Success = toast only (never in history); failure = error Notification. `SkillCard`/`SkillsList`/`NotificationsModal` take `copyToClipboard` instead of `notify` (it was their only use of `notify`). The modal keeps its local check-icon feedback on `true`.
- `291e8a0` refactor(projects): `notifyError(err)` (formatError → notify unless silent) + `formatError` on the reporter, passed from `App.tsx` → `ProjectsPage` → `AssignmentMatrix` alongside `notify`. The 9 `describeCommandError` sites in those two files are gone; `bulkAssign`'s per-tool detail uses `formatError`.
- `97c413e` refactor(app): `App.tsx` imports `Toaster` from `sonner` directly (comment names it the sanctioned second importer), `visibleToasts={5}`; re-export removed from the reporter. `.notif-badge` colour → `var(--accent-primary-fg)` (the pairing `.btn-danger` already uses with `--status-error`).

**Deviations**
- Acceptance line "`describeCommandError` imported by the reporter (and tests) only": met for the notification path (`ProjectsPage`/`AssignmentMatrix`, as the ticket body scopes it). Two *inline-display* uses remain untouched: `SettingsPage.tsx` (updater errors into local state) and `SkillDetailView.tsx:475` (file-read error into the content pane) — ticket 06 edits `SkillDetailView`, so I left both rather than widen the diff; a follow-up can pass `formatError` into them.
- `StatusReporter` gained three members (`notifyError`, `formatError` typed as `FormatErrorFn`, `copyToClipboard`) — additive; the two full-shape reporter doubles (`useSkillLibrary.test.ts`, `useAddSkillFlow.test.ts`) got the two new `vi.fn()`s. `App.tsx` changed beyond the Toaster import only to thread these props.

**Notes for the orchestrator**
- Gate: `npm run version:check` OK (1.2.3); `npm run check` green — vitest 13 files / 181 tests (was 172), cargo 445 tests, clippy clean, build OK.
- Merge touchpoints for ticket 06: `SkillCard.tsx` prop `notify` → `copyToClipboard` (import + props type + destructure + `handleCopy`); `SkillsList.tsx` likewise. `App.tsx` destructures `notifyError`, `formatError`, `copyToClipboard` from the reporter.
- No i18n keys added (reused `copied` / `copyFailed`).
