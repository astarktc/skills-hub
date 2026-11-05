# 28 — Executable bits survive the GitHub Contents fast path

Status: done — cd33e1e
Blocked by: —
Blocks: 19 (1.3.0 push)
Source: operator, 2026-10-27, via another agent's report (UNI-163; AU handoff 2026-10-24 §5 on the work Mac). Added to
1.3.0 by the operator. It was opened here directly rather than as a BACKLOG line.

## Problem

Skills acquired through the **GitHub Contents API fast path** land in the central repo without their executable bits,
and every Refresh strips them again.
- Example: `impeccable/scripts/impeccable` is `100755` in git but `-rw-r--r--` in the central repo on both Macs.
- Effect: a `SKILL.md` step like `<skill-base-dir>/scripts/impeccable context` dies with "Permission denied", and the
  skill's context load is silently skipped.
- Scale on the work Mac: 254 of 527 files under `~/.skillshub/*/scripts/`.

**Cause** (verified 2026-10-27):
- `core/github_download.rs::download_dir_recursive` writes each file with `std::fs::write`, so it gets the default
  mode. The Contents API carries no modes.
- The clone path keeps modes, which explains the mixed population.
- The copy is not at fault: `sync_engine::copy_dir_recursive` uses `std::fs::copy`, which preserves modes on Unix.
- `install_finalize::finalize_update` replaces the central dir on every Update, which is why a hand `chmod` never
  survives a Refresh.

**Upstream** (`qufei1993/skills-hub` @ `1647138`, 2026-10-16) has the same `fs::write`, and no issue or PR exists.
Consider filing one after the fix.

## Work

- **Faithful modes first.** After the Contents download, make **one** request:
  `GET /repos/{owner}/{repo}/git/trees/{ref}?recursive=1`, with the same `ref` the downloads used, and the same token
  and error classification as the other GitHub GETs.
  - For every file written, look up its tree entry by repo path. Use `item.path`, which already accounts for a
    followed upstream symlink (`LinkChain`).
  - For mode `100755`, add the execute bits (`mode | 0o111`). Unix only; Windows is a no-op.
- **Fallback.** If the trees call fails for any reason (403 rate limit, 404, network) or answers `truncated: true`,
  give `+x` to any written file whose first bytes are `#!`.
  - **Never fail an acquisition over modes**: the bytes have landed. Log the fallback.
  - A shebang file that wasn't executable upstream gaining `+x` is harmless.
- **Cost.** One extra API request per fast-path acquisition. Note it in the module doc and keep it out of the
  per-file loop.
- Keep the `GithubApi` trait seam honest: acquisition tests stay network-free, and the stub gains whatever the new step
  needs.
- **Docs.** One CHANGELOG line in the 1.3.0 "Fixed" section (scripts in skills added from GitHub keep their
  executable permission; Refresh restores it). Also update the `git_acquisition.rs` / `github_download.rs` module docs.

## Acceptance

- **Rust tests**, using the existing local-test-server pattern in `github_download.rs`'s tests:
  - a listing with an executable script, a plain file and a nested dir, plus a trees answer with modes. Assert `0o755`
    on the script, `0o644` on the plain file, and the nested dir's script executable;
  - the trees call failing (500, and 403) → the shebang fallback applies and the acquisition succeeds;
  - `truncated: true` → the fallback applies;
  - a followed upstream symlink path still maps modes by the real repo path.
- **End to end:** after a Refresh of such a skill, the central copy's script is executable, and so is a copy-mode
  target. Propagation re-copies copy-mode targets every time and `fs::copy` preserves the mode; pin this with a test
  if none exists.
