# Research: TS type generation from Rust DTOs (ts-rs vs tauri-specta)

Status: resolved

Type: research

## Question

Skills Hub hand-mirrors Rust DTO structs (in `src-tauri/src/commands/`) into TypeScript (`src/components/skills/types.ts`, `src/components/projects/types.ts`), verified by nothing. Which 2026-current tool best generates the TS side from the Rust structs for a **Tauri 2** app: `ts-rs`, `specta`/`tauri-specta`, or something newer?

Evaluate against: Tauri 2 compatibility and maintenance health (recent releases, open-issue posture); ergonomics for the existing plain-`invoke` call style (tauri-specta generates typed command bindings — is that a plus or an unwanted rewrite of all 85 invoke sites?); serde attribute fidelity (snake_case fields, `Option<T>` → optionality, enums — including a serde-tagged error enum like `{ code: "...", ...fields }`); build integration (where generated files land, how CI keeps them fresh, interplay with the dual TS6/TS7 compiler setup); and effort to adopt incrementally (types only first, bindings later?).

Deliverable: findings file at `.scratch/v-next/assets/research-typegen.md` with a recommendation. (Deviation from the throwaway-branch convention: single shared checkout — no branch switching.)

## Answer

**ts-rs v12, types-only adoption.** Full findings: [research-typegen.md](../assets/research-typegen.md) (2026-08-29, primary sources).

- **ts-rs v12.0.1** (Jan 2026, active) wins on stability and fit; plain generated declarations are consumed identically by both TS compilers, and the 83 plain-invoke sites stay untouched.
- **tauri-specta rejected**: 3-year RC treadmill (2.0.0-rc.25, specta-typescript still 0.0.x) — typed command bindings not worth the churn now; can be revisited later without conflict.
- **typeshare disqualified**: cannot express the upcoming internally-tagged error enum `{ code, tool, path }`; ts-rs renders it as a proper TS discriminated union (exactly what ticket 02 needs).
- **Measured surface**: 23 IPC-crossing DTO structs — 8 live in `core/` (onboarding, project_ops, installer), not just `commands/`, so derives touch core too (acceptable: a derive, not logic).
- **Mirror defects found** (evidence the hand-mirror is already broken): systematic `?: T | null` optionality lie vs serde's always-present-key `T | null`; latent `rename_all = "camelCase"` trap on `SkillFileEntry`; `SyncResultDto` unmirrored; inline `GitignoreStatusDto` in `EditProjectModal.tsx:9–12` escaping types.ts.
- **Blast radius ~half a day**: 23 derives + `.cargo/config.toml` (`TS_RS_EXPORT_DIR`; set `TS_RS_LARGE_INT="number"` — known landmine) + mirror files become re-export shims + one CI `git diff --exit-code` freshness guard; generation rides the existing `cargo test` gate.

Adoption decisions (file layout, shim strategy, AGENTS.md invariant rewrite) are ticket 07's, now unblocked.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Fired as an AFK research sub-agent at charting time (2026-08-29). Resolved same day.
