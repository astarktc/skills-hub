# 05 — Onboarding plan carries `name_is_managed`; `remove_originals` is a per-run input; `SKILL.md`-gated scan

Status: ready-for-agent
Blocked by: —
Spec: `.scratch/round19/spec.md` — D13, D14 (grill Q11, Q12). Absorbs BACKLOG **#46**. Source: `flows.md` Flow 3.

## Work

- **Plan.** `OnboardingPlan` groups gain `name_is_managed: bool` (and the managed skill's id when true) so the review
  can start them unchecked and offer Replace/Leave (`core/onboarding.rs`). Counts: the plan exposes both name-group
  count and copy count; the frontend stops deriving one from the other.
- **`SKILL.md`-gated scan (#46).** `tool_adapters::scan_tool_dir` lists every subdirectory today
  (`tool_adapters/mod.rs` ~888–926); it now admits only directories whose `SKILL.md` parses to a valid manifest,
  and reports the invalid ones as a separate list with a typed reason (`missing_skill_md`, `missing_name`, …) so the
  review can show them disabled with the reason in words. Re-verify the line numbers against the current file.
- **`remove_originals: bool` on `import_onboarding_selection`** (`#[serde(default)]` is wrong here — make it required
  so the frontend must decide). Auto-sync on: byte-identical originals are taken over by the sync batch as today;
  auto-sync off: originals removed **only when `remove_originals` is true**, otherwise left and reported as
  `left_in_place` rows per Tool. The report lists removed originals per Tool in either mode.
- **Managed-name groups**: importing one with `replace: true` routes the Tool's copy through the same `TARGET_EXISTS`
  path ticket 04 makes askable (the managed skill is synced over the copy with an Overwrite override); with no
  decision it is skipped with a typed reason, never the current `already exists — go to My Skills` failure.
- Bindings regenerated; shims re-export; the existing `ImportModal` gets the minimum to compile (pass
  `remove_originals` = current policy; hide nothing) — the review UI is ticket 21.
- **Fixture**: `first-run` seeds two managed-name groups, one invalid manifest, `remove_originals` honoured.
- **Tests**: gated scan admits/reports; managed-name flag; `remove_originals` both values × both modes; replace path
  produces an askable row.
- Docs: CONTEXT.md § Onboarding import (the per-run choice; managed-name groups); CHANGELOG.

## Review

Orchestrator diff read; Astra only if the removal step changes shape (it should not — it still pairs with
`ensure_path_within_tool_dirs`).

## Result

(child fills in)
