# 05: Import takes over every byte-identical original in the group; the default variant prefers a real directory

Status: done — 7a63d32

**What to build:** Importing a skill that exists in two Tools — one a real directory, the other a symlink to it — chooses the real directory as the source by default, and with auto-sync on leaves **both** Tools holding a link to the central copy: every Tool whose variant is byte-identical to the chosen one is force-included in the target set and overwritten in place (the round-3 rule widened from "the chosen variant's Tool" to "every identical variant's Tool"). A divergent sibling is left in place and reported, exactly as the auto-sync-off path already does. The success toast carries one line per Tool included beyond the policy. The operator's smoke-test case (`plane-fallback`: real dir in Pi — deselected — linked from Claude) ends with Pi holding a link, not an untracked copy, and the import dialog no longer lists it.

Source: `../spec.md` Q3, Q4 and the "Registry order" fact; operator smoke test 2026-09-05; round-3 ticket 07 (the rule being widened).

**Blocked by:** None (can start immediately)

- [x] Core test: two Tools hold the same bytes (one real dir, one link to it), the real dir's Tool is deselected, auto-sync on → both Tools end with a link to central, a target row exists for each, the report lists the deselected Tool as included beyond the policy
- [x] Core test: a divergent sibling in a third Tool is left untouched and reported
- [x] Core test: policy names every Tool → report lists nothing beyond the policy (unchanged behaviour)
- [x] Hook test: for a consistent group with a link variant listed before a directory variant, the selection payload names the directory
- [x] The report field becomes a list of Tool keys (DTO + regenerated binding); the toast composes one line per key (EN + ZH copy unchanged or extended)
- [x] CONTEXT.md **Onboarding import** entry states the widened rule
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 7a63d32. Evidence: Cited 7a63d32 integrated as 04024f0; 134f6ff/86e330e adds real-directory preference and forced-tool copy. src-tauri/src/core/onboarding_import.rs:342–347 includes every identical Tool; src-tauri/src/core/tests/onboarding_import.rs:333 covers takeover.

### 2026-09-05 — implementation (branch r4/05-import-takes-over-identical-originals)

**Shipped** (3 commits on top of main `f057190`):

- `7a63d32` feat(import): take over every byte-identical original in the group — `core/onboarding_import.rs`
  `sync_imported_unlocked` now builds `policy.tools ∪ {Tools of every variant with the chosen variant's
  fingerprint}` (identical Tools appended *after* the policy's, in variant order, so the shared-dir dedupe
  is unchanged for the in-policy case), one force-overwrite `BatchOverride` per identical Tool; variants
  with a different fingerprint are reported as `OriginalOutcome { KeptDivergent }` in `originals` (the
  auto-sync-on path used to leave `originals` empty). `ImportGroupStatus::Imported.forced_source_tool:
  Option<String>` → `forced_tools: Vec<String>`; `ImportGroupStatusDto` follows; `src/bindings/index.ts`
  regenerated and committed. Three new core tests + the two existing forced-tool tests updated.
- `134f6ff` feat(import): default variant prefers a real directory; toast lists every forced Tool —
  `defaultImportVariantPath(group)` in `src/lib/skillPresentation.ts` (5 tests) is the single rule;
  `useAddSkillFlow` uses it in both `fetchPlan`'s default and `handleImport`'s fallback;
  `importSuccessToast` emits one `status.importSourceToolForced` line per key. Two hook tests (one
  replaces the round-3 single-key toast test).
- `eea0ef3` docs(context): Onboarding import entry restated with the widened rule and the default-variant rule.

**Gate**: `npm run version:check` OK (1.2.3); `npm run check` green — vitest 178/178 (12 files), build,
fmt, clippy `-D warnings`, `cargo test` 448/448. `cargo test --all` 448/448 (run three times; one run hit
the known elapsed-time flake in `core/tests/refresh.rs:375` `acquisitions_overlap_instead_of_running_one_at_a_time`,
which is ticket 03's item #4 — not touched here).

**Deviations**: none from the ticket. Two judgment calls worth knowing:
1. Identity rule: a variant is "identical" when it *is* the chosen path, or when both fingerprints are
   `Some` and equal. A variant whose hash failed (`fingerprint: None`) is never assumed identical — it is
   reported `KeptDivergent` rather than overwritten. Conservative by design.
2. With auto-sync on, a divergent sibling is reported `KeptDivergent` regardless of whether the policy
   names its Tool. If the policy does name it, the batch *also* reports that target as `TARGET_EXISTS`
   (unchanged behaviour), so the operator sees two entries for that Tool — one per aspect (sync failed /
   original kept). Left as is: both are true, and changing target reporting is out of this ticket.
3. `defaultImportVariantPath` applies the real-dir preference only to consistent groups (Q4 verbatim);
   a conflicting group keeps its first variant for the operator to resolve.

**Notes for the orchestrator**:
- i18n: `status.importSourceToolForced` EN/ZH copy unchanged — it reads correctly per line. No new keys.
- Wire contract change: `forced_source_tool: string | null` → `forced_tools: string[]` on
  `ImportGroupStatusDto`. Ticket 06 (blocked on this) touches `install_local_skill` / what import
  *records*; nothing here touches provenance or the record.
- Files touched: `CONTEXT.md`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/core/onboarding_import.rs`,
  `src-tauri/src/core/tests/onboarding_import.rs`, `src/bindings/index.ts`, `src/hooks/useAddSkillFlow.ts`
  (+test), `src/lib/skillPresentation.ts` (+test). None of the round-3 high-risk shared files besides
  `commands/mod.rs` (the DTO + one match arm only).
