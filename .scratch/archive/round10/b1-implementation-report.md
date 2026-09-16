# B1 implementation report

## Checkout and commits

- Checkout: `~/Projects/skills-hub/.scratch/round10/worktrees/git-resolution`
- Branch: `r10b/git-resolution`
- Base: `a824caf45fffb0bd5c38318ba10b2ffa0ca11c62`
- Characterization-first commit: `342995f` — `test(git): characterize full listing interface before resolution refactor`
- Implementation commit / HEAD: `b4637b04b241848dde4f8621d64b9f03af3172f9` — `refactor(git): own source resolution and atomically publish previews`
- No merge, rebase, push, release, tag, workspace handoff, or children. Only this checkout was modified. No live app/dev process or operator library/database access. Tests use temporary roots, SQLite stores, scripted APIs and local git fixtures. All Cargo invocations used `CARGO_BUILD_JOBS=4`.
- This report and evidence are gitignored under this checkout's `.scratch/round10/`; nothing was force-added.

## Interface and locality

`core/git_acquisition.rs` now owns:

```rust
pub enum SkillIntent<'a> {
    StoredRecord { name: &'a str, subpath: Option<&'a str> },
    Selection(GitSelection<'a>),
    ByName(Option<&'a str>),
}
pub struct GitSelection<'a> {
    pub subpath: Option<&'a str>,
    pub resolution: Option<&'a GitSourceResolution>,
}
pub struct GitSourceResolution {
    pub branch: Option<String>,
    pub subpath: Option<String>,
}
pub fn acquire(req: &AcquireRequest, api: &dyn GithubApi) -> Result<Acquired>;
pub(crate) fn list_candidates_with(
    source: &GitSource,
    api: &dyn GithubApi,
    checkout: impl FnOnce(&GitSource) -> Result<PathBuf>,
) -> Result<(Vec<DiscoveredSkill>, GitSourceResolution)>;
```

- `AcquireRequest.stored_subpath` is removed: only `StoredRecord` supplies that knowledge.
- Listing constructs `Selection` with no selected subpath **inside acquisition**. The listing door returns admitted candidates and source resolution, not a split protocol. Its injected full-checkout adapter preserves listing → install cache reuse without copying the entire repository to a new staging directory.
- `core/git_acquisition/resolution.rs` is private. It owns effective intents, parser/stored-hint/matching-refs state, supplied-resolution reuse, suffix repair, and assumed-branch failure classification. No caller can construct `TreeSplit`, invoke `resolve_tree_source`, choose `acquire_resolved`, or choose a repair. The latter bypass is removed entirely. See `evidence/resolution-locality.log`.
- One private `installable_candidates` admission fold over `skill_discovery` serves listing and acquisition name matching. The existing root/nested selection semantics for strict and legacy intents remain.
- `installer::list_git_skills_with` only projects DTOs and matches the optional target name; it no longer discovers/filters/scopes candidates itself.
- `install_git_skill_from_listing(..., selection: GitSelection, ...)` calls the single `install_git_selection_with` path. No tuple, coordinate patch, null-resolution branch or acquisition bypass remains in production.
- `skill_update::acquire_update` passes `StoredRecord` for Update/Restore and `ByName` for Re-point. Its admission/finalize/propagation protocol is unchanged; repaired subpaths still only ride in `UpdateRequest.record` until finalize.
- Explore uses `ByName`. The private `clone_for_explore_preview_with(..., acquire_to: impl FnOnce(&Path) -> Result<()>)` seam owns cache publication. A unique sibling `StagingDir` receives bytes outside the preview mutex; only completed, nonempty bytes are renamed under the short lock. A loser returns the winner without replacing it; RAII removes only its private directory. No lock is held across acquisition/git-cache work.

## Q5 characterization first, old outputs and intended delta

Before **any production edit**, `listing_interface_characterization` passed through `list_git_skills_with`, using a scripted refs API and injected filesystem checkout. Its literal ordered candidate tuples include `(name, description, repo-relative subpath, resolution)` plus `target_match`.

The exact old outputs are durable in `evidence/q5-old-green.log` and the original literal assertions in commit `342995f`. Cases cover root-only; root+nested; folder container; folder skill with nested skill; missing folder; slash-branch coordinates; malformed/missing manifests and `.claude/skills` exception; ambiguous name; unmatched name; and Unix aliases collapsing to the real-directory identity.

