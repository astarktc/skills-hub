---
phase: 3
slug: ipc-commands
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-07
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

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command                                 | File Exists | Status     |
| ------- | ---- | ---- | ----------- | ---------- | --------------- | --------- | ------------------------------------------------- | ----------- | ---------- |
| 3-01-01 | 01   | 1    | ASGN-01     | —          | N/A             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml` | ✅          | ⬜ pending |
| 3-01-02 | 01   | 1    | ASGN-04     | —          | N/A             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml` | ✅          | ⬜ pending |
| 3-01-03 | 01   | 1    | SYNC-02     | —          | N/A             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml` | ✅          | ⬜ pending |
| 3-01-04 | 01   | 1    | SYNC-03     | —          | N/A             | unit      | `cargo test --manifest-path src-tauri/Cargo.toml` | ✅          | ⬜ pending |

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

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
