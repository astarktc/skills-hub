# Coding Conventions

**Analysis Date:** 2026-04-07

## High-Level Summary

### TypeScript
- Strict mode: `noUnusedLocals` and `noUnusedParameters` are enabled — unused variables/params cause compile errors
- Component files: PascalCase (`SkillCard.tsx`)
- Props types: `ComponentNameProps` (`SkillCardProps`)
- CSS class names: kebab-case (`modal-backdrop`, `skill-card`)
- Modal conditional rendering: `if (!open) return null` (full unmount, not display:none)
- Wrap presentational components with `memo()`
- All user-visible text must use i18n (`t('key')`), translation keys defined in `src/i18n/resources.ts`
- When adding new text, always provide both English and Chinese translations
- DTO types are defined in `src/components/skills/types.ts` and must stay in sync with the Rust DTOs in `commands/mod.rs`

### Rust
- Functions/methods: snake_case
- Constants: SCREAMING_SNAKE_CASE
- Tauri command parameters use camelCase (to match frontend JS calling convention)
- Use `anyhow::Context` to add context to errors
- New core modules must be exported in `core/mod.rs`
- Tests use `tempfile` crate for temp directories and `mockito` for HTTP mocking

### Styling
- Component styles go in `src/App.css` (not CSS Modules), using semantic CSS class names
- Theming via CSS variables + `[data-theme="dark"]` selector, variables defined in `src/index.css`
- Tailwind utility classes and custom CSS classes can be mixed

## Naming Patterns

**Files:**

- React component files use PascalCase filenames in `src/components/skills/` such as `src/components/skills/SkillCard.tsx`, `src/components/skills/SettingsPage.tsx`, and `src/components/skills/modals/AddSkillModal.tsx`.
- Frontend non-component support files use lowercase names when they represent infrastructure or setup, such as `src/main.tsx`, `src/i18n/index.ts`, and `src/i18n/resources.ts`.
- Rust production modules use snake_case filenames such as `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/github_search.rs`, and `src-tauri/src/commands/mod.rs`.
- Rust test files mirror the module name they cover under `src-tauri/src/core/tests/`, such as `src-tauri/src/core/tests/installer.rs` and `src-tauri/src/core/tests/skill_store.rs`.

**Functions:**

- TypeScript component functions use PascalCase when they define components, for example `SkillCard` in `src/components/skills/SkillCard.tsx` and `AddSkillModal` in `src/components/skills/modals/AddSkillModal.tsx`.
- TypeScript helper functions use camelCase, for example `formatRelative`, `getSkillSourceLabel`, and `getGithubInfo` in `src/App.tsx`, plus `formatCount` in `src/components/skills/ExplorePage.tsx`.
- Rust functions and methods use snake_case throughout, for example `format_anyhow_error` in `src-tauri/src/commands/mod.rs`, `ensure_schema` in `src-tauri/src/core/skill_store.rs`, and `hash_dir` tested from `src-tauri/src/core/tests/content_hash.rs`.

**Variables:**

- Frontend local variables and state use camelCase, including state setters from `useState` in `src/App.tsx` like `managedSkills`, `searchResults`, `showAddModal`, and `updateAvailableVersion`.
- Boolean state names prefer `is*`, `show*`, `can*`, or `has*`, as seen in `src/App.tsx`, `src/components/skills/ExplorePage.tsx`, and `src/components/skills/SettingsPage.tsx`.
- Rust locals use snake_case, such as `user_version`, `db_path`, and `newly_installed` in `src-tauri/src/core/skill_store.rs` and `src-tauri/src/commands/mod.rs`.

**Types:**

- Type aliases and prop types use PascalCase with descriptive suffixes, such as `SkillCardProps` in `src/components/skills/SkillCard.tsx`, `SettingsPageProps` in `src/components/skills/SettingsPage.tsx`, and DTO aliases in `src/components/skills/types.ts`.
- Rust structs use PascalCase and DTO suffixes for IPC types, for example `ToolInfoDto`, `ToolStatusDto`, and `InstallResultDto` in `src-tauri/src/commands/mod.rs`.
- Shared frontend DTOs are centralized in `src/components/skills/types.ts` and mirror backend command DTOs from `src-tauri/src/commands/mod.rs`; keep names and fields aligned across both files.

