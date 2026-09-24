# 02 — A skills-only footprint is not an installed tool (D2)

Status: done — (this commit)
Spec: `.scratch/round17/spec.md` — D2.

## Work

- `core/tool_adapters/mod.rs::is_installed_in`: detect dir exists && (virtual-group entry || !skills-only
  footprint). Private `is_skills_only_footprint(detect_dir, skills_dir)`: `strip_prefix` the detect dir off the
  skills dir (every registry entry nests its skills dir under its detect dir — assert this in a table test);
  walk the components; at each level `read_dir` must yield exactly one entry named the next component. Any
  read error → not a footprint (installed), so a permissions hiccup never hides a tool.
- Doc comment on `is_installed_in` states the rule and the group exemption.
- Tests (`core/tests/tool_adapters.rs`): extend `installedness_is_decided_by_detect_dir_not_skills_dir` —
  `.kiro/skills/x` alone → not installed; add a sibling file → installed; `.pi/agent/skills/x` alone → not
  installed, `.pi/agent/settings.json` beside it → installed; `.agents/skills/x` alone → installed (group
  exemption); every adapter's skills dir is under its detect dir. Catalog tests (`tool_catalog.rs`) that build
  fixtures with `install()` (empty detect dir) keep passing unchanged.
- Existing onboarding fixtures (`.cursor/skills/foo` only) become footprints: give each fixture tool a marker file
  beside `skills/` so the test states "installed tool with an unmanaged skill" honestly.

## Comments

- 2026-09-23 — `is_installed_in` = detect dir exists && (virtual group || !skills-only footprint); `.DS_Store` ignored on the walk; read errors read as installed. Test-only `tool_adapters::mark_installed_in` installs a tool honestly (detect dir + `installed.marker`) and replaces every fixture's bare `create_dir_all(detect_dir)` (10 test files). New tests: footprint, every-level walk (`.pi/agent`), group exemption, every adapter nests skills under detect. cargo 668, clippy clean.
