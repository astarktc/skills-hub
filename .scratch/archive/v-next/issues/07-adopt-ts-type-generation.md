# Adopt TS type generation for the IPC seam

Status: resolved

Type: grilling
Blocked by: 06

## Question

Given ticket 06's recommendation, decide and implement adoption: which DTOs get derived first (all of `commands/` DTOs? the ticket-02 error enum when it exists?), where generated files live, how `npm run check`/CI keeps them fresh, and what happens to the hand-written `types.ts` files (deleted vs re-exporting generated types).

Then update AGENTS.md: the "DTO changes: update both sides" invariant is replaced by generation (note the same-spirit caveat as scan finding 9 — this replaces a documented invariant with a smaller one).

Context: scan finding 9a; report card #9. Near-free safety win: the seam gets checked by both compilers with no runtime change.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

2026-08-30 (ticket 02 resolution): **the ts-rs pipeline is already live** — ticket 02 adopted ts-rs
v12 for `CommandError`/`GitCloneFailureKind`. Already in place: `ts-rs = "12"` dep,
`src-tauri/.cargo/config.toml` (`TS_RS_EXPORT_DIR=../src/bindings`, `TS_RS_LARGE_INT=number`),
generation via `cargo test`, committed bindings, and the CI diff-guard
(`git diff --exit-code -- ../src/bindings`). Remaining scope: annotate the 23 DTO structs, decide
`Option<T>` mapping (wire-accurate `T | null` vs `#[ts(optional = nullable)]`), turn
`skills/types.ts` / `projects/types.ts` into re-export shims, swap EditProjectModal's inline
`GitignoreStatusDto`, and rewrite the AGENTS.md DTO invariant. `rust-version` already bumped to 1.88.

## Answer

Landed green on main in **`e69ac42`**. Decisions (grilled 2026-08-31, all as recommended):

1. **`Option<T>` maps wire-accurate to `T | null`** — the fictional `?` from the hand-mirrors is
   gone. Serde always emits the key, so key-presence logic now typechecks truthfully. `npm run build`
   confirmed zero frontend construction sites depended on the `?`. Recorded in AGENTS.md: don't add
   `#[ts(optional)]`.
2. **`sync_skill_dir` deleted** (command + `SyncResultDto` + `generate_handler!` entry + the now-unused
   `sync_dir_hybrid` import). It was registered but had zero frontend invoke sites — ticket 03 deleted
   its sibling `sync_skill_to_tool`; this one had survived unused. So 21 remaining DTOs became 20
   annotated + 1 deleted.
3. **`ManagedSkill` naming kept via shim alias** — `export type { ManagedSkillDto as ManagedSkill }`;
   the shim is the naming-adaptation seam, no 10-file rename. Everything else keeps its Rust name.
4. **Shims are permanent** — `components/skills/types.ts` and `components/projects/types.ts` are now
   pure re-export shims (plus frontend-only `ToolOption`), the single import home per world;
   components never import `src/bindings/` directly. `EditProjectModal`'s inline `GitignoreStatusDto`
   swapped for the generated one.
5. **CI guard strengthened** — `git diff --exit-code` alone misses *untracked* binding files (exactly
   what this ticket's 20 new files would have been); the step now also runs
   `test -z "$(git status --porcelain -- ../src/bindings)"`.
6. **AGENTS.md DTO invariant replaced**: "generated, never hand-mirrored" — derive `TS` +
   `#[ts(export)]` (including `core/` DTOs), `cargo test` regenerates committed bindings, add via
   derive + test + shim re-export. This closes the last standing AGENTS.md revision in the map Notes.

Coverage: `src/bindings/` now holds all **31** IPC-crossing types (11 from tickets 02/03 + 20 new:
8 in `commands/mod.rs`, 4 in `commands/projects.rs`, 3 each in `core/onboarding.rs` +
`core/project_ops.rs`, 2 in `core/installer.rs`). ts-rs serde-compat handled the lone
`rename_all = "camelCase"` DTO (`SkillFileEntry`) and `PathBuf → string` (`OnboardingVariant`)
correctly; `TS_RS_LARGE_INT=number` keeps `u64 → number`.

Gate: `npm run version:check` exit 0; `npm run check` exit 0 (unmasked); `cargo test --all` exit 0
(232 passed); `lens_diagnostics mode=all` — no errors (one pre-existing Tauri-boilerplate warning).
