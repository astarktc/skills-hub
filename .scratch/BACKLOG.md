# BACKLOG — the only cross-effort live queue

Read this first every session, then the newest note in `.scratch/handoffs/`. An item leaves this file in the
commit that closes it (or that opens the effort/ticket which absorbs it — say which). Every item keeps its
source pointer. Procedure and status vocabulary: `docs/agents/issue-tracker.md` § Lifecycle.

Numbers are stable: never renumber; retire by deleting the line (history keeps it). Next free number: **#35**.

## Now — evidence of unfinished work is strong

- **#02 Wave C — one report representation across the wire + ADR-0001 amendment.** Round-10 decision Q8 deferred #7
  to wave C; the four report DTOs are still separate (`commands/mod.rs` BatchSyncReportDto / RemovalReportDto /
  RefreshReportDto / ImportReportDto). Source: `archive/round10/decisions.md` Q8; research `archive/round9/panel/opus.md:84–109`.

## Later — real, not urgent

- **#12 Replace the old `bulk_assign_*` hand-simulated tests with engine calls.** `tests/project_sync.rs:697,763,816`
  coexist with the engine suite at :1025. Source: `archive/v-next/issues/25:41`.
- **#15 Byte acquisition must refuse a listing-only intent.** `git_acquisition` still accepts an optional-subpath
  `Selection`; current callers cannot reach it — revisit before a new caller does. Source: `archive/round10/wave-b-review-disposition.md:7`.
- **#16 Old nonempty Explore preview cache: validate or migrate.** New publication is atomic, but inherited entries are
  trusted by `preview_has_content`/`read_dir` (`installer.rs:502,528,542`). Source: `archive/round10/wave-b-execution.md:40`.
- **#17 Permissioned Cursor smoke: hidden-target directory symlink + full IDE restart.** Ticket 38 flipped the capability
  on unit gates only. Needs operator permission (touches the live library). Source: `archive/v-next/assets/research-cursor-symlinks.md:224–228`.
- **#18 Windows: Cursor junction fallback is unverified.** `sync_engine.rs:63` path never exercised on a Windows host.
  Source: same research, :231–233; `archive/v-next/issues/38:33–34`.



## Parked — needs a product decision before it is work

- **#21 Notification history persistence** (DB table or JSONL). Session-only ring was deliberate. Source: `archive/round3/spec.md:28`.
- **#22 Refuse Tool-dir local sources at listing time** (UX; second predicate call site). Source: `archive/round4/issues/08:31`.
- **#23 Bulk local Re-point** of many stale rows at once. Source: `archive/round4/spec.md:191–192`.
- **#24 WSL ↔ Windows path translation.** Explicitly excluded in round 4. Source: `archive/round4/spec.md:193`.
- **#25 GitHub API path follows only a leaf symlink** (ancestor symlinks need the clone path). Limitation accepted;
  CONTEXT.md **Acquisition** records it. Source: `archive/round4/issues/12:10`, `archive/round5/spec.md:39`.
- **#26 Skill Edit V2 / Fork** (keep upstream, layer operator edits, replay on Update, flag conflicts). Edit V1 is the
  foundation; Manifest module is the write door. Source: `archive/round7/issues/02:19`, `archive/round10/issues/04:35`.
- **#27 Collapse the three unsync commands into one scoped removal command.** Distinct toasts are the recorded reason to
  keep them. Source: `archive/arch-deepening/issues/03:90–91`.
- **#28 Decouple `update_managed_skill` from the batch command's signature**; rename the `refreshProgress` channel factory.
  Source: `archive/round10/wave-b-review-disposition.md:8–9`.
- **#29 Document `content_identity::record`'s upsert effect** (or rename). Doc comment at :25 may already suffice.
  Source: `archive/round7/backlog.md` #19.
- **#30 v-next feature ideas beyond the invocation badge** — operator input needed. Source: `archive/v-next/map.md:68`.

## Dropped by name (recorded so nobody requeues them)

- Modal/route-state ownership module (round-9 panel #9) — dropped, round-10 decisions Q10 ("backlog note only").
- Virtual-group capability as AND of constituents — rejected, v-next ticket 36 (capability is the group's own registry fact).
- `NameIntent::FolderDerived` variant — rejected, v-next ticket 34:66 (variants express naming policy).
- Backend defaults over the wire — rejected, v-next ticket 34:72 (pre-load placeholders still needed).
- ts-rs type generation — superseded by tauri-specta, v-next map:54.
- Double single-action toast, fence rules, override convergence — fixed (round 9/10), not open.
- Operator smoke of a release — not a queue item: the operator installs each GitHub release build and smokes that.
- Pin "unknown status + copy + matching hash → Synced" as an integration test — declined, v-next 35:41; stored-string tests exist.
