---
phase: 2
slug: sync-logic
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-07
validated: 2026-04-07
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property               | Value                                                         |
| ---------------------- | ------------------------------------------------------------- |
| **Framework**          | Rust built-in test harness (`cargo test`)                     |
| **Config file**        | `src-tauri/Cargo.toml` (test dependencies: tempfile, mockito) |
| **Quick run command**  | `cargo test -p app_lib --lib`                                 |
| **Full suite command** | `npm run check`                                               |
| **Estimated runtime**  | ~30 seconds                                                   |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p app_lib --lib`
- **After every plan wave:** Run `npm run check`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID  | Plan | Wave | Requirement | Test Type | Automated Command                    | Test(s)                                                                                             | Status                       |
| -------- | ---- | ---- | ----------- | --------- | ------------------------------------ | --------------------------------------------------------------------------------------------------- | ---------------------------- |
| 02-01-01 | 01   | 1    | ASGN-02     | unit      | `cargo test -p app_lib project_sync` | `assign_creates_symlink`, `assign_stores_hash_for_copy`, `assign_records_error_on_sync_failure`     | ✅ green                     |
| 02-01-02 | 01   | 1    | ASGN-03     | unit      | `cargo test -p app_lib project_sync` | `unassign_removes_symlink`, `unassign_target_not_found_cleans_db`                                   | ✅ green                     |
| 02-01-03 | 01   | 1    | ASGN-05     | unit      | `cargo test -p app_lib project_sync` | `global_and_project_sync_independent`                                                               | ✅ green                     |
| 02-01-04 | 01   | 1    | SYNC-04     | unit      | `cargo test -p app_lib project_sync` | `staleness_detected_for_copy`, `staleness_skipped_for_symlink`, `staleness_source_missing_no_crash` | ✅ green                     |
| 02-01-05 | 01   | 1    | INFR-01     | manual    | N/A — requires cross-mount           | `assign_stores_hash_for_copy` (partial: copy-mode path only)                                        | ✅ green (automated portion) |
| 02-01-06 | 01   | 1    | INFR-02     | unit      | `cargo test -p app_lib project_sync` | `sync_serialization`                                                                                | ✅ green                     |

_Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky_

---

## Wave 0 Requirements

- [x] `src-tauri/src/core/tests/project_sync.rs` — test module for project sync operations (13 tests)
- [x] Test fixtures using `tempfile` crate for isolated filesystem scenarios

_Existing test infrastructure (tempfile, mockito, cargo test) covers framework needs._

---

## Manual-Only Verifications

| Behavior                          | Requirement | Why Manual                                  | Test Instructions                                                              |
| --------------------------------- | ----------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| Cross-filesystem symlink fallback | INFR-01     | Requires WSL2-to-NTFS mount                 | Create project on /mnt/c, assign skill, verify copy created instead of symlink |
| Concurrent UI toggle rapid-fire   | INFR-02     | Requires real Tauri app with UI interaction | Rapidly toggle sync on/off for multiple skills, verify no corruption           |

---

## Validation Sign-Off

- [x] All tasks have automated verify or justified Manual-Only entry
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated

---

## Validation Audit 2026-04-07

| Metric                | Count                |
| --------------------- | -------------------- |
| Requirements          | 6                    |
| Automated (COVERED)   | 5                    |
| Partial (manual-only) | 1 (INFR-01 cross-fs) |
| Missing               | 0                    |
| Total tests           | 13                   |
| All green             | yes                  |
