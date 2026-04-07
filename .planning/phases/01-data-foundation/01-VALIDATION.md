---
phase: 1
slug: data-foundation
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-07
validated: 2026-04-07
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property               | Value                                                         |
| ---------------------- | ------------------------------------------------------------- |
| **Framework**          | Rust built-in test harness (`cargo test`)                     |
| **Config file**        | `src-tauri/Cargo.toml` (test dependencies: tempfile, mockito) |
| **Quick run command**  | `cd src-tauri && cargo test --lib`                            |
| **Full suite command** | `npm run rust:test`                                           |
| **Estimated runtime**  | ~3 seconds (96 tests)                                         |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib`
- **After every plan wave:** Run `npm run rust:test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 3 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command                           | File Exists | Status   |
| ------- | ---- | ---- | ----------- | ---------- | --------------- | --------- | ------------------------------------------- | ----------- | -------- |
| 1-01-01 | 01   | 1    | INFR-04     | T-01-01    | params![] SQL   | unit      | `cd src-tauri && cargo test skill_store`    | ✅          | ✅ green |
| 1-01-02 | 01   | 1    | INFR-05     | —          | N/A             | compile   | `cd src-tauri && cargo check`               | ✅          | ✅ green |
| 1-02-01 | 02   | 1    | PROJ-01     | T-02-01    | canonicalize    | unit      | `cd src-tauri && cargo test project_ops`    | ✅          | ✅ green |
| 1-02-02 | 02   | 1    | PROJ-02     | T-02-03    | params![] SQL   | unit      | `cd src-tauri && cargo test skill_store`    | ✅          | ✅ green |
| 1-02-03 | 02   | 1    | PROJ-03     | —          | N/A             | unit      | `cd src-tauri && cargo test aggregate_sync` | ✅          | ✅ green |
| 1-03-01 | 03   | 2    | TOOL-01     | —          | N/A             | unit      | `cd src-tauri && cargo test project_tools`  | ✅          | ✅ green |

_Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky_

---

## Test Coverage Detail

| Requirement | Tests (35 total)                                                                                                                                                                                                                                                    | Test File                                      |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| INFR-04     | `v4_migration_creates_project_tables`, `v4_migration_preserves_existing_data`, `v4_tables_have_correct_constraints`, `schema_is_idempotent`                                                                                                                         | `tests/skill_store.rs`                         |
| INFR-05     | Structural — `commands/projects.rs` exists, `pub mod projects` in mod.rs, 9 commands registered in `generate_handler![]`                                                                                                                                            | compile-time verification                      |
| PROJ-01     | `register_project`, `register_rejects_non_dir`, `register_rejects_empty_path`, `register_stores_canonical_path`, `register_rejects_duplicate`, `get_project_by_path`, `get_project_by_id`                                                                           | `tests/skill_store.rs`, `tests/project_ops.rs` |
| PROJ-02     | `delete_project`, `delete_project_cascades_tools_and_assignments`, `delete_skill_cascades_project_assignments`, `assignment_crud`, `assignment_unique_constraint`, `list_project_assignments_by_project`, `list_project_skill_assignments_for_project_tool_filters` | `tests/skill_store.rs`                         |
| PROJ-03     | `aggregate_sync_status_all_synced`, `aggregate_sync_status_mixed`, `aggregate_sync_status_no_assignments`, `to_project_dto_includes_sync_status`, `list_project_dtos_returns_counts`                                                                                | `tests/skill_store.rs`, `tests/project_ops.rs` |
| TOOL-01     | `project_tools_crud`, `project_tools_duplicate_ignored`, `count_project_assignments_and_tools`                                                                                                                                                                      | `tests/skill_store.rs`                         |

---

## Manual-Only Verifications

| Behavior                                 | Requirement | Why Manual                                 | Test Instructions                                                               |
| ---------------------------------------- | ----------- | ------------------------------------------ | ------------------------------------------------------------------------------- |
| Schema migration preserves existing data | INFR-04     | Requires populated DB with real skill data | 1. Create DB with V3 schema + data, 2. Run migration, 3. Verify old data intact |

_Note: `v4_migration_preserves_existing_data` test partially covers this — it creates a V3-era store with data, calls ensure_schema again, and verifies survival. The manual test covers real-world DB files with more varied data._

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 15s (actual: ~3s)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-04-07

---

## Validation Audit 2026-04-07

| Metric     | Count |
| ---------- | ----- |
| Gaps found | 0     |
| Resolved   | 0     |
| Escalated  | 0     |

**Evidence:** `cargo test --lib` — 96 passed, 0 failed, 0 ignored (3.15s). All 6 requirements (INFR-04, INFR-05, PROJ-01, PROJ-02, PROJ-03, TOOL-01) have automated test coverage across 35 phase-specific tests in 2 test files.
