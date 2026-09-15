# Round-9 panel — Fable seat (fresh explore, `main` @ 96cd33c)

Read-only. Walked AGENTS.md, CONTEXT.md, ADRs 1–3, the last 150 commits, and the round-6–8 hot spots. Candidates ranked; each passed the deletion test unless stated.

---

## 1. Skill content identity has one supplier

**Files**: `core/install_finalize.rs:469-486`, `core/skill_edits.rs:74-78`, `core/project_sync.rs:206-215, 500-508`, `core/propagation.rs:377-390`, `core/global_sync.rs:80-90`, `core/content_hash.rs`.

**Problem**: "what is this skill's content hash right now" is answered by five callers with three policies. `install_finalize::compute_content_hash` is gated by `cfg!(debug_assertions) || SKILLS_HUB_COMPUTE_HASH` (→ `None` in a release build); `skill_edits::record_hash` hashes unconditionally; `project_sync::hash_source` hashes on demand and logs failure; `observe_assignment` hashes-and-backfills; `global_sync::target_has_same_content` hashes both sides. The consequence hides in how they are called: in a release build, Update → `finalize_update` records `content_hash: None` → Propagation supplies `None` to `sync_assignment_target` for every copy-mode row → the next reconcile pass backfills the *source* hash (`observe_assignment`) but the row's `recorded_hash` is `None` → `next_status` (`sync_status.rs:172-177`) returns **`Stale`**, permanently, for every copy-mode project assignment after every Update — unless the skill happens to carry an Edit, in which case `replay_unlocked`'s ungated `record_hash` fixes it. Tests never see this: `cfg!(debug_assertions)` is true under `cargo test`, and `tests/propagation.rs:113` passes `Some("hash-v2")` in by hand. Copy mode is exactly the Windows-without-symlink-privilege path.

**Solution**: One identity module — "the hash of a central copy" computed once per finalize/edit and read from the record everywhere else — with the compute policy (if any is still wanted; the gate predates hashed reconcile) owned there. Propagation, the reconcile backfill and the same-content check read one answer instead of each deciding when to hash.

**Benefits**: Locality — the release/debug divergence disappears; one place to reason about "hash absent". Test — the finalize→propagate→reconcile chain becomes testable through one interface with the real supplier, not a hand-fed string. Leverage — `a041ff4` (exclude internal symlinks from identity) would have been one call site to reason about, not five.

**Evidence**: Deletion test on `compute_content_hash`/`hash_source`/`record_hash`: delete them and the hashing reappears at five sites — they earn their keep but split one rule. Commits paying: `a041ff4`, `fb71b1b` (touched `content_hash.rs` + `skill_edits.rs`), round-3 ticket 04 (the closure-supplier decision, which fixed the *rule* but left the *supply* divergent — its own text notes the `None`-in-release case).

**Strength**: Strong (latent bug, not only friction).

---

## 2. Git source resolution is one module, and the git listing goes through it

**Files**: `core/git_acquisition.rs:219-370` (`acquire`, `acquire_resolved`, `resolve_tree_source`), `:386-400` (`fast_path_coords` assumes `main`), `:462-509` (`classify_fast_path_failure(branch_assumed)`), `:707-830` (parsers), `core/installer.rs:296-330` (Update composes intent/hint), `:449-512` (`list_git_skills`, `git_candidates_in`), `core/git_acquisition.rs:676-692` (`installable_skills_in_repo`).

**Problem**: "Which branch, which subpath, and what does the caller mean by it" is spread across the parser's first-segment split, `resolve_tree_source` (stored-hint suffix → matching-refs → parser), the S3 repair inside `acquire_resolved`, the `main` guess in `fast_path_coords`, and `branch_assumed` in the failure classifier. The caller must know invisible coupling: the S3 repair fires only when `intent == Subpath(stored_subpath)` (`:264-272`), so `installer.rs:307-321` builds `intent` and `stored_subpath` from the same column in lock-step and strips both on override (`f48c001`). Meanwhile `list_git_skills` bypasses `acquire`: it calls `resolve_tree_source` itself, then `fetch_through_cache` directly, then its *own* candidate rule (`git_candidates_in`: subpath-prefixing, "folder is the skill" retention) that differs from acquisition's `installable_skills_in_repo` + `resolve_subpath`. Add and Update can disagree about the same repo.

**Solution**: A resolution module inside acquisition that owns the whole split ladder (parser, hint, refs, assumed-branch, self-heal) and answers once with a resolved source plus how it was resolved; callers state *what they want* (a stored record, an operator selection, a name) rather than assembling intent + hint + override themselves. The Add listing becomes a third adapter over the same resolved source and the same candidate rule.

**Benefits**: Locality — six of the last 40 fixes touched this ladder (`93ee36e`, `6b0ec7d`, `b5e398b`, `364f1bc`, `f48c001`, `0779a41`), most editing 2–3 files each. Leverage — backlog #2 (double `matching-refs`) falls out for free once listing and install share one resolution. Test — the scripted `GithubApi` already exists; the listing would finally be tested through it.

**Evidence**: Deletion test on `resolve_tree_source`: complexity reappears in both `acquire` and `list_git_skills` — real, but its interface (`TreeSplit` origin leaking out so `acquire_resolved` can branch on it) is nearly as complex as its body. Round-8's "extract S3 block" cosmetic is the symptom, not the cure.

**Strength**: Strong.

---

## 3. Update is one module (acquire adapter, apply, both Re-points, and the Edit write)

**Files**: `core/installer.rs:215-400` (`AcquiredUpdate`, `acquire_managed_skill_update_{with,from}`, `finalize_and_propagate_unlocked`), `core/refresh.rs:181-254, 456-488`, `core/unlocatable.rs:83-133`, `core/skill_edits.rs:80-146`.

