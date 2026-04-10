---
id: 260409-r9e
type: quick
title: Fix updater.json version to always use 3-part semver
status: complete
completed: "2026-04-10T00:41:00Z"
duration: 83s
tasks_completed: 2
tasks_total: 2
files_modified:
  - .github/workflows/release.yml
  - scripts/version.mjs
commits:
  - hash: b4ab31e
    message: "fix(260409-r9e): normalize updater.json version to 3-part semver"
  - hash: 0da47ec
    message: "fix(260409-r9e): validate 3-part semver in version.mjs set command"
---

# Quick Task 260409-r9e: Fix updater.json version to always use 3-part semver

Hardened both the release workflow and version script to guarantee updater.json always contains strict 3-part semver (x.y.z), preventing Tauri updater failures from shortened tags like v1.0.

## Changes

### Task 1: Normalize VERSION in release workflow (b4ab31e)

Added a normalization block in `.github/workflows/release.yml` immediately after `VERSION="${TAG#v}"`. The block splits the version on `.`, pads missing parts with `0`, and reassembles as `x.y.z`. This means tags like `v1.0` produce `"1.0.0"` and `v1` produces `"1.0.0"` in updater.json.

### Task 2: Validate 3-part semver in version.mjs set command (0da47ec)

Added a regex guard (`/^\d+\.\d+\.\d+$/`) in `scripts/version.mjs` before the `set` command writes to package.json. Running `node scripts/version.mjs set 1.0` now exits with a clear error message, preventing 2-part versions from propagating to package.json, tauri.conf.json, and Cargo.toml.

## Verification

- `grep -A8 'VERSION="${TAG#v}"' .github/workflows/release.yml` shows the normalization block -- PASS
- `node scripts/version.mjs set 1.0` exits with "Error: Version must be 3-part semver (x.y.z)" -- PASS
- `node scripts/version.mjs set 1.0.0` succeeds with "Version set to 1.0.0" -- PASS
- `npm run check` passes (lint, build, rust:fmt:check, rust:clippy, rust:test -- 146 tests) -- PASS

## Deviations from Plan

None -- plan executed exactly as written.

## Self-Check: PASSED

- FOUND: .github/workflows/release.yml
- FOUND: scripts/version.mjs
- FOUND: commit b4ab31e
- FOUND: commit 0da47ec
