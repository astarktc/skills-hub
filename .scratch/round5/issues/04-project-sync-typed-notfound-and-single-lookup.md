# 04: No panic on a command path; one adapter lookup per global row

**What to build:** (1) `project_sync::sync_assignment_target` returns the typed not-found error when a live skill name does not locate its artifact instead of `expect`-panicking (a panic inside `spawn_blocking` reaches the operator as an opaque internal error). (2) The global-rows Propagation path in `propagation.rs` resolves each row's adapter once and reuses it for the sharing-set computation. Behaviour otherwise identical.

**Blocked by:** None (can start immediately)

**Status:** implemented (approved test deviation below)

- [ ] Core test: the not-found path returns the typed error (construct the row so the name lookup fails) — no `expect` remains on that path
- [x] Propagation tests unchanged and green; a grep shows one `adapter_by_key` per global row iteration
- [x] `cargo test --all`, clippy clean; `npm run version:check && npm run check` green

## Orchestrator notes

- `project_sync.rs:116` — `.expect("a live skill name always locates the artifact")`; the typed condition already exists (look for the `NotFound`-style `SignalError` used by neighbouring lookups).
- `propagation.rs:165–189` — `adapter_by_key(&row.tool)` then `adapters_sharing_skills_dir(adapter)` and a second `adapter_by_key` in the filter at ~189; the comment at ~180 explains the test-shadow subtlety — preserve that behaviour.
- Both files are on AGENTS.md's high-risk shared list: minimal hunks, no reformatting.

## Comments

- Shipped in `7c13cee` (`fix(core): guard assignment lookup and reuse propagation adapters`): replaced the assignment artifact `expect` with `SignalError::NotFound { kind: "skill", id: assignment.skill_id }`; resolved each global target row's adapter once and reused it for group membership and sync policy. Preserved the explicit own-tool membership rule and test-shadow behavior. No tests or name-resolution behavior changed.
- Approved deviation: the live-name fallback returns `Ok(Some(ctx.skill.name.clone()))`, so successful name resolution is true by construction; the expect is replaced with a typed error so a future caller change cannot panic on a command path. No constructed row can exercise the `None` branch today, even with an empty name or deleted database row. The operator approved defensive replacement without fabricating an unreachable-state test. The first acceptance box is intentionally unticked; its no-`expect` portion is satisfied.
- Gates: `npm run version:check && npm run check` passed (version 1.2.4; ESLint, 13 Vitest files / 214 tests, TypeScript-7 + Vite build, rustfmt, clippy with warnings denied, 517 Rust tests). Separate `cargo test --all` passed: 517 tests, zero failures. All 10 existing Propagation tests passed unchanged, including shared-dir and test-shadow coverage. Targeted LSP diagnostics reported zero errors. `git diff --check` passed. Grep of the global-row function shows one executable `adapter_by_key` call in its row-resolution map, with none in the group filter.
- Local gate output: `.scratch/round5/ticket04-check.log` and `.scratch/round5/ticket04-cargo-all.log` (not committed).
- Follow-ups: none for this ticket. If a future caller can supply an absent live name, add the typed-error regression test at that point. `npm ci` reported 7 dependency vulnerabilities (1 moderate, 6 high); dependency changes are outside this ticket's scope.
