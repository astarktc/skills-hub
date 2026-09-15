# 11: One shared-skills-dir confirmation

Status: resolved

Type: task
Source: `../spec.md` Q24; CONTEXT.md **Shared skills dir group**

**What to build:** Warning the operator that a Tool shares its skills directory with others — before a per-skill toggle and before a sync-target change — is one building-block hook: given a Tool key it decides whether confirmation is needed, produces the other members' labels, and exposes one pending-confirmation value both flows drive through the existing Modal shell. The blocking `window.confirm` is gone, and the shared-dir map leaves the sync world hook's interface.

**Blocked by:** None (can start any time; lowest payoff, so last)

- [ ] No `window.confirm` in the app; both flows show the same modal with the same label text
- [ ] The confirmation decision and label arithmetic have direct hook-level tests; the sync-orchestration tests no longer stub `window.confirm`
- [ ] The sync world hook's returned interface no longer exposes the shared-tool map
- [ ] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

Resolved on `arch/11-shared-dir-confirm` (commit c6e9110).

**Shape built** — `src/hooks/useSharedDirConfirmation.ts`: input `SharedDirTool[]`
(`{ id, label, sharedWith }`), returns `{ needsConfirmation, sharedLabels, pending, request, cancel }`.
`request(toolKey)` resolves `true` immediately when the tool is standalone; otherwise it sets one
pending value (`{ toolKey, toolLabel, labels, resolve }`) and resolves with the operator's answer.
`cancel()` resolves `false`. 9 direct hook tests in `useSharedDirConfirmation.test.ts`.

**Instantiation choice (documented per ticket)** — instantiated once inside `useSyncOrchestration`,
the world that already owns `toolInfos` + localized labels. It exposes
`sharedDirPending` / `cancelSharedDirConfirmation` / `requestSharedDirConfirmation`;
`useSkillLibrary` receives `requestSharedDirConfirmation` through its existing
`Pick<SyncOrchestration, …>` seam (replacing `sharedToolIdsByToolId`), and `App.tsx` renders the one
`SharedDirModal` from `sync.sharedDirPending`. This is the smallest change consistent with AGENTS.md:
no new cross-world seam, no state library, building-block hook owned by a world. The
`sharedToolIdsByToolId` memo remains **internal** to `useSyncOrchestration` (target-group expansion
only) and is no longer in its returned interface.

**Also** — `handleSyncTargetChange` and `handleToggleToolForSkill` are now async and `await` the
confirmation; `useSkillLibrary` lost `pendingSharedToggle` / `pendingSharedLabels` /
`handleSharedConfirm` / `handleSharedCancel`; `SharedDirModal` takes the `pending` value directly;
i18n key `sharedDirConfirm` deleted, replaced by a `sharedDir.{title,body,confirm,cancel}` set
appended at the end of the EN and ZH blocks (merge-conflict friendly).

**Gate** — `npm run version:check` (Version OK 1.2.1) and `npm run check` green (136 frontend tests,
360 Rust tests, lint, build, fmt, clippy). Nothing under `src-tauri/` or `src/bindings/` touched.
