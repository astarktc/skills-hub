# Quick Task 260428-xo: UI Scaling Option Feasibility - Research

**Researched:** 2026-04-28
**Domain:** Tauri 2 WebView zoom / UI scaling
**Confidence:** HIGH

## Summary

Tauri 2 provides a first-class `setZoom(scaleFactor)` API on both the Rust backend (`WebviewWindow::set_zoom`) and the JavaScript frontend (`getCurrentWebview().setZoom()`). This API is supported on all three target platforms: WebKitGTK (Linux), WebView2 (Windows), and WKWebView (macOS 11+). It scales the entire WebView content proportionally -- exactly the "full-UI zoom" behavior the user wants.

The implementation is **trivial**: one permission addition, one Tauri command for persistence, a few lines of React for the UI control, and an early-apply mechanism using `getCurrentWebview().setZoom()` during app startup. No new dependencies are needed.

**Primary recommendation:** Use `getCurrentWebview().setZoom(scaleFactor)` from `@tauri-apps/api/webview` as the sole zoom mechanism. It is the simplest, most reliable, and most maintainable approach.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions

- Research all approaches (WebView zoom, CSS scaling, font-size variables) and present trade-offs
- Strong preference for full-UI zoom (everything scales proportionally)
- Discrete presets (e.g., 75%, 100%, 125%, 150%, 200%) -- not a freeform slider

### Claude's Discretion

- Specific preset values to offer
- Where in Settings UI to place the control
- How to persist the setting (existing SQLite settings table is available)

### Deferred Ideas (OUT OF SCOPE)

None stated.
</user_constraints>

## Approach Comparison

### Approach 1: Tauri WebView Zoom API (RECOMMENDED)

**What:** Call `getCurrentWebview().setZoom(scaleFactor)` from `@tauri-apps/api/webview`. [VERIFIED: node_modules/@tauri-apps/api/webview.js, docs.rs/tauri/2.9.5]

**API signature (TypeScript):**

```typescript
import { getCurrentWebview } from "@tauri-apps/api/webview";
await getCurrentWebview().setZoom(1.5); // 150% zoom
```

**API signature (Rust):**

```rust
pub fn set_zoom(&self, scale_factor: f64) -> crate::Result<()>
```

**Platform support:**
| Platform | Backend | Supported | Notes |
|----------|---------|-----------|-------|
| Linux | WebKitGTK | Yes | Uses `webkit_web_view_set_zoom_level` [VERIFIED: docs.rs/tauri/2.9.5] |
| Windows | WebView2 | Yes | Uses `ICoreWebView2Controller::put_ZoomFactor` [VERIFIED: docs.rs/tauri/2.9.5] |
| macOS | WKWebView | Yes | macOS 11+ only (Big Sur) [VERIFIED: docs.rs/tauri/2.9.5] |

**Permission required:** `core:webview:allow-set-webview-zoom` -- must be added to `src-tauri/capabilities/default.json`. This permission is NOT included in `core:default`. [VERIFIED: src-tauri/gen/schemas/desktop-schema.json]

**Complexity:** TRIVIAL (5-10 lines of meaningful code)

**Pros:**

- Scales everything: text, images, layout, spacing -- exactly like browser Ctrl+/- zoom
- Native implementation per platform -- no CSS hacks or layout quirks
- Already in the installed `@tauri-apps/api@2.10.1` package [VERIFIED: package.json]
- No new dependencies
- Zoom persists within the WebView session automatically; we just need to reapply on startup

**Cons:**

- Does not persist across app restarts (must reapply programmatically)
- macOS requires 11+ (Big Sur) -- acceptable since current macOS versions are 13-15

### Approach 2: CSS `zoom` Property

**What:** Set `document.documentElement.style.zoom = '1.5'` in JavaScript.

**Complexity:** TRIVIAL (similar line count to Approach 1)

**Pros:**

- Pure frontend, no Tauri permission or backend involvement
- Works in all WebView engines (WebView2, WebKitGTK, WKWebView)

**Cons:**

- CSS `zoom` is a non-standard property -- while widely supported, WebKitGTK support is inconsistent [ASSUMED]
- Does not behave identically to native zoom: fixed-position elements, scrollbar width, viewport calculations may differ
- Not a W3C standard (originally IE-only, adopted by Chrome/Safari but behavior varies) [ASSUMED]
- Redundant when a proper native API exists

**Verdict:** Inferior to Approach 1 in every way for this use case.

### Approach 3: CSS `transform: scale()`

**What:** Apply `transform: scale(1.5)` to the root element.

**Complexity:** MODERATE -- requires handling overflow, scroll, and layout containment

**Cons:**

- Content overflows its container -- requires wrapper element with `transform-origin` and overflow management
- Breaks scrolling behavior
- Fixed-position elements (toasts, modals) need special handling
- Scrollbar positions become incorrect
- Mouse event coordinates shift

