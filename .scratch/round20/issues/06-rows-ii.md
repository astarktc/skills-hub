# 06: Rows II

Status: done — 01e4ec3

**Blocked by:** 05. There's no content dependency, but the rows share the registry file, so they land in sequence.

**Spec:** `.scratch/round20/spec.md` § Registry structure (detect-dir hazards are binding; Deep Agents Code copies);
the brief § Part 1 table A, § Part 2 "Recommended Part 1 additions", I4, I7 and I8.

**What to build:** twelve harnesses with their own skills folder become rows.

## Rows

- Grok Build (`.grok/skills`)
- Deep Agents Code (`.deepagents/agent/skills`, **`supports_symlink: false`**)
- Reasonix (`.reasonix/skills`; Windows `%APPDATA%\reasonix\skills` through 01's typed-root override, its first
  production consumer)
- Cortex Code (`.snowflake/cortex/skills`, detect `.snowflake/cortex`, **never** `.snowflake`)
- Rovo Dev (`.rovodev/skills`)
- IBM Bob (`.bob/skills`)
- ForgeCode (`.forge/skills`, detect `.forge` + `forge`)
- Firebender (`.firebender/skills`)
- MiniMax Code (`.minimax/skills`, detect-dir flag in the comment)
- Posit Assistant (`.posit/assistant/skills`, detect `.posit/assistant` + `.positai`, **never** `.posit`)
- AiderDesk (`.aider-desk/skills`)
- Autohand Code CLI (`.autohand/skills`)

## Acceptance criteria

- [ ] Every row is re-verified against its brief source before it's added (as in ticket 05; deviations go in Comments).
- [ ] Each row has everything ticket 05 lists: variant, `as_key`, literal with facts, README row, table-test rows and an
      i18n label.
- [ ] Env-var homes (`GROK_HOME`, `DEEPAGENTS_HOME`, `REASONIX_HOME`, `AUTOHAND_HOME`, `AIDER_DESK_HOME_DIR`) are named
      in the row comments, not honoured.
- [ ] Deep Agents Code deploys by copy. Pin it with a test through `sync_dir_for_tool_with_overwrite`, or the existing
      capability test.
- [ ] Reasonix resolves its Windows override in a test with a substituted `HostRoots`.
- [ ] A draft PR's `gh pr checks` proves the Windows build.
- [ ] `npm run version:check && npm run check` passes.

## Comments

- 2026-11-06 (implementer, branch `t3/round20-t06-rows-ii`, draft PR #16): all twelve rows built; gate passes. Keys:
  `grok_build`, `deep_agents_code`, `reasonix`, `cortex_code`, `rovo_dev`, `ibm_bob`, `forgecode`, `firebender`,
  `minimax_code`, `posit_assistant`, `aider_desk`, `autohand_code`. All `group: None`, project dir per the brief's
  Part 1 column. Every source re-fetched on 2026-11-06 (T3 docs URL now 404s; its `docs-site/docs/agent-mode/skills.md`
  source in `hotovo/aider-desk` still says the same). Deviations:
  - **ForgeCode not detected on legacy `forge`** (spec's binding I4 line says `.forge` + `forge`). Evidence:
    `crates/forge_config/src/reader.rs` `resolve_base_path`: `FORGE_CONFIG`, else `~/forge` *if it exists*, else
    `~/.forge`. So a legacy install reads `~/forge/skills`, never `~/.forge/skills`; detecting `forge` would deploy
    into a folder it does not read (story 10). Pinned by `new_rows_are_never_detected_by_a_shared_or_unread_root`.
    Brief Part 1 row annotated. Operator call if a `former`/second-dir model for legacy Forge is wanted.
  - **MiniMax Code precedence `?` → `Shared`** from source: own `<dataDir>/skills` root has no priority (0) and
    shares a rank with the external user roots (30–40), so the external copy wins (`registry.ts` `sortEntries`);
    dataDir `~/.minimax` now VERIFIED (`data-dir.ts`; legacy `~/.mavis`, env `MINIMAX_DATA_DIR`). Brief rows updated.
  - **Reasonix** precedence/duplicates encoded `Unknown` (brief: own LIKELY). It also scans `~/.reasonix/skills` on
    Windows besides `%APPDATA%\reasonix\skills` (comment only; no other row's dir).
  - **Firebender** duplicates `ShowsAll`, from the brief's "up to 6 copies" hazard (OpenCode's precedent).
  - Posit's legacy `.positai` detect kept per the spec, although an old release reads `~/.positai/skills`.
  - The fixture has the rows (undetected) and their `DUPLICATE_READS`; the `registry` scenario is untouched (05 owns it).
  - CI (head `878349c`): rust, web, rust-windows pass. The first rust-windows run caught
    `only_the_global_catalog_carries_placement_straight_from_the_registry` assuming `~/{rel}`, fixed in `878349c`.
    One rerun was for the known SQLite flake `concurrent_readers_and_writers_all_succeed` ("database is locked",
    already recorded in `.scratch/reviews/1.3.2-patch-review.md`).
- 2026-11-06 (orchestrator): ran in parallel with 05 and 07. Merged with them by replaying 06's diff onto `main`
  (`af3d971`). The orchestrator decided the open Posit question per spec Q7: the `.positai` detect dir is dropped,
  because an old release reads `~/.positai/skills`, as with ForgeCode's `forge`. A test pins it. Gate green (Rust 962);
  CI green. The combined row review for 05–07 follows.

