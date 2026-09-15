# Carve App.tsx into hooks (useProjectState pattern)

Status: resolved

Type: grilling
Blocked by: 03

## Question

App.tsx (2,716 lines, 60 useState, 85 invokeTauri) interleaves five state worlds in one closure: settings (~10 states, 431–473), update-checker (5 states, 520–541), explore (6 states), add/import flow (~14 states), and sync orchestration. `useProjectState.ts` (442 lines) is the sanctioned counter-example: one hook = one world's state + actions + error normalization; page stays a thin binder.

Decide, then implement (per map Notes — this is the ticket most likely to need splitting into follow-up task tickets):

- The hook inventory and each hook's interface: `useSkillLibrary`, `useSyncOrchestration` (consumes ticket 03's batch command), `useSettingsState`, `useExploreState`, `useUpdateChecker` — confirm the carve lines against the worlds as they exist *after* tickets 02–04 land.
- Extraction order (lowest-risk first?), and what stays in App() as the binder.
- `useProjectState`'s own wrinkle: the "re-fetch assignments on error, silent fallback" block repeats 4× — extract `refreshAssignments()` while in the area?
- Stays strictly within the useState+props rule: hooks returning `{data, actions}`, no Context/store. Reword AGENTS.md's state-management rule in plain terms while here (per map Notes).
- Sets up the fog item "unit-test the extracted hooks (vitest)".

Context: scan finding 8; report card #5.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

2026-09-12 (ticket 03 resolution note, kept for history): unblocked. Ticket 03 shrank this ticket's `useSyncOrchestration` to extracting two existing App.tsx helpers — `syncSkillsToTools` (Channel + invoke) and `syncFailureEntries` — plus their call sites; all fan-out choreography now lives backend-side. Fold into this ticket: (a) the flagged refresh-all oddity — it syncs with no overwrite flags, so changed targets collect `TARGET_EXISTS` errors (pre-existing; probably wants `overwrite: true`); (b) `errorCode`/`toCommandError` are no longer imported by App.tsx — check for dead exports in `commandError.ts` when carving.

## Answer

All recommendations accepted (Q1–Q10, one round). Landed green on main as `fc89d1f` (carve) + `bb77d83` (fold-ins). App.tsx 2,588 → 629 lines; no session split needed.

**Decisions:**

1. **Six world hooks, not five** (Q1): the add/import flow (~16 states, ~620 handler lines) got its own `useAddSkillFlow` rather than being folded into `useSkillLibrary` or left in App.
2. **One shared status surface** (Q2): `useStatusReporter` owns loading/loadingStartAt/actionMessage plus the one-shot error/success toast triggers, `formatError`, `showActionErrors`, and `cancelLoading`. Instantiated once in App; every world hook receives it as a dependency (accept dependencies, don't create them) — single spinner/toast UX preserved, hooks testable with a mock reporter.
3. **App wires, hooks never import each other** (Q3): dependency order reporter → sync → library → settings/explore/addFlow. Key unblocking move: **`autoSyncEnabled` belongs to the sync world** (it's the auto-sync switch), which dissolves the settings↔library cycle; settings receives `onManagedSkillsChanged`, explore receives `onOpenExploreDetail` (navigation stays in the binder). Cross-world composites live in App (`handleSyncAllNewTools` = sync.enableTargetsFor + library.syncAllManagedToTools).
4. **Placement** (Q4): app-level hooks in `src/hooks/`; new `src/lib/tauri.ts` exports `invokeTauri`/`isTauri` as module functions (the seam every hook shares). `components/shared/` stays presentational.
5. **What stays in App** (the binder): i18n/language, view/navigation state (activeView, detailSkill, searchQuery, sortBy, groupByRepo, viewMode + storage effects), `visibleSkills` filtering, display helpers, render tree.
6. **Refresh-all overwrite fix** (Q6, behavior change): the post-refresh bulk sync now passes `{ overwrite: true }` — refresh means "push the updated content"; previously changed targets all failed with TARGET_EXISTS.
7. **Dead exports** (Q7): `errorCode` + `CommandErrorCode` deleted from `commandError.ts` (zero importers since ticket 03).
8. **`refreshAssignments()`** (Q8): extracted in `useProjectState` for the 4 guarded silent re-fetch blocks; toggle/bulk **success-path** re-fetches deliberately stay unguarded (failure there surfaced to the caller before, and still does).
9. **AGENTS.md reword** (Q9): no-state-library rule now describes the hooks shape (per-world hooks return data+actions, App binds, still plain useState+props, no Context/store).
10. **vitest deferred** (Q10): graduated as ticket 09; this carve stayed behavior-preserving (gate: build + lint + clippy + 212 rust tests) except the decided overwrite fix.

**Interface inventory** (for future sessions): `useStatusReporter(t)`, `useUpdateChecker(t)`, `useSyncOrchestration({t, reporter})`, `useSkillLibrary({t, reporter, sync})`, `useSettingsState({t, reporter, onManagedSkillsChanged})`, `useExploreState({t, reporter, onOpenExploreDetail})`, `useAddSkillFlow({t, reporter, sync, library})`. `sync`/`library` deps are `Pick<>`-narrowed in consumer signatures.

Gate: `npm run version:check` exit 0; `npm run check` exit 0; `cargo test --all` 212 passed (unmasked exit codes).
