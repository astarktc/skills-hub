# Design inputs — T3 Code: project defaults and overrides (2026-11-06)

Read-only research for round 20 Wave B (#109 / #84), especially `.scratch/round20/spec.md` Grill log Q3, Q17,
Q21 and Q22. No implementation or spec change. **Provenance:** T3 Code's public source, README and user docs at
commit **6577c6b6185167c0e502fa01f9e8e8f2dee6ff25** (2026-11-06). All T3 links below are pinned to that commit.
**Confidence:** VERIFIED means directly documented or visible in source; DERIVED means a consequence of the code,
not a separately observed interaction. Recommendations are proposals, not T3 features or approved Skills Hub decisions.
The installed `/Applications/T3 Code (Beta).app` reports **0.1.6** in `Info.plist`; its source revision was not
established. This is a current-source study, **not a claim that every detail ships in that installed build**.
No live app, real project settings or credentials were opened or changed.

## Summary

1. **T3 Code does have the comparable pattern.** A sparse per-project override record sits alongside environment
   defaults. Missing project keys inherit; present keys override, even when their value equals the default. [P2–P4]
2. **One Settings surface, two scopes:** “Applying settings for [All projects / project] across [environments]”.
   The same category and row edit defaults or project overrides; Projects is not a duplicate settings form. [P1, P6]
3. **Show the effective value, explain its origin.** A layers icon beside each server-setting title opens a precedence
   chain. An override gets a primary-colour icon and a reset affordance. Inheritance is not merely an empty input. [P7–P8]
4. **Defaults can expose their exceptions.** The row's layers popover counts and lists overriding projects, their values,
   links to them, and “Reset it” / “Reset all”. This is a per-setting bulk view, not a project-by-setting matrix. [P8–P9]
5. **Do not copy its multi-machine semantics as “global”.** “All environments” bulk-writes connected servers;
   it is explicitly not a synchronized global default, and offline servers retain their old settings. [P1, P9]
6. **Borrow the sparse model, shared rows, source chain and reset.** Extend the chain to Wave B's
   folder › project › scope › built-in. Unlike T3's ordinary settings saves, Skills Hub must present a redeploy
   confirmation and settle one action with a report; resetting an override has those consequences too. [L1]

## § 1 — Data and resolution

### Where values live (VERIFIED)

- `packages/contracts/src/settings.ts` defines `ServerSettings`, decoded defaults (`DEFAULT_SERVER_SETTINGS`), the
  explicit `PROJECT_SCOPED_SERVER_SETTING_KEYS` allow-list, and `ProjectSettingsOverrides`. The server-settings
  object contains `projectSettingsOverrides: Record<ProjectId, ProjectSettingsOverrides>` (default empty). It is
  **not a complete copy of defaults on each project**, nor a separate file in every repository. [P2]
- The server persists settings in `settingsPath`: `<baseDir>/userdata/settings.json` in the normal path derivation,
  or `<baseDir>/dev/settings.json` for the indicated development case. `serverSettings.ts` strips built-in/default
  values before an atomic write and publishes settings changes. This is separate from `statev2.sqlite`. [P5]
- Legacy project aggregate values and older per-setting maps are folded into the canonical override map once.
  `projectSettingsFolded` prevents stale legacy fields from resurrecting an override after reset/restart. [P5]
- Device/client preferences are a separate class: appearance, confirmations and browser profiles ignore the project
  selection. Server settings can be environment-wide or project-overridable. [P1, P9]

### “Inherit” is presence, not value equality (VERIFIED unless marked)

| Representation | Meaning | Source |
| --- | --- | --- |
| No project entry, or key absent from its entry | Inherit that setting | P2, P3 |
| Key present with a valid value (including `false`, an empty list, or an equal-to-default value) | Explicit project override | P2–P4; equal-value consequence DERIVED |
| Present `undefined` after forward-compatible decoding | Ignored, not an override | P3 and its regression test |
| `null` for a supported nullable override, e.g. `sidebarAutoSettleAfterDays` | A real value, e.g. “Never”, **not reset** | P2–P3, P8 |
| A project's entire entry is `null` in an update patch | Delete the project's entry | P4 |
| Clearing selected override keys | Delete those keys; return `null` if the entry becomes empty | P3, P9 |

Do not generalize `null` into one sentinel. The UI patch planner treats `null` as removal for keys not in
`isNullableProjectSettingsOverride`, but preserves it for that explicit nullable set. The normal reset path calls
`clearProjectSettingsOverrides`, so it can remove a key even when `null` is a legitimate value. [P2, P3, P9]

### Resolver and propagation (VERIFIED / DERIVED)

`packages/shared/src/projectSettings.ts::resolveProjectSettings` returns **effective settings + per-key sources + raw
overrides**. It starts from environment settings and replaces each allowed key whose project value is present and not
`undefined`. Object-valued overrides generally replace a whole setting; provider enabledness has a special per-instance
resolver. This is a per-key cascade, not a universal recursive merge. The UI completes partial object edits from the
target's effective value before storing the whole object. [P2–P3, P9]

For repository-file-backed settings (workspace and submodule initialization are documented examples), resolution is:

**project override › non-null environment value › repository `t3.json` › built-in value.**

The file tier is applied only to designated `PROJECT_FILE_BACKED_SETTINGS` when the earlier resolved value is `null`.
It is **not** a tier for every setting. T3's ordinary source enum is `project | environment | t3.json`; the UI further
distinguishes built-in from a customized environment value. [P1, P3, P8]

**Changed default:** the project map is left intact. On the next resolution, an absent project key reads the new
value; an explicit project key keeps its own value. There is no need to rewrite every inheriting project. This
propagation is DERIVED from the resolver and patch code, and explicitly described by the user docs. The server emits
settings updates; the scoped UI resolves against current server config. This does **not** mean all existing work is
retroactively changed: new-thread defaults concern new threads, and browser access changes apply when an agent
session next starts. [P1, P3–P5, P9]

## § 2 — UI pattern

### Shared categories with a scope sentence (VERIFIED)

`SettingsScopeSentence.tsx` puts **“Applying settings for … across …”** above applicable Settings pages, with project
and environment pickers. It starts at All projects / All environments and retains scope across category navigation
and search. Device-only pages omit the sentence. Providers and diagnostics target one machine; most other server
settings fan out to the selected connected environments. [P1, P6, P9]

Selecting a project changes the meaning of the **same row** to “edit this project's override”. The Projects category
holds project identity, actions, checkouts and removal; defaults such as workspace, source control and browser access
stay in their topical categories. Thus “project settings” is a scope, not only a dedicated Project modal. [P1]

### Effective control + origin + reversible override (VERIFIED)

`settingsLayout.tsx::SettingsRow` composes ordinary controls with `SettingInheritance` and `SettingResetButton`. [P7]

| State | Presentation / affordance | Source |
| --- | --- | --- |
| Project inherits | Control gets resolved value; layers tooltip says “Inherited from {environment}” or “Inherited from the repository's t3.json”; Project layer says “Inherits” | P7–P9 |
| Project overrides | Layers icon uses primary colour; tooltip “Overridden for this project”; separate reset button with tooltip “Reset to inherited value” | P7–P8 |
| Inspect origin | Click layers icon: ordered Project / Environment / `t3.json` when applicable / Default list, values on each line, effective line highlighted with a check | P8 |
| Multiple targets differ | “Mixed” control state; amber/warning layers icon; tooltip “Mixed across selected environments”; choosing one value writes to selected targets | P1, P7–P9 |
| Default row with exceptions | Tooltip includes count of project overrides; popover adds “Overridden by” with project names, values, links and “Reset it” / “Reset all” | P8–P9 |
| Setting cannot be project-overridden | Visible but inert with explanation “Environment-wide setting. Select an environment to change it.” | P7, P9 |

The source chain, tooltip and reset are the general mechanism. There is **not a universal “Use default” option inside
every dropdown**, nor a permanent textual override badge on every row. Some file-backed controls expose “Inherit”;
mobile documents a page-level “Use defaults”. Locally-set/built-in row markers without project exceptions wait for
hover/focus, whereas inherited/overridden project markers remain visible. [P1, P7–P8]

**Bulk visibility boundary:** the inspected implementation has a **per-setting exceptions list** (including reset-all),
not an all-settings matrix showing every project's override fields. The docs and implementation substantiate the
former; no broader bulk matrix was found in this study. [P1, P8–P9]

## § 3 — Edge cases worth retaining

| Case | What T3 does | Evidence / boundary |
| --- | --- | --- |
| Change a default while projects override | Does not overwrite project entries; exceptions remain visible from the default row | P1, P4, P8 |
| Remove one override | Delete only selected keys, preserving unrelated keys; delete an empty entry | P3–P4, P9 |
| Explicit value equals inherited value | Still a project override because resolution tests presence, not equality; it stays pinned on future default changes | DERIVED from P3–P4 and P9; no dedicated equal-value test claimed |
| Nullable value vs inheritance | Preserve a legitimate nullable value; reset by deleting the key | P2–P3, P9 |
| Object override | Snapshot a whole setting from an effective base, not a deep inheritance promise for every nested property | P2–P3, P9 |
| Offline / partially successful bulk saves | Only connected environments are targeted; save result separates saved and failed environments | P1, P9 |
| Older server silently ignores a setting | Compare saved response against requested change and report unsupported setting rather than leaving an unexplained Mixed state | P9 |
| Setting not project-overridable | Explicit allow-list plus UI read-only state and planner refusal, rather than hiding the row or accidentally saving globally | P2, P7, P9 |
| Legacy data after reset | One-time migration flag prevents old aggregate values from restoring the override | P3, P5 |
| A provider is disabled for a project | Certain model overrides are ignored/fallback-resolved, rather than mechanically selecting an unusable provider | P3; specialized constraint, not generic precedence |

## § 4 — Carry-over to Skills Hub (recommendations, not observed T3 behavior)

1. **Use sparse typed overrides and one pure resolver that returns `{ value, source }`.** Persist only explicit values;
   derive effective ones. Keep an explicit equal-to-default value pinned until reset. For booleans, do not collapse
   `false` into “unset”. `Option<T>`/wire `null` can mean inheritance for these non-nullable preference domains if
   the contract says so; do not copy T3's nullable-value ambiguities. [P2–P4]
2. **Share rows between defaults and project scope.** Put the default controls in Settings → Harnesses and provide
   a plainly visible project-scope selector or a project entry point that opens those same rows. Label the default
   scope **“All projects” / “Project defaults”**, distinct from **“Global deployments”**. Wave B has two scope
   defaults, not T3's multiple independent environments. [P1, P6; L1]
3. **Display effective values with explicit source text and reset.** For the shared-folder toggle and `.agents` /
   `.claude` choice, show “Inherited · On” / “Inherited · .agents”; editing makes a project override. Offer
   “Use project defaults” per overridden row. A source popover can carry detail without overloading the folder cards.
   For link/copy, expose folder › project › relevant scope › default (Link), highlighting the winner. [P7–P8; L1]
4. **Show exceptions from the default.** Start with “N projects override this” and a project/value list linking to the
   project, rather than a large comparison table. A bulk reset is useful but consequential: include its affected
   projects in Skills Hub's confirmation, never silently fan out because T3 does. [P8–P9; L1]
5. **Keep deployment policy authoritative in Rust.** `core/settings.rs` already owns typed storage/default rules;
   give project/folder overrides an equally typed persistent backend home and resolve them once per operation.
   `src/lib/preferences.ts` remains the home for view preferences (e.g. remembered UI scope), not a browser-local
   duplicate of deployment policy. Pass settings/actions through existing plain hooks and App; no Context/Zustand
   or T3-specific Effect/client-runtime machinery is needed. [L2, AGENTS.md]
6. **A settings edit or reset is one consequential operator action.** Confirm the affected Deployments, save the
   intended policy and run redeploy under the existing lifecycle, then show one settlement report. An override
   reset may change folders/modes just like a new selection. Preserve Q17: declining the first-launch move does
   not turn the desired preference off; show the remaining move work. Partial/declined effects must not masquerade
   as complete application of the new policy. Exact transaction/recovery details remain for the Wave B spec. [L1]

**Do not carry over:** multi-machine Mixed-state complexity unless Skills Hub actually introduces multiple targets;
repository `t3.json`; provider/model fallback rules; or automatic ordinary-settings persistence as a substitute for
redeploy confirmation/reporting. Copy-only capability constraints must still win over a requested Link, with an
explanation of requested versus effective mode (Q21), not an apparently working but ineffective override. [L1]

**Spec acceptance probes to add:** inheriting project follows changed default; pinned equal-value override does not;
reset preserves other override keys; toggle Off survives reload; folder mode beats project/scope; copy-only constraint
is explained; a default change lists affected inheritors and unaffected exceptions; declined/failed redeploy leaves
honest pending work. These are proposed checks, not tests run by this research.

## § Sources

All inspected 2026-11-06. P = first-party T3 source/docs, L = local Skills Hub requirements/source. T3 commit:
`6577c6b6185167c0e502fa01f9e8e8f2dee6ff25`. Source inspection only; no browser usability or installed-build parity test.
No secondary comparison was needed: T3 has the requested comparable mechanism.

| Id | Source | Note |
| --- | --- | --- |
| P0 | [README.md](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/README.md) | Product identity, Electron desktop, link to project-settings docs |
| P1 | [docs/user/project-settings.md](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/docs/user/project-settings.md) | User-facing scope, layers, exceptions/reset, file precedence, mobile differences and effective-time caveats |
| P2 | [packages/contracts/src/settings.ts](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/packages/contracts/src/settings.ts#L1067-L1166) | Override allow-list, sparse schema, nullable-value rules; same file contains ServerSettings and override patch schema |
| P3 | [packages/shared/src/projectSettings.ts](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/packages/shared/src/projectSettings.ts) · [projectSettings.test.ts](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/packages/shared/src/projectSettings.test.ts) | Resolver, source provenance, clear helper; tests for undefined/null, precedence and clearing |
| P4 | [packages/shared/src/serverSettings.ts](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/packages/shared/src/serverSettings.ts) | Canonical patch merge, entry replacement/removal and preservation of unrelated entries |
| P5 | [apps/server/src/config.ts](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/apps/server/src/config.ts#L137-L170) · [serverSettings.ts](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/apps/server/src/serverSettings.ts) | File paths; stripDefaultServerSettings, foldLegacyProjectSettings, atomic writes and change publication |
| P6 | [apps/web/src/components/settings/SettingsScopeSentence.tsx](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/apps/web/src/components/settings/SettingsScopeSentence.tsx) | Applying-settings sentence, scope menus and device-only pages |
| P7 | [apps/web/src/components/settings/settingsLayout.tsx](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/apps/web/src/components/settings/settingsLayout.tsx#L340-L545) | SettingsRow inheritance state, read-only reasons, reset, marker visibility |
| P8 | [apps/web/src/components/settings/SettingInheritance.tsx](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/apps/web/src/components/settings/SettingInheritance.tsx) | Layers, labels, colour states, winner check, exceptions list and bulk reset |
| P9 | [apps/web/src/components/settings/scopedSettings.ts](https://github.com/pingdotgg/t3code/blob/6577c6b6185167c0e502fa01f9e8e8f2dee6ff25/apps/web/src/components/settings/scopedSettings.ts) | Effective targets, per-key writes, clear planning, override listing, mixed values, partial-save/unsupported-version handling |
| L1 | `.scratch/round20/spec.md` § Wave B → Grill log Q3, Q17, Q21, Q22 | Approved consequential setting changes, desired policy, four-level mode precedence, project overrides |
| L2 | `src/lib/preferences.ts`; `src-tauri/src/core/settings.rs`; `AGENTS.md` | View-preference seam, backend typed-settings ownership, plain-hook/action/DTO architecture |
