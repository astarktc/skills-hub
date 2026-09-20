# Handoff — 2026-09-16 · round 12 (tool-selection integrity) closed, unreleased

The queue is `.scratch/BACKLOG.md` — this note does not repeat it.

## Where things stand

- **Round 12 is done and archived** (`archive/round12/`): BACKLOG #06/#07/#08 shipped in `396374e` — a corrupt saved
  tool selection now refuses to sync (`SETTING_CORRUPT`, startup warning, re-save repairs), unknown registry keys are
  pruned when the selection is read and refused on write, and Add/import/Sync-All report a selected-but-undetected
  tool as one warning per tool instead of dropping it silently.
- **Not released.** CHANGELOG has the three entries under `[Unreleased]`; `main` is at 1.2.12. Releasing = `npm run
  version:set 1.2.13`, move the entries under a dated heading, `git fetch && git rebase origin/main`, push.
- Gate at close: `npm run version:check && npm run check` green; `cargo test --all` 643 passed; vitest 353 passed.
- No live effort.

## Exact next step

1. Decide whether to release 1.2.13 now (three user-visible fixes) or batch more first.
2. Then pick from BACKLOG "Now": #02 (wave C, largest) or a hygiene item (#09 CHANGELOG reconstruction, #31 workflow
   hardening).

## Gotchas learned this session

- A mount-once `useEffect` that reads `t`/`notify` trips `react-hooks/exhaustive-deps`; the repo's convention is a
  commented `eslint-disable-next-line` on the deps line (see `useAddSkillFlow.ts:493`).
- i18next 26 resolves `key_one`/`key_other` from `count` natively; ZH only ever hits `_other`.
- `lens_diagnostics` flags `github_token: "old-token"` in `useSettingsState.test.ts` as a credential — it is a fixture.
