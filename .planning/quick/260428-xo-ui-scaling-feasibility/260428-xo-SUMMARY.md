---
status: complete
quick_id: 260428-xo
date: "2026-04-28"
---

# Quick Task 260428-xo: UI Scaling Option — Summary

## What Was Done

Added a "Default Zoom Level" dropdown to the Settings page that uses Tauri 2's native `setZoom()` API for full-UI proportional scaling. The feature solves the 4K/HiDPI readability problem on Linux/WSL2 where OS-level DPI scaling doesn't work as well as macOS.

### Changes

**Backend (Rust):**

- Added `core:webview:allow-set-webview-zoom` permission to `capabilities/default.json`
- Added `zoomHotkeysEnabled: true` to window config in `tauri.conf.json` (enables Ctrl+/- for session-only zoom)
- Added `get_ui_zoom_level` and `set_ui_zoom_level` Tauri commands in `commands/mod.rs`
- Registered both commands in `generate_handler!` in `lib.rs`
- Added Rust-side startup zoom apply in `lib.rs` setup hook — reads zoom from SQLite and calls `set_zoom` before webview loads content (no flash of default zoom)

**Frontend (React/TypeScript):**

- Added `zoomLevel` state and `handleZoomLevelChange` handler in `App.tsx`
- Added `useEffect` to load initial zoom level from backend on mount
- Added zoom preset dropdown to `SettingsPage.tsx` (placed after Theme setting)
- Presets: 75%, 100% (Default), 110%, 125%, 150%, 175%, 200%
- Added i18n strings for both English and Chinese in `resources.ts`

### Files Modified

| File                                     | Change                                       |
| ---------------------------------------- | -------------------------------------------- |
| `src-tauri/capabilities/default.json`    | Added zoom permission                        |
| `src-tauri/tauri.conf.json`              | Added `zoomHotkeysEnabled: true`             |
| `src-tauri/src/commands/mod.rs`          | Added get/set zoom commands                  |
| `src-tauri/src/lib.rs`                   | Registered commands + startup zoom apply     |
| `src/App.tsx`                            | Added zoom state, handler, effect, and props |
| `src/components/skills/SettingsPage.tsx` | Added zoom dropdown UI                       |
| `src/i18n/resources.ts`                  | Added zoom-related i18n keys                 |

## Commits

| Hash      | Description                                               |
| --------- | --------------------------------------------------------- |
| `965de7d` | Backend: IPC commands, capability, hotkeys, startup apply |
| `38fe19c` | Frontend: Default Zoom Level dropdown in Settings         |

## Verification

- `npm run check` passes (lint + build + rust:fmt:check + rust:clippy + rust:test)
- 180 tests pass (164 unit + 16 integration)
- Manual verification pending (human-verify checkpoint)
