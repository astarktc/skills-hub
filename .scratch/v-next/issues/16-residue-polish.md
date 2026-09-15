# 16: Residue polish — mechanical cleanups, docs, CI, dead command

Status: resolved

Type: task
Blocked by: 10, 12, 13, 15

## What to build

The verified tier-3 residue from the epic review (locations as of `be9a74c`; several files move under tickets 10–15, hence the blockers — re-locate by content, not line number):

**Code:**
- Redundant nested `{ }` blocks inside `catch` clauses: `useSkillLibrary.ts` (~:200), `useSyncOrchestration.ts` (~:240), `useExploreState.ts` (~:87, ~:106).
- Modal-conversion JSX left two indent levels too deep: `App.tsx` update modal, `shared/ToolConfigModal.tsx`, `DeleteModal.tsx`, `EditProjectModal.tsx`.
- `useProjectState.ts::normalizeError` — its value is stored in `loadError` but never rendered (`ProjectList` shows static copy); simplify to what's actually consumed (a boolean or render the value — your call, keep it honest).
- `shared/ToolConfigModal.tsx` hardcoded ` (installed)` badge → use the existing `status.installed` key (EN "Installed" / ZH "已安装").
- `useSyncOrchestration.ts` (~:109) hardcoded `"、"` (Chinese enumeration comma) joining tool labels for all locales → locale-aware separator (i18n key or `Intl.ListFormat`); also collapse the duplicated "relevant newly-installed" filter (memo at ~:93 vs local `relevantNew` at ~:155).
- `useSyncOrchestration.test.ts` `FakeChannel<unknown>` erases DTO typing — a fixture passes `skill_id`, which doesn't exist on `SyncProgressDto`. Type it `FakeChannel<SyncProgressDto>` and fix the fixture.
- **Delete the dead `search_github` command**: `core/github_search.rs`'s `RepoSummary` return, the command in `commands/mod.rs`, and its `generate_handler!` registration in `lib.rs`. Verified born-dead: present since the initial upstream import, zero frontend callers in the repo's entire history (`git log -S search_github -- src/` is empty). Follow the `sync_skill_dir` deletion precedent from ticket 07. Remove whatever in `github_search.rs` becomes unreferenced.

**CI/docs:**
- `.github/workflows/ci.yml` bindings guard: `git diff -- ../src/bindings` lacks `--exit-code` (can't fail); add it and make the comment describe the real two-part mechanism.
- AGENTS.md: "hooks never import each other" → "no *runtime* imports between hooks; type-only `Pick<>`-narrowed dep imports are the pattern" (resolves a reviewer-confusing ambiguity).

## Acceptance criteria

- [x] Every bullet addressed (or explicitly noted as already-fixed by an earlier ticket).
- [x] `search_github`/`RepoSummary` fully gone, no dangling references.
- [x] `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `58e86ba` (Fable 5 low subagent; orchestrator-verified, rebased, combined-tree gate green). Every bullet fixed: 4 nested catch blocks removed; 4 modal files re-indented (whitespace-only, `git diff -w` clean); `normalizeError` dead pipeline → honest `loadFailed: boolean`; badge → `({t("status.installed")})`; `、` → `common.listSeparator` (EN ", " / ZH "、") + one `filterRelevantNewlyInstalled()`; `FakeChannel<SyncProgressDto>` restores fixture enforcement; `search_github` deleted whole (command, registration, module, tests, `core/mod.rs` decl — `urlencoding` kept, still used by `skills_search.rs`); orphaned `noSkillsFoundInRepo` keys removed; CI diff line got `--exit-code` + honest comment; AGENTS.md hooks rule reworded to the runtime-imports form. Net −184 lines.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
