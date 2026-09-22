# 04 — Frontend-B: "Change source…" on every managed skill

Status: ready-for-agent
Blocked by: 02 (done — 3c48a86, merged to main)
Spec: `.scratch/round16/spec.md` — D3, D4, D5. Read the spec first. The wire map from ticket 02's report is
pasted under § Wire map below before this ticket is claimed.

## Goal

One affordance, one modal: any managed skill (git, local, imported) can be re-pointed at a GitHub URL or a
local folder. The Unlocatable `source_missing` repair opens the same modal with Local preselected.

## Today

- `SkillCard.tsx:298–308` shows the MapPin "Re-point" button only for `sourceKind(skill) === "git"`; the
  `source_missing` badge offers `repoint` via `unlocatableRepairs` (`skillPresentation.ts:302`).
- `useSkillLibrary.ts`: `gitRepointSelection` state (`:88`), `handleRepointGitSkill` (`:154`), `canRepoint`
  git-only (`:169`), `handleConfirmRepointGitSkill` (`:361`), `handleRepointSkill` (`:383`: git → modal, local →
  folder picker → `repointLocalSkillSource`), modal `GitRepointModal.tsx`.
- Error copy `GIT_REPOINT_REQUIRES_GIT` in `src/commandError.ts` + i18n — retired by ticket 02.

## Work

1. **Modal**: evolve `GitRepointModal` into `ChangeSourceModal` (rename file/component; update the modals
   index if any): a kind picker (GitHub URL / Local folder) with the skill's current kind preselected; the git
   arm is today's URL field + validation copy; the local arm shows the chosen folder path with a "Choose
   folder…" button (`@tauri-apps/plugin-dialog` `open({ directory: true })`, as `handleRepointSkill` does today)
   and an editable path input (supports `~`). Confirm calls `invokeTauri("repointSkillSource", skill.id, target,
   { reassert_auto_sync })` with `target = { kind: "git", url } | { kind: "local", path }`. Loading/disabled
   states as the git modal has.
2. **Hook**: `gitRepointSelection` → `repointSelection: { skillId, name, preselect: "git" | "local" }`;
   `canRepoint` true for every managed skill; `handleRepointSkill(skill)` opens the modal preselecting the
   skill's current kind (`imported` → `local`); the `source_missing` repair opens it with `local`. Delete the
   inline folder-picker path in `handleRepointSkill`. Keep the `skillGone` warning. Toasts stay
   `actions.repointing` / `status.repointed`; the outcome is the existing `refreshOutcome` fold (a batch of one),
   `closeModal` completion closes the modal. Hook tests updated (`useSkillLibrary.test.ts`).
3. **Card**: the MapPin button becomes "Change source…" (i18n `changeSource.action`) on every managed card,
   not gated on kind; tooltip/aria updated. `unlocatableRepairs` unchanged in shape.
4. **Error copy**: remove the `GIT_REPOINT_REQUIRES_GIT` branch from `describeCommandError` and its i18n keys
   (ticket 02 already removed the code from the union). `LocalSourceInsideToolDir` and `SourcePathMissing`
   copy must read correctly when raised from Change source (they already exist for Add/Re-point — verify the
   wording fits both).
5. **i18n**: rename the `gitRepoint.*` family to `changeSource.*` (or keep and add — say which); every new key
   in **both** `en` and `zh`. No hardcoded UI text.

## Constraints

- Backend only through `invokeTauri`; DTO types via `src/components/skills/types.ts`.
- No new state library; styles in `App.css`; Modal shell for confirmations.
- `npm run lint && npm run test && npm run build` green. Work in your assigned worktree; commit on your branch;
  do not merge; `.scratch/` only via a dated `## Comments` entry here.

## Wire map (from ticket 02)

From ticket 02's report (merged to main as `3c48a86`):

| Before | After |
|---|---|
| `repointLocalSkillSource(skillId, newPath, policy)` | removed |
| `repointGitSkillSource(skillId, newUrl, policy)` | removed |
| — | `repointSkillSource(skillId: string, target: RepointTarget, policy: RefreshPolicyDto) => Promise<SkillMutationResultDto>` |

```ts
export type RepointTarget =
  | { kind: "git"; url: string }    // full GitHub repo/skill URL; stored as given (trimmed)
  | { kind: "local"; path: string } // folder as typed or picked; "~" is expanded in the backend
```

- Removed error code: `GIT_REPOINT_REQUIRES_GIT` (already gone from `src/commandError.ts`; its i18n keys
  `errors.gitRepointRequiresGit` at `src/i18n/resources.ts:~324` (EN) and `:~930` (ZH) are now unused — delete).
- Unchanged: `SkillMutationResultDto` (`{ report: RefreshReport, skills }`). No progress channel.
- Errors the modal can receive: `SOURCE_PATH_MISSING`, `SKILL_INVALID`, `LOCAL_SOURCE_INSIDE_TOOL_DIR` (local arm);
  `INVALID_GITHUB_URL` (git arm); `NOT_FOUND`. Acquisition/finalize failures are report data (`refreshOutcome`).
- `RepointTarget` is re-exported from `src/components/skills/types.ts`.
- Lane 02 already switched the two `invokeTauri` call sites in `src/hooks/useSkillLibrary.ts` (`:~371` git,
  `:~409` local) to `repointSkillSource` with the matching `target` — the hook still has today's two-path shape
  (modal for git, inline folder picker for local); D5 replaces that with one modal.
