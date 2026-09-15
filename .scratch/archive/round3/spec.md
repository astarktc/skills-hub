# Round 3 — review follow-ups + notification UX (post v1.2.2)

Source: the two open tickets carried over from round 2, now in this directory as
`source-13-review-followups.md` (12 verified judgement-call findings + one tooling gap, from the
3-model review panel of `af71598..64c19be`) and `source-14-toast-lifetime.md` (operator-reported
UX gap while smoke-testing v1.2.2). Decisions below were settled with the operator on 2026-09-04
at HEAD `f400440` (main, v1.2.2 + nightly featured-skills bump).

Vocabulary is in `CONTEXT.md`. This round adds one term, **Notification** (see Q4), which ticket 02
adds to the glossary.

## Why

Round 2 deepened the modules; the panel's non-blocking residue is the same-rule-in-two-places kind
(shared-dir predicate ×3, project-artifact naming rule ×2, sync-one-assignment ×2, error chain
flattened in one of two sibling reports) plus dead seams and doc drift. Separately, the app's only
outcome surface — toasts — vanishes in 1.8–3.2 s and leaves no trace, so a failed install/refresh is
unreadable and unrecoverable. Both are cheap to fix now and expensive to leave (every new call site
copies the wrong rule / adds another `toast.*` with its own duration).

## Settled decisions (do not re-litigate; reopen via a comment on the ticket)

