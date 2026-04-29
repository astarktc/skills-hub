---
status: complete
---

# Quick Task 260429-5b: Set default app window resolution to 1440x1080

## Changes

- Updated `src-tauri/tauri.conf.json` window dimensions from 960x680 to 1440x1080

## Verification

- `npm run check` passes (lint, build, rust:fmt:check, rust:clippy, rust:test — all 164 tests pass)
