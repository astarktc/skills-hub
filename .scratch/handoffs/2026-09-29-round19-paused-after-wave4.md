# Handoff — 2026-09-29 · round 19 paused after wave 4 (tickets 01–10 committed); ticket 11 + an Astra re-check were in flight

The queue is `.scratch/BACKLOG.md`. Round 19's record is `.scratch/round19/spec.md` (D1–D33) and `round19/issues/01`–`26`;
every implemented ticket carries a `## Result` and dated `## Comments` (handoffs between tickets live there — read the target
ticket's Comments before briefing its child). Reviews under `round19/review/ticket-{01,04,07,09}-review.md`.

## Where things stand

- **Operator decisions**: spec + tickets pre-accepted; orchestrate 19a and 19b end to end. Model routing: Opus 5.5 **high** for
  implementation, Astra high for adversarial review, GPT-6 Luna (not 5.6) for mechanical migrations. Paused by the operator
  2026-09-29 mid-wave-5: **dispatch no new children until Alex says continue.**
- **Committed on `main`** (all gates green at each commit; `npm run check` last run: Rust 792 / vitest 744 / styles:check ok):
  - `9d70918` wave 1 — 01 Disable (durable un-deploy; status-independent ownership rule `owns_artifact` = `synced_at` set), 07 world
    (tokens measured, Base UI primitives, `styles:check`, DESIGN.md).
  - `2803e6c` wave 2 — 02 persisted skill health, 05 onboarding plan (`name_is_managed`, gated scan, `remove_originals`), 08 surface
    state + hash + list lens (asks resolve confirmed | declined | dismissed), 07's review fixes (focus rings actually draw).
  - `6d7b6f4` wave 3 — 03 count-line headline + `cancelled`/`declined`, 06 re-point preview + local source, 10 tool catalog /
    `platform.ts` / `useAppUpdate` / project world in the binder.
  - `df7cfc8` wave 4 — 04 install batch + one overwrite ask across every producer, 09 action door + Outcome lifecycle + health fold;
    both Astra reviews' findings closed (04: F1–F5; 09: B1–B3, S4–S9).
  - Tickets 01–10 `Status: implemented`. **Round 18** is closed (`round18/spec.md` § Closure) but not yet `git mv`'d to `archive/`
    — do it at 19a closure (children were briefed with `round18/` paths).
- **In flight when paused** (durable T3 child threads; check with `t3_thread_list` / `task_status`, do not re-dispatch):
  - `r19-t11-my-skills-v1` — ticket 11, My Skills as the first migrated surface (Opus 5.5 high). It edits
    `src/components/skills/**`, `App.tsx` shell markup, `App.css` (deleting rules), `scripts/styles-baseline.json`, i18n. If it
    finished: verify (`git status`, `npm run version:check && npm run check`), read its `## Result`, dispatch the D31 Astra audit
    (keyboard transcript, contrast, grayscale strip, 960×640), commit. If it died mid-way: re-brief a fresh child with "take the
    tree as is; make your first tool call immediately" (two earlier children died: a provider timeout at 3h20 and a 56-min
    reasoning loop with zero tool calls).
  - `r19-t04-t09-rereview-astra-v1` — bounded Astra re-check of the 04/09 blockers on commit `df7cfc8`; writes only
    `round19/review/ticket-0{4,9}-review.md` § Re-review + ticket Comments. If it says fix-then-ship, brief a narrow fix child.
- Fixture server: was running on 5175 from the orchestrator (`/tmp/skillshub-fixture.log`); a fresh session must restart it
  (`npm run dev:fixture &`) before briefing any child that uses `preview_*`.

## Exact next steps (in order)

1. Read the two in-flight results (above). Land 11 (+ its D31 audit) → commit → 18 becomes actionable (`ready-for-human`: the
   45-min think-aloud with Alex on `?scenario=failures` and `?scenario=first-run`).
2. Then sequentially, each with its D31 child audit before `implemented`: **12** inspector → **13** selection + bulk rail →
   **14** notifications by run + header progress → **15** Emil pass → **16** Add (Astra review) → **17** Projects → **19**
   release 1.3.0 (`version:set 1.3.0`, CHANGELOG, AGENTS/CONTEXT invariants re-read, push = release; operator VoiceOver smoke).
3. 19b: 20 ∥ 22 ∥ 23 → 21 → 24 → 25 → 26; then close the effort per `docs/agents/issue-tracker.md` § Lifecycle.

## Briefing rules that proved out

- Every brief: "make your first tool call immediately"; "state your approach in two lines and continue"; explicit file ownership
  when two children share the tree; "never revert/stash/commit"; "re-read `resources.ts` before each edit, append at end of `en`";
  gate = `npm run version:check && npm run check`, report other-child failures precisely rather than fixing them.
- Astra reviews paid for themselves every time (01: three blockers incl. data loss; 04: two; 07: the focus ring never drew;
  09: three). Keep fix-then-ship with a bounded re-review for anything touching settlement, removal, asks or the lifecycle.
- The T3 hidden preview tab cannot match `:focus-visible` — keyboard/focus proofs need a headless `agent-browser --session`
  run (07's fix child did this); contrast can be measured in the hidden tab inside one `preview_evaluate`.
- Two children coordinated well on shared files by re-reading before each edit and leaving each other notes in ticket Comments.

## Command-API compromises recorded by 09 (judge at the re-check; BACKLOG if accepted)

Per-skill project-assign commands (no batch); Deploy all = Enable then sync (two commands, one ask); a successfully answered
project collision row is dropped from the Update/Edit report rather than shown synced; Enable retry re-runs the whole scope.
