# 03 Bump GitHub Actions to current majors

Status: claimed
Lane: B (after 02)
Source: BACKLOG #32 (from `archive/round13/issues/04:comments`)

Round 13 #31 pinned every action to a SHA within its existing major (checkout v4.4.0, setup-node v4.4.0,
upload-artifact v4.6.2, download-artifact v4.3.0, softprops/action-gh-release v2.6.2, Swatinem/rust-cache v2.9.2,
dtolnay/rust-toolchain master) across `.github/workflows/{ci,release,update-featured-skills,auto-tag}.yml`. The
backlog note names targets (checkout v7, setup-node v7, upload-artifact v7, download-artifact v8, gh-release v3) —
**verify each against the action's GitHub releases page before pinning**; take the newest stable major that exists,
not the number in the note.

For each bump: new full-length commit SHA + `# vX.Y.Z` comment (same style as today), and read that major's release
notes for breaking changes that touch our usage — known ones: `upload-artifact`/`download-artifact` v4→v5+ changed
`name`/`path`/`merge-multiple` semantics and artifact retention; `setup-node` newer majors change default caching
and Node resolution (`release.yml` still builds on Node 20, `ci.yml` on 22 — keep both); `action-gh-release` v3
changed `files` globbing and draft/prerelease defaults — `release.yml:570` must still publish `updater.json` and all
signed `.tar.gz.sig` assets with the job-level `contents: write` (not top-level). Do not lower any `permissions:`.
`dtolnay/rust-toolchain` and `Swatinem/rust-cache`: bump only if a newer major exists; otherwise leave.

Keep `persist-credentials: false` on every checkout. Keep the least-privilege shape from #31.

You cannot run the workflows; list in the ticket comment exactly which the parent's push exercises (ci, then
auto-tag → release on the version bump) and what to watch in each. If a bump needs an input rename, say which line.

## Done when

Every action in the four workflows is on its newest stable major, SHA-pinned with a version comment; breaking-change
review recorded per action in the comment; `ci.yml` and `release.yml` remain valid YAML (`node -e` parse or
`actionlint` if available).

## Comments

### 2026-09-20 — lane B (implementation)

Verified every target against the action's GitHub releases (`gh release list` / `gh release view`) — the backlog
numbers hold for all five. Tag → commit resolution via `gh api repos/<r>/git/ref/tags/<tag>`; only
`softprops/action-gh-release` uses annotated tags (tag object `e598afbe…` dereferenced to the commit below); the
`actions/*` tags are lightweight. Each SHA cross-checked with `gh api repos/<r>/commits/<sha>` (subject line noted).

| Action | Old | New | Commit SHA | Confirmed commit subject |
|---|---|---|---|---|
| actions/checkout | v4.4.0 `11d5960a…` | **v7.0.1** | `3d3c42e5aac5ba805825da76410c181273ba90b1` | "prep v7.0.1 release (#2531)" |
| actions/setup-node | v4.4.0 `49933ea5…` | **v7.0.0** | `820762786026740c76f36085b0efc47a31fe5020` | "Migrate to ESM and upgrade dependencies (#1574)" |
| actions/upload-artifact | v4.6.2 `ea165f8d…` | **v7.0.1** | `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` | "Merge pull request #797 …update-dependency" |
| actions/download-artifact | v4.3.0 `d3f86a10…` | **v8.0.1** | `3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c` | "Add regression tests for CJK characters (#471)" |
| softprops/action-gh-release | v2.6.2 `3bb12739…` | **v3.0.3** | `efb35369e0ad2afab669f228072c1b0d510eae64` | "release 3.0.3 (#840)" |
| Swatinem/rust-cache | v2.9.2 | unchanged | `6323deb1…` (= v2.9.2 tag, still Latest; no v3 exists) | — |
| dtolnay/rust-toolchain | master 2026-09-12 | unchanged | `02cb101e…` (= current master head; only major is v1) | — |

Files: `ci.yml` (checkout ×2, setup-node), `release.yml` (checkout ×2, setup-node, upload-artifact,
download-artifact, action-gh-release), `auto-tag.yml` (checkout), `update-featured-skills.yml` (checkout,
setup-node). `persist-credentials: false` kept on every read-only checkout; `true` kept on the two pushing jobs.
No `permissions:` changed; `release.yml` still has top-level `contents: read` and job-level `contents: write` on
`assemble-updater-json` only. No input renames needed — every `with:` key we use still exists under the new majors.

Breaking-change review per major crossed (from the release notes):