| # | Decision |
|---|---|
| Q0 | Round lives in `.scratch/round3/`; round-2 dir is closed. Source tickets moved here as `source-*.md` (read-only references). |
| Q1 | **Onboarding import, auto-sync on, source Tool deselected (#10)**: the Tool the chosen variant was found in is **force-included** in the sync target set (union with `policy.tools`), overwriting the original in place per ticket 06's wording. Never deleted, never left untracked. The report names the forced inclusion so the UI can say why a deselected Tool received a link. No UI change to the selection itself. |
| Q2 | **Glossary vs. names (#4)**: rename **Rust-internal** names only — `unassign_and_cleanup`, `remove_tool_with_cleanup`, `remove_project_with_cleanup(_unlocked)` and the `cleanup`-worded doc comments follow the *Artifact removal* vocabulary (`…_and_remove_artifacts` / `remove_…_artifacts` style; ticket picks the exact names, consistent across the three). The wire code `DELETE_CLEANUP_FAILED`, the `CommandError`/`SignalError` variant names and the `errors.importCleanupFailedTitle` i18n key **stay** (compat surface; no binding regen for a rename). CONTEXT.md's *Avoid* line becomes "in new names; the wire code `DELETE_CLEANUP_FAILED` predates the term". |
| Q3 | **Notification history persistence**: **session-only**, in memory, bounded ring of the last **100** entries. Post-restart forensics is the backend log, reachable via "Open log folder" (Q5). Persistence (DB table or JSONL) is explicitly a possible later ticket, not this round. |
| Q4 | **Notification surface**: a bell icon in the app header with an unread-count badge; clicking opens a panel through the existing Modal shell (no native dialogs). Entries: kind (`error`/`warning`/`success`/`info`), title, message, timestamp; per-entry and whole-list copy-to-clipboard; "Clear". Errors and warnings count as unread until the panel is opened. **Every** user-visible notification goes through `useStatusReporter` — the 24 direct `toast.*` calls outside it (`AssignmentMatrix`, `ProjectsPage`, `SkillCard`, `SkillDetailView`, `useUpdateChecker`) are routed through the reporter, which gains `notify(kind, title, message?)`; every notification is *recorded* through the reporter: `notify` toasts and records, `showActionErrors` toasts once and records each. |
| Q5 | **Toast lifetime**: error toasts `duration: Infinity` + `closeButton`; warning 5 s; success/info 2 s. One place (the reporter) owns durations; `<Toaster toastOptions>` in `App.tsx` carries no per-kind duration. Ships **first**, by hand on `main`, before anything else. "Open log folder" (Settings action → `tauri-plugin-log` `LogDir` via the opener plugin) ships in the same ticket. |
| Q6 | **Double fetch on Add for non-GitHub hosts (#11)** is **in** the round: the listing's full clone and the install's sparse clone share one `git_cache` key so the second call is a cache hit within TTL. Perf-only; must not change what bytes land or the recorded SHA. **Ruling (ticket 09):** within the TTL an install records the listing's snapshot (real SHA, that snapshot's bytes) instead of re-fetching — accepted; that is the cache working. |
| Q7 | **Version for the round: `1.2.3`** via `npm run version:set 1.2.3` in the last ticket, after ticket 08 makes `version:set` rewrite both lockfiles. |
| Q8 | **Removal report error chain (#2)**: `RemovalTargetStatus::Failed` carries `anyhow::Error` (as Propagation's report does), so typed `SignalError`s survive to the command seam and `from_anyhow` classifies them; the `anyhow!("{}", error)` revival in `commands/mod.rs` goes away. Cloning/`Debug` needs of the report are met the way `propagation.rs` meets them. |
| Q9 | **Dead seams (#3)**: callerless `*_unlocked` twins and `skill_store::delete_skill_targets` / `delete_all_skill_targets` are **deleted**, not kept behind `#[allow(dead_code)]`; the ADR-0002 argument (blind bulk deletes are forbidden) is the reason. Stale `// Used in Phase 2` comments go with them. |
| Q10 | **One naming rule for project artifacts (#5)**: one core helper answers "which name locates this assignment's artifact on disk"; `artifact_removal`, `propagation` and `project_sync` all call it. The rule is the stored `assignment.skill_name` (what was materialised), never the live `skill.name` — the ticket documents that choice in the helper's doc comment and adds the rename-divergence test. |
| Q11 | **`sync_single_assignment` vs `propagate_one_assignment` (#6)**: extract the shared "sync one assignment + `SyncCompleted`" step into one function taking the hash rule as an explicit parameter; the two callers pass theirs. If the hash rules turn out to be genuinely equivalent, collapse to one rule and say so in the ticket comment. |
| Q12 | **Frontend project world (#7, #8)**: state derived from the selected project is set outside the `setSelectedProjectId` updater (effect or explicit sequence — no side effects in updaters); `ProjectsPage` project removal gets the same failure-path `refreshView` as toggle/bulk/configure. |
| Q13 | **Docs/tooling (#9, #12, #13)**: AGENTS.md's "Onboarding import propagates" sentence corrected to "syncs through the global sync batch" (code is the intent — Propagation is for *updates* to an already-managed skill; import is a first sync); ticket-12 comment fixed; `version:set` rewrites `package-lock.json` root `version` and `src-tauri/Cargo.lock`'s `app` entry, `version:check` verifies both. |
| Q14 | **Execution**: every ticket delegated to a Fable 5.1 child, one ticket per child, one worktree each; 01→02 and 03→04 are the only edges; 05, 06, 07, 08 start immediately; 09 last. The operator owns the `tauri:dev` smoke test of the notification UX (real skill library) and the Release confirmation. Merge ritual, gate and model routing as in round 2 (`npm run version:check && npm run check`; rebase → diff → gate → `--ff-only` → deletion check). Branch `r3/<NN>-<slug>`. |
| Q15 | End-of-round 3-model review panel (Standards + Spec axes) before the version bump, same brief shape as round 2. |

## Facts verified at HEAD (so tickets don't re-derive them)

- `useAddSkillFlow.ts:419` passes `tools: autoSyncEnabled ? getSelectedInstalledIds() : null` — the
  *global* auto-sync selection, so #10 is reachable whenever the found-in Tool is deselected globally.
  `onboarding_import.rs:267-303` builds the target list from `policy.tools` and only adds a per-(skill,
  tool) overwrite override when the source Tool is already in it.
- Toast call sites: `useStatusReporter.ts` (error 3200 / success 1800 / error 2600),
  `App.tsx:194` `<Toaster toastOptions={{ duration: 1800 }}>`, and 24 direct `toast.*` calls in
  `components/projects/AssignmentMatrix.tsx`, `components/projects/ProjectsPage.tsx`,
  `components/skills/SkillCard.tsx`, `components/skills/SkillDetailView.tsx`, `hooks/useUpdateChecker.ts`.
  The reporter exposes `setError`, `setSuccessToastMessage`, `showActionErrors` — no warning/info entry
  point and no record of anything shown.
- `DELETE_CLEANUP_FAILED` is wired end to end (`errors.rs:55`, `commands/error.rs:97/219`,
  `commandError.ts:30/122`); renaming it would touch the generated binding and the ADR-0001 procedure —
  hence Q2 keeps it.
- The registry fn is `tool_adapters::adapters_sharing_skills_dir(&ToolAdapter)` (`mod.rs:642`); the two
  inline copies are `propagation.rs:~182` and `global_sync.rs:~365`.
- `RemovalTargetStatus::Failed { error: String }` (`artifact_removal.rs:~156`) is revived at
  `commands/mod.rs:~539` as `CommandError::from_anyhow(anyhow!("{}", error))` → always `OTHER`.
- The backend log goes to `tauri-plugin-log` `LogDir` (`lib.rs:75`); the app already depends on the
  opener plugin (used for "open in file manager" actions) — ticket 01 reuses it.

## Ticket map

```
01 toast lifetime + reporter notify() + open log folder
   └─► 02 notification history panel + every toast through the reporter
03 removal error chain (#2) + dead seams (#3) + Artifact-removal names (#4)
   └─► 04 shared-dir predicate (#1) + artifact naming rule (#5) + sync-one (#6)
05 frontend project world (#7, #8)                         (parallel, any time)
06 docs drift (#9, #12) + glossary line (Q2) + version tooling (#13)   (parallel, any time)
07 import force-includes the source Tool (#10)             (parallel, any time)
08 one clone for listing + install on non-GitHub hosts (#11)   (parallel, any time)
09 review panel + version:set 1.2.3 + Release ◄── everything
```

Work the frontier: lowest-numbered open ticket whose blockers are resolved. First wave: 01, 03, 05,
06, 07, 08 in parallel worktrees. Every ticket is delegated (Fable 5.1); the operator owns the final
smoke test and the Release confirmation.

## Closure — 2026-09-15

Shipped: v1.2.3 / f057190 (tag verified; Release run 33945623532 succeeded). Tickets: 12 terminal (10 done, 1 superseded, +09 done on release-run confirmation), 0 still open (none).
Residue → BACKLOG: #21. Dropped by name: none.