**Problem**: Understanding "Update" means reading `installer.rs` (which is also Add, listing, Explore preview and description backfill), `refresh.rs` (pool + guard + reassert), `unlocatable.rs` (local Re-point), `install_finalize.rs`, `skill_edits.rs`, `propagation.rs`. The two Re-points have opposite atomicity: git carries the new source in memory until finalize (`installer.rs:262-272`), local writes `source_ref` first and then runs the batch (`unlocatable.rs:96-104`) — a failed local Update leaves the record re-pointed. `set_invocation_override` is a third "change central bytes, then propagate" path that re-implements finalize-and-propagate's tail and **logs** propagation failures (`skill_edits.rs:132-139`) instead of returning them as report data — the one place the fan-out rule is broken. `acquire_managed_skill_update_with` is a pure pass-through (deletion test: passes).

**Solution**: One Update module whose interface is "bring this Managed skill's central copy to these bytes and settle its targets", with acquisition (git/local/override) as an internal adapter, the apply tail (finalize → replay Edits → propagate → reassert) as its body, and the Edit write as one more caller of the same tail. `installer.rs` returns to being the Add flows.

**Benefits**: Locality — one atomicity rule for Re-point; one report shape for every central-bytes change. Test — `tests/refresh.rs` (1704 lines) and `tests/installer.rs` (1418) split the same pipeline by file rather than by behaviour.

**Evidence**: `a261e89`/`fb71b1b` (Edit V1) touched `installer.rs`, `refresh.rs`, `skill_edits.rs` and `skill_catalog.rs` to add one step to the tail; `95b7893` and `8666f8e` each threaded the reassert policy through another door.

**Strength**: Worth exploring (Strong for the `set_invocation_override` report leak alone).

---

## 4. The SKILL.md manifest is parsed once

**Files**: `core/skill_discovery.rs:322-490` (`parse_skill_md_with_reason`, `parse_invocation_mode`), `core/frontmatter_edit.rs:70-125` (`header_end`, `read_invocation_lines`), `:49-60` (`InvocationLines::mode`), `core/install_finalize.rs:462`, `core/skill_lock.rs`.

**Problem**: Three independent line-walkers each re-decide where frontmatter ends and what a value is; `InvocationLines::mode()` reconstructs a synthetic document from saved lines and re-parses it to learn its own mode — the tell of a missing shared representation. Backlog #10 (block-scalar `---`) already needs all three changed together; that is the recurring cost, not a one-off.

**Solution**: One manifest module that parses a `SKILL.md` into a structured header (keys with their line spans and raw bytes) and offers name/description/validity/invocation reads and byte-preserving key writes on that structure; discovery, finalize, Edit and skill-lock enrichment become readers.

**Benefits**: Locality for every future frontmatter key (Edit V2 will add one). Test — one parser test surface instead of three.

**Evidence**: Deletion test on `header_end`: passes (a 10-line duplicate of the fence logic in `parse_invocation_mode`). Commits: `fb71b1b`, `a261e89`, `6cb48a0`.

**Strength**: Worth exploring.

---

## 5. Refresh-report presentation is a pure fold, not hook closures

**Files**: `src/hooks/useSkillLibrary.ts:169-330` (`skillFailureEntries`, `targetFailureEntries`, `skippedEntries`, `editConflictEntries`, the two `successToast` rules), `:555-600` (`settleSingleReport`, `runSingleRefresh`), `:659-703` (`handleRepointSkill`).

**Problem**: ~200 lines of pure `RefreshReportDto → {toast, errors, warnings}` interpretation live as `useCallback`s, testable only through the 1174-line hook test with a mocked seam. The rule already forked: `runSingleRefresh` reloads in `finally` and turns an edit conflict into a warning toast; the local Re-point path (`:685-700`) reloads only on success and ignores edit conflicts. `handleRefresh` and `runSingleRefresh` compute the same conflict/failed/skipped precedence twice.

**Solution**: One pure module in `src/lib/` (the `skillPresentation.ts` pattern) that folds a report into a completion toast plus error/warning entries for both the batch and the batch-of-one; the hook only invokes and applies. Both Re-points, Update, Restore and Refresh (all) call it.

**Benefits**: Test — table tests over report shapes, no `renderHook`. Locality — one precedence rule; the local Re-point divergence closes by construction.

**Evidence**: `a403359`, `9fa3e4c`, `fb71b1b`, `12f5d91` each edited these closures; three of them also edited the hook test to reach them.

**Strength**: Worth exploring.

---

## Top recommendation

**#1 first** — it is small, it is a shipped behaviour divergence between the tested build and the released one, and its fix (one supplier read from the record) is the prerequisite for testing #3's tail honestly. **#2 second**: the branch/path ladder is where the last three rounds spent most of their fix budget, and folding the Add listing through it retires backlog #2 as a side effect.

## What I would not deepen

The `GithubApi` trait (`git_acquisition.rs:156`) and the `AcquireFn` seam in `refresh_managed_skills_with` are real seams — production HTTP plus a scripted stub, and the 1700-line acquisition/refresh suites are the proof they pay back. `mutation_guard` (`serialized`/`try_serialized`, private mutex) is as deep as a lock gets. `artifact_removal` is the model the rest should copy: scopes in, report out, one presence rule. `sync_assignment_target`'s hash-supplier closure was the right call in round 3 — the rule lives once; #1 fixes the *supply*, not the seam. `invokeTauri` + generated bindings + `describeCommandError` (ADR-0001) is one deep interface end to end; leave it. No ADR needs reopening for any candidate above.
