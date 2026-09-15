# Round-8 review — v1.2.8 safety slice (single reviewer: Fable)

Repo: `/Users/alexstark/Projects/skills-hub` (main checkout, branch `main` — READ ONLY; do not edit, commit, run mutating npm/cargo commands, or launch the app; `cargo test --all`/`npm test`/`npm run build` are fine to confirm something). Do not search outside this directory (cloud-synced folders). Do not run `npm run tauri:dev`. Do not `git add` anything; `.scratch/` is gitignored. AGENTS.md Workflow step 1 does not apply to a review — start immediately. Write your report incrementally.

## What to review
- Fixed point: `51dcae6` (v1.2.7). HEAD = `3190623`. Diff: `git diff 51dcae6...HEAD`; 5 commits, two lanes merged fast-forward (Rust lane 4 commits, frontend lane 1).
- Spec/rulings: `.scratch/round8/spec.md`. Tickets: `.scratch/round8/issues/01-rust-safety.md` (S1–S4), `02-detail-view.md`. Origin: `.scratch/round7/backlog.md` items 1, 3, 4, 5, 8 and the triage rulings appended there.
- Implementer claims (verify): S1 parent-local 7-day sweep, no symlink following; S2 landed bytes removed on upsert failure, double fault names the path; S3 stored-hint SHA 404 → `matching-refs` re-resolve → one bounded retry, corrected subpath persisted only by finalize (acquire-first — a SQLite trigger test proves no upsert in acquisition); shared GitHub GET helper extracted; S4 `github_token_or_none` at all five sites. Frontend: `effectiveView` derived once in `App.tsx`, no effect.
- Standards sources: `AGENTS.md`, `CONTEXT.md`, `docs/adr/*.md`.

## Axis 1 — Standards
Documented-standard breaches (cite file + rule) and baseline smells (Fowler ch.3 vocabulary), hard vs judgement. Skip tooling-enforced things. Under 300 words.

## Axis 2 — Spec
Missing/partial requirements, scope creep, implemented-but-wrong — quote the ticket line. Under 300 words. Scrutiny:
- S1: can the sweep ever remove something that is not an app-created backup (prefix match only, parent-local, symlink not followed)? Does `remove_dir_all` on a symlink follow it on any platform we ship (macOS/Linux/Windows)? Is a `.skills-hub-old-*` sibling of a *different* skill (same parent = the central repo root!) swept — is that the intended "per-skill" scope or a wider one? Age from `modified()` — is that the rename time or the original dir's mtime (rename preserves mtime; is the 7-day clock therefore measured from the *last content change*, not from when the backup was made)?
- S2: is the original upsert error preserved as the source; can a second Add of the same name now succeed; does the doc's "known hole" note go away?
- S3: acquire-first holds (no `upsert_skill` before finalize on any path); the retry is bounded; a hint that resolves fine makes NO refs call (round-6 D4 cache promise); Re-point still passes `stored_subpath: None`; `classify_fast_path_failure` unchanged for the non-hint case; the `Subpath` intent rewrite only when it equals the stored hint; the GET helper is behaviour-preserving (UA/Accept/Bearer/status classification).
- S4: five sites, Settings page still surfaces the error, warn-logged once.
- Frontend: every raw `activeView` read replaced or deliberately left; `explore-detail` unaffected; no `useEffect` writing state.
- Worktree safety: `git diff 51dcae6..HEAD -- src-tauri/src src/` shows no unintended reverts of v1.2.7 (Edit V1) code.

## Output
Write to `/Users/alexstark/Projects/skills-hub/.scratch/round8/r8-review-fable.md` AND return it as the final message. `## Standards`, `## Spec`, `## Summary` with **Blocking** (must fix before v1.2.8: standard breach, spec miss, data-safety bug) vs **Follow-up**. Verify every Blocking at HEAD. Concrete; no generic advice.
