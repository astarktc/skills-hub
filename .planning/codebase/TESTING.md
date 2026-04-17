# Testing Patterns

**Analysis Date:** 2026-04-07

## Test Framework

**Runner:**

- Rust built-in test harness via `cargo test`
- Config: `package.json` script `rust:test` runs `cd src-tauri && cargo test`

**Assertion Library:**

- Rust standard assertions: `assert!`, `assert_eq!`, and `assert_ne!`
- Error-path assertions frequently inspect formatted error chains with `format!("{:#}", err)` in files such as `src-tauri/src/core/tests/github_search.rs`, `src-tauri/src/commands/tests/commands.rs`, and `src-tauri/src/core/tests/skill_store.rs`

**Run Commands:**

```bash
npm run rust:test         # Run all Rust tests
cd src-tauri && cargo test # Direct Rust test run
npm run check             # Lint, build, format check, clippy, and Rust tests
```

## Test File Organization

**Location:**

- Backend tests are split between inline unit tests and dedicated test modules.
- Inline `#[cfg(test)]` modules exist inside production files for small self-contained logic, such as `src-tauri/src/core/cancel_token.rs` and `src-tauri/src/core/github_download.rs`.
- Most backend tests live in dedicated files under `src-tauri/src/core/tests/`, such as `src-tauri/src/core/tests/installer.rs`, `src-tauri/src/core/tests/sync_engine.rs`, and `src-tauri/src/core/tests/github_search.rs`.
- Command-layer tests live in `src-tauri/src/commands/tests/commands.rs`.
- Frontend test files are not detected under `src/`; there is no Jest, Vitest, Playwright, or React Testing Library configuration in the project root.

**Naming:**

- Rust test files use the module name they cover, for example `src-tauri/src/core/tests/skill_store.rs` and `src-tauri/src/core/tests/onboarding.rs`.
- Individual test names are behavior-oriented snake_case descriptions, such as `groups_by_name_and_detects_conflicts_by_fingerprint` in `src-tauri/src/core/tests/onboarding.rs` and `hybrid_sync_with_overwrite_replaces_existing` in `src-tauri/src/core/tests/sync_engine.rs`.

**Structure:**

```
src-tauri/src/
├── commands/
│   └── tests/
│       └── commands.rs
├── core/
│   ├── tests/
│   │   ├── central_repo.rs
│   │   ├── content_hash.rs
│   │   ├── featured_skills.rs
│   │   ├── git_fetcher.rs
│   │   ├── github_search.rs
│   │   ├── installer.rs
│   │   ├── onboarding.rs
│   │   ├── skill_store.rs
│   │   ├── skills_search.rs
│   │   ├── sync_engine.rs
│   │   ├── temp_cleanup.rs
│   │   └── tool_adapters.rs
│   ├── cancel_token.rs      # contains inline tests
│   └── github_download.rs   # contains inline tests
```

## Test Structure

**Suite Organization:**

```rust
use super::*;

fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

#[test]
fn skills_upsert_list_get_delete() {
    let (_dir, store) = make_store();

    let a = make_skill("a", "A", "/central/a", 10);
    store.upsert_skill(&a).unwrap();

    let listed = store.list_skills().unwrap();
    assert_eq!(listed.len(), 1);
}
```

- This pattern appears in `src-tauri/src/core/tests/skill_store.rs` and `src-tauri/src/commands/tests/commands.rs`: setup helpers at file scope, then many focused `#[test]` functions.

**Patterns:**

- Setup is usually file-local helper functions such as `make_store`, `set_central_path`, `init_git_repo`, and `commit_all` in `src-tauri/src/core/tests/installer.rs` and `src-tauri/src/core/tests/skill_store.rs`.
- Tests favor real filesystem and SQLite interactions in temporary directories instead of abstract mocks, for example in `src-tauri/src/core/tests/sync_engine.rs`, `src-tauri/src/core/tests/content_hash.rs`, and `src-tauri/src/core/tests/installer.rs`.
- Assertions check both returned values and on-disk side effects, such as verifying copied files, created links, or DB rows in `src-tauri/src/core/tests/sync_engine.rs`, `src-tauri/src/core/tests/installer.rs`, and `src-tauri/src/core/tests/skill_store.rs`.
- Error assertions inspect message contents for stability of user-visible or protocol-visible behavior, such as `MULTI_SKILLS|`, `RATE_LIMITED|`, and context text in `src-tauri/src/commands/tests/commands.rs` and `src-tauri/src/core/github_download.rs`.

## Mocking

**Framework:**

- `mockito` is the HTTP mocking library, declared in `src-tauri/Cargo.toml` and used in `src-tauri/src/core/tests/github_search.rs` and inline tests in `src-tauri/src/core/github_download.rs`.
- Tauri app handles are faked with `tauri::test::mock_app()` in integration-style backend tests such as `src-tauri/src/core/tests/installer.rs`.

**Patterns:**

```rust
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
```

- This exact style is used in `src-tauri/src/core/tests/github_search.rs`.

```rust
let app = tauri::test::mock_app();
let (_dir, store) = make_store();
let res = super::install_local_skill(app.handle(), &store, source.path(), Some("local1".to_string())).unwrap();
assert!(res.central_path.exists());
```

