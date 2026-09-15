# 14 — Toasts vanish too fast; no notification history

**Status:** triaged → round 3 (`spec.md`); this file is a read-only source reference
**Reported by:** operator, 2026-09-03, while smoke-testing v1.2.2 (pre-existing behaviour, not from round 2).

## Problem

Toasts are the only surface for action outcomes, and they auto-dismiss in 1.8–3.2 s:

- `src/App.tsx:194` `<Toaster … toastOptions={{ duration: 1800 }} />`
- `src/hooks/useStatusReporter.ts:138` error `duration: 3200`; `:145` success `1800`; `:155` error `2600`
- ~15 direct `toast.error/success/warning` calls in `components/projects/*` use the 1.8 s default.

An install/refresh **error** (a batch report with per-target failures, a `CommandError` with `detail`) is
gone before it can be read, and there is nowhere to look it up afterwards. The backend writes an app log
(`tauri-plugin-log` → `LogDir`, `lib.rs:75`), but it is not surfaced in the UI and a report-level failure
that the frontend rendered from data (not an error the backend logged) never reaches it.

## Direction (to settle before implementing)

1. **Errors should not auto-dismiss.** sonner supports `duration: Infinity` + `closeButton`; keep success
   at ~2 s, warnings ~5 s. Cheapest fix, ship first.
2. **A notification history.** An in-app "Activity" panel (bell icon / footer strip) holding the last N
   reporter entries (`ActionErrorEntry` already has `title` + `message`; success/warning entries would be
   added) for the session, with copy-to-clipboard. Persisting across restarts is optional — if wanted,
   the natural home is a small `notifications` table in the app DB or a JSONL beside the log file.
   All entries go through `useStatusReporter`; the direct `toast.*` calls in `components/projects/*`
   should be routed through it so the history is complete (also fixes the current inconsistency in
   durations).
3. **"Open log folder"** action in Settings pointing at the `tauri-plugin-log` `LogDir` — zero-risk,
   independent of 1–2.

Out of scope: changing what the backend logs.
