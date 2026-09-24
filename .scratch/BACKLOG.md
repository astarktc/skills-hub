# BACKLOG — the only cross-effort live queue

Read this first every session, then the newest note in `.scratch/handoffs/`. An item leaves this file in the
commit that closes it (or that opens the effort/ticket which absorbs it — say which). Every item keeps its
source pointer; re-verify it against the code before scheduling. Procedure and status vocabulary:
`docs/agents/issue-tracker.md` § Lifecycle.

Numbers are stable: never renumber; retire by deleting the line (history keeps it). Next free number: **#61**.

## Now — evidence of unfinished work is strong

- **#43 Augment global skills dir is `.augment/skills`, not `.augment/rules`** (`tool_adapters/mod.rs:292-295`; upstream fixed it
  in v0.5.0 `00c41cc`) — why Review & Import never saw `~/.augment/skills` during the QM-192 sweep. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §A1.
- **#44 GitHub token is a plaintext settings row** (`core/settings.rs:32,112`) → OS keychain behind a `CredentialStore` trait with a
  memory adapter for tests; one-time migration of the row. Upstream's `core/github_token.rs` + `device_sync/credentials.rs:14-18`
  is a clean port. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §A2.

(Earlier dispositions: every pre-round-16 `Later`/`Parked` line was absorbed by `round16/spec.md` § Dispositions — items #12 #15 #22
#28 #35 #36 → tickets; #18 → ticket 08; #16 #17 #21 #23 #24 #25 #27 #29 dropped by name there; #30 superseded by #37–#41; #26 moved
to Future efforts. #42 (overwrite ask on `TARGET_EXISTS`) was absorbed by `round17/spec.md` D3 / `round17/issues/03`. Items #43–#60
were seeded 2026-09-24 from the upstream/field survey in `docs/design-inputs/` — each carries its § pointer; re-verify against the
code before scheduling.)

## Later — real, not urgent

- **#45 Four adapter rows to copy verbatim** — `deepseek_harness`, `zcode`, `codewhale`, `workbuddy` — plus three path changes to
  *verify first* (Antigravity `.gemini/config/skills`, Kimi `.kimi-code/skills`, Cline `.agents/skills`); keep our Amp detect. Absorbs
  into #40 if that effort opens first. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §A3–A4.
- **#46 `SKILL.md`-gated onboarding scan** — `scan_tool_dir` (`tool_adapters/mod.rs:888-926`) lists every subdirectory; the local
  pickers already apply the rule (1.2.13). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B1.
- **#47 Search in `GitPickModal` / `LocalPickModal`**, installing only the checked candidates that match the filter. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B2.
- **#48 Network-boundary lint** — a build-failing script that finds every `reqwest::Client` / `git2::FetchOptions` / `Repository::clone`
  outside one module (we construct clients in several). The proxy feature it guards upstream is not wanted. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B4.
- **#49 Aggregate issues banner** "N skills have sync issues → View issues" + an `Issues` chip in `FilterBar` — one derived count over
  data we already hold; seed of the doctor view. Absorbs into #37 if that effort opens first. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §E1.
- **#50 `PreparedDirReplacement` mechanics** behind `finalize_update`'s backup and Propagation's copy re-materialisation — port the
  staging/backup/rollback, drop its read-time compare-and-swap hash (ADR-0005). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B6.

## Parked — needs a product decision before it is work

- **#51 Tags** — clean backend port (2 tables / 9 store methods / 7 commands) but answers "how do I find it", which the repo grouping
  (1.2.2) and wildcard search (1.1.9) already answer at ~80 skills; #52 answers "what is active". Decide at ~150 skills. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B5.
- **#59 Subtree-SHA update anchor** (record the skill subdirectory's tree SHA, not the branch commit, so monorepo commits elsewhere
  never trigger a refresh or an Edit-conflict check) — changes what "upstream changed" means for every git row; decide with #26.
  Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C9.
- **#60 Smaller peer-derived ideas, one line so they stay findable**: provenance backfill for imported skills from `npx skills`
  lockfiles (§C10); `metadata.targets` frontmatter as per-harness routing (§C11); per-tool "hold at previous central copy" pin (§C12);
  web mode from the CLI binary after #54 (§E10). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md`.

## Future efforts — decided as "yes, someday", each needs its own grill/spec before it is work

