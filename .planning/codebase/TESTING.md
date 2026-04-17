# Testing Patterns

**Analysis Date:** 2026-04-16

## Test Framework

**Runner:**

- Rust built-in test harness via `cargo test`.
- Config: `package.json` scripts call `cargo test`, and CI executes `cargo test --all` from `.github/workflows/ci.yml`.

**Assertion Library:**

- Rust standard assertions: `assert_eq!`, `assert!`, and `unwrap`/`expect`-driven setup checks, used throughout `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/project_sync.rs`, and `src-tauri/src/commands/tests/commands.rs`.

**Run Commands:**

```bash
npm run rust:test        # Run backend tests through package.json
cd src-tauri && cargo test --all   # Run backend tests directly
npm run check            # Lint, build, fmt, clippy, and Rust tests
```

## Test File Organization

**Location:**

- Rust tests are mostly separated into dedicated backend test modules under `src-tauri/src/core/tests/` and `src-tauri/src/commands/tests/`.
- Some backend modules also keep inline `#[cfg(test)]` blocks next to implementation code, for example `src-tauri/src/core/github_download.rs` and `src-tauri/src/core/cancel_token.rs`.
- Frontend test files are not detected under `/home/alexwsl/skills-hub/src`; there is no configured Jest, Vitest, or React Testing Library setup in `package.json`.

**Naming:**

- Use module-focused Rust filenames such as `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/installer.rs`, and `src-tauri/src/core/tests/tool_adapters.rs`.
- Use descriptive snake_case test function names that describe behavior, such as `assign_creates_symlink`, `settings_roundtrip_and_update`, `format_anyhow_error_passthrough_prefixes`, and `limit_is_clamped`.

**Structure:**

```
src-tauri/src/
├── commands/tests/
│   └── commands.rs
├── core/tests/
│   ├── content_hash.rs
│   ├── github_search.rs
│   ├── installer.rs
│   ├── project_ops.rs
│   ├── project_sync.rs
│   ├── skill_lock.rs
│   ├── skill_store.rs
│   ├── sync_engine.rs
│   └── tool_adapters.rs
└── core/*.rs          # Some modules also contain inline #[cfg(test)] tests
```

## Test Structure

**Suite Organization:**

```rust
fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

#[test]
fn settings_roundtrip_and_update() {
    let (_dir, store) = make_store();

    assert_eq!(store.get_setting("missing").unwrap(), None);
    store.set_setting("k", "v1").unwrap();
    assert_eq!(store.get_setting("k").unwrap().as_deref(), Some("v1"));
}
```

- Pattern source: `src-tauri/src/core/tests/skill_store.rs`.

**Patterns:**

- Setup pattern: create a temp directory fixture helper at file scope, then construct a real `SkillStore` or filesystem tree, as in `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/project_sync.rs`, and `src-tauri/src/commands/tests/commands.rs`.
- Teardown pattern: rely on `tempfile::TempDir` RAII cleanup instead of manual teardown.
- Assertion pattern: assert both DTO/state outputs and real filesystem side effects, as in `src-tauri/src/core/tests/project_sync.rs` and `src-tauri/src/core/tests/installer.rs`.

## Mocking

**Framework:** `mockito` for HTTP mocking; `tauri::test::mock_app()` for Tauri app handles in backend tests.

**Patterns:**

```rust
let mut server = mockito::Server::new();
let _m = server
    .mock("GET", "/search/repositories")
    .with_status(200)
    .with_header("content-type", "application/json")
    .with_body(json_one_repo())
    .create();

let out = search_github_repos_inner(&server.url(), "x", 2, None).unwrap();
assert_eq!(out[0].full_name, "o/r");
```

- Pattern source: `src-tauri/src/core/tests/github_search.rs`.

```rust
let app = tauri::test::mock_app();
let res = super::install_local_skill(
    app.handle(),
    &store,
    source.path(),
    Some("local1".to_string()),
)
.unwrap();
```

- Pattern source: `src-tauri/src/core/tests/installer.rs`.

**What to Mock:**

- Outbound HTTP requests to GitHub and other network-facing flows, using `mockito` in `src-tauri/src/core/tests/github_search.rs` and the inline tests in `src-tauri/src/core/github_download.rs`.
- Tauri app handles and runtime shell boundaries using `tauri::test::mock_app()` in `src-tauri/src/core/tests/installer.rs`.

**What NOT to Mock:**

- Do not mock the SQLite store for core persistence logic. Tests instantiate a real file-backed temporary DB in `src-tauri/src/core/tests/skill_store.rs` and `src-tauri/src/commands/tests/commands.rs`.
- Do not mock filesystem behavior when validating sync semantics. Tests create real directories, symlinks, copied files, and missing-path scenarios in `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/tool_adapters.rs`, and `src-tauri/src/core/tests/installer.rs`.

