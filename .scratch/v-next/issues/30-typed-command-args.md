# 30: Typed command args across the invoke seam — spike tauri-specta, adopt or fall back to a typed registry

Status: resolved

Type: task
Blocked by: 23, 24, 28

## What to build

Map fog item #4 (research doc §1.5): all ~83 `invokeTauri` sites pass untyped string-keyed arg objects. Root cause is inherent to Tauri — `invoke("name", { args })` is stringly-typed at the JS boundary by design; compile-time safety needs codegen or a hand registry. Ticket 06 deferred tauri-specta "until v2 leaves RC"; grilling (2026-08-30) revised that: it is still `2.0.0-rc.25` (2026-05-08, ~4 years in RC, 1.1M downloads, actively maintained) and the label alone is not disqualifying — **compatibility facts are**. Decided: spike-then-adopt.

**Phase 1 — spike (bounded, in this worktree).** Pin `tauri-specta = "=2.0.0-rc.25"` (+ matching `specta`), annotate a representative slice, and inspect the generated TS for the three hard shapes:

1. the `Channel`-streaming batch sync command (`sync_skills_to_tools`);
2. the internally-tagged `CommandError` enum (must round-trip as the same `{ code, …fields }` union `describeCommandError` consumes);
3. `Option<T>` → wire-accurate `T | null` (the fidelity ticket 07 fought for; no fictional `?`).

Record the verdict **with evidence** (generated-output excerpts/diffs) in the Answer.

**Phase 2a — clean → adopt wholesale.** specta becomes the single generator: `#[specta::specta]` on all 57 commands, builder in `lib.rs` exporting one typed function per command (args, result, error), **replacing ts-rs** (running both = two parallel type sets). `src/bindings/` becomes the specta output; the CI drift guard + "untracked bindings" check keep working (adjust the generation step); the per-world `types.ts` shims keep re-exporting. `invokeTauri` is replaced by or wraps the generated functions; every hook/component call site migrates; the "raw `invoke` only in components" allowance ends (components use the typed seam too). Hook tests mock the new seam. AGENTS.md invariants ("IPC DTOs are generated", error contract) and ADR 0001 get updated to name specta; record the pin-and-bump-deliberately policy. If the full migration outgrows one session, land the pipeline + one world green and split the remaining worlds into follow-up tickets (map rule).

**Phase 2b — fights us on any shape → typed registry.** One `CommandMap` interface in `src/lib/tauri.ts` (`{ command_name: { args: {...}; result: GeneratedDto } }`), `invokeTauri` generic over it, result types referencing the ts-rs bindings; all ~83 sites typed; raw `invoke` in components migrated; document the honest limit (args are hand-maintained; drift caught at call sites, not from Rust) in AGENTS.md and note the upgrade path.

## Acceptance criteria

- [ ] Spike verdict recorded with generated-output evidence for all three shapes.
- [ ] Either exit: zero untyped `invokeTauri`/`invoke` arg objects in `src/`; a wrong arg key or command name fails `npm run build`; zero raw `@tauri-apps/api/core` `invoke` outside the typed seam.
- [ ] If specta: ts-rs removed, single generator, CI drift guard green, ADR 0001 + AGENTS.md updated, exact pin recorded. If registry: AGENTS.md documents the registry and its limit.
- [ ] `npm run version:check && npm run check` green.

## Answer

**Spike verdict: ADOPT tauri-specta (exit 2a).** Landed green in `2cb5efc` + `cdbbd15` (Fable 5.1 child, medium thinking; orchestrator-verified incl. a fresh `cargo test` → `git status --porcelain -- src/bindings` empty; 317 cargo (357 − 41 ts-rs per-type export tests + 1 `export_bindings`) + 89 vitest, full gate green on main).

**Pins** (`src-tauri/Cargo.toml`, exact `=`): `specta = "=2.0.0-rc.25"`, `tauri-specta = { version = "=2.0.0-rc.25", features = ["typescript"] }`, `specta-typescript = "0.0.12"` (what tauri-specta's metadata requires). Builds against locked Tauri 2.11.3 via tauri's `specta` feature; needs Rust ≥ 1.88 (matches `rust-version`). `ts-rs` removed — zero hits in `Cargo.lock`/`src`.

**Evidence, three hard shapes** (verbatim from generated `src/bindings/index.ts`):
1. Channel command — `syncSkillsToTools: (skills: BatchSyncSkillDto[], tools: string[], policy: BatchSyncPolicyDto, onProgress: Channel<SyncProgressDto>) => __TAURI_INVOKE<BatchSyncReportDto>("sync_skills_to_tools", { skills, tools, policy, onProgress })` (ts-rs never typed commands at all).
2. `CommandError` — `{ code: "TOOL_NOT_INSTALLED"; tool: string } | … | { code: "RATE_LIMITED"; resetMinutes: number } | … | { code: "OTHER"; message: string }` — same union ts-rs emitted (the enum already had `rename_all_fields = "camelCase"`, so `resetMinutes` is the true wire key); `COMMAND_ERROR_CODE_MAP` and the `syncStatus.ts` `satisfies` guards compile unchanged. `SyncTargetStatusDto` / `CandidateMatch` / `SettingUpdate` tagged enums identical.
3. `Option<T>` — `content_hash: string | null`, `global_selected_tools: string[] | null`, arg `name: string | null`; no fictional `?`. Bonus fidelity: `#[serde(default)]` Deserialize-only inputs become `overwrite?: boolean` (ts-rs emitted them required).

**Snags handled**: `i64/u64/usize` refused by default → `Builder::dangerously_cast_bigints_to_number()` (all timestamps/counters; ts-rs silently emitted `number`); default `ErrorHandlingMode::Result` wraps results in `{status,…}` → `Throw` keeps the `runAction`/`toCommandError` throw contract; `f64` is hard-coded `number | null` (serde_json writes NaN/∞ as null) → the three finite-by-construction settings floats carry `#[specta(type = specta_typescript::Number)]`. **Real bug surfaced by the typed seam**: `useAddSkillFlow` passed `undefined` for `Option<String>` args — now `null`.

**What changed**: all 41 IPC types derive `specta::Type`; all 44 commands carry `#[specta::specta]`; `lib.rs::specta_builder()` holds the single `collect_commands![…]` list feeding both `invoke_handler` and the `export_bindings` test — registration and bindings can no longer drift (an unlisted command has no binding → frontend build fails). `src/bindings/` = one generated `index.ts` (41 per-type files deleted). `src/lib/tauri.ts`: `invokeTauri<K>(name: K, ...args: Parameters<Commands[K]>)` + exported `Commands`/`CommandName`/`InvokeTauri`; all 55 call sites migrated (camelCase names, positional args); `SkillDetailView` prop typed `InvokeTauri`; 7 test files stub by camelCase name. Proof: deliberate typos → `TS2345 '"unsyncSkil"' is not assignable…`, `TS2554 Expected 1 arguments, but got 2` — build exit 1; restored → 0. Docs: AGENTS.md (new-command, IPC-types, test-seam, error-contract invariants + fidelity rules + pin-and-bump-deliberately policy), ADR 0001 amendment, CHANGELOG, CI comment.

**Honest limits**: args are positional (specta's shape) — two same-typed params swapped (e.g. `unsyncSkillFromTool(skillId, tool)`) are not caught by TS; `configureProjectTools`'s `Option<IgnoreUpdateOptions>` arg is inlined structurally rather than by name (specta quirk; type-compatible, cosmetic).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