**Verdict:** Not viable for a full-app zoom. This approach is designed for individual element transforms, not viewport scaling.

### Approach 4: CSS `font-size` Root Variable

**What:** Scale all rem-based sizes by changing `html { font-size }`.

**Complexity:** MODERATE -- requires refactoring all px-based sizes to rem

**Cons:**

- Only scales text and rem-based dimensions; images, borders, shadows, and px-based spacing stay the same
- The existing codebase uses px values and Tailwind utility classes, not rem-based custom properties
- Significant refactor needed to convert to rem-based system
- User explicitly rejected text-only scaling

**Verdict:** Not viable -- user wants full-UI zoom, not text-only scaling.

### Approach 5: Electron-style `webFrame.setZoomLevel`

**What:** Tauri does not expose an Electron-compatible `webFrame` object. [VERIFIED: no webFrame API in @tauri-apps/api]

**Verdict:** Not applicable. Tauri's equivalent is `getCurrentWebview().setZoom()` (Approach 1).

## Recommended Approach: Detailed Implementation Plan

### 1. Permission (one-line change)

**File:** `src-tauri/capabilities/default.json`

```json
{
  "permissions": [
    "core:default",
    "core:webview:allow-set-webview-zoom",
    "dialog:default",
    "dialog:allow-open",
    "updater:default"
  ]
}
```

[VERIFIED: `core:webview:allow-set-webview-zoom` exists in desktop-schema.json]

### 2. Zoom Hotkeys (optional bonus)

**File:** `src-tauri/tauri.conf.json` -- add to the window config:

```json
{
  "title": "Skills Hub",
  "width": 960,
  "height": 680,
  "resizable": true,
  "fullscreen": false,
  "zoomHotkeysEnabled": true
}
```

This enables Ctrl+/Ctrl- keyboard shortcuts and mousewheel zoom. On macOS/Linux it uses a polyfill that calls `set_webview_zoom` under the hood. Requires the same `core:webview:allow-set-webview-zoom` permission. [VERIFIED: v2.tauri.app/reference/javascript/api/namespacewebview, docs.rs/tauri/2.9.5]

**Note:** This gives users Ctrl+/- zoom even without the Settings UI -- a nice accessibility default. However, it does NOT persist the zoom level -- it resets on restart.

### 3. Persistence via SQLite Settings

Use existing `get_setting`/`set_setting` with key `"ui_zoom_level"`. [VERIFIED: src-tauri/src/core/skill_store.rs lines 236-260]

No new Tauri command is strictly needed -- use existing setting commands pattern:

**File:** `src-tauri/src/commands/mod.rs` -- add two commands:

```rust
#[tauri::command]
async fn get_ui_zoom_level(store: State<'_, SkillStore>) -> Result<f64, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let val = store.get_setting("ui_zoom_level")
            .map_err(|e| format_anyhow_error(e))?;
        Ok(val.and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.0))
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn set_ui_zoom_level(zoomLevel: f64, store: State<'_, SkillStore>) -> Result<(), String> {
    let clamped = zoomLevel.clamp(0.5, 3.0);
    tauri::async_runtime::spawn_blocking(move || {
        store.set_setting("ui_zoom_level", &clamped.to_string())
            .map_err(|e| format_anyhow_error(e))
    }).await.map_err(|e| e.to_string())?
}
```

Register in `generate_handler!` in `lib.rs`.

### 4. Apply Zoom on App Startup

In `App.tsx`, add an effect that runs once on mount:

```typescript
// Load zoom level from backend and apply
useEffect(() => {
  if (!isTauri) return;
  (async () => {
    try {
      const level = await invokeTauri<number>("get_ui_zoom_level");
      if (level !== 1.0) {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        await getCurrentWebview().setZoom(level);
      }
    } catch {
      /* ignore -- default 1.0 is fine */
    }
  })();
}, [isTauri]);
```

### 5. Settings UI Control

Add a select dropdown in `SettingsPage.tsx` (similar to existing theme selector pattern):

**Recommended presets:**

| Label | Scale Factor | Use Case                         |
| ----- | ------------ | -------------------------------- |
| 75%   | 0.75         | Large screens, want more content |
| 100%  | 1.0          | Default                          |
| 110%  | 1.1          | Slight bump                      |
| 125%  | 1.25         | Common HiDPI compensation        |
| 150%  | 1.5          | 4K screens at 100% OS scaling    |
| 175%  | 1.75         | Very high DPI                    |
| 200%  | 2.0          | Extreme case                     |

Place it right after the Theme setting (second position in Settings -- display-related settings grouped together).

### 6. Wire It Up

When user selects a new zoom level:

