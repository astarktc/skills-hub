# Ticket 21 — Spec review and adversarial audit

**Verdict: FAIL.** One destructive-path blocker (F1), plus correctness/accessibility findings that need fixing before the next ticket.

Reviewed the staged deletion, unstaged diff and new files in `t3/import-review`, HEAD **`8b42ce5`**. Source remained read-only; no commits. This is an independent review, not an endorsement of the child's gate. Browser evidence used **my own fixture server on 5184**, ego-browser task space 12, and only in-memory data. Captures are under `/tmp/ticket21-*`. The browser space was closed after testing. Web Interface Guidelines were fetched fresh and applied, with the repo's documented exceptions taking precedence.

## Spec

### (a) Asked-for requirements missing or partial

- “**Report lists removed / left-in-place originals per Tool**”: shared-path divergent removal loses the second harness's outcome (F6). The new irreversible path lacks the requested divergent-symlink and comparison-failure Rust regressions; existing passing symlink tests exercise the old identical-copy path, not this new branch (F1 and the destructive-path coverage below).
- “**Migration: `ImportModal` joins `migrated`; its rules leave `App.css`**”: the replacement is correctly migrated. However, the earlier Comment also explicitly asks to “**rename `list_all_skill_target_paths` to say ‘owned’**”; that rename is still missing (F9).
- “**unmanaged skills stay visible and revisitable**”: valid remaining groups stay visitable. If only invalid entries remain, the banner disappears; the review can no longer show those entries. This is a boundary caveat, not a claim that invalid manifests are importable skills.

### (b) Scope creep

No material unjustified scope creep. The keep/remove backend change is required by “**offer the keep/remove choice he asked for**”; description/mtime/comparison fields and `central_path` support the specified UI and one ask. The hook split, Result-until-Close and after-Close cascade match the accepted decisions. ≥10 was already the threshold in DESIGN.md, not a new arbitrary product decision.

### (c) Implemented but wrong

- “**A Conflict's unchosen copy … State each variant's fate**”: unreadable/unknown identity can be treated as divergent and deleted, including a byte-identical copy (F1).
- “**Add a Result step with the D11 count line**”: an all-Keep Replace still counts as imported; skipped target outcomes can produce a green, empty “Replaced” row (F3–F4).
- “**Astra audit … keyboard-only review**”: Close/Escape settles focus on `<body>`, not the opener/fallback (F2). The Result's toast can cover and intercept its Close button (F5).

## Findings

### F1 — P0 blocks commit: unknown identity authorizes irreversible deletion

**Evidence:** `src-tauri/src/core/onboarding_import.rs:655–660` first uses `path.symlink_metadata().ok()?`, then skips removal only when `content_identity::same_content(...)` is true. Every false result goes to `ensure_path_within_tool_dirs(...).and_then(remove_path_any)`.

But `src-tauri/src/core/content_identity.rs:61–68` returns false for **unknown** identities as well as different bytes (`_ => false`). Its own contract at :29 says “Unknown identity must not be interpreted as drift.” This was a safe conservative answer for the old keep/overwrite-refusal paths; negating it to authorize deletion is not safe.

**Concrete regression construction:** seed three variants of `alpha`: A and C with identical bytes, B with different bytes. Keep `SKILL.md` readable in all three, but make a non-manifest content file in C unreadable while leaving C's parent writable. The plan still has a Conflict from A/B; C's fingerprint is unknown. Select A, `remove_divergent=true`, `auto_sync=false`, `remove_originals=false`. A is finalized successfully. The new branch treats C's failed hash as a mismatch and attempts to delete C, although its bytes match A and the identical-original policy says leave it. File read permission and permission to unlink it are independent. A transient hash/store failure after finalization can reach the same branch. This is a source-proven unsafe authorization, **not a claim that I executed this new Rust regression**.

Separately, `.symlink_metadata().ok()?` silently drops non-NotFound metadata failures. An inaccessible copy is not equivalent to an absent one and must not disappear from the report.

**Fresh test evidence:** `cd src-tauri && cargo test onboarding_import` → **27 passed**, including all four new remove-divergent tests. None supplies an unknown identity or metadata-read failure. Passing those tests does not cover this authorization defect.

**Exact proposed fix:** make deletion require two successfully read identities that are unequal. Preserve a known-equal copy for the existing originals policy. On an unknown comparison or a non-NotFound metadata error, retain the path and return an `OriginalStatus::Failed { error: CommandError }` explaining the failed comparison/read; do not authorize deletion. Protect the selected source explicitly as well. Add the A/B/C regression, unknown-managed-identity regression, and metadata failure test; assert disk bytes and every report row, not only counts. Keep the existing guard immediately before the actual removal.

### F2 — P1 fix before next ticket: closing the Result loses keyboard focus

**Evidence:** repeated ego-browser runs on `first-run` after both a partial import and a full import:

