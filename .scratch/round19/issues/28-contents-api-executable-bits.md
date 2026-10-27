# 28 — Executable bits survive the GitHub Contents fast path

Status: ready-for-agent
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

## Comments
