# 01: Rust safety + regressions — backup sweep, refs fallback, install cleanup, token-free listing

Status: done — 1e79c92

**Lane:** A (Astra medium)  **Blocked by:** None (can start immediately).
**Files:** `src-tauri/src/core/{install_finalize,git_acquisition,github_download,installer,settings}.rs` and their tests under
`src-tauri/src/core/tests/`. Rust only; no bindings change expected (confirm `src/bindings/index.ts` stays clean after `cargo test`).
**Do NOT touch:** `skill_edits.rs`, `frontmatter_edit.rs`, `refresh.rs`, `propagation.rs`, `artifact_removal.rs`, anything under `src/`.

## S1 — orphaned `.skills-hub-old-<uuid>` backups get a per-skill sweep (backlog 1)
`finalize_update` moves the old central copy aside as a hidden UUID sibling; a failed post-success cleanup (logged) or a failed
rollback (typed `FinalizeRollbackFailed`, names the backup) leaves it forever — `temp_cleanup` sweeps only the cache, and the
`Skill` removal scope removes `central_path` only.
→ In `move_old_central_aside`, before creating the new backup, sweep **siblings of that central path** named `.skills-hub-old-*`
whose mtime is older than **7 days** (`std::fs::metadata(...).modified()`; a sibling whose mtime cannot be read is skipped).
Best-effort: a failed removal is `log::warn!` and never fails the Update. Never touch anything not matching the exact prefix,
never follow symlinks (`symlink_metadata`; a symlink sibling with that name is removed as a link, not followed). No startup
walker, no marker files — a failed-rollback backup is named in the typed error and relies on age alone (operator ruling).
Tests: an aged sibling (set mtime with `filetime` if already a dev-dep, else create then `File::set_modified`) is removed on the
next Update; a fresh one is kept; a sibling of a *different* skill's central path is untouched; a removal failure does not fail
the Update.

## S2 — `finalize_install` cleans up when the upsert fails after `move_into` (backlog 4)
Today an upsert failure leaves untracked bytes under the final name → the next Add of that name hits `SkillExists` (documented
in the fn doc as a known hole).
→ On `store.upsert_skill` failure, `remove_dir_all(&central_path)` best-effort; if that removal also fails, the returned error
must name the retained path (extend the anyhow context — no new typed variant: this is a double fault the UI cannot produce
deliberately; keep the original upsert error as the source). Delete the "known hole" note from the doc. No backup, no rename
aside — a fresh install's bytes are reproducible from a source that still exists.
Test: mirror `finalize_update_restores_old_bytes_when_upsert_fails`'s fault seam (however that test injects the store failure)
→ after a failed install upsert, `central_path` does not exist and a second Add of the same name succeeds.

## S3 — refs fallback when a stored-subpath hint 404s, self-healing the split (backlog 3)
Records written before v1.2.6's fix may store a wrong branch/subpath split (e.g. a `tree/feature/x/skills/foo` URL stored as
subpath `x/skills/foo`). `resolve_tree_source` trusts the stored-subpath suffix rule and returns before any `matching_refs`
call, so `branch_sha` 404s on a branch that does not exist and `classify_fast_path_failure` raises `GithubSkillNotFound` — never
retried, pinned forever.
→ In the fast path: when the SHA stage 404s **and** the split came from the stored-subpath hint (thread a `split_from_hint: bool`
or equivalent from `resolve_tree_source` — make it return how it decided), re-resolve with `matching_refs` (the same rule the
no-hint path uses), and retry the SHA stage once with the corrected coords. If refs also fail or no branch matches, fall through
to today's classification (typed 404). The corrected subpath goes out in `Acquired::resolved_subpath` (already the channel
finalize backfills from — `installer.rs` writes `record.source_subpath = acquired.resolved_subpath…` on the update path);
**no `upsert_skill` before finalize** (acquire-first rule, round-6 D4). Re-point passes `stored_subpath: None` today — keep it.
Tests (API double): hint `x/skills/foo`, refs `["feature/x"]`, URL path `feature/x/skills/foo` → first SHA 404, second SHA call
uses branch `feature/x` + subpath `skills/foo`, `resolved_subpath == "skills/foo"`; and refs failing → typed `GithubSkillNotFound`
as before; and a hint that resolves fine makes **no** refs call (the cache promise of D4 holds).
Hygiene riding along (backlog 6): `fetch_branch_sha` and `matching_refs_at` in `github_download.rs` duplicate client/UA/Accept/
Bearer/check — extract one private GitHub GET helper. Behaviour-preserving; existing tests must not change.

## S4 — settings-read failures never block Add listing / install / Explore preview (backlog 8)
`installer.rs` has three `super::settings::github_token(store)?` sites (listing, install, preview) while `refresh.rs` uses
`github_token(store).unwrap_or_default()`.
→ Add `settings::github_token_or_none(store) -> Option<String>` that logs (`log::warn!`) and returns `None` on a store error;
use it at all five sites (three in `installer.rs`, two in `refresh.rs`). `settings::github_token` itself keeps its `Result` for
the Settings page (which must surface the error).
Test: a store whose `get_setting` fails (reuse whatever fault seam S2 found, or a closed/read-only DB) → `list_git_skills`
still lists (API double sees no bearer).

## Gate
`npm run version:check && npm run check` green (CI runs `cargo test --all` — use it), bindings not dirty. Conventional commits on
your branch, one per S-item. Do NOT push/merge/rebase. Do NOT run `npm run tauri:dev`. `.scratch/` is gitignored — never
`git add -f`. Paste `## Comments` in the final message: per item what changed, the fault seams used, deviations, gate counts.

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 1e79c92. Evidence: git log v1.2.7..v1.2.8 finds first slice 1e79c92; a247958,0779a41,4bbedc0,27e0481 complete cleanup/hint repair/token fallback/backup dating. src-tauri/src/core/install_finalize.rs:333 invokes the sweep; src-tauri/src/core/tests/install_finalize.rs:450 and src-tauri/src/core/tests/git_acquisition.rs:683 retain cleanup/repair regressions.
