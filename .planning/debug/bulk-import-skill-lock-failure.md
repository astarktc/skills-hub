---
status: awaiting_human_verify
trigger: "bulk-import-skill-lock-failure: After adding try_enrich_from_skill_lock() to install_local_skill(), batch onboarding import of 52 skills fails"
created: 2026-04-09T00:00:00Z
updated: 2026-04-09T00:01:00Z
---

## Current Focus

hypothesis: CONFIRMED -- The handleImport loop in App.tsx calls import_existing_skill WITHOUT a per-skill try-catch. When any skill fails, the entire batch aborts via the outer catch. Earlier skills in the batch have already been imported (copied + DB) and their originals removed. UI never refreshes because loadManagedSkills is after the loop. The user sees 0 skills.
test: Verified DB has 48 skills, .skillshub has 48 dirs, .claude/skills has 8 remaining symlinks (the unprocessed tail)
expecting: Fix by wrapping import_existing_skill in per-skill try-catch, collecting errors instead of aborting
next_action: Implement fix -- wrap import_existing_skill in try-catch within the handleImport group loop

## Symptoms

expected: All 52 skills imported via onboarding UI should be copied to ~/.skillshub/, registered in SQLite DB, and appear in the UI with enriched git provenance from ~/.agents/.skill-lock.json
actual: 48 of 52 skills copied to ~/.skillshub/ but NONE registered in DB/UI. Original symlinks deleted from ~/.claude/skills/. SKILL_INVALID|missing_skill_md errors appear because the symlinks are gone. App shows 52 skills in onboarding (cached from initial load) but can't import them.
errors: SKILL_INVALID|missing_skill_md -- occurs because symlinks were deleted from tool dirs before DB registration completed
reproduction: Open Skills Hub app, trigger onboarding import for ~52 discovered skills at once. Works for small batches (1-3 skills).
started: Started immediately after deploying the skill_lock.rs enrichment (commits ee7740a and 74e6607).

## Eliminated

- hypothesis: try_enrich_from_skill_lock causes panics, errors, or performance issues that break the batch
  evidence: Function uses only .ok()? (Option early return) on every fallible call. Cannot panic. Returns Option, not Result. Purely read-only. Confirmed by reading full source.
  timestamp: 2026-04-09T00:00:30Z

- hypothesis: symlink deletion during sync causes subsequent skills to lose their source paths
  evidence: Each skill has a unique path in the tool directory (e.g. ~/.claude/skills/skill-A). Syncing/removing skill-A does not affect skill-B's path. The 8 unprocessed skills still have intact symlinks.
  timestamp: 2026-04-09T00:00:40Z

- hypothesis: Race condition with concurrent imports
  evidence: Frontend handleImport uses sequential await in a for loop. No concurrency. Each skill is processed fully before the next.
  timestamp: 2026-04-09T00:00:45Z

## Evidence

- timestamp: 2026-04-09T00:00:10Z
  checked: try_enrich_from_skill_lock source code
  found: All fallible operations use .ok()? returning Option::None. No unwrap(), no indexing, no panics possible. Function is purely read-only.
  implication: skill_lock.rs enrichment is NOT the cause of the batch failure.

- timestamp: 2026-04-09T00:00:20Z
  checked: handleImport flow in App.tsx lines 986-1095
  found: The import_existing_skill call at line 1002 is NOT wrapped in a per-skill try-catch. It is inside the outer try-catch at line 992. If ANY skill fails, the entire for-loop over plan.groups is aborted. loadManagedSkills() at line 1083 is never reached.
  implication: A single skill failure aborts the entire batch, leaving UI unrefreshed and remaining skills unprocessed.

- timestamp: 2026-04-09T00:00:50Z
  checked: Current filesystem state
  found: ~/.skillshub/ has exactly 48 skills. ~/.claude/skills/ has 8 symlinks. ~/.agents/skills/ has 56 skills. The 8 symlinks in .claude correspond exactly to the 8 skills NOT in .skillshub.
  implication: 48 skills were successfully processed before the batch failed. The 8 remaining were never reached.

- timestamp: 2026-04-09T00:00:55Z
  checked: SQLite database at ~/.local/share/com.skillshub.app/skills_hub.db
  found: skills table has 48 rows. skill_targets table has 0 rows (autoSyncEnabled was false).
  implication: Skills ARE registered in DB. The symptom "NONE registered in DB/UI" was misleading -- they ARE in DB, but the UI never refreshed because loadManagedSkills() was skipped when the error fired.

- timestamp: 2026-04-09T00:01:00Z
  checked: autoSyncEnabled path (commit 873d9e0)
  found: With autoSyncEnabled=false, the import flow calls remove_skill_source for each variant after successful import. This deletes originals from tool directories. Combined with the batch-abort bug, this creates a catastrophic state: originals deleted, but UI shows nothing.
  implication: The interaction of two features (batch-abort-on-error + original-removal) creates an unrecoverable state for the user.

## Resolution

root_cause: The handleImport function in App.tsx does not wrap the import_existing_skill IPC call in a per-skill try-catch. When one skill fails (e.g., SKILL_INVALID|missing_skill_md for a skill discovered during onboarding but lacking valid SKILL.md), the error propagates to the outer catch, aborting the entire batch. By that point, earlier skills have been imported AND their original symlinks removed (via sync or remove_skill_source). The UI never calls loadManagedSkills() because it comes after the batch loop. Result: skills exist in DB and .skillshub but UI shows nothing; originals are gone.

fix: Wrapped the import_existing_skill IPC call in handleImport (App.tsx) inside a per-skill try-catch. On failure, error is collected in collectedErrors and `continue` skips to the next group. The sync/cleanup block only executes if import succeeded. This ensures (a) remaining skills continue processing, (b) loadManagedSkills() is always reached, (c) errors are reported as toast, not batch-abort.
verification: tsc + vite build passes. ESLint passes (0 new errors). i18n key errors.importFailedTitle exists in both EN and ZH. formatErrorMessage is in scope. Awaiting human verify of actual batch import behavior.
files_changed: [src/App.tsx]
