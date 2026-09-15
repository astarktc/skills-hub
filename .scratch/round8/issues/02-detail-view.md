# 02: `activeView === "detail"` with no detail skill renders — and behaves — as My Skills

Status: done — 3190623

**Lane:** B (Astra low)  **Blocked by:** None (can start immediately).
**Files:** `src/App.tsx` only (plus a hook test only if you find one that can express it — App has no tests by design).
**Do NOT touch:** anything under `src-tauri/`, `src/hooks/`, `src/components/`.

## Today (backlog 5, Fable F7 from the round-6 panel)
`App.tsx` keeps `activeView` in `useState`; `handleOpenDetail` sets `"detail"`, and the render falls back to the library list
when `activeView === "detail"` but `detailSkill` is null (the skill vanished — deleted, or the list reloaded without it). The
state stays `"detail"`, so anything keyed on `activeView` (header nav highlight, `handleViewChange`'s `closeDetail` branch,
future conditionals) still believes it is in detail.

## Decision (grilled: derived, not synced)
Derive the effective view once, near the top of `App`:
`const effectiveView = activeView === "detail" && !library.detailSkill ? "myskills" : activeView;`
and use `effectiveView` everywhere the render and the nav read the view today (`<main>` branches, `Header`'s
`activeView`/`onViewChange` props, the detail-vs-explore-detail `onBack` selection). Do **not** add a `useEffect` that writes
`setActiveView` to repair state, and do not change the hooks — AGENTS.md prefers derived state over synced state. The raw
`"detail"` value staying in state is harmless because nothing reads it raw any more; `handleOpenDetail` still sets it and a
later `openDetail` still works because `detailSkillId` is what changed.
Check `explore-detail` is unaffected (it derives `detailSkill` from `exploreDetailSkill`, not `library.detailSkill`).

## Gate
`npm run lint && npm run test && npm run build` green (no Rust touched — skip cargo). Do NOT run `npm run tauri:dev`; `npm run dev`
for a visual check is fine. One conventional commit (`fix(app): derive the effective view when the detail skill is gone`). Do NOT
push/merge/rebase. `.scratch/` is gitignored — never `git add -f`. Paste `## Comments` in the final message.

## Comments

- 2026-09-15 — Status reconciliation: ready-for-agent → done — 3190623. Evidence: git log v1.2.7..v1.2.8 finds 3190623; src/App.tsx:85 derives effectiveView and :260 passes it to the header.
