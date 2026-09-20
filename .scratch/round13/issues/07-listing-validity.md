# 07 Align local listing validity with installability

Status: open
Lane: C
Source: BACKLOG #11

`src-tauri/src/core/skill_discovery.rs` lines 125, 241, 292 admit `is_claude_skill_dir` (no SKILL.md manifest) while install (`installer::ensure_installable_skill_dir`) requires a manifest. Align: a listing candidate is `valid` only if install would accept it; keep `is_claude_skill_dir` dirs *listed* but marked invalid with the existing invalid-reason mechanism (read how invalid candidates are surfaced today before changing). TDD: failing test in `core/tests/skill_discovery.rs` first. Check the frontend does not assume valid ⇔ listed.

## Done when

Test proves a manifest-less dir is listed-but-invalid; install path unchanged; `cargo test skill_discovery` green.

## Comments
