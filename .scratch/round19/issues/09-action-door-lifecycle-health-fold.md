# 09 — Skill action door, action lifecycle (Outcome executor, per-subject pending, focus rules), health/source fold

Status: ready-for-agent
Blocked by: 02, 03, 08
Spec: `.scratch/round19/spec.md` — D7 (arch 2, 3, 6), D19, D20, D21. Source: `architecture.md` candidates 2, 3, 6.

## Work

- **Action door — `src/hooks/useSkillActions.ts`** (candidate 2): one `run(verb, skills: ManagedSkill[] | ids, opts)`
  with verbs `syncToTargets · disable · enable · update · restore · repoint · detach · delete · editInvocation ·
  assignToProject · unassignFromProject · deployGroup`. The door decides batch vs one-at-a-time, where confirmation
  happens (through the Dialog-backed ask hooks), which fold settles it, and the reload rule ("a thrown sync also
  reloads" — `syncOrReload` — becomes door knowledge). Card, inspector, bulk rail and palette are callers. The 11
  per-skill callbacks threaded App → `SkillsList` → `SkillCard` collapse to one handle. Backend reached only through
  `invokeTauri`.
- **Health/source fold — `src/lib/skillPresentation.ts`** (candidate 3): `skillHealth(skill, toolCatalog) →
  { rows: HealthRow[], severity, source: { label, href, copyValue, icon } }` with skill-level rows (Unlocatable source
  / central copy, stored `health` from ticket 02: fetch failed, Edit conflict, delete incomplete, …) and target-level
  rows (N failed, N stale, disabled, not deployed), one precedence rule. Card, ladder, banner, chip and strip read it.
  Corpus-style tests as in the file's existing pattern. `SkillDetailView`'s second source-line rule is deleted.
- **Lifecycle — `useStatusReporter`** (candidate 6): the reporter **executes an `Outcome`** (`applyOutcome(outcome)`
  — toast with the composite headline, history entry with `runId`, completion), so the five hand-rolled unpackers
  (`useSkillLibrary`, `useAddSkillFlow` ×2, `useCandidatePick`, `ProjectsPage`) go. One failure dialect (`notifyError`);
  `setError(formatError())` sites migrate. **Per-subject pending**: `pending: { kind: 'skill' | 'cell' | 'batch', id? }`
  replaces the single `loading` boolean and its 18 prop-threading sites; a streamed batch exposes `{ label, done,
  total, cancellable }` for the header progress (rendered in ticket 14; the `LoadingOverlay` keeps rendering from the
  new state until then). The project world's `pendingCells` folds into it. **One mutation at a time** (D19) is a
  reporter rule: while a mutation is pending, other mutations are refused with a typed reason the UI can show.
- **Cancel (D20)**: `cancellable` is true only for the token-taking commands (list them from `commands/mod.rs` +
  ticket 04's batch); `cancelLoading` no longer hides anything it did not stop.
- **Focus rules (D21)** live here as `focusAfter(outcome, context)` returning the target per the spec's table, consumed
  by the Dialog primitive's `finalFocus` and by list/inspector after mutations. Unit-tested as a pure function.
- Tests: door (parametrised over verbs: fold called, reload rule, confirmation path), reporter (Outcome execution,
  pending kinds, refusal while pending, cancellable set), fold (corpus).
- AGENTS.md § Ambiguity: the "Frontend presentation logic" and hook bullets gain the door, the fold and the reporter's
  Outcome execution; the "props carry state, never pre-bound functions" rule is restated for the door handle.

## Gate

`npm run check` green; fixture walk: every existing action still works from the card through the door; a Refresh
shows the progress state; Cancel appears only on Refresh/Update/Re-point/Add.

## Review

Astra @ high: the door's confirmation and reload paths, the reporter's one-at-a-time rule under a thrown request,
focus-rule table coverage.

## Result

(child fills in)

## Comments

- 2026-09-29 — from ticket 02: `ManagedSkill.health: SkillHealth | null` (`{ kind, error, at }`, six kinds) is on the wire; `skillPresentation.skillHealth` is a stub returning it — build the full fold here. Decide how `delete_incomplete` / `import_failed` are dismissed (today only a newer outcome or Delete clears them) — recommend: Retry (delete → finish; import → nothing to retry, so an explicit "Dismiss" that clears the family). `deployAction` counts an `error` row as deployed, so a skill whose only target is a failed first sync shows no deploy — the door's `retry` verb must cover it.
