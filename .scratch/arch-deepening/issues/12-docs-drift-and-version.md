# 12: Docs drift and version bump for the round

Status: resolved

Type: task
Source: `../spec.md` Q25

**What to build:** The agent context and user docs describe the codebase as it now is: AGENTS.md invariants gain the mutation guard ("operations that materialise or remove Sync targets serialise themselves in core; commands carry no lock"), extend "never loop a per-pair sync command in the frontend" to update/refresh and import, and note the new error variant; the README changes only if operator-visible behaviour changed (Refresh reporting, import's divergent-sibling rule); CHANGELOG entry; CONTEXT.md re-read for terms that landed differently than written. Then one `npm run version:set` for the round.

**Blocked by:** 01, 02, 03, 04, 05, 06, 07, 08, 09, 10, 11

- [ ] AGENTS.md "Invariants" and "Do not" sections reflect the mutation guard, Propagation, Artifact removal and Onboarding import
- [ ] No doc references the deleted commands (`update_managed_skill`, `import_existing_skill`, `remove_skill_source`)
- [ ] CHANGELOG lists the round's user-visible changes
- [ ] `npm run version:set` run once; `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Shipped on `arch/12-docs-and-version`, merged to `main` as `e094456` docs, `8af3355` release: v1.2.2
(worktree SHAs before the rebase: `f1eff91`, `64c19be`). The round shipped as **v1.2.2**, not the 1.3.0
the branch first carried: `npm run version:set 1.2.2` rewrote `package.json`, `tauri.conf.json`,
`Cargo.toml`, and the CHANGELOG heading, but `version:set` did not touch `Cargo.lock`, so the release
commit shipped its `app` entry at the `1.3.0` cargo had resolved earlier; the next cargo run corrected it
in `6f973f6` ("sync Cargo.lock app version to 1.2.2") — the gap round-3 ticket 06 closes by making
`version:set` rewrite both lockfiles.

**AGENTS.md**
- Invariants: new **Sync-target mutation** bullet (guard at entry points, private mutex, `*_unlocked`
  seams, non-reentrant "an entry point never calls another entry point", `try_serialized` +
  `reconciled: false`).
- Ambiguity resolution: the "re-invoke the relevant command after a mutation" sentence now names both
  worlds — project applies the returned `ProjectViewDto` (`applyView`, failure-path `refreshView`
  only), skills re-invokes `getManagedSkills`.
- Error contract: `PATH_OUTSIDE_TOOL_DIRS` / `ensure_path_within_tool_dirs` as the ADR-0001 worked
  example.
- "Global sync fan-out is backend-owned" → **Target fan-out is backend-owned**: four batch commands
  (sync / refresh / import / removal) + Propagation as the only writer on an update path.
- New bullets: **Artifact removal** (7 scopes, presence + settlement rule, ADR-0002) and **Git bytes
  have two single entry points** (`git_cache::fetch_through_cache`, `git_acquisition::acquire`) and
  **Frontend presentation logic is pure and lives once**.
- Do not: removal only through `artifact_removal` (`remove_path_any` callers), confirmations through
  the Modal shell, one implementation each of relative time / repo grouping / storage access.
- Worktree safety: added `core/artifact_removal.rs`, `core/refresh.rs`, `commands/projects.rs`.

**CONTEXT.md** — sharpened **Propagation** (link skip, copy→link re-materialisation, truthful mode),
**Refresh (all)** (two phases, pool of 4, per-skill guard, cancellation finalizes nothing),
**Artifact removal** (the seven scopes; scopes carry their own roots), **Onboarding import** (plan
re-read in core, force-overwrite of the source tool, byte-identical rule). Added **Project view**,
**Mutation guard**, **Git acquisition**.

**README.md / docs/README.zh.md** — Key features: import's byte-identical rule, GitHub API fast path,
Update/Refresh reporting + parallel fetch. FAQ: kept-`error` rows after a failed removal, and the
"statuses not re-checked" notice.

**CHANGELOG.md** — `[1.2.2] - 2026-09-03` entry (Added / Changed / Fixed / Internal-architecture).

**Gate** — `npm run version:check` (Version OK 1.2.2) and `npm run check` exit 0 (152 frontend tests,
423 Rust tests); `cargo test --all` 423 passed.

**Grep gate** — clean except: historical archives (`docs/releases/v0.1-v0.2/*`,
`docs/releases/v1.0.0/RELEASE_NOTES_v1.0.md`, `plan-mode-archive/`) which are frozen records like the
CHANGELOG, and one code comment in `src/hooks/useSharedDirConfirmation.test.ts` ("No backend, no
window.confirm") which is accurate and is code, not docs.