- Keyboard flow at 960: choose commit-message with arrows, Space on remove-other, tdd → Replace, Shift+Tab to Keep existing, Enter, Result, Escape.
- After Escape and two animation frames: `{ focus: 'BODY', banner: 'Review 4 discovered skills' }`.
- After partial failed import: `{ focus: 'BODY', banner: 'Review 6 discovered skills' }`.
- Full import, after settlement/cascade: `{ focus: 'BODY' }`.

This persisted into subsequent browser calls; it was not merely an immediate unmount frame. Result entry itself correctly focuses its heading.

`src/components/skills/SkillsList.tsx:185,254–255` disables the opener during the asynchronous scan. `useImportReview.ts:150–163` opens only after that scan. `ImportReviewDialog.tsx:88` supplies initial focus but **no caller fallback**. The Dialog's opener capture can therefore see no usable opener; its documented fallback contract is not satisfied by this call site. Keep the fix in the shared focus contract, not a new surface Escape listener.

**Exact proposed fix:** keep both Review invitation buttons focusable while disabled (`focusableWhenDisabled` + refusal), and give Import's Dialog a stable fallback resolver to the surviving Review button, otherwise the My Skills heading/main landmark. Test delayed scan → dialog → Result → Close/Escape, both with a surviving banner and a fully consumed plan. Assert the exact final element after frame settlement.

### F3 — P1 fix before next ticket: “imported” counts Replace, even when nothing was replaced

**Evidence:** browser at 960, eight new groups plus tdd Replace, all three tdd collisions answered Keep:

```text
headline: 9 imported · 3 kept existing
rows: 8 × Imported, tdd = Kept
library: 2 before → 10 after
```

Capture: `/tmp/ticket21-result-kept-960.png`. Full import with ten new groups and two successful Replaces says **“12 imported”**, although only ten skills were added: `/tmp/ticket21-full-result-1280.png`.

`src/lib/reportOutcome.ts:365–368,1131` counts each `status: replaced` as done and feeds `counts.imported + counts.replaced` to the **imported** noun. `src/lib/importReview.ts:445–446` separately recognizes all-Keep as `kept`, so the two displayed accounts disagree. The child's claim “9 imported · 3 kept existing (tdd Kept)” accurately describes the bug, not correct accounting.

**Exact proposed fix:** have the shared import settlement fold derive distinct group outcomes from settled target rows: imported, actually replaced, kept, skipped/no-op and failed. Feed the composite headline correctly named counts; do not count an all-Keep/all-skipped Replace as imported or replaced. Keep target-level declines as separately named **folders/targets**, not an unlabeled group tally. Add tests for eight imports + all-Keep Replace, ten imports + two successful Replaces, all-skipped Replace and mixed Replace. The group counts should reconcile to sent selections; target counts are additional dimensions and must not be summed as groups.

### F4 — P1 fix before next ticket: skipped Replace targets disappear into a green success

**Evidence:** independent fixture-module probe in the served page (not a component stub): build `first-run`, set the managed tdd's `dto.disabled_at`, then invoke `importOnboardingSelection` with tdd `replace:true`. The returned three targets are all:

```text
{ status: 'skipped', error: { code: 'TARGET_DISABLED', tool: ... } }
```

Passing that report to the real `importResultRows` returned:

```text
{ name: 'tdd', status: 'replaced', lines: [], attention: false, retryable: false }
```

`src/lib/importReview.ts:401–404` only keeps skipped rows with `TOOL_NOT_WRITABLE`; other skip reasons vanish. At :445–446 the group falls through to `replaced`. `ResultItem` then paints it green (`ImportReviewDialog.tsx:648–656`).

**Exact proposed fix:** render non-failure skips as neutral per-harness facts with their reasons (disabled, not installed, etc.). Derive Replace's top-level presentation from its settled targets: no successful replacements must not become green “Replaced”. Do not turn deliberate skips into errors or enable disabled skills. Test disabled-all, uninstalled-all, mixed synced/skipped and mixed declined/unanswered collisions. Also tighten `kept` to require every relevant target to have been kept, rather than merely one decline and no noncollision failure.

### F5 — P1 fix before next ticket: the toast can physically block Close

**Evidence:** the child's reported overlap is real, and not limited to a short-lived success toast. At 1280×800, move the fixture's pdf source from Claude Code to Cursor **after opening Review**, then import the seven defaults. Result: **“6 imported · 1 failed”**. Its toast covers Close. Capture: `/tmp/ticket21-partial-failure-1280.png`.

The subsequent command `page.click('[data-slot="dialog-footer"] button')` failed with:

```text
matched 1 elements, but none can receive input; <div> intercepts pointer events
```

Keyboard Escape still works. At 960 the transient toast geometry also intersected the footer (toast x580–936/y544–602, footer x121–839/y550–615); after animation it covered a result row instead. The full-import success capture also shows content occlusion. `useImportReview.ts:240–241` opens Result and immediately emits the duplicate fold toast; `App.tsx:607` fixes the Toaster bottom-right, lifted 6rem. #80's placement fix protected the page header/rail, not this modal. “Keyboard can still reach it” is not sufficient.

