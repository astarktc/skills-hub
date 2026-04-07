---
phase: 1
slug: data-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-07
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
| **Estimated runtime**  | ~15 seconds                                                   |

---

## Sampling Rate

- **After every task commit:** Run `cd src-tauri && cargo test --lib`
- **After every plan wave:** Run `npm run rust:test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command                        | File Exists | Status     |
| ------- | ---- | ---- | ----------- | ---------- | --------------- | --------- | ---------------------------------------- | ----------- | ---------- |
| 1-01-01 | 01   | 1    | INFR-04     | —          | N/A             | unit      | `cd src-tauri && cargo test skill_store` | ✅          | ⬜ pending |
| 1-01-02 | 01   | 1    | INFR-05     | —          | N/A             | unit      | `cd src-tauri && cargo test skill_store` | ✅          | ⬜ pending |
| 1-02-01 | 02   | 1    | PROJ-01     | —          | N/A             | unit      | `cd src-tauri && cargo test skill_store` | ❌ W0       | ⬜ pending |
| 1-02-02 | 02   | 1    | PROJ-02     | —          | N/A             | unit      | `cd src-tauri && cargo test skill_store` | ❌ W0       | ⬜ pending |
| 1-02-03 | 02   | 1    | PROJ-03     | —          | N/A             | unit      | `cd src-tauri && cargo test skill_store` | ❌ W0       | ⬜ pending |
| 1-03-01 | 03   | 2    | TOOL-01     | —          | N/A             | unit      | `cd src-tauri && cargo test projects`    | ❌ W0       | ⬜ pending |

_Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky_

---

## Wave 0 Requirements

- [ ] `src-tauri/src/core/tests/skill_store.rs` — extend with project CRUD test cases
- [ ] `src-tauri/src/commands/tests/` — add project command tests (if commands/projects.rs module created)

_Existing test infrastructure (tempfile, mockito, cargo test) covers framework needs._

---

## Manual-Only Verifications

| Behavior                                 | Requirement | Why Manual                                 | Test Instructions                                                               |
| ---------------------------------------- | ----------- | ------------------------------------------ | ------------------------------------------------------------------------------- |
| Schema migration preserves existing data | INFR-04     | Requires populated DB with real skill data | 1. Create DB with V3 schema + data, 2. Run migration, 3. Verify old data intact |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
