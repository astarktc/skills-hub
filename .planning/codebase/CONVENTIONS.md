# Coding Conventions

**Analysis Date:** 2026-04-16

## Naming Patterns

**Files:**

- Use PascalCase filenames for React component modules in `src/components/skills/` and `src/components/projects/`, for example `src/components/skills/SkillCard.tsx`, `src/components/skills/modals/AddSkillModal.tsx`, and `src/components/projects/ProjectsPage.tsx`.
- Use lowercase or framework-standard filenames for app bootstrap and support modules, for example `src/main.tsx`, `src/i18n/index.ts`, and `src/i18n/resources.ts`.
- Use snake_case filenames for Rust production modules in `src-tauri/src/core/` and `src-tauri/src/commands/`, for example `src-tauri/src/core/skill_store.rs`, `src-tauri/src/core/project_sync.rs`, and `src-tauri/src/commands/projects.rs`.
- Mirror backend areas in Rust test filenames under `src-tauri/src/core/tests/` and `src-tauri/src/commands/tests/`, for example `src-tauri/src/core/tests/skill_store.rs` and `src-tauri/src/commands/tests/commands.rs`.

**Functions:**

- Use PascalCase for React component functions, for example `SkillCard` in `src/components/skills/SkillCard.tsx`, `ProjectsPage` in `src/components/projects/ProjectsPage.tsx`, and `AddProjectModal` in `src/components/projects/AddProjectModal.tsx`.
- Use camelCase for TypeScript helpers, callbacks, and hook-returned actions, for example `formatProjectError`, `loadProjects`, `toggleAssignment`, `getGithubInfo`, and `formatRelative` in `src/components/projects/useProjectState.ts` and `src/App.tsx`.
- Use snake_case for Rust functions and methods, for example `register_project`, `update_project_path`, `assign_and_sync`, and `format_anyhow_error` in `src-tauri/src/commands/projects.rs`, `src-tauri/src/core/project_sync.rs`, and `src-tauri/src/commands/mod.rs`.
- Keep Tauri command function names in snake_case, but use camelCase parameter names on the Rust side when values are passed from the frontend, for example `projectId` and `skillId` in `src-tauri/src/commands/projects.rs`.

**Variables:**

- Use camelCase for frontend locals, props, and React state, for example `pendingGitignoreRef`, `selectedProjectId`, `toolStatus`, `showAddModal`, and `updateAvailableVersion` in `src/components/projects/ProjectsPage.tsx`, `src/components/projects/useProjectState.ts`, and `src/App.tsx`.
- Prefer boolean prefixes such as `is*`, `show*`, `can*`, and `has*`, for example `isDuplicate`, `showRemoveModal`, `canClose`, and `has_conflict` across `src/components/projects/AddProjectModal.tsx`, `src/components/skills/modals/AddSkillModal.tsx`, and backend DTOs.
- Use snake_case for Rust locals and struct fields, for example `project_id`, `skill_id`, `last_error`, `synced_at`, and `content_hash` in `src-tauri/src/core/tests/project_sync.rs` and `src-tauri/src/core/skill_store.rs`.

**Types:**

- Use PascalCase with descriptive suffixes for frontend props and DTO aliases, for example `SkillCardProps`, `AddProjectModalProps`, `ToolStatusDto`, and `ManagedSkill` in `src/components/skills/SkillCard.tsx`, `src/components/projects/AddProjectModal.tsx`, and `src/components/skills/types.ts`.
- Use PascalCase for Rust structs and DTOs, especially IPC-facing types, for example `ProjectDto`, `ProjectSkillAssignmentDto`, `ProjectToolDto`, and `ResyncSummaryDto` in `src-tauri/src/commands/projects.rs` and `src-tauri/src/core/project_ops.rs`.
- Keep TypeScript DTO names and fields aligned with the Rust command layer. `src/components/skills/types.ts` mirrors DTO shapes returned from `src-tauri/src/commands/mod.rs`, and `src/components/projects/types.ts` mirrors DTOs returned from `src-tauri/src/commands/projects.rs`.

## Code Style

**Formatting:**