- **checkout v5**: node24 runtime, min runner 2.327.1 — GitHub-hosted `ubuntu-latest`/`ubuntu-22.04`/`macos-14`/
  `windows-2022` all qualify. **v6**: `persist-credentials` now stores the token in a separate file under
  `$RUNNER_TEMP` wired in via git-config include — README: "No workflow changes required — `git fetch`, `git push`
  continue to work"; the Docker-container-action caveat (runner ≥ 2.329.0) does not apply, our pushes are `run:`
  steps. I updated the two `persist-credentials: true` comments so they no longer claim the token is in `.git/config`.
  **v7**: refuses to check out a fork PR under `pull_request_target`/`workflow_run` — we use neither trigger; ESM
  migration is internal. **v7.0.1**: fixes only.
- **setup-node v5**: automatic caching when `package.json` has a `packageManager` field (ours has none) and node24
  runtime. **v6**: auto-caching limited to npm. **v7**: ESM migration, new `cache-primary-key`/`cache-matched-key`
  outputs, drops a dummy `NODE_AUTH_TOKEN` export, `@actions/cache` 5.1.0. Our explicit `node-version` (22 in
  ci.yml, "20" in release.yml, 20 in update-featured-skills.yml) and `cache: npm` inputs are unchanged in
  semantics. Node 20 vs 22 split is preserved.
- **upload-artifact v5**: node24 support (opt-in). **v6**: node24 default, min runner 2.327.1. **v7**: new
  `archive: false` direct-upload mode for single files (we don't set it; our glob is multi-file and stays zipped);
  ESM migration. `name`/`path`/`if-no-files-found` unchanged. **v7.0.1**: docs + dependency bump.
- **download-artifact v5**: BREAKING only for single downloads by `artifact-ids` (output path no longer nested) — we
  download by `pattern` + `merge-multiple: true`, explicitly listed as "no action needed". **v6/v7**: node24.
  **v8**: ESM; non-zip downloads are no longer unzipped (our uploads are zipped, so unaffected; `skip-decompress`
  left default); **`digest-mismatch` now defaults to `error`** — a hash mismatch on download fails the release run
  instead of warning. That is the secure default and desired; noted as the one behavioural change that can surface.
  **v8.0.1**: CJK artifact names fix.
- **action-gh-release v3.0.0**: the only breaking change is the Node 20 → 24 runtime (`action.yml` at v3.0.3:
  `using: "node24"`). The ticket's hypotheses — `files` globbing changes and draft/prerelease default changes — are
  **not** in the v3.0.0–v3.0.3 notes; `files`, `tag_name`, `name`, `prerelease`, `body_path` keep their v2
  semantics, so `release.yml:570` still publishes `dl/*` (all signed `.tar.gz.sig`/installers) plus `updater.json`
  under the job-level `contents: write`. v3.0.2 additionally reuses an existing draft when publishing prereleases
  and hardens streamed asset uploads (small checksum assets); v3.0.3 classifies malformed API errors safely.

Validation: all four files parse (`uv run --with pyyaml python3 -c "yaml.safe_load(...)"`, jobs enumerated:
`tag`, `web,rust`, `release,assemble-updater-json`, `update`). `actionlint` is not installed here and Go is not
available to run it ad hoc — parse + manual review only. `grep` confirms none of the newly-sensitive inputs/triggers
(`pull_request_target`, `workflow_run`, `artifact-ids`, `archive:`, `skip-decompress`, `digest-mismatch`) appear
in any workflow.

What the parent's push exercises, and what to watch:

1. **ci.yml** (push to main): both jobs run on the new checkout/setup-node. Watch: setup-node v7 restores the npm
   cache (`cache-matched-key` in the log) and `npm ci` still runs; rust job's new `cargo test --release --all
   propagation` step passes (16 tests) and the bindings-drift step after it stays green; rust-cache post-step size.
2. **auto-tag.yml** (fires on the `package.json` version bump): first run of checkout v7 with
   `persist-credentials: true` — watch that `git push origin vX.Y.Z` authenticates (v6+ credential-file path) and
   that `gh workflow run release.yml --ref vX.Y.Z` dispatches.
3. **release.yml** (dispatched by auto-tag): five build jobs upload via upload-artifact v7 (zipped, named
   `release-assets-<target>`); `assemble-updater-json` downloads via download-artifact v8 with `pattern` +
   `merge-multiple` — watch the "Download workflow artifacts" step for a digest-mismatch error (new hard failure
   mode) and that `dl/` is flat as before; then action-gh-release v3.0.3 runs on node24 — watch that the release
   carries every platform asset, each `.sig`, and `updater.json`, and that `prerelease` is false for a plain
   `vX.Y.Z` tag.
4. **update-featured-skills.yml** is not exercised by the push (nightly cron / manual); its first run tonight is the
   second `persist-credentials: true` + checkout v7 `git push` path to glance at.
