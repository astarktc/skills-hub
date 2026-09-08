# 03: Error-contract rule, typed symlink depth bound, i18n key maps

**What to build:** Three settled review items. (1) AGENTS.md **Error wire contract** bullet gains the rule: *a guard against a condition the UI cannot produce may bail with prose (it surfaces as `OTHER`); a condition the operator or upstream can cause is a typed variant.* (2) Under that rule the `LinkChain` depth bound in `repo_subpath.rs` (an upstream repo with too deep a symlink chain — operator-visible on Refresh) becomes typed: a new `SignalError` variant carrying the subpath, its `CommandError` mirror, regenerated bindings, a `describeCommandError` branch, EN + ZH copy; `unlocatable::require_local` stays prose (UI cannot reach it). `{{target}}` interpolation in `symlinkEscapesRepo`/`pathOutsideToolDirs` stays as is. (3) The `unlocatable.*` i18n keys become camelCase and `SkillCard` stops building keys from wire-enum spelling: `skillPresentation.ts` exports `Record<UnlocatableState, string>` (state → key, and tooltip key) and a repair → key record; `useSkillLibrary`'s `SKIPPED_REASON_KEY` moves there too so the mapping lives once.

**Blocked by:** None (can start immediately)

**Status:** complete

- [x] Core test: a link chain longer than the bound raises the new typed variant (extend `repo_subpath` tests); commands wire test pins its tag and payload
- [x] `describeCommandError` branch + EN/ZH keys; `commandError.test.ts` covers it
- [x] No `t(\`unlocatable.${…}\`)` template keys remain; the maps are the only place state/repair meet i18n keys; `skillPresentation.test.ts` covers the maps' exhaustiveness (type-level `satisfies Record<…>` is enough plus one runtime assertion)
- [x] AGENTS.md rule added (one sentence in the Error wire contract bullet)
- [x] `npm run version:check && npm run check` green

## Orchestrator notes

- `repo_subpath.rs` bails at ~lines 75/88/100/107; `SymlinkEscapesRepo` is the typed sibling — mirror its plumbing (`errors.rs`, `commands/error.rs` `from_anyhow` arm, `describeCommandError`, `resources.ts` both locales). Worked example in AGENTS.md: `PATH_OUTSIDE_TOOL_DIRS`.
- `SkillCard.tsx:95,190,194` build the template keys; `useSkillLibrary.ts:20` has `SKIPPED_REASON_KEY`.
- Sibling ownership: ticket 01 also touches `SkillCard.tsx`/`useSkillLibrary.ts`/`resources.ts` (adds Re-point keys) — keep your hunks to the unlocatable block and the new error key so the rebase is additive. Ticket 02 owns the AGENTS.md `skillPresentation` bullet; you own the Error wire contract bullet only.

## Comments

- Shipped in `f0f8805` (`fix: type symlink depth failures and centralize i18n maps`): `SymlinkChainTooDeep { subpath }` flows through the command seam as `SYMLINK_CHAIN_TOO_DEEP`, with generated bindings and localized EN/ZH copy. Presentation owns exhaustive state, tooltip, repair and Refresh-skipped maps; SkillCard and useSkillLibrary consume them. Added the one-sentence AGENTS.md error rule.
- Deviations: none. `unlocatable::require_local` remains prose; existing escape/path interpolation is unchanged. No version bump, push, merge, rebase or live-library mutation.
- TDD: the core and wire tests first failed for missing typed plumbing; the frontend error and map tests failed on the old behavior, then passed. Targeted frontend run: 84 tests passed.
- Full gates: `npm run version:check && npm run check` passed (version 1.2.4; ESLint; 13 Vitest files / 216 tests; TypeScript-7 + Vite; rustfmt; clippy; 518 Rust tests). Separate `cargo test --all`: 518 passed, zero failures. LSP checks and final pi-lens session diagnostics reported no issues.
- Goal-backward checks: all 10 presentation/error catalog keys resolve in both EN and ZH; the typed error renders localized copy plus the structured subpath. No template-built or snake_case `unlocatable` translation keys remain. Bindings were generated, not hand-edited.
- Non-blocking tooling notes: npm ci reported 7 dependency audit findings; Vite reported chunk-size/mixed-import warnings. Generated Specta bindings retain the generator's existing trailing-space style. The design-context launcher was permission-denied; no visual changes were made.
- Follow-ups: no ticket-specific follow-ups. Dependency audit and bundle warnings remain outside this ticket's scope.

