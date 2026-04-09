# Quick Task 260409-c0k: Summary

**Task:** Rebrand app identifier to com.skillshub.app, update qufei1993 URLs to astarktc, add Linux x86_64 to release workflow
**Date:** 2026-04-09
**Status:** Complete

## Commits

| Commit    | Description                                                      |
| --------- | ---------------------------------------------------------------- |
| `42ecfd8` | Rebrand identifier to com.skillshub.app and update upstream URLs |
| `9614454` | Add Linux x86_64 to release workflow                             |

## Changes

### Task 1: Rebrand identifier and update upstream URLs

- **src-tauri/tauri.conf.json**: Identifier changed to `com.skillshub.app`, updater endpoint updated to `astarktc/skills-hub`
- **src-tauri/src/core/skill_store.rs**: Added `com.qufei1993.skillshub` to `LEGACY_APP_IDENTIFIERS` for automatic DB migration
- **src/App.tsx**: Release notes API URL updated to `astarktc/skills-hub`
- **src-tauri/src/core/featured_skills.rs**: Featured skills catalog URL updated to `astarktc/skills-hub`
- **src-tauri/Cargo.toml**: Repository metadata updated to `astarktc/skills-hub`

### Task 2: Add Linux x86_64 to release workflow

- **`.github/workflows/release.yml`**: Added `ubuntu-22.04` / `x86_64-unknown-linux-gnu` matrix entry
- Linux deps install step (GTK, WebKit, libappindicator, etc.)
- Linux build step producing `deb` + `appimage` bundles
- Linux asset preparation step (deb, AppImage, updater tar.gz + sig)
- Updater.json generation now includes `linux-x86_64` platform
- Upload artifact names use `matrix.target` to avoid macOS/Linux x86_64 collision
