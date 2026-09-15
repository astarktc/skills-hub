# 06: Docs drift, glossary line, `version:set` covers both lockfiles

Status: done — 991db77

**What to build:** `npm run version:set X.Y.Z` leaves nothing for the next `cargo test` or `npm ci` to dirty: it also rewrites the root `version` entries of `package-lock.json` (top-level and the `""` package) and the `app` package entry in `Cargo.lock`, and `npm run version:check` fails when any of the five locations disagree. AGENTS.md's Onboarding-import sentence says it syncs through the global sync batch (a first sync), not that it propagates (Propagation is for updates to an already-Managed skill). The round-2 ticket 12 comment cites the real release commit (`64c19be`, v1.2.2). CONTEXT.md's *Artifact removal* "Avoid" line reads: cleanup, unsync, unassign — in new names; the wire code `DELETE_CLEANUP_FAILED` predates the term.

Source: `../spec.md` Q2 (glossary line), Q13; `../source-13-review-followups.md` #9, #12, #13. Skill: **writing-for-agents** for the AGENTS.md edit.

**Blocked by:** None (can start immediately)

- [x] `version:check` fails on a hand-desynced `package-lock.json` or `Cargo.lock` and passes after `version:set`
- [x] Running `cargo test` immediately after `version:set` leaves the tree clean
- [x] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Status reconciliation: done (branch `r3/06-docs-version-tooling`, awaiting merge) → done — 991db77. Evidence: Cited 991db77 integrated on main as 8ea2cba; docs followed in a2ba9c4/e0c5cfe. The old awaiting-merge/not-merged claim is obsolete (rebased/fast-forward integration); scripts/version.mjs:64,109 rewrites both lockfiles.
- 2026-09-15 — Previous status wording (historical, not a current merge/publication claim): done (branch `r3/06-docs-version-tooling`, awaiting merge)

Shipped on `r3/06-docs-version-tooling` (not merged):

- `991db77` chore(version): `version:set`/`version:sync` also rewrite `package-lock.json` (root `version`
  + `packages[""].version`, re-serialised as `JSON.stringify(lock, null, 2) + "\n"` — npm's own format,
  diff confirmed to touch only the two version lines) and `src-tauri/Cargo.lock` (regex anchored on
  `[[package]]\nname = "app"\nversion = "…"`, every other byte preserved; no TOML dependency).
  `version:check` now reports each of the five locations by name. Same commit moves
  `package-lock.json`'s stale root version from `1.1.9` to `1.2.2` (intended).
- `809761a` docs(agents): Onboarding import line → "sync through the global sync batch (auto-sync on — a
  first sync, not Propagation)"; Version invariant + the two `version:*` command comments name all five
  locations. The **Sync-target mutation** bullet already said "Onboarding import's apply phase" (no
  "propagate" wording) and CONTEXT.md's **Onboarding import** entry already said "synced to the requested
  Tools" — both left as-is.
- `6d83975` docs(glossary): *Artifact removal* Avoid line → "cleanup, unsync, unassign — in new names; the
  wire code `DELETE_CLEANUP_FAILED` predates the term (…)".
- `.scratch/arch-deepening/issues/12-docs-drift-and-version.md` (gitignored, edited in place): comment now
  cites the SHAs on `main`.

**Evidence**

- Stale lockfile caught before the fix: `version:check` → `package-lock.json version=1.1.9 (expected
  1.2.2)` + `packages[""].version=1.1.9`, exit 1. Hand-desynced `Cargo.lock` `app` entry to `0.0.1` →
  `src-tauri/Cargo.lock [[package]] app version=0.0.1 (expected 1.2.2)`, exit 1; `version:set 1.2.2` →
  `Version OK (1.2.2)`.
- `version:set 9.9.9` then `cargo test` + `npm install --package-lock-only`: `git status` identical before
  and after (six files, all from `version:set`); restored with `version:set 1.2.2`, tree clean.
- Gate: `Version OK (1.2.2)`; `npm run check` exit 0 (430 Rust tests).

**Deviations / for the orchestrator**

- The ticket says to cite `64c19be` as the release commit. That SHA is a dangling pre-rebase commit
  (`git merge-base --is-ancestor 64c19be HEAD` → false); the commit on `main` is `8af3355` (same
  content, rebased). The ticket-12 comment cites `8af3355`/`e094456` and names `64c19be`/`f1eff91` as
  the pre-rebase worktree SHAs so either lookup lands. Also corrected the same comment's "`[1.3.0]`
  entry" / "Version OK 1.3.0" lines to 1.2.2, and recorded what actually happened to `Cargo.lock`: the
  release commit shipped its `app` entry at the `1.3.0` cargo had resolved earlier; `6f973f6` fixed it.
- Pre-existing flake, not touched: `core::refresh::tests::acquisitions_overlap_instead_of_running_one_at_a_time`
  (`src/core/tests/refresh.rs:375`, asserts wall-clock `< 60%` of sequential) failed in 3 of 4 full
  `cargo test` runs while the machine load average was ~45 (parallel round-3 worktrees), and passed in
  isolation and in the final gate. Worth a loosened bound or a `#[ignore]`-under-load strategy in a
  later ticket if it bites the merge ritual.
- No CHANGELOG entry added (ticket 09 owns the round's `[Unreleased]` → `[1.2.3]` roll-up); the tooling
  change is developer-facing and can be listed under Internal there if wanted.
