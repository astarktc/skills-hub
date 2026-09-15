# 12: Round-3 review follow-ups (non-blocking)

**Status:** needs-triage
**Source:** ticket-09 panel (Fable 5.1 / Opus 5 / GPT-5.6 Sol) over `f400440..4d17493`, plus operator smoke test 2026-09-04. Each item verified at HEAD by the orchestrator. Items fixed in ticket 11 are omitted.

## From the smoke test
1. **Upstream in-repo symlinks break acquisition** — `tanstack-skills/tanstack-skills` turned `plugins/tanstack-all/skills/*` into symlinks; sparse checkout yields a dangling link, the Contents API returns `type: symlink`. → Resolve symlinks in the acquire path (add the target to the sparse set; follow API symlink targets). Pre-existing.
2. **Raw `OTHER` strings leak absolute paths** — `path not found in repo: "<cache-internal abs path>"`, `central path not found`, `source path not found`. → Typed `SignalError`s (`SourcePathMissing { path }`, `CentralPathMissing`, `SubpathMissing { subpath }`) per ADR-0001 with EN/ZH copy; paths as `detail`.
3. **Stale rows have no affordance** — a Managed skill whose central dir or local source is gone fails every Refresh forever. → A "source missing" state on the My Skills card with re-point / remove actions.
4. **`refresh::tests::acquisitions_overlap_instead_of_running_one_at_a_time`** (`refresh.rs:375`) is wall-clock-bound (< 60 % of sequential) and failed in 3/4 runs under six-way cargo load. → Assert overlap via ordering (a barrier / observed concurrency ≥ 2), not elapsed time.

## From the panel
5. **`visibleToasts`** — sonner default 3; with `error: Infinity` a 4th unclosed error stacks hidden until hover (`App.tsx:219`). → Set `visibleToasts` (5–6) or accept the stack since the panel lists everything. (Opus)
6. **`open_log_folder` failures are raw `OTHER`** (`commands/mod.rs:202-229`). → Typed `RevealLogFailed` + copy; or fold into #2. (Sol)
7. **`NotificationToaster` re-export** from a hook module (`useStatusReporter.ts:11`) — a Middle Man to satisfy the "one importer" grep. → Let the binder import `Toaster` directly and document it as the sanctioned second import. (Fable)
8. **Notification ring wants its own hook** — `useStatusReporter.ts` now has four reasons to change. → `useNotificationHistory` composed by the reporter. (Opus)
9. **`describeCommandError` × 9 in `ProjectsPage`/`AssignmentMatrix`** repeating the `if (msg)` dance `reporter.formatError` owns. → Pass `formatError` (or a `notifyError(err)` helper) alongside `notify`. (Opus)
10. **Duplicate lookups per assignment** — adapter resolved in `propagate_one_assignment` and again in `sync_assignment_target`; skill fetched in `sync_single_assignment` and possibly again in the helper; `observe_assignment` re-queries. → Pass the resolved adapter/skill in. (Fable, Opus)
11. **`propagation.rs:184-191` latent trap** — group from static `adapters_sharing_skills_dir` while rows resolve via `adapter_by_key` (test overrides); the row's own tool is no longer reflexively in its group. → Include `row.tool` unconditionally. (Opus)
12. **`resolve_project_sync_target`** is `pub` with one in-module caller (`project_sync.rs:24`). → `pub(crate)`/private. (Opus)
13. **Two subpath normalisers** — `git_cache.rs:124` and `git_fetcher.rs:249`. → One. (Fable)
14. **Two clipboard-copy shapes** — `SkillCard.tsx:64-70`, `NotificationsModal.tsx:79-88`; and `SkillCard` copy-success fills the history with "Copied". → One `copyToClipboard(notify)` helper; decide whether copy-success is a Notification at all. (Fable)
15. **`showActionErrors` records head first** so it sits at the bottom of its batch in the newest-first panel (`useStatusReporter.ts:280-282`). → Record in reverse. (Fable)
16. **`sync_assignment_target` 7 positional params** (`project_sync.rs:89`). → A small `AssignmentSyncContext`. (Opus)
17. **`log_reveal_target(log_dir, name)`** could be a pure core fn with a test (`commands/mod.rs:201-234`). (Fable)
18. **`.notif-badge { color: #ffffff }`** in a token-only stylesheet (`App.css:849`). (Opus)
19. **Q10 fallback doc** — `assignment_artifact_name`'s live-name fallback fires only for an empty (un-backfilled pre-V6) stored name; say so explicitly in the doc comment, or return `None`. (Sol; downgraded from blocking — see spec Comments)

## Rulings recorded
- **Q6 / recorded SHA within TTL** (Opus): within the git-cache TTL an install records the listing's snapshot rather than a fresh fetch. Ruled acceptable: the operator installs what they just browsed, the SHA is real, the bytes are that snapshot's; the previous behaviour only differed by fetching twice. Recorded in `spec.md` Q6.
