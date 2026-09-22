# 02 — Backend-B: one Re-point over a target enum, every provenance

Status: ready-for-agent
Spec: `.scratch/round16/spec.md` — decisions D3, D4. Read the spec first; this ticket is the Re-point backend
half. Ticket 04 (frontend-B) follows the bindings you regenerate.

## Goal

Re-point becomes one operation, `repoint_skill_source(skill, RepointTarget)`, defined for every managed skill
(`git`, `local`, `imported`) toward either a GitHub URL or a local folder — the operator can point a centrally
managed skill at a folder they maintain, or a folder skill at its GitHub home. The two same-kind commands and
their kind guards are retired.

## Today (verified @ `2145472`)

- `core/unlocatable.rs:83` `repoint_and_update` — local→local only (`require_local` at `:172`); validates the
  folder (`validated_local_repoint`: present, `require_skill_md`, `tool_holding_path` refusal →
  `LocalSourceInsideToolDir`), then runs `refresh_managed_skills_with(…, RefreshSelection::Ids([id]), policy,
  …, &|_, _| UpdateRequest::local(record.clone(), new_source, true))`.
- `core/refresh.rs:187` `repoint_git_skill` / `:210` `repoint_git_skill_with(api)` — git→git only
  (`GitRepointRequiresGit` at `:228`); validates the URL, acquires first, and only on success changes the
  source and Updates. Comment: "The new source is carried only in the acquired record, never written first."
- Commands `repoint_local_skill_source` (`commands/mod.rs:638`) and `repoint_git_skill_source` (`:677`), both
  `-> SkillMutationResultDto` via the Update/Restore catalog-bearing response.
- `SignalError::GitRepointRequiresGit` (`core/errors.rs:61,154,295,481`) → `CommandError::GIT_REPOINT_REQUIRES_GIT`
  (`src/commandError.ts:33,151`).
- Provenance: `core/provenance.rs` `Provenance::{Git, Local, Imported}`, `SkillProvenance::local(path)` etc.;
  record fields `source_type`, `source_ref`, `source_subpath`, `source_revision`, `imported_from_tool`
  (`core/skill_store.rs:141–153`). ADR-0003: an imported skill has no external source.
- `UpdateRequest::local(record, source, repoint: bool)` (`core/skill_update.rs:72`); git acquisitions build
  their request in `refresh.rs`'s acquire closure.

## Work

1. **Core**: one function, suggested home `core/skill_update.rs` or a new `core/repoint.rs` (declare in
   `core/mod.rs`):
   ```rust
   #[derive(Clone, Debug, serde::Deserialize, specta::Type)]
   #[serde(tag = "kind", rename_all = "snake_case")]
   pub enum RepointTarget { Git { url: String }, Local { path: String } }

   pub fn repoint_skill_source(paths, store, skill_id, target: RepointTarget, policy: RefreshPolicy,
                               cancel: Option<&CancelToken>, now: i64, on_progress) -> Result<RefreshReport>
   ```
   (`Local.path` is the raw operator string; the command expands `~` with `expand_home_path_in(&paths.home, …)`
   as today's local command does — keep that at the seam, or take the expanded `PathBuf` in core and keep the
   DTO separate; your call, say which.)
   - Look up the record by id (`NotFound` if absent). **No provenance guard**: any of `git`/`local`/`imported`
     may proceed.
   - `Local` arm: today's `validated_local_repoint` validation, then the same `refresh_managed_skills_with`
     call with `UpdateRequest::local(record, path, true)`. `unlocatable::repoint_and_update` and `require_local`
     go away (the Unlocatable `source_missing` repair calls the new function with `Local`).
   - `Git` arm: today's `repoint_git_skill_with` body (URL validation, acquire-then-settle, injectable
     `GithubApi` seam kept for tests). `refresh::repoint_git_skill{,_with}` fold into the new function.
   - **D4 provenance on settle** — the proposal the Update settles must rewrite the record's source fields:
     `→ Local`: `source_type = local`, `source_ref = folder`, `source_subpath = None`, `source_revision = None`.
     `→ Git`: `source_type = git`, `source_ref`/`source_subpath`/`source_revision` from the acquisition, as Add
     records them. Both: `imported_from_tool = None` (the skill is no longer imported). Verify the Update
     module actually persists these from the proposal (`SourceProposal` / the Acquired variant from round 14)
     rather than re-reading the old record — the existing rule "a failed acquisition/finalize changes nothing"
     must hold across kinds; add a test that a failed `Local` re-point of a `git` skill leaves `source_type`
     `git` and the old `source_revision` intact.
   - Edit V1 replay happens inside finalize as on any Update — no new code, but add one test: a skill with a
     persisted invocation Edit re-pointed `git → local` keeps its Edit applied to the new bytes.
   - `refresh_eligibility` (`core/provenance.rs`) must now report the formerly-imported skill refreshable; check
     the library listing's `refreshable`/`detachable` flags follow the new provenance with no cached state.
