# 15: useAddSkillFlow dedupe — one canonical install→deploy path

Status: resolved

Type: task
Blocked by: None (can start immediately)

## What to build

Verified review findings (locations as of `be9a74c`), all in the add-skill flow:

1. The ~22-line "install → auto-sync to selected installed targets" block is copied **5×** in `src/hooks/useAddSkillFlow.ts` (5 `noSyncTargets` occurrences — the `if (autoSyncEnabled) { getSelectedInstalledIds() … syncSkillsToTools([created…]) … syncFailureEntries … }` shape). Extract one `deployNewSkill(created, …)`-style helper used by all five call sites.
2. `handleInstallSelectedLocalCandidates` and `handleInstallSelectedCandidates` are ~55-line near-twins differing only in command name/args and which state they reset. Merge onto a shared core.
3. The inline hand-mirrored DTO at ~:291 (`invokeTauri<{ skill_id: string; central_path: string }>("import_existing_skill", …)`) must use `InstallResultDto` — already imported at line 4 of the same file and used by the sibling calls.
4. The `{skill_id, name, source_path}` batch-item mapping is copied 4× in `src/hooks/useSkillLibrary.ts` (~:134/:220/:262/:362) — extract a small helper there too.

Strictly behavior-preserving: the existing vitest suite (`useAddSkillFlow.test.ts`, `useSkillLibrary.test.ts`) must pass unchanged (extend coverage if the helper earns it, but don't rewrite tests to fit).

## Acceptance criteria

- [x] One helper owns the install→deploy shape; ≤1 `noSyncTargets` reference in `useAddSkillFlow.ts` production code.
- [x] Twin install functions share their core; inline DTO replaced by `InstallResultDto`.
- [x] `useSkillLibrary` mapping helper in place.
- [x] Existing tests pass unmodified; `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `f0149de` (Fable 5 low subagent; orchestrator-verified, rebased, combined-tree gate green). `deployNewSkill(created, { noTargets: "set-error" | "collect" })` is the one canonical install→auto-sync tail (the mode captures the only real per-site difference); `runBatchInstall` shares the twin handlers' core; inline `{skill_id, central_path}` → `InstallResultDto`; module-local `toSyncItem` helpers in both hooks (no cross-hook import). Net −80 lines; 50/50 vitest unchanged. `handleImport`'s sync block deliberately stays inline — not one of the 5 duplicated sites (custom overrides/error mapping).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
