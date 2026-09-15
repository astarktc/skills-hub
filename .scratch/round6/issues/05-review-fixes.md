# 05: Round-6 review fixes (panel: Fable 5.1 / Opus 5 / Astra — reports `r6-review-*.md`)

Status: done — f48c001

**Files:** `src-tauri/src/core/{installer,git_acquisition,install_finalize,errors,skill_discovery,content_hash}.rs`,
`src-tauri/src/commands/error.rs`, `src/commandError.ts`, `src/i18n/resources.ts`, `src/hooks/useSkillLibrary.ts`,
`src/components/skills/SkillDetailView.tsx`, tests. Bindings regenerate via `cargo test`.

## Blocking

### B1 — Re-point feeds the OLD record's `source_subpath` as the branch hint for the NEW URL (all three reviewers)
`installer.rs:325` passes `stored_subpath: record.source_subpath.as_deref()` unconditionally, including when
`source_override.is_some()` (git Re-point). `resolve_tree_source` (`git_acquisition.rs:288`) then `strip_suffix`es the
old subpath off the new tree path and returns before any refs lookup. Trace: record `source_subpath = "foo"`, operator
re-points to `…/tree/main/skills/foo` → branch `main/skills`, subpath `foo` → `branch_sha` 404 on a named branch →
typed `GithubSkillNotFound` for a valid URL, never retried as a clone.
→ Pass `stored_subpath: None` when `source_override.is_some()` (Re-point pays one `matching-refs` call; D4 only promised
the cache for Refresh/Update). Add a `refresh.rs` regression: Re-point where the old subpath IS a suffix of the new tree
path; assert the branch/subpath actually requested from the API double, not just returned metadata.

### B2 — a slash-branch-only URL skips name discovery, unlike `tree/main` (Fable)
`resolve_tree_source` yields `subpath: Some(".")` when the matched branch consumes the whole tree path
(`git_acquisition.rs:~315` `.unwrap_or(".")`), and `acquire` (`:223–232`) rewrites `NamedSkill | NamedSkillOrWholeRepo`
into `Subpath(".")` whenever `source.subpath == Some(".")`. So `tree/feature/x` (no path) forces the repo root while
`tree/main` (parser → `None`) still discovers by name — Refresh of a legacy record, Explore preview and Re-point stop
discovering on slash branches.
→ When the branch consumes the whole tree path, `resolve_tree_source` sets `subpath = None` (same shape the parser
produces for `tree/main`). Keep the `acquire` rewrite ONLY for a subpath that was explicitly `"."` in the operator's
URL (the `/blob/<branch>/SKILL.md` root case Opus notes it preserves) — if you cannot distinguish "explicit root" from
"branch consumed the path" without the resolver, make the resolver the one that decides and delete the rewrite. Add a
unit test: refs `["feature/x"]`, URL `tree/feature/x` → branch `feature/x`, subpath `None`, and a `NamedSkill` intent
still reaches name discovery.

### B3 — rollback-failure recovery message is backend prose (Astra, AGENTS.md error contract)
`install_finalize.rs:~347` composes `"rollback to {:?} failed: {:#}; old bytes retained at {:?} for manual recovery"`
through `anyhow`; it surfaces as `OTHER` verbatim (English for zh users). It is an operator-causable condition
(permissions) carrying a path the operator must act on.
→ New `SignalError::FinalizeRollbackFailed { central: String, backup: Option<String> }` (`errors.rs`), raised with the
original failure as `.context`/source chain (keep it as diagnostics); `CommandError` arm (`commands/error.rs`,
internally tagged, `detail` carries the rendered chain); `describeCommandError` branch in `src/commandError.ts`; i18n
keys in BOTH `en` and `zh` (message names the backup path). Follow the `PATH_OUTSIDE_TOOL_DIRS` worked example in
AGENTS.md. Existing test `rollback_rename_failure_names_and_preserves_backup` asserts the typed variant via downcast.
`cargo test` regenerates `src/bindings/index.ts` — commit it.

## Also (small, in scope)
- A1 (Opus S3): `skill_discovery.rs:~130` known-scan-base loop — add the `is_hidden_dir_name` guard so `.skills-hub-old-*`
  is excluded on that path too; extend the existing hidden-backup test.
- A2 (Fable F3): D9 completion — `useSkillLibrary.ts:~166` (`sourceKind(managed) === "git"` in `skillFailureEntries`)
  and `SkillDetailView.tsx:~531` use `repointDoor(...) === "git"`.
- A3 (Fable F6): doc note on `finalize_install` stating the known hole (upsert failure after `move_into` leaves untracked
  bytes under the final name → next Add of that name hits `SkillExists`; cleanup is a separate change).
- A4 (Opus P4): one line in `hash_dir`'s doc — a skill consisting only of symlinks hashes as empty.

## NOT in scope (round-7 backlog, `.scratch/round7/backlog.md`)
backup sweep for orphaned `.skills-hub-old-*`; `matching-refs` called twice per Add; refs fallback when a hinted branch
404s; `activeView === "detail"` reset; GitHub GET helper extraction; named rollback fn; `repointDoor` tautology;
`list_git_skills` settings-read failure; `finalize_install` cleanup.

## Gate
`npm run version:check && npm run check` green, bindings not dirty. Commit(s) on your branch with conventional titles
(one per B-item preferred). Do NOT push/merge/rebase. `.scratch/` is gitignored — never `git add -f`. Paste `## Comments`
in the final message.

## Comments

- 2026-09-15 — Status reconciliation: ready → done — f48c001. Evidence: git log v1.2.5..v1.2.6 finds f48c001 first, then b5e398b,e149e19,1e84fc3 for slash-root, typed rollback and residue. src-tauri/src/core/tests/git_acquisition.rs:451 and src-tauri/src/core/tests/install_finalize.rs:722 retain the regressions.