2. **Retire**: `SignalError::GitRepointRequiresGit` and `CommandError::GitRepointRequiresGit` (enum arm, `Display`,
   `From` arm, the `code` in the TS union — the bindings diff must show `GIT_REPOINT_REQUIRES_GIT` gone). The
   condition no longer exists; do **not** replace it with a new variant. Ticket 04 removes the
   `describeCommandError` branch and i18n keys — to keep `npm run build` green in your lane, remove only the
   `src/commandError.ts` references the type-check forces you to (the `:33` table entry and the `:151` case),
   and note it in your report.
3. **Commands** (`commands/mod.rs`): delete `repoint_local_skill_source` and `repoint_git_skill_source`; add
   `repoint_skill_source(app, store, cancel, skillId: String, target: RepointTarget, policy: RefreshPolicyDto)
   -> Result<SkillMutationResultDto, CommandError>` with the same catalog-bearing response and progress channel
   shape the two old commands used (check whether they took `on_progress: Channel<RefreshProgressDto>` — match
   `update_managed_skill`). Update `collect_commands![…]` in `src-tauri/src/lib.rs`.
4. **Tests**: port the existing local/git re-point tests (`core/tests/unlocatable.rs`, `core/tests/refresh.rs` or
   wherever `repoint_git_skill_with` is exercised) onto the new function; add the cross-kind matrix — at minimum
   `git → local`, `local → git` (mock `GithubApi`), `imported → local`, `imported → git`, plus the two failure
   tests named in step 1. `commands/tests/commands.rs` may reference the old commands — update.
5. **Bindings**: `cd src-tauri && cargo test --all` regenerates `src/bindings/index.ts` — commit it. Re-export
   `RepointTarget` from `src/components/skills/types.ts` (per-world shim). Make only the minimal frontend
   type-level changes `npm run build` forces (the two old `invokeTauri("repointGitSkillSource" | "repointLocalSkillSource", …)`
   call sites in `src/hooks/useSkillLibrary.ts:~409, ~371` will fail the typed seam — switch them to the new
   command with the matching `target` so the build passes; ticket 04 owns the real UI). Note every such touch.

## Constraints

- AGENTS.md invariants: new command → `#[specta::specta]` + `collect_commands!`; `commands/` is wiring only;
  core never resolves filesystem roots (`paths.home` comes in); the Update pipeline is the only settle path
  (no direct store writes of source fields outside it).
- CONTEXT.md **Re-point** and **Unlocatable skill** describe today's two-provenance shape; ticket 07 rewrites
  them — do not edit CONTEXT.md/ADRs; list in your report what you believe changed for the docs.
- Backend prose: English only. Tests: `cargo test --all`, clippy clean, `cargo fmt`.
- Work in your assigned worktree only. Commit on your branch with conventional messages; do not merge, do not
  touch `.scratch/` beyond appending a dated `## Comments` entry to this ticket, never `git mv`/archive.

## Report back

Old→new wire map (command names, `RepointTarget` shape, removed error code, changed `SkillMutationResultDto`
if any), every frontend line you touched to keep the build green, the doc-facing changes you believe ticket 07
must make, gate output.
