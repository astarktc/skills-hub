# Implementer comments (claims to verify), per lane

## R1 — `fix(finalize): make finalize_update failure-atomic via rename-aside`
- Rename-aside preserves old bytes until upsert succeeds. Rollback failures retain the backup and name its recovery path; post-commit cleanup failures only log.
- Backups use collision-checked `.skills-hub-old-<UUID>` siblings.
- Four regression tests failed before the fix; all 15 finalize tests now pass.
- Read-only-parent testing blocks rename-aside itself. Missing staging additionally tests move failure after rename-aside.
- Upsert failure uses a real SQLite abort trigger; no fault hook.
- `finalize_install` can leave untracked bytes after upsert failure. It needs new-install cleanup rather than an old-directory swap; left unchanged.
- Managed listing/unlocatable checks use database records, not directory globbing. Ordinary root/recursive discovery skips hidden backups (tested). Explicit known-scan-base discovery does not universally exclude hidden children.

## R2 — `fix: exclude internal symlinks from skill content identity`
- Internal symlinks contribute neither names nor targets to hashes; copies explicitly skip them.
- `global_sync::target_has_same_content` already delegates to `hash_dir`; no separate change needed. Other hash consumers inherit the rule.
- Three regressions failed before the fix and passed afterward, including onboarding with auto-sync both on and off.

## R3 — `fix(git): resolve GitHub branches containing slashes`
- Parser unchanged. Acquisition uses stored suffix first, then longest matching branch; discovery failures retain the original split.
- `installer.rs` passes `record.source_subpath` into acquisition for Update/Refresh/Re-point and forwards resolved metadata to finalize.
- Add's pre-acquisition listing also uses the shared resolver.
- Explicit selections remain authoritative. No DTO or protected-file changes.

## F1 — six commits (`fix: guard stale skill repair notification actions`, `docs: clarify head-only batch toast actions`, `fix: keep git repoint modal open until success`, `refactor: own managed detail selection in skill library`, `refactor: centralize skill repoint door selection`, `fix: show library when selected detail disappears`)
- A: Notification action resolves captured ID against latest state via ref; missing skills produce an EN/ZH warning. Regression tested.
- B: Documented head-only toast action; behavior unchanged.
- C: `runSingleRefresh` returns `true` on completion through `runAction`; throws and `ActionExit` failures resolve to `undefined`. Only `true` closes the modal. Pending, success, and failure paths tested.
- D: Library owns managed detail selection; updates follow current rows and disappearance yields `null`. App falls back to the library. Explore preview/install handoff preserved separately.
- E: Tested `repointDoor` predicate shared by card visibility and dispatch.