## Fixtures and Factories

**Test Data:**

```rust
fn register_project_and_skill(
    store: &SkillStore,
    project_path: &str,
    skill_name: &str,
    skill_central_path: &str,
) -> (ProjectRecord, SkillRecord) {
    let now = 1000i64;
    let project = ProjectRecord { ... };
    store.register_project(&project).unwrap();

    let skill = SkillRecord { ... };
    store.upsert_skill(&skill).unwrap();

    (project, skill)
}
```

- Pattern source: `src-tauri/src/core/tests/project_sync.rs`.

```rust
fn make_skill(id: &str, name: &str, central_path: &str, updated_at: i64) -> SkillRecord {
    SkillRecord {
        id: id.to_string(),
        name: name.to_string(),
        ...
    }
}
```

- Pattern source: `src-tauri/src/core/tests/skill_store.rs`.

**Location:**

- Keep small fixture helpers in the same test file as the tests that use them, rather than in a shared global fixtures module. This pattern appears in `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/installer.rs`, and `src-tauri/src/commands/tests/commands.rs`.

## Coverage

**Requirements:**

- No explicit coverage threshold or coverage tool is detected in `package.json`, `.github/workflows/ci.yml`, or root config files.
- The enforced quality gate is execution-based: `npm run check` runs frontend lint/build plus Rust fmt, clippy, and tests from `package.json`.

**View Coverage:**

```bash
Not applicable
```

## Test Types

**Unit Tests:**

- Predominant test type. Small, behavior-focused backend tests validate pure parsing, error formatting, adapters, and storage rules in files such as `src-tauri/src/core/tests/tool_adapters.rs`, `src-tauri/src/core/tests/github_search.rs`, and `src-tauri/src/commands/tests/commands.rs`.

**Integration Tests:**

- File-backed and filesystem-backed backend integration tests are common. Examples include:
  - SQLite schema and CRUD integration in `src-tauri/src/core/tests/skill_store.rs`
  - Real symlink/copy synchronization flows in `src-tauri/src/core/tests/project_sync.rs`
  - Git repository and install/update flows in `src-tauri/src/core/tests/installer.rs`
- These tests use real temp directories and real crates rather than mocked repositories or in-memory substitutes.

**E2E Tests:**

- Not used. No frontend/browser E2E framework is detected in `package.json` or project config files.

## Common Patterns

**Async Testing:**

```rust
#[test]
fn limit_is_clamped() {
    let mut server = mockito::Server::new();
    // mock HTTP response
    let out = search_github_repos_inner(&server.url(), "hello", 0, None).unwrap();
    assert_eq!(out.len(), 1);
}
```

- Pattern source: `src-tauri/src/core/tests/github_search.rs`.
- Backend tests usually avoid async test harnesses by testing synchronous inner functions or by invoking command helpers through blocking code paths.

**Error Testing:**

```rust
let err = search_github_repos_inner(&server.url(), "x", 2, None).unwrap_err();
let msg = format!("{:#}", err);
assert!(msg.contains("GitHub search returned error"), "{msg}");
```

- Pattern source: `src-tauri/src/core/tests/github_search.rs`.

```rust
let err = anyhow::anyhow!("NOT_FOUND|project:abc-123");
assert_eq!(format_anyhow_error(err), "NOT_FOUND|project:abc-123");
```

- Pattern source: `src-tauri/src/commands/tests/commands.rs`.
- Preserve externally consumed error contracts with exact string assertions when the frontend depends on parsing prefixes.

## Prescriptive Guidance

- Add new backend tests next to the backend area they exercise, usually under `src-tauri/src/core/tests/` or `src-tauri/src/commands/tests/`.
- Start each persistence-heavy test file with a local helper like `make_store()` that creates a temporary DB and runs `ensure_schema()`, matching `src-tauri/src/core/tests/skill_store.rs` and `src-tauri/src/commands/tests/commands.rs`.
- Prefer real temp directories and real file operations for sync/install/path logic, matching `src-tauri/src/core/tests/project_sync.rs` and `src-tauri/src/core/tests/installer.rs`.
- Use `mockito` only at HTTP boundaries, not for internal business logic.
- When adding frontend functionality, plan for manual verification because no frontend test runner is currently configured in `/home/alexwsl/skills-hub`.
- Keep critical frontend-backend contracts covered by backend tests that assert exact DTO fields or exact error prefix strings, because the UI depends on them in `src/App.tsx` and `src/components/projects/useProjectState.ts`.

---

_Testing analysis: 2026-04-16_
