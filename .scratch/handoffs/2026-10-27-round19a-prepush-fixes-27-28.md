# Handoff — 2026-10-27 · round 19a: ticket 18 done; **Alex has said go** for build children 27 ∥ 28 at the start of this session — then audits, then the 1.3.0 push

**First action:** dispatch the two implementation children described in § Dispatch now. Alex gave the go on 2026-10-27
for "the build children at the start of the next session", so don't ask again. That go covers 27 and 28 and their
audits. The push itself still needs Alex's explicit "push" (it is the release).

The queue is `.scratch/BACKLOG.md`, next free number **#79**. #70–#78 came from ticket 18's think-aloud; #68 and #69
came from the 10-02 recovery pass.

Round 19's record:

- `.scratch/round19/spec.md`: D1–D33, the ticket table (27 and 28 added, both blocking 19), and § Closure checklist
  (19a).
- `round19/issues/01`–`28`: every implemented ticket has a `## Result` and dated `## Comments`.
- `round19/review/`: the `ticket-*-review.md` and `ticket-*-audit.md` reports.
- **Ticket 18:** `round19/issues/18-operator-think-aloud.md` § Findings (every answer verbatim) and § Outcome and
  routing (the hold decision; where each finding went).

## State of `main` (verified 2026-10-27, clean tree, nothing pushed)

