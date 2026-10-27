# BACKLOG — the only cross-effort live queue

Read this first every session, then the newest note in `.scratch/handoffs/`. An item leaves this file in the
commit that closes it (or that opens the effort/ticket which absorbs it — say which). Every item keeps its
source pointer; re-verify it against the code before scheduling. Procedure and status vocabulary:
`docs/agents/issue-tracker.md` § Lifecycle.

Numbers are stable: never renumber; retire by deleting the line (history keeps it). Next free number: **#81**.

## Now — evidence of unfinished work is strong

- **#79 A toast raised while a modal is open is inert to screen readers.** The one toaster is mounted inside `#root`
  (`src/App.tsx` ~:594); a modal layer makes `#root` inert (`layer-stack.ts::applyInert`), so a toast raised while a
  dialog or the inspector sheet stands open shows visibly but its polite live region is absent from the accessibility
  tree — nothing is announced. Pre-existing for every toast, made sharper by ticket 27: a held control's refusal
  inside the sheet ("Disable on Codex" while "Disable on Claude Code" runs) is seen, not heard (the reason stays
  available as the control's focus description, which is not an activation announcement). Fix once in the shared
  layer, never per control: a centrally owned non-interactive announcement host exempt from background inerting, or
  deliver the toast within the active modal boundary; keep one announcement, the shared refusal toast id, and the
  Notifications-panel suppression; test top and nested dialogs; operator VoiceOver smoke. **Operator decision
  2026-10-27 pending: fix before the 1.3.0 push, or ship with it as a known limitation.** Source:
  `round19/review/ticket-27-audit.md` § F2 (P1).
- **#71 "Not synced" vs Disabled, and Refresh's re-assert: grill first.** The operator can't tell the two apart. In
  the think-aloud he called them "kind of the same thing … redundant" and asked why both exist. With auto-sync on,
  "not synced" means "the next Refresh puts it here": `core/refresh.rs::reassert_auto_sync_unlocked` syncs every
  managed skill to every Tool in the effective selection. The screen never says so.
  - Observed in the session:
    - he read zod-schema-first on 1 of 8 Tools as a choice;
    - a Refresh deployed it and prisma-review to five more Tools each, raised an overwrite ask, and added 16 failed
      targets on an unwritable shared dir ("suddenly … a lot more skills have failed");
    - the re-assert spread a skill whose Delete was incomplete (release-train) to two more Tools;
    - failures read "Could not update the synced copy" for what was a first deploy.
  - Decide in a grill: show "Not synced" as "Next Refresh adds it"; or make Refresh ask before adding Tools; or drop
    "not synced" for selected Tools.
  - Either way: the run headline counts re-assert deploys as their own kind; re-assert failures get their own wording;
    the re-assert skips a skill with `delete_incomplete` health; and auto-sync's state is visible from My Skills.
  - Source: `round19/issues/18-operator-think-aloud.md` § Findings (P1·Q3, P2·Q4, P2·Q6, P2·Q10, debrief 4).
- **#70 The overwrite ask can't be answered knowingly.** Twice in the think-aloud the operator chose Overwrite without
  being able to judge: *"it gives me no way to see them whatsoever, and I can't even see the diff."*
  - The ask should name **who put the folder there**. A re-assert, Add or Enable collision has no target row, so it is
    never Skills Hub's.
  - It should offer a **content view or diff** against the skill, plus Reveal in Finder per row.
  - Each button should state its **consequence** ("Keep theirs — {skill} stays off {Tool}; Enable later to
    replace it"). He couldn't predict Keep.
  - The answered rows should be **recorded in the run report**: he had forgotten which folders he overwrote, and
    answered pairs settle as plain `synced` (same family as #64 (3)).
  - An ask that arrives after Cancel should say the skill was already being added.
  - Unchanged since 1.2.17, so it didn't hold 1.3.0. Source: `round19/issues/18-operator-think-aloud.md` § Findings (P2·Q4, P5·Q3, P5·Q4).

(Absorbed earlier: #39 #46 #47 #49 #52 and the §E borrows were absorbed by `round19/spec.md` on 2026-09-27: #52 → tickets 01 + 13, #49 → 11, #39 → 11, #47 → 16, #46 → 05 + 21. Earlier: #43 #44 → `round18/issues/01`, `02`.)

(Earlier dispositions: #43 #44 → `round18/issues/01`, `02`. #37 opened as the round 18 → 19 UI effort (`round18/spec.md`, `round19/spec.md`). every pre-round-16 `Later`/`Parked` line was absorbed by `round16/spec.md` § Dispositions — items #12 #15 #22
#28 #35 #36 → tickets; #18 → ticket 08; #16 #17 #21 #23 #24 #25 #27 #29 dropped by name there; #30 superseded by #37–#41; #26 moved
to Future efforts. #42 (overwrite ask on `TARGET_EXISTS`) was absorbed by `round17/spec.md` D3 / `round17/issues/03`. Items #43–#60
were seeded 2026-09-24 from the upstream/field survey in `docs/design-inputs/` — each carries its § pointer; re-verify against the
code before scheduling.)

## Later — real, not urgent

- **#80 The toast stack covers the header controls, and hover-pause prolongs it.** The toaster sits `top-right`
  over Configure Tools, Refresh all and Add skill; a refusal toast hides them for its 2 s, a trusted click on Refresh
  all landed on the toast, and a pointer resting where the button was pauses Sonner's timer so the toast stays until
  the pointer leaves. Reserve non-obstructing space for the stack (verify 960 and 1280 widths); don't blanket
  `pointer-events: none` (outcome toasts carry View report) and don't drop hover-pause globally. Compounds #79's
  shared-toaster work. Source: `round19/review/ticket-27-audit.md` § F3 (P2), ticket 27 Result.
- **#78 A restrained colour pass on the token world.** *"in both light and dark mode, it's very, very monochrome. The only
  splash of color is the little bit of purple … it looks better in dark mode than in light."* Run an `impeccable`
  colorize pass within ADR-0006 and DESIGN.md's measured contrast; the light theme is the weaker one. Source: `round19/issues/18-operator-think-aloud.md` § Findings (P7).
- **#76 The removal report's scope labels, and a permission failure's next step.**
  - Finish removing printed "Removed from … Claude Code" (a project row) next to "Still in Claude Code" (the global
    row): the "Removed from" summary doesn't name the project, while the "Still in" lines do ("· monorepo").
  - "Permission denied (os error 13)" comes with Retry and Reveal but no explanation of what to check (ownership,
    read-only, a process holding it) or that Retry will fail the same way.
  - Source: `round19/issues/18-operator-think-aloud.md` § Findings (P2·Q10, verified on the fixture).
- **#75 Undo on a project matrix cell's toggle-off.** One accidental click on a matrix cell un-assigns the skill (the
  project's copy is removed), with no confirmation and no Undo. The inspector's targets list has a 5 s Undo (D21);
  the matrix should match. Check whether ticket 17's larger cell hit area made misclicks likelier. Source: `round19/issues/18-operator-think-aloud.md` § Findings (P6·Q1).
- **#74 Notifications after a relaunch, and the vanishing run toast.**
  - Asked "tomorrow morning, how would you find today's failures?", the operator said he'd use the Notifications
    history. That history is session memory only, so the panel is empty after a relaunch. This reopens #21 (dropped by
    name in round 16) with operator evidence: persist the last N runs, or have the empty panel point at "View N skills
    with issues".
  - Separately, a Refresh run toast vanished before he could click View report, although failure-tone toasts are
    `Infinity` (`useStatusReporter` `TOAST_DURATION_MS`). Reproduce first (two Refresh alls; does a second run or
    opening the panel dismiss it?).
  - Source: `round19/issues/18-operator-think-aloud.md` § Findings (P2·Q4, P2·Q7).
- **#73 Bulk confirmation copy.**
  - The Enable sentence ("Enable 3 skills — 10 Sync targets on 7 Tools sync again, a first sync each",
    `resources.ts` ~:994) was unparseable on first read: *"I have no clue what the hell that thing is saying."*
  - The Disable sentence's zero case, "no project assignment changes" (~:985), was read as "a skill-grain Disable
    leaves project assignments live". It actually means none exist. Say "None of these skills is assigned to a
    project".
  - Source: `round19/issues/18-operator-think-aloud.md` § Findings (P2·Q8).
- **#72 My Skills reading fixes.**
  - The summary "7 failed targets · View 11 skills with issues" was read as 7 skills: two units side by side, and the
    first number taken.
  - The list is grouped by source, so 4 of 11 issue rows sat below the fold and were missed.
  - The strip's marks are positional and unlabelled. He mapped Cursor by matching the inspector's order; add a Tool
    legend or initials over the strip column.
  - "Stale" was read as "a newer version upstream". In fact it is a copy that drifted from Skills Hub's copy, and Sync
    this fixes it. Reword it on the strip and the matrix ("Out of date copy").
  - The inspector offers "Sync" on a Tool it labels "(not detected)" and fails (`TOOL_NOT_INSTALLED`). Refuse it in
    place with its reason.
  - A run headline's "1 Edit to review" item sits last in a run collapsed by default; order skill-level items first,
    or link the headline to them.
  - Source: `round19/issues/18-operator-think-aloud.md` § Findings (P1·Q1, P2·Q5, P2·Q6, P6·Q1).
- **#69 Promote the CDP key driver out of gitignored evidence** — every round-19 child since ticket 13 drives keyboard proofs through `.scratch/round19/evidence/13-fix/keys-cdp.ts` (+ `cdp.ts`, `driver.ts` beside it): trusted keys via CDP `Input.dispatchKeyEvent`, because agent-browser 0.31.1's `press` floods Chrome 154 with thousands of phantom keydowns and ends on `about:blank` (`press Meta+a` leaves Meta held). Those files sit under a gitignored `evidence/` dir, so round 19's closure prune — or losing the machine — deletes the only copy. Move them to a tracked dev-tool path (e.g. `scripts/browser/`), with the hardcoded fixture target (`localhost:5175`) made a parameter. Alternatively, drop them if a re-test shows agent-browser's `press` is fixed upstream. Source: ticket 13 fix Comments (`round19/issues/13-selection-bulk-rail-verbs.md`), the header of `keys-cdp.ts`; session log 2026-09-29 → 10-01.
- **#68 Retire the reporter's legacy `setError` dialect** — `notifyError` is meant to be the one failure dialect, but `useStatusReporter` still exposes `setError`, with five call sites left: `useSyncOrchestration.ts` (~:649, ~:686) and `useAddSkillFlow.ts` (~:335, ~:371, ~:669). `showActionErrors` / `showActionWarnings` have no production caller; only `useAddSkillFlow.test.ts`'s reporter literal uses them. Round 19 ticket 09 gave this migration to ticket 04's files. Neither 04 nor 16 (the Add rewrite) did it, no 19b ticket owns the sync or Add worlds, and the ticket-19 docs pass left AGENTS.md saying "remains only in the sync and Add worlds until they migrate" with no owner. Migrate those call sites to `notifyError` / `notify("error", …)`, delete the three reporter members, and drop the AGENTS.md clause in the same change. Source: `round19/issues/09-action-door-lifecycle-health-fold.md` ~:250 ("Legacy reporter members"); re-verified against the code 2026-10-02.
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

- **#77 Re-point renames to the upstream name by default.** Change source to a folder with a different skill name keeps
  the managed name and takes the new bytes. The operator wants *"it rename by default or give me a choice … the
  renaming should be default for sure"* (an upstream rename). A rename moves every target folder (global and project)
  and the central dir, and touches Edits and health keys. Decide the default and the override. Source: `round19/issues/18-operator-think-aloud.md` § Findings (P4·Q2).
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
  a cross-process lock (Mutation guard is process-wide only). Not upstream's `auto_update.rs`. Operator evidence (round 19 think-aloud, P7): he assumes his morning starts after an automatic schedule has refreshed; watch #71, because every Refresh also deploys. Source: `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §B3, §E6.
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
- Notification history persistence (#21) — per-row `last_error` is the durable record; round16 spec. **Reopened as #74** (operator evidence, 2026-10-27).
- Bulk local Re-point (#23) — never hit; its own effort if demand shows; round16 spec.
- WSL ↔ Windows path translation (#24), GitHub API ancestor-symlink (#25) — explicit exclusions, no new evidence; round16 spec.
- Collapse the three unsync commands (#27) — uniform since round 15; distinct toasts remain the reason; round16 spec.
  Moot since 1.3.0: round 19 ticket 01 replaced all three with Disable / Enable.
- Decouple `update_managed_skill` from the batch signature (#28 half) — "a batch of one" is the documented design; round16 spec.
- Document `content_identity::record`'s upsert (#29) — doc + 2-line body already say it; round16 spec.
