# Wave B review disposition

Initial single-seat review: `.scratch/round10/r10b-review-opus.md`, exact HEAD 960e5d5, baseline a824caf. Anthropic Opus 5 high returned OK with notes, 0 declared blockers; 4 Standards and 3 Spec findings. Parent does not treat reviewer severity as acceptance authority.

## Standards

1. Listing intent accepted by byte acquisition but refused after clone: **valid non-blocker, deferred**. No production caller can reach it; Q20 explicitly selected optional Selection for listing. A different public intent shape is a design change, not necessary release cleanup. Revisit before a new caller uses that door.
2. Update command delegates existing async batch command: **valid coupling observation, deferred**. It is wiring-only reuse and ticket05 expressly permits it; no reentrant core lock or copied policy. Keep scope rather than invent another orchestration abstraction to satisfy style.
3. `refreshProgress` factory naming: **valid low-priority clarity note, deferred**. Behavior unambiguous at current two call sites; no correctness change needed.
4. Corpus equates Rust header presence with TS metadata presence: **valid test-contract defect, fix in same Manifest follow-up**. Empty/comment-only header must have explicit independent per-language expectations; do not change production fallback policy to force artificial equality.

## Spec

1. Pre-upgrade Edit under indented opening cannot clear safely: **valid current-scope compatibility defect; parent blocks completion pending fix**, despite review's follow-up label. Ticket04 promises persisted Edit bases clear/replay byte-for-byte. New strict grammar makes restore no-op while core still deletes row; new re-choose/replay must not strand old forced keys or synthetic headers. New Astra high isolated fix from 960e5d5, regression-first literal old JSON+bytes through real temporary store. Preserve strict NEW grammar; allow only persisted-base-backed compatibility. No operator data/schema/startup migrations. Focused Opus re-review after integration required.
2. Q5 user-visible picker/name-match consequence omitted in changelog: **valid cosmetic, parent clarified**. Add chooses among root+nested; name-matched Explore can find nested. No behavior change.
3. Missing changelog releases 1.2.7–1.2.11: **pre-existing, deferred**. Backfilling historical release notes is outside this wave; do not fabricate history during final release prep.

## Fix lane

- Checkout `~/Projects/skills-hub/.scratch/round10/worktrees/review-fixes`, branch `r10b/review-fixes`, base 960e5d5, sole writer Astra high. node_modules/target isolated APFS clones; Cargo jobs4.
- Task suffix `r10b-fix-legacy-edit-960e5d5-v1` under common execution-board task prefix. Async running.
- Claims Manifest implementation/tests, narrow Edit compatibility wiring/tests, shared TS corpus/tests. No git resolution/envelopes/sync/AGENTS/CONTEXT/changelog/version changes. Own branch commit permitted; parent integration/publication authority.
- Required original-red/final-green/source-reversal proof, clear + re-choose→clear + replay→clear, fresh invalid-header strict behavior, unchanged rollback/typed failures, full gates.

## Final outcome

Fix integrated as c1ff64c after rebase; compatibility invariant 23458bb. Parent verified six-file diff and actual failing-byte/source-reversal proof, and historical writer fixture proof. Strict new grammar preserved; legacy clear/re-choose compatibility gated by persisted base; upstream replay strict/current-base. Corpus independent fields and empty/comment/name-missing cases added. Child full gate640 Rust/347 TS; parent repeated at23458bb: all definition-of-done gates + explicit cargo --all pass640/347, bindings and worktree clean. Focused Opus re-review COMPLETED OK with notes at23458bb (`r10b-review-opus-recheck.md`), accepted SPEC1/STANDARDS4/SPEC2 resolved and no new regressions. Parent accepts; remaining STANDARDS1/2/3 and pre-existing SPEC3 deferred as above. All tickets resolved; all child tasks terminal. Parent repeated full640/347 gate after removing clean merged worktrees/branches; evidence archived locally. Nothing pushed/tagged/published. No live app smoke test. Residual grammar consequence: an already-malformed indented header remains invalid to normal reads; existing saved Edits remain reversible, while fresh upstream replay uses strict/current-byte semantics. This is compatibility preservation, not a general malformed-manifest repair feature.
