# Round 17 adversarial review

**Verdict: fix-then-ship.** Two must-fix findings; three should-fix findings.

Reviewed `git diff a1a3b94..fea772f` (six commits); HEAD was `fea772f` throughout.
Read AGENTS.md, CONTEXT.md, the spec and all four tickets. Source was not changed.

## M — must fix before release

### M1 — The reviewed plan and the import's scope can differ after changing tool configuration

**Locations:** `src-tauri/src/commands/mod.rs:793` (fresh scope at import),
`src/hooks/useAddSkillFlow.ts:181–220,237–246` (cached plan),
`src/hooks/useSyncOrchestration.ts:384–410` (configuration save).

The backend now applies current settings, but the frontend keeps the plan fetched at mount.
Saving tool selection / scan scope neither invalidates nor reloads that plan, and Review & Import
explicitly reuses it. This is a new dependency introduced by D1 that was not wired through App.

Concrete destructive scenario: start with selected-only scanning and only Claude selected;
`foo` has identical originals in Claude and Cursor. The cached plan contains only Claude.
Turn selected-only scanning off in Configure tools, then open Review & Import: it still shows
only Claude. Import with auto-sync **off** rebuilds an Installed-scope group and removes BOTH
originals, including Cursor's never-reviewed original. With auto-sync on, it can similarly
force-take-over the unreviewed original. Narrowing the scope instead shows excluded candidates
that subsequently fail admission. Starting with an empty plan and widening the scope leaves
no Review button at all (`SkillsList.tsx:95`).

**Evidence:** an inline, mocked-seam probe of the actual `useAddSkillFlow` hook produced
`planFetches: 1`, `modalOpen: true`, `reviewedTools: [claude_code]` while the backend's current
plan contained `[claude_code, cursor]`. The import's rebuild/removal path is directly visible
in `onboarding_import.rs:173,273–287`.

**Smallest complete fix:** pass the saved selection / scan-setting change (or a configuration
revision) through App into the add/import world; invalidate and reload the plan on that change,
reset its choices, and prevent opening/importing a stale plan. Also fetch fresh on Review.
Merely fetching on Review does not fix the empty-plan/no-button case. Keep worlds decoupled;
do not import the sync hook into the add hook. For a stronger cross-window/settings-race
contract, carry a scope revision with the reviewed plan and refuse/re-review if it changed at
import; that fuller fix requires a deliberate wire/spec amendment, not trusting caller paths.

### M2 — Kimi's normal installed layout is now rejected as a deployer's footprint

**Locations:** `src-tauri/src/core/tool_adapters/mod.rs:271–280,825–835`;
`src-tauri/src/core/global_sync.rs:264–265,312–326`.

Kimi's registry detect directory is the generic `~/.config/agents`, not its application
configuration directory. A normally configured Kimi installation can therefore have:

```text
~/.kimi/config.toml
~/.config/agents/skills/foo/SKILL.md
```

The old rule accepted it. The new walk sees only `skills` inside `.config/agents` and rejects
Kimi, even though it is installed and the skill is in its recommended directory. Selecting
Kimi does not rescue sync: global_sync emits `TOOL_NOT_INSTALLED` and never writes it.
`group: Some(AgentsStandard)` is constituent membership, NOT `as_virtual_group()`, so it gets
no exemption. A Kimi-only installation also loses the project's AGENTS-group installed badge.
Amp shares the same registry shape and needs checking at the same seam; the concrete vendor-
verified regression here is Kimi, not an assertion about every other tool's runtime layout.

First-party evidence, fetched during this review:

