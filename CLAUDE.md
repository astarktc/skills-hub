# Skills Hub - Project Rules

## Overview

Skills Hub is a cross-platform desktop app (Tauri 2 + React 19) for managing AI Agent Skills and syncing them to 47+ AI coding tools. Core concept: "Install once, sync everywhere."

## Tech Stack - read .planning/codebase/STACK.md

## Common Commands

```bash
npm run dev              # Vite dev server (port 5173)
npm run tauri:dev        # Tauri dev window (frontend + backend)
npm run build            # tsc + vite build
npm run check            # Full check: lint + build + rust:fmt:check + rust:clippy + rust:test
npm run lint             # ESLint (flat config v9)
npm run rust:test        # cargo test
npm run rust:clippy      # Rust lint
npm run rust:fmt         # Rust format
npm run rust:fmt:check   # Rust format check
```

Always run `npm run check` before committing to ensure all checks pass.

## Codebase and Directory Structure - read .planning/codebase/STRUCTURE.md

## Architecture - read .planning/codebase/ARCHITECTURE.md

### Frontend ↔ Backend Communication
- Uses Tauri IPC (`invoke`) to call backend commands
- Frontend call pattern: `const result = await invoke('command_name', { param })`
- Backend commands are defined in `commands/mod.rs` and registered in `lib.rs` via `generate_handler!`
- New commands must be registered in both places

### Frontend State Management
- **No state management library** — all state is centralized in `App.tsx` via `useState`
- Passed to child components via props drilling (modals receive many props)
- Data refresh pattern: call `invoke('get_managed_skills')` after operations to re-fetch the list

### Backend Layering
- `commands/` layer: Tauri command definitions, DTO conversions, error formatting (no business logic)
- `core/` layer: Pure business logic, independently testable
- Async commands use `tauri::async_runtime::spawn_blocking` to wrap synchronous operations
- Shared state injected via `app.manage(store)` + `State<'_, SkillStore>`

### Error Handling
- Backend uses `anyhow::Result<T>`, converted to string via `format_anyhow_error()` for the frontend
- Special error prefixes for frontend identification: `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`
- Frontend catches with try-catch and displays errors via sonner toast

## External Integrations - read .planning/codebase/INTEGRATIONS.md

## Coding Conventions - read .planning/codebase/CONVENTIONS.md

## Development Workflow

1. **Before implementing**: Briefly describe the approach and list the files to be modified. Wait for confirmation before writing code.
2. **Implement completely**: For features involving both frontend and backend, modify both sides in one pass — including Tauri command registration, DTO types, i18n translations (both EN and ZH), and UI.
3. **Verify after changes**: Always run `npm run check` after implementation to ensure lint, build, and all Rust checks pass. Fix any errors before presenting the result.
4. **Keep changes minimal**: Only modify what is necessary for the requirement. Do not refactor, add comments, or "improve" unrelated code.

## Testing - read .planning/codebase/TESTING.md

## Important Notes

- Path handling must support `~` expansion (backend has `expand_home_path()`)
- Sync strategy uses triple fallback: symlink → junction (Windows) → copy
- Git uses vendored-openssl, HTTP uses rustls-tls — avoids system SSL issues
- Version numbers must stay in sync between `package.json` and `src-tauri/tauri.conf.json` (validate with `npm run version:check`)
- Rust crate is named `app_lib` (not the default package name) — use `app_lib::...` for imports
- Database has a schema migration mechanism (`migrate_legacy_db_if_needed`) — consider migrations when modifying table structures
- Tool adapter list is in `tool_adapters/mod.rs` — adding a new AI tool requires both a `ToolId` enum variant and an adapter instance
