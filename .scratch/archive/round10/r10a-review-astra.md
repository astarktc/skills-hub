## Standards

**Blocking: none verified at `300f89bbd63b632eb81377f437478a11ca797dfa`.** Compared with `7a7c519` (v1.2.9).

The changed paths follow AGENTS.md's explicit-roots, guarded-mutation, typed-invoke, generated-binding and bilingual-copy rules. ADR-0004's persisted-row snapshot and replay rollback remain intact. Edit target failures now cross the command seam as report data; intentionally ignoring that report in the UI is authorized by Q13/Q18, not an overlooked wave-A breach.

**Depth / deletion test:**
- `core/content_identity.rs` earns its three doors: deleting it redistributes stored-vs-computed identity, backfill and unknown-identity policy across finalize, Edit, reconcile, sync and onboarding. `Source::Directory` is justified for unmanaged candidates.
- `core/skill_update.rs:111–163` owns real admission and current-record settlement, not merely the relocated tail. Deleting it duplicates those rules between Refresh/Re-point and Edit. Keeping byte rollback inside finalize and Edit-row rollback inside replay preserves their locality.
- `src/lib/reportOutcome.ts` earns its interface: deleting it restores interpretation and completion policy in several hooks. The trivial `deleteOutcome` alone is shallow, but explicitly enumerating that command's null result is reasonable within the requested interface.

**Follow-up — judgement, possible Data Clumps / Primitive Obsession:** `core/skill_update.rs:43–48` exposes two entire `SkillRecord`s plus an independent `repoint: bool`; callers can construct inconsistent proposals or accidentally bypass admission. Current constructors are correct. Consider private request fields and explicit Edit/Re-point constructors so callers cannot manufacture the admission protocol. This is interface hardening, not a demonstrated safety bug.

`ACQUISITION_SKIP_KEY` beside the existing skipped-state presentation maps is a defensible home under AGENTS.md's one-presentation-implementation rule; no second formatter or policy was introduced.

## Spec

**Blocking: none verified.**

B1: `hash_dir` has no external caller; Propagation reads identity once. SQL-trigger coverage proves one backfill; failed reads yield `None` and preserve `obs.current`. The manifest-temp prefix is folded into the common ignore predicate. Onboarding's migration is justified by Q2/Q15's single identity supplier, not unrelated scope.

B2: admission reads the current row under the guard and compares all three required provenance fields; explicit Re-point bypasses mismatch, never deletion. Skips drop owned staging. Local acquisition remains outside the guard. No acquisition upserts a skill. Failed local Re-point preserves row and bytes; Edit returns failed targets; replay remains inside finalize's rollback window. Refresh retains orchestration/reassert; installer retains Add, listing, preview and backfill.

F1 deliberate operator-visible changes versus v1.2.9, all **correct**:
- Conflict now wins even alongside failures/skips; failure wins over skip. Details remain available. Target/reassert failures produce warnings rather than successful completion.
- Single-action failures become named report entries, with eligible Git repair actions, plus warning summaries; skips receive warnings. Local Re-point gains conflict-aware summaries and reload on thrown requests.
- Empty/skipped reports no longer close Git repair. Successful central settlement still closes it despite target failure/conflict—consistent with the previous behavior. Repair selection follows current rows; deletion hides it and warns instead of retaining a stale DTO.
- Bulk sync failures suppress the success toast. Toggles batch all report failures instead of surfacing only the first through `action.fail`; failed toggles still do not reload.
- Divergent import originals become warnings rather than errors; target/cleanup problems now prevent an unconditional success summary. Forced-tool explanations remain.
- Installation/deployment problems and no selected targets yield partial-outcome warnings. Successfully installed bytes now complete/reload even when deployment throws. A picker with no successful installs stays open without resetting/reloading or announcing success.

**Follow-up:** add both acquisition-skip reasons to `src/lib/reportOutcome.test.ts`'s fixture table. The integration branch currently works (direct HEAD execution verified), but lacks committed regression coverage. F2's required per-action invoke→fold→reporter→completion tests otherwise remain. Q18's `.entry` extraction is the explicit report-reading exception.

## Summary

**Standards: 0 Blocking, 1 Follow-up. Spec: 0 Blocking, 1 Follow-up.** No release-blocking regression found in this review.

Fresh verification:
- `npm test`: **265 passed**, 14 files.
- `cd src-tauri && cargo test --all`: **605 passed**; main/doc targets also passed. Includes admission disposal, four adapters, local Re-point atomicity, Edit propagation failure and byte-for-byte replay rollback tests.
- `npm run version:check`: **1.2.9 consistent**.
- `npm run build`: passed with the project's TypeScript-7 compiler; Vite reported chunk-size/mixed-import warnings.
- Primary LSP diagnostics: five changed frontend modules clean.
- Direct `bun` execution of `refreshOutcome` for `skill_gone` and `stale_acquisition`: warning entry/summary, reload true, close false.
- `git diff --check 7a7c519...HEAD`: clean; tracked checkout remains clean after binding regeneration.

Source review of the baseline-to-HEAD changes found no unintended D1/D2/H-item reverts: production hashing remains unconditional, replay retains recoverable bytes and persisted-row restoration, branch-resolution forwarding remains, and the polling test was replaced by deterministic guarded reconciliation. No app launch or live-library mutation. Full lint/clippy gate was not rerun; this is a source-and-test review, not runtime UI dogfooding.
