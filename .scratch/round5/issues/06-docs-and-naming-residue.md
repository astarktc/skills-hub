# 06: Glossary, naming and comment residue

**What to build:** (1) CONTEXT.md **Acquisition** paragraph records the limitation: the API fast path follows an upstream link only when it is the leaf of the subpath; a link on a path component is a typed not-found (not retried as a clone). (2) CONTEXT.md **Unlocatable skill** records the Re-point validation rule: the new folder must be a skill folder and must not sit inside a Tool's skills dir (that is Import's door). (3) `legacy_reclassification.rs`: if `tool_owning_path` is a wrapper of the registry's `tool_holding_path`, delete it and call the registry; rename `tool_shaping_path` → `tool_by_home_shaped_prefix`. (4) The five `spec Qn` comments in `useStatusReporter.test.ts`, `useNotificationHistory.ts`, `App.tsx` state the rule inline ("errors never auto-dismiss", "opening the history marks it read", "copying is a courtesy, not an outcome"); the two `v-next ticket 38` citations stay.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] CONTEXT.md two additions, one sentence each, in glossary voice
- [x] `legacy_reclassification` tests green after the helper change; `rg "tool_owning_path|tool_shaping_path"` empty
- [x] `rg "spec Q[0-9]" src src-tauri/src` empty
- [x] `npm run version:check && npm run check` green

## Orchestrator notes

- `legacy_reclassification.rs:97,108,115,140,156`; registry fn `tool_adapters::tool_holding_path(home, path) -> Option<&ToolAdapter>` at `mod.rs:709`.
- Comment sites: `useStatusReporter.test.ts:33,113,225`; `useNotificationHistory.ts:8`; `App.tsx:142`.
- Sibling ownership: ticket 01 widens one Re-point sentence in **Unlocatable skill** — put your validation-rule sentence at the end of that entry so both hunks are additive.

## Comments

- Shipped in `5a271b4` (`refactor: clarify glossary and remove naming residue`): one sentence each for Git acquisition's API leaf-link limitation and local Re-point validation; direct registry lookup and the home-shaped-prefix helper name; all five notification comments state their rules without spec citations. Both ticket-38 citations remain unchanged.
- Deviations: no scope changes. The deleted wrapper also excluded the Tool's skills directory itself; that filter stays inline at the registry call site to preserve behavior. The local Re-point sentence is on a separate line at the end of its definition to minimize overlap with ticket 01.
- Gates: `npm run version:check && npm run check` passed (version 1.2.4; ESLint; 13 Vitest files / 214 tests; TypeScript-7 + Vite build; rustfmt; clippy with warnings denied; 517 Rust tests). `cargo test --all` separately passed all 517 Rust tests, including all 7 legacy-reclassification tests; binary/doc targets had 0 tests. Initial rustfmt failure was corrected before the successful full rerun.
- Verification: old helper names and spec-Q citations have no matches in `src` or `src-tauri/src`; both ticket-38 citations remain; `git diff --check` and primary LSP diagnostics passed. Full gate output is in `.scratch/round5/06-check.log`; all-target output is in `.scratch/round5/06-test-all.log` (local evidence, not committed).
- Follow-ups: none for this ticket. Existing npm audit findings (1 moderate, 6 high) and Vite chunk/dynamic-import warnings remain outside scope.
