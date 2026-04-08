---
phase: 2
slug: sync-logic
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-07
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

| Task ID  | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command                    | File Exists | Status     |
| -------- | ---- | ---- | ----------- | ---------- | --------------- | --------- | ------------------------------------ | ----------- | ---------- |
| 02-01-01 | 01   | 1    | ASGN-02     | —          | N/A             | unit      | `cargo test -p app_lib project_sync` | ❌ W0       | ⬜ pending |
| 02-01-02 | 01   | 1    | ASGN-03     | —          | N/A             | unit      | `cargo test -p app_lib project_sync` | ❌ W0       | ⬜ pending |
| 02-01-03 | 01   | 1    | ASGN-05     | —          | N/A             | unit      | `cargo test -p app_lib project_sync` | ❌ W0       | ⬜ pending |
| 02-01-04 | 01   | 1    | SYNC-04     | —          | N/A             | unit      | `cargo test -p app_lib project_sync` | ❌ W0       | ⬜ pending |
| 02-01-05 | 01   | 1    | INFR-01     | —          | N/A             | unit      | `cargo test -p app_lib project_sync` | ❌ W0       | ⬜ pending |
| 02-01-06 | 01   | 1    | INFR-02     | —          | N/A             | unit      | `cargo test -p app_lib project_sync` | ❌ W0       | ⬜ pending |

_Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky_

---

## Wave 0 Requirements

- [ ] `src-tauri/src/core/tests/project_sync.rs` — test module for project sync operations
- [ ] Test fixtures using `tempfile` crate for isolated filesystem scenarios

_Existing test infrastructure (tempfile, mockito, cargo test) covers framework needs._

---

## Manual-Only Verifications

| Behavior                          | Requirement | Why Manual                                  | Test Instructions                                                              |
| --------------------------------- | ----------- | ------------------------------------------- | ------------------------------------------------------------------------------ |
| Cross-filesystem symlink fallback | INFR-01     | Requires WSL2-to-NTFS mount                 | Create project on /mnt/c, assign skill, verify copy created instead of symlink |
| Concurrent UI toggle rapid-fire   | INFR-02     | Requires real Tauri app with UI interaction | Rapidly toggle sync on/off for multiple skills, verify no corruption           |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
