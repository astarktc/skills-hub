# Round-5 review — fable

Scope: `git diff ed630c0...HEAD` (v1.2.4 → `378a473`) plus `6cd91cf`. Read-only; no gate re-run (implementer logs claim green, not disputed).

## Standards

**Hard (documented standard)**

1. `src/components/skills/SkillDetailView.tsx:531` and `src/hooks/useSkillLibrary.ts` (`handleRepointSkill`, `skillFailureEntries`) test provenance with `skill.source_type === "git"`, while `SkillCard` uses `sourceKind(skill) === "git"`. AGENTS.md: "`src/lib/skillPresentation.ts` owns source kind … one implementation each". `sourceKind` accepts `"GitHub"`-style spellings; the raw compare does not — the two doors can disagree for the same row. Fix: call `sourceKind` at all three sites.

**Judgement calls (baseline smells)**

2. *Duplicated Code / Shotgun Surgery* — `refresh.rs::parse_repoint_source` re-implements URL-shape knowledge (`/tree/`, `.git`, segment charset) that `git_acquisition::parse_github_url` already owns, then calls it anyway. Two parsers must now be kept in step; they already disagree (see Spec 1). Better: parse once, then validate the *result* (`source.api.is_some()`, `branch/subpath` sanity via `require_plain_subpath`).
3. *Primitive Obsession* — `refresh.rs:214` `if record.source_type != "git"`; every neighbour (`installer.rs:301`, `unlocatable.rs`, `legacy_reclassification.rs`) goes through `Provenance::parse`. One spelling authority, please.
4. *Speculative Generality* — `GitRepointModal` takes `loading`/`closeDisabled`, but `handleConfirmRepointGitSkill` calls `setPendingGitRepointSkill(null)` *before* the invoke, so the modal is never mounted while loading. Either keep the modal open through the action (better UX: the operator sees the spinner where they typed) or drop the dead props.
5. *Feature Envy* — `App.tsx:249` computes `library.managedSkills.find(s => s.id === detailSkill.id) ?? detailSkill` inline in JSX. The "detail row follows the library" rule belongs in the binder as a memo (or the detail state should hold an id, not a snapshot); the inline lookup runs on every render.
6. *Inconsistent siblings* — `describeCommandError` renders `SYMLINK_CHAIN_TOO_DEEP` as `withDetail(copy, e.subpath)` while its named sibling `SYMLINK_ESCAPES_REPO` interpolates `{{subpath}}`. Spec Q6 kept interpolation for the escape variant; the new one should match its sibling (interpolate) rather than introduce a second rendering for the same field.
7. Stale docstring — `SkillIntent::NamedSkillOrWholeRepo` (`git_acquisition.rs:86-91`) still says an unresolved name "takes the whole repo"; after `6cd91cf` a lone nested skill wins over the whole repo. Comment should state the new rule.
8. Ticket comments cite worktree SHAs (`d3c3076`, `628b85d`, `71f2a54`, `7c13cee`, `5a271b4`) that do not exist on main (`1897f79`, `6b0fc7a`, `7591fe3`, `0011fec`, `57591c8`). Docs-only; the acceptance evidence is otherwise verifiable.

Nothing else breaches AGENTS.md: the new command is registered in `collect_commands!`, bindings regenerated, all three new `SignalError`s have `CommandError` arms + `describeCommandError` branches + EN/ZH; the prose-vs-typed rule is applied (`SymlinkChainTooDeep` typed, `require_local` prose); Re-point never nests entry points (one `refresh_managed_skills_with` call, guard taken per skill inside); no `remove_path_any` outside `artifact_removal`/`onboarding_import`.

## Spec

**Missing / wrong**

1. **Re-point refuses `/blob/` URLs.** `parse_repoint_source` requires `parts[2] == "tree"`; `parse_github_url` (and the Add flow) accept `/blob/<branch>/<path>[/SKILL.md]`. Spec Q12: "input is a full GitHub URL parsed by the existing GitHub URL parser". The URL an operator gets by opening the moved `SKILL.md` on GitHub is a `/blob/` URL — the most likely paste for the seven moved skills (Q14) — and it is bounced with `INVALID_GITHUB_URL`. Same for `http://github.com/…`, `github.com/…`. Fix: accept `blob` alongside `tree` (one token), or validate the parsed result instead.
2. **Round-trip drift on `source_subpath`.** A `/tree/<b>/SKILL.md` (or `/blob/` once allowed) URL normalises to subpath `"."`; the override path stores `record.source_subpath = Some(".")` whereas Add stores `None` for the root (`installer.rs:592`). Harmless today (`acquire` filters `"."`), but two spellings of "root" now exist in the DB.
3. Ticket 04 first box "Core test: the not-found path returns the typed error" is unticked — approved deviation, recorded; `NotFound { kind: "skill" }` is the right kind (the only `None` source is an absent live name, i.e. the skill row).

**Scope creep** — none material. `ensure_installable_skill_dir` on the override path is a sensible safety addition the ticket records.

**Verified claims**

- *Acquire-first*: `acquire_managed_skill_update_from` mutates only the in-memory `record`; the only store write on the git path (`upsert_skill` backfill) is inside `else if known_subpath.is_none()` and unreachable with an override. `finalize_update` persists via `..record.clone()`, so the new `source_ref`/`source_subpath` land exactly once, with the revision. 404 (typed), MultiSkills, non-skill dir (`SKILL_INVALID`), cancel: all reported/thrown before finalize; tests `git_repoint_404_preserves_every_record_field_and_central_bytes` / `_ambiguous_repo_preserves_the_record_byte_for_byte` pin it. Override honoured by both adapters (the `GitSource` carries `api`, `branch`, `clone_url`).
- *Discovery fix*: canonicalisation is consistent for a root behind a symlink and for `/private/var`; Windows `\\?\` prefixes appear on both sides of the compare; a link outside the repo stays its own candidate. Behaviour change not pinned by a test: a legacy `NamedSkillOrWholeRepo` record whose name no longer matches now Updates to the lone nested skill instead of the whole repo — an improvement, but undocumented (Standards 7).
- *Import*: `walkdir` 2.5 follows the root link, so a symlinked variant hashes as its target. Residual: a skill dir containing an *internal* symlink hashes differently from its central copy (`copy_dir_recursive` drops symlink entries; `hash_dir` hashes their path) → the chosen original itself is reported `KeptDivergent` and its selected target fails. Pre-fix the chosen path was accepted unconditionally. Rare; note it.
- *Notification action*: `listText` copies `at/kind/title/message` only; nothing serialises `action`. `skillFailureEntries` captures the pre-refresh `managedSkills`; a failed refresh does not change the row, and the modal uses only `id`/`name`, so a stale pick is not possible short of a concurrent delete (then typed `NOT_FOUND`).
- Story 4/5/6/10/11/13/16/17: present and correct.

## Summary

**Blocking**
- Spec 1 — `/blob/` (and `http://`/schemeless) GitHub URLs refused by `parse_repoint_source` (`refresh.rs`), contradicting Q12 and stranding the most natural paste for the seven moved skills.
- Standards 1 — raw `source_type === "git"` in `SkillDetailView.tsx` / `useSkillLibrary.ts` bypasses `sourceKind` (AGENTS.md "one implementation" rule).

**Follow-up**
- Standards 2, 3 (one URL parser; `Provenance::parse` in `refresh.rs`).
- Standards 4 (dead `loading` on `GitRepointModal` — or keep the modal open during the action).
- Standards 5, 6, 7, 8.
- Spec 2 (`Some(".")` vs `None` for the root subpath).
- Import residual: internal symlink in an original → chosen copy reported divergent.
