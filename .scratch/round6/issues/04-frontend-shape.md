# 04: frontend follow-ups (#5 #6 #7 #8 #9)

Status: done — 70651c8

**Lane:** F1  **Files:** `src/hooks/useSkillLibrary.ts` (+ test), `src/hooks/useStatusReporter.ts` (doc only),
`src/App.tsx`, `src/components/skills/SkillCard.tsx`, `src/components/skills/modals/GitRepointModal.tsx`,
`src/lib/skillPresentation.ts` (+ test), `src/i18n/resources.ts`. No Rust.

## A — stale notification action after delete (spec D5)
`skillFailureEntries` (`useSkillLibrary.ts` ~148) captures the `ManagedSkill` object in the Re-point action closure; after
the skill is deleted the panel action still opens the modal and then fails `NOT_FOUND`.
→ Capture the **id**; on click resolve against the latest `managedSkills` (keep a ref updated from state); if missing,
`notify({ kind: "warning", title: t("errors.skillGone", { name }) })` — add the key to **both** `en` and `zh` — and do not
open the modal. Hook test: action clicked after the skill vanished → warning notified, no pending modal.

## B — toast action is head-only (spec D6)
Behaviour stays. Update `showActionBatch` / `showActionErrors` doc in `useStatusReporter.ts`: the toast carries the head
entry's action; every entry's action lives in the notification panel.

## C — GitRepointModal lifecycle (spec D7)
`handleConfirmRepointGitSkill` clears `pendingGitRepointSkill` before invoking, so `loading`/`closeDisabled` on the modal
are dead. → Keep the modal open through the action: clear the pending skill on **success only**; on failure it stays with the
operator's URL. `loading` (already passed from App) drives spinner/disabled. Check `runAction`'s return/`ActionExit`
contract to detect success without a second seam. Hook test for both outcomes.

## D — `detailSkill` ownership (spec D8)
`App.tsx:64` holds a `ManagedSkill` object and re-finds it inline in JSX (~250). → `useSkillLibrary` owns `detailSkillId`
and exposes derived `detailSkill` (selected from `managedSkills`; `null` when gone) plus `openDetail(id)`/`closeDetail()`;
App only renders. Keep the Explore-install handoff at App.tsx ~184 working.

## E — "one repair, two provenances" decided once (spec D9)
`SkillCard.tsx:278` (`kind === "git"` → always-available Re-point button) and `handleRepointSkill` (`sourceKind` → door)
decide the same thing twice. → One pure predicate in `src/lib/skillPresentation.ts` (e.g.
`repointDoor(skill): "git" | "local"`) with a unit test; both call sites use it.

## Gate
`npm run lint && npm run test && npm run build` (no Rust changes; if you must touch bindings you've gone wrong).
Commit on your branch with one conventional title per part or one combined — your call. Paste `## Comments` in the final message.

## Comments

- 2026-09-15 — Status reconciliation: ready → done — 70651c8. Evidence: git log v1.2.5..v1.2.6 finds first slice 70651c8; cf47020,7b760ec,2379c50 complete modal/detail/repair ownership. src/hooks/useSkillLibrary.test.ts:403,596 retain stale-action/detail behavior; the tautological repointDoor was intentionally removed later in d24b6bf.
