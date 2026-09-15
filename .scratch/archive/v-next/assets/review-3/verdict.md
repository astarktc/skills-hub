# Code review #3 — tickets 19–30 (`943f85c...cdbbd15`), orchestrator-verified verdict

Panel (blind, read-only, Pi harness, thinking high): Claude Fable 5.1 (`anthropic/claude-fable-5-1`), Claude Opus 5 (`cortex/claude-opus-5`), GPT-5.6 Sol (`cortex-responses/openai-gpt-5.6-sol`). Raw reports: [fable.md](fable.md), [opus.md](opus.md), [sol.md](sol.md).

Every claim below was checked against the code at `cdbbd15` and, where relevant, against `943f85c`. Verdicts: **regression** (behaviour worse than base), **pre-existing** (true at base too), **polish** (true, non-breaking), **rejected** (claim wrong or misclassified).

Cross-panel: 3/3 reviewers agreed on the T28 intent-drop regression and the `cache_cleanup` re-export; 2/3 on `now_ms` duplication + finalize↔installer import cycle, both AGENTS.md doc-drift items, the gitignore data clump, `get_settings` side effect. Zero fabricated citations — every file:line resolved. Two Sol "hard" claims were **downgraded** after verification (see Spec / T23, T27).

## Spec axis

| # | Ticket | Claim | Verdict | Ticket |
|---|---|---|---|---|
| S1 | 28 | `useProjectState.configureTools` clears `pendingIgnore` **before** awaiting `configureProjectTools`; T28 now keeps the modal open on failure, so a retry runs with `gitignore: null` and the ignore block is never written (`useProjectState.ts:355-360`, `ProjectsPage.tsx:45-51`). Base also cleared early but closed the modal + `toast.warning`, so no misleading retry existed. | **regression** (minor, user-visible) — 3/3 | 31 |
| S2 | 21 | `handleImport` awaits `fetchPlan()` inside the `runAction` body; a plan-reload failure after a successful import now suppresses `status.importCompleted` and leaves the import modal open (`useAddSkillFlow.ts:384-390`). Base `loadPlan` swallowed (`943f85c` `:153-178`) and closed the modal. Answer disclosed only the error toast. | **regression** (edge, low) — Fable | 32 |
| S3 | 23 | Cursor-only install at project scope presents `agents_skills`, whose adapter has `supports_symlink: true` → symlinked (`tool_adapters/mod.rs:180-198`, `catalog.rs:55-78`, `project_sync.rs:40-61`). | Mechanics **confirmed**, but base keyed project sync by `"agents_skills"` too (never `"cursor"`) so base symlinked as well. **Pre-existing design gap, not a T23 fidelity failure.** Sol's "hard violation" downgraded. | 36 |
| S4 | 27 | Unknown stored status → `Error` at the seam, but `listProjectSkillAssignments` reconcile recomputes copy-mode rows and writes `Synced` on hash match, clearing `last_error` (`sync_status.rs:149-169`, `project_sync.rs:394-419`). | Mechanics **confirmed**; base's copy branch also ran "for ANY current DB status" and rewrote to synced/stale. The new write is filesystem-grounded (hash match), not blind coercion. **Not a bug; Answer wording "never rewrites on read" is imprecise** (store seam vs list-reconcile). Sol's "hard violation" downgraded to doc polish. | 35 |
| S5 | 22 | `StagingDir::move_into` copy fallback: mid-copy failure leaves partial `dest` in the central repo; `Drop` only removes staging (`install_finalize.rs:111-124`). Cross-device only. | **polish** — Fable | 34 |
| S6 | 22 | Local flow passes a folder-derived name as `NameIntent::UserProvided` (`installer.rs:63-69, 88-96`), bypassing the SKILL.md preference the enum documents. Behaviour preserved vs base. | **polish** (interface honesty) — Opus | 34 |
| S7 | 26 | Folder-URL git listing widened: base ran only `collect_skill_dirs(&dir)`; HEAD `git_candidates_in` runs full `discover_skills` (marketplace + walk) in the folder (`installer.rs:609-636`). Not in the Answer's drift list. | **polish** (undisclosed, likely desirable) — Opus | 35 |
| S8 | 26 | Root admitted via `is_claude_skill_dir(root)` without `SKILL.md` (`skill_discovery.rs:118`); base required `SKILL.md` at root. Lists valid → install rejects `SKILL_INVALID`. | **polish** (cosmetic, undisclosed) — Fable | 35 |
| S9 | 29 | `match_skill_candidate` bidirectional containment: an empty candidate name matches every target (`skill_matching.rs:80-83`); table test lacks the blank-candidate case. | **polish** — Opus | 34 |
| S10 | 25 | Answer says "sweeps orphan `skill_targets`" but `skill_removal.rs:182-201` sweeps FS paths without deleting rows (same as base). | **polish** (wording) — Fable | 35 |
| S11 | 27 | `remove_project_with_cleanup` now also cleans `Error` rows (`has_deployed_artifact`). Disclosed in Answer. | **rejected** as scope creep — intentional, disclosed, correct. | — |
| S12 | 24 | `SkillStore::set_onboarding_completed` still calls raw `set_setting` (`skill_store.rs:356-362`). | **confirmed dead code** — zero production callers (tests only). polish | 34 |
| S13 | 30 | `specta-typescript = "0.0.12"` lacks the `=` AGENTS.md prescribes. | **polish** — `^0.0.12` is effectively exact for 0.0.x, but the doc says `=`. | 34 |

