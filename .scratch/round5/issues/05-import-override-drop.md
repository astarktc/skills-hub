# 05: Import relies on the same-content policy alone

**What to build:** The onboarding import no longer forces `overwrite: true` per byte-identical Tool. First verify that the identity predicates agree: `is_identical_to_chosen` (fingerprint equality) and the batch's `overwrite_if_same_content` (`target_has_same_content`, directory hash). If they agree for every identical original, the per-tool overrides are removed and the sync batch runs on `overwrite: false` + `overwrite_if_same_content: true`; the TOCTOU window between plan and force vanishes with them. If they can disagree (e.g. fingerprint ignores something the hash includes), make them one predicate first, then drop the overrides. A test proves a *divergent* original is still never overwritten, and an identical original is still taken over.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] Written finding in this ticket's Comments: do the two predicates agree, and why
- [x] `BatchOverride` construction removed from `onboarding_import.rs` (the `forced_tools` extension of `tools` stays if still needed for identical originals whose Tool is not selected — decide and record)
- [x] Tests: identical original taken over; divergent original kept (`KeptDivergent`), central untouched
- [x] `npm run version:check && npm run check` green

## Orchestrator notes

- `onboarding_import.rs:~308–345`; `is_identical_to_chosen` at ~376; `global_sync.rs:80` `target_has_same_content`, policy applied at ~128. Fingerprint source: the onboarding scan (`onboarding*.rs`).
- If `BatchOverride` becomes unused anywhere, clippy `-D dead-code` will fail — remove or `#[cfg(test)]` accordingly, but check it isn't used by the sync command's per-(skill,tool) overwrite decisions first (AGENTS.md says it is).

## Comments

### Implementation — r5/05-import-override-drop

**Finding:** Both fingerprints (`onboarding.rs`) and `target_has_same_content`
(`global_sync.rs`) call the very same `content_hash::hash_dir`. They share its
ignore list (`.git`, `.DS_Store`, `Thumbs.db`, `.gitignore`) and traversal rules;
there is no second, narrower fingerprint algorithm. However, the predicates
were not equivalent: `is_identical_to_chosen` compared plan-time fingerprints
and unconditionally accepted the chosen path, whereas the batch compares live
original/central hashes and fails closed on missing targets or hash errors.
An original changed after planning could therefore be force-overwritten.
Import now calls `target_has_same_content` against the finalized central copy
for classification too; the cached/path-equality predicate is removed.

**Shipped:** `71f2a54` (`fix(import): rely on live same-content checks instead of overrides`)
removes import's `BatchOverride` construction and uses `overwrite: false`,
`overwrite_if_same_content: true`, empty overrides. `forced_tools` and its
extension of `tools` remain: a deselected Tool holding an identical original
must still receive a tracked Sync target. The command's explicit per-target
operator decisions still use `BatchOverride` (`commands/mod.rs`), so the type
remains unchanged. A separate documentation commit records these findings.

**TDD:** The new public import-seam test edits an initially identical sibling
at the Applying progress callback, after planning. Before the fix it failed:
the external edit was replaced with the chosen bytes. After the fix the edit
survives, is reported `KeptDivergent`, the selected divergent target fails
without replacement, the central copy retains the chosen bytes, and the
identical deselected source Tool is taken over. Existing real-dir/link,
shared-dir, divergent-sibling and forced-inclusion coverage remains green.

**Deviations:** No scope deviation. Chose the ticket's predicate-unification
branch because temporal identity and the unconditional chosen-path shortcut
can disagree even though the hash algorithm is shared. No DTO/UI changes,
version bump, dependency fixes, or changes outside the assigned worktree.

**Fresh gates:** `npm run version:check && npm run check` green (version 1.2.4;
lint; 214 Vitest tests in 13 files; production build; rustfmt; clippy with
`-D warnings`; 518 Rust tests). Separate `cargo test --all`: 518 passed,
0 failed. Focused import suite: 14 passed. Primary LSP: 2 files, 0 diagnostics.
Logs are local under `.scratch/round5/{red,green,gate,all}-05.log` (not committed).

**Follow-ups / limits:** None required for this ticket. Removing the plan-time
force authorization does not make filesystem comparison and replacement atomic
against external processes; the existing batch's hash-to-write interval remains.
The process mutation guard and unlocked batch seam are unchanged. `npm ci`
reported 7 existing dependency audit findings (1 moderate, 6 high), left outside
this ticket's scope.
