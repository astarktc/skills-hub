# 02: Notification history panel; every toast through the reporter

Status: done — dee04b6

**What to build:** A bell icon in the app header shows how many unread error/warning notifications the session has produced. Clicking it opens a panel (through the Modal shell — no native dialogs) listing the last 100 notifications newest first: kind glyph, title, message, relative time (the existing `skillPresentation` formatter — never a second one), per-entry copy, copy-all, and Clear. Opening the panel marks everything read. The reporter owns the ring (`notifications`, `unreadCount`, `markAllRead`, `clearNotifications`); it is session-only, in memory. Every direct `toast.*` call outside the reporter (the assignment matrix, projects page, skill card, skill detail view, the update checker) is replaced by a reporter call so the history is complete and durations are consistent; after this ticket the toast library is imported only by the reporter. CONTEXT.md gains a **Notification** glossary entry.

Source: `../spec.md` Q3, Q4; `../source-14-toast-lifetime.md` step 2. Skills: **impeccable** for the panel, **tdd** for the hook, **domain-modeling** for the glossary entry.

**Blocked by:** 01

- [x] Hook tests: ring bounded at 100 (oldest dropped); unread counts only error and warning; `markAllRead` zeroes it; `clearNotifications` empties it; `notify` both toasts and records
- [x] The toast library is imported from exactly one production module (the reporter hook)
- [x] Panel has EN + ZH copy, an empty state, and closes like every other Modal use
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — dee04b6. Evidence: Cited dee04b6 (integrated as 9b1976b) records the bounded history; src/hooks/useNotificationHistory.ts:80. Panel and toast routing followed in a8a8c47 and 0f9969e.

### 2026-09-04 — implementation (branch `r3/02-notification-history`)

**Shipped** (4 commits on top of `5401f4c`)

- `dee04b6` feat(reporter): the reporter owns the session history. `Notification = { id, kind, title,
  message?, at }` (Q4 shape; `id` is a per-session monotonic number). `notify` toasts and prepends
  into `notifications` (newest first, `.slice(0, NOTIFICATION_HISTORY_LIMIT = 100)`); `unreadCount`
  is derived — entries above an `id` watermark whose kind is error/warning; `markAllRead` moves the
  watermark; `clearNotifications` empties the ring and zeroes unread. Session-only, in memory (Q3).
  6 new hook tests in `useStatusReporter.test.ts` (records + toasts; setters land in history; ring
  bounded at 100 with the oldest dropped; unread counts only error/warning; markAllRead; clear). The two
  fully-typed mock reporters (`useAddSkillFlow`, `useSkillLibrary` tests) gained the four new members.
- `fb87c58` feat(ui): `Header` gets a bell (`icon-btn notif-btn`) with a red unread badge (caps at
  `99+`, aria-label carries the real count) next to the settings control; `App.tsx` owns
  `showNotifications` and `handleOpenNotifications` = `markAllRead()` + open. New
  `src/components/shared/NotificationsModal.tsx` through the Modal shell (title + ✕, backdrop/Escape
  close like every other use): kind glyph (lucide `CircleAlert`/`TriangleAlert`/`CircleCheck`/`Info`
  tinted by `--status-*`), title, message (`pre-wrap`), relative time via
  `skillPresentation.formatRelativeTime` with an absolute `title`/`dateTime`, hover-revealed per-entry
  copy (`title\nmessage`), footer `Clear` | `Copy all` (newest first, one line per entry with an ISO
  timestamp and `[kind]`, message indented beneath), both disabled when empty; an empty state
  ("Nothing to report yet" + one hint line). Copy success shows an inline check for 1.5 s instead of a
  toast (a success toast would insert an entry into the very list being read); copy failure goes
  through `notify("error", copyFailed)`. Styles in `App.css` (`.notif-*`, `.modal-notifications`).
  EN + ZH under `notifications.*`.
- `4784e72` refactor(ui): all 21 direct `toast.*` calls routed through `notify`. Route chosen for the
  projects world: **`notify` as a prop** — `App → ProjectsPage({ notify }) → AssignmentMatrix`,
  replacing `toast.x(msg)` with `notify("x", msg)` one-for-one (smallest diff; keeps
  `useProjectState` untouched so ticket 05's worktree does not collide). Skills world:
  `App → SkillsList → SkillCard` and `App → SkillDetailView`, same shape. `useUpdateChecker` now takes
  `{ reporter: Pick<StatusReporter, "notify" | "formatError"> }` like every other world hook (its
  doc comment updated: it still never touches the loading overlay). `NotifyFn` is exported from the
  reporter module as the prop type; components type-import it only.
- `341a889` docs(glossary): **Notification** entry in `CONTEXT.md`.

**Deviations / notes**

- "Toast library imported from exactly one production module": `App.tsx` still needed the `<Toaster>`
  mount, so the reporter module re-exports it as `NotificationToaster` and `App.tsx` imports that.
  `grep -rl sonner src --include=*.ts --include=*.tsx | grep -v .test.` → only `useStatusReporter.ts`.
  If the orchestrator would rather count the mount as a sanctioned second importer, reverting that one
  hunk is trivial.
- Unread is marked on **open** only (spec wording); an error raised while the panel is open counts as
  unread after closing. Deliberate — the operator may not have noticed it land.
- No plural keys were needed (`bellUnread` reads the same for 1 and n in both languages).
- Impeccable's detector over the changed files reports only two pre-existing findings in `App.css`
  (gradient brand text at line 100, a 3px side border at line 2716) — outside this ticket, left alone.
- The `lsp_diagnostics` tool resolves relative paths against the main checkout, so it could not see the
  new file; the typescript-7 build + eslint in the gate are the authoritative checks and are green.

**For the orchestrator**

- Operator smoke test (`tauri:dev`): raise an error (e.g. Update on a skill whose source is gone) →
  bell badge shows 1; open → badge clears, entry listed with "just now"; hover → copy; Copy all →
  paste shows ISO-stamped lines; Clear → empty state; Escape/backdrop closes.
- Gate: `Version OK (1.2.2)`; vitest 169/169 (12 files); cargo 431/431; clippy/rustfmt clean; build OK;
  working tree clean after `cargo test` (no bindings drift — no Rust change).
