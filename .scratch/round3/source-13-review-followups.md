# 13 — Round-2 review follow-ups (non-blocking)

**Status:** triaged → round 3 (`spec.md`); this file is a read-only source reference
**Source:** round-level review of `af71598..64c19be` by a 3-model panel (Fable 5.1 / Opus 5 / GPT-5.6 Sol,
2026-09-03). The blocking findings shipped as `fix/review-a|b|c` before v1.2.2. Everything below was
judged real but not release-blocking; each item is verified at HEAD by the orchestrator.

## Judgement-call standards findings

1. **Shared-skills-dir predicate has three copies** — `propagation.rs:~182`
   `.filter(|(_, a)| a.relative_skills_dir == adapter.relative_skills_dir)`, `global_sync.rs:~365`, and the
   registry's `tool_adapters::adapters_sharing_skills_dir` (`mod.rs:~642`). CONTEXT.md says the group is owned
   by the registry. → Make the other two call the registry fn.
2. **`RemovalTargetStatus::Failed { error: String }` flattens the error chain** (`artifact_removal.rs:~156`);
   `commands/mod.rs:~539` revives it as `CommandError::from_anyhow(anyhow!("{}", error))` → always `OTHER`.
   `propagation.rs` keeps `anyhow::Error` in its report and maps typed variants. Two sibling report modules
   disagree on the same question; ticket 03 deviation 4 stated "unclassified IO" as the reason. → Keep
   `anyhow::Error` in the report so typed signals survive to the seam.
3. **Callerless `*_unlocked` twins** — `project_ops::remove_project_with_cleanup_unlocked` (~289),
   `project_sync::assign_skill_to_project_tools_unlocked` (~207), `project_sync::resync_all_projects_unlocked`
   (~349); plus `skill_store::delete_skill_targets` / `delete_all_skill_targets` (~707/716, `#[allow(dead_code)]`,
   and blind bulk deletes ADR-0002 forbids). Ticket 01 said "collapse if unused". → Collapse.
   Also stale `// Used in Phase 2` comments at `skill_store.rs:~1073,1113`.
4. **Glossary drift** — CONTEXT.md *Artifact removal* says avoid "cleanup / unsync / unassign", yet the entry
   points are `unsync_skill_targets`, `unassign_and_cleanup`, `remove_tool_with_cleanup`, `DeleteCleanupFailed`,
   `errors.importCleanupFailedTitle`. → Either rename (wire-visible: `DELETE_CLEANUP_FAILED` is a CommandError
   code) or soften the glossary's *Avoid* list to "in new names".
5. **Two rules for "which name locates a project artifact"** — `artifact_removal.rs:~369-380` plans from the
   stored `assignment.skill_name`; `propagation.rs:~369` and `project_sync.rs:~65/282/408` from live
   `skill.name`. Equal today (finalize never renames); diverge the day it does. → One helper, one rule.
6. **`sync_single_assignment` (`project_sync.rs:~257-292`) vs `propagate_one_assignment`
   (`propagation.rs:~345-407`)** — same "sync one assignment + `SyncCompleted`" shape with two hash rules
   (`hash_after_sync` vs `can_drift ? content_hash : None`). → Extract, or document why the rules differ.
7. **`useProjectState.ts:~142-149`** sets `tools/assignments/assignmentsReconciled` inside a
   `setSelectedProjectId` updater — side effects in an updater; double-fires under StrictMode. → Move out.
8. **`ProjectsPage.tsx:~56-66`** has no failure-path `refreshView` after `DELETE_CLEANUP_FAILED` on project
   removal, unlike toggle/bulk/configure — the `error` assignment rows ADR-0002 promises stay invisible until
   reselect. → Add the failure-path refresh.
9. **AGENTS.md says Onboarding import "propagates"** — the code uses the sync batch
   (`onboarding_import.rs:~298 sync_skills_to_tools_unlocked`), not the Propagation module. → Fix the sentence
   (or route through Propagation if that is the intent).

## Spec / product questions

10. **Onboarding import, auto-sync on, source Tool deselected** — `onboarding_import.rs:~270-298` only
    force-overwrites the source Tool if it is already in `policy.tools`; if the operator excludes it, the
    original stays as an untracked copy. Ticket 06: "the chosen variant's own Tool is overwritten in place".
    → Decide: force-include the source Tool, or confirm the UI never allows deselecting it.
11. **Double fetch on Add for non-GitHub hosts** — the listing full-clones, then install sparse-clones under a
    different `git_cache` key (ticket 09 deviation 4). Perf only. → Consider reusing the listing's clone.
12. **Ticket 12 comment still says `c923c50 release: v1.3.0`** — shipped as `64c19be release: v1.2.2`.

## Panel verdict on the ticket-02 deviation

All three reviewers independently ruled that re-materialising a drifting **copy** as a **link** on a
symlink-capable Tool **follows from** the spec (capability-aware entry point is the only way bytes reach a
target; a copy was never an operator choice, only a fallback; the row records the mode actually used).
Closed — no change.

## Tooling

13. **`scripts/version.mjs` does not touch either lockfile.** `src-tauri/Cargo.lock`'s `app` entry only
    changes when cargo next runs (so the first `cargo test` after a bump dirties the tree — bit us on
    `64c19be`, fixed by `c2a4592`), and `package-lock.json`'s root `version` has sat at `1.1.9` for
    several releases. → Make `version:set` rewrite both, and `version:check` verify them.
