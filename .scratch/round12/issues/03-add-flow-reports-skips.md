# D3 — Add/import deploys to the full selection; not-installed skips surface as warnings

Status: claimed

Source: BACKLOG #08 (round-11 review C4).

## Change

- `src/hooks/useAddSkillFlow.ts`: `getSelectedInstalledIds` → `getSelectedIds` (selection only, no `isInstalled`
  intersection). `no-targets` remains the empty-selection case. Drop `isInstalled` from the `SyncOrchestration`
  `Pick` if it becomes unused.
- `src/lib/reportOutcome.ts` `syncOutcome`: for `action ∈ {install, bulk}`, every `skipped` result whose error is
  `TOOL_NOT_INSTALLED` becomes one **warning** entry per distinct tool —
  title `errors.syncSkippedNotInstalledTitle` ({{tool}}), message `errors.syncSkippedNotInstalledMessage`
  ({{count}} skills; install + the operator's repair hint: untick the tool in Configure Tools or install it).
  Warnings do not affect `completion` (the install/sync succeeded). `toggle` unchanged.
- `src/i18n/resources.ts`: the two keys in EN and ZH.

## Tests

- `src/hooks/useAddSkillFlow.test.ts`: a selected-but-undetected tool is passed to `syncSkillsToTools`; an empty
  selection still yields `no-targets`.
- `src/lib/reportOutcome.test.ts`: `install` and `bulk` produce one warning per skipped tool with the skill count;
  `toggle` still yields an error; `TOOL_NOT_WRITABLE` skips keep their existing treatment.
