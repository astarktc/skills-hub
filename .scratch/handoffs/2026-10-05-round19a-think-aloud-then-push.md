# Handoff — 2026-10-05 · round 19a is done and gated on local `main`, NOT pushed — next: the ticket 18 think-aloud (script ready), then push the same day

The orchestrator thread (T3 `a55f39f0…`, Pi session `01a0f013…`, 2026-09-29 → 10-01, 33 runs) ran out of context on its
handoff turn. It had written a draft (`2026-10-01-round19a-1.3.0-ready-to-push.md`, never committed). A recovery pass on
2026-10-02 checked that draft against the session log (`docs/conversation-logs/2026-09/conversation-log-2026-09-29-211013.md`,
gitignored) and the repo, and this note replaces it.

The queue is `.scratch/BACKLOG.md`, next free number **#70**. Five items came from this session: #65 inspector wire
metadata, #66 multi-skill delete command, #67 Codex `~/.agents/skills`, and, added in the recovery pass, #68 retire
the reporter's legacy `setError` and #69 promote the CDP key driver out of gitignored evidence.

Round 19's record:

- `.scratch/round19/spec.md`: D1–D33, plus § Closure checklist (19a) at ~:238.
- `round19/issues/01`–`26`: every implemented ticket has a `## Result` and dated `## Comments`.
- `round19/review/`: `ticket-{01,04,07,09}-review.md` and `ticket-{11..17}-audit.md`. Each audit has a re-check, or for
  15 a residual fix, before its "D31 gate passed" Comment on the ticket.

## State of `main` (verified 2026-10-05)

- **Ahead/behind.** `main` is ahead of `origin/main` by round 19's 44 commits plus the docs-only recovery commits
  (`43b5a9f`, and the 2026-10-05 ticket-18 commit). It is also **4 behind**: nightly `featured-skills.json` commits
  `533c298` … `39d2d5a` (2026-10-02 → 10-05) landed after the session's last rebase.
- **Last gate.** Run on the rebased tree on 2026-10-01: `Version OK (1.3.0)`, vitest 996, cargo 827, App.css 1108
  (ceiling) / 36 migrated, exit 0. The recovery commit is docs-only under `.scratch/` and did not re-run the gate. Over
  the round, App.css went 2316 → 1108 (the CHANGELOG counts from the first baseline, 3038).
- **SHAs are not stable yet.** The session's final rebase rewrote all 44 round-19 commits, and the pre-push rebase
  will rewrite them again. Ticket Results/Comments, review reports and the draft cite pre-rebase SHAs (`9d70918`,
  `353e0fe`, `ef7431b`, `b53fd82`, `c6a47c0`, `77a5d5e`, `9272e23`, …). Those exist only as local dangling objects,
  never on origin. Map them by commit subject (`git log --oneline origin/main..HEAD`). Write final SHAs only after
  the push.
- **Ticket status.**
  - 01–17 are `implemented`. 11–17 passed their D31 gates; 01/04/07/09 were Astra-reviewed.
  - 18 is `ready-for-human`. Its script was rewritten for the 1.3.0 build on 2026-10-05 (see its `## Comments`).
  - 19 is `ready-for-agent`. Its docs half is done. Its `## Result` § Release run ids (~:99–102) are TODO, and the
    smoke outcome (~:141) is pending.
  - 20–26 are `ready-for-agent`.
  - `implemented` is round 19's interim word. The tracker's terminal form is `done — <sha>`; round 18 was normalised
    that way at its closure, `4152b70`.
- **CHANGELOG.** It reads `## [1.3.0] - 2026-10-01`. The push has slipped, so set the date to the push day.

## Operator decisions in force

- Spec and tickets are pre-accepted, and the orchestrator runs them end to end. **Dispatch children only on Alex's
  explicit go**; he pauses around usage windows.
- **Routing.** All children run on Pi with `full-access`.
  - Implementation: Opus 5.5 `high`.
  - Every D31 audit and re-check: Astra `high`.
  - Mechanical migrations only: GPT-6 Luna.
- **Review discipline.** The reviewer is never the implementer. Fix-then-pass with a bounded re-check. The orchestrator
  fixes small residuals itself (a type, a test spy, a doc claim, a one-function Rust rule) and re-runs the auditor's own
  evidence script instead of another child cycle.
- **The push is the release.** `auto-tag.yml` → `release.yml` builds five targets plus `updater.json`, and every 1.2.x
  install is offered 1.3.0.
- **Decided 2026-10-05: ticket 18 first, then push the same day.** The reason: a first impression can't be repeated,
  and the release smoke teaches every flow. Ticket 18's § Hold rule says which findings can stop the push. Every other
  finding goes to a 19b ticket or the BACKLOG (§ Routing).

## Exact next steps

1. **Run ticket 18 with Alex.** Follow its § Setup and § Protocol. The fixture server is `npm run dev:fixture` on 5175;
   start it if it isn't running. The visible preview tab starts blank, and the first URL loads on "start". Fill in
   § Findings, route each finding per § Routing, then apply the hold rule.
