# BACKLOG — the only cross-effort live queue

Read this first every session, then the newest note in `.scratch/handoffs/`. An item leaves this file in the
commit that closes it (or that opens the effort/ticket which absorbs it — say which). Every item keeps its
source pointer; re-verify it against the code before scheduling. Procedure and status vocabulary:
`docs/agents/issue-tracker.md` § Lifecycle.

Numbers are stable: never renumber; retire by deleting the line (history keeps it). Next free number: **#68**.

## Now — evidence of unfinished work is strong

(empty — #39 #46 #47 #49 #52 and the §E borrows were absorbed by `round19/spec.md` on 2026-09-27: #52 → tickets 01 + 13, #49 → 11, #39 → 11, #47 → 16, #46 → 05 + 21. Earlier: #43 #44 → `round18/issues/01`, `02`.)

(Earlier dispositions: #43 #44 → `round18/issues/01`, `02`. #37 opened as the round 18 → 19 UI effort (`round18/spec.md`, `round19/spec.md`). every pre-round-16 `Later`/`Parked` line was absorbed by `round16/spec.md` § Dispositions — items #12 #15 #22
#28 #35 #36 → tickets; #18 → ticket 08; #16 #17 #21 #23 #24 #25 #27 #29 dropped by name there; #30 superseded by #37–#41; #26 moved
to Future efforts. #42 (overwrite ask on `TARGET_EXISTS`) was absorbed by `round17/spec.md` D3 / `round17/issues/03`. Items #43–#60
were seeded 2026-09-24 from the upstream/field survey in `docs/design-inputs/` — each carries its § pointer; re-verify against the
code before scheduling.)

## Later — real, not urgent

- **#67 Codex global skills dir may have moved to `~/.agents/skills`** — operator report (via another agent, 2026-09-30): modern Codex reads `~/.agents/skills` and treats `~/.codex/skills` as legacy. Today the `codex` adapter row has `relative_skills_dir: ".codex/skills"`, `former_relative_skills_dirs: &[]`, detect `.codex` (`core/tool_adapters/mod.rs` ~:244), and the README says Codex still reads `~/.codex/skills`. **Verify against the Codex CLI docs/changelog first** (which version, whether both are read, precedence). If confirmed: the `agents_skills` virtual-group row already manages `~/.agents/skills` globally, so either Codex joins the group's global membership (global `shared_with` dedupe + the README's group sentence) or its row moves to `.agents/skills` with `.codex/skills` in `former_relative_skills_dirs` so Target relocation migrates existing targets; README row + `project_relative_skills_dir_for_every_tool` table test in the same change. Related: #45 (adapter path changes to verify), #56 (detection).
- **#66 One multi-skill delete command (round 19 ticket 13)** — `delete_managed_skill` takes one skill; the bulk rail's Remove loops it inside one `runAction` (failures collected, one `RemovalReport` fold — the rail's report dialog counts "N of M not deleted"). Add `delete_managed_skills(ids)` under one `serialized` section answering one report so a whole-command failure mid-batch still carries the settled rows (same class as #64 (1)). Source: ticket 13 Result "Delete batch shape".
- **#65 Inspector metadata the wire does not carry (round 19 ticket 12, audit I9)** — three D15/D18/D22 clauses shipped honestly degraded because `ManagedSkillDto` / `SkillTargetDto` lack the fields: (1) a **typed settled error per target row** (`last_error` classified as `CommandError` where it settles, exported through specta, folded once by `skillHealth`) so a target row can read "permission denied" / "Overwrite and retry" instead of state words — today the inspector's Retry raises the ask from its fresh report and the row shows product-state copy only; (2) the **recorded git ref/SHA** on the skill (the store has it; the DTO does not) for the Upstream section's revision line (also #62's input); (3) a **detached-origin path** kept on Detach so the provenance line can read "Detached from ~/…" (today Detach clears source metadata and the line reads "No upstream — Refresh skips it"). Never hand-mirror these in TS; each is derive + `cargo test` + shim re-export. Source: `round19/review/ticket-12-audit.md` § I9; ticket 12 Result "BACKLOG follow-ups".
- **#64 Four accepted command-API compromises from round 19 ticket 09** — each ships in 1.3.0 under one outer lifecycle action with one ask/fold/run, but falls short of one-command-per-operator-action: (1) project assign/unassign over several skills is one `bulk_assign_skill` / `bulk_unassign_skill` per skill → a multi-skill command that still carries settled partial reports when a later skill fails whole-command; (2) Deploy all is Enable then first sync, two commands over disjoint partitions (no atomic cross-partition promise) → a combined backend surface; (3) a successfully answered project collision row is dropped from the Update/Edit report (`resolve_project_collisions` returns no sync mode; folds count only failures, views are re-read) → a lossless typed settled-target representation before any consumer treats the array as a complete audit; (4) the Enable retry re-runs the whole scope with overrides (it reselects only still-disabled rows) → a pair-exact answered-pair Enable API, keeping the tests that unrelated completed pairs stay untouched. Source: `round19/review/ticket-09-review.md` § Re-review ("Four command-API compromises").
- **#63 Tighten the secure-deletion zeroing test** — `core/tests/settings.rs` (~:768) asserts live bytes are zeroed but also passes without `secure_delete` (VACUUM alone hides it); use a page-spanning sentinel and a sensitivity check so `secure_delete` is pinned on its own. Source: Astra final check of `4f6e312`, `.scratch/round18/review/ticket-02-review.md` § Final check.
- **#45 Four adapter rows to copy verbatim** — `deepseek_harness`, `zcode`, `codewhale`, `workbuddy` — plus three path changes to
  *verify first* (Antigravity `.gemini/config/skills`, Kimi `.kimi-code/skills`, Cline `.agents/skills`); keep our Amp detect. Absorbs
  into #40 if that effort opens first. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §A3–A4.
- **#48 Network-boundary lint** — a build-failing script that finds every `reqwest::Client` / `git2::FetchOptions` / `Repository::clone`
  outside one module (we construct clients in several). The proxy feature it guards upstream is not wanted. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B4.
- **#62 Passive upstream update check** — a read-only "check upstream" that compares the recorded SHA with the remote (Contents API /
  `ls-remote`) and surfaces `Update available` on the row and in the inspector without acquiring or applying; Refresh (all) stays the
  apply step. Decide the anchor with #59 (branch commit vs subtree SHA) before building. Source: round 18 bake-off, both prototypes
  rendered a state the backend cannot produce (`.scratch/round18/spec.md` § Bake-off).