- Tool used: TypeScript uses a Prettier-like style enforced socially plus ESLint; Rust uses `cargo fmt` via `package.json` scripts and CI in `.github/workflows/ci.yml`.
- Key settings:
  - Frontend code uses single quotes in older files such as `src/components/skills/modals/AddSkillModal.tsx`, but newer project feature files use double quotes in files such as `src/components/projects/ProjectsPage.tsx` and `src/components/projects/useProjectState.ts`. Preserve the dominant style of the file you edit instead of normalizing unrelated files.
  - Many frontend files omit semicolons in the older skills area, while newer project-area files include semicolons. Match the surrounding file.
  - Multiline arrays, objects, and JSX props use trailing commas in files such as `src/App.tsx`, `src/components/projects/ProjectsPage.tsx`, and `src/components/projects/useProjectState.ts`.
  - JSX becomes one-prop-per-line once elements grow beyond trivial size, as shown in `src/components/skills/modals/AddSkillModal.tsx` and `src/components/projects/AddProjectModal.tsx`.
  - CSS lives in global stylesheets `src/App.css` and `src/index.css`, not CSS Modules.

**Linting:**

- Tool used: ESLint flat config in `eslint.config.js` for `**/*.{ts,tsx}`.
- Key rules:
  - Extend `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, and `eslint-plugin-react-refresh` in `eslint.config.js`.
  - Ignore build output directories `dist` and `src-tauri/target` in `eslint.config.js`.
  - TypeScript strictness is enforced in `tsconfig.app.json` with `strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`, and `noUncheckedSideEffectImports` enabled.
  - Treat unused variables and parameters as build-breaking, not advisory, because `npm run build` runs `tsc -b` from `package.json`.

## Import Organization

**Order:**

1. External packages and framework imports first, for example React, i18n, Tauri plugins, icons, and `sonner` in `src/App.tsx`, `src/components/skills/SkillCard.tsx`, and `src/components/projects/ProjectsPage.tsx`.
2. Internal component, type, and style imports second, for example `./components/skills/ExplorePage`, `./types`, and `./App.css` in `src/App.tsx` and component modules.
3. Type-only imports are separated with `import type` where practical, for example `import type { TFunction } from 'i18next'` in `src/components/skills/SkillCard.tsx` and `src/components/projects/AddProjectModal.tsx`.

**Path Aliases:**

- Not detected. Use relative imports such as `./types`, `../skills/types`, and `./components/projects/ProjectsPage` in `src/components/projects/useProjectState.ts`, `src/components/projects/AssignmentMatrix.tsx`, and `src/App.tsx`.

## Error Handling

**Patterns:**

- Wrap async UI actions in `try/catch` and normalize unknown values with `err instanceof Error ? err.message : String(err)`, as shown in `src/components/projects/ProjectsPage.tsx`, `src/components/projects/useProjectState.ts`, and `src/App.tsx`.
- Convert backend-prefixed wire errors into translated user-facing messages in one place. Follow `formatErrorMessage()` in `src/App.tsx` for general skill flows and `formatProjectError()` in `src/components/projects/useProjectState.ts` for project flows.
- Use optimistic or semi-optimistic flows with corrective re-fetch on failure in project state logic, for example `toggleAssignment()` and `bulkAssign()` in `src/components/projects/useProjectState.ts` re-read assignments after failures.
- On the backend, return `Result<_, String>` at the Tauri boundary and map internal `anyhow::Error` values through `format_anyhow_error` in `src-tauri/src/commands/mod.rs` and `src-tauri/src/commands/projects.rs`.
- Preserve machine-readable prefixes for frontend parsing, such as `MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, `DUPLICATE_PROJECT|`, `ASSIGNMENT_EXISTS|`, and `NOT_FOUND|`, verified by tests in `src-tauri/src/commands/tests/commands.rs`.
- Run blocking backend work inside `tauri::async_runtime::spawn_blocking`, as shown throughout `src-tauri/src/commands/projects.rs`.
- Add backend context with `anyhow::Context`, for example command modules and storage/database code such as `src-tauri/src/commands/projects.rs` and `src-tauri/src/core/skill_store.rs`.

## Logging

**Framework:** `sonner` toasts on the frontend; Rust `log` crate with Tauri logging on the backend.

**Patterns:**

