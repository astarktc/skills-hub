# 03 — Remove the ZH locale; EN-only invariant; gitignore `.pi/`

Status: ready-for-agent
Spec: `.scratch/round18/spec.md` — D7, ticket 03.

## Work

- `src/i18n/resources.ts`: delete the `zh` tree; `src/i18n/resources.test.ts`: retire the parity test (replace with a
  test that every `t()` key used in `src/` exists in `en` — grep-based, or keep a minimal shape test). Keep
  i18next, the language detector and every `t()` call; `fallbackLng: 'en'` stays.
- Remove the language switcher from `SettingsPage.tsx` and its state in `useSettingsState.ts` / `preferences.ts`
  (the persisted key is a compat contract: read-and-ignore, don't crash on a stored `zh`). `App.tsx` and
  `reportOutcome.test.ts` references.
- `docs/README.zh.md` deleted; README link removed.
- `AGENTS.md` invariant "add keys to both en and zh" → "add keys to `en` in `src/i18n/resources.ts`; EN only
  (BACKLOG #61)". `CHANGELOG.md` entry.
- `.gitignore`: add `.pi` beside `.claude` / `.agents` (skills mirror; currently untracked-and-unignored).
- `npm run check` green.
