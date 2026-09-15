# Round-9 review — v1.2.9 hygiene + defect patches (single reviewer: Fable)

Repo: `/Users/alexstark/Projects/skills-hub` (main checkout, branch `main` — READ ONLY; do not edit, commit, run mutating npm/cargo commands, or launch the app; `cargo test --all`/`npm test`/`npm run build` are fine to confirm something). Do not search outside this directory (cloud-synced folders). Do not run `npm run tauri:dev`. Do not `git add` anything; `.scratch/` is gitignored. AGENTS.md Workflow step 1 does not apply to a review — start immediately. Write your report incrementally.

## What to review
- Fixed point: `96cd33c` (v1.2.8). HEAD = `0acafd7` (release: v1.2.9, unpublished). Diff: `git diff 96cd33c...HEAD`; 10 commits: 7 hygiene (H1–H7), 2 defect patches (D1, D2), 1 version bump.
- Tickets: `.scratch/round9/issues/01-hygiene.md` (H1–H7), `.scratch/round9/issues/02-defect-patches.md` (D1, D2). Origin of D1/D2: `.scratch/round9/panel/{opus,fable,astra}.md` (Opus 1 / Fable 1 = D1, Astra 1 = D2).
- Implementer claims (verify): H1 install reuses the listing's resolution (`GitSourceResolution`, `#[serde(default)]`), one `matching-refs` call per Add, legacy caller still resolves; H3 `repointDoor` gone, 4 call sites on `sourceKind`; H5 one column-0 fence rule used by all three parsers; H6 convergence clears the conflict flag and keeps the override; D1 finalize always hashes, env flag + AGENTS clause removed, release-mode regression Update → copy assignment → reconcile = Synced; D2 `finalize_update` gains a `settle` closure run after the row upsert and before backup release, failure restores old bytes + the **pre-finalize** row (snapshotted from the store, not the input record) + the Edit snapshot; double faults named.
- Standards sources: `AGENTS.md`, `CONTEXT.md`, `docs/adr/*.md`.

## Axis 1 — Standards
Documented-standard breaches (cite file + rule) and baseline smells (Fowler ch.3 vocabulary), hard vs judgement. Skip tooling-enforced things. Under 300 words.

## Axis 2 — Spec
Missing/partial requirements, scope creep, implemented-but-wrong — quote the ticket line. Under 300 words. Scrutiny:
- D2: is `previous` (store snapshot) the right row to restore when the input `record` carries a Re-point override — does a failed Re-point Update leave the OLD source, and a failed plain Update leave the old row byte-for-byte (incl. `content_hash`, `updated_at`)? Order inside `replay_unlocked`: edit row upserted before bytes (row-is-source-of-truth) — on failure after the manifest write but before `record_hash`, are bytes rolled back by finalize AND edit row by replay, leaving nothing half-settled? Restore of an Unlocatable skill (row present, central gone, no backup) still passes through the new `get_skill_by_id` precondition? Any caller of `finalize_update` outside `finalize_and_propagate_unlocked` missed? Acquire-first untouched (no upsert before finalize on any path)?
- D1: any remaining reader that assumes the hash may be absent for a *policy* reason (vs an I/O failure) — comments/docs now stale (`propagation.rs` "absent when finalize did not compute one")? Does the new propagation/reconcile test feed the hash through the real supplier rather than a literal?
- H1: `resolution` optional on the wire — old frontend/new backend and vice-versa both fine; Re-point still `stored_subpath: None`; no upsert before finalize.
- H5: trailing-whitespace tolerance on the closing fence consistent across all three sites; block-scalar test covers Edit's byte-preserving write.
- H6: the flag clears only when upstream == override; an unchanged upstream (== base) keeps a previously set flag.
- Worktree safety: `git diff 96cd33c..HEAD -- src-tauri/src src/` shows no unintended reverts of v1.2.7/v1.2.8 code.

## Output
Write to `/Users/alexstark/Projects/skills-hub/.scratch/round9/r9-review-fable.md` AND return it as the final message. `## Standards`, `## Spec`, `## Summary` with **Blocking** (must fix before v1.2.9 ships) vs **Follow-up**. Verify every Blocking at HEAD. Concrete; no generic advice.