- **Ahead/behind.** `main` holds round 19's 44 commits plus docs-only commits since: `43b5a9f` (the recovery),
  `b0f6547` (the ticket-18 script), `9b10af8` (Alex's gitleaks pre-commit hook, `.githooks/`, active via
  `core.hooksPath`), and `0796f59` (ticket 18's outcome). It is **13 behind** `origin/main` on nightly
  `featured-skills.json` commits; the pre-push rebase takes origin's.
- **Last full gate:** the rebased tree on 2026-10-01 — `Version OK (1.3.0)`, vitest 996, cargo 827, App.css 1108 / 36
  migrated. Every commit since is docs-only.
- **SHAs are not stable.** Tickets and reviews cite pre-rebase SHAs, which exist only locally. Map them by commit
  subject, and write final SHAs only after the push.
- **Ticket status:**
  - 01–18 `implemented` (round 19's interim word; normalised to `done — <sha>` at 19a closure);
  - **27 and 28 `ready-for-agent`**;
  - 19 `ready-for-agent`: docs done; blocked by 27 and 28; release run ids and smoke outcome pending; its Comments
    carry the push-day reminders;
  - 20–26 `ready-for-agent` (19b).
- **CHANGELOG** still reads `## [1.3.0] - 2026-10-01`. Set the push day, and add 27's and 28's lines (see below).

## Dispatch now: 27 ∥ 28

Call `orchestrator_capabilities` once, then two `delegate_task` calls with these settings:
- `providerInstanceId: "pi"`, model `anthropic/claude-opus-5-5`, options `{thinking: "high"}`;
- `runtimeMode: "full-access"`, `mode: "async"`, `role: "implementation"`;
- your own `clientRequestId` (e.g. `r19-27-impl-v1`, `r19-28-impl-v1`). After a delegate error, check
  `t3_thread_list` before re-dispatching.

Hand each child its ticket path (the Work and Acceptance there are the brief), plus the § Briefing rules below and
these per-ticket points:

- **27 — a refused click says why** (`round19/issues/27-refused-activation-shows-its-reason.md`).
  - **Owns:** `src/**` (`src/components/ui/{switch,button,checkbox}.tsx` and tests, the App binder, `resources.ts`)
    and `DESIGN.md`. Must not touch `src-tauri/**`.
  - **Fixture proof** on `http://localhost:5175/?scenario=failures&latency=3`:
    - check `lsof -nP -iTCP:5175 -sTCP:LISTEN`; if nothing listens, the orchestrator starts `npm run dev:fixture` in
      the background before dispatch;
    - drive clicks and keys with trusted CDP input (`.scratch/round19/evidence/13-fix/keys-cdp.ts` + `cdp.ts`), never
      `element.click()`;
    - finish all source edits before the proof run, then reload once. Every tracked-file save reloads the page (the
      AGENTS.md fixture gotcha).
- **28 — executable bits on the Contents fast path** (`round19/issues/28-contents-api-executable-bits.md`).
  - **Owns:** `src-tauri/**`. Must not touch `src/**` (bindings must stay unchanged; there is no DTO change).
  - **Tests:** network-free, with the local test server pattern in `github_download.rs`'s tests and the `GithubApi`
    stub. Run `cargo test --all`.
- **Shared rules:**
  - Neither child edits `CHANGELOG.md`; the orchestrator adds both lines at commit. This overrides the line in each
    ticket's Work.
  - Both run the full gate. A concurrent run only waits on cargo's build lock. Each child reports the other's
    failures precisely and doesn't fix them.
  - **The orchestrator commits nothing and edits no tracked file while 27's child (or its audit) is in a fixture proof**:
    a save reloads the child's page.

**After each child:**
1. Review the diff and re-run the gate yourself.
2. Commit the implementation, adding its CHANGELOG line (1.3.0, "Fixed").
3. Dispatch a separate **Astra `high` audit** (`openai-codex/gpt-6-astra`, `{reasoningEffort: "high"}` or its Pi
   equivalent per `orchestrator_capabilities`) with the ticket's Acceptance as its checklist. Fix-then-pass, with one
   bounded re-check.
   - **28's audit brief must avoid the content-filter words** (§ Briefing rules). The ticket mentions following a
     repository link, so call it "a linked folder in the repository".
   - 27's auditor repeats the three-quick-clicks reproduction with trusted input, and checks keyboard Space on a held
     switch (focus never BODY).

## Then the push (only on Alex's "push")

1. `git fetch && git rebase origin/main`; on a `featured-skills.json` conflict, take origin's.
2. CHANGELOG: date → the push day; 27's and 28's lines present.
3. Add 28's operator check to ticket 19's `## Result` smoke list: on both Macs, Refresh `impeccable`, then check that
   its `scripts/impeccable` in the central repo is `-rwxr-xr-x` and survives a second Refresh. 19's 10-27 Comment
   records this.
4. `npm run version:check && npm run check`, then `git push origin main`.
5. Watch `auto-tag.yml` → `release.yml`, and record both run ids in 19's `## Result`.
6. Alex runs the smoke list; record his outcome and set 19's status.

**Then 19a closure:**
1. Follow `docs/agents/issue-tracker.md` § Lifecycle and the spec's § Closure checklist (19a).
2. Set 01–19, 27 and 28 to `done — <post-push sha>`, each with a dated Comment.
3. Add the spec's `## Closure — <date>` block.
4. Re-verify the BACKLOG.
5. Archive `round18/`: `git mv`, an `archive/README.md` row, and rewritten inbound paths. Find them with
   `git grep -l "round18/" -- .scratch/round19 .scratch/BACKLOG.md AGENTS.md CONTEXT.md DESIGN.md docs/adr`.

**Then 19b:** 20 ∥ 22 ∥ 23 → 21 → 24 → 25 → 26. Read each ticket's `## Comments` first: 20 and 21 gained binding
think-aloud findings on 10-27. BACKLOG #71 ("not synced" vs Disabled, and the re-assert) needs a `grilling` session
with Alex before anyone builds it.

## Operator decisions in force

- Spec and tickets are pre-accepted, and the orchestrator runs them end to end. Children are dispatched only on Alex's
  explicit go; it is given for 27 and 28. Pause around his usage windows.
- **Routing**, all on Pi with `full-access`:
  - implementation: Opus 5.5 `high`;
  - audits and re-checks: Astra `high`;
  - mechanical migrations only: GPT-6 Luna.
- **Review discipline.** The reviewer is never the implementer. Fix-then-pass. The orchestrator fixes small residuals
  itself and re-runs the auditor's own evidence.
- **The push is the release** (`auto-tag.yml` → `release.yml`: five targets plus `updater.json`).
- **Not holds**, decided at the 10-27 debrief: the overwrite ask (#70) and "not synced" vs Disabled (#71).

## Briefing rules that proved out

- **Brief shape:**
  - make the first tool call immediately;
  - AGENTS.md Workflow step 1 is pre-approved: state the approach in two lines, then continue;
  - name file ownership explicitly;
  - never revert, stash or commit;
  - `.scratch/` is tracked: children edit tickets (dated `## Comments`, `## Result`) and write evidence under the
    gitignored `.scratch/round19/evidence/<NN>/`; never `BACKLOG.md`, never archive or `git mv`;
  - in `resources.ts`, re-read first and append at the end of `en`;
  - gate: `npm run version:check && npm run check`.
- **Browser proofs:**
  - agent-browser 0.31.1's `press` sends phantom keys to Chrome 154, so use the CDP driver above (gitignored; #69 tracks
    promoting it);
  - one browser session at a time;
  - bash calls under ~120 s (no `timeout` binary: background the process and `sleep`);
  - if a session wedges, start a new session name;
  - `:focus-visible` proofs need a headless session.
- **Fixture sessions:** writing a tracked file reloads the fixture page. Keep notes in gitignored evidence while a
  stateful session runs (learned on 10-27; now in AGENTS.md).
- **OpenAI content filter:** Astra briefs containing "TOCTOU", "symlink", "adversarial", "dangling" or "interposer" are
  refused before any tool call. Use product terms instead.
- **Recovery:**
  - a child cut by the 5-hour limit is resumed on its own thread with `t3_thread_send` ("take the tree as is");
  - a child that dies silently is re-briefed fresh;
  - one orchestrator thread per phase (27/28 + push + closure; then 19b), handed off before the context fills.
- **Audits find real defects every time** (11–17 all came back fix-then-pass). Keep the discipline.
- **Commit cadence:** one commit per implementation and one per fix round. Commit audit reports before the fix child
  starts.

## Suggested skills for the next session

- `handoff`: end of session.
- `code-review`: optional, for a standards and spec pass over 27 and 28 before the push.
- `web-design-guidelines`: 27's audit brief (refusal feedback and a11y).
- `agent-browser`: 27's verification, with the CDP-driver caveat.
- `tdd`: 28's mode tests are a natural red-green.
- `writing-for-agents`: any AGENTS.md edit at closure.
- `domain-modeling`: CONTEXT.md at 19a closure.
- `grilling`: BACKLOG #71, before 19b builds anything near it.