## Code Style

**Formatting:**

- Frontend code uses a Prettier-like style with single quotes, no semicolons, and trailing commas in multiline structures, as shown in `src/App.tsx`, `src/components/skills/SkillCard.tsx`, and `src/i18n/index.ts`.
- React code prefers destructured props with a typed props object followed by an arrow function definition, for example in `src/components/skills/modals/AddSkillModal.tsx` and `src/components/skills/SettingsPage.tsx`.
- JSX keeps one prop per line once elements become non-trivial, especially in modal and list components under `src/components/skills/`.
- CSS lives in global stylesheets, primarily `src/App.css` and `src/index.css`, and uses semantic kebab-case class names such as `.skill-card`, `.modal-backdrop`, and `.explore-card`.
- Rust is formatted with `cargo fmt`, enforced by the `rust:fmt:check` script in `package.json`.

**Linting:**

- ESLint flat config is defined in `eslint.config.js` and applies to `**/*.{ts,tsx}`.
- The frontend extends `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, and `eslint-plugin-react-refresh` via `eslint.config.js`.
- TypeScript strictness is enforced in `tsconfig.app.json` and `tsconfig.node.json` with `strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`, and `noUncheckedSideEffectImports` enabled.
- Use TypeScript’s strict null handling and explicit fallback expressions such as `?? ''`, `?? null`, and early returns, as seen in `src/App.tsx` and `src/components/skills/SkillDetailView.tsx`.

## Import Organization

**Order:**

1. External libraries and framework imports first, such as `react`, `lucide-react`, `sonner`, and `react-i18next` in `src/App.tsx` and `src/components/skills/SkillCard.tsx`
2. Local stylesheet imports near the top for entry files, such as `./App.css` in `src/App.tsx` and `./index.css` in `src/main.tsx`
3. Local component and type imports after external imports, such as imports from `./components/skills/*` and `./components/skills/types` in `src/App.tsx`

**Path Aliases:**

- Not detected. Frontend imports use relative paths such as `./components/skills/ExplorePage` in `src/App.tsx` and `../types` in `src/components/skills/modals/AddSkillModal.tsx`.
- Rust modules use `crate::core::...` absolute crate paths in backend code such as `src-tauri/src/commands/mod.rs`.

## Error Handling

**Patterns:**

- Frontend async actions generally use `try/catch` with `err instanceof Error ? err.message : String(err)` normalization, as seen in `src/App.tsx` and `src/components/skills/SettingsPage.tsx`.
- Frontend converts backend wire-format errors into user-facing translation keys in `formatErrorMessage` inside `src/App.tsx`; preserve this pattern when adding new backend error prefixes.
- Toast notifications via `sonner` are the standard user-facing error and success channel in components like `src/App.tsx`, `src/components/skills/SkillCard.tsx`, and `src/components/skills/SkillDetailView.tsx`.
- Backend command handlers return `Result<T, String>` at the Tauri boundary and map internal `anyhow::Error` values through `format_anyhow_error` in `src-tauri/src/commands/mod.rs`.
- Backend business logic adds context with `anyhow::Context`, for example in `expand_home_path` and `ensure_schema` code paths in `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/skill_store.rs`.
- Long-running synchronous backend work is wrapped in `tauri::async_runtime::spawn_blocking` in `src-tauri/src/commands/mod.rs`; use this when exposing blocking filesystem, git, or SQLite work to the frontend.

## Logging

**Framework:**

- Frontend uses `sonner` toasts for user-visible operational feedback rather than console logging, with examples in `src/App.tsx` and `src/components/skills/SkillCard.tsx`.
- Backend uses the `log` crate with `tauri-plugin-log` initialization in `src-tauri/src/lib.rs`.

**Patterns:**

- Backend setup and cleanup emit `log::info!` for best-effort maintenance events in `src-tauri/src/lib.rs`.
- Direct `console.log` usage is not detected in the frontend source under `src/`; follow the existing pattern and surface UI feedback through state or toast instead.
- Silent failures are occasionally intentional for non-critical browser or storage operations, such as `catch {}` in `src/components/skills/SkillCard.tsx`, `src/components/skills/SettingsPage.tsx`, and `src/i18n/index.ts`.

## Comments

**When to Comment:**

- Comments are sparse and used to explain non-obvious logic boundaries, not routine code. Examples include section comments in `src/components/skills/SkillDetailView.tsx` and lifecycle/setup comments in `src-tauri/src/lib.rs`.
- Use comments for behavior constraints, safety assumptions, and algorithm notes, such as the cleanup safety bullets in `src-tauri/src/lib.rs` and migration notes in `src-tauri/src/core/skill_store.rs`.
- Avoid redundant comments on self-explanatory JSX or straightforward assignments; most component files omit them.

**JSDoc/TSDoc:**

- Not generally used in TypeScript component files under `src/components/skills/`.
- Rust doc comments are minimal but present for selected internals, for example `CancelToken` in `src-tauri/src/core/cancel_token.rs`.

## Function Design

**Size:**

- Presentational components are kept moderate and focused, often one component per file, such as `src/components/skills/SkillCard.tsx`, `src/components/skills/ExplorePage.tsx`, and modal files under `src/components/skills/modals/`.
- `src/App.tsx` is the notable orchestration exception: it centralizes application state, view switching, and command orchestration. New global stateful workflows currently belong there unless the surrounding architecture is deliberately changed.
- Rust command and core modules favor many small functions over monolithic logic, with helper functions and DTO mappers split inside `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/*.rs`.

**Parameters:**

- React components receive explicit prop objects with typed callbacks and data dependencies, as seen in `src/components/skills/modals/AddSkillModal.tsx` and `src/components/skills/SettingsPage.tsx`.
- Event handlers usually pass primitive values upward rather than DOM events, for example `onLocalPathChange(event.target.value)` in `src/components/skills/modals/AddSkillModal.tsx`.
- Backend command parameters use camelCase for frontend compatibility, while internal Rust functions stay snake_case in `src-tauri/src/commands/mod.rs`.

**Return Values:**

- Frontend components use early null returns for conditional mounting, for example `if (!open) return null` in `src/components/skills/modals/DeleteModal.tsx`, `src/components/skills/modals/AddSkillModal.tsx`, and `src/components/skills/LoadingOverlay.tsx`.
- Frontend helpers often return normalized primitives or nullable objects, such as `getGithubInfo` in `src/App.tsx` and `isInstalled` in `src/components/skills/ExplorePage.tsx`.
- Backend functions return `Result<T>` internally and serialize DTO structs at the command boundary in `src-tauri/src/commands/mod.rs`.

## Module Design

**Exports:**

- Frontend component files usually define a single component and default-export `memo(Component)` for presentational modules, as shown in `src/components/skills/SkillCard.tsx`, `src/components/skills/ExplorePage.tsx`, `src/components/skills/SettingsPage.tsx`, and modal components under `src/components/skills/modals/`.
- Frontend shared DTOs use named `export type` declarations from `src/components/skills/types.ts`.
- Backend command aggregation uses a single module file `src-tauri/src/commands/mod.rs` with public command functions and DTO structs.
- Backend core modules expose focused APIs through per-file modules under `src-tauri/src/core/` and are re-exported via `src-tauri/src/core/mod.rs`.

**Barrel Files:**

- Not used on the frontend; components are imported directly from their file paths in `src/App.tsx`.
- Rust module indexing is handled through `mod.rs` files such as `src-tauri/src/core/mod.rs` and `src-tauri/src/commands/mod.rs`.

## Prescriptive Patterns to Follow

- Put new user-visible strings in `src/i18n/resources.ts` and consume them through `t('key')`; do not inline English text in JSX.
- Keep new frontend DTO fields synchronized between `src/components/skills/types.ts` and `src-tauri/src/commands/mod.rs`.
- Wrap new presentational React components in `memo()` when they primarily render props, matching `src/components/skills/SkillCard.tsx` and `src/components/skills/ExplorePage.tsx`.
- Use early-return modal rendering (`if (!open) return null`) for overlay components, matching files in `src/components/skills/modals/`.
- Put global app-level orchestration, shared fetch/reload flows, and Tauri command wiring in `src/App.tsx` unless the architecture is intentionally reworked.
- For backend commands, keep business logic in `src-tauri/src/core/` and make `src-tauri/src/commands/mod.rs` responsible for Tauri command wrappers, DTOs, and error formatting.

---

_Convention analysis: 2026-04-07_
