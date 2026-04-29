# Coding Conventions

**Analysis Date:** 2026-04-29

## Project Skills

**Detected skill indexes:**

- `.claude/skills/code-simplification/SKILL.md`: prefer the simplest code that works; extract duplication only at the third occurrence; avoid one-caller abstractions, redundant defensive checks, and dead code.
- `.claude/skills/vercel-react-best-practices/SKILL.md`: React changes should avoid unnecessary re-renders, prefer direct imports over barrels, use primitive effect dependencies, avoid inline components, and parallelize independent async work.
- `.claude/skills/vercel-composition-patterns/SKILL.md`: prefer composition and explicit variant components over boolean prop proliferation for reusable React APIs.
- `.claude/skills/frontend-design/SKILL.md`: UI additions should be production-grade, visually intentional, accessible, and cohesive with existing theme variables.
- `.claude/skills/web-design-guidelines/SKILL.md`: use for UI/accessibility review tasks; fetch current guidelines before formal review.

## Naming Patterns

**Files:**

- React component files use PascalCase filenames: `src/components/skills/SkillCard.tsx`, `src/components/skills/SettingsPage.tsx`, `src/components/projects/ProjectsPage.tsx`, `src/components/projects/AssignmentMatrix.tsx`.
- React modal files use PascalCase plus a `Modal` suffix: `src/components/skills/modals/AddSkillModal.tsx`, `src/components/projects/RemoveProjectModal.tsx`, `src/components/projects/ToolConfigModal.tsx`.
- React hook files use a `use*` camelCase filename: `src/components/projects/useProjectState.ts`.
- Frontend setup and infrastructure files use lowercase names: `src/main.tsx`, `src/i18n/index.ts`, `src/i18n/resources.ts`.
- Shared frontend type files are named `types.ts`: `src/components/skills/types.ts`, `src/components/projects/types.ts`.
- Rust production modules use snake_case filenames: `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/project_sync.rs`, `src-tauri/src/core/github_search.rs`.
- Rust test files mirror the module under test in `src-tauri/src/core/tests/`: `src-tauri/src/core/tests/project_ops.rs`, `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/sync_engine.rs`.

**Functions:**

- React component functions use PascalCase: `SkillCard` in `src/components/skills/SkillCard.tsx`, `ProjectsPage` in `src/components/projects/ProjectsPage.tsx`, `AddSkillModal` in `src/components/skills/modals/AddSkillModal.tsx`.
- React hooks use `use*`: `useProjectState` in `src/components/projects/useProjectState.ts`.
- TypeScript handlers and helpers use camelCase: `handleAddProject`, `handleToolConfigConfirm`, `formatProjectError`, and `normalizeError` in `src/components/projects/`.
- Rust functions and methods use snake_case: `format_anyhow_error` and `expand_home_path` in `src-tauri/src/commands/mod.rs`, `ensure_schema` in `src-tauri/src/core/skill_store.rs`, `assign_and_sync` in `src-tauri/src/core/project_sync.rs`.
- Tauri command function names use snake_case at the command boundary, but parameters that are consumed by JavaScript may intentionally use camelCase with `#[allow(non_snake_case)]`, as in `install_local(sourcePath, ...)` in `src-tauri/src/commands/mod.rs`.

**Variables:**

- Frontend state variables use camelCase with React setter names: `managedSkills`, `setManagedSkills`, `selectedProjectId`, `setSelectedProjectId` in `src/App.tsx` and `src/components/projects/useProjectState.ts`.
- Boolean frontend state uses `is*`, `show*`, `can*`, or `has*` names: `isTauri` in `src/App.tsx`, `showAddModal` in `src/components/projects/useProjectState.ts`, `canClose` in `src/components/skills/modals/AddSkillModal.tsx`.
- TypeScript event handlers use `handle*` for local callbacks and `on*` for props passed into components, as seen in `src/components/projects/ProjectsPage.tsx` and `src/components/skills/modals/AddSkillModal.tsx`.
- Rust locals use snake_case: `user_version`, `db_path`, `newly_installed`, `project_id`, `skill_cache` in `src-tauri/src/**/*.rs`.
- Rust constants use SCREAMING_SNAKE_CASE: `DB_FILE_NAME`, `LEGACY_APP_IDENTIFIERS`, and `SCHEMA_VERSION` in `src-tauri/src/core/skill_store.rs`.

