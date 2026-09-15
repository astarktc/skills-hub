# Unit-test the extracted hooks (vitest, mocked invoke)

Status: resolved

Type: grilling
Blocked by: —

## Question

Ticket 05 carved App.tsx into seven hooks (`src/hooks/`), each receiving its dependencies (`reporter`, `sync`, `library` interfaces, `invokeTauri` via `src/lib/tauri.ts`) instead of creating them — they are now testable through their interfaces with a mocked `invoke`. The repo has **no frontend test runner at all** (no `"test"` script, no vitest/jest/@testing-library). Scan finding 9b; fog graduated at ticket 05 resolution.

Decide, then implement:

- Add vitest (+ @testing-library/react for `renderHook`, or vitest's own environment?) — dev-dependency and `"test"` script; should `npm run check` grow a frontend-test leg, and should CI?
- Mock seam: `src/lib/tauri.ts` is a module — mock it with `vi.mock`, or inject further? (The hooks import it directly by design; decide whether that seam is good enough for tests or wants a parameter.)
- Which hooks first, and what to assert: candidates ordered by logic density — `useAddSkillFlow` (name-collision/candidate matching branches), `useSkillLibrary` (refresh error collection, toggle flows), `useSyncOrchestration` (target defaulting, shared-dir expansion), `useStatusReporter` (toast one-shots). Hooks only, **no JSX tests** (per fog note).
- Also cover `describeCommandError` (pure function, cheap wins).

Context: scan finding 9; ticket 05 Answer (interface inventory).

## Answer

All six decisions grilled (one round, all recommendations accepted) and implemented same session; landed green in `be9a74c` (50 tests, 5 files).

1. **Runner & environment — vitest 4 + @testing-library/react (`renderHook`) + jsdom.** Vitest reuses the existing Vite pipeline (config lives as a `test` block in `vite.config.ts` via `/// <reference types="vitest/config" />`); hooks run inside `renderHook`'s throwaway component in a jsdom environment. No JSX tests.
2. **Gate & CI both grow a test leg.** `"test": "vitest run"`; `npm run check` is now lint → test → build → rust gates; CI's `web` job runs `npm run test` between lint and build. Rationale: a suite outside the gate rots silently.
3. **Mock at module seams, no production refactor.** `vi.mock("../lib/tauri")` for backend calls (per-command dispatch stubs), `vi.mock("sonner")` to assert toasts, `vi.mock("@tauri-apps/api/core")` with a minimal `FakeChannel` for sync progress. Parameter-injecting `invokeTauri` was rejected: one adapter = hypothetical seam (codebase-design rule); the module seam is already mockable.
4. **`useProjectState` migrated onto `invokeTauri`** (21 call sites, mechanical) — every hook now crosses the backend at the one seam. Raw `invoke` remains only in components (ProjectsPage, EditProjectModal), which stay untested by design.
5. **Scope delivered: all five candidates**, densest-first — `describeCommandError` (13: wire-contract → copy mapping incl. CANCELLED→null, legacy prose recovery), `useStatusReporter` (6: one-shot toast semantics, silenced entries, cancel flow), `useSyncOrchestration` (12: target defaulting from saved selection vs installed, scan-selected-only filtering, shared-dir group toggling + confirm, batch policy wire shape, progress → reporter, failure surfacing incl. the not-writable-skips opt-in), `useSkillLibrary` (9: refresh error collection, refresh-means-overwrite contract, auto-sync-off short-circuit, per-tool toggle sync/unsync/TARGET_EXISTS detail, shared-dir modal deferral, case-insensitive name collision), `useAddSkillFlow` (10: git/local candidate routing single-vs-picker, name-collision guards, explore auto-select exact/containment/mismatch/fallback, selected∩installed deploy targeting). No follow-up ticket needed.
6. **Tests colocated (`src/**/*.test.ts`) and type-checked by the build** — `tsconfig.app.json` already includes `src/`, so typescript-7 checks them for free. This immediately paid off: the build caught a `CommandError` variant missing its required `tool` field that vitest's transpile-only mode ran happily.

Invariants for future sessions: hook tests mock at the three module seams above, never at hook internals; expected i18n values assert key + params via a catalog-less `t` stub that honors `defaultValue` (how tool labels resolve); DTO fixtures must satisfy the generated bindings (the build enforces this). AGENTS.md commands + state-rule sections updated inline.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