- **#26 Skill Edit V2 / Fork** (keep upstream, layer operator edits, replay on Update, flag conflicts). Edit V1 is
  the foundation; Manifest module is the write door. Source: `archive/round7/issues/02:19`, `archive/round10/issues/04:35`.
  Design references (2026-09-24): Skillfile's `pin`/`diff`/`resolve` patch overlay — still the only structured one in the field;
  SkillDock's per-hunk diff / revert / push-back as the owned-skill upstream-PR path; upstream's `device_sync/merge.rs` +
  `text_merge.rs` as a pure three-way planner. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C13–C14.
- **#37 Overall UI audit** — usability, user-friendliness, design quality across the app (the `impeccable` skill
  is the natural vehicle). Source: operator, 2026-09-22. Borrow list from upstream v0.8–v0.10 (list view, "Synced 3/7", sidebar,
  sticky install summary, detail health dot, a `UI-DESIGN-GUIDELINES.md` of our own): `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §E.
- **#38 Add skills via `npx skill add …`-style commands**, not only GitHub repo URLs. Source: operator, 2026-09-22.
- **#39 Repo-level skill deployment from the My Skills page.** Source: operator, 2026-09-22.
- **#40 Harness-specific detection/deployment audit** — what does and doesn't exist per Tool (e.g. `openai.yaml`
  deployment for Codex). Source: operator, 2026-09-22. Detection half is #56; adapter rows are #45. Upstream still has the
  skills-only-footprint bug we fixed in 1.2.17 — keep `is_installed_in` when copying rows (`docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §A4).
- **#41 Repo scan finds both a Codex and a Claude Code variant of one skill** — keep both, collapse, or
  something else; needs analysis, likely absorbs part of #40. Source: operator, 2026-09-22.
- **#52 Skill enable/disable that keeps the target map and project assignments** (a `disabled` transition, rows kept, Propagation
  reports `Skipped`) **+ multi-select bulk actions on My Skills** — one effort; the reversible un-deploy QM-147 needs. Source:
  `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C1–C2.
- **#53 Scheduled `Refresh (all)`** via the OS scheduler — port upstream's `system_scheduler.rs` builders; needs a headless entry and
  a cross-process lock (Mutation guard is process-wide only). Not upstream's `auto_update.rs`. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B3, §E6.
- **#54 `skillshub` CLI (or MCP) on the Rust core, sharing the DB** — sessions, the session-end audit and a Mac over SSH act without
  the GUI. Grill first: changes what the app is. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C3.
- **#55 Declarative library manifest — sync the roster, not the bytes** (sources, tool selection, project assignments, Edits;
  versioned in `~/Projects/agent-skills`; the doctor view's diff target). The two-Mac gap upstream's device sync does not fill.
  Grill first. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C4.
- **#56 Custom tool rows + generic `~/.*/skills` sweep + vendored `vercel-labs/skills` `agents.ts` behind a verified tier** — one
  detection effort (`ToolKey = Builtin \| Custom`, the Tools page showing the matched detect dir); absorbs the detection half of #40.
  Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C5–C7, §D3.
- **#57 Presets / kits as desired state with drift** ("BDA kit" applied to a new project or Mac). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C8.
- **#58 Doctor / usage view** — resident-vs-body token cost per skill/harness with a residency ladder (asm's design), effective skill
  set incl. plugin-provided skills (kitter's `effective_skills.rs`), drskill's fired-in-30-days data. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §D1–D2.

## Dropped by name (recorded so nobody requeues them)

- Upstream v0.10 multi-device sync as built — syncs bytes + tags, not targets/projects/Edits; would shadow `agent-skills`; bypasses
  finalize; open data-loss PRs #157/#158. #55 is the answer. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §F.
- Upstream `tool_distribution.rs` (a copy-refresh helper, not project sync — Propagation covers it), `skill_issues.rs` (substring
  classification of error prose), `auto_update.rs` run loop (`AppHandle` + settings-KV progress), recycle bin as built (deletes the
  row inside the archive — ADR-0002), GitHub proxy, discovery-scan settings as a second list, install-scope choice at Add, CC plugin
  discovery in onboarding, tool-hiding, avatars, tray, Korean, the v0.8 CSS. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §F.
- Upstream *patterns*: `CODE|…` string errors in any form, settings table as per-row state, read-time re-hashing, `AppHandle` in
  core, a second tool catalog in commands, feature modules importing other features' internals. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §F.

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
