# Testing Patterns

**Analysis Date:** 2026-04-29

## Test Framework

**Runner:**

- Rust built-in test harness (`cargo test`) for backend tests under `src-tauri/src/core/tests/` and `src-tauri/src/commands/tests/`.
- Config: `src-tauri/Cargo.toml` defines Rust dependencies and dev-dependencies; no separate Rust test config file is detected.
- No dedicated frontend test runner is detected. `package.json` includes `playwright` as a dev dependency, but no `test`, `test:e2e`, `vitest`, `jest`, or Playwright script/config is detected.

**Assertion Library:**

- Rust standard assertions: `assert!`, `assert_eq!`, `assert_ne!` in files such as `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/skill_store.rs`, and `src-tauri/src/commands/tests/commands.rs`.
- Error assertions inspect formatted messages with `format!("{:#}", err)` or `err.to_string()`, as in `src-tauri/src/core/tests/github_search.rs` and `src-tauri/src/core/tests/skill_store.rs`.

**Run Commands:**

```bash
npm run rust:test      # Run backend Rust tests via `cd src-tauri && cargo test`
npm run check          # Run lint, TypeScript build, Rust format check, Clippy, and Rust tests
npm run build          # Type-check frontend with `tsc -b` and build with Vite
npm run lint           # Run ESLint across TypeScript/TSX files
npm run rust:clippy    # Run Rust Clippy with warnings denied
```

## Test File Organization

**Location:**

- Backend core tests are separate files under `src-tauri/src/core/tests/`, such as `src-tauri/src/core/tests/project_ops.rs`, `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/skill_store.rs`, and `src-tauri/src/core/tests/github_search.rs`.
- Backend command-layer tests are under `src-tauri/src/commands/tests/commands.rs`.
- Inline test modules are not the dominant pattern; use separate test files for new backend module coverage.
- Frontend `.test.*` or `.spec.*` files are not detected under `src/`.

**Naming:**

- Test files use the module name they cover: `src-tauri/src/core/tests/content_hash.rs` tests `src-tauri/src/core/content_hash.rs`, `src-tauri/src/core/tests/project_sync.rs` tests `src-tauri/src/core/project_sync.rs`.
- Test functions use snake_case and describe expected behavior: `register_rejects_non_dir`, `assign_creates_symlink`, `resync_continues_on_error`, `format_anyhow_error_passthrough_prefixes`.
- Helper functions use snake*case and usually start with `make*\*`or describe their fixture role:`make_store`, `make_skill`, `make_skill_dir`, `register_project_and_skill`.

**Structure:**

```text
src-tauri/src/
├── commands/
│   ├── mod.rs
│   ├── projects.rs
│   └── tests/
│       └── commands.rs
└── core/
    ├── project_sync.rs
    ├── skill_store.rs
    ├── github_search.rs
    └── tests/
        ├── project_sync.rs
        ├── skill_store.rs
        ├── github_search.rs
        └── ...
```

## Test Structure

**Suite Organization:**

```rust
use std::fs;
use std::path::Path;

use crate::core::project_sync;
use crate::core::skill_store::{ProjectRecord, SkillRecord, SkillStore};

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

#[test]
fn assign_creates_symlink() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    // arrange filesystem and records
    // act by calling the core function directly
    // assert database state and filesystem artifacts
}
```

**Patterns:**

- Use small per-file fixtures at the top of test files, not a global fixture framework. Examples: `make_store` in `src-tauri/src/core/tests/skill_store.rs`, `make_skill_dir` in `src-tauri/src/core/tests/project_sync.rs`, and `json_one_repo` in `src-tauri/src/core/tests/github_search.rs`.
- Arrange/act/assert sections are often expressed with brief comments for filesystem-heavy tests in `src-tauri/src/core/tests/project_sync.rs` and `src-tauri/src/core/tests/project_ops.rs`.
- Tests call backend core functions directly rather than going through Tauri IPC whenever possible, for example `project_sync::assign_and_sync` in `src-tauri/src/core/tests/project_sync.rs` and `project_ops::register_project_path` in `src-tauri/src/core/tests/project_ops.rs`.
- Command tests target pure helpers and DTO mappers from the command layer, such as `format_anyhow_error`, `expand_home_path`, `remove_path_any`, and `get_managed_skills_impl` in `src-tauri/src/commands/tests/commands.rs`.
- Use clear assertion messages when the failure needs context: `assert!(msg.contains("GitHub search returned error"), "{msg}")` in `src-tauri/src/core/tests/github_search.rs` and custom messages in `src-tauri/src/core/tests/project_ops.rs`.
- Keep tests deterministic by using fixed timestamps where possible, such as `1000i64`, `2000`, and `3000` in `src-tauri/src/core/tests/project_sync.rs`. Use `now_ms()` only when exact time is not part of the behavior under test, as in `src-tauri/src/core/tests/project_ops.rs`.

