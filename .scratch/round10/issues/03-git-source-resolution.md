# 03: Wave B1 — Source resolution inside Git acquisition, with atomic Explore preview publication

Status: resolved

**What to build:** Add listing, selected install, Update/Restore/Re-point and Explore preview agree on the git source and candidate rules. A listing's resolved source is reused, legacy wrong splits heal only when finalized, and no preview caller can observe a half-acquired cache entry.

**Blocked by:** None (can start immediately from integrated wave A). B1, B2 (#04) and B3 (#05) are logically independent; parent isolates worktrees and owns final integration/review.

**Completed:** integrated on `main`; final reviewed candidate `23458bb`. Parent verification and two Opus passes: `.scratch/round10/wave-b-execution.md`, `b1-implementation-report.md`, `r10b-review-opus.md`, `r10b-review-opus-recheck.md`. v1.2.12 prepared, not published.

**Release / lane:** v1.2.12 / B1 (#3 + #8 rider). Source verified on `main` at `a824caf45fffb0bd5c38318ba10b2ffa0ca11c62`. Implementer: Pi `openai-codex/gpt-6-astra`, xhigh. Review: one model seat, Anthropic Opus 5 high (temporary replacement for exhausted Fable); parent discovers the exact available model ID.

## Read first / working contract

Read `AGENTS.md`, `CONTEXT.md`, ADR-0001/0003/0004, `.scratch/round10/decisions.md` (Q5/Q10/Q20), the wave-A tickets 01/02 as history, and `.scratch/round7/backlog.md` (especially 2/3/12/13). Use `implement`, `codebase-design`, `pi-lens-lsp-navigation`, and `tdd`; use React best-practices for the narrow hook changes. State your approach and continue: operator pre-approved implementation, no confirmation pause. Search only inside your project checkout. No live app/dev runs or operator library/DB access. Use temp roots, temporary stores and scripted/local git fixtures. No children. Parent owns commits/integration/release unless separately authorized. `.scratch/` is gitignored; NEVER `git add -f`.

If the ticket is factually wrong about the code, say so in your report and adapt.

Prefer targeted edits and re-read after writing. Symbol anchors below are the authority, not stale historical line numbers.

## Problem / verified evidence

- `core/git_acquisition.rs::SkillIntent` currently has `Subpath`, `NamedSkill`, `NamedSkillOrWholeRepo`; `AcquireRequest` separately exposes `stored_subpath`. `acquire` resolves and then calls `acquire_resolved`; `retry_stored_hint`, `TreeSplit`, `resolve_tree_source` and `classify_fast_path_failure(..., branch_assumed)` encode different parts of resolution.
- `core/skill_update.rs::acquire_update` chooses strict name intent for Re-point, explicit subpath for stored records, or lenient name/whole-repo backfill. It separately passes the stored suffix. Corrected subpath is proposed in `UpdateRequest.record`; only `apply_unlocked`/`install_finalize::finalize_update` persists it.
- `core/installer.rs::list_git_skills_with` calls `resolve_tree_source` directly, then a checkout closure. `git_candidates_in` discovers/filter/rebases candidates itself; `git_acquisition::installable_skills_in_repo` separately supplies root + nested acquisition candidates. Both already filter with `Validity::is_installable`; duplication is NOT simply valid-versus-invalid filtering. Listing suppresses nested candidates for a folder URL whose scoped root is itself a skill. Capture this behavior before replacing policy.
- `install_git_selection_with` patches parsed coordinates and bypasses resolution through `acquire_resolved(..., TreeSplit::Parser)` when a listing resolution exists. `install_git_skill_from_listing` still branches between two wrappers and passes a tuple selection.
- `GitSkillCandidate.resolution` in `installer.rs` still has `#[serde(default)]`; generated binding is `resolution?: GitSourceResolution | null`. `useAddSkillFlow.ts` passes `candidate.resolution ?? null` in the picker adapter and `chosen.resolution ?? null` in `handleCreateGit`. `components/skills/types.ts` does not re-export `GitSourceResolution`.
- `clone_for_explore_preview` holds `EXPLORE_CACHE_LOCK` only for probe/prepare. It creates the final path, releases the lock, then acquires into that public directory. A hit currently means any non-`.git` entry; partial bytes can therefore look complete.

## Settled decisions / implementation

1. **Q5: characterization FIRST.** Add executable listing characterization through the listing interface, before changing production listing/resolution. Record literal ordered candidates (name, description, repo-relative subpath, resolution) and `target_match`: root-only, root+nested, folder skill/container, missing folder, malformed manifest, missing manifest, `.claude/skills` exception, aliases, ambiguous/unmatched name, and slash-branch coordinates. Existing private-helper tests are useful fixtures, not a substitute for the full listing seam. Record old outputs and explicitly identify intentional Q5 deltas when adopting acquisition's rule; do not silently rewrite assertions to whatever the refactor produces.
2. **Q20: one deep module.** Extend `SkillIntent` to `StoredRecord` / `Selection` (carrying optional `GitSourceResolution`) / `ByName`. Listing is `Selection` without a selected subpath, not a separate resolution protocol. Keep resolution state, stored-hint repair and assumed-branch policy private to a `resolution` submodule of `git_acquisition` (`core/git_acquisition/resolution.rs` is a natural location). Callers must not construct `TreeSplit`, select `acquire_resolved`, or independently choose repair rules. Acquisition owns git candidate admission (root vs nested included); listing projects candidates and matches a requested name without independently selecting a skill or maintaining a competing discovery rule. Keep the common discovery scan ladder in `skill_discovery`.
3. Preserve listing's full checkout/cache reuse, URL scoping and repo-relative subpaths; explicit selections remain explicit. `Selection` with a supplied resolution reuses it, including `branch: null` as a deliberate default-branch choice. A legacy caller without resolution still resolves. Replace the tuple/wrapper branch with a named selection and one install call. Move the resolution DTO to the owning module if useful without changing its branch/subpath wire spelling.
4. Remove the Serialize-only candidate's `#[serde(default)]`; regenerate bindings to required `resolution: GitSourceResolution | null`. Remove both redundant frontend fallbacks, re-export the DTO from the skills shim, update candidate fixtures with explicit null where needed. Keep the command's optional resolution input compatible.
5. Preserve error distinctions: explicit-branch SHA 404 and content 404/403 remain typed and never fall back to clone. Existing exceptions remain: assumed `main` SHA 404 can use clone default branch; a **stored-hint SHA** 404 can perform one matching-refs repair/retry. Discovery itself remains best-effort; download 404 does not retry refs; independent explicit selection must never be repaired as if it were a stored hint. No pre-finalize upsert on a successful repair, and failed acquisition/finalize leaves original row/bytes intact. Preserve cancellation, actual revision SHA, symlink refusal/alias identity and cache widening behavior.
6. **Q10 rider:** acquire previews in a unique private temporary sibling of the final cache key. Publish only a completed acquisition using rename under the short preview-cache lock. Concurrent same-key callers may acquire separately; the loser must reuse the completed winner and clean its private bytes, not delete/overwrite an entry another caller is reading. Error/cancellation leaves no public partial entry; clean private scratch on failure. Keep the cache-hit/no-git-cache behavior and do not hold this lock across acquisition or compose it re-entrantly with the git-cache lock. No preview TTL/eviction redesign.

## Ownership / explicit do-not-touch

Paths below are relative to `src-tauri/src/` unless prefixed `src/`.

- **Own:** `core/git_acquisition.rs`, new private `core/git_acquisition/resolution.rs`, `core/tests/git_acquisition.rs`; git listing/selection/preview symbols and DTOs in `core/installer.rs`, related tests in `core/tests/installer.rs`.
- **Narrow allowances:** `core/skill_update.rs::acquire_update` only intent/request construction and resolved result handoff; `core/tests/skill_update.rs` only git acquisition/split compatibility tests. `core/test_git_api` is actually wired to `core/tests/git_stub.rs`; change that stub only if needed for new resolution cases. `commands/mod.rs::install_git_selection` only resolution type/import/named selection call. `core/mod.rs` only an acquisition-related declaration if genuinely required (a private nested submodule normally needs none).
- **Frontend allowance:** `src/hooks/useAddSkillFlow.ts` only the two git resolution arguments/imports; `useAddSkillFlow.test.ts` only git candidate/forwarding fixtures. Other candidate fixtures only compiler-required explicit-null updates. `src/components/skills/types.ts` only `GitSourceResolution` export. `src/bindings/index.ts` only generator output.
- **B2 owns:** `core/manifest.rs`, manifest parsing/writing, `skill_discovery` parser internals, `install_finalize` metadata read, `skill_lock`, `skill_edits` manifest imports/calls, and `SkillDetailView` fence follow-up. Do not change their policies. B2 may touch `installer.rs` manifest imports; retain compatibility re-exports or reconcile those imports at parent integration.
- **B3 owns:** mutation envelopes/commands, `useSkillLibrary`, `reportOutcome`, mutation tests, i18n and registration. Do not change refresh DTOs/return shapes, Edit settlement, Update admission, catalog or notification policy.
- **Inevitable overlap:** generated bindings and the skills type shim with B3; `commands/mod.rs` separate symbols with B3/B2; installer imports and installer test file with B2. Do not take an entire shared file from one worktree. B3 should need no `useAddSkillFlow` changes; any requested expansion is an ownership handoff, not implicit permission.
- **Excluded:** content-identity follow-ups, broad `UpdateRequest` cleanup, malformed `global_selected_tools`, round11 C3/C4, wave C #7 wire-report unification, #9, versions/changelog, `AGENTS.md`, `CONTEXT.md`, ADR amendments and unrelated module cleanup. Parent owns domain/invariant edits.

## Acceptance

- [x] Characterization ran green on old source before production refactor, with saved expected outputs and explicit intentional candidate-rule deltas.
- [x] All four consumers use acquisition's intent interface; branch split/repair/assumed-branch and candidate policy no longer leak into caller protocols.
- [x] Listing→install performs one refs resolution; null resolution remains supported for old callers; explicit selection is never silently retargeted.
- [x] Stored wrong split self-heals only at successful finalize; typed refusal, no-clone and cancellation cases are preserved.
- [x] Required nullable candidate resolution is correct in generated bindings and both frontend install paths forward it.
- [x] Concurrent previews never return partial bytes; failures/cancellation never seed a false cache hit; completed winner is retained, scratch cleaned, no deadlock.
- [x] Independent worktree gate passes with no dependency on B2/B3 or excluded work.

## Tests / gates / red-green proof

Use existing temporary `InstallerPaths`/`SkillStore`, local committed git repos and scripted `GithubApi`. Verified starting seams: `list_git_skills_with` (checkout injection), listing→install `CountingRefsApi`, `acquire` with scripted API, and `skill_update::acquire_update`→guarded `apply_unlocked` for finalize-only repair. Migrate resolution helper tests to the intent interface rather than preserving an externally visible private implementation just for tests.

Keep/extend `listing_and_install_share_one_refs_resolution_and_old_selection_still_resolves`, `git_candidates_*`, `stale_stored_split_*`, `stored_split_download_404_does_not_retry_refs`, `stale_split_repair_is_acquire_first_and_persisted_only_by_finalize`, and preview hit/deadlock cases. Add an injected preview acquisition adapter with channels/barriers: pause after the first private file, invoke a second same-key caller, then prove neither returns incomplete output; release completion and assert both see the complete winner. Cover error and cancellation separately. Deadlines may bound hangs; sleeps must not establish ordering. Do not use real HTTP or live library paths.

Prove the test bites: revert your source change, confirm the test fails, restore.

For pure behavior-preserving characterization/refactoring, **green on old code is expected**. Prove those assertions with a deliberate temporary behavior mutation (e.g. retain missing candidates, discard resolution, choose a wrong split), observe the targeted assertion failure, restore. For preview publication and intentional listing-rule changes, demonstrate real old-source red → new-source green. Compile errors alone do not prove behavioral sensitivity: keep the test adapter compiling for the reverse check. Re-run after restoring and report exact commands/results; leave no mutation behind.

Run targeted Rust suites (`cargo test --all core::tests` is not their module path: use filters `installer`, `git_acquisition`, `skill_update`), relevant Vitest file(s), `cargo test --all`, then `npm run version:check && npm run check`. Build uses the repository's TypeScript-7 command, not bare `tsc`. Check changed paths with active LSP when available, and `lens_diagnostics mode=all` at end; report unavailable coverage honestly. Regenerate bindings via `cargo test`, review drift and re-read changed files. Parent re-runs the full gate after integration.

## Completion report

List exact intent interface and locality gained, characterization evidence/intentional deltas, preserved failure policies, preview concurrency red/green evidence, gate output, ownership deviations, worktree/branch/HEAD and remaining risks. Parent will add **Source resolution** to `CONTEXT.md` and update acquisition invariants separately.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
