---
phase: 3
slug: ipc-commands
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-04-07
validated: 2026-04-08
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property               | Value                                             |
| ---------------------- | ------------------------------------------------- |
| **Framework**          | Rust built-in test harness (`cargo test`)         |
| **Config file**        | `src-tauri/Cargo.toml`                            |
| **Quick run command**  | `cargo test --manifest-path src-tauri/Cargo.toml` |
| **Full suite command** | `npm run check`                                   |
| **Estimated runtime**  | ~30 seconds                                       |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --manifest-path src-tauri/Cargo.toml`
- **After every plan wave:** Run `npm run check`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command                                 | File Exists | Status   |
| ------- | ---- | ---- | ----------- | ---------- | --------------- | --------- | ------------------------------------------------- | ----------- | -------- |
| 3-01-01 | 01   | 1    | ASGN-01     | —          | N/A             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml` | ✅          | ✅ green |
| 3-01-02 | 01   | 1    | ASGN-04     | —          | N/A             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml` | ✅          | ✅ green |
| 3-01-03 | 01   | 1    | SYNC-02     | —          | N/A             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml` | ✅          | ✅ green |
| 3-01-04 | 01   | 1    | SYNC-03     | —          | N/A             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml` | ✅          | ✅ green |

_Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky_

---

## Wave 0 Requirements

_Existing infrastructure covers all phase requirements._

---

## Manual-Only Verifications

| Behavior                                | Requirement | Why Manual                     | Test Instructions                                                          |
| --------------------------------------- | ----------- | ------------------------------ | -------------------------------------------------------------------------- |
| Frontend can invoke bulk-assign via IPC | ASGN-04     | Requires Tauri webview runtime | Call `invoke('bulk_assign_skill', {...})` from browser console in dev mode |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-04-08

---

## Validation Audit 2026-04-08

| Metric     | Count |
| ---------- | ----- |
| Gaps found | 0     |
| Resolved   | 0     |
| Escalated  | 0     |

### Coverage Detail

| Requirement | Tests Covering It                                                                                                                                     | Files                            |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------- |
| ASGN-01     | `assign_creates_symlink`, `assign_stores_hash_for_copy`, `assign_records_error_on_sync_failure`, `format_anyhow_error_passthrough_assignment_exists`  | `project_sync.rs`, `commands.rs` |
| ASGN-04     | `bulk_assign_to_multiple_tools`, `bulk_assign_skips_already_assigned`, `bulk_assign_continues_on_error`, `bulk_assign_skill_not_found_error_contract` | `project_sync.rs`, `commands.rs` |
| SYNC-02     | `resync_updates_all`, `resync_continues_on_error`                                                                                                     | `project_sync.rs`                |
| SYNC-03     | `resync_all_multiple_projects`                                                                                                                        | `project_sync.rs`                |

**All 119 tests pass.** No new tests needed — existing coverage is complete.
