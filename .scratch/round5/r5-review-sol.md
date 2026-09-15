## Standards

- **Hard — Git Re-point bypasses the documented Update policy** (`src-tauri/src/core/refresh.rs`, `src/hooks/useSkillLibrary.ts`). AGENTS.md says Update/Refresh uses `reassert_auto_sync`, and CONTEXT.md defines git Re-point as “change the source and Update.” The new path instead hard-codes `RefreshPolicy::default()` while ordinary Update sends `{ reassert_auto_sync: autoSyncEnabled }`. With auto-sync enabled, Re-point therefore does not restore missing installed-Tool targets.
- **Judgment — Duplicated Code** (`src-tauri/src/core/refresh.rs`, `src-tauri/src/core/git_acquisition.rs`). `parse_repoint_source` reimplements a second GitHub URL grammar (`parts[2] != "tree"`, character checks) and then calls `parse_github_url`. The two parsers already disagree on `/blob/`. Fix by giving the single parser an explicit strict-input mode/result rather than validating the same domain twice.

No other documented breach found; the three new `SignalError`s have CommandError mappings, generated union members, `describeCommandError` branches, and EN/ZH copy.

## Spec

- **Acquire-first is not failure-atomic** (`src-tauri/src/core/installer.rs`, `install_finalize.rs`). Ticket 01 requires: “On any failure … the record is untouched.” `finalize_update` executes `remove_dir_all(&central_path)`, then `staged.move_into`, then `store.upsert_skill`. A move/copy or SQLite upsert failure can return a failed Re-point after deleting/replacing the working central copy while the database still names the old source. This is a data-safety blocker.
- **A repo URL can select the wrong skill** (`git_acquisition.rs`). Ticket 01 says a repo URL resolves by the existing skill name and ambiguity leaves the record unchanged. The new fallback says `if let [only] = candidates … if root.is_some() { None }`: in a repo with a root skill plus one nested skill, a name matching neither silently chooses the root and can overwrite the Managed skill. Conversely, root plus two nested skills cannot match the root at all.
- **Valid GitHub URLs are refused/misparsed** (`refresh.rs`). Q12 says input is “parsed by the existing GitHub URL parser”; `parse_repoint_source` rejects `/blob/<branch>/<path>/SKILL.md`, which `parse_github_url` accepts. A valid `/tree/feature/foo/path` URL is accepted but parsed as branch `feature`, subpath `foo/path`, so branches containing `/` fail acquisition.
- **Not the normal Update policy** (`refresh.rs`, `useSkillLibrary.ts`). Q12 says “This is Update with a source override”; the command accepts no policy and uses `RefreshPolicy::default()` (`reassert_auto_sync: false`).
- **Notification toast action is order-dependent** (`useStatusReporter.ts`). Ticket 07 requires the not-found row “(and its toast)” to carry Re-point, but batching calls `showToast(..., head.action)`. A GitHub-not-found entry after another failure gets a panel action but no toast action.
- **Release requirement remains open**: spec story 18 requires “v1.2.5 with a CHANGELOG entry”; at `378a473`, `package.json` is still `1.2.4`, and neither version files nor `CHANGELOG.md` changed.

## Summary

**Blocking:** make finalize/source change failure-atomic; fix repo-root/nested name resolution; honor auto-sync Update policy; support legitimate GitHub URL forms (including slash refs); ensure every actionable 404 is actionable from its toast; add CHANGELOG and bump all version locations to 1.2.5.

**Follow-up:** remove the duplicate Re-point URL grammar when fixing URL handling.
