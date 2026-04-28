# Quick Task 260428-xo: UI Scaling Option Feasibility Research - Context

**Gathered:** 2026-04-28
**Status:** Ready for planning

<domain>
## Task Boundary

Investigate the feasibility and complexity of adding a UI scaling/zoom option to the app's Settings page. The user's problem: on a 4K screen running via Linux (WSL2 on Windows), fonts are too small due to OS-level DPI scaling differences vs macOS. Research all viable approaches in Tauri 2 and report back with recommendations.

</domain>

<decisions>
## Implementation Decisions

### Scaling Mechanism

- Research all approaches (WebView zoom, CSS scaling, font-size variables) and present trade-offs
- User is open to the best option — not locked to a specific mechanism yet

### Scope of Scaling

- Strong preference for full-UI zoom (everything scales proportionally)
- Goal: match OS-level zoom behavior — everything bigger/smaller uniformly
- Text-only scaling rejected due to potential layout weirdness

### User Control

- Discrete presets (e.g., 75%, 100%, 125%, 150%, 200%)
- Not a freeform slider — keep it simple and testable

### Claude's Discretion

- Specific preset values to offer
- Where in Settings UI to place the control
- How to persist the setting (existing SQLite settings table is available)

</decisions>

<specifics>
## Specific Ideas

- User runs the app on WSL2/Linux on a Windows machine with a 4K display
- macOS handles DPI scaling well natively; Linux does not
- This is a user-facing Settings feature, not a hidden config
- Existing settings persistence infrastructure: SQLite key-value store (`get_setting`/`set_setting`)

</specifics>

<canonical_refs>

## Canonical References

- Tauri 2 WebView configuration and window management APIs
- WebKitGTK (Linux backend for Tauri 2) zoom/scaling capabilities

</canonical_refs>