- Use `toast.success`, `toast.error`, and `toast.warning` for user-visible feedback, not `console.log`, as shown in `src/App.tsx`, `src/components/projects/AssignmentMatrix.tsx`, and `src/components/skills/SkillCard.tsx`.
- Surface localized success and warning states from UI handlers, for example in `src/components/projects/ProjectsPage.tsx` and `src/components/projects/AssignmentMatrix.tsx`.
- Silent catches are acceptable only for intentionally non-critical paths, such as localStorage access in `src/i18n/index.ts` and background consistency refreshes in `src/components/projects/useProjectState.ts`.

## Comments

**When to Comment:**

- Comment only when behavior depends on sequencing, invariants, or safety assumptions. Examples include the gitignore sequencing explanation in `src/components/projects/ProjectsPage.tsx` and backend cleanup/migration notes in `src-tauri/src/lib.rs` and `src-tauri/src/core/skill_store.rs`.
- Do not add comments for straightforward JSX, simple setters, or obvious render logic. Most component files such as `src/components/skills/SkillCard.tsx` and `src/components/projects/AddProjectModal.tsx` rely on readable code instead.

**JSDoc/TSDoc:**

- Minimal usage. Prefer expressive type aliases and prop types over heavy docblocks in TypeScript. Small targeted doc comments appear more often in Rust internals than in frontend files.

## Function Design

**Size:**

- Keep presentational components focused and file-local, for example `src/components/skills/SkillCard.tsx`, `src/components/projects/AddProjectModal.tsx`, and `src/components/projects/ProjectList.tsx`.
- Accept that orchestration files are larger when they coordinate app-wide behavior. `src/App.tsx` remains the main stateful coordinator for the skills area, while `src/components/projects/useProjectState.ts` centralizes stateful project workflows.

**Parameters:**

- Pass typed prop objects into React components, then destructure in the function signature, as shown in `src/components/skills/modals/AddSkillModal.tsx`, `src/components/projects/AddProjectModal.tsx`, and `src/components/skills/SkillCard.tsx`.
- Pass primitive values and domain values upward in callbacks rather than raw DOM events, for example `onLocalPathChange(event.target.value)` in `src/components/skills/modals/AddSkillModal.tsx` and `onToggleAssignment(skillId, tool)` in `src/components/projects/AssignmentMatrix.tsx`.
- Use generics on Tauri `invoke` wrappers so callers receive typed DTOs, as shown in `invokeTauri<T>()` in `src/App.tsx` and typed `invoke<ProjectDto[]>()` calls in `src/components/projects/useProjectState.ts`.

**Return Values:**

- Return normalized primitives, nullable objects, or DTOs from frontend helpers, for example `getGithubInfo()` in `src/App.tsx` and `shortRepoLabel()` in `src/components/projects/AssignmentMatrix.tsx`.
- In Rust, return `Result<T>` internally and convert to serializable DTOs at the command boundary in `src-tauri/src/commands/projects.rs` and `src-tauri/src/commands/mod.rs`.
- Prefer early returns to flatten branches, for example modal guards like `if (!open) return null` in `src/components/skills/modals/AddSkillModal.tsx` and `src/components/projects/AddProjectModal.tsx`.

## Module Design

**Exports:**

- Default-export presentational React components wrapped in `memo()`, for example `src/components/skills/SkillCard.tsx`, `src/components/skills/modals/AddSkillModal.tsx`, `src/components/projects/AddProjectModal.tsx`, and `src/components/projects/ProjectsPage.tsx`.
- Use named exports for shared types and hooks, for example `ManagedSkill` in `src/components/skills/types.ts`, `ProjectState` and `useProjectState` in `src/components/projects/useProjectState.ts`, and DTOs in project/skills type files.
- Keep backend command modules as named public functions grouped by area, as shown in `src-tauri/src/commands/projects.rs`.

**Barrel Files:**

- Not used on the frontend. Import directly from concrete file paths, such as `./components/skills/ExplorePage` in `src/App.tsx` and `./ProjectList` in `src/components/projects/ProjectsPage.tsx`.
- Rust uses `mod.rs` aggregation instead of JavaScript-style barrels, for example `src-tauri/src/core/mod.rs` and `src-tauri/src/commands/mod.rs`.

---

_Convention analysis: 2026-04-16_