Intentional Q5 delta (recorded next to the changed assertion, not silently re-baselined):

```text
URL owner/repo/tree/main/pack, target Alpha
OLD candidates:
  [Pack, pack docs, pack, {branch: main, subpath: pack}]
OLD target_match: {kind: none}
NEW candidates (ordered):
  [Alpha, child docs, pack/skills/a, {branch: main, subpath: pack}]
  [Pack, pack docs, pack, {branch: main, subpath: pack}]
NEW target_match: {kind: resolved, subpath: pack/skills/a}
```

The folder root no longer suppresses nested candidates. Repository roots, URL scope, ordered metadata, repo-relative paths, missing-folder empty listings, installable malformed manifests, `.claude/skills` admission, alias identity, name ambiguity and unmatched results are preserved. Explicit root blob URLs likewise use the common candidate admission rule for listing; an explicit install/root byte intent stays explicit.

## Behavioral red/green evidence

All commands below were run from the checkout above. Rust commands used `CARGO_BUILD_JOBS=4`.

| Check / command suffix | Behavioral evidence | Logs under `evidence/` |
|---|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml listing_interface_characterization -- --nocapture` | Old implementation green first. Deliberately dropped candidate resolution: failed `root-only` with literal expected resolution object vs actual null. Restored old source green before production refactor. | `q5-old-green.log`, `q5-lost-resolution-red.log`, `q5-restored-green.log` |
| Same characterization command | Changed only the intentional folder+nested expected output first: old source failed `folder-skill-with-nested`, with Pack-only/none vs Alpha+Pack/resolved. New code green. | `q5-candidate-delta-old-red.log`, `q5-final-green.log` |
| `cargo test --manifest-path src-tauri/Cargo.toml explore_preview_` | Old publication algorithm with the new adapter kept compiling: 3 behavioral failures (partial return, error residue, cancelled publication). New source green. Restored old algorithm after implementation: same 3 failures, then restored new code green. | `preview-old-red.log`, `preview-new-green.log`, `preview-reversed-red.log`, `preview-restored-green.log`, `preview-final-green.log` |
| `cargo test --manifest-path src-tauri/Cargo.toml explore_preview_empty_acquisition_is_not_published` | Old algorithm returned `Ok(path)` for an empty acquisition; new code refuses and leaves no entry. | `preview-empty-old-red.log`, `preview-final-green.log` |
| `bun run test -- src/hooks/useAddSkillFlow.test.ts` | Deliberately replaced both resolution arguments with null: 4 forwarding tests failed (non-null and deliberate-default objects, each direct and picker). Restored source: all 27 passed. | `frontend-forwarding-red.log`, `frontend-restored-green.log` |

Concurrency test uses channels only, no sleeps. It pauses the first writer after its first file, invokes a same-key second caller, observes that it also enters a **different private destination**, then releases the second first. Each caller captures both file contents **at return**; both must see the second writer's completed bytes at the same public path. The first/loser cannot overwrite the winner, and only one public entry remains. Thirty-second deadlines only bound hangs. Existing cache-hit/no-git-cache and real local-git miss/no-deadlock tests remain green.

Failure preservation:

- `listing_and_install_share_one_refs_resolution_and_old_selection_still_resolves`: reused listing requires only one refs resolution; legacy null selection still resolves.
- New explicit-default-resolution test overrides wrong parser coordinates, makes no refs call, and uses default-branch clone fallback on assumed-main SHA 404.
- New supplied-explicit-branch SHA 404 test proves no repair, no clone, no destination. Existing explicit selection identity test still passes.
- Stored-hint SHA 404 permits one matching-refs retry only; download 404 never retries refs. Discovery 403/404/500/no-match remains best-effort; acquisition 403/404 remains typed and never clones outside the approved assumed-main exception.
- Expanded `stale_split_repair_is_acquire_first_and_persisted_only_by_finalize`: SQL trigger permits rollback of the old row but rejects the repaired write; failed guarded finalize preserves original row and bytes. A repaired acquisition followed by download 404 also preserves both. Removing the fault and retrying persists the corrected subpath/revision only at finalize.
- Actual SHA, alias identity, traversal/symlink refusal, cache widening, cancellation, Restore/Re-point and existing Update/Refresh tests pass.

## Wire/frontend

- Removed Serialize-only `#[serde(default)]` from `GitSkillCandidate.resolution`.
- Cargo's binding generator produced `resolution: GitSourceResolution | null` (required nullable). `branch`/`subpath` remain nullable; command input remains optional-compatible. Binding diff is this optional marker and a DTO doc comment only.
- Skills shim exports `GitSourceResolution`.
- The only hook implementation edits remove the two redundant `?? null` expressions.
- Candidate fixture helper supplies explicit null; direct and picker tests each cover non-null resolution, deliberate `{branch:null, subpath:null}`, and legacy null.

