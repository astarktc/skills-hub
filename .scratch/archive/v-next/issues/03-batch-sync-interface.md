# Deepen sync: one batch interface in core

Status: resolved

Type: grilling
Blocked by: 02

## Question

`sync_skill_to_tool` is shallow (one skill×tool pair per call), so the fan-out choreography — filter selected+installed → `uniqueToolIdsBySkillsDir` dedupe → loop → skip `TOOL_NOT_INSTALLED`/`TOOL_NOT_WRITABLE` → collect errors — is re-implemented ~8× in untested App.tsx (828–880, ~1300–1350, 2176–2250, 2272–2330, plus install-then-sync blocks at 1476, 1597, 1724, 1793, 1973, 2085), with subtle divergence (`overwrite` vs `overwriteIfSameContent`).

Decide, then implement (per map Notes):

- The batch command's interface in `core/`: `sync_skill_to_tools(skills, tools, policy) → per-target results`? What is the policy vocabulary (skip-not-installed, overwrite modes) and the structured result shape (uses ticket 02's typed errors)?
- Does progress reporting need to stream to the UI (Tauri events), or are per-target results-on-completion enough?
- **Folded in (scan finding 7)**: the shared-skills-dir invariant is re-derived on both sides of the seam (`adapters_sharing_skills_dir` in Rust; `sharedToolIdsByToolId`/`uniqueToolIdsBySkillsDir` in App.tsx:615–647). Backend owns grouping; expose it as data (`shared_with` on `ToolInfoDto`) for the SharedDirModal UX; delete the frontend dedupe.
- Which of the ~8 App.tsx call sites collapse now vs during ticket 05's hook carve?

Context: scan findings 2 and 7; report cards #2 and #8. Rust-side tests for the batch semantics; DTO changes mirrored per the type strategy from tickets 06/07.

## Answer

Landed green on main as **`8484538`** “refactor: replace the ~9-copy sync fan-out with one backend batch command (v-next ticket 03)”. Gate: `npm run version:check` + `npm run check` exit 0 (unmasked); `cargo test --all` 212 passed (+6 batch tests); clippy `-D warnings` and eslint clean. Verification was compilers + tests only — no runtime smoke (dev runs mutate the real skill library).

Grilled one round (Q1–Q8); Alex accepted all recommendations.

**1. Interface — one batch command, single-pair command deleted (Q1).**
`sync_skills_to_tools(skills, tools, policy, on_progress) → BatchSyncReportDto` is the only global sync path; `sync_skill_to_tool` is gone (command, registration, and core wrapper `sync_skill_to_tool_with_records`). Single-pair callers (the toggle) pass a batch of one. `unsync_skill_from_tool` untouched.

**2. Core seam placement.** The engine lives in `core/global_sync.rs`, split on the repo’s established probe/deterministic seam:
- `plan_batch_tool_targets(tool_keys)` — environment probing (adapter lookup, installedness, resolved root, installed shared-dir group per tool).
- `sync_skills_to_planned_tools(store, skills, targets, policy, now, on_progress)` — deterministic engine, driven by tests with fabricated roots: dedupe installed targets by root (caller order, first wins; deduped tools covered by record fan-out), emit `Skipped` per skill for not-installed tools, per-pair policy resolution, per-target failure isolation.
- `sync_skills_to_tools` composes the two; planning failures become `Failed` outcomes — the function itself never errors.

**3. Policy vocabulary (Q2).** `BatchPolicy { overwrite, overwrite_if_same_content, overrides: Vec<BatchOverride{skill_id, tool_key, overwrite}> }`. An override applies to any target tool sharing the named tool’s skills dir (dir, not key, is a target’s identity) — this is how the import flow’s per-tool “overwrite the chosen variant’s own copy” survives backend dedupe. Skip-not-installed/not-writable is always-on, not policy.

**4. Result shape (Q3, refined).** Per-target results are data; the command only `Err`s on infrastructure. `SyncTargetStatusDto` = `synced{mode_used} | skipped{error} | failed{error}` — **skips carry the full typed `CommandError`** (refinement over reason-only: not-writable display needs tool+path, and nothing is lost). `skipped` = expected-and-ignorable (`TOOL_NOT_INSTALLED`, `TOOL_NOT_WRITABLE`); `failed` = everything else, so the import flow’s `TARGET_EXISTS` special copy still works.

**5. Progress (Q4).** Tauri v2 `ipc::Channel<SyncProgressDto{index,total,skill_name,tool}>` passed as a command arg; core takes a plain closure — the Channel exists only in the command wrapper. Frontend feeds `setActionMessage` from it (uniform `actions.syncStep`; no new i18n keys).

**6. Shared-dir grouping to backend (Q5, scan finding 7).** `ToolInfoDto.shared_with: string[]` (full group incl. self, adapter order; project-status entries are self-only groups since AgentsStandard absorption already collapses them). Frontend `uniqueToolIdsBySkillsDir` **deleted**; `sharedToolIdsByToolId` (SharedDirModal/checkbox UX) now reads `shared_with` instead of re-deriving from `skills_dir` strings.

**7. Call sites (Q6).** All 9 loops collapsed now onto two helpers in App.tsx — `syncSkillsToTools` (channel + invoke) and `syncFailureEntries(report, {includeNotWritableSkips?})`. The old three-way error-behaviour divergence is now an explicit presentation choice over shared data: bulk flows (sync-all, refresh, sync-all-managed) ignore skips; install flows surface not-writable skips (`includeNotWritableSkips: true`); the single toggle surfaces every non-success. Ticket 05’s `useSyncOrchestration` shrinks to extracting these two helpers.

**8. Typing (Q7).** All new DTOs ship via ts-rs bindings (no new hand-mirrors); `ToolInfoDto`/`ToolStatusDto` migrated off the hand-mirror to re-export shims in `types.ts`. Ticket 07’s remaining count drops by two DTOs.

**9. Tests (Q8).** 6 new Rust tests on the deterministic engine: N×M fan-out + progress ticks, shared-root dedupe with record fan-out, typed not-installed skip, per-target failure isolation, override shared-dir expansion (incl. negative case), override skill-scoping.

**Preserved oddity (flagged, not fixed):** the refresh-all flow syncs with *no* overwrite flags, so previously-synced targets whose content just changed hit `TARGET_EXISTS` and are collected as errors — exactly as before the refactor. Likely wants `overwrite: true` or `overwrite_if_same_content`; left for a deliberate decision (candidate for ticket 05 or a follow-up).

**Behaviour deltas (accepted, minor):** batch failures now surface via one aggregated `showActionErrors` toast instead of per-target toasts in sync-to-all; `handleSyncAllManagedToTools` no longer early-returns silently when all passed tools are uninstalled (report comes back all-skipped, success toast shows).

**AGENTS.md** gained the backend-owned-fan-out invariant (Ambiguity resolution). No ADR: the decision is recorded here, reversal is cheap (one seam), and ADR 0001 already covers the error-contract half.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
