---
phase: 4
slug: frontend-component-tree
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-08
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property               | Value                                                       |
| ---------------------- | ----------------------------------------------------------- |
| **Framework**          | Rust test harness (cargo test) — no frontend test framework |
| **Config file**        | src-tauri/Cargo.toml                                        |
| **Quick run command**  | `npm run build`                                             |
| **Full suite command** | `npm run check`                                             |
| **Estimated runtime**  | ~30 seconds                                                 |

---

## Sampling Rate

- **After every task commit:** Run `npm run build`
- **After every plan wave:** Run `npm run check`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID  | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status     |
| -------- | ---- | ---- | ----------- | ---------- | --------------- | --------- | ----------------- | ----------- | ---------- |
| 04-01-01 | 01   | 1    | UI-01       | —          | N/A             | build     | `npm run build`   | ✅          | ⬜ pending |
| 04-01-02 | 01   | 1    | UI-02       | —          | N/A             | build     | `npm run build`   | ✅          | ⬜ pending |
| 04-01-03 | 01   | 1    | UI-03       | —          | N/A             | build     | `npm run build`   | ✅          | ⬜ pending |
| 04-01-04 | 01   | 1    | UI-04       | —          | N/A             | build     | `npm run build`   | ✅          | ⬜ pending |
| 04-01-05 | 01   | 1    | UI-05       | —          | N/A             | build     | `npm run build`   | ✅          | ⬜ pending |
| 04-01-06 | 01   | 1    | SYNC-01     | —          | N/A             | build     | `npm run build`   | ✅          | ⬜ pending |
| 04-01-07 | 01   | 1    | TOOL-02     | —          | N/A             | build     | `npm run build`   | ✅          | ⬜ pending |
| 04-01-08 | 01   | 1    | TOOL-03     | —          | N/A             | build     | `npm run build`   | ✅          | ⬜ pending |

_Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky_

---

## Wave 0 Requirements

_Existing infrastructure covers all phase requirements. No frontend test framework — validation via TypeScript build (tsc --noEmit), ESLint, and Rust checks._

---

## Manual-Only Verifications

| Behavior                              | Requirement | Why Manual                   | Test Instructions                                       |
| ------------------------------------- | ----------- | ---------------------------- | ------------------------------------------------------- |
| Projects tab visible in navigation    | UI-01       | Visual UI rendering          | Launch app, verify "Projects" tab in header             |
| Folder picker opens native dialog     | UI-02       | Native OS dialog interaction | Click register project, verify folder picker opens      |
| Checkbox matrix renders correctly     | UI-03       | Visual grid layout           | Register project with skills, verify matrix grid        |
| Status indicators show correct colors | UI-04       | Visual color verification    | Assign skill, verify green/yellow/red/gray indicators   |
| Tool column add/remove works          | UI-05       | Interactive UI flow          | Add tool column, verify it appears; remove, verify gone |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