**Exact proposed fix:** centralize an inline-Result presentation option in the reporter: still apply reload/history/outcome accounting, but defer the duplicate toast until the Result closes (the focused Result heading and rows already present the outcome). Preserve the accessible live host and do not make all toasts pointer-transparent. Alternatively reserve real, nonoverlapping space for a modal-aware toast stack. Re-test short and tall Results, success and persistent failure toasts, at both sizes; pointer and keyboard Close must both work.

### F6 — P1 fix before next ticket: a shared directory is removed once but reported for only one harness

**Evidence:** source trace across `onboarding.rs:192–231` (scans every included adapter), `tool_adapters/mod.rs:280–298` (Amp and Kimi both use `.config/agents/skills`), and `onboarding_import.rs:432–453,655`:

1. With both harnesses installed, the real plan contains two variants naming the same physical path.
2. The first divergent variant removes the path and produces `Removed` for its harness.
3. The second sees missing metadata and returns `None`.
4. `settled` excludes **all** variants with that path from subsequent settlement.
5. Only the first harness has a removal row, contrary to the per-harness report requirement.

This is a source-derived case, not a newly executed Rust test. The fixture cannot establish parity here: `fixtures/backend.ts:1298–1305` deduplicates `seenPaths` while the Rust plan does not. My independent fixture construction of Claude + Amp + Kimi indeed produced only Claude + Amp plan variants, masking the second-harness case.

**Exact proposed fix:** settle removal once per physical/as-recorded path, retain the resulting status, then fan that status out to every plan variant/harness referencing the path. Do not call removal twice and mistake the second absence for “nothing to report”. Add a Rust test with both Amp and Kimi marked installed, a chosen Claude copy and their shared divergent directory; assert one deletion and both harness outcomes. Bring the fixture's plan/settlement treatment into parity with the chosen backend count contract.

### F7 — P2 note: an intentional keep is still “to review”

**Evidence:** auto-deploy off, choose handoff's Claude copy, leave remove-other unchecked, keep the identical-originals removal checkbox checked. The real UI Result says **“8 imported · 1 original to review”** and **“Your different copy kept in Pi”** (`/tmp/ticket21-off-result-960.png`). `reportOutcome.ts:1105–1114` counts every `kept_divergent` as a warning/review item regardless of the explicit keep/remove model.

This is not data loss, and the copy remains revisitable. It is misleading feedback after the operator has answered the question. The child's note is confirmed.

**Exact proposed fix:** carry or fold the run's explicit keep decision so intentional retention becomes a quiet “1 different copy kept” note. Reserve “to review” for an unresolved collision or an actual unknown/failure. Keep the banner for the retained unmanaged copy. Do not globally suppress all old `kept_divergent` warnings without preserving the intent distinction.

### F8 — P2 note: `git diff --check` is red in generated bindings

**Evidence:** fresh `git diff --check` exits **2**:

```text
src/bindings/index.ts:509: trailing whitespace.
src/bindings/index.ts:531: trailing whitespace.
src/bindings/index.ts:536: trailing whitespace.
```

`cargo test export_bindings` is stable, so this is generated output, not hand-edit drift. The child's stated gate did not include `git diff --check`.

**Exact proposed fix:** normalize trailing whitespace in the binding-generation/export path (and pin that normalization), then regenerate. Do not hand-edit generated DTOs or merely trim the current artifact so the next export reintroduces it.

### F9 — P2 note: the binding “owned paths” rename was not implemented

**Evidence:** ticket Comment 2026-09-29 explicitly asks to rename `list_all_skill_target_paths` to say “owned”. `src-tauri/src/core/skill_store.rs:1120` and `onboarding.rs:109` still use the old name. This does not change ownership behavior, but the Comment is not fully satisfied.

**Exact proposed fix:** rename the method and callers/tests to `list_owned_skill_target_paths`; leave its filtering unchanged. If intentionally deferred, obtain and record an explicit disposition rather than implying every Comment is complete.

## Verification and coverage

### 1. Destructive path