- This integration-style command/core testing pattern appears in `src-tauri/src/core/tests/installer.rs`.

**What to Mock:**

- Mock outbound HTTP requests with `mockito` when testing GitHub API behavior, rate limits, status handling, or response mapping, as in `src-tauri/src/core/tests/github_search.rs` and `src-tauri/src/core/github_download.rs`.
- Mock the Tauri runtime with `tauri::test::mock_app()` when a function requires an app handle, as in `src-tauri/src/core/tests/installer.rs`.

**What NOT to Mock:**

- Do not mock local filesystem behavior when testing sync, install, hashing, onboarding, or SQLite store logic. Existing tests exercise real temp directories and real files in `src-tauri/src/core/tests/sync_engine.rs`, `src-tauri/src/core/tests/content_hash.rs`, `src-tauri/src/core/tests/onboarding.rs`, and `src-tauri/src/core/tests/skill_store.rs`.
- Do not mock SQLite for store-layer tests; use a temporary on-disk database via `SkillStore::new(dir.path().join("test.db"))` as in `src-tauri/src/core/tests/skill_store.rs`.

## Fixtures and Factories

**Test Data:**

```rust
fn make_store() -> (tempfile::TempDir, SkillStore) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SkillStore::new(dir.path().join("test.db"));
    store.ensure_schema().expect("ensure_schema");
    (dir, store)
}

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

- This factory style appears in `src-tauri/src/core/tests/skill_store.rs`.

**Location:**

- Fixtures are local helper functions inside each test file rather than shared global fixture modules.
- Temporary directories are created inline with `tempfile::tempdir()` in `src-tauri/src/core/tests/installer.rs`, `src-tauri/src/core/tests/sync_engine.rs`, and `src-tauri/src/core/tests/onboarding.rs`.
- Sample JSON payload builders are also file-local, such as `json_one_repo()` in `src-tauri/src/core/tests/github_search.rs`.

## Coverage

**Requirements:**

- No explicit coverage target or coverage enforcement is detected in `package.json`, the project root, or Rust configuration.
- Quality gates are linting, TypeScript build, Rust fmt, clippy, and Rust tests through `npm run check` in `package.json`.

**View Coverage:**

```bash
Not configured in this repository.
```

## Test Types

**Unit Tests:**

- Small pure or stateful units are tested directly with standard `#[test]` functions, such as `CancelToken` in `src-tauri/src/core/cancel_token.rs`, URL parsing and rate-limit helpers in `src-tauri/src/core/github_download.rs`, and DB helpers in `src-tauri/src/core/tests/skill_store.rs`.

**Integration Tests:**

- Most backend tests are integration-style within the crate: they combine temp filesystem state, SQLite, Tauri mock app handles, and real module calls. Examples include installer flows in `src-tauri/src/core/tests/installer.rs`, onboarding scans in `src-tauri/src/core/tests/onboarding.rs`, and sync behavior in `src-tauri/src/core/tests/sync_engine.rs`.

**E2E Tests:**

- Not used. No browser, desktop UI, or end-to-end framework is detected in the project root.

## Common Patterns

**Async Testing:**

```rust
#[test]
fn installs_local_skill_and_updates_from_source() {
    let app = tauri::test::mock_app();
    let (_dir, store) = make_store();
    let res = super::install_local_skill(app.handle(), &store, source.path(), Some("local1".to_string())).unwrap();
    assert!(res.central_path.exists());
}
```

- Async Tauri commands are generally tested indirectly through synchronous core functions or mock-app-backed helpers rather than async Rust test executors. No `#[tokio::test]` usage is detected.

**Error Testing:**

```rust
let err = search_github_repos_inner(&server.url(), "x", 2, None).unwrap_err();
let msg = format!("{:#}", err);
assert!(msg.contains("GitHub search returned error"), "{msg}");
```

- This pattern appears in `src-tauri/src/core/tests/github_search.rs`.

```rust
let err = match super::install_git_skill(app.handle(), &store, repo_dir.path().to_string_lossy().as_ref(), None, None) {
    Ok(_) => panic!("expected error"),
    Err(e) => e,
};
assert!(format!("{:#}", err).contains("MULTI_SKILLS|"));
```

- This pattern appears in `src-tauri/src/core/tests/installer.rs` and `src-tauri/src/commands/tests/commands.rs`.

## Prescriptive Patterns to Follow

- Add new backend tests next to the covered backend area: `src-tauri/src/core/tests/<module>.rs` for core logic and `src-tauri/src/commands/tests/commands.rs` for command-boundary behavior.
- Prefer real temp directories, real files, and a temporary SQLite database over mocks for filesystem-heavy logic.
- Use `mockito` only for HTTP boundaries and keep mocked assertions precise on URL, query params, status code, and body.
- Write behavior-first test names in snake_case that describe the observable rule, matching `src-tauri/src/core/tests/onboarding.rs` and `src-tauri/src/core/tests/sync_engine.rs`.
- Assert side effects, not only return values: verify files on disk, DB state, and error prefixes when those outputs drive UI behavior.
- Frontend behavior currently lacks automated tests; changes to frontend flows should be verified through build/lint and manual runtime checks until a frontend test framework is introduced.

---

_Testing analysis: 2026-04-07_
