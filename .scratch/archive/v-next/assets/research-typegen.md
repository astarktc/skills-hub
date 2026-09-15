# Research: TS type generation from Rust DTOs (ts-rs vs tauri-specta vs alternatives)

Resolves `.scratch/v-next/issues/06-research-ts-type-generation.md`. Researched 2026-08-29 against
primary sources (GitHub repos/APIs, docs.rs, crates.io, tool source code) plus a full read of this
repo's DTO surface. All repo claims cite files in this checkout; all tool claims cite the owning source.

---

## 1. The repo's actual type-mirror surface (measured, not estimated)

### IPC-crossing `Serialize` DTO structs: 23

| Location | Structs |
|---|---|
| `src-tauri/src/commands/mod.rs` (11) | `ToolInfoDto`, `ToolStatusDto`, `InstallResultDto`, `SyncResultDto`, `UpdateResultDto`, `GlobalToolConfigDto`, `ManagedSkillDto`, `SkillTargetDto`, `FeaturedSkillDto`, `OnlineSkillDto`, `SkillFileEntry` |
| `src-tauri/src/commands/projects.rs` (4) | `ResyncSummaryDto`, `BulkAssignResultDto`, `BulkAssignErrorDto`, `GitignoreStatusDto` |
| `src-tauri/src/core/onboarding.rs` (3) | `OnboardingVariant`, `OnboardingGroup`, `OnboardingPlan` |
| `src-tauri/src/core/project_ops.rs` (3) | `ProjectDto`, `ProjectToolDto`, `ProjectSkillAssignmentDto` |
| `src-tauri/src/core/installer.rs` (2) | `GitSkillCandidate`, `LocalSkillCandidate` |

Note that 8 of the 23 live in `core/`, not `commands/` — any generation setup must annotate core
modules too, mildly blurring the "commands/ is wiring, core/ is logic" boundary (acceptable: it's a
derive, not logic).

### Call sites and mirrors

- **60 registered commands** (`generate_handler!` in `src-tauri/src/lib.rs:101–161`).
- **~83 frontend invoke sites**: 55 via the `invokeTauri` wrapper in `src/App.tsx:189–195`, 28 direct
  `invoke(`/`invoke<` (`ProjectsPage.tsx`, `useProjectState.ts`, `EditProjectModal.tsx`, …), hitting
  **46 distinct command names**. All are plain string-name invokes.
- Hand mirrors: `src/components/skills/types.ts` (15 types, incl. frontend-only `ToolOption`),
  `src/components/projects/types.ts` (6 types), **plus** an inline `GitignoreStatusDto` in
  `src/components/projects/EditProjectModal.tsx:9–12` — the "types.ts is the mirror" convention is
  already eroding.

### Concrete mirror defects found (evidence the problem is real)

1. **Systematic optionality lie.** Every `Option<T>` field is mirrored as `field?: T | null`
   (e.g. `ManagedSkill.description`, `ProjectSkillAssignmentDto.last_error`). Serde always emits the
   key (`"description": null`), so the wire type is `T | null` with the key **always present**; the
   `?` is fiction. Harmless for reads today, but it means `"description" in obj` / key-presence logic
   would typecheck and misbehave.
2. **Latent `rename_all` trap.** `SkillFileEntry` is the only DTO with
   `#[serde(rename_all = "camelCase")]` (`commands/mod.rs:1228`). Its current fields (`path`, `size`)
   are single words so nothing breaks — the first two-word field added will silently desync from the
   snake_case-assuming TS mirror.
3. **Missing mirror.** `SyncResultDto` (`mode_used`, `target_path`) has no TS counterpart at all;
   `sync_skill_to_tool` is invoked ~10× in `App.tsx` with the result untyped/ignored.
4. **Numeric edge.** `u64` fields (`FeaturedSkillDto.downloads/stars`, `OnlineSkillDto.installs`,
   `SkillFileEntry.size`) are mirrored as `number` — correct for the JSON wire, but a generator's
   default must be configured to match (see ts-rs `TS_RS_LARGE_INT` below).
5. **Args side is fully untyped** — camelCase `#[allow(non_snake_case)]` params
   (`skillId`, `sourcePath`) exist only as string-keyed object literals at 83 call sites. No
   types-only tool fixes this; only command bindings would (relevant to the "later" phase).

### Error-contract context (upcoming ticket)

`format_anyhow_error()` (`commands/mod.rs:40–48`) returns a `String` with five pipe-prefixes
(`MULTI_SKILLS|`, `TARGET_EXISTS|`, `TOOL_NOT_INSTALLED|`, `TOOL_NOT_WRITABLE|`, `SKILL_INVALID|`)
parsed in `src/App.tsx`. The planned replacement `{ code: "TOOL_NOT_WRITABLE", tool, path }` is an
**internally-tagged** serde enum (`#[serde(tag = "code")]` with struct variants) — the tagging-fidelity
test below is decided by exactly this shape.

---

## 2. Candidate tools (primary-source status, checked 2026-08-29)

### ts-rs (Aleph-Alpha/ts-rs)

