# Round-4 review — `opus`

Fixed point `f057190` → HEAD `7fcec94`. Verified green locally: `npm test` 206/206, `cargo test --all` 504/504, `npm run version:check` → 1.2.3 (correct: ticket 10 bumps last).

## Standards

**Hard (documented standard).**

1. **Tool-key→label rule duplicated three times.** AGENTS.md: *"Frontend presentation logic is pure and lives once. `src/lib/skillPresentation.ts` owns source kind, repo label/href …"*. The same expression now lives in `SkillCard.tsx` (`importedFromTool = skill.imported_from_tool ? t(\`tools.${…}\`, { defaultValue: … }) : t("unknown")`), `SkillDetailView.tsx` (inline inside `sourceLabel`), and `commandError.ts` (`t(\`tools.${e.tool}\`, { defaultValue: e.tool })`). One `toolLabel(t, key)` in `skillPresentation.ts` is the repo's own answer. Also *Duplicated Code*.

**Judgement calls.**

2. **Primitive Obsession + a real empty-string hole.** `onboarding_import.rs::apply_one_unlocked`: `.map(|variant| variant.tool.as_str()).unwrap_or_default()` then `install_imported_skill(…, found_in_tool)`. A `&str` stands in for a Tool key, and the `unwrap_or_default()` silently records `imported_from_tool: Some("")` — the card then renders `Imported from ` (empty), not `unknown`. `Option<&str>` (or the registry's `&'static ToolAdapter`) makes the state unrepresentable.
3. **`expect` on a command path.** Ticket 03's declared deviation: *"`sync_assignment_target` now `expect`s the name resolution … the skill is in hand by construction."* `project_sync.rs` is reached from `propagate_one_assignment` inside a Tauri command; a panic aborts instead of returning a `CommandError`, against the module's own "every target … is report data" contract. Prior art exists, so judgement — but it is a new `expect`, not an inherited one.
4. **Two entry points, one operator action.** `commands/mod.rs::repoint_local_skill_source` calls `repoint_local_source` (unguarded store write) then `refresh_managed_skills_core` (its own guard). Legal under AGENTS.md ("an entry point never calls another entry point" — these are sequential, not nested), but the source rewrite and the Update are not one mutation: a concurrent batch can interleave between them.
5. **i18n key casing.** `unlocatable.source_missing` / `source_missingTooltip` / `central_missing…` are snake_case in a catalog that is camelCase everywhere else, to let `SkillCard` write `t(\`unlocatable.${unlocatable}\`)`. Wire spelling leaking into the copy catalog; a `Record<UnlocatableState, string>` map (as `useSkillLibrary.ts`'s `SKIPPED_REASON_KEY` already does) keeps both conventions.
6. **Untyped refusal, prose-encoded condition.** `unlocatable.rs::require_local` bails `"only a local skill has a source folder to re-point or detach from: {name} is {source_type}"` → wire `OTHER`. Declared deviation 5. ADR-0001: *"never encode error conditions in message strings."* Acceptable as a caller guard only because the card never offers the action otherwise.

## Spec

**(c) Implemented but wrong / unsafe.**

1. **Legacy reclassification rule (iii) can destroy a genuine `local` source, irreversibly and silently.** Spec Q5 rule (iii) is implemented verbatim (`legacy_reclassification.rs::tool_shaping_path`: shape match under *any* home prefix + `!source.exists()`), but the only guard against a real folder is *current existence*. A genuine own source at, say, `~/Projects/myrepo/.agents/skills/mine` (a project skills dir the operator maintains) or any path on an unmounted volume / offline cloud-synced folder matches the shape and is absent → the row becomes `imported` with **`source_ref: None`**. The pass logs only a count (`"reclassified {} legacy local skill(s) as imported"`), so the discarded path exists nowhere afterwards and Re-point cannot even show the operator where it used to be. This contradicts story 9: *"I want a `local` skill whose folder is genuinely my own (outside every Tool dir) left exactly as it is by that reclassification, so that upgrading never changes a real source."* Minimum fix: `log::info!` each reclassified `(name, discarded source_ref, rule)`. Better: anchor rule (iii) to a home-shaped prefix (`…/Users/<x>/` | `/home/<x>/` | `/mnt/…`) so an in-repo `.agents/skills` never matches.
   (The `ensure_schema` V9 + `user_version` fix itself is sound: only DBs already stuck at ≥6 survive a second launch today, and every remaining step ≤8 is idempotent, so writing `user_version` after the incremental branch strictly improves second-launch safety.)
2. **Detach in the both-gone state produces an unrepairable skill.** Spec (ticket-09 amendment): *"Both paths gone reports `source_missing` (Re-point's Update rebuilds central)."* True — but the card offers all three `source_missing` repairs, and Detach on a both-gone row yields `imported` + `central_missing` + `refreshable: false` → the only remaining affordance is Remove, with every Tool link left dangling. Story 13 (*"detach … so that a skill I no longer maintain externally keeps working everywhere"*) is not delivered in that case. Either hide Detach when `central_path` is also gone, or refuse it typed.
3. **A `..`-bearing alias subpath still escapes the checkout.** Story 4: *"a symlink whose target escapes the repository refused with a clear error, so that a hostile or broken repo can never make the app read outside the checkout."* `LinkChain::follow` enforces this for link *targets*, but `repo_subpath::normalize_subpath` documents *"`..` is not interpreted here — a subpath is a name, not a traversal"*, and `git_acquisition::clone_path` then does `repo_dir.join(checkout_subpath)`. A recorded/URL-derived `source_subpath` containing `..` reads outside the cached checkout and is copied into central. Cheap guard: reject `..` segments in `normalize_subpath`'s callers (or return `Result`).

**(a) Missing / partial.**

4. Ticket 02's last acceptance box is still open: `[ ] The five operator rows (plugins/tanstack-all/skills/*) refresh in tauri:dev — operator-verified after merge`. Operator-owned, but the round is not closable without it (Q11: *"The operator owns the `tauri:dev` smoke test … and the Release confirmation."*).

**(b) Scope creep.** Only one, self-declared and small: `open_log_folder` now *opens* a non-`.app` log dir instead of revealing it in its parent (ticket 01 deviation). Q9 asked only that *"`log_reveal_target` becomes a pure core function with a test"* — a behaviour change on Linux/Windows rode along.

**Verified clean (no finding).**
- Error contract: all 7 new `SignalError`s have a `CommandError` arm, a `describeCommandError` branch, and both EN + ZH keys; paths travel as `withDetail` lines / structured fields, never inside the localized sentence.
- Import take-over cannot overwrite a divergent sibling: `BatchPolicy { overwrite: false, overwrite_if_same_content: true }` and force-overwrite `BatchOverride`s only for fingerprint-identical Tools; a shared skills dir implies one variant path, so no divergent row hides behind an identical one.
- `refresh_eligibility` / `is_refreshable` / DTO agree: `All` settles skipped outcomes *before* dispatch, so `reassert_auto_sync_unlocked` (per-dispatched-skill) can never mint a target for an unlocatable or imported skill; the card's Update is `refreshable && !unlocatable`.
- Cache entries are only widened (`Checkout::covers` / union in `git_cache.rs`); `follow_upstream_links` records the alias, never the resolved path.

## Summary

**Blocking (before v1.2.4 ships)**
- **Spec 1** — `legacy_reclassification` rule (iii) can reclassify a genuine `local` skill whose folder is merely *currently* absent, clearing `source_ref` with no record of the discarded path anywhere (violates story 9; data loss, irreversible). Log the discarded path at minimum; prefer anchoring the shape match to a home-like prefix.
- **Standards 1** — the tool-key→label rule duplicated across `SkillCard.tsx`, `SkillDetailView.tsx` and `commandError.ts` breaches AGENTS.md's "presentation logic … lives once" (`skillPresentation.ts`).
- **Standards 2** — `unwrap_or_default()` in `onboarding_import.rs::apply_one_unlocked` can persist `imported_from_tool: Some("")`, which the card renders as `Imported from ` (blank). One-line fix.

**Follow-up**
- Spec 2 — hide/refuse Detach when the central copy is also gone.
- Spec 3 — refuse `..` segments in acquisition subpaths (story 4's guarantee).
- Spec 4 — operator `tauri:dev` verification of the five `tanstack-all` rows still outstanding.
- Spec 5 — non-macOS `open_log_folder` behaviour change (declared, unasked).
- Standards 3 — new `expect` on a command path in `project_sync::sync_assignment_target`.
- Standards 4 — `repoint_local_skill_source`'s two sequential entry points are not one atomic mutation.
- Standards 5 — snake_case `unlocatable.*` i18n keys; use a state→key map as `useSkillLibrary` already does.
- Standards 6 — `require_local`'s prose-encoded refusal lands on the wire as `OTHER`.