- **Fence:** both removal sites in `onboarding_import.rs` call `ensure_path_within_tool_dirs` before `remove_path_any` (:659 and :683–695). The selection cannot supply arbitrary deletion paths: admission chooses a freshly scanned group. The guard is a **lexical registry-root check**, not a claim that ancestors cannot be symlinks (`tool_adapters/mod.rs:868–881`). I found no new direct unguarded call.
- **Equal bytes:** `remove_divergent_removes_only_the_copies_that_differ` passes and preserves the chosen identical original under `remove_originals=false`. F1 explains why this is not a proof for unknown comparisons.
- **Replace:** `Admission::Replace` directly calls `replace_with_managed_unlocked`; the remove-divergent loop exists only in `apply_one_unlocked`. `remove_divergent_never_touches_a_managed_name_group` passes. Differing Replace copies still raise askable `TARGET_EXISTS`.
- **Symlink construction/read-through:** construct Claude `alpha` as chosen, Cursor `alpha` as a symlink to a divergent Codex `beta` directory (different group, not selected). `remove_divergent_original` passes Cursor's **link path**, not its target, to `remove_path_any`. `sync_engine.rs:420–431` uses `symlink_metadata`, then `remove_file` for a symlink, so it removes the link, not `beta`. If the referent itself is another divergent `alpha` variant, that separate variant may intentionally be removed too; that is not following the link during unlink. An absolute link into the central repo is excluded by `is_unmanaged` (`onboarding.rs:291–309`). Existing passing `auto_sync_on_takes_over_every_identical_original_in_the_group` physically constructs a Tool-to-Tool symlink, but it exercises **the identical sync path**. **The requested new destructive-symlink Rust test was not present and was not added under the source-read-only constraint.** Add the divergent link, same-content link and central-link cases before calling that proof complete.
- **Keep default:** `defaultChoices` sets `removeOthers:{}`; `ImportReviewDialog` renders it unchecked; `importSelections` omits `remove_divergent` unless the checkbox is true. Serde defaults the optional input to false. Browser initial state showed all three remove-other controls unchecked. The targeted hook/fold tests verify both omitted default and explicit true payloads.
- **Failures:** actual removal errors are classified into `OriginalStatus::Failed`, not thrown from a group, and feed skill health. Independent fixture invocation with Pi unwritable returned `{status:'failed',error:{code:'OTHER',message:'Permission denied ...'}}` alongside the successful imported group; `importResultRows` rendered the Pi failure. Unknown metadata errors are the exception described in F1. Shared-harness completeness fails F6.
- **Order and loss:** finalization → divergent deletion → global sync. There is **no rollback/backup** if deployment subsequently fails. The divergent bytes are gone and a failed/skipped target can have no link; the central chosen copy remains. The checkbox says the differing copies “are deleted”, and their fate says “Removed”, not “safely replaced”. This is consistent with an explicit independent delete choice, not an atomic migrate guarantee. A short warning that deletion is permanent even if deployment fails would improve informed consent. Do not advertise transactional replacement.

### 2–3. Words, counts and defaults

Independent `first-run` plan computation: **12 groups, 22 variants, six harness keys** (`codex`, `gemini_cli`, `claude_code`, `amp`, `cursor`, `pi`), one invalid entry. Header matches exactly. Managed fixtures carry both comparison states: frontend-design `[true,true]`, tdd `[false,false,false]`. Rich header independently reads **5 skills · 7 copies in 5 harnesses**; failures scenario exposes **Review 9 discovered skills**.

Match explicitly says copies agree **with each other**. Managed names appear in their own section, say the name is already taken, and show same/different **before** Leave/Replace. Conflict descriptions, modified times, harness/path radio names and changing fates were present. `broken-skill` is disabled with “Can't be imported: Invalid frontmatter”. Seven new non-conflict groups start checked; three conflicts unchecked; managed groups Leave; invalid unchecked/disabled.

Checking a conflict alone **does not block Import**: it uses the visibly preselected default variant (the existing non-link preference). Choosing another radio checks the group. D14 requires the **group** to start unchecked, not an initially unselected radio; this is consistent with D14, though the interaction should remain explicit in docs.

Auto-deploy on names the selected harnesses and “Import and deploy”. Off initially shows “Remove my 11 identical originals …” checked; selecting handoff makes 12. Keyboard Space changes the button to **Import**, then back to **Import and remove 12 originals**. The off result correctly lists removed/retained originals. Policy text wraps within the available width; the footer remains inside the viewport at 960×640.

### 4. One ask

Observed one top-level **Folder already exists** popup for the three tdd collisions (MutationObserver count **1**). A full import collected **six collisions across four groups** into one popup. The implementation uses the shared `OverwriteModal`/Dialog rather than ConfirmDialog so Escape is distinct from Keep; it is not a second confirmation system.

`useImportReview.ts:173–208` builds sync parts with `source_path: status.central_path`, one `settleCollisionParts` call, then replaces corresponding report targets. The hook test pins that source and answered report. Keep settles to `declined`, recorded as quiet notes by the shared fold; the Result displays the answers (headline defect F3 remains). Dismissal was tested on handoff → Pi: Result **“8 imported · 1 folder to review”**; the Pi target row was absent both before and after Escape. No retry/Keep marker was sent on dismissal; `settleClaims` returns early for `dismissed`.

### 5–6. Result, revisit and keyboard

Result stays until Close. A source moved after Review generated **6 imported · 1 failed**, one failed pdf row with Retry. Reopening showed only six remaining groups, pdf **unchecked** with “Last import failed: Item not found …”; all six successful imports were gone from Review. In a separate run, restoring pdf's source and clicking **Retry pdf** added exactly one new ID, retained the eight prior IDs, updated the merged headline to **7 imported**, removed Retry, and kept focus on the Result heading. The hook test additionally pins the single-group request payload.

Partial import banners remained at **Review 4** / **Review 6** as appropriate. A full import/Replace/Overwrite consumed all valid groups; no Review banner remained, while fixture `foreign` still contained invalid `broken-skill`. That invalid-only state has no review entry point (the banner tests `group_count`, not `invalid.length`).

