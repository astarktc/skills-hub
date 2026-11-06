# Round 20 ticket 01 (registry structure and relabels) — adversarial review

## Round 1 — GPT-6 Astra (high), 2026-11-06, `6bfc0c0a` vs `main`: **fix-then-ship**

No P1 finding. All 45 existing rows keep their global, former, detect and project paths, and their grouping, symlink
capability and `as_key`. The reviewer confirmed this with an independent baseline-vs-head comparison.

1. **P2, a detected reader-only harness can't receive its first global deployment.** Global installedness of
   `agents_skills` checked only `~/.agents`. With a reader detected but no `~/.agents` yet:
   - the detected-only filter hides the group;
   - selecting the group skips it with `TOOL_NOT_INSTALLED`.

   The project catalog already counted readers (`catalog.rs:130`).

**Deviations accepted:**

- the field renames;
- backend-formatted display paths;
- the README project-dir column;
- the separate `READER_ONLY_HARNESSES` table;
- the frontend-derived group count.

**Sound:**

- path resolution, including Windows separators;
- root discipline (`dirs::*` only in `environment.rs`);
- relocation, onboarding, removal scopes and Propagation;
- `None` project dirs, which never fall back to the project root;
- reader isolation;
- the relabels and the deferred detect dirs;
- the bindings and shims;
- independent-literal snapshot tests and the README test.

## Disposition

1. Fixed by the orchestrator in `7a02003`: `is_installed_in` counts a group as installed when any of its reader-only
   harnesses is detected. Regressions, both red before the fix:
   - `a_detected_reader_makes_its_group_deployable_before_the_group_folder_exists` (global sync);
   - the extended `a_reader_only_harness_is_listed_under_the_agents_entry_never_as_a_target` (catalog).

Gate green (Rust 938), CI green. Merged `8401dfa`.