**Types:**

- TypeScript prop types use `ComponentNameProps`: `SkillCardProps` in `src/components/skills/SkillCard.tsx`, `AddSkillModalProps` in `src/components/skills/modals/AddSkillModal.tsx`.
- TypeScript DTO types use a `Dto` suffix and preserve backend field names, including snake_case wire fields: `ProjectDto`, `ProjectToolDto`, `ProjectSkillAssignmentDto` in `src/components/projects/types.ts`.
- Rust DTO structs use PascalCase plus `Dto`: `ToolInfoDto`, `ToolStatusDto`, `InstallResultDto` in `src-tauri/src/commands/mod.rs`.
- Rust database records use PascalCase plus `Record`: `SkillRecord`, `SkillTargetRecord`, `ProjectRecord`, `ProjectSkillAssignmentRecord` in `src-tauri/src/core/skill_store.rs`.

## Code Style

**Formatting:**

- Frontend uses TypeScript with strict compilation rather than a detected Prettier config. There is no `.prettierrc` or `biome.json`; preserve the style of the surrounding file.
- Existing frontend files currently mix quote/semicolon styles: `src/App.tsx` and `src/components/projects/useProjectState.ts` use double quotes with semicolons, while `src/components/skills/modals/AddSkillModal.tsx` uses single quotes without semicolons. Match the local file being edited.
- JSX should keep one prop per line for non-trivial elements, matching `src/components/skills/SkillCard.tsx` and `src/components/projects/ProjectsPage.tsx`.
- Use early returns for conditional modal mounting: `if (!open) return null` in `src/components/skills/modals/AddSkillModal.tsx`.
- CSS lives in global stylesheets, primarily `src/App.css` and `src/index.css`; use semantic kebab-case class names such as `skill-card`, `modal-backdrop`, `projects-page`, and `matrix-panel`.
- Rust formatting is `cargo fmt`, enforced by `npm run rust:fmt:check` from `package.json`.

**Linting:**

- ESLint flat config is in `eslint.config.js` and applies to `**/*.{ts,tsx}`.
- ESLint extends `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, and `eslint-plugin-react-refresh` in `eslint.config.js`.
- TypeScript strictness is configured in `tsconfig.app.json`: `strict`, `noUnusedLocals`, `noUnusedParameters`, `erasableSyntaxOnly`, `noFallthroughCasesInSwitch`, and `noUncheckedSideEffectImports` are enabled.
- Rust linting uses `cargo clippy --all-targets --all-features -- -D warnings` via `npm run rust:clippy` in `package.json`.
- Full verification command is `npm run check`, which runs lint, build, Rust format check, Clippy, and Rust tests.

## Import Organization

**Order:**

1. External value imports first: React hooks, Tauri APIs, i18n, icons, toast libraries; examples are `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, and `src/components/skills/SkillCard.tsx`.
2. Local component imports next: `src/App.tsx` imports `./components/skills/*` and `./components/projects/ProjectsPage` after external imports.
3. Type-only imports should use `import type` and usually follow value imports: `src/components/projects/useProjectState.ts`, `src/components/projects/ProjectsPage.tsx`, and `src/components/skills/SkillCard.tsx`.
4. Rust imports group `std` imports first, external crates next, then `crate::...` imports, as shown in `src-tauri/src/core/project_sync.rs` and `src-tauri/src/commands/mod.rs`.

**Path Aliases:**

