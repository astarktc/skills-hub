# Round-6 review — reviewer `opus`

Range verified at `95b7893...HEAD` (`6b0ec7d`). `cargo test --all` 555 pass, `npm test` 236 pass,
`npm run version:check` OK, `git status` clean after `cargo test` (no binding drift).
Worktree safety: two-dot and three-dot `--stat` are identical over `src-tauri/src src/`; every `-` line in
the diff belongs to a hunk this round intended. **No reverts from `95b7893`.**

## Pre-answer — the `hash_dir` symlinked-root question

**walkdir 2.5.0 yields a symlinked root as a *symlink entry*, not as a followed directory** — and
`target_has_same_content` over a symlinked target still returns `true`.

`walkdir-2.5.0/src/lib.rs` (the `follow_root_links` branch): *"the DirEntry should still respect the
`follow_links` setting. When it's disabled, it should report itself as a symlink"* — it then does
`fs::metadata(...)`, and if that is a dir it `self.push(&dent)`, i.e. **descends anyway**. Empirically
(standalone walkdir 2.5.0 binary, `follow_links(false)`, root = symlink to a skill dir):

```
--- symlinked target root ---
SKIP-symlink ".../target-link" depth=0
HASH "sub" depth=1 dir=true
HASH "sub/a.txt" depth=2 dir=false
HASH "SKILL.md" depth=1 dir=false
```

So the new `|| entry.file_type().is_symlink()` skips only the depth-0 entry, whose sole contribution was
`hasher.update(relative)` with `relative == ""` — a no-op `update` of zero bytes. Children are unchanged
in path and order. `hash_dir(symlinked target) == hash_dir(central)` before and after, and
`target.exists()` in `global_sync.rs:81` follows the link. **No regression; the answer is "symlink entry,
still descended".** (Same reasoning covers `copy_dir_recursive` with a symlinked `source`: the depth-0
entry was already neither `is_dir()` nor `is_file()` under `follow_links(false)`, so the new `continue`
changes nothing.)

## Standards

**S1 (judgement) — `repointDoor` is a Middle Man.** `src/lib/skillPresentation.ts:58`:

```ts
return sourceKind(skill) === "git" ? "git" : "local";
```

`sourceKind` already returns exactly `"git" | "local"`, so the body is literally `return sourceKind(skill)`
and the two exported predicates are now indistinguishable at every call site. Spec D9 asked for the
predicate, so this is not a breach — but AGENTS.md's *"One implementation each of relative time, repo
grouping and storage access: extend `skillPresentation.ts` instead of adding a second …"* is the same
instinct, and a tautological second name is exactly what invites a future re-derivation. Either give it a
body that can diverge (e.g. also refuse a door for `imported` with no `source_ref`) or delete `repointDoor`
and let both call sites read `sourceKind(skill) === "git"`. `SkillCard.tsx` still computes `kind` from
`sourceKind` two lines above the `repointDoor(skill)` call, so both live side by side today.

**S2 (judgement) — the `.skills-hub-old-<uuid>` backup has no owner.** `install_finalize.rs`
`move_old_central_aside` creates a full copy of a skill inside the central repo, and the double-fault path
deliberately leaves it:

```rust
Some(backup) => format!(
    "rollback to {:?} failed: {:#}; old bytes retained at {:?} for manual recovery", …
```

Nothing ever reclaims it: `temp_cleanup.rs` only sweeps `skills-hub-git-` under the *cache* dir, and
`artifact_removal`'s `Skill` scope removes `record.central_path` only — so deleting the skill orphans the
backup permanently, with the only pointer to it in a transient error toast. AGENTS.md's *"Artifact removal
is one module"* is not literally breached (a backup is not a Sync target), but this is the one byte-producing
path in the app with no reclamation story. A `.skills-hub-old-*` sweep in `move_old_central_aside` (age-bounded,
best-effort) would close it.

