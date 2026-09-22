# BACKLOG — the only cross-effort live queue

Read this first every session, then the newest note in `.scratch/handoffs/`. An item leaves this file in the
commit that closes it (or that opens the effort/ticket which absorbs it — say which). Every item keeps its
source pointer; re-verify it against the code before scheduling. Procedure and status vocabulary:
`docs/agents/issue-tracker.md` § Lifecycle.

Numbers are stable: never renumber; retire by deleting the line (history keeps it). Next free number: **#42**.

## Now — evidence of unfinished work is strong

(empty — every `Later`/`Parked` line was absorbed by `round16/spec.md` § Dispositions in its opening commit:
#12 #15 #22 #28 #35 #36 → tickets; #18 → ticket 08; #16 #17 #21 #23 #24 #25 #27 #29 dropped by name there;
#30 superseded by #37–#41 below; #26 moved to Future efforts.)

## Later — real, not urgent

(empty)

## Parked — needs a product decision before it is work

(empty)

## Future efforts — decided as "yes, someday", each needs its own grill/spec before it is work

- **#26 Skill Edit V2 / Fork** (keep upstream, layer operator edits, replay on Update, flag conflicts). Edit V1 is
  the foundation; Manifest module is the write door. Source: `archive/round7/issues/02:19`, `archive/round10/issues/04:35`.
- **#37 Overall UI audit** — usability, user-friendliness, design quality across the app (the `impeccable` skill
  is the natural vehicle). Source: operator, 2026-09-22.
- **#38 Add skills via `npx skill add …`-style commands**, not only GitHub repo URLs. Source: operator, 2026-09-22.
- **#39 Repo-level skill deployment from the My Skills page.** Source: operator, 2026-09-22.
- **#40 Harness-specific detection/deployment audit** — what does and doesn't exist per Tool (e.g. `openai.yaml`
  deployment for Codex). Source: operator, 2026-09-22.
- **#41 Repo scan finds both a Codex and a Claude Code variant of one skill** — keep both, collapse, or
  something else; needs analysis, likely absorbs part of #40. Source: operator, 2026-09-22.

## Dropped by name (recorded so nobody requeues them)

- Modal/route-state ownership module (round-9 panel #9) — dropped, round-10 decisions Q10 ("backlog note only").
- Virtual-group capability as AND of constituents — rejected, v-next ticket 36 (capability is the group's own registry fact).
- `NameIntent::FolderDerived` variant — rejected, v-next ticket 34:66 (variants express naming policy).
- Backend defaults over the wire — rejected, v-next ticket 34:72 (pre-load placeholders still needed).
- ts-rs type generation — superseded by tauri-specta, v-next map:54.
- Double single-action toast, fence rules, override convergence — fixed (round 9/10), not open.
- Operator smoke of a release — not a queue item: the operator installs each GitHub release build and smokes that.
- Pin "unknown status + copy + matching hash → Synced" as an integration test — declined, v-next 35:41; stored-string tests exist.
- Old nonempty Explore preview cache validation (#16) — startup wipes `.explore-cache` (`lib.rs:179`); round16 spec.
- Permissioned Cursor symlink smoke (#17) and Cursor-through-junction on Windows (#18 facet b) — no Cursor on any
  operator host; vendor docs; `supports_symlink` is the revert lever; round16 spec.
- Notification history persistence (#21) — per-row `last_error` is the durable record; round16 spec.
- Bulk local Re-point (#23) — never hit; its own effort if demand shows; round16 spec.
- WSL ↔ Windows path translation (#24), GitHub API ancestor-symlink (#25) — explicit exclusions, no new evidence; round16 spec.
- Collapse the three unsync commands (#27) — uniform since round 15; distinct toasts remain the reason; round16 spec.
- Decouple `update_managed_skill` from the batch signature (#28 half) — "a batch of one" is the documented design; round16 spec.
- Document `content_identity::record`'s upsert (#29) — doc + 2-line body already say it; round16 spec.
