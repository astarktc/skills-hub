## Standards

**S1 — Blocking: the new manual-recovery condition bypasses the typed/localized error contract.** `src-tauri/src/core/install_finalize.rs:347–359` constructs operator-facing recovery instructions as an `anyhow` string: `"rollback to {:?} failed: {:#}; old bytes retained at {:?} for manual recovery"`. A permission-denied rollback follows this path (covered by `rollback_rename_failure_names_and_preserves_backup`). It has no `SignalError` variant; `commands/error.rs:164–225` turns the chain into `OTHER`, and `src/commandError.ts:172–173` returns that English message verbatim, including for Chinese users.

AGENTS.md, **Error wire contract**, requires “a condition the operator or upstream can cause is a typed variant” and “the backend composes no user-facing prose in any language.” ADR-0001 likewise places copy in the frontend. Keep the original failure as diagnostics, but represent failed recovery and the retained backup path structurally, with EN/ZH recovery copy. D1’s requirement to name the backup does not require English-only backend instructions. Verified at HEAD; introduced by **fix(finalize): make finalize_update failure-atomic via rename-aside**, not inherited.

No additional actionable Fowler smells or documented-standard breaches found.

## Spec

**P1 — Blocking: Re-point reuses an unrelated source’s subpath as authoritative branch evidence.** `src-tauri/src/core/installer.rs:325` unconditionally passes `stored_subpath: record.source_subpath.as_deref()`, including when `source_override` supplies a different URL. `git_acquisition.rs:288–293` then uses `strip_suffix(...)` and returns before consulting matching refs.

Concrete HEAD trace: an existing skill has `source_subpath = "skills/foo"`; the operator re-points it to `https://github.com/new/repo/tree/main/nested/skills/foo`, where the actual branch is `main` and the directory is `nested/skills/foo`. The resolver instead selects branch `main/nested`, directory `skills/foo`, and makes no discovery call. The SHA lookup consequently targets a nonexistent branch and a valid Re-point fails with GitHub-not-found instead of acquiring the selected directory.

D4 requires this to work “uniformly to Add, Refresh, Re-point”; ticket 03 describes the hint as a **cache** of a stored resolution. The old URL’s selection is not a cached resolution of the replacement URL. Preserve the shortcut for the source it describes; otherwise resolve the new URL without that hint. Add a Re-point regression with a deeper destination path sharing the old suffix and assert requested branch/path, not merely returned metadata. Verified at HEAD; introduced by **fix(git): resolve GitHub branches containing slashes**.

**P2 — Follow-up, inherited and explicitly outside this fix:** `install_finalize.rs:220,241` still performs `staged.move_into(&central_path)?` before `store.upsert_skill(&record)?` without new-install cleanup. A failed upsert leaves untracked bytes and blocks retry by name collision. Ticket 01 says “report rather than silently widen scope”; the implementer correctly reported this. It needs a separate cleanup change, not the update swap.

## Summary

- **Blocking: 2** — Standards S1 (typed/localized recovery condition); Spec P1 (wrong branch/path on Re-point).
- **Follow-up: 1** — Spec P2 (inherited new-install cleanup gap).
- Fresh confirmation at `6b0ec7d`: `cargo test --manifest-path src-tauri/Cargo.toml --all core:: -- --test-threads=4` — **535 passed**; targeted library/reporter/presentation tests — **110 passed**; `git diff --check 95b7893...HEAD` — clean. The existing tests do not cover P1’s stale-suffix case; its verification is a source-path trace.
- Reviewed the source diff for unintended reverts; none found beyond the behavior identified above. D1 rollback/cleanup ordering, D3’s shared hash consumers, and the D5–D9 frontend changes otherwise follow the decisions. Tracked working tree remained clean; only this requested report was written.