## Mocking

**Framework:**

- `mockito` for HTTP server mocking in Rust tests; configured in `src-tauri/Cargo.toml` under `[dev-dependencies]`.
- `tempfile` for temporary directories and files in Rust filesystem and SQLite tests; configured in `src-tauri/Cargo.toml` under `[dev-dependencies]`.
- No frontend mocking framework is detected.

**Patterns:**

```rust
use mockito::Matcher;

#[test]
fn maps_fields() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/search/repositories")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("q".into(), "x".into()),
            Matcher::UrlEncoded("per_page".into(), "2".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json_one_repo())
        .create();

    let out = search_github_repos_inner(&server.url(), "x", 2, None).unwrap();
    assert_eq!(out[0].full_name, "o/r");
}
```

**What to Mock:**

- Mock external HTTP endpoints with `mockito::Server`, as in `src-tauri/src/core/tests/github_search.rs`, `src-tauri/src/core/tests/skills_search.rs`, and `src-tauri/src/core/tests/featured_skills.rs`.
- Mock home/path expansion or input normalization by injecting helper functions where production code supports it; `test_expand_home` is passed into `project_ops::register_project_path` in `src-tauri/src/core/tests/project_ops.rs`.
- Use temporary SQLite files rather than mocking `SkillStore`; tests in `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/project_ops.rs`, and `src-tauri/src/core/tests/project_sync.rs` exercise the real `rusqlite` store.
- Use temporary filesystem trees rather than mocking filesystem behavior for sync, cleanup, hashing, and installer tests.

**What NOT to Mock:**

- Do not mock `SkillStore` for store and workflow tests; use `SkillStore::new(temp_dir.path().join("test.db"))` and `ensure_schema()`.
- Do not mock the filesystem for sync-engine behavior; create real temp directories and assert actual files, directories, symlinks, and copy outputs.
- Do not call real GitHub or skills.sh APIs from tests; use inner functions that accept a base URL and point them to `mockito`, as in `search_github_repos_inner` in `src-tauri/src/core/tests/github_search.rs`.
- Do not require a real Tauri app/window for core behavior; keep logic in `src-tauri/src/core/` so it can be exercised directly.

## Fixtures and Factories

**Test Data:**

```rust
fn make_skill(id: &str, name: &str, central_path: &str, updated_at: i64) -> SkillRecord {
    SkillRecord {
        id: id.to_string(),
        name: name.to_string(),
        description: None,
        source_type: "local".to_string(),
        source_ref: Some("/tmp/source".to_string()),
        source_subpath: None,
        source_revision: None,
        central_path: central_path.to_string(),
        content_hash: None,
        created_at: 1,
        updated_at,
        last_sync_at: None,
        last_seen_at: 1,
        status: "ok".to_string(),
    }
}
```

**Location:**

- Fixtures are local helper functions inside each Rust test file. There is no shared `fixtures/` directory.
- SQLite fixture setup appears as `make_store` in `src-tauri/src/core/tests/skill_store.rs`, `src-tauri/src/core/tests/project_ops.rs`, `src-tauri/src/core/tests/project_sync.rs`, and `src-tauri/src/commands/tests/commands.rs`.
- Filesystem fixture setup appears as `make_skill_dir` in `src-tauri/src/core/tests/project_sync.rs` and `src-tauri/src/core/tests/project_ops.rs`.
- HTTP JSON fixtures are string helper functions, such as `json_one_repo` in `src-tauri/src/core/tests/github_search.rs`.

## Coverage

**Requirements:** None enforced by configuration. No coverage threshold or coverage command is detected in `package.json`, `src-tauri/Cargo.toml`, or root test config files.

**View Coverage:**

```bash
# Not configured. Use cargo tooling manually if coverage is needed, but no repo-standard command exists.
```

## Test Types

**Unit Tests:**

- Backend unit tests cover pure helpers and storage behavior: `src-tauri/src/core/tests/content_hash.rs`, `src-tauri/src/core/tests/skill_lock.rs`, `src-tauri/src/core/tests/temp_cleanup.rs`, `src-tauri/src/commands/tests/commands.rs`.
- Use direct function calls and focused assertions. Example: `format_anyhow_error_passthrough_prefixes` in `src-tauri/src/commands/tests/commands.rs` verifies command error contracts without starting Tauri.

**Integration Tests:**

