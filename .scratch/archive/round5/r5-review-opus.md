# Round-5 review — `opus`

Scope: `git diff ed630c0...HEAD` (HEAD `378a473`) plus `6cd91cf`. Fresh evidence: `cd src-tauri && cargo test repoint` → **12 passed, 0 failed** at HEAD.

## Standards

**Verified clean** (no findings): the error contract for all three new variants is complete end-to-end — `SignalError::{SymlinkChainTooDeep,InvalidGithubUrl,GitRepointRequiresGit}` → `CommandError` arm → generated binding → `describeCommandError` branch → EN + ZH copy → wire tests (`commands/tests/commands.rs`). `repoint_git_skill` correctly does **not** take `mutation_guard::serialized` itself (AGENTS.md: "the guard is non-reentrant, so an entry point never calls another entry point") — it reuses Refresh's per-skill apply phase. No `Notification.action` is serialized anywhere (nothing writes `notifications` to storage). `unlocatable::require_local` stayed prose, as Q5 required.

1. **Duplicated Code / Divergent Change — a second GitHub-URL grammar** (`src-tauri/src/core/refresh.rs`, `parse_repoint_source`). `refresh.rs` now owns URL syntax that `git_acquisition.rs` already owns (`parse_github_url`, `looks_like_github_shorthand`), and expresses it as one six-clause boolean: `if parts.len() < 2 || (parts.len() != 2 && (parts.len() < 4 || parts[2] != "tree")) || parts.iter().any(…) || input.chars().any(…) || parts[..2].iter().any(…) || parts[1].trim_end_matches(".git").is_empty()`. Fix: move it beside `parse_github_url` as a strict mode of the one parser, with named predicates. (Judgement call; it is also the root of Spec finding 1.)

2. **Glossary shape** (`CONTEXT.md`, **Unlocatable skill**). The entry now ends "…also available when a git upstream moved without any local path being missing" — i.e. the definition of *Unlocatable skill* now documents a repair offered on every healthy `git` card. Per `docs/agents/domain.md` voice, Re-point has outgrown its host entry: give it its own entry and cross-reference. (Judgement call.)

3. **Derivation in the binder** (`src/App.tsx:249–251`): `skill={activeView === "detail" ? library.managedSkills.find((skill) => skill.id === detailSkill.id) ?? detailSkill : detailSkill}`. AGENTS.md: "`src/App.tsx` is the binder that composes the hooks and passes props down." The freshness lookup belongs next to `detailSkill`'s owner (store the id, select in the hook). (Judgement call.)

4. **Repeated Switches** — "one repair, two provenances" is now decided twice: `useSkillLibrary.handleRepointSkill` (`if (skill.source_type === "git") { handleRepointGitSkill(skill); return; }`) and `SkillCard` (`{kind === "git" ? …}`). One of the two is redundant. (Judgement call.)

5. **Nit**: the new AGENTS.md `skillPresentation` sentence is a single ~240-char line in a file otherwise wrapped at ~100 cols.

## Spec

**Verified correct**: Re-point *is* acquire-first. `acquire_managed_skill_update_from` mutates only its in-memory `record` after `acquire(…)?` returns, and `store.upsert_skill` is reached only on the non-override backfill branch; the staging dir is dropped on every error. 404, ambiguity, non-git, malformed URL and non-skill-download paths are each pinned (`core/tests/refresh.rs`, 12 tests green). The override reaches **both** adapters (`source_override` replaces the whole `GitSource`, `api` included; `parse_repoint_source` requires `source.api.is_some()`), and a stale `source_subpath` is correctly cleared because `known_subpath` comes from the new URL, not the record.

1. **Wrong (blocking): the URL validator refuses shapes the spec's named parser accepts.** Q12/Q13: "input is a full GitHub URL **parsed by the existing GitHub URL parser** (repo and subpath may both change)." `parse_repoint_source` rejects everything except `https://github.com/<o>/<r>[/tree/…]` — in particular `parts[2] != "tree"` refuses `/blob/` URLs, which `parse_github_url` explicitly accepts (`if parts.len() >= 4 && (parts[2] == "tree" || parts[2] == "blob")`) and `normalize_github_skill_subpath` even normalises by stripping a trailing `/SKILL.md`. A `/blob/…/SKILL.md` link is exactly what GitHub's UI copies for a moved skill file, so ticket 08's "operator smoke test: Re-point one of the seven moved skills" is likely to hit `INVALID_GITHUB_URL`. One-token fix: accept `"blob"` alongside `"tree"`. (`http://`, `github.com/o/r` and `owner/repo` shorthand are also refused — defensible per ticket 01's "full GitHub URL", but `blob` is not a shorthand.)

