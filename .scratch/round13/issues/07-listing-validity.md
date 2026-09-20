# 07 Align local listing validity with installability

Status: done — pending parent commit
Lane: C
Source: BACKLOG #11

`src-tauri/src/core/skill_discovery.rs` lines 125, 241, 292 admit `is_claude_skill_dir` (no SKILL.md manifest) while install (`installer::ensure_installable_skill_dir`) requires a manifest. Align: a listing candidate is `valid` only if install would accept it; keep `is_claude_skill_dir` dirs *listed* but marked invalid with the existing invalid-reason mechanism (read how invalid candidates are surfaced today before changing). TDD: failing test in `core/tests/skill_discovery.rs` first. Check the frontend does not assume valid ⇔ listed.

## Done when

Test proves a manifest-less dir is listed-but-invalid; install path unchanged; `cargo test skill_discovery` green.

## Comments

- 2026-09-16 (lane C child): TDD. Red: added `claude_skills_child_without_skill_md_is_listed_but_invalid` (renamed
  from `..._is_valid_with_plugin_description`, now asserts `Validity::MissingSkillMd`, reason `missing_skill_md`,
  `!is_valid()`, `!is_installable()`, and that `require_skill_md` refuses the same dir with the same token) and
  `listing_rooted_at_claude_skills_lists_manifestless_children_as_invalid` (root = `.claude/skills`: `bare` listed
  invalid, `good` valid) in `core/tests/skill_discovery.rs` — both failed with `left: Valid, right: MissingSkillMd`.
  Green: `core/skill_discovery.rs::inspect`'s `None if is_claude_skill_dir(dir)` arm now yields
  `Validity::MissingSkillMd` (keeps the plugin.json description fallback); module/`Validity`/`is_skill_dir`/
  `is_claude_skill_dir` docs updated. Lines 125/292 (`is_claude_skill_dir` in root check and `is_skill_dir`) untouched
  so such dirs stay *listed*; `require_skill_md`, `ensure_installable_skill_dir` and every install door unchanged.
  Frontend: `LocalPickModal.tsx` already disables `!c.valid` rows and maps `missing_skill_md` to
  `localSkillInvalid.missingSkillMd`; `useAddSkillFlow` gates auto-select on `valid` — no DTO/wire/frontend change,
  `src/bindings/index.ts` unmodified.
  Out-of-lane touch (flagged): the git listing admits by `is_installable`, so `core/tests/installer.rs::
  listing_interface_characterization` case `malformed-missing-and-claude-exception` encoded the misalignment (offered
  `.claude/skills/optional`, which `install_git_skill_from_selection` refuses with `missing_skill_md` because the
  landed staging dir has no `.claude/skills` parent). Renamed to `malformed-and-missing`, dropped the `optional` row,
  fixture kept to prove exclusion.
  Evidence: `cargo test skill_discovery` → `test result: ok. 23 passed`; `cargo test git_candidates` → `2 passed`.


- 2026-09-16 (parent, post-review) — Spec review (Astra) flagged that the git listing now *omits* a manifest-less
  `.claude/skills/` child rather than listing it invalid. Requirement narrowed deliberately: the git listing is by
  design the installable set (AGENTS.md: listing uses acquisition's candidate admission; `is_installable` was already
  its rule), and install has always refused such a dir — the exception was the only path by which an uninstallable
  candidate reached the git picker. "Listed but invalid" applies to the local picker, which has the validity/reason
  DTO fields; the git DTO has none and gains none. CONTEXT.md **Skill candidate** updated to match; CHANGELOG says so.