- No frontend path aliases are configured in `tsconfig.app.json` or `vite.config.ts`; use relative paths such as `./components/skills/ExplorePage` and `../skills/types`.
- Rust backend code uses crate-relative paths such as `crate::core::skill_store::SkillStore` in `src-tauri/src/commands/mod.rs` and `src-tauri/src/core/project_sync.rs`.
- Avoid barrel files for new frontend code; project skill guidance in `.claude/skills/vercel-react-best-practices/SKILL.md` prefers direct imports for bundle analyzability.

## Error Handling

**Patterns:**

- Frontend async actions use `try/catch` and normalize unknown errors with `err instanceof Error ? err.message : String(err)`, as in `src/components/projects/ProjectsPage.tsx` and `src/components/projects/useProjectState.ts`.
- Frontend user-facing errors and successes should use `sonner` toasts (`toast.error`, `toast.success`, `toast.warning`) as in `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, and `src/components/skills/SkillCard.tsx`.
- Frontend special backend error contracts are parsed by prefix. Use `formatErrorMessage` in `src/App.tsx` for global skill flows and `formatProjectError` in `src/components/projects/useProjectState.ts` for project flows.
- Backend command functions return `Result<T, String>` and map `anyhow::Error` through `format_anyhow_error` in `src-tauri/src/commands/mod.rs`.
- Backend core code returns `anyhow::Result<T>` and should add context with `anyhow::Context` around filesystem, database, git, and network failures, as in `src-tauri/src/core/skill_store.rs` and `src-tauri/src/commands/mod.rs`.
- Long-running or blocking Tauri commands should wrap synchronous core work in `tauri::async_runtime::spawn_blocking`, matching `get_tool_status`, `get_onboarding_plan`, and `set_central_repo_path` in `src-tauri/src/commands/mod.rs`.
- Preserve frontend-relevant error prefixes in `format_anyhow_error`: `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, `TOOL_NOT_WRITABLE|`, `SKILL_INVALID|`, `DUPLICATE_PROJECT|`, `ASSIGNMENT_EXISTS|`, and `NOT_FOUND|` in `src-tauri/src/commands/mod.rs`.
- Some backend workflows intentionally record failure status rather than returning `Err`, for example `assign_and_sync` in `src-tauri/src/core/project_sync.rs` returns an assignment with `status: "error"` when sync fails.

## Logging

**Framework:** backend `log` crate with `tauri-plugin-log`; frontend `sonner` toasts for user-visible messages.

**Patterns:**

