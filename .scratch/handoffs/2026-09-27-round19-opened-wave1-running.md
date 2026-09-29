# Handoff — 2026-09-27 · round 19 opened (spec + 26 tickets, pre-accepted); wave 1 running

The queue is `.scratch/BACKLOG.md`. Round 19's record is `.scratch/round19/spec.md` (D1–D33) and `round19/issues/01`–`26`.
Round 18 is closed (`round18/spec.md` § Closure) but **not yet moved to `archive/`** — the move waits until no running child
was briefed with `round18/` paths (wave 1 was). When moving: `git mv .scratch/round18 .scratch/archive/round18`, add the
README row, and `sed` the `round18/` references in `round19/issues/*.md`, `round19/spec.md`, AGENTS.md and this note.

## Where things stand

- Operator decisions this session: spec and tickets pre-accepted; orchestrate 19a and 19b end to end. Model routing: Opus 5.5
  **high** for implementation, Astra high for review/audit, GPT-6 Luna (not 5.6) for mechanical migrations — AGENTS.md updated.
- **Wave 1 dispatched** (`delegate_task`, Pi, Opus 5.5 high, full-access, one shared working tree, file ownership stated in each brief):
  - ticket 01 `disabled` target state — clientRequestId `r19-t01-disabled-v1`
  - ticket 07 world ticket (tokens, primitives on Base UI, guard, DESIGN.md) — `r19-t07-world-v1`
- Fixture server running on 5175 from the orchestrator (`/tmp/skillshub-fixture.log`); children were told not to start another.

## Progress log

- 2026-09-29 — **Wave 1 committed as `9d70918`**: ticket 01 (Disable; Astra review fix-then-ship ×2, all findings closed incl.
  the status-independent ownership rule `owns_artifact` = `synced_at` set) and ticket 07 (world: tokens measured, primitives,
  guard, DESIGN.md). Lessons: two children died mid-task (provider timeout at 3h20; a 56-min reasoning loop with zero tool
  calls) — re-brief with "make your first tool call immediately" and "take the tree as is". Ticket 01's first child kept
  design choices the ticket text did not prescribe (`disabled_at` off `SkillRecord`; `ProjectSyncSkip`) — sound, recorded.
- **Wave 2 dispatched** (`r19-t07-review-astra-v1` reviews the commit; `r19-t08-surface-state-v1`, `r19-t02-health-v1`,
  `r19-t05-onboarding-v1` implement on the shared tree with stated file ownership).

- 2026-09-29 — **Wave 2 committed as `2803e6c`**: 02 (health), 05 (onboarding plan), 08 (surface state; steered mid-flight to add
  the `dismissed` ask answer so Escape never records Keep), plus 07's Astra fixes (focus ring never drew under Tailwind 4.3
  `outline-none` — proven on compiled CSS + a headless keyboard run; the T3 hidden tab cannot match `:focus-visible`, so
  ring/keyboard proofs need a headless agent-browser session). Reviews under `round19/review/`.
- **Wave 3 dispatched**: `r19-t03-outcomes-v1`, `r19-t06-repoint-v1`, `r19-t10-modules-v1` (disjoint files; ownership stated).
  Next: 04 ∥ 09 (09 after 03 + 10), Astra on 04 + 09, then 11 alone.

## Wave plan (D9: sequential on styling hotspots; parallel only on disjoint files)

1. 01 ∥ 07 → Astra reviews both (01: settlement/guard/skips; 07: contrast re-measured, Dialog transcript, guard rules).
2. 08 (after 07) ∥ 02 (after 01) ∥ 05 (after 01; shares fixture/bindings — targeted edits only).
3. 03 ∥ 06 ∥ 10 (10 after 08).
4. 04 ∥ 09 (09 after 02, 03, 08) → Astra reviews 04 and 09.
5. 11 (alone; first migrated surface) → child audit (Astra). Then 18 (operator think-aloud, `ready-for-human`).
6. 12 → 13 → 14 → 15 (each with its D31 audit) → 16 (Astra review) → 17 → 19 (release 1.3.0; operator VoiceOver smoke).
7. 19b: 20 ∥ 22 ∥ 23 (disjoint surfaces) → 21 → 24 → 25 → 26; then close the effort per the lifecycle doc.

## Orchestrator duties per child

Verify the child's writes yourself (`git diff --stat`, run `npm run version:check && npm run check`), read the ticket's
`## Result`, commit with a message naming the ticket, then dispatch the review child (different model) where the spec's
§ Execution says so; fix-then-ship with a bounded re-review. After ANY `delegate_task` error, `t3_thread_list` before
re-dispatching (duplicate-writer risk). Never `mode: wait` for children longer than a minute.

## Gotchas carried from round 18

Children in one tree: `cargo test` regenerates `src/bindings/index.ts` and hot-reloads every fixture preview; sequence tickets that
share files; two children's preview tabs share browser storage. Brief for "prove the cheap option first". Fixture fidelity is a
review input.
