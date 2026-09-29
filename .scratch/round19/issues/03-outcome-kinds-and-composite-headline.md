# 03 — Outcome kinds `declined` and `cancelled`; composite count headline in the fold

Status: ready-for-agent
Blocked by: 01
Spec: `.scratch/round19/spec.md` — D10 (grill Q8). Source: `round18/review/flows.md` § "The fold's precedence contract".

## Why

`conflict › failure › skipped › success` picked "Refresh complete — invocation Edit conflicts need review" over five
failures; a cancelled Refresh reads "0 skills refreshed, 15 failed"; "Keep existing" is a red toast; Edit-with-drift
never says "saved".

## Work

### Backend

- `core/refresh.rs`: a cancelled run answers rows of a new kind **`Cancelled`** (its own variant in the per-skill
  outcome enum, beside `Refreshed`/`Failed`/`Skipped`), not `Failed { Cancelled }`. The same variant for candidates
  the new install batch (ticket 04) never reached — define the variant here so 04 reuses it. `specta::Type`, bindings
  regenerated.
- `global_sync`'s per-target outcome gains **`Declined`** (the pair the operator kept — ticket 02 records it as
  `disabled`; the report row says *declined*, not failed and not skipped).

### Frontend (`src/lib/reportOutcome.ts` + tests + i18n; nothing else)

- `Outcome` gains a **composite headline**: counts per kind in a fixed order — done · failed · skipped · declined ·
  cancelled · to review — rendered as "12 refreshed · 5 failed · 4 skipped · 1 Edit to review" (i18n with plural
  keys; a zero count is omitted; the verb is the action's: refreshed / installed / synced / imported / removed /
  assigned). The toast `tone` keeps a precedence, now **failure › conflict › skipped › declined › success**;
  `cancelled` alone is neutral ("Refresh cancelled — nothing was changed").
- `declined` rows produce **no error entry**; they produce one quiet warning-free line in the report
  ("ts-reset: kept your existing Claude Code folder").
- Edit with target drift: headline "Saved · 2 targets failed" (success tone is wrong; use warning) — the Edit *did*
  save centrally (ADR-0004).
- `refreshSummary` no longer drops the skipped count when anything failed.
- `completion` semantics unchanged (reload / closeModal / conflict); the `conflict` completion still fires when any
  conflict exists, independent of the headline.
- Every existing test in `reportOutcome.test.ts` is re-read against the new contract; add cases: mixed Refresh (the
  observed 5/6/4/1), cancelled Refresh, declined-only sync, Edit saved-with-drift, install batch with cancelled tail.
- `useNotificationHistory` stores the headline + the entries (grouping by run is ticket 14; keep the shape ready — a
  `runId` per fold call is cheap to add now).

### Fixture

- Cancelled Refresh answers the all-`Cancelled` report instead of throwing `CANCELLED` (a listed fixture gap).

## Out of scope

Toast/notification presentation (ticket 14); the header progress (ticket 14).

## Review

Orchestrator reads the diff; Astra not required unless the Rust enum change touches settlement paths beyond
`refresh.rs`.

## Result

(child fills in)