- Backend logs non-fatal operational issues with `log::warn!`, for example hash computation failures and project resync failures in `src-tauri/src/core/project_sync.rs`, and rename fallback information in `src-tauri/src/commands/mod.rs`.
- Use toast notifications instead of `console.log` for visible frontend feedback, matching `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, and `src/components/skills/SkillCard.tsx`.
- Silent `catch {}` blocks are used only for non-critical fallback paths, such as clipboard/storage failure handling in `src/components/skills/SkillCard.tsx` and best-effort reloads in `src/components/projects/useProjectState.ts`.

## Comments

**When to Comment:**

- Use comments for behavior constraints, migration notes, safety assumptions, stale-result protection, and non-obvious sequencing. Examples include schema migration comments in `src-tauri/src/core/skill_store.rs`, stale selection comments in `src/components/projects/useProjectState.ts`, and gitignore sequencing comments in `src/components/projects/ProjectsPage.tsx`.
- Avoid comments that restate straightforward JSX, assignments, imports, or type declarations.
- Remove commented-out code; Git history is the source for removed implementation details.

**JSDoc/TSDoc:**

- Not generally used in frontend component files under `src/components/`.
- Rust doc comments are sparse and reserved for selected public/internal abstractions; prefer clear names and focused tests unless API documentation is needed.

## Function Design

**Size:**

- Prefer focused components and helpers. Presentational components should stay close to the size of `src/components/skills/SkillCard.tsx`, `src/components/projects/ProjectList.tsx`, or modal files under `src/components/**/`.
- `src/App.tsx` is an orchestration exception with many states and handlers; new feature-specific state should stay in the relevant feature subtree when possible, as project state does in `src/components/projects/useProjectState.ts`.
- Follow the code-simplification skill thresholds from `.claude/skills/code-simplification/SKILL.md`: review functions above 20-40 lines, refactor functions above 40 lines when a clean split exists, and keep nesting at two levels where practical.

**Parameters:**

- React presentational components receive explicit prop objects and callbacks instead of reading global state directly, as in `src/components/skills/SkillCard.tsx` and `src/components/skills/modals/AddSkillModal.tsx`.
- Event handlers should convert DOM events into primitive values before passing them upward, as in `onLocalPathChange(event.target.value)` in `src/components/skills/modals/AddSkillModal.tsx`.
- Backend command parameters exposed to JavaScript may use camelCase to match `invoke` calls; internal Rust helpers should remain snake_case, as seen in `src-tauri/src/commands/mod.rs`.
- Avoid boolean prop proliferation for new reusable components; project skill guidance in `.claude/skills/vercel-composition-patterns/SKILL.md` prefers explicit variants or composition when behavior diverges.

**Return Values:**

- Frontend async state actions return `Promise<T>` and throw normalized `Error` values for caller-level toasts, as in `src/components/projects/useProjectState.ts`.
- Backend core functions return `anyhow::Result<T>`; command functions convert to `Result<T, String>` at the Tauri boundary in `src-tauri/src/commands/mod.rs`.
- DTO conversion should happen at boundary modules, not deep inside UI components or core storage functions.

## Module Design

**Exports:**

- Presentational React components usually default-export `memo(Component)`, as in `src/components/skills/SkillCard.tsx`, `src/components/skills/modals/AddSkillModal.tsx`, and `src/components/projects/ProjectsPage.tsx`.
- Frontend DTOs use named `export type` declarations from `src/components/skills/types.ts` and `src/components/projects/types.ts`.
- Feature state hooks export named functions and types: `ProjectState`, `formatProjectError`, and `useProjectState` in `src/components/projects/useProjectState.ts`.
- Rust core modules are declared and exported through `src-tauri/src/core/mod.rs`.
- Rust command modules are declared through `src-tauri/src/commands/mod.rs`; project-specific commands live in `src-tauri/src/commands/projects.rs`.

**Barrel Files:**

- Frontend barrel files are not used; import components directly by file path.
- Rust uses `mod.rs` files for module indexing: `src-tauri/src/core/mod.rs` and `src-tauri/src/commands/mod.rs`.

## Prescriptive Patterns

- Put new user-visible frontend strings in `src/i18n/resources.ts` and consume them through `t('key')` or `t("key")`; do not inline user-visible English in JSX.
- Keep DTOs synchronized between Rust command structs in `src-tauri/src/commands/mod.rs` / `src-tauri/src/commands/projects.rs` and TypeScript types in `src/components/skills/types.ts` / `src/components/projects/types.ts`.
- Keep backend business logic in `src-tauri/src/core/`; keep Tauri commands responsible for argument conversion, `spawn_blocking`, DTO conversion, and error formatting.
- Register new Tauri commands in both the command module and the `generate_handler!` list in `src-tauri/src/lib.rs`.
- Use direct component imports and avoid new barrels, matching existing frontend imports and Vercel bundle guidance.
- Use `useCallback` for callbacks passed across component boundaries when existing nearby code does so, as in `src/components/projects/ProjectsPage.tsx` and `src/components/projects/useProjectState.ts`.
- Use `Promise.all` for independent frontend IPC calls, as in `selectProject` in `src/components/projects/useProjectState.ts`.
- Avoid one-off abstractions with a single caller unless they isolate a real boundary such as IPC, storage, or filesystem behavior.

---

_Convention analysis: 2026-04-29_
