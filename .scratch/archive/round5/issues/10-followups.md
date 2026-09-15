# 10: Round-5 review follow-ups (non-blocking)

Status: superseded — round6/issues/01-finalize-atomicity

**Source:** round-5 panel (Fable 5.1 / Opus 5 / GPT-5.6 Sol / GPT-6 Astra-low) over `ed630c0..378a473`; each item verified at HEAD by the orchestrator. Items taken into ticket 09 are omitted. Reports: `r5-review-{fable,opus,sol,astra}.md`.

## Needs an operator ruling
1. **Should a Re-point (local or git) honour the auto-sync `reassert` policy like Update does?** (Sol Std 1 / Spec 4.) Both Re-point commands run `RefreshPolicy::default()` (`reassert_auto_sync: false`) — round 4 chose that for local Re-point; git Re-point mirrors it. Spec Q12 calls it "Update with a source override". Options: (a) add a `reassertAutoSync` arg to both Re-point commands and pass `autoSyncEnabled` from the hook, exactly as Update; (b) keep repairs side-effect-minimal and record the rule in CONTEXT.md **Re-point**.

## Data safety (inherited, not a round-5 regression)
2. **`finalize_update` is not failure-atomic** (Astra P2, Sol Spec 1): `remove_dir_all(central)` → `staged.move_into` → `upsert_skill`. A move failure leaves central missing (→ Unlocatable `central_missing`, Restore repairs it); an upsert failure leaves new bytes with the old source row. Dates from `fe60d94` (round 2); every Update has it. → Swap-in via rename-aside (`central.old`) and remove after upsert, or a WAL-style two-phase; test with injected failures.
3. **Internal symlink inside a skill folder** (Opus Spec 2, Fable import residual): `copy_dir_recursive` drops symlink entries; `hash_dir` hashes their path → the chosen original of an import can be reported `KeptDivergent` against its own central copy, and Propagation's same-content check disagrees with copy. → Decide once: copy symlinks as symlinks, or make `hash_dir` skip them; then re-check `onboarding_import`'s header claim "the chosen variant's own Tool always among them".

## Behaviour
4. **Branch names containing `/`** (Sol Spec 3): `parse_github_url` splits `/tree/<branch>/<path>` at the first `/`. Pre-existing for Add and Update. → Resolve against the repo's ref list (API `git/refs/heads`) or accept `?ref=` disambiguation.
5. **Stale notification action after delete** (Astra P3): the recorded closure holds the `ManagedSkill`; deleting the skill leaves a Re-point action that opens the modal and then fails `NOT_FOUND`. → Resolve by id at click time and refuse gracefully.
6. **Toast action is head-only** (Sol Spec 5): `showActionBatch` toasts only the first entry's action; later GitHub-not-found rows have the action in the panel only. By design of batching; note in the reporter doc or toast each actionable entry.

## Code shape
7. `GitRepointModal` `loading`/`closeDisabled` props are dead — the hook closes the modal before invoking (Fable Std 4). → Keep the modal open through the action (spinner where the operator typed) or drop the props.
8. `detailSkill` freshness lookup inline in `App.tsx` JSX (Fable Std 5, Opus Std 3). → Store the id; select in the hook.
9. "One repair, two provenances" decided twice: `SkillCard` (`kind === "git"`) and `useSkillLibrary.handleRepointSkill` (Opus Std 4). → One door.
10. Ticket Comments cite worktree SHAs that were rewritten by rebase (Fable Std 8). → Orchestrator note: children should cite commit *titles*, not SHAs.

## Verified clean by all four (for the record)
Acquire-first holds on every Re-point failure path (in-memory record; `upsert_skill` only on the non-override backfill branch); override reaches both adapters; all three new `SignalError`s have complete typed/localised plumbing; `require_local` stays prose per the new rule; no `Notification.action` is serialised; `skillFailureEntries` cannot pick a wrong skill; import can never overwrite a divergent original (fail-safe direction).

## Reported by the operator after the 1.2.5 smoke test
11. **The warning toast "X skills refreshed, Y failed." cannot be dismissed** — no close button, and it does not auto-dismiss. `useStatusReporter.ts::showToast` gives `warning` a 5 s duration and no `closeButton` (only `error` gets one, because its duration is `Infinity`). Suspects: sonner pauses timers while the pointer is over the toaster region or while an infinite error toast is stacked with it; the batch error toasts (`showActionErrors`) are raised in the same tick as the summary. Reproduce with a Refresh that has ≥1 failure. Fix direction: `closeButton: true` on every kind whose lifetime can exceed a glance (warning at least) — the reporter is "the single owner of toast lifetime", so it is one line — and verify the 5 s timer actually fires when an infinite sibling is present.

## Comments

- 2026-09-15 — Status reconciliation: needs-triage → superseded — round6/issues/01-finalize-atomicity. Evidence: Round6/spec.md D1–D10 routes the aggregate to terminal round6/issues/01-finalize-atomicity (primary), 02-internal-symlinks, 03-branch-with-slash, 04-frontend-shape and 05-review-fixes; all exist and are terminal. Pre-lane f961ce8/95b7893 handle warning close/Re-point policy; src/hooks/useStatusReporter.ts:60 confirms warning closeButton. Head-only actions and title-citation policy are recorded decisions, not remaining implementation.