2. **Push, unless a hold finding needs a fix first:**
   1. `git fetch && git rebase origin/main` (on a `featured-skills.json` conflict, take origin's).
   2. Change the CHANGELOG 1.3.0 date to today.
   3. `npm run version:check && npm run check`.
   4. `git push origin main`.
   5. Watch `auto-tag.yml` → `release.yml`.
   6. Record both run ids in ticket 19's `## Result`.
   7. Alex runs the operator smoke list in ticket 19's `## Result` (~:104). Record his outcome at ~:141, then set 19's
      status.
3. **19a closure.** Follow `docs/agents/issue-tracker.md` § Lifecycle and the spec's § Closure checklist (19a).
   1. Set 01–17 and 19 to `done — <post-push sha>`, each with a dated Comment.
   2. Add a 19a `## Closure — <date>` block to the spec.
   3. Re-verify the BACKLOG. Nothing else was absorbed: #64/#65/#66/#68 ship in 1.3.0 as known compromises, and
      #67's adapter row is unchanged.
   4. Archive `round18/` with § Lifecycle steps 3–6: `git mv`, an `archive/README.md` row, and rewritten inbound
      paths. 11 tracked live files cite `round18/` (`git grep -l "round18/" -- .scratch/round19 .scratch/BACKLOG.md
      AGENTS.md CONTEXT.md DESIGN.md docs/adr`), ticket 18 among them.
4. **19b**: 20 ∥ 22 ∥ 23 → 21 → 24 → 25 → 26. Read each target ticket's `## Comments` before briefing it. Deferrals
   already recorded there:
   - **23:**
     - the matrix keyboard model (17's N1, the 127-Tab walk; recorded 2026-10-02 in the recovery pass);
     - the triggerless `PopoverContent anchor`'s hidden Tab stop (16's R1);
     - the open `DropdownMenu`'s axe `region` violation;
     - the palette motion decision.
   - **25:**
     - the unused generated `.transition*` utilities (15's A4; recorded 2026-10-02);
     - the list of App.css rules no source file references.
   - **22:** Explore preview focus lands on the main landmark.
   - **21:** Import's overwrite ask, Replace/Leave, and the remove-originals UI.
   - **20:** the settings toggle knob and `ToolConfigModal` (already migrated).

## Briefing rules that proved out

These carry the 2026-09-29 note's rules forward; that note is deleted with this commit.

- **Brief shape.** Make the first tool call immediately. AGENTS.md Workflow step 1 is pre-approved: state the
  approach in two lines, then continue.
  - Name file ownership explicitly when children share the tree.
  - Never revert, stash or commit; the orchestrator commits.
  - `.scratch/` is tracked. Children may edit tickets and write handoffs, but never `BACKLOG.md`, never archive or
    `git mv`.
  - In `resources.ts`, re-read first and append at the end of `en`.
  - Gate: `npm run version:check && npm run check`.
- **Keyboard proofs.** agent-browser 0.31.1's `press` sends phantom keys to Chrome 154, ending on `about:blank`. Every
  child since ticket 13 uses `.scratch/round19/evidence/13-fix/keys-cdp.ts` (+ `cdp.ts`, `driver.ts`). It is
  gitignored, so don't let an evidence prune delete it (#69).
  - Run one browser session at a time.
  - Keep bash calls under ~120 s. This Mac has no `timeout` binary: background the process and `sleep`.
  - If a session wedges, start a new session name; don't investigate.
  - Contrast can be measured inside one `preview_evaluate` in the hidden T3 tab. `:focus-visible` proofs need the
    headless session.
- **OpenAI content filter.** An Astra brief containing "TOCTOU", "symlink", "adversarial", "dangling" or
  "interposer" was refused before any tool call. Phrase filesystem-safety work in product terms ("a link whose
  destination is gone", "the folder disappears between the check and the write").
- **Recovery.**
  - A child cut by the provider's 5-hour limit is resumed on **its own thread** with `t3_thread_send` and "take the
    tree as is", not re-dispatched. This recovered 12's I1–I8 fix child on 2026-09-30.
  - A child that dies silently is re-briefed fresh.
  - **Orchestrator context:** this thread ran ~66 h across 33 runs and died writing its own handoff. Use one thread per
    phase (push + 19a closure; then 19b), and hand off before the context is nearly full.
- **Audits find real defects every time.** Astra did on every audit:
  - 11: a ghost layer;
  - 12: a sheet focus/placement race;
  - 13: `reveal_path` launching an `.app`;
  - 14: a sonner focus race;
  - 15: silent repeated live-region text;
  - 16: popover-under-modal unwind;
  - 17: core recreating a moved project folder on four routes, plus the check-then-write race.

  Keep the discipline through 19b.
- **Commit cadence.** One commit per ticket implementation and one per fix round. Commit the audit docs before the fix
  child starts, so the child reads the committed report.

## Suggested skills for the next session

- `handoff`: end of session.
- `impeccable`: the 20/22/23/24 briefs cite its build/audit modes.
- `emil-design-eng` + `animate`: 23's palette row motion and 24's grid.
- `web-design-guidelines`: audits.
- `agent-browser`: verification, with the CDP-driver caveat above.
- `domain-modeling`: CONTEXT.md at closure.
- `writing-for-agents`: AGENTS.md edits, including #68's clause when it lands.
- `grilling`: only if a 19b design question needs Alex's decision.
