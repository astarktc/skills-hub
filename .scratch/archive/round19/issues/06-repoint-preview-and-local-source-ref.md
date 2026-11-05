# 06 — Re-point preview command; local skills expose their recorded source on the wire

Status: done — 27c936f
Blocked by: —
Spec: `.scratch/round19/spec.md` — D15 (grill Q15). Source: `flows.md` Flow 5 gaps 1–3.

## Work

- **`preview_repoint_source(skill_id, target)`** (`core/repoint.rs`, beside `repoint_skill_source`): validates and
  resolves the `RepointTarget::{Git, Local}` exactly as the real re-point would (same acquisition intent, listing
  clone reused through the cache), **writes nothing**, and answers `{ resolved_name, ref, subpath, name_matches:
  bool, candidates: Vec<…> | null }` — `candidates` non-null when the target is a multi-skill repo so the frontend can
  open the picker (reuse listing's candidate admission). Typed refusals (`GITHUB_SKILL_NOT_FOUND`, …) are thrown as
  today's command errors.
- **`source_ref` on the wire for local skills.** `skillSourceLabel` returns `central_path` for every non-git skill, so a
  local skill never shows its folder — the one fact the source-missing repair needs. Confirm `ManagedSkill` carries
  `source_ref`; if it does, this half is frontend-only (`skillPresentation.ts` + test + `SkillCard`/detail source
  line show the folder, copyable). The Re-point modal pre-fills the current source ("was: ~/Downloads/scratchpad").
- Bindings; shims; fixture handler for the preview (name mismatch and multi-skill producible in `failures`);
  tests: preview writes nothing, mismatch flagged, multi-skill answers candidates, local source shown.
- The confirm-on-mismatch UI and inline errors are ticket 12.

## Result

**Preview command** — `preview_repoint_source(skillId, target: RepointTarget) -> RepointPreview`
(`commands/mod.rs`, `#[tauri::command] + #[specta::specta]`, listed in `collect_commands!`; resets and honours the shared
`CancelToken`; errors thrown via `from_anyhow`). Core: `core/repoint.rs::preview_repoint_source{,_with}`.

```ts
RepointPreview = {
  resolved_name: string | null,   // landing skill's SKILL.md name (else folder name); null only for an unresolved multi-skill repo
  ref: string | null,             // branch acquisition resolved (slash branches repaired); null = local / default-branch clone
  subpath: string | null,         // recorded folder; null = repo root / local
  name_matches: boolean,          // case-insensitive vs the skill's name; a mismatch is never a refusal
  candidates: GitSkillCandidate[] | null, // listing's admission, when the URL names a repo (no folder) holding >1 skill
}
```

- Same validation and intent as the Re-point: local → `expand_home_path_in` + `validate_local_folder`
  (SOURCE_PATH_MISSING / SKILL_INVALID / LOCAL_SOURCE_INSIDE_TOOL_DIR); git → `parse_full_github_url` +
  `acquire(SkillIntent::ByName(Some(record.name)), allow_fast_path)` + `ensure_installable_skill_dir` — i.e. exactly
  `acquire_git_repoint`'s acquisition. `MULTI_SKILLS` on a repository URL becomes `candidates` (via
  `installer::list_git_skills_with`, now `pub(crate)`, full checkout through `fetch_through_cache` — the entry the
  acquisition's full clone just filled, and the one the picker's install reuses). Every other refusal is thrown typed.
- **Writes nothing**: acquired bytes land in a `StagingDir` under `<cache_dir>/repoint-preview/` (never the central
  repo), removed on every exit (RAII). Only the git clone cache is written, as listing does.
- `git_acquisition::Acquired` gained `branch: Option<String>` (the ref the bytes came from) so the preview reports
  the ref acquisition actually resolved.

**Local source on the wire** — `ManagedSkillDto.source_ref` already carries the folder; frontend-only fix:
`skillSourceLabel` now returns a git/local skill's `source_ref` (central path only when none is recorded / imported).
New pure `repointCurrentSource(skill)` → `{ kind, ref } | null`. `SkillCard`'s Unlocatable `source_missing` strip shows
"was at <path>". `ChangeSourceModal` takes `source` (App passes `library.pendingRepointSkill` — one-line App.tsx edit),
pre-fills the arm of the skill's own kind and shows "was: <path>". No preview call in the modal (left to ticket 12).
i18n: `repointSource.{was, missingFolder}` appended to `en`. Shim re-exports `RepointPreview`.

**Fixture** — `previewRepointSource` handler mirrors the Rust (fast path → `GITHUB_SKILL_NOT_FOUND`, repo URL →
candidates via a shared `listingCandidates` helper also used by `listGitSkillsCmd`). In `failures`, deprecated-helper →
`…/mattpocock/skills/tree/main/skills/engineering/ts-error-fixer` is a name mismatch; → `https://github.com/mattpocock/skills`
answers candidates. Verified live on 5175: local-notes card/strip show `/Users/alex/Projects/notes-skill`; Re-point
dialog pre-filled with "was: …".

**Tests** — Rust (`core/tests/repoint.rs`): `preview_writes_nothing_for_either_target` (whole temp root — home, central,
DB file bytes — plus source folder byte-identical, no scratch left), `preview_flags_a_name_mismatch_without_refusing`,
`preview_of_a_multi_skill_repo_answers_its_candidates` (also byte-identical), 
`preview_of_a_multi_skill_repo_resolving_the_name_still_offers_the_choice`, `preview_refuses_typed_as_the_repoint_does`
(invalid URL, missing folder, inside Tool dir, unknown skill, cancelled). Vitest: `skillPresentation.test.ts`
(`skillSourceLabel` ×3, `repointCurrentSource` ×2); `backend.test.ts` "fixture backend: Re-point preview (D15)" ×4.

**Gate** — `version:check` ok; lint ok; styles:check ok; `cargo fmt --check` / clippy / `cargo test --all` (775) green.
`npm run test` / `npm run build` red only in other children's in-flight files: ticket 10 (`App.tsx` catalog wiring,
`ProjectsPage.tsx`, `useSkillLibrary/useSyncOrchestration/useAddSkillFlow` tests — `catalog` / `toolLabelById`) and
ticket 03 (`reportOutcome.test.ts`, `fixtures/backend.ts:1005` `declined`, the D9 cancelled-Refresh fixture test). No
error in a file this ticket touched. Note: `rustfmt` over the crate also reformatted `core/global_sync.rs` (ticket 03's
file) — formatting only.

**Ticket 12 builds on** `commands.previewRepointSource` (typed binding + fixture): show "replaces X's bytes with
**resolved_name** (`subpath` @ `ref ?? default branch`)", highlight `!name_matches`, open the Add picker on `candidates`
(each carries `subpath` + `resolution`). Open question for 12: `RepointTarget::Git` takes only a URL, so picking a
candidate on a default-branch repo (`resolution.branch` null) needs a tree URL with a branch name, or a
`subpath`/`resolution` on the git target.

## Comments

- 2026-10-29 — **done — 27c936f** (post-rebase SHAs on `origin/main`; the pre-rebase SHAs cited above exist only in the orchestrator's local history). Shipped in 1.3.0 (`v1.3.0`, published 2026-10-28; operator smoke on the main Mac passed 2026-10-29, ticket 19 § Smoke outcome). 19a closure: `round19/spec.md` § Closure — 2026-10-29.