1. Call `getCurrentWebview().setZoom(newLevel)` -- immediate visual feedback
2. Call `invoke('set_ui_zoom_level', { zoomLevel: newLevel })` -- persist to SQLite
3. Update local state to keep the select in sync

## Common Pitfalls

### Pitfall 1: Zoom Not Applied Before First Paint

**What goes wrong:** User sees a flash of 100% content before zoom applies.
**Why it happens:** React renders before the zoom `useEffect` runs.
**How to avoid:** The zoom apply happens very fast (single IPC call), so the flash is minimal. If it becomes noticeable, consider applying zoom via `initialization_script` in the Tauri builder (runs before page load). However, this requires reading from SQLite on the Rust side during window setup, which adds startup complexity.
**Recommendation:** Start with the useEffect approach. If flash is noticeable, upgrade to initialization_script later.

### Pitfall 2: Zoom Hotkeys Conflict with Settings UI

**What goes wrong:** User changes zoom via Ctrl+/- but Settings dropdown still shows old value.
**Why it happens:** `zoomHotkeysEnabled` operates independently from the Settings UI state.
**How to avoid:** Either (a) don't enable `zoomHotkeysEnabled` and keep zoom Settings-only, or (b) enable it but add a note in the UI that keyboard shortcuts also work. Since the hotkey polyfill does not fire events we can listen to, syncing the UI is difficult.
**Recommendation:** Enable `zoomHotkeysEnabled` for accessibility (bonus feature) but make the Settings preset the "source of truth" that gets reapplied on startup. Users who use Ctrl+/- get a session-only adjustment.

### Pitfall 3: Forgotten Permission

**What goes wrong:** `setZoom` call silently fails or throws a permission error.
**Why it happens:** `core:webview:allow-set-webview-zoom` is NOT part of `core:default`.
**How to avoid:** Must explicitly add it to `src-tauri/capabilities/default.json`.

## Assumptions Log

| #   | Claim                                                     | Section    | Risk if Wrong                               |
| --- | --------------------------------------------------------- | ---------- | ------------------------------------------- |
| A1  | CSS `zoom` property has inconsistent support in WebKitGTK | Approach 2 | Low -- we're not using this approach anyway |
| A2  | CSS `zoom` is non-standard with varying behavior          | Approach 2 | Low -- same reason                          |

**All critical claims verified.** The core recommendation (Tauri `setZoom` API) is fully verified against installed code and official docs.

## Complexity Assessment

| Component                        | Complexity  | Estimated Lines                               |
| -------------------------------- | ----------- | --------------------------------------------- |
| Permission addition              | Trivial     | 1 line in JSON                                |
| Rust commands (get/set)          | Trivial     | ~20 lines                                     |
| Command registration             | Trivial     | 2 lines                                       |
| Settings UI control              | Trivial     | ~30 lines (following existing select pattern) |
| Zoom apply on startup            | Trivial     | ~10 lines                                     |
| App.tsx wiring (state + handler) | Trivial     | ~15 lines                                     |
| i18n strings                     | Trivial     | 2-4 keys                                      |
| **Total**                        | **Trivial** | **~80 lines**                                 |

**Overall complexity: TRIVIAL.** This is a straightforward feature that follows existing patterns exactly. No architectural changes, no new dependencies, no complex platform-specific code.

## Sources

### Primary (HIGH confidence)

- [docs.rs/tauri/2.9.5] - `WebviewWindow::set_zoom` Rust API signature, platform support matrix
- [v2.tauri.app/reference/javascript/api] - `setZoom()` TypeScript API, `zoomHotkeysEnabled` config option
- [node_modules/@tauri-apps/api/webview.js] - Verified `setZoom` method exists in installed v2.10.1
- [node_modules/@tauri-apps/api/webview.d.ts] - TypeScript signature: `setZoom(scaleFactor: number): Promise<void>`
- [src-tauri/gen/schemas/desktop-schema.json] - Permission string `core:webview:allow-set-webview-zoom` verified
- [src-tauri/capabilities/default.json] - Current permissions do not include zoom
- [src-tauri/src/core/skill_store.rs] - `get_setting`/`set_setting` pattern verified at lines 236-260

### Secondary (MEDIUM confidence)

- Context7 /tauri-apps/tauri-docs - `zoom_hotkeys_enabled` documentation, polyfill behavior on macOS/Linux

## Metadata

**Confidence breakdown:**

- WebView zoom API availability: HIGH - verified in installed code and multiple doc sources
- Platform support: HIGH - verified in official Rust docs with platform-specific annotations
- Implementation pattern: HIGH - follows exact existing patterns in the codebase
- CSS alternatives: MEDIUM - general web knowledge, not critical since we're not using them

**Research date:** 2026-04-28
**Valid until:** 2026-07-28 (stable API, unlikely to change)