- [Kimi configuration](https://moonshotai.github.io/kimi-cli/en/configuration/config-files.html):
  default config is `~/.kimi/config.toml`, automatically created on first run.
- [Kimi skills](https://moonshotai.github.io/kimi-cli/en/customization/skills.html):
  `~/.config/agents/skills/` is the recommended generic user-skills directory.

**Fix:** separate the affected registry entries' actual detection roots from their shared
skill convention (for Kimi, the vendor-owned `.kimi` root), keeping detection in the registry.
Revise the new blanket nesting assertion: a legitimate detect root need not contain skills;
the helper already returns “not a footprint” for non-nested roots. Add an installed-tool fixture
with the real layout and a global-sync/project-catalog regression test, rather than putting
`installed.marker` in the generic convention directory.

A smaller presence-only exemption for `.config/agents` would restore old behavior but bends
D2's virtual-group-only exception and reintroduces phantom tools. Do not silently use that
shortcut; the fuller registry correction is preferable. This is a concrete regression to
address now, not a request to execute the entire deferred harness audit (#40).

## S — should fix

### S1 — Overwrite confirmations can orphan waiting actions

**Location:** `src/hooks/useOverwriteConfirmation.ts:30–49`.

`request` overwrites the only resolver without settling its predecessor; there is no unmount
cleanup. Two syncs reaching the ask leave the first promise pending forever. Resolving the
second leaves `pending = null` but does not release the first `runAction`. Unmounting similarly
never settles the outstanding request. The loading overlay reduces ordinary overlapping clicks,
but neither the seam nor `runAction` serializes requests; cancel-and-restart/in-flight overlap
must not depend on a single-render UI guard.

**Evidence:** an inline React hook probe returned first=`pending`, second=`false`, pending=null
after cancelling the second ask; a third request remained `pending` after unmount.

**Fix:** keep resolver ownership in a ref with a request identity and one-shot settlement.
Define a policy for a second request (reject/cancel the newcomer or queue it), settle pending
work false on unmount, and let only the current request clear state. Make cancel read the
current ref. The present `[pending]` callback is fresh in normal rendered use; the issue is
ownership across overlapping requests/retained callbacks, not a missing dependency. Test overlap,
unmount, repeated resolve and an old callback firing after a new request.

### S2 — Directory-iteration errors are swallowed, violating the fail-open detection rule

**Location:** `src-tauri/src/core/tool_adapters/mod.rs:850–858`.

`read_dir` can succeed while an individual iterator item is `Err`. `filter_map(|e| e.ok()...)`
drops those errors. For the sequence `Ok(skills), Err(EIO)`, the code concludes that `skills`
is the only entry, hides the tool, and causes global sync to skip it. This contradicts ticket
02 and the helper's “any read error ... must never hide a tool” contract. Failure opening the
directory is handled correctly; failure enumerating its entries is not.

**Fix:** explicitly propagate any per-entry error to `false` (not a footprint); collect names
fallibly or iterate with an `Err => return false` arm before deciding uniqueness. Preserve the
`.DS_Store` exception. Test a fallible-entry iterator seam deterministically; no unreliable
permission-based fixture is necessary. This finding is control-flow proof, not a reproduced
OS enumeration failure.

### S3 — A thrown retry discards settled first-batch results and bypasses catalog refresh

**Location:** `src/hooks/useSyncOrchestration.ts:340–349`;
`src/hooks/useSkillLibrary.ts:239–244,260–264`.

Suppose the first batch syncs Claude and asks about Cursor. Confirm, then the retry IPC rejects
(a whole-command/transport failure, rather than a report row). The seam throws away the known
first report. Library callers never reach `applyOutcome`, so Claude's persisted target is not
reloaded into the UI; the operator gets only the retry error. This differs from a first-call
failure: successful mutations are already known. Add-flow callers do reload after installation,
but also lose deployment detail. A whole-command error is valid; losing the earlier settled
work is not a good multi-phase failure policy.

**Smallest UI fix:** ensure the library reloads on this thrown path before propagating the error.
**Fuller fix:** preserve the first report plus a separate retry-command error in the frontend
operation result, and let the central outcome fold report both and request reload. Do not
invent per-target failures in the hook or return the old `TARGET_EXISTS` rows as proof that
nothing was overwritten: a lost retry response has an uncertain outcome. That shortcut would
bend the backend-owned settlement/report contract. Add a retry-rejection regression test.

## Checked / not findings

- All production callers of the frontend sync seam pass `overwriteIfSameContent: true`,
  including the changed single-skill bulk handler. `global_sync.rs:113–115` still checks
  content identity before rejecting an occupied target. Identical content retains silent replacement.
- The overwrite and loading backdrops mount as siblings under `.skills-app`; neither is trapped
  in a lower ancestor stacking context. Both are fixed; CSS 2001 beats 2000. Buttons are not
  disabled by loading. Escape/backdrop decline the ordinary current request; the native promise
  settles once. This is source/CSS verification, not a live Tauri click test.
- `mergeRetry` replaces asked pairs with retry skipped/failed rows as well as successes. Shared-dir
  overrides fan out within one registry-owned physical root; normal retry dedupe retains its
  representative. No additional distinct-directory force overwrite was found. The spec expressly
  requests a Cartesian retry and drops unasked rows; those cross-pairs can be reattempted, so this
  is not literally a pair-only backend request. No redesign of that accepted policy is counted here.
- Declined asks remain `TARGET_EXISTS` failures. Sync and install folds render the revised “left
  in place” copy and do not turn those failures into unconditional sync success.
- Unknown selection keys are dropped by settings; a configured empty/all-unknown selection scans
  zero adapters rather than falling back. Corrupt JSON falls back to Installed as ticket 01
  explicitly directs. `total_tools_scanned` counts known scoped adapters once, absent ones included.
- With unchanged settings, originals outside a narrowed plan are deliberately not taken over or
  removed. CONTEXT's “originals ... exactly the ones ... reviewed” documents that boundary. M1
  concerns changing settings while retaining the old plan, not the narrow-scope policy itself.
- Detect-directory symlinks and intermediate links are followed by `read_dir`; a terminal skills
  symlink is not traversed by the footprint test, as specified. Case-sensitive filename comparison
  on a case-insensitive volume can conservatively retain a mixed-case footprint (false positive),
  not hide a tool because of case. Windows junction behavior was inspected, not executed on Windows.
  Project grouping still calls “any constituent installed”; M2 changes the correctness of that input.

## Verification

- `(cd src-tauri && cargo test --all)`: **668 passed, 0 failed**, plus 0 main tests / 0 doctests.
  `git status --short` immediately afterward was empty: no generated-binding drift.
  Initial cargo invocation from the repository root failed for missing Cargo.toml; corrected above.
- `npm run test`: **397 passed, 0 failed, 15 files**.
- `npx eslint src`: exit 0, **0 errors / 0 warnings** (empty output).
- `npm run build`: passed (typescript-7 + Vite); existing dynamic-import/chunk-size warnings remain.
- Actual i18next probe: EN/ZH **500 leaf keys each, exact parity**; `overwrite.body` resolves at
  counts 1 and 2 in both locales. Chinese uses `_other` even for 1, correctly for its plural rules.
- Two inline Bun/jsdom hook probes (no saved source changes): cached-plan divergence and orphaned
  overwrite promises, results above. LSP check of the four changed modal/sync files had no type
  errors; auxiliary style warnings reflect existing extensionless-import/logging conventions.
- Attempted a standalone Rust probe against the compiled library; it could not access private
  `core`, so it supplied no runtime evidence for M2. M2 rests on source inspection + vendor docs.
- No live library mutations, Tauri dev process, commits, or source edits. Only this review is written.