Keyboard walks at 1280 and 960 reached group checkboxes, each selected conflict radio, remove-other, both managed radiogroups, policy control when off, and the action/footer. ArrowDown selected Gemini for commit-message and Replace for tdd; Space toggled remove-other and policy; Enter submitted and answered the ask. Native radio names include harness + path. Result entry and successful Retry focus the heading. **The final focus-return proof fails F2.** The pointer Close proof fails F5. Some early automation observations were made before initial focus/settlement; they were retried with explicit focus waits and are not used as standalone findings.

### 7. Cascade

Initial mount: **0 lit marks**. The large-import run added ten new skills, then Close produced per-imported-row mark delays **0, .02, .04, .06, .08, .10, .12 seconds**, duration **.12s**. Delayed entry styles later withdrew to **0 elements**. Existing Replace rows can still carry their ordinary `useSyncLight` marker; that is not a repeated import cascade.

Under CDP `Emulation.setEmulatedMedia` with `prefers-reduced-motion: reduce`, a second ten-import run produced settled marks with **transition-duration `0s`, transition-property `none`, opacity `1`**, then withdrew entry styles. Capture: `/tmp/ticket21-reduced-cascade-960.png`. The threshold is `LANDING_MIN_SKILLS = 10` in `useImportReview.ts:47`, checked after Result Close at :293; auto-deploy off does not qualify. It matches DESIGN.md's pre-existing “large import” threshold. Timing is per strip, not a ten-row serial cascade, as documented.

### 8–10. Wire, migration, docs and gates

Fresh commands in this worktree:

| Command | Result |
| --- | --- |
| `npx vitest run src` | **54 files, 1167 tests passed** |
| `npx vitest run src/lib/reportOutcome.test.ts src/lib/importReview.test.ts src/hooks/useAddSkillFlow.test.ts` | **3 files, 197 tests passed** |
| `npx tsc --noEmit -p tsconfig.app.json` | exit **0** |
| `npm run styles:check` | **606 counted lines, 40 migrated files**, self-test passed |
| `cd src-tauri && cargo test onboarding_import` | **27 passed**, including skill-disable import coverage |
| `cargo test each_copy_carries` | **1 passed** |
| `cargo test onboarding::tests` | **11 passed** |
| `cargo test export_bindings` | **1 passed**, no additional generated drift |
| `git diff --check` | **exit 2**, F8 |

Bindings SHA-1 before **and** after fresh export: `0fcd779816f51f71a1f33a8bd8359c4bd6fde6f5`. “Clean” here means unchanged from the submitted working-tree binding, not identical to HEAD. `description`, `modified_at`, `same_as_managed` are required nullable keys; `remove_divergent?: boolean` matches `#[serde(default)]`; `central_path` is required on both imported/replaced. Existing type shims already re-export these types; no new DTO type needs a shim addition. The report fold compiles/tests but has the semantic defects above.

`newest_modified` uses WalkDir, `filter_entry(!is_ignored)`, file-only metadata and `.follow_links(false)`. Internal symlinks are not traversed. Do **not** overstate this as “never follows any link”: root skill links are a supported input, WalkDir follows root links by default, and the fallback explicitly uses `std::fs::metadata(dir)` (follows a root link). Cost is an additional O(entries) metadata walk per discovered copy, on top of fingerprinting; it is not another content-byte hash. I did not benchmark a large real Tool directory or traverse the operator's library. A regression covering ignored subtrees/internal links/root links would make the claim precise.