No findings on tickets 19, 20 (all three reviewers verified the acceptance criteria positively).

## Standards axis

| # | Claim | Verdict | Ticket |
|---|---|---|---|
| H1 | Prose error conditions in core cross the wire as `OTHER { message }` and reach users verbatim (`commandError.ts:110` returns `e.message`): new-in-batch `project_ops.rs:199 bail!("unknown tool")`, plus `project_ops.rs:112,237,333` / `project_sync.rs:41,250,252,276,462` "project/skill not found", "unknown tool", `gitignore.rs:226`, `project_ops.rs:90,318` "path is not a directory" — several sit next to typed `SignalError::NotFound`. ADR 0001 / English-primary. | **confirmed** (mostly pre-existing; `:199` is new). Standards hard. | 33 |
| H2 | `errors.skillNotFoundInRepo` present in `en`, absent in `zh` (`resources.ts:269`). AGENTS.md "both en and zh". | **confirmed, pre-existing** (1 occurrence at base too). | 33 |
| H3 | AGENTS.md:95 still says `invoke('get_managed_skills')`; seam is `invokeTauri("getManagedSkills")`. | **confirmed** doc drift | 35 |
| H4 | AGENTS.md "Core never reads the environment" heading vs 6 `std::env::var` reads in core (`git_fetcher.rs:34,274,282,296`, `sync_engine.rs:221`, `install_finalize.rs:253` — the last relocated by t22). Body scopes the rule to `home_dir`/`AppHandle`, which holds. | **confirmed** heading overclaims | 35 |
| H5 | `migrate_legacy_db_if_needed` calls `dirs::data_dir()` in core (`skill_store.rs:1165`). | **pre-existing** (base `:1087`); not listed among AGENTS.md's thin-adapter exceptions. polish | 34 |
| J1 | `cache_cleanup.rs:11-14` re-export `git_cache_ttl_secs as get_git_cache_ttl_secs` with a stale "parallel change" comment; `installer.rs:7,778,857` still route through it. | **confirmed** — 3/3 | 34 |
| J2 | Four `now_ms()` copies (`central_repo.rs:53` new, `installer.rs:292`, `cache_cleanup.rs:95`, `commands/mod.rs:728`); `install_finalize.rs:22` imports `installer::now_ms` back while installer imports finalize (cycle). | **confirmed** | 34 |
| J3 | `install_local` / `install_git` registered in `collect_commands!` with zero frontend call sites. | **confirmed** dead wire surface | 34 |
| J4 | Gitignore booleans cross the wire in two shapes (`update_project_gitignore` positional vs `configure_project_tools` DTO); camel→snake conversions at `ProjectsPage.tsx:26-29,91-94`. | **confirmed** data clump | 34 |
| J5 | `catalog.rs` project entry: `skills_dir` = global dir, `shared_with` grouped by project dir — mixed path families on one record (unread by FE). | **confirmed** | 34 |
| J6 | `has_deployed_artifact` (Synced\|Stale\|Error) vs `has_deployed_target` (Synced\|Stale\|Missing) in separate impl blocks; three cleanup gates differ with no comment. | **confirmed** naming/comment | 34 |
| J7 | `get_settings` getter calls `ensure_central_repo` (side effect), now invoked on mount by two hooks. Inherited from `get_central_repo_path`. | **confirmed**, pre-existing shape | 34 |
| J8 | `useSettingsState.adoptSettings` overwrites all six fields on every single-setting write; zoom hotkey writes without adopting. Race class pre-existing, blast radius grew. | **confirmed** | 34 |
| J9 | `useProjectState.test.ts:249,258` fixture rejects with code `"INTERNAL"`, not a real `CommandError` code. | **confirmed** | 33 |
| J10 | `useCandidatePick.install` not memoised (only member); defaults `1.0/30/60` retyped in `useSettingsState`; `.cloned()` out of static registry; six direct `toast.*` calls bypass the reporter; three copies of the cleanup-walker shape in `project_ops.rs`. | **confirmed**, minor | 34 |

## Summary

- **Regressions: 2**, both frontend edge-path lifecycle issues (S1 T28 intent drop — unanimous; S2 T21 import completion). One-line-class fixes with hook tests.
- **Standards hard: 2** (H1 prose errors incl. one new site; H2 zh key) + doc drift ×2.
- **Downgraded: 2** of Sol's headline "bugs" (S3, S4) are pre-existing/behaviour-preserving; S3 remains a real design question (virtual-group capability).
- **Core claims of all 12 Answers hold**; no Answer claims something the code does not do, except wording imprecision (S4, S7, S8, S10).
- Reviewers' own gates: lint, 89 vitest, cargo test 316, clippy `-D warnings`, tsc-7 — all green.