- Backend integration-style tests combine real SQLite, temp filesystems, and core workflows. Examples: `src-tauri/src/core/tests/project_sync.rs`, `src-tauri/src/core/tests/project_ops.rs`, `src-tauri/src/core/tests/installer.rs`, and `src-tauri/src/core/tests/sync_engine.rs`.
- HTTP integration boundaries are tested with `mockito` instead of live network calls in `src-tauri/src/core/tests/github_search.rs`, `src-tauri/src/core/tests/skills_search.rs`, and `src-tauri/src/core/tests/featured_skills.rs`.
- Schema and migration behavior is tested through real SQLite operations in `src-tauri/src/core/tests/skill_store.rs`.

**E2E Tests:**

- Not used. `playwright` is present in `package.json`, but no Playwright config, browser test files, or npm script is detected.
- Tauri UI workflows are not covered by automated frontend/E2E tests in the current repo state.

**Frontend Tests:**

- No frontend test files are detected under `src/`.
- Frontend correctness is currently enforced through `npm run lint` and `npm run build`, not component tests.
- When adding frontend tests in the future, establish a repo-standard runner and scripts before adding ad-hoc test files.

## Common Patterns

**Async Testing:**

```rust
#[test]
fn resync_continues_on_error() {
    let (_db_dir, store) = make_store();
    let tmpdir = tempfile::tempdir().expect("tmpdir");

    // Build real project/skill fixtures.
    // Force one assignment to fail.
    let summary = project_sync::resync_project(&store, &project.id, 3000)
        .expect("resync_project should succeed overall");

    assert_eq!(summary.synced, 1, "one assignment should succeed");
    assert_eq!(summary.failed, 1, "one assignment should fail");
}
```

- Backend core tests are synchronous. Even when production commands are async Tauri commands, tests usually target the synchronous inner/core function.
- Test `spawn_blocking` command behavior indirectly through pure command helpers unless a full Tauri test is necessary.

**Error Testing:**

```rust
#[test]
fn http_error_has_context() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/search/repositories")
        .with_status(500)
        .with_body("oops")
        .create();

    let err = search_github_repos_inner(&server.url(), "x", 2, None).unwrap_err();
    let msg = format!("{:#}", err);
    assert!(msg.contains("GitHub search returned error"), "{msg}");
}
```

- Use `unwrap_err()` when failure is expected, then assert on stable substrings or prefixes.
- For frontend-dependent error contracts, assert exact prefix preservation in command tests, as in `format_anyhow_error_passthrough_duplicate_project`, `format_anyhow_error_passthrough_assignment_exists`, and `bulk_assign_skill_not_found_error_contract` in `src-tauri/src/commands/tests/commands.rs`.
- Prefer stable machine-readable prefixes over translated or full human messages when asserting cross-layer contracts.

**Database Testing:**

```rust
fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("test.db");
    let store = SkillStore::new(db);
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}
```

- Always call `ensure_schema()` before using a `SkillStore` in tests.
- Keep the returned `TempDir` alive for the whole test so the database file is not deleted early.
- Test schema migrations and constraints through actual insert/list/get operations in `src-tauri/src/core/tests/skill_store.rs`.

**Filesystem Testing:**

```rust
fn make_skill_dir(base: &Path, name: &str) -> std::path::PathBuf {
    let dir = base.join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(dir.join("SKILL.md"), "# Test Skill\nTest content.").expect("write SKILL.md");
    dir
}
```

- Use `tempfile::tempdir()` for every test that writes files.
- Assert both logical outcomes and filesystem artifacts: `target.exists()`, `target.symlink_metadata().unwrap().file_type().is_symlink()`, and missing-target checks in `src-tauri/src/core/tests/project_sync.rs`.
- Gate platform-specific assertions with `#[cfg(unix)]` when needed, as in `remove_path_any_removes_symlink_only` in `src-tauri/src/commands/tests/commands.rs`.

## Test Addition Guidance

- Put new backend tests next to the relevant module under `src-tauri/src/core/tests/` or `src-tauri/src/commands/tests/`.
- Add test helpers locally first. Extract shared helpers only after the same setup appears in at least three places, following `.claude/skills/code-simplification/SKILL.md`.
- Prefer testing `src-tauri/src/core/` functions directly; keep command tests for IPC contracts, DTO mapping, and command-specific error formatting.
- For new project-sync functionality, include assertions for SQLite state and target filesystem state because existing tests exercise both.
- For new external API functionality, expose or use an inner function that accepts a base URL so tests can use `mockito` and avoid live network calls.
- Run `npm run check` after test changes because Rust tests, Clippy, format checks, TypeScript build, and ESLint are all part of the project quality gate.

---

_Testing analysis: 2026-04-29_
