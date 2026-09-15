# 07: Re-point from the "not found on GitHub" failure Notification

Status: done — 378a473

**What to build:** When Refresh reports a git skill as not found on GitHub, the error row in the notification panel (and its toast) carries a **Re-point** action that opens ticket 01's URL Modal for that skill. A `Notification` gains an optional action (label + handler; notifications are session-only, so a closure is fine). Other notifications are unaffected.

**Blocked by:** 01

- [x] `useStatusReporter` test: an `ActionErrorEntry` with an action renders it on the recorded Notification and the toast
- [x] `useSkillLibrary` test: `skillFailureEntries` attaches the action only for `GITHUB_SKILL_NOT_FOUND` failures
- [x] EN + ZH label; `npm run version:check && npm run check` green

## Orchestrator notes

- `useNotificationHistory.ts:11` `Notification`; `useStatusReporter.ts:19` `ActionErrorEntry`, `showActionBatch` ~291; `useSkillLibrary.ts` `skillFailureEntries`. Keep the action optional so every existing caller compiles unchanged.

## Comments

- 2026-09-15 — Status reconciliation: done (378a473) → done — 378a473. Evidence: Preserve cited done commit 378a473, integrated as 9fa3e4c; src/lib/reportOutcome.ts:72–76 attaches the ID-based GitHub-not-found repair action (subsequently moved from the hook).
- 2026-09-15 — Previous status wording (historical, not a current merge/publication claim): done (378a473)

(Reconstructed by the orchestrator from the child's final summary — the child's own uncommitted Comments were lost when its worktree was removed.)

- Shipped `378a473` (`feat(notifications): offer git re-point from refresh failures`): `NotificationAction = { label, onClick }` optional on `Notification` and `ActionErrorEntry`; `showToast` passes it to sonner's `action`; `record` stores it; `NotificationsModal` renders a small button; `skillFailureEntries` attaches it only for `GITHUB_SKILL_NOT_FOUND` on a managed `git` skill, calling `handleRepointGitSkill(managedSkill)`.
- Reused existing EN/ZH label `gitRepoint.action`; no scope deviations.
- Gate: vitest 222, cargo 528, build/lint clean. Visual smoke test not performed.
