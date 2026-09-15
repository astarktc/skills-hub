# 04: One skill presentation module and one persisted-preference helper (frontend)

Status: resolved

Type: task
Source: `../spec.md` Q15, Q16. Runs in a parallel worktree; touches no merge-risk file.

**What to build:** How a Managed skill's source is identified and displayed has one home: a pure module that answers "git or local?", the repo label and href, the repo-grouping fold (local group last), the My Skills search/sort fold (wildcard search included), and one relative-time formatter using a single i18n key family. The My Skills list, the skill card, the detail view, the assignment matrix and the Explore page all import it; the behaviour props that used to carry these functions down from the app binder are deleted, and the binder no longer defines presentation logic. A sibling one-purpose helper owns "persisted view preference" (storage key + codec + ignore-on-failure) and replaces the hand-rolled storage effects across the app. Both modules are pure and get the first direct unit tests of this logic.

**Blocked by:** None (can start immediately; parallel with the backend chain)

- [ ] The same Managed skill renders the same repo label on My Skills, the assignment matrix and Explore
- [ ] Exactly one repo-grouping implementation and one relative-time implementation remain; `projects.*` relative-time keys are gone from EN and ZH; `relative.*` is the only family
- [ ] No component prop carries a formatting/labelling function
- [ ] Pure unit tests cover: wildcard search escaping, the three sort modes, `git+` prefix and `.git` suffix and non-GitHub hosts, local group ordering, relative-time thresholds, preference round-trip and corrupt-storage tolerance
- [ ] All previous storage sites go through the preference helper; storage keys are unchanged so existing users keep their settings
- [ ] `npm run version:check && npm run check` green

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.

### What shipped

- `src/lib/skillPresentation.ts` (pure, no React): `sourceKind`, `repoInfo`, `skillSourceLabel`,
  `groupSkillsByRepo` (local group last), `filterAndSortSkills` (wildcard + regex escaping, three
  sort modes), `formatRelativeTime` (only the `relative.*` i18n family). Colocated tests pin
  `git+` prefix, `.git` suffix, deep URLs, scp-like ssh refs, non-GitHub hosts, local-group
  ordering, the three sorts, wildcard escaping and the relative-time thresholds.
- `src/lib/persistedPreference.ts`: `createPersistedPreference` plus `booleanPreference` /
  `unionPreference` / `stringPreference` codecs, ignore-on-failure on both read and write.
  `src/lib/preferences.ts` defines every app preference once. Tests assert literal keys,
  round-trip, corrupt-value tolerance and throwing storage.
- `src/hooks/usePersistedPreference.ts`: `[value, set]` seeded from storage, written on change.
- Callers migrated: `App.tsx` (binder no longer defines presentation logic or storage effects),
  `SkillsList`, `SkillCard`, `SkillDetailView`, `AssignmentMatrix`, `ExplorePage`,
  `useExploreState`, `useSettingsState`, `useUpdateChecker`, `i18n/index.ts`.
- Deleted behaviour props: `formatRelative`, `getGithubInfo`, `getSkillSourceLabel`.
- `projects.justNow/minutesAgo/hoursAgo/daysAgo` removed from EN and ZH.

### Deviations (with reasons)

1. **Concrete preference definitions live in `src/lib/preferences.ts`**, not inside
   `persistedPreference.ts` — keeps the generic factory free of app specifics while giving the
   storage keys one home the tests can assert against.
2. **`formatRelativeTime(ms, t, now = Date.now())`** rather than a required `now` third-of-three
   parameter order: the `react-hooks/purity` ESLint rule rejects `Date.now()` in component render,
   so the default lives in the pure module. Tests still inject a fixed clock.
3. **Repo-group `href` bug fixed during extraction.** The old `SkillsList` fold built
   `https://github.com/<key>` for any key containing `/`, so a non-GitHub git URL produced a
   nonsense link. The extracted fold carries the href from `repoInfo` and leaves it null otherwise;
   a test pins this.
4. **Explore card author labels now use `repoInfo(...).label`** (falling back to the raw source),
   which is what makes acceptance criterion 1 true. The Explore *installed-match* key
   (`normalizeGithubRepo`) is identity matching, not labelling, and was left alone.
5. **`SkillDetailView` source label** now uses `repoInfo` too (fallback: raw `source_ref`), so a
   deep GitHub URL shows the same `owner/repo` as the other views.

### Verification

`npm run version:check` → Version OK (1.2.1). `npm run check` → lint clean, 125 frontend tests
passing (11 files), build OK, rustfmt/clippy clean, 337 Rust tests passing.
No file under `src-tauri/` or `src/bindings/` changed.
