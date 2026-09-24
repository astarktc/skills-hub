# 03 — Overwrite ask on `TARGET_EXISTS` (D3, BACKLOG #42)

Status: done — (this commit)
Spec: `.scratch/round17/spec.md` — D3.

## Work

- `src/hooks/useOverwriteConfirmation.ts` (building block, mirrors `useSharedDirConfirmation`):
  `OverwriteRow { skillId, skillName, toolKey, toolLabel, path }`, `pending: { rows, resolve } | null`,
  `request(rows): Promise<boolean>`, `cancel()`.
- `useSyncOrchestration.syncSkillsToTools`: after the batch, collect rows whose status is `failed` with
  `error.code === "TARGET_EXISTS"`. None → return. Else `setActionMessage(t("overwrite.waiting"))`, `request(rows)`;
  declined → return the report as is; confirmed → one more `syncSkillsToTools` invoke over the affected skills
  (from the original `skills` arg) × affected tool keys, policy `{ overwrite: false, overwrite_if_same_content:
  <caller's value>, overrides: rows.map(({skillId, toolKey}) => ({ skill_id, tool, overwrite: true })) }`, then
  return the first report with each asked `(skill_id, tool_key)` row replaced by the retry's row for that pair
  (rows the retry produced for other pairs are dropped — the report stays faithful to the operator's action).
  Expose `overwritePending` / `cancelOverwriteConfirmation` from the hook.
- `src/components/skills/modals/OverwriteModal.tsx`: Modal shell; body lists `skillName → toolLabel` with the
  path; footer Cancel / Overwrite. Not disabled on `loading` (it is shown while an action runs); backdrop class
  `modal-backdrop-over-loading` with `z-index: 2001` in `App.css`. Wire in `App.tsx` beside `SharedDirModal`.
- `useSkillLibrary.handleSyncSkillToAllTools`: pass `{ overwriteIfSameContent: true }`.
- i18n (`en` + `zh`): `overwrite.title`, `overwrite.body`, `overwrite.confirm`, `overwrite.cancel`,
  `overwrite.waiting`; reword `errors.targetExists` / `errors.targetExistsDetail` /
  `errors.syncTargetExistsMessage` so they no longer instruct "remove it and try again".
- Tests (`useSyncOrchestration.test.ts`): no `TARGET_EXISTS` → one invoke; present + confirmed → second invoke
  with the exact overrides and same-content flag, merged report; present + declined → one invoke, report
  unchanged; two skills × two tools with one occupied pair each → retry batch is the 2×2 with two overrides and
  only those two rows replaced.

## Comments

- 2026-09-23 — `useOverwriteConfirmation` + `OverwriteModal` (Modal shell gains `backdropClassName`; `.modal-backdrop-over-loading` z-index 2001); the seam splits into `invokeBatchSync` (the wire call) and `syncSkillsToTools` (batch → ask → retry with per-pair overrides → `mergeRetry`); `handleSyncSkillToAllTools` adopts the same-content rule; `errors.targetExists*` / `syncTargetExistsMessage` reworded (EN+ZH); `overwrite.*` keys with `_one/_other` plurals. Four seam tests (no ask / confirm merges exactly the asked rows / affected skills only with the caller's same-content flag / decline). vitest 397, eslint 0 warnings, build green.
