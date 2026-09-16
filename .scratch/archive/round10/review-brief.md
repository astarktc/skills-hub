# Round-10 wave-A review — 3-seat panel (Fable, Opus, Astra)

Repo: `~/Projects/skills-hub` (main checkout, branch `main` — READ ONLY; do not edit, commit, run mutating commands, or launch the app; `cargo test --all`/`npm test`/`npm run build` are fine to confirm something). Do not search outside this directory (cloud-synced folders). Do not run `npm run tauri:dev`. Do not `git add` anything; `.scratch/` is gitignored. AGENTS.md Workflow step 1 does not apply to a review — start immediately. Write your report incrementally.

## What to review
- Fixed point: `7a7c519` (v1.2.9). HEAD = `git rev-parse HEAD` (9 commits: 7 backend, 1 frontend, 1 integration fix). Diff: `git diff 7a7c519...HEAD`.
- Tickets: `.scratch/round10/issues/01-backend-content-identity-and-update.md` (B1 content identity, B2 skill update), `02-frontend-report-outcome.md` (F1 fold, F2 tests). Rulings: `.scratch/round10/decisions.md`. Origin: round-9 panel `.scratch/round9/panel/architecture-review.html` candidates #1, #2, #4 (seat reports beside it).
- Vocabulary: `~/.pi/agent/skills/codebase-design/SKILL.md` — judge depth/seams/locality in those terms.
- Implementer claims (verify): B1 `content_identity::{record, read, same_content}` with `Source` enum; hashing private; Propagation reads once; polling test replaced. B2 `skill_update::apply_unlocked(paths, store, UpdateRequest) -> ApplyOutcome` with four byte adapters; admission → `skipped_acquisition { skill_gone | stale_acquisition }`; local Re-point atomic; Edit returns `InvocationEditOutcome { entry, propagation }`; onboarding comparisons migrated to recorded identity; local acquisition stays outside the guard. F1 six folds `(report, ctx) → Outcome{toast, errors, warnings, completion{reload, closeModal, conflict}}`, actions carry skill ids resolved at click time, `managedSkillsRef` gone, precedence conflict › failure › skipped › success unified across Update/Restore/both Re-points. F2 vitest 237→265. Integration: `ACQUISITION_SKIP_KEY` + `skipped_acquisition` branch in `refreshOutcome`.
- Standards sources: `AGENTS.md`, `CONTEXT.md`, `docs/adr/*.md` (esp. 0004).

## Axis 1 — Standards
Documented-standard breaches (cite file + rule) and baseline smells (Fowler ch.3), hard vs judgement. Also: is each new module actually DEEP (small interface, real implementation) — or did the tail just move? Apply the deletion test to `skill_update`, `content_identity`, `reportOutcome`. Under 350 words.

## Axis 2 — Spec
Missing/partial requirements, scope creep, implemented-but-wrong — quote the ticket/ruling line. Under 350 words. Scrutiny:
- B1: any remaining `hash_dir` caller outside the module? `read` backfills once (not per target)? I/O failure → `None` and status unchanged? `.skills-hub-manifest-` decision documented? Onboarding migration (not in ticket) — justified or scope creep?
- B2: admission compares against the CURRENT row under the guard — which fields; does a re-point adapter correctly bypass the stale check (ruling Q4 "and the adapter does not itself carry a re-point")? Staged bytes actually discarded on skip? Acquire-first: no upsert before finalize on any adapter? Local Re-point: failed → old `source_ref` AND old bytes (test exists)? Edit write: target failures in `propagation`, not logged; guard held once (non-reentrant — did anything call an entry point from inside another)? D2/ADR-0004 rollback still byte-for-byte? `refresh.rs` still owns pool + guard + reassert only? What is left in `installer.rs`?
- F1/F2: any report field read in a hook outside the fold? Click-time id resolution really resolves against current state? The unified precedence/completion rule — list every behaviour change vs v1.2.9 the implementer made ("deliberate changes") and judge each: correct, or a regression in operator-visible behaviour? Deleted hook tests: does at least one hook test per action remain proving fold→reporter→completion? i18n: no hardcoded copy, EN+ZH.
- Integration commit: `ACQUISITION_SKIP_KEY` in `skillPresentation.ts` — right home?
- Worktree safety: `git diff 7a7c519..HEAD -- src-tauri/src src/` — no unintended reverts of v1.2.9 D1/D2/H-items.

## Output
Write to `~/Projects/skills-hub/.scratch/round10/r10a-review-<seat>.md` (seat = fable | opus | astra) AND return it as the final message. `## Standards`, `## Spec`, `## Summary` with **Blocking** (must fix before release: standard breach, spec miss, data-safety bug, operator-visible regression) vs **Follow-up**. Verify every Blocking at HEAD. Concrete; no generic advice.
