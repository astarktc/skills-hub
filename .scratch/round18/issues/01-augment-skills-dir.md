# 01 — Augment global skills dir is `.augment/skills` (#43)

Status: implemented
Spec: `.scratch/round18/spec.md` — ticket 01.

## Work

- `core/tool_adapters/mod.rs:292`: `relative_skills_dir: ".augment/rules"` → `".augment/skills"`; fix the comment.
  Detect dir `.augment` and project dir `.augment/skills` stay. **Verify against the vendor first**: fetch Augment's
  current docs (both the docs site and the CLI/README if they differ — Kimi taught us vendors disagree with
  themselves) and record the URL(s) in the ticket comment; if the vendor genuinely ships `rules`, stop and report.
- Legacy targets: a skill previously synced to `~/.augment/rules/<skill>` has a target row pointing there. Decide
  and implement one of: (a) reconcile reports it as unlocatable and Refresh re-materialises to the new dir (rows
  updated by Propagation); (b) a one-time migration. Prefer (a) if the existing reconcile already yields it —
  prove with a test, don't assume.
- README supported-tools table row for Augment; `project_relative_skills_dir_for_every_tool` table test still
  passes; add a test pinning the global dir.
- `cargo test --all`; bindings unchanged (no wire change).

## Result

**User-facing one-liner (for CHANGELOG, ticket 10):** Augment skills now sync to `~/.augment/skills` (Augment's
documented skills dir) instead of `~/.augment/rules` (its rules dir); skills already synced to the old location
are moved automatically on first launch.

### Vendor verification (2026-09-27)
- https://docs.augmentcode.com/cli/skills, section "Skill Locations": `~/.augment/skills/` (User, highest precedence),
  `<workspace>/.augment/skills/`, plus `.claude/skills` / `.agents/skills` compatibility dirs. Rules are a separate feature.
- https://docs.augmentcode.com/using-augment/skills (IDE docs, same `.augment/skills/<name>/SKILL.md` layout) and
  https://docs.augmentcode.com/cli/reference ("loaded automatically from `.augment/skills/` … in both your workspace
  and home directory").
- add-skill (vercel-labs/skills) README agent table: `augment` | `.augment/skills/` | `~/.augment/skills/`.
- The docs site and CLI docs agree. No source says `rules`.

### Legacy targets: option (b), proven necessary
Option (a) is **not** yielded by existing code, and a test proves it
(`propagation_alone_leaves_a_legacy_link_row_in_the_former_dir`): Propagation writes to the row's *stored*
`target_path` and skips link rows outright (`LinkFollowsSource`). No reconcile pass runs on global targets, and
global sync re-records a pair only if the operator syncs it again, which orphans the old artifact. Removal's
uninstalled-tool fallback would also have refused the legacy path (`PATH_OUTSIDE_TOOL_DIRS`).

The fix is a one-time, idempotent startup relocation driven by a new registry fact:
- `ToolAdapter.former_relative_skills_dirs` is on every registry literal: Augment `[".augment/rules"]`, every other
  tool `[]`. The fact lives in the literal, per the AGENTS.md adapter invariant, and there is a helper `former_skills_dirs_in`.
- The deletion rule `ensure_path_within_tool_dirs` also accepts former dirs, so a row left there stays removable.
- New `core/target_relocation.rs` → `relocate_former_global_targets(store, home, now)` (under the mutation guard;
  called once in `lib.rs` setup after legacy reclassification). It acts on each global row whose artifact sits
  *directly* in a former dir:
  - If the old artifact is present, the skill is materialised in the current dir from the central copy through
    `sync_dir_for_tool_with_overwrite`. An existing target there is replaced only if its content is identical;
    foreign bytes leave the row in place, logged and retried next launch.
  - The row is settled with `SyncCompleted` at the new path.
  - The old artifact is removed through the new fenced seam
    `artifact_removal::remove_superseded_artifact_unlocked`, which refuses anything whose parent is not a former dir.
  - An absent old artifact or a missing central copy leaves the row in place. Nothing is materialised for a tool
    that may be gone.
- A legacy *copy* is re-materialised from the central copy as a link, the same behaviour Propagation has, so local
  drift in a copy under `~/.augment/rules` is not preserved.
- Project scope was already `.augment/skills`, so nothing changes there.

### Files touched
- `src-tauri/src/core/tool_adapters/mod.rs`: Augment dir and comment, the new field on all 45 literals,
  `former_skills_dirs_in`, the deletion-rule extension.
- `src-tauri/src/core/target_relocation.rs` (new) and `src-tauri/src/core/tests/target_relocation.rs` (new, 11 tests).
- `src-tauri/src/core/artifact_removal.rs`: `remove_superseded_artifact_unlocked`.
- `src-tauri/src/core/mod.rs`: `pub mod target_relocation;`.
- `src-tauri/src/lib.rs`: the startup call.
- `src-tauri/src/core/tests/tool_adapters.rs`: the field on the 2 test literals; tests
  `augment_global_skills_dir_is_augment_skills_and_rules_is_former`, `no_former_skills_dir_is_a_current_skills_dir`,
  `a_path_inside_a_former_tool_skills_dir_is_allowed`.
- `README.md`: Augment row only.

### Verification
- `cargo test --all`: 683 passed, 0 failed, including `project_relative_skills_dir_for_every_tool` and the 11
  relocation tests. `cargo fmt --check` and clippy are clean.
- `npm run version:check` is OK. Within `npm run check`, lint and vitest (407) pass. **`npm run build` fails** in
  `src/commandError.ts` (`CREDENTIAL_STORE_UNAVAILABLE` missing from the code record). That comes from ticket 02's
  in-progress credential work (`core/credentials.rs`, `errors.rs`, bindings), not this ticket. It was still failing
  after the ~2-minute wait and re-run.
- No wire change from this ticket: it adds no specta types or commands. The current `src/bindings/index.ts` diff
  belongs to ticket 02.

### Open questions
- There's no CONTEXT.md entry for "Former skills dir" / relocation. Worth adding if the orchestrator treats it as a
  domain term.
- Historical `docs/releases/v0.1-v0.2/system-design*.md` still say `.augment/rules`. They are frozen release docs
  and were left untouched.
