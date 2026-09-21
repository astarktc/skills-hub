# 03 Bump GitHub Actions to current majors

Status: ready-for-agent
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
