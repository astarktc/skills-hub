# 03: GitHub URLs whose branch name contains `/`

Status: done — 6b0ec7d

**Lane:** R3  **Files:** `src-tauri/src/core/git_acquisition.rs`, `src-tauri/src/core/github_api.rs`
(or wherever `fetch_branch_sha` lives), their tests. Do not touch `install_finalize.rs`, `content_hash.rs`, `sync_engine.rs`.

## Problem
`parse_full_github_url` splits `/tree/<branch>/<path>` at the first `/`, so `…/tree/feature/x/skills/foo` is read as
branch `feature`, subpath `x/skills/foo`. Pre-existing for Add, Update/Refresh and Re-point (all go through the parser +
`git_acquisition::acquire`).

## Decision (spec D4)
Resolve in **acquisition**, not the parser (the parser stays a pure grammar; keep its strict clauses intact).
When the parsed source has a tree path with ≥2 segments after `tree/`:
1. **Cache first**: a Managed skill persists `source_subpath` (`skill_store.rs:120`). If the acquisition request carries a
   stored subpath that is a suffix of the tree path, the branch is the remainder — no network. Wire the stored subpath into
   the acquisition intent if it isn't already there (check `AcquireRequest`/intent types; Refresh and Re-point build them).
2. Otherwise call GitHub `GET /repos/{owner}/{repo}/git/matching-refs/heads/<seg1>` once (same client/auth path as
   `fetch_branch_sha`, same error classification), pick the **longest** ref that is a prefix of `<seg1>/<rest>`, split there.
3. On API failure or no match, keep today's first-segment split (log at debug). A 404/403 here must NOT raise the typed
   GitHub-not-found errors — those are reserved for the real acquisition stages.
The clone fallback must use the resolved branch too. Single-segment paths make no extra call.

## Tests
- Unit: resolution over a fake refs list (`["feature", "feature/x"]` → picks `feature/x`; `[]` → first-segment fallback;
  stored subpath suffix → no API call). Use the existing `GithubApi` trait/test double in `git_acquisition.rs` (`branch_sha`
  is already abstracted; add `matching_refs` beside it).
- Existing parser tests unchanged.

## Gate
`npm run version:check && npm run check`. Commit on your branch with a conventional title. Paste `## Comments` in the final message.

## Comments

- 2026-09-15 — Status reconciliation: ready → done — 6b0ec7d. Evidence: git log v1.2.5..v1.2.6 identifies 6b0ec7d; src-tauri/src/core/git_acquisition/resolution.rs:148 queries refs, with longest-branch regression at src-tauri/src/core/tests/git_acquisition.rs:370.
