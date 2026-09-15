## Standards

No hard documented-standard breach identified in the reviewed changes. The three new errors have SignalError → CommandError → describeCommandError → EN/ZH plumbing; formatter props were removed; import uses the existing unlocked batch under its owning guard.

- **S1 — Follow-up, possible Duplicated Code:** `src-tauri/src/core/refresh.rs::parse_repoint_source` independently interprets GitHub path structure (`parts[2] != "tree"`) before calling `git_acquisition::parse_github_url`, whose corresponding rule accepts `"tree" || "blob"`. These two owners already disagree (P1). Put validated GitHub URL interpretation beside the existing parser, retaining an explicit full-URL policy for Re-point.
- **S2 — Follow-up, stale glossary:** `CONTEXT.md`, **Skill discovery**, still says “one dedup by subpath”; `skill_discovery.rs` now adds canonical-directory identity (`kept.iter_mut().find(|(c, _, _)| *c == canonical)`). Document alias collapse and the preferred real-directory subpath so the canonical vocabulary describes the new behavior.

## Spec

- **P1 — Blocking: valid Add URLs refused by Re-point.** Q12/Q13 requires a “full GitHub URL parsed by the existing GitHub URL parser.” In `refresh.rs::parse_repoint_source`, `parts[2] != "tree"` rejects `https://github.com/owner/repo/blob/main/skills/foo/SKILL.md`. The existing `git_acquisition.rs::parse_github_url` explicitly accepts `blob` and normalizes the manifest URL to its containing skill folder. This is not malformed input; the added validator narrows the agreed parser contract. Ticket 01's Comments acknowledge tree-only validation but claim no feature-scope deviation. Accept the existing parser's full GitHub skill-link shapes and pin this regression.

- **P2 — Blocking: finalize failure can strand the repaired skill.** Ticket 01 promises “On any failure … the record is untouched”; story 2 says a typo must never strand a working skill. The new `installer::acquire_managed_skill_update_from` correctly keeps the override in memory during acquisition, but its Refresh apply path calls existing `install_finalize.rs::finalize_update`: `remove_dir_all(&central_path)` → `staged.move_into(&central_path)?` → `store.upsert_skill(&updated)?`. A failed move/copy can leave central missing after deleting the working copy; a failed database write leaves replacement bytes paired with the old source. This is inherited finalizer behavior newly used by Re-point, not an acquisition-first violation. Preserve a rollback copy through the filesystem/database settlement and test injected move/upsert failures.

- **P3 — Follow-up: historical action retains stale skill data.** Ticket 07 says the action opens the Modal “for that skill.” `useSkillLibrary.ts` stores `onClick: () => handleRepointGitSkill(managedSkill)` in session history. Removing that skill later does not invalidate the action: clicking it opens a repair form for a deleted row and confirmation returns not-found. IDs prevent targeting another skill, but resolve the current row by ID at click time and gracefully refuse stale history actions.

## Summary

**Blocking:** P1 (URL compatibility), P2 (finalize data safety).

**Follow-up:** S1 (duplicate parsing), S2 (discovery glossary), P3 (stale notification action).

Reviewed `ed630c0…378a473`, including `6cd91cf`, by source and test inspection. No tests, builds, app launches, or library mutations were performed. The checkout advanced to `cbc3896` during review; these findings concern the requested `378a473` snapshot. Only this report was written.