## Final verification

Final fresh commands, all successful:

```sh
export CARGO_BUILD_JOBS=4
npm run version:check && npm run check
cargo test --manifest-path src-tauri/Cargo.toml --all
cargo test --manifest-path src-tauri/Cargo.toml git_acquisition
cargo test --manifest-path src-tauri/Cargo.toml installer
cargo test --manifest-path src-tauri/Cargo.toml skill_update
bun run test -- src/hooks/useAddSkillFlow.test.ts
git diff --check
```

- Version gate: **Version OK (1.2.11)**. Ticket targets future 1.2.12; no version/changelog edits were made.
- Full gate: ESLint, Vitest **14 files / 288 tests**, repository TypeScript-7 build, Vite build, Rust fmt check, clippy all-targets/all-features with `-D warnings`, Rust **621 tests**, binary/doc targets all pass.
- Independent explicit `cargo test --all`: **621 passed**, no failures. Baseline before characterization was 614 tests.
- Target filters: acquisition **50**, installer **37**, Update **10**, AddSkillFlow **27** (was 22).
- Logs: `version-check.log`, `full-check-final.log`, `cargo-all-final.log`, `acquisition-final.log`, `installer-final.log`, `update-final.log`, `frontend-restored-green.log`.
- One late gate rerun caught a leftover blank line after deleting a duplicate assertion. `cargo fmt` fixed it; both full gate and `cargo test --all` were rerun green. Historical failure is in `full-check-format-failure.log`.
- Active LSP probe of all **12 changed absolute paths**, concurrency 4 / 2-second per-file budget: **10 clean, 2 with auxiliary findings, 0 inconclusive/unavailable/failed**. No blocking errors. The 13 auxiliary warnings/hints concern pre-existing extensionless imports, a command re-export, an existing filter/map, and existing Record annotations; no unrelated cleanup performed.
- Final `lens_diagnostics mode=all` also run: no errors, existing hook complexity/style warnings; its session-cache view identifies the harness root and omits stale entries. The explicit absolute-worktree-path LSP probe and fresh compiler gates, not the session-cache view, establish coverage. No full-workspace LSP sweep or live UI verification was claimed.

## Review, deviations, and residual risks

- Manual Standards and Spec reviews performed against the assigned base and ticket. No additional reviewers/children launched: parent owns the independent integration/review cycle. No blocking finding remained in self-review.
- Kept the old null-resolution install convenience functions under `#[cfg(test)]` for existing installer/Update/Refresh fixtures. They are absent from production; production has one named selection path. This avoids unrelated Refresh-test ownership changes.
- Reused existing `StagingDir` for private preview scratch, rather than adding a new guard or promoting the dev-only tempfile dependency. Publication uses `std::fs::rename` directly (no copy fallback, which could expose partial bytes).
- **Small preview behavior delta:** an acquisition returning no non-`.git` content is now typed `SkillInvalid { reason: missing_skill_md }` and is not published. Otherwise an empty completed entry would be mistaken for a cache miss and could be removed beneath a returned reader. Existing cache-hit policy stays non-`.git` content; this is not a TTL/eviction redesign.
- Existing nonempty cache entries remain trusted as requested. This does not migrate or validate a partial cache entry left by an older app version. Process-crash cleanup of private staging directories remains outside this ticket; RAII cleanup is tested for ordinary error/cancel/loser paths.
- Verification is local macOS plus scripted/local fixtures, not live GitHub or a Windows runtime. Existing Vite ineffective-dynamic-import / large-chunk warnings remain unchanged.
- No B2 manifest/parser policy, B3 envelopes/report folding, registration/i18n, broad Update cleanup, version, AGENTS/CONTEXT/ADR, cache TTL, or target propagation changes. Parent owns domain documentation and integration.
