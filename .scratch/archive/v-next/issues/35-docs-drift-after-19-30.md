# 35: Docs drift after tickets 19–30

Status: resolved

Type: task
Source: review #3 (verified) — [verdict](../assets/review-3/verdict.md): H3, H4, S4, S7, S8, S10

## What to build

Doc-only. No code changes.

### AGENTS.md
- `:95` — "Refresh data by re-invoking the relevant command (e.g. `invoke('get_managed_skills')`)" contradicts the t30 invariant six lines up ("raw `invoke` appears nowhere outside `src/bindings/index.ts`"). Rewrite as `invokeTauri("getManagedSkills")`.
- `:122` — heading "**Core never reads the environment.**" is literally false: six `std::env::var` reads live in core (`git_fetcher.rs:34,274,282,296`, `sync_engine.rs:221`, `install_finalize.rs:253` — the last relocated by t22). The body's actual rule (only `environment.rs` calls `dirs::home_dir()`; no `AppHandle`; explicit roots) holds exactly. Narrow the heading ("Core never resolves roots from the environment") and list the env-var *feature flags* as the sanctioned exception; add `skill_store::migrate_legacy_db_if_needed` (`dirs::data_dir()`) to the thin-adapter exception list unless ticket 34 removes it.

### CONTEXT.md / ticket 27 Answer — unknown lifecycle values
- CONTEXT.md **Sync status** ("an unrecognised value surfaces as `error` … never as healthy") and the t27 Answer ("never rewrites on read — a re-sync writes canonical strings") describe the **store seam** only. The list command's reconcile pass (`project_sync::reconcile_assignment` → `next_status`) legitimately re-derives a copy-mode row from the filesystem and writes `Synced`/`Stale` (this matches base behaviour, which ran the copy branch for any status). Reword both: "the store never coerces; the reconcile pass may re-derive a status from observed source/target/hash and write it canonically". Consider a test pinning "unknown status + copy + matching hash → Synced with `last_error` cleared" so the policy is explicit rather than incidental.

### Ticket 26 Answer — undisclosed drift
- Folder-URL git listing (`github.com/o/r/tree/main/<dir>`) now runs the full discovery ladder (marketplace + depth walk) inside the folder; base ran only `collect_skill_dirs`. Add to the drift list as an intentional widening (or narrow it in ticket 34 if not wanted).
- Local listing now admits the root when `is_claude_skill_dir(root)` even without `SKILL.md`; base required `SKILL.md` at root. The candidate is listed valid and then rejected at install with `SKILL_INVALID`. Disclose; decide whether the local `valid` rule should match the install path's manifest requirement (`installer.rs:729-734`).

### Ticket 25 Answer — wording
- "missing skill row still sweeps orphan `skill_targets`" → the sweep removes the **filesystem** targets of those rows; the rows themselves are not deleted (same as base). Say so.

## Acceptance criteria

- [ ] AGENTS.md has no `invoke('…')` example and its environment invariant is true as stated.
- [ ] CONTEXT.md Sync status entry and t27/t26/t25 Answers amended as above.
- [ ] `npm run version:check && npm run check` green (docs only; lint covers markdown if configured).

## Answer

Docs only, done by the orchestrator in the main checkout.

- **AGENTS.md:95** — example rewritten to `invokeTauri("getManagedSkills")`.
- **AGENTS.md environment invariant** — heading narrowed to "Core never resolves filesystem roots from the environment"; the body now lists the sanctioned exceptions explicitly: thin `dirs::*` adapters (`cache_cleanup.rs`, `temp_cleanup.rs`, `skill_store::default_db_path`, `skill_store::migrate_legacy_db_if_needed`) and the `SKILLS_HUB_*` feature-flag env vars (`git_fetcher.rs`, `sync_engine.rs`, `install_finalize.rs`) — "they tune behaviour, never locate data". If ticket 34 moves `migrate_legacy_db_if_needed`'s root to the seam, drop it from the list.
- **CONTEXT.md Sync status** — "never as healthy" reworded to "the store never coerces it to healthy", plus a sentence stating the reconcile pass may re-derive any row's status from observed disk state and write it canonically (observation-grounded recovery, not coercion).
- **Ticket 27 Answer** — decision (1) amended with the reconcile clarification (unknown-status copy row + matching hash → `Synced`, `last_error` cleared; identical to base's copy branch).
- **Ticket 26 Answer** — new "Drift disclosed later" paragraph: folder-URL git listing widened to the full ladder (kept, desirable); local root admitted without `SKILL.md` then rejected at install (cosmetic; alignment noted as future polish).
- **Ticket 25 Answer** — "sweeps orphan `skill_targets`" → "sweeps the filesystem paths of orphan rows; rows not deleted (same as base)".
- Declined: the suggested pinning test for "unknown status + copy + matching hash → Synced" is code, out of this doc-only ticket; noted for ticket 34's Answer or a later pass.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