**S3 (judgement) — discovery exclusion is asserted narrowly.** The new test asserts
`discover_skills(central) .is_empty()`, which exercises `skill_discovery.rs` steps 1/3 (`is_hidden_dir_name`).
Step 2, the known-scan-base loop (`skill_discovery.rs:130`), pushes **every child directory** with no hidden
check. That only bites if the operator repoints `central_repo_path` under a scan base, but the implementer's
own claim already concedes it ("Explicit known-scan-base discovery does not universally exclude hidden
children") — worth one `is_hidden_dir_name` guard in that loop rather than a comment.

No hard documented-standard breaches in the diff. i18n `errors.skillGone` is present in **both** `en` and `zh`
(`resources.ts:345`, `:897`). `commands/` untouched; no raw `invoke`; no bindings hand-edited.

## Spec

**P1 (blocking) — D4's stored-subpath shortcut mis-splits a Re-point URL.**
`installer.rs:325` passes the hint unconditionally:

```rust
stored_subpath: record.source_subpath.as_deref(),
```

but on a Re-point `source` is the **operator's new URL**, while `source_subpath` is still the **old** URL's
path. `git_acquisition.rs:288` then splits on a suffix match with no check that the two describe the same
source:

```rust
let branch = tree_path.strip_suffix(&format!("/{subpath}"));
```

Concrete: a skill installed from `…/tree/main/foo` (`source_subpath = "foo"`), re-pointed to
`…/tree/main/skills/foo`. `tree_path = "main/skills/foo"`, `strip_suffix("/foo")` → branch `"main/skills"`,
subpath `"foo"`. The fast path then asks `branch_sha` for branch `main/skills`, gets a 404 on a *named*
branch, and `classify_fast_path_failure` raises the typed `GithubSkillNotFound` — no clone retry. A valid
URL fails with "skill not found". This is exactly the brief's *"cannot pick a wrong branch when the URL path
is a legitimate deeper path"* check, and it fails. The existing test only survives because
`git_repoint_fixture`'s `source_subpath = Some("old")` is not a suffix of the test URL
(`refresh.rs:244` vs `:255`). Minimal fix: `stored_subpath: source_override.is_none().then(|| …).flatten()`
— Re-point then pays one `matching-refs` call, which D4 only promised to avoid for Refresh/Update.

**P2 (follow-up) — scope beyond ticket 03.** `issues/03-branch-with-slash.md`: *"Files: `git_acquisition.rs`,
`github_api.rs` … their tests."* `installer.rs` — on AGENTS.md's *"Highest-risk shared files"* list — instead
had its whole intent/backfill block rewritten (`known_subpath` deleted; `intent` re-derived; the pre-finalize
`store.upsert_skill(&record)` backfill removed; `install_git_skill_from_selection_with` switched from the
selection subpath to `acquired.resolved_subpath`). Wiring `stored_subpath` was in scope; the rest was not,
and the ticket asked to *"report rather than silently widen"* for exactly this kind of adjacent hole.
Behaviourally I could not find a regression: `resolve_subpath` short-circuits on `known_subpath`, so the
`Subpath` → `NamedSkillOrWholeRepo` intent change is inert whenever the URL carries a subpath, and the new
`source.subpath == Some(".")` → `Subpath(".")` rewrite in `acquire` restores what the old
`known_subpath = …or(source.subpath)` did for `/blob/<branch>/SKILL.md`. Losing the eager backfill upsert is
*better* AGENTS-wise ("acquire-first … no `upsert_skill` before finalize"); the only cost is that a legacy
record's discovered subpath is re-discovered next time if finalize fails. Report it, don't revert it.

**P3 (follow-up) — split-brain window between listing and install.** `list_git_skills` (`installer.rs:448`)
and `acquire` each call `matching-refs` independently. If the listing resolves `feature/x` and the install's
call is rate-limited seconds later, the install falls back to branch `feature` with the listing's
`skills/foo` and 404s. Loud, not silent, but D4's *"call … once"* reads like one resolution per operator
action; threading the listing's resolved `GitSource` into `install_git_skill_from_selection_with` would close it.

**P4 (follow-up) — D3 edge: an all-symlink skill hashes to the empty digest.** `find_skill_md` uses
`path.is_file()` (follows links), so a skill whose `SKILL.md` is a symlink is *discoverable* but now hashes
as if empty — two such skills compare `target_has_same_content == true`. Vanishingly rare; worth one line in
`hash_dir`'s doc rather than code.

Everything else in D1–D9 checks out. D1: all four failure paths verified — `move_old_central_aside` fails
*before* touching old bytes; move failure and upsert failure both `remove_dir_all(central)` then rename back;
rollback failure returns the original as `root_cause()` with the backup path in the context; cleanup failure
only `log::warn!`s. The collision loop is sound (a non-`NotFound` `symlink_metadata` error returns rather than
spinning). A leftover backup cannot break the next Update — each takes a fresh UUID. `finalize_install` was
correctly reported-not-widened. D2: real faults (SQLite `RAISE(ABORT)` trigger, `0o555` parent, missing
staging), no `#[cfg(test)]` hook. D3: every comparison path — `target_has_same_content`, `onboarding.rs:79`
fingerprint, `project_sync.rs:212/504`, `install_finalize.rs:372` — routes through the one `hash_dir`;
symlinked *directories* are skipped identically by both walkers (`follow_links(false)` never descends them);
symlink-free skills hash bit-identically. D5–D9: `settleSingleReport` returns `action.fail(...)` (an
`ActionExit`) on failure, `undefined` on success, so `?? true` yields `true` only on success and `runAction`
maps `ActionExit` → `undefined` (`useStatusReporter.ts:406`) — the modal stays open on failure, correctly.
The other two `runSingleRefresh` callers ignore the return. `Header.tsx:52` already treats `activeView ===
"detail"` as My-Skills-active, so the new `|| activeView === "detail"` render fallback is visually coherent;
`explore-detail` keeps its own `exploreDetailSkill` and the install handoff still clears it.

## Summary

**Blocking**
1. **P1** — `installer.rs:325` + `git_acquisition.rs:288`: the D4 stored-subpath shortcut is applied to
   Re-point, where the stored subpath belongs to the *old* URL. A legitimate deeper new URL yields a
   non-existent branch and a typed `GithubSkillNotFound`. Suppress the hint when `source_override.is_some()`.

**Follow-up**
2. **S2** — `.skills-hub-old-<uuid>` backups have no reclamation path (`temp_cleanup` and `artifact_removal`
   both miss them); add an age-bounded sweep.
3. **S3** — `skill_discovery.rs:130` known-scan-base loop lacks the `is_hidden_dir_name` guard the new test
   relies on elsewhere.
4. **S1** — `repointDoor` is a tautological alias of `sourceKind`; give it a body or drop it.
5. **P2** — ticket-03 scope creep into `installer.rs` (a top-risk shared file); no regression found, but it
   should have been reported, not absorbed.
6. **P3** — two independent `matching-refs` resolutions per Add; thread the listing's resolved source into install.
7. **P4** — `hash_dir` doc should note that a skill made only of symlinks now hashes as empty.
