# Handoff — 2026-09-29 · round 19: tickets 01–11 committed; 11's D31 audit is fix-then-pass (P0 A1 + P1 A2–A4); next = fix child → re-check → ticket 12

The queue is `.scratch/BACKLOG.md` (next free number **#65**; #64 = the four accepted command-API compromises). Round 19's
record: `.scratch/round19/spec.md` (D1–D33) and `round19/issues/01`–`26`; every implemented ticket has a `## Result` and dated
`## Comments` (inter-ticket handoffs live there — read the target ticket's Comments before briefing its child). Reviews:
`round19/review/ticket-{01,04,07,09}-review.md` (all closed) and `ticket-11-audit.md` (open, see below).

## Operator decisions in force

- Spec + tickets pre-accepted; orchestrate 19a and 19b end to end. **Dispatch children only on Alex's explicit go** (he pauses
  around usage windows; a child cut by a 429 is resumed with `t3_thread_send` on ITS OWN thread, not re-dispatched).
- Routing: Opus 5.5 **high** for implementation, Astra high for adversarial review/audit, GPT-6 Luna (not 5.6) for mechanical
  migrations. All on Pi, `full-access`. Reviewer ≠ implementer; fix-then-ship with a bounded re-review on anything touching
  settlement, removal, asks or the lifecycle.

## State of `main` (clean tree; every commit gated — last: Rust 792 / vitest 768 / styles:check ok, App.css ceiling 2316)

`9d70918` wave 1 (01 Disable + 07 world) · `2803e6c` wave 2 (02 health, 05 onboarding plan, 08 surface state, 07 fixes) ·
`6d7b6f4` wave 3 (03 count line, 06 re-point preview, 10 catalog/platform/update/project world) · `df7cfc8` wave 4 (04 install
batch + one ask, 09 door/lifecycle/fold, both reviews' fixes) · `353e0fe` **11 My Skills, the first migrated surface** ·
`26f0dbf` 09 re-review residuals · `4217cd7` 11's audit report. Tickets 01–10 `implemented` and review-closed; 11 `implemented`
but **its D31 gate is not passed yet**. Round 18 is closed (`round18/spec.md` § Closure) but not yet moved to `archive/` — do it
at 19a closure.

## Exact next steps

1. **On Alex's go — fix ticket 11's audit findings** (`round19/review/ticket-11-audit.md` § Findings, A1–A4, each with
   file:line + fix + evidence paths under `round19/evidence/11-audit/`, gitignored). One Opus 5.5 child:
   - **A1 (P0)**: Escape from the **Enable overwrite ask** leaves the root `inert` and focus on `<body>` (reproduced at default
     latency, every run). It lives in the shared dialog/ask seam (`src/hooks/useSurfaceState.ts` ask stack, `OverwriteModal.tsx`
     rebuilt on Dialog primitives by 08, `components/ui/dialog.tsx` + `layer-stack.ts`, `useStatusReporter` pending) — the
     dismissed answer must clear the layer, un-inert the root, restore focus to the switch, and settle the pending action.
   - **A2 (P1)**: group-header repository link ring clipped by the `h2`'s `overflow:hidden` (`SkillsList.tsx` / group header).
   - **A3 (P1)**: at 960 px, exception badges squeeze a long name (`scratchpad`) to near-invisible (`SkillRow.tsx`; the name
     needs a min width / the marks a max, per DESIGN.md's row anatomy).
   - **A4 (P1)**: the strip's sr-only count text escapes the list's scroll container, creating an outer scrollbar
     (`SyncedStrip.tsx` — clip the sr-only span inside the row or use `aria-label` on the strip).
   Brief: "make your first tool call immediately", targeted edits, `npm run version:check && npm run check`, dated Comments on
   ticket 11. Then a **bounded Astra re-check of A1 only** (keyboard transcript through the ask in a headless session — the T3
   hidden tab cannot match `:focus-visible`), commit, mark 11's gate passed in its Comments.
2. **Ticket 18** (`ready-for-human`): the 45-min think-aloud with Alex on the fixture (`?scenario=failures`, `?scenario=first-run`;
   `npm run dev:fixture`, port 5175) using `round18/review/flows.md`'s per-flow questions; findings → Comments on 12/13/16/17/21.
   Schedule whenever Alex wants; it does not block 12.
3. Sequentially, each with its D31 child audit before `implemented` is accepted: **12** inspector (Comments carry 02/06 handoffs:
   Retry on failed first-sync rows; the re-point candidate URL decision) → **13** selection + bulk rail (new `confirm-count` ask
   kind; the toolbar's library-wide Disable has NO path today — 11 removed the button) → **14** notifications by run + header
   progress (replaces `LoadingOverlay`; `recordRun` already wired; passive update affordance from `useAppUpdate`; the
   overwrite-ask-mid-action focus gap) → **15** Emil pass → **16** Add (deletes `install_git_selection`/`install_local_selection`;
   Astra review) → **17** Projects → **19** release 1.3.0 (`npm run version:set 1.3.0`, CHANGELOG, AGENTS/CONTEXT re-read,
   `git fetch && git rebase origin/main` taking origin's `featured-skills.json`, push = release; operator VoiceOver smoke).
4. 19b: 20 ∥ 22 ∥ 23 → 21 → 24 → 25 → 26; then close the effort per `docs/agents/issue-tracker.md` § Lifecycle.

## Briefing rules that proved out this session

- Every brief states: first tool call immediately; two-line approach then continue; explicit file ownership when two children
  share the tree (re-read before each edit; leave notes in ticket Comments); never revert/stash/commit; `resources.ts` re-read +
  append at end of `en`; gate = `npm run version:check && npm run check`, report other-child failures precisely.
- Two children died silently (provider timeout at 3h20; a 56-min reasoning loop with zero tool calls) → "take the tree as is"
  re-briefs recovered them. A 429 mid-task is resumed on the same thread.
- Astra found real defects on every review (01: data-loss ownership hole; 04: Keep ignored + Cancel discarded; 07: focus ring
  never drew under Tailwind 4.3 `outline-none`; 09: three lifecycle bypasses; 11: A1). Keep the discipline.
- Contrast can be measured in the hidden T3 tab inside one `preview_evaluate`; keyboard/focus/`:focus-visible` proofs need a
  headless `agent-browser --session <name>` run.
- The orchestrator commits (children never do); shared-file waves are committed as one commit after the combined gate.

## Suggested skills for the next session

`handoff` (end of session), `code-review` (if Alex wants a standards/spec pass on a wave), `impeccable` (tickets 12–17 briefs
cite its build/audit modes), `emil-design-eng` + `animate` (ticket 15), `web-design-guidelines` (audits), `agent-browser`
(headless keyboard proofs), `domain-modeling` (CONTEXT.md terms at 19a closure), `grilling` only if a ticket's design question
needs Alex's decision.
