# 01 — Augment global skills dir is `.augment/skills` (#43)

Status: ready-for-agent
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