`ImportModal.tsx` is absent; the replacement is in `migrated`, ceiling **812 → 606**. A source scan of the removed class names found no surviving product className use (generic hits such as `tone="danger"` and Badge's `data-slot="badge"` are not CSS references). All **11** deleted literal i18n keys had **zero** surviving `t("key")` reads; the i18n tests also pass.

Independent rendered contrast probe (Canvas-composited ancestor backgrounds, dark/light): Match **7.65/6.75**, Conflict **8.94/4.94**, modified time **5.52/6.24**, “Differs” and invalid reason **10.13/5.68**. These reproduce the child's reported values. No contrast blocker found. DESIGN.md accurately describes the rendered structure, default choices and cascade, but its focus-return and comprehensive-result promises fail the cases above. CONTEXT/CHANGELOG describe the intended deletion and one ask accurately for known comparisons; “within harness dirs” is the lexical fence, and “reported per harness” needs F6. The child's full `npm run check` / 844-test claim was **not rerun as a full gate by this constrained review**; the fresh checks above are the evidence I rely on.

## Summary

**FAIL.** The review is substantially implemented: clear comparisons, meaningful defaults and fates, one consolidated ask, a durable Result, failed-group retry/revisit, migration and reduced-motion cascade all have evidence. However, the new irreversible removal path converts an unknown identity into permission to delete; that blocks commit. Focus return, outcome counting, skipped-target presentation, toast occlusion and shared-harness removal reporting also need correction. Passing unit tests and stable bindings do not establish the missing destructive-edge proofs. Fix F1 first, add the specified Rust cases, then re-check the Result and keyboard regressions before accepting ticket 21.

## Re-check (2026-11-04)

**Bounded re-check verdict: PASS (merge). F1–F9 CLOSED.** This supersedes the original verdict for the fix-pass working tree, not the historical observations above.

Reviewed `/Users/alexstark/.t3/worktrees/skills-hub/t3-be1c965b`, branch `t3/import-review`, HEAD `8b42ce5`, including the staged deletion, unstaged diff and untracked implementation files. Read the original findings and the ticket's **Result → Fix pass (audit)** claims before testing. Source remained read-only; the only authored change is this appended section. No commits. The explicitly requested binding exports rewrote their generated artifact byte-identically. All commands below ran in this worktree; Cargo commands ran in `src-tauri/`.

Browser evidence is fresh, from my own `npx vite --mode fixture --port 5192 --strictPort`, ego-browser task space **15**, using only the in-memory fixture. No real skill library or native Tauri app was opened. The space was closed after the measurements. No screenshots are needed to establish the DOM/focus/hit-test results below.

### F1 — CLOSED: unknown identity no longer authorizes divergent deletion

**Fresh commands and output:**

```text
cargo test remove_divergent -- --nocapture
running 11 tests
...remove_divergent_retains_a_copy_whose_identity_is_unknown ... ok
...remove_divergent_retains_a_copy_when_the_managed_identity_is_unknown ... ok
...remove_divergent_retains_and_reports_a_copy_whose_metadata_is_unreadable ... ok
...remove_divergent_never_touches_the_chosen_path ... ok
...remove_divergent_removes_a_link_not_the_other_tools_dir_it_points_at ... ok
...remove_divergent_never_follows_a_link_into_the_central_repo ... ok
...remove_divergent_settles_a_shared_dir_once_and_reports_every_harness ... ok
[the other four matched tests also passed]
test result: ok. 11 passed; 0 failed; 0 ignored
REMOVE_DIVERGENT_EXIT=0

cargo test compare_says_unknown -- --nocapture
...compare_says_unknown_when_either_identity_cannot_be_read ... ok
test result: ok. 1 passed; 0 failed; 0 ignored
COMPARE_UNKNOWN_EXIT=0
```

No permission-test `skipped:` message appeared. I read the test bodies: the A/B/C test asserts B is gone, C's manifest **and sidecar bytes** remain with `IDENTITY_UNKNOWN`, A remains `LeftInPlace`, and all three originals are reported. The managed-hash and metadata-failure tests also assert retained bytes, not just a counter.

**Source confirmation:** `content_identity.rs:73–83` returns `Different` only for two `Some` identities that differ; a missing target or failed identity read returns `Unknown`. The managed identity is the stored/backfilled identity, as the existing content-identity contract requires—not a promise to rehash the managed directory on every comparison. `onboarding_import.rs:691–728` guards `path == chosen` before metadata/comparison, reports non-NotFound metadata errors and `Comparison::Unknown` as `OriginalStatus::Failed { IDENTITY_UNKNOWN }`, and calls removal only in `Comparison::Different`. The registry guard is still immediately before removal. `sync_engine.rs:420–439` uses leaf `symlink_metadata` and `remove_file` for a symlink, never recursive removal of its referent. The fresh link regressions verify the other tool's directory survives and the central-repo link is excluded from discovery.

**One additional attempted bypass, source-traced (not a newly injected Rust race test):** start with readable A and C including a sidecar, fingerprint the plan, then let C's sidecar vanish before the removal comparison while its `SKILL.md` remains readable. The plan fingerprint is **not** the deletion authority: `compare` re-walks C. If the walker has yielded the sidecar but `fs::read` now fails, `hash_dir` propagates the error, `read` returns `None`, and the copy is retained with `IDENTITY_UNKNOWN`. If the file vanished before enumeration, the current directory can be successfully hashed without it; removal then requires that newly read identity and the known managed identity to differ. If it vanishes only **after** a successful comparison, the delete already has two read identities; this is an ordinary post-check filesystem race, not the original unknown-as-different bypass. No path from a failed read to deletion remains in this construction.

I also traced a leaf link directly to the unchanged chosen copy: the supported root-link hash compares equal, so it is not a divergent-removal candidate; a broken link is `Unknown`. A symlinked **parent** remains a limitation of the existing lexical registry fence: this pass does not prove physical ancestor containment or atomicity against external filesystem mutation. The chosen guard protects the as-recorded path, not every possible alias. These limits are not presented as new safety guarantees or as executed race coverage.

### F2 — CLOSED: scan refusal preserves focusability; Result returns focus

Fresh `first-run` browser flow: open Review, choose Gemini CLI for commit-message, check remove-other, choose tdd Replace, import, answer all tdd collisions **Keep existing**, then wait for the Result's Close control to be enabled (the post-import catalog/plan reload has finished).

- During the scan, a `MutationObserver` on `[data-import-review]` captured:
  `{ disabled: false, ariaDisabled: "true", tabIndex: 0, focus: true, reason: "Scan is running — wait for it to finish" }`.
  After scanning it returned to `ariaDisabled: "false"`, with the reason removed. This is focusable-disabled, not native-disabled.
- Partial Result → trusted `page.keyboard.press("Escape")` → **two `requestAnimationFrame`s**:
  `{ focus: "BUTTON", text: "Review 4 discovered skills", dialogCount: 0 }`.
- Fresh full import (all ten new groups, both managed groups Replace, every collision Overwrite) → actual mouse `page.click('[data-slot="dialog-footer"] button')` → **two frames**:
  `{ focus: "H1", text: "My Skills", heading: "skills", banner: null, dialogCount: 0 }`.

An earlier Escape sent while the Result was already drawn but the reload was still running correctly did not dismiss it; I did not mistake that in-flight state for the settled focus-return check. The dialog's shared `fallbackFocus` and the two focusable invitation buttons implement the claimed fix; no surface-owned Escape handler was added.

### F3 — CLOSED: group headline reconciles imports, Replaces and Keeps

Fresh browser measurements on `first-run`:

| Action | Result heading | Reconciliation |
| --- | --- | --- |
| Seven defaults + commit-message (remove other copy) + tdd Replace; all three tdd collisions Keep | **8 imported · 1 kept** | Eight Imported rows; tdd **Kept**, with “Kept your existing folder in Claude Code, Codex, Pi”; Library 2 → 10 |
| Select all ten new groups + both managed-name Replaces; all six collisions Overwrite | **10 imported · 2 replaced** | Library has 12 skills; no Review banner after Close |

The browser used the real rendered Result and the fixture command backend, not a mocked presentation return. `importGroupKind` is now shared by the headline and result-row folds. All-Keep no longer counts as imported; successful Replace gets its own verb.

### F4 — CLOSED: all-skipped Replace is neutral and explains each skip

Fresh browser setup on `first-run`:

```js
const s = window.__fixture.state.skills.find(s => s.dto.name === "tdd");
s.dto.disabled_at = Date.now();
```

Opened Review, cleared all new-group selections via Select all on/off, selected only tdd Replace, and clicked **Replace 1 skill's copies**. The settled, rendered result was:

```text
1 skipped
 tdd  Skipped
 Skipped Claude Code: Claude Code is disabled for this skill. Enable it to deploy there again.
 Skipped Codex: Codex is disabled for this skill. Enable it to deploy there again.
 Skipped Pi: Pi is disabled for this skill. Enable it to deploy there again.
```

Measured icon: `lucide-circle-minus … text-muted-foreground`, computed color **rgb(163, 163, 163)**. Each explanatory paragraph was `text-muted-foreground` with the same neutral color. No green success icon, no “Replaced”, and no missing target reasons. This closes the fixer's explicitly unproven browser portion of F4.

### F5 — CLOSED: Import and Add failure toasts wait until mouse Close

Fresh `failures` scenario, default four imports. For each size I waited for enabled Close, counted `[data-sonner-toast]`, and hit-tested the **centre of Close** with `document.elementFromPoint`, then used actual pointer `page.click`, not DOM `.click()`.

| Surface / viewport | Settled Result | Toasts while open | Close rectangle `(x, y, w, h)` | Centre receives input | After mouse Close |
| --- | --- | --- | --- | --- | --- |
| Import, **960×640** | 4 imported · 4 deployments failed | **0** | `(758.14, 567, 60.86, 32)` | **true** | 0 dialogs; toast “4 imported · 4 deployments failed” |
| Import, **1280×800** | 4 imported · 4 deployments failed | **0** | `(918.14, 673.5, 60.86, 32)` | **true** | 0 dialogs; same failure toast |
| Add, **960×640** | Added 0 of 1; release-notes Failed, rate-limit reason and Retry | **0** | `(698.14, 448.5, 60.86, 32)` | **true** | 0 dialogs; toast “Could not add release-notes” |
| Add, **1280×800** | Same failed Add | **0** | `(858.14, 528.5, 60.86, 32)` | **true** | 0 dialogs; same failure toast |

Add reproduction: `https://github.com/acme/collide-kit` in the fixture → Continue → deselect all → select only **release-notes** → Add selected. Its scenario-supplied `failOnce: RATE_LIMITED` produced the real Add settlement path; no source patch or native/network install was used. Fresh navigation resets that fault between viewport checks.

The 960 measurement was explicitly rerun with `innerWidth/innerHeight == [960,640]` after observing that a CDP viewport override from an earlier ego invocation did not persist. The table uses the corrected bounded measurement, not the earlier mislabeled one. The 1280 Import override and flow ran in the same invocation; Add also measured its actual dimensions in each result.

The reporter's `resultShownIn` defers the duplicate toast, not reload/history/completion or the accessible toaster host. The full frontend suite includes both reporter deferral tests and the Import/Add call-site assertions.

### F6 — CLOSED: shared path settles once and both harnesses receive rows

```text
cargo test remove_divergent_settles_a_shared_dir -- --nocapture
...remove_divergent_settles_a_shared_dir_once_and_reports_every_harness ... ok
test result: ok. 1 passed; 0 failed; 0 ignored
SHARED_DIR_EXIT=0
```

Read `remove_divergent_copies` (`onboarding_import.rs:647–674`): an already-seen path reuses its stored `Option<OriginalStatus>`; removal is called only on the first occurrence, and each variant gets its own `(path, tool, status.clone())` row. The regression installs Amp and Kimi through `mark_installed_in`, asserts their shared directory is gone, asserts **both** `amp` and `kimi_cli` are `Removed`, and asserts three originals total (chosen Claude + both harness rows). This directly covers the original production defect.

**Retained coverage caveat, not a new finding:** the fixture plan still deduplicates shared paths with `seenPaths`, as the fixer disclosed. Browser fixture tests do not prove this fan-out. The real Rust regression does; closing F6 does not claim fixture parity or canonical/inode-based alias deduplication.

### F7 — CLOSED: intentional keep is a quiet note, not pending review

Fresh `first-run` browser: Settings → turn **Auto-deploy off** → Back → Review → check handoff with its Claude Code variant; leave **Remove the other copy of handoff** unchecked (`aria-checked=false`); keep identical-original removal checked. Submit **Import and remove 12 originals** (seven defaults + handoff).

Measured result at **960×640**:

- Headline **8 imported**, with no “to review”.
- Handoff: “Added to your Library only”; “Copies removed from Claude Code”; **“Your different copy kept in Pi”**. All three lines are neutral `text-muted-foreground`.
- Fixture still contains the original Pi handoff with its divergent fingerprint; the chosen Claude original is gone.
- After Close, Notifications shows a **Success** run headed **8 imported**, with the quiet note **“handoff: 1 different copy kept in Pi”**; `toReview: false` in both Result and history.

This proves the exact counted note as well as retention. The number is in the run note, while the inline Result uses the per-harness sentence.

### F8 — CLOSED: whitespace gate clean; two exports byte-identical

```text
git diff --check
[no output]
DIFF_CHECK_EXIT=0

shasum src/bindings/index.ts
96c8697308ed8ca8e0175a0b1eb4cc5dbebb58f9
cargo test export_bindings
1 passed; 0 failed; EXPORT_1_EXIT=0
shasum src/bindings/index.ts
96c8697308ed8ca8e0175a0b1eb4cc5dbebb58f9
cargo test export_bindings
1 passed; 0 failed; EXPORT_2_EXIT=0
shasum src/bindings/index.ts
96c8697308ed8ca8e0175a0b1eb4cc5dbebb58f9
```

The exporter normalizes line endings' trailing whitespace rather than hand-editing the generated DTOs. The full Rust run also passes `bindings_normalisation_trims_line_ends_and_keeps_the_final_newline`. A subsequent `git diff --check` still exited **0**.

### F9 — CLOSED: old source identifier gone

```text
rg list_all_skill_target_paths
[no hits]
RG_ALL_EXIT=1

rg -n 'list_all_skill_target_paths' src src-tauri
[no hits]
RG_SOURCE_EXIT=1
```

Ripgrep's **1 means no matches**, not a tool failure. These are ordinary repository/source searches; the immutable historical audit and ticket comments under hidden `.scratch/` still quote the old identifier as evidence. The implementation uses `list_owned_skill_target_paths` with the ownership filter unchanged.

### Full-suite fresh exits

These were executed, not copied from the fixer's claims. Exit markers were printed immediately after each command.

| Command | Fresh result | Exit |
| --- | --- | --- |
| `npx vitest run src` | **54 files, 1199 tests passed**; 18.85 s | **0** (`VITEST_FULL_EXIT=0`) |
| `cd src-tauri && cargo test --all` | **853 passed, 0 failed, 0 ignored**; main and doc-test targets also pass; 15.48 s for library tests | **0** (`CARGO_ALL_EXIT=0`) |
| `npx tsc --noEmit -p tsconfig.app.json` | No diagnostics | **0** (`TSC_EXIT=0`) |
| `npm run styles:check` | **606 counted App.css lines, 40 migrated files; self-test passed** | **0** (`STYLES_EXIT=0`) |
| `git diff --check` | No whitespace errors | **0** |

Vitest emitted the environment's experimental localStorage warning, but exited **0**, with no failed suite or unhandled-error summary. This bounded re-check is not a claim that lint, production build, clippy, version-check or a native release smoke were separately rerun.

**New findings:** none established in this bounded re-check. The known fixture shared-path parity gap and non-atomic/lexical filesystem boundary are stated above, not silently upgraded into stronger assurances. The requested regressions now demonstrate the goal: unknown comparisons retain bytes and report why; shared-path deletion reports every harness; Results count and explain what actually happened, remain mouse-closeable without their duplicate toast, and return focus usefully.

**Re-check verdict: PASS (merge).**