- **#50 `PreparedDirReplacement` mechanics** behind `finalize_update`'s backup and Propagation's copy re-materialisation — port the
  staging/backup/rollback, drop its read-time compare-and-swap hash (ADR-0005). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B6.

## Parked — needs a product decision before it is work

- **#51 Tags** — clean backend port (2 tables / 9 store methods / 7 commands) but answers "how do I find it", which the repo grouping
  (1.2.2) and wildcard search (1.1.9) already answer at ~80 skills; #52 answers "what is active". Decide at ~150 skills. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B5.
- **#59 Subtree-SHA update anchor** (record the skill subdirectory's tree SHA, not the branch commit, so monorepo commits elsewhere
  never trigger a refresh or an Edit-conflict check) — changes what "upstream changed" means for every git row; decide with #26.
  Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C9.
- **#61 Broaden localisation** to the top ~5 languages via machine translation, once the copy pass (round 19 `clarify`) has settled EN.
  ZH was removed in round 18 (unverifiable upstream inheritance; `git show <sha>:src/i18n/resources.ts` recovers it). Source: operator,
  2026-09-24, `round18/issues/03`.
- **#60 Smaller peer-derived ideas, one line so they stay findable**: provenance backfill for imported skills from `npx skills`
  lockfiles (§C10); `metadata.targets` frontmatter as per-harness routing (§C11); per-tool "hold at previous central copy" pin (§C12);
  web mode from the CLI binary after #54 (§E10). Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md`.

## Future efforts — decided as "yes, someday", each needs its own grill/spec before it is work

- **#26 Skill Edit V2 / Fork** (keep upstream, layer operator edits, replay on Update, flag conflicts). Edit V1 is
  the foundation; Manifest module is the write door. Source: `archive/round7/issues/02:19`, `archive/round10/issues/04:35`.
  Design references (2026-09-24): Skillfile's `pin`/`diff`/`resolve` patch overlay — still the only structured one in the field;
  SkillDock's per-hunk diff / revert / push-back as the owned-skill upstream-PR path; upstream's `device_sync/merge.rs` +
  `text_merge.rs` as a pure three-way planner. `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §C13–C14.
- **#38 Add skills via `npx skill add …`-style commands**, not only GitHub repo URLs. Source: operator, 2026-09-22.
- **#40 Harness-specific detection/deployment audit** — what does and doesn't exist per Tool (e.g. `openai.yaml`
  deployment for Codex). Source: operator, 2026-09-22. Detection half is #56; adapter rows are #45. Upstream still has the
  skills-only-footprint bug we fixed in 1.2.17 — keep `is_installed_in` when copying rows (`docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §A4).
- **#41 Repo scan finds both a Codex and a Claude Code variant of one skill** — keep both, collapse, or
  something else; needs analysis, likely absorbs part of #40. Source: operator, 2026-09-22.
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
