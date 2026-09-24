# 01 — Onboarding scan honours "only scan selected tools" (D1)

Status: ready-for-agent
Spec: `.scratch/round17/spec.md` — D1.

## Work

- `core/onboarding.rs`: `pub enum OnboardingScanScope { Installed, Selected(Vec<String>) }`.
  `build_onboarding_plan(home, central_dir, store, scope)`; `build_onboarding_plan_in_home` takes the scope and
  filters adapters by it (`Installed` → `is_installed_in`; `Selected(keys)` → `keys.contains(adapter.key())`,
  no installedness check — an absent skills dir scans to nothing). `total_tools_scanned` = adapters in scope.
- `commands/mod.rs::get_onboarding_plan`: read `load_settings`'s `scan_selected_tools_only` and
  `global_selected_tools`; `Some(keys)` with the flag on → `Selected(keys)`, else `Installed`. A corrupt selection
  reads as `None` there → `Installed` (display shape; the sync path refuses separately).
- Tests (`core/tests/onboarding.rs`): existing tests pass `Installed`; new: `Selected` skips an installed,
  unselected tool and scans a selected one; `total_tools_scanned` matches the scope.

## Comments
