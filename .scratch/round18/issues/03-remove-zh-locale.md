# 03 — Remove the ZH locale; EN-only invariant; gitignore `.pi/`

Status: implemented
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

## Result

Implemented, uncommitted (working tree only).

**What changed**
- `src/i18n/resources.ts`: `zh` tree deleted; also dropped the now-unused `languageShort`, `languageOptions`,
  `language`, `interfaceLanguage` EN keys (only the switchers used them).
- `src/i18n/resources.test.ts`: parity test retired. New guard scans every non-test `src/**/*.{ts,tsx}` (via
  `import.meta.glob(?raw)` — `node:fs` is not in the app tsconfig's types) for literal `t("…")` / `ctx.t("…")`
  first arguments and asserts each resolves in `en` (plural keys via `_one`/`_other`); template-literal keys are
  skipped. Plus a non-vacuity check (>100 keys scanned) and `Object.keys(resources) == ["en"]`. Mutation-tested:
  renaming `t("navExplore")` → `t("navExploreBogus")` fails with the key and file named.
- `src/i18n/index.ts`: `lng: 'en'` (was `languagePreference.read()`); `fallbackLng: 'en'` kept; i18next and the
  `i18next-browser-languagedetector` dependency untouched (the detector was never wired in `src/`).
- Language switchers removed from **both** `SettingsPage.tsx` (select) and `Header.tsx` (the `EN/中文` header
  button — not named in the ticket but the same switcher state), with `language`/`onToggleLanguage` props and
  App's `toggleLanguage` + persistence effect. Dead `.lang-btn` CSS removed from `src/App.css`.
  `useSettingsState.ts` held no language state — untouched.
- `src/lib/preferences.ts`: `languagePreference` removed; a comment records `"skills-language"` as a retired key
  (never read, never reused). A stored `zh` is simply never read → cannot crash. Its key assertion dropped from
  `persistedPreference.test.ts`.
- `src/lib/reportOutcome.test.ts`: `zh` assertions removed; "both locales" test names/loops now EN-only.
- `docs/README.zh.md` deleted (plain `rm`, not staged); README "中文" link line removed (tools table untouched).
- `AGENTS.md` UI-strings invariant → "add keys to `en` in `src/i18n/resources.ts`; EN only (BACKLOG #61)" + a
  pointer to the new guard test.
- `.gitignore`: `.pi` added after `.agents`; `git check-ignore .pi/skills` confirms; no tracked path matches.

**Files touched**: `.gitignore`, `AGENTS.md`, `README.md`, `docs/README.zh.md` (deleted), `src/App.css`,
`src/App.tsx`, `src/components/skills/Header.tsx`, `src/components/skills/SettingsPage.tsx`, `src/i18n/index.ts`,
`src/i18n/resources.ts`, `src/i18n/resources.test.ts`, `src/lib/preferences.ts`,
`src/lib/persistedPreference.test.ts`, `src/lib/reportOutcome.test.ts`.

**Verification**: `npm run check` exit 0 — vitest 16 files / 407 tests passed, `vite build` ok, cargo 669 passed;
`npm run version:check` → `Version OK (1.2.17)`. `lens_diagnostics` shows only pre-existing warnings.

**CHANGELOG one-liner (for ticket 10's 1.2.18 block)**:
"Removed the Chinese (中文) UI locale and the language switcher; Skills Hub is now English-only. A previously
saved language choice is ignored."

**Open questions**
- AGENTS.md "Do not" still lists `.claude/`, `.agents/` as never-commit and "Environment gotchas" mentions the
  `.claude`/`.agents` hardlinked mirror; `.pi/` (now ignored) is not named there. Left alone (ticket scoped the
  AGENTS.md edit to the invariant) — orchestrator may want to add it.
- `i18next-browser-languagedetector` is a declared but unused dependency; kept per ticket.