- **Gate:** `cargo test --all`, then `npm run version:check && npm run check` green.
- **Operator check after release** (added to ticket 19's smoke list): on both Macs, Refresh `impeccable`, check that
  `ls -l` on its `scripts/impeccable` in the central repo shows `-rwxr-xr-x`, then run a second Refresh and check it
  still does. Never fix it by hand.

## Result

**Approach.** The mode step lives inside the HTTP adapter (`core/github_download.rs::download_github_directory_from`),
so the `GithubApi` trait signature is unchanged: `download_directory` now promises "executable bits included (best
effort)". `download_dir_recursive` records each written file as `WrittenFile { repo_path: item.path, local_path,
shebang }` (no extra I/O: the shebang flag comes from the bytes already in memory). After the whole download, **one**
`GET {api}/repos/{owner}/{repo}/git/trees/{branch}?recursive=1` goes out on the same client with the same headers,
token and `check_github_response` classification (`executable_tree_paths`). Each written file whose repo path is a
blob in the tree gets `mode | 0o111` when its mode is `100755` (`add_execute_bits`, Unix only; `cfg!(unix)` skips the
request entirely on Windows, so the cost there is zero). Fallback to the `#!` rule: the trees call fails for any reason
(HTTP status, network, parse), the answer is `truncated: true`, or (per file) a written path has no tree entry, for
example if the branch moved between the calls. Every fallback and every failed `chmod` is a `log::warn!`. Modes never
fail the acquisition. No request is made when nothing was written, and a cancellation observed after the download
bails `Cancelled` before the trees call. Module docs updated in both `github_download.rs` (rule + cost) and
`git_acquisition.rs` (adapter one + the trait method's contract).

Ref note: the trees URL uses the branch name as the Contents `?ref=` does. A slash branch such as `feature/x` may not
resolve on the trees path, and in that case the shebang fallback covers it.

**Files touched** (all `src-tauri/**`): `core/github_download.rs`, `core/git_acquisition.rs` (docs only),
`core/tests/git_stub.rs` (the stub gains an opt-in `executable_script` that lands `scripts/run.sh` at `0o755`, as the
production adapter now does), `core/tests/skill_update.rs`, `core/tests/sync_engine.rs`. No DTO change, no CHANGELOG
edit (the orchestrator adds the line).

**Tests** (red first for the four adapter tests: modes were `0o644` and no trees request was made):
- `github_download::tests::the_trees_answer_restores_executable_bits_on_every_written_file`: script `0o755`, plain
  `0o644`, nested-dir file executable only because the tree says so (no shebang), a shebang file recorded `100644`
  stays `0o644`; trees mock `expect(1)` with the bearer token, UA and Accept headers.
- `github_download::tests::a_failed_trees_call_falls_back_to_shebang_detection`: 500 and 403. Shebang files `0o755`,
  others `0o644`, acquisition `Ok`.
- `github_download::tests::a_truncated_tree_falls_back_to_shebang_detection`
- `github_download::tests::modes_map_by_the_real_path_behind_a_followed_upstream_link`: alias `plugins/all/a` links to
  `skills/a`, and the no-shebang `nested/tool` is executable only via the tree entry at the real path.
- `sync_engine::tests::copy_dir_recursive_preserves_executable_bits` (pin): `fs::copy` keeps `0o755` / `0o644`.
- `skill_update::tests::a_refresh_keeps_an_executable_script_executable_in_the_central_copy_and_a_copy_target` (pin,
  end to end): fast-path install, then two Refresh rounds (`acquire_update` + `apply_unlocked`: finalize + Propagation)
  with a copy-only Tool's `Copy` target. In both rounds the central `scripts/run.sh` and the target's copy are `0o755`.

The exact `0o755` / `0o644` literals assume umask 022 (macOS and the CI Linux runner).

**Gate** (2026-10-27; logs under `.scratch/round19/evidence/28/`):
- `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean.
- `cargo test --all`: 833 passed, 0 failed.
- `npm run version:check && npm run check`: exit 0, including vitest (48 files / 1031 tests) and build, with the
  sibling's in-progress `src/**` work present in the tree.
- `git diff --stat src/bindings/` after both runs is empty, so the bindings are byte-identical.

## Comments

- 2026-10-27 (implementation child): Implemented as above. The trait seam was kept: modes are the HTTP adapter's job,
  so acquisition policy and every stub-based test are untouched, and the stub only gained an opt-in executable script
  for the Refresh pin. The upstream issue/PR (`qufei1993/skills-hub`) has not been filed. The orchestrator owns the
  CHANGELOG line and ticket 19's smoke-list addition.
- 2026-10-27 (independent audit): **PASS after fixes**; full checklist and primary-source research in
  `../review/ticket-28-audit.md`, evidence in `../evidence/28-audit/`. Fixed the Trees ref to URL-encode slash
  branches (existing dependency; trait unchanged), removed umask-022 assumptions from default-created-file
  assertions, and guarded Unix-only test helpers. Added three regression tests for the encoded ref, malformed/missing
  tree data, and cancellation after the last file; extended HTTP fallback coverage to 404. Original commit:
  833 Rust tests passed. Corrected tree: 836 Rust tests and 1031 frontend tests passed; fmt, clippy, version check and
  full gate green; affected tests also pass under umask 077; bindings byte-identical. This supersedes Result's slash-ref
  caveat and exact-mode/umask note. Windows was checked statically, not cross-built. No open blocking defects;
  the two-Mac post-release smoke remains pending with ticket 19. Audit changes are left for the orchestrator to commit.
- 2026-10-27 (audit final shared-tree recheck): After the green ticket-28 gate, an unowned concurrent change appeared
  in `src/components/ui/held-refusal.test.tsx`. The rerun now fails its two new Switch/Checkbox wrapping-label
  activation tests at line 183 (expected one refusal, received none; 1031 other tests pass). Evidence:
  `../evidence/28-audit/gate-after-sibling.log`. Ticket 28's code verdict remains PASS after fixes, but checklist item 10
  is FAIL on the current shared checkout until the UI owner settles that work and the orchestrator reruns the gate.
  No `src/**` file was edited or reverted by this auditor.
- 2026-10-29 — **done — cd33e1e (+ 3905da3 audit fix)** (post-rebase SHAs on `origin/main`; the pre-rebase SHAs cited above exist only in the orchestrator's local history). Shipped in 1.3.0 (`v1.3.0`, published 2026-10-28; operator smoke on the main Mac passed 2026-10-29, ticket 19 § Smoke outcome). 19a closure: `round19/spec.md` § Closure — 2026-10-29.
