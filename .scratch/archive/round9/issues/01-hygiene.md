# 01: Round-9 hygiene — close backlog items 2, 6, 7, 9, 10, 11 + two cosmetics

Status: done — 76ca354

**Lane:** single (Astra low)  **Blocked by:** none.
**Origin:** `.scratch/round7/backlog.md` (items 2, 6, 7, 9, 10, 11) + round-8 review cosmetics.
**Files (Rust):** `src-tauri/src/core/{installer,git_acquisition,github_download,install_finalize,content_hash,skill_files,frontmatter_edit,skill_discovery,skill_edits}.rs` + their tests.
**Files (TS):** `src/lib/skillPresentation.ts`, `src/lib/skillPresentation.test.ts`, `src/components/skills/{SkillDetailView,SkillCard}.tsx`, `src/hooks/useSkillLibrary.ts`.
**Do NOT touch:** `refresh.rs`, `propagation.rs`, `artifact_removal.rs`, `mutation_guard.rs`, `App.tsx`, versions (parent bumps).

## H1 — Add resolves `matching-refs` once (backlog 2)
Listing (`installer.rs` `list_git_skills`, ~line 449) resolves the source via `matching-refs`, then install passes `stored_subpath: None` and resolves again. Under rate limiting the two resolutions can disagree (split-brain).
→ Install reuses the listing's resolution. Find what the selection already carries from listing → frontend → `install_git_skill_from_selection_with`; if the listing DTO must gain the resolved branch/subpath, add it as `#[serde(default)]` optional fields (`cargo test` regenerates `src/bindings/index.ts` — commit it), and the frontend passes it through unchanged. Re-point keeps `stored_subpath: None`. Acquire-first still holds (no upsert before finalize).
Tests (API double): Add from a listing makes exactly one `matching-refs` call across listing + install; a selection without the resolved coords (old caller) still resolves as today.

## H2 — name the rollback IIFE (backlog 6)
`install_finalize.rs`: the inline closure that performs the finalize rollback becomes a named fn (e.g. `roll_back_update`) with a one-line doc. Behaviour-preserving; existing tests unchanged.

## H3 — drop `repointDoor` (backlog 7)
`skillPresentation.ts` `repointDoor` is a tautological alias of `sourceKind` (deletion test: it is a pass-through; Edit V1 did not give it an eligibility body). Delete it; the 4 call sites (`SkillDetailView.tsx:532`, `SkillCard.tsx:287`, `useSkillLibrary.ts:178,661`) use `sourceKind(skill) === "git"`. Remove its `describe` block in `skillPresentation.test.ts`; keep `sourceKind`'s tests.

## H4 — one `is_ignored` (backlog 9)
`IGNORE_NAMES`/`is_ignored` duplicated in `content_hash.rs` and `skill_files.rs`. Keep one `pub(crate)` definition in `skill_files.rs`; `content_hash.rs` imports it. No behaviour change.

## H5 — frontmatter closing fence is column-0 only (backlog 10)
`frontmatter_edit::header_end` (line ~70), `skill_discovery::parse_invocation_mode` (~448) and `skill_discovery::parse_skill_md_with_reason` (~328) treat an indented `---` inside a block scalar (`description: |` followed by `  ---`) as the closing fence.
→ One fence rule, one place: a `pub(crate)` helper (in `skill_discovery.rs` or `frontmatter_edit.rs`, your call — the other imports it) that finds the header end using **`---` at column 0 exactly** (trailing whitespace tolerated, leading whitespace not). All three sites use it; change them in one commit.
Tests: a manifest whose description block scalar contains an indented `---` line parses name/description/mode correctly in all three parsers; an Edit V1 write (`write_invocation_mode`) on such a manifest keeps the body intact. Existing fence tests unchanged.

## H6 — upstream converging to the override is not a conflict (backlog 11)
`skill_edits::replay_unlocked` (~line 159): `conflict = upstream_mode != base_mode` flags even when `upstream_mode == override_mode` (upstream adopted what the operator chose).
→ Conflict only when `upstream_mode != base_mode && upstream_mode != override_mode`. When upstream now equals the override, **clear** a previously set `edit.conflict` flag (the disagreement is resolved) and report `None`. The edit row stays (override still in force; bytes still follow the row). Keep "an unresolved conflict remains flagged across later unchanged Updates" for the genuine case.
Tests: base=user, override=model, upstream→model ⇒ no conflict, flag cleared if previously set; base=user, override=model, upstream→disabled ⇒ conflict as today.

## H7 — cosmetics from the round-8 review
- `git_acquisition.rs` `acquire_resolved` (~line 226): extract the S3 stored-hint 404 → refs re-resolve → retry block into a named private fn. Behaviour-preserving; the existing trigger/API-double tests must pass unchanged.
- `github_download.rs` ~line 391: `settings_read_failure_sends_no_bearer_to_github` duplicates a test elsewhere (find it with `rg`); delete the one in `github_download.rs` if the other covers the same seam, else keep the better-placed one.

## Gate
Work on branch `r9/hygiene` off `main` (`96cd33c`) **in a git worktree** at `/tmp/skills-hub-r9-hygiene` (`git worktree add /tmp/skills-hub-r9-hygiene -b r9/hygiene main`, then `npm ci` there) so the main checkout stays untouched (a review panel is reading it). `npm run version:check && npm run check` green (CI runs `cargo test --all` — use it); `src/bindings/index.ts` committed if regenerated, no untracked bindings. Conventional commits, one per H-item, no push/merge/rebase. Do NOT run `npm run tauri:dev`. `.scratch/` is gitignored — never `git add -f`. If the pi-lens commit guard blocks `git commit`, write `git diff` per item to `~/Projects/skills-hub/.scratch/round9/<item>.patch` and report; do not bypass. Final message: `## Comments` — per item what changed, deviations, gate counts, the worktree path and branch HEAD.

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 76ca354. Evidence: git log v1.2.8..v1.2.9 finds first slice 76ca354, followed by 365a3f8,ace8725,75728dc,5516eab,d24b6bf,7ff30a5. src-tauri/src/core/skill_edits.rs:177–179 clears conflict on upstream convergence; src-tauri/src/core/content_identity.rs:95 uses the shared ignore predicate.