2. **Looks implemented, looks wrong (follow-up): import identity can misreport an identical original as divergent.** Ticket 05: "A test proves a *divergent* original is still never overwritten, and **an identical original is still taken over**." Its finding — "there is no second, narrower fingerprint algorithm" — is true of hashing but overlooks the *copy* asymmetry: `sync_engine::copy_dir_recursive` handles only `is_dir`/`is_file` (and `should_skip_copy` skips only `.git`), so an **in-repo symlink inside a skill folder is silently dropped from the central copy**, while `content_hash::hash_dir` hashes every entry's relative path. `target_has_same_content(&installed.central_path, &variant.path)` therefore returns false for the chosen original itself, which then falls into `originals.push(OriginalStatus::KeptDivergent)` and, if its Tool was selected, produces a `Failed` target for the very Tool the skill was imported from. The module header still claims "(the chosen variant's own Tool always among them)". Fail-safe direction (nothing is overwritten) and it matches the pre-existing auto-sync-off path, hence follow-up, not blocking.

3. **Unpinned behaviour change in `6cd91cf` (follow-up).** `resolve_subpath`'s new `if let [only] = candidates.as_slice()` branch runs **before** the lenient check, so a legacy record with no `source_subpath` (`SkillIntent::NamedSkillOrWholeRepo`) whose name matches nothing now adopts the repo's single nested skill instead of taking the whole repo — and `NamedSkill(Some("foo"))` on a repo whose only skill is `bar` silently returns `bar`. The new test `a_lone_nested_skill_is_the_skill_named_or_not` only covers the matching name and `None`; the mismatching-name case is unpinned in either direction.

4. **Bookkeeping (follow-up).** Ticket `07-repoint-from-failure-notification.md` still reads `**Status:** ready-for-agent` with all three boxes unticked and **no `## Comments` section**, although `378a473` shipped it. I verified the substance independently: both required tests exist (`useStatusReporter.test.ts` "keeps batch actions on each history row…", `useSkillLibrary.test.ts` "offers Re-point only for a known git skill's GitHub-not-found failure") and `gitRepoint.action` exists in EN and ZH. `skillFailureEntries` cannot pick a *wrong* skill (it matches `managed.id === skill.skill_id && managed.source_type === "git"` and confirms with `skill.id`); the only staleness is a pre-refresh `name` in the modal title.

5. **Open by design**: ticket 04's first acceptance box (unreachable-branch test) is unticked with an operator-approved deviation; ticket 08 (CHANGELOG + `version:set 1.2.5`) is unstarted — `package.json` is still `1.2.4`, so story 18 remains outstanding as expected for a pre-panel HEAD.

## Summary

**Blocking**

- **Spec 1** — `parse_repoint_source` (`src-tauri/src/core/refresh.rs`) refuses `/blob/` GitHub URLs that spec Q12's named parser (`git_acquisition::parse_github_url`) accepts and normalises. Accept `"blob"` in the `parts[2]` check before the operator's seven-skill smoke test.

**Follow-up**

- **Spec 2** — an inner symlink in an imported skill folder makes the chosen original hash-divergent from its own central copy (`copy_dir_recursive` drops symlink entries; `hash_dir` counts them) → spurious `KeptDivergent` + a `Failed` target for a selected Tool. Either make `copy_dir_recursive` reproduce symlinks or make `hash_dir` skip them, then re-check the module header's "the chosen variant's own Tool always among them".
- **Spec 3** — pin the lone-nested-skill rule for a *mismatching* name (`resolve_subpath`), both for `NamedSkill` and `NamedSkillOrWholeRepo`.
- **Spec 4** — record ticket 07's `## Comments` / status; its implementation itself checks out.
- **Standards 1** — fold `parse_repoint_source` into `git_acquisition` as a strict mode of the one parser (this also fixes Spec 1 structurally).
- **Standards 2** — give Re-point its own `CONTEXT.md` entry; it is no longer only an Unlocatable-skill repair.
- **Standards 3/4/5** — move the `detailSkill` freshness lookup out of `App.tsx`; drop one of the two provenance switches; rewrap the AGENTS.md sentence.