- **Version/health**: v12.0.1 on crates.io (2026-01-31); steady major cadence — v10 2024-09,
  v10.1 2024-12, v11 2025-06, v11.1 2025-10, v12 2026-01 (GitHub releases API). Repo pushed
  2026-08-11, 33 open issues, 1,865 stars, not archived (GitHub repo API). **Stable, actively
  maintained, out of RC.**
- **Tauri relationship**: none — framework-agnostic, zero Tauri dependency, so zero coupling to
  Tauri major versions. Works by deriving a `TS` trait; bindings are written by auto-generated
  tests on `cargo test` (or `cargo test export_bindings`), to `TS_RS_EXPORT_DIR` (default
  `./bindings`), configurable persistently via `[env]` in `.cargo/config.toml` (docs.rs ts-rs 12).
- **Serde fidelity** (`serde-compat` on by default; docs.rs + `ts-rs/src/lib.rs` doc-comments):
  - Supported attrs: `rename`, `rename_all`, `rename_all_fields`, `tag`, `content`, `untagged`,
    `skip`, `skip_serializing(_if)`, `flatten`, `default`. Unsupported attrs emit build warnings
    rather than silent wrong output.
  - `Option<T>` default → `t: T | null` — **exactly the wire truth this repo's mirrors get wrong**.
    Opt-outs: `#[ts(optional)]` → `t?: T`; `#[ts(optional = nullable)]` → `t?: T | null`
    (reproduces the current hand-written style verbatim if churn-free migration is preferred);
    struct-level `#[ts(optional_fields)]` (lib.rs:288–318).
  - **Internally-tagged enums supported** (`#[serde(tag = "code")]` → TS discriminated union);
    adjacently-tagged (`tag` + `content`) and `untagged` also supported. Constraint: internally
    tagged enums can't use tuple variants — struct variants like
    `ToolNotWritable { tool, path }` are fine (ts-rs wiki "Deriving the TS trait").
  - **One config landmine**: v12 exports `i64`/`u64` as `bigint` by default; must set
    `TS_RS_LARGE_INT = "number"` to match Tauri's JSON IPC reality (v12 release notes; docs.rs).
    `PathBuf` → `string` matches `OnboardingVariant.path` already.

### tauri-specta + specta (specta-rs)

- **Version/health**: `tauri-specta` 2.0.0-rc.25 (2026-05-08); `specta` 2.0.0-rc.25 (2026-05-07);
  `specta-typescript` **0.0.12** (crates.io). The only stable release is 1.0.2 = Tauri v1-only
  (README version matrix). **v2 has been in RC since ~2023 and still is in mid-2026**; rc.24→rc.25
  releases were self-described "Phasing Forward" upgrades with type-representation changes.
  Repos alive (tauri-specta pushed 2026-07-26, 28 open issues; specta pushed 2026-08-25) but the
  RC treadmill is the maintenance posture: each rc has shipped breaking changes.
- **Model**: generates a `bindings.ts` containing **typed command functions** (`commands.getToolStatus()`)
  plus types and typed events, wired through a `tauri_specta::Builder` + `collect_commands!` in
  `lib.rs`. That replaces both `generate_handler!` registration style and, to get value, the call
  style at all 83 invoke sites (docs.rs tauri-specta).
- **Types-only adoption?** Technically possible — `specta` + `specta-typescript`'s `Typescript::export`
  can dump types without adopting the command bindings — but you'd be building on `specta-typescript`
  0.0.x + `specta` RC with documented breaking churn, to get the same artifact ts-rs produces from a
  stable release. No leverage until you actually want the bindings.

### TauRPC (MatsDK/TauRPC)

0.7.1; healthy (pushed 2026-07-03, 2 open issues, 332 stars) but small, and it requires restructuring
commands into trait-based "procedures" resolvers with runtime-generated types (docs.rs taurpc; GitHub
README). That is a rewrite of the entire command layer — strictly larger blast radius than
tauri-specta for the same benefit. Not a fit.

### typeshare (1Password)

1.0.5 (2026-01-02). **Disqualified by the error-enum requirement**: enums with data must be
*adjacently* tagged (`#[serde(tag = "t", content = "c")]`); internally tagged is unsupported —
"a limitation for the entire Typeshare framework, which already throws parsing errors in the absence
of adjacent enum tags" (1Password member, issue #217; PR #146 adding internal tagging was closed
unmerged; docs show only adjacent tagging). The planned `{ code: "...", tool, path }` payload cannot
be expressed. Also CLI-driven with its own annotation dialect, weaker serde fidelity overall.

---

## 3. Build integration for the recommended tool (ts-rs)

- **Where files land**: set in `src-tauri/.cargo/config.toml`:
  `TS_RS_EXPORT_DIR = { value = "../src/bindings", relative = true }` and
  `TS_RS_LARGE_INT = "number"`. One `.ts` file per type + generated imports between them; commit them.
- **Freshness in CI for free**: `npm run check` already runs `cargo test` (`rust:test`), and
  `#[ts(export)]` emits bindings *from tests* — so every existing gate run regenerates bindings before
  anything TS-side compiles. Add one guard so drift fails loudly rather than silently regenerating in
  a dirty tree: a `git diff --exit-code -- src/bindings` step in `.github/workflows/ci.yml` after the
  test step (or a tiny npm script in `check`). Note `npm run check` runs lint/build *before* rust:test
  today; either reorder or rely on the CI diff-guard — recommend the diff-guard, it also catches
  "edited Rust, forgot to run tests locally".
- **Dual TS compiler setup: non-issue.** ts-rs emits plain `export type` declarations with zero
  runtime code and no compiler-version-sensitive syntax; both `typescript@~6.0.3` (eslint) and the
  `typescript-7` build alias consume them identically. (tauri-specta's bindings.ts embeds runtime
  invoke wrappers — also plain TS, but a bigger surface to keep lint-clean; moot given the
  recommendation.)
- **Crate impact**: `ts-rs = "12"` as a normal dependency of `app_lib` (derives sit on shipping
  structs); `serde-compat` default feature suffices. MSRV 1.88.

---

## 4. Recommendation

**Tool: ts-rs v12 — types-only, keep all 83 plain `invoke` sites unchanged.**

Reasons, in order: (a) it's the only candidate that is simultaneously stable (v12.0.1 vs
tauri-specta's 3-year RC / specta-typescript 0.0.x), actively maintained, and decoupled from Tauri's
release cycle; (b) it round-trips this repo's exact needs — snake_case defaults, the lone
`rename_all = "camelCase"` DTO, `Option<T> → T | null` wire truth, and the upcoming internally-tagged
`{ code, ...fields }` error enum as a proper discriminated union; (c) generation rides the existing
`cargo test` gate, so CI freshness costs one diff-guard line; (d) typed command *bindings* are the
one thing it doesn't do, and that's precisely the part this codebase doesn't want rewritten today.

tauri-specta is not rejected forever: if/when v2 stabilizes and typed args/bindings become desirable
(the untyped-args gap in §1.5), it can be adopted *then* — the DTO structs will already be
serde-clean, and `#[derive(specta::Type)]` can sit beside `#[derive(TS)]` during any transition.

**Adoption path (incremental):**
1. Add `ts-rs = "12"`; create `src-tauri/.cargo/config.toml` with `TS_RS_EXPORT_DIR=../src/bindings`
   + `TS_RS_LARGE_INT="number"`.
2. Annotate the 23 structs with `#[derive(TS)] #[ts(export)]`. Decision point: adopt wire-accurate
   `field: T | null` (recommended; the frontend only *reads* these DTOs so it compiles strictly
   safer) or add `#[ts(optional = nullable)]` per Option field to reproduce today's `?: T | null`
   with zero TS churn.
3. Run `cargo test export_bindings`; replace the bodies of `skills/types.ts` / `projects/types.ts`
   with re-exports from `src/bindings/` (keeps 30+ existing import sites untouched; `ToolOption`
   stays hand-written), and swap `EditProjectModal.tsx`'s inline `GitignoreStatusDto` for the
   generated one. Add the missing `SyncResultDto` usage or leave the command result unused as today.
4. Add the CI diff-guard; run `npm run version:check && npm run check`.
5. (Separate future ticket) revisit tauri-specta for typed args/bindings post-stabilization.

**Estimated blast radius**: small. ~23 derive annotations + 1 config file + 2 mirror files becoming
re-export shims + 1 component import swap + 1 CI line; zero invoke-site changes; zero runtime
behavior change. Roughly a half-day including verification, plus review. The error-enum ticket then
gets its TS union for free by deriving `TS` on the new error type.

---

## Sources

- Repo: `src-tauri/src/commands/mod.rs`, `commands/projects.rs`, `core/{onboarding,project_ops,installer}.rs`,
  `src-tauri/src/lib.rs`, `src/components/skills/types.ts`, `src/components/projects/types.ts`,
  `src/components/projects/EditProjectModal.tsx`, `src/App.tsx`, `package.json` (this checkout, 2026-08-29).
- ts-rs: docs.rs/ts-rs 12 (config, serde-compat table, MSRV); `ts-rs/src/lib.rs` doc-comments on
  `optional`/`optional_fields`/`tag`/`content` (raw.githubusercontent.com, main); GitHub releases API
  (v10–v12 dates, v12 bigint/HashMap notes); GitHub repo API (pushed_at 2026-08-11, 33 issues); crates.io (12.0.1).
- specta-rs: crates.io (tauri-specta 2.0.0-rc.25 2026-05-08, specta 2.0.0-rc.25, specta-typescript 0.0.12);
  tauri-specta README version matrix (raw, main); GitHub releases (rc changelog); GitHub repo API
  (pushed_at, issue counts); docs.rs/tauri-specta (Builder/collect_commands model).
- TauRPC: docs.rs/taurpc 0.7.1; github.com/MatsDK/TauRPC; GitHub repo API.
- typeshare: crates.io (1.0.5); docs/src/usage/annotations.md (raw, main); issue #217 (adjacent-tag-only
  framework limitation, member statement); PR #146 (internal tagging, closed unmerged).
