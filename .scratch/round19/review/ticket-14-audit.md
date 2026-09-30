# Ticket 14 — independent D31 audit of `86bb753`

**2026-09-30 · FIX-THEN-PASS. Four P1 findings; no reproduced P0.** The nonmodal progress replacement works, but the new report surface has two persistent focus failures, an unusable Enable-collision repair, and an inaccessible Cancel path. Passing axe and the full build/test gate do not close these interaction defects.

## Method and evidence

Reviewed `86bb753e0a9ad942558fec0f506102b3028c5a9f` against its parent, starting at that HEAD with a clean tracked tree. During the audit another actor committed `bb64241` (BACKLOG #67 only); final `git diff 86bb753..HEAD -- src src-tauri scripts DESIGN.md AGENTS.md` is empty, so the audited application stayed unchanged. Read AGENTS.md (including all Ambiguity resolution), ticket 14's Work/Gate/Result/Comments, the specified decisions and tickets 12/13's audit/re-check obligations. The panel actually lives at **`src/components/shared/NotificationsModal.tsx`**, not the `skills/modals` path in the brief. Checked standards and spec in this terminal reviewer, without further delegation. Loaded web-design-guidelines, Impeccable audit, agent-browser/core, React/shadcn and review guidance; fetched fresh Web Interface Guidelines. Repository decisions override generic guidance (desktop minimum 960×640, dense targets 24 px, D5's limited hash state, custom Base UI primitives).

**E = `.scratch/round19/evidence/14-audit/`**, gitignored. Reused the already-running fixture server on 5175; never launched Tauri, operated on the real library/database, or invoked the native updater. One isolated headless `agent-browser` session, `r19-14-audit`, closed afterward; existing server left running. `failures&latency=3`, dark **1280×800** and light **960×640** for primary flows; rich/failures × both themes for the eight axe runs. Fixture update versions use `?update=`. No application source changes, revert, stash, or commit.

Copied and read `13-fix/keys-cdp.ts` + `cdp.ts`. All keyboard events use that trusted CDP driver, **never agent-browser press**. Read/adapted the implementer's flow drivers, executed them afresh, split long flows into bounded calls, then added independent `panel-extra.ts`, `enable-review.ts`, `switch.ts`, and assertions in `verify.ts`. Primary flows use keyboard navigation, not injected focus. Earlier exploratory `extra-*` runs used pointer/programmatic focus and had driver failures; they are retained but **not the proof of A1**. A1 is reproduced anew by Shift+Tab to the toast action and trusted Enter. Clipboard evidence observes `writeText` arguments while forwarding the real write; OS clipboard readback was denied, so no native clipboard-read claim. The existing Add-foot probe's input value/focus setup is supplemental, not keyboard-only proof.

`enable-review.ts` explicitly supplements the fixture with **one foreign directory** at a disabled retired-playbook/Claude Code pair; all following interaction is trusted keyboard. Rust and fixture both independently implement the disabled-marker behavior it tests. No filesystem bytes are changed.

`verified-summary.txt`: **1,635 primary transcript samples, nine BODY observations** — eight are the persistent A1/A2 samples; one is the short resize/reparent gap N2. Do not quote this audit as “never BODY.” Samples are observations after CDP/CLI round trips, not exhaustive frame-by-frame proof. Screenshots of header, panel, ask and bulk actions were captured; inspected dark panel and light minimum-width header images. `commit.diff`, `core.diff`, `docs.diff`, `architecture.txt`, `motion-css.json`, and the fresh gate logs retain code/command evidence.

## Per-clause verdicts

| Clause | Verdict | Evidence / qualification |
|---|---|---|
| Refresh header: own copy, counts, Cancel, inline row pending | **Pass** | `refresh-{dark-1280,light-960}.json`: Refreshing, 1/19, 22 busy rows; no dialog, root not inert. |
| j/k/Enter browse/open inspector during Refresh | **Pass** | Same transcripts: alias-skill opens alongside at 1280 and in the sheet at 960; sheet Escape returns to the row. |
| Second mutation refused, not queued, accessible reason | **Pass for exercised header/rail/inspector controls** | Add skill stays focusable, `aria-disabled=true`; matching title + aria-description “Refresh is running — wait for it to finish”; Enter opens nothing. Reporter ref guards same-tick double admission; hook tests pass. Switch's description/focusability polish remains 15. |
| Early Cancel / no Cancel during apply | **Pass** | Cancellation retains pending until settled, “Refresh cancelled — nothing was changed,” focus Refresh all. Apply tick 1/19 has no Cancel; permanent withdrawal survives uncancellable ask phase in reporter test. See A4 for panel-originated work. |
| Bulk Disable: rail stays, no overlay/inert, count outcome | **Pass** | `flows-*-bulkDisable,bulkRemove,dialogClose.json`: three rows busy; rail disabled with reason; Disabling header; “5 disabled” outcome; same rail trigger focused. The header is ongoing activity, not an invented final count. |
| Bulk Remove: inline rows, Delete alone owns progress | **Pass** | Two rows busy, empty header, Delete pending Remove button; “2 removed”; next surviving row focused. |
| One grouped report per folded action, action name/time, expand | **Pass for tested report actions** | Refresh → one run/27 items; Retry → a second run; newest-first, retained expansion, metadata and disclosure. Standalone `notify` entries still exist by design; this is not a claim every non-report verb gained a run. |
| Show / Change source / Settings → GitHub | **Pass** | `report-*`: Show closes panel and focuses inspector H2 (side/sheet); Change source opens its dialog; close uses row/heading fallback because panel was replaced; GitHub focuses `#settings-github-token`. |
| Retry / Overwrite… | **Partial — A3/A4** | Normal sync collision reopens one ask; dismiss returns to Overwrite… and records a new run without a toast. Enable-originated collision cannot be repaired this way. Source Retry runs but its Cancel is inert. |
| Project identity / per-row Copy / Copy all / Clear | **Partial — A2/N1** | Expanded Refresh project item says “in monorepo: …”; copy controls have row-specific names. Real write arguments carry the row / full grouped history. Clear empties history but strands focus. Project-headed runs/export have N1. |
| Toast policy; View report targets its run | **Partial — A1** | No toast while panel open; Retry outcome remains a new history run, not lost or deferred. Target run is expanded on View report, but focus does not enter it. Clean success with no detail deliberately omits View report (`useStatusReporter.ts:550`); consider that a minor interpretation of “headline + View report,” not lost history. |
| Dismissed ask: warning “folders to review,” no failure/implicit Keep/retry | **Pass for exercised sync/Refresh paths** | `flows-*-dialogClose,restore,resize,dismissed.json`: warning “4 synced · 1 skipped · 1 folder to review”; foreign pair state byte-for-byte identical before/after. Door test asserts zero retry commands and no error history. Import qualification below. |
| Re-assert additions in Refresh report | **Pass** | `panel-extra-*` and `report-panel-*`: run detail names additional Cursor/Codex/Pi/Gemini CLI deployments, including zod-schema-first. |
| Passive update/no startup modal | **Pass** | `surfaces.json`, `update-partial.json`: 1.3.1 item, no startup dialog; opens release notes; Escape returns to item; install → installed/Done; ignore removes item. `check`, install failure/retry, ignored startup and shared Settings state are pinned by passing `useAppUpdate.test.ts`. Fixture Settings' idle Check is native-gated; failed exploratory attempt to reach it is not claimed as browser coverage. |
| D21 dialog/menu/Restore/Delete/late navigation/resize | **Partial — A1/A2/N2** | Existing paths return to menu button, Health, next row, or Settings; child stays top across 1200. Supplementary 0/300/2000 samples in `flows-*-dialogClose,restore,resize,dismissed.json`; Delete in `flows-*-restore,delete.json`. N2 is one transient reparent observation. |
| Reduced motion / axe / contrast | **Pass for measured scope** | Eight axe runs zero violations, text pairs ≥5.32 panel / ≥8.16 header; reduced spinner hidden, fill 0s, no running animations. Details below. |
| Architecture, styling, bindings, EN literal keys | **Pass with qualifications below** | No overlay coupling remains; pure grouping fold/test; one reporter history/toast policy; generated bindings unchanged; full gate passes. History repair data loses Enable route (A3). |
| Definition-of-done gate | **Pass on fresh rerun** | Initial run timed out in one test; full unchanged-tree rerun exits 0. Both logs preserved. |

## Findings

### A1 — P1 — “View report” opens the panel but leaves focus on BODY

**Location:** `src/hooks/useStatusReporter.ts:464–473,550–556`; `src/components/shared/NotificationsModal.tsx:148`. Interaction with Sonner's focus restoration (`node_modules/sonner/dist/index.mjs:1064–1079,1109–1120`).

**Reproduction:** failures + latency 3 → Refresh all → Escape from overwrite ask → Shift+Tab through header/sidebar to **View report** → Enter. In **both themes/sizes**, the requested run expands and the panel opens, but at **300 and 2000 ms `activeElement=BODY`**, root inert, no visible focus. At the first sample focus is briefly My Skills outside the modal. `panel-extra-{dark-1280,light-960}.json`, `view-report-*.png`. Tab subsequently recovers into the panel; this is not an irreversible trap/P0.

**Impact:** D11's report entry point breaks D21 and loses keyboard position exactly when opening the report. Sidebar-open evidence does not cover this path.

**Cause/fix:** Reporter dismisses the focused toast as it opens Dialog. Sonner explicitly returns focus to the element before its toast on blur/cleanup; the observed return is My Skills, then inert application drops it to BODY. Coordinate this handoff in reporter/Dialog/toaster integration so toast focus restoration completes **before** the report receives initial focus, or disable that competing return for this navigation. Do not restore the removed 12-frame hold or scatter `.focus()` retries at call sites. Pin focused-toast → run-focused-dialog, including toast auto-removal and pointer activation, with appropriate seam/primitive tests plus the browser transcript.

### A2 — P1 — Clear disables the focused button and leaves the open panel on BODY

**Location:** `src/components/shared/NotificationsModal.tsx:287–290`.

**Reproduction:** open nonempty Notifications → keyboard to **Clear** → Enter. History empties; `disabled={isEmpty}` disables the native focused button. Both `panel-extra-*` runs show Clear at 0 ms, then **BODY at 300/2000 ms** while Notifications remains open and root inert.

**Impact:** Basic panel maintenance loses focus rather than retaining a meaningful position, violating D21. No running action settles afterward to rescue it.

**Fix:** retain focus via the Button primitive's existing `focusableWhenDisabled` contract (as the rail already does), or deliberately hand it to the empty-state heading/Close through a documented primitive rule. Test clearing the last run and subsequent Tab/Escape. Do not rely on reporter post-mutation focus for this non-mutation.

### A3 — P1 — Overwrite… cannot resolve a dismissed Enable collision

**Location:** `src/lib/reportOutcome.ts:286–297,959–1003`; `src/lib/notificationRuns.ts:99–103,131–140`; `src/components/shared/NotificationsModal.tsx:235–239`; contrast `src-tauri/src/core/skill_disable.rs:450–480`.

**Reproduction:** the supplemental `enable-review.ts` setup places different unmanaged content at retired-playbook's disabled Claude Code target. Keyboard Enable → initial **Folder already exists** ask → Escape. Backend/fixture correctly retains that pair **disabled, owning nothing**. Notifications now offers Overwrite… for it. Pressing that button runs `run("retry", …, { tool })` → ordinary sync. There is **no overwrite ask**; the new run says **“Not synced: retired-playbook -> Claude Code … Enable it to sync there again.”** The old Overwrite… button remains. `enable-review.json`, `enable-review.png`; foreign directory unchanged and target still disabled.

**Impact:** The advertised recovery action cannot recover this valid report. Repeating it only adds reports. This is not a theoretical stale-history case: it fails immediately after the original ask was dismissed.

**Fix:** preserve the repair's route/intent as data, or have the skill door resolve the appropriate current-state repair. An Enable collision needs the **Enable** path with its disabled-marker admission and one per-pair overwrite ask, not ordinary sync. Keep the current catalog check, exact pair identity and D19 lifecycle; do not bypass Disable by passing blind overwrite or replay a whole historic batch. Add a real-door test covering Enable → dismissed ask → history repair → ask → Keep/Overwrite/dismiss, including later operator disable/state changes.

### A4 — P1 — Cancellable Retry from Notifications has no reachable Cancel

**Location:** `src/components/shared/NotificationsModal.tsx:240–242,285–296`; `src/App.tsx:545–550,790–806`; `src/components/shell/HeaderProgress.tsx:98–112`.

**Reproduction:** Refresh report → **Retry: pr-review** (a source Update, not target sync). At 100 ms in both `panel-extra-*` runs, Retry retains focus and the new Update runs, but the **only `Cancel Update` is in the header beneath the modal with an inert ancestor**. Notifications has no progress/Cancel footer. Retry eventually records another run and correctly suppresses its toast.

**Impact:** This new panel action starts cancellable work without exposing its cancellation control in the active layer. An operator must infer that closing the report is necessary to reach Cancel; it is unusable directly from the action surface. The implementation explicitly solved this for Add/Re-point but omitted Notifications.

**Fix:** give the active report action a reachable progress/Cancel placement using the **same reporter pending**, or intentionally close the panel on dispatch with a clear focus/progress handoff. Do not create a second pending state or offer Cancel for non-token target Retry. Verify a slow panel source Retry can be cancelled by keyboard; verify target Retry/overwrite retry still offers none. Also cover opening the panel/sheet during an already-cancellable Refresh.

### N1 — P2 — Project identity is absent when an item becomes a run heading, and from copies

**Location:** `src/components/shared/NotificationsModal.tsx:311–371,385–399`; `src/lib/notificationRuns.ts:146–168`.

A failed project toggle has no headline; `groupRuns` promotes its first entry to `head`. Only **ItemRow**, not RunRowView, prefixes `in {project}:`. `rowText`/`historyText` copy only title/message, not the subject's project. The pure probe `project-head-copy.txt` shows a p1 project failure promoted to a head and exported with no project identity. Expanded Refresh project items do label correctly. This is code/pure-fold proof, not a claimed matrix-browser reproduction.

**Fix:** use one project-aware text presentation for both heading/items and export; include a single-error project-toggle regression. Avoid duplicating the project in resync-all titles that already name it.

### N2 — P2 — A brief BODY gap still exists during deferred inspector reparenting

**Location:** `src/App.tsx:267–279,329–388` (deferred placement and arrival focus).

In `flows-dark-1280-dialogClose,restore,resize,dismissed.json`, closing Change source after 1280→960→1280→960 produces one immediate `key Escape` sample with **no dialog/inspector and BODY**. The next observation (the explicit 0-ms sample after a further round trip) already has the new sheet H2; 300/2000 ms remain there. Light and the earlier sampled runs do not reproduce a persistent failure. No stuck inert or late focus theft observed.

**Fix/qualification:** preserve focus across the unmount/mount placement boundary, or explicitly accept this sub-frame transition as distinct from a settled-focus guarantee. Do not assert literal “never BODY” from 300-ms-only sampling. Recheck this narrow transition when shared focus code changes for A1/A2.

### N3 — P2 — The documented handoff/producer claims overstate completion

**Location:** `src/lib/reportOutcome.ts:271–278`; ticket 14 Result/Comments; `.scratch/round19/issues/22-explore-rows.md` (no Comments section).

“Every producer asks first” is false for current Import (below). Ticket 15 has its Switch comment and ticket 17 has its dialog-foot comment, but **22 has no Explore-focus handoff Comment**; only ticket 14's Result mentions it. Correct the collision comment to mean an **unresolved** collision, not proof that an ask happened. Orchestrator should add the missing 22 handoff (outside this reviewer's permitted writes) and retain Import's existing ticket-21 obligation.

## Verdicts on the six implementer decisions

1. **Accept summary replacement.** D19 requires a browsable library, not a persistent stale summary. The Issues chip, search, selection and inspector remain available; measured 960 header stays one row. The run takes appropriate priority. No separate summary blocker.
2. **Accept the fold classification, reject the universal premise.** An unresolved TARGET_EXISTS is “to review,” never a failure whether or not an ask was shown. Current producer tracing:

   | Producer | Actual route before fold |
   |---|---|
   | Global batch / target Retry | `syncSkillsToTools` → `resolveGlobalCollisions`, or collectSync → `settleCollisionParts` → one ask. |
   | Install batch | `useAddSkillFlow.ts:179–205`: installTargets → globalCollisionRows → `action.uncancellable(resolveGlobalCollisions)` → merge → fold. If retry throws, original unresolved rows remain plus separately reported command error; “dismissed” is not necessarily why they remain. |
   | Enable | `enableSkillTargets` → Enable CollisionPart → one ask, route-aware retry while still in that original action. History loses this route: A3. |
   | Project assign/toggle/resync/all | `useProjectState.ts:443,466,571,590` → injected `settleProjectCollisions`; App supplies it. Whole-action report is settled before fold. |
   | Refresh / Update / Restore / Re-point | door `settleRefresh` → refresh CollisionPart includes **both Propagation and re-assert**. No current unattended frontend Refresh bypass was found; a hypothetical scheduled/headless backend run does not itself execute a frontend fold. |
   | Edit Propagation | door `editInvocation` → propagation CollisionPart → fold. |
   | **Import** | **No ask today:** `useAddSkillFlow.ts:453–474` sends import then directly applies `importOutcome`. This is already explicitly deferred to ticket **21** (its Work/Comments), not implemented D12 coverage. |

   `completion.conflict` is currently the **Edit-conflict** bit (`reportOutcome.ts:446`), preserved independently of headline tone and returned by applyOutcome; it is not an overwrite-dispatch mechanism. D12 asks run **before** the fold, not from that bit. Collision folds returning `conflict:false` therefore do not reopen a dismissed ask, but they also do **not** rescue Import's missing ask. Keep ticket 21's gate; do not claim all D12 producers passed here.
3. **Accept a fresh ask, reject the uniform retry route.** Clicking Overwrite… is a new operator action; one fresh ask is appropriate and safer than treating a historical row as current overwrite authorization. Normal sync case passes. Enable case is **fix-then-pass, A3**. Carry repair intent, not an already-given overwrite answer (there was none).
4. **Accept Refresh's permanent Cancel withdrawal.** Rust `refresh.rs:253–300` acquires/cancels before apply; no cancellation check during apply. Applying ticks set `cancelWithdrawnRef`; `uncancellable` cannot restore it. The Add listing→install boundary explicitly checks `cancelRequested()` before the resetting token command. This does not excuse A4's inaccessible Cancel during a legitimately cancellable panel Update.
5. **The anticipated BODY loss was not reproduced on the row switch.** In both `switch-*.json`, the rendered control is **SPAN role=switch**, not a natively disabled button. While one row is busy it has `aria-disabled=true`, `tabindex=-1`, retains focus/ring at **0/300/2000 ms**, then re-enters the tab order. Ticket 15's Comment exists but its “drops focus” description is not what this Chrome run does. Keep the planned primitive review (native/VoiceOver still separate); no Switch blocker from this audit.
6. **17 yes; 22 no.** Projects foot progress is explicitly handed to 17. Explore's MAIN landing is recorded in 14 but not as a Comment on 22 (N3). The orchestrator must add that handoff or explicitly retain it here. No application code was changed to address either deferred surface.

## Axe, contrast, motion and design integrity

Eight **fresh axe 4.10.3** checks (`a11y-{rich,failures}-{dark-1280,light-960}.json`, each running + panel): **zero violations in all eight**. Panel runs each retain two `aria-hidden-focus` incompletes (Base UI guards). Running-page contrast incompletes: rich dark 2/light 14; failures dark 3/light 10. These are reported, not silently counted as passes. Manual computed-pair measurement for enabled text in the actual audited scopes found **zero <4.5:1**:

| Surface | Dark min | Light min |
|---|---:|---:|
| Header progress | 8.31 | 8.16 |
| Report panel | 6.61 | 5.32 |

No new width overflow in inspected 960/1280 captures. Report rows wrap and controls remain discernible. Disabled text was excluded from contrast conformance measurements. These checks do not test native VoiceOver or certify every product surface.

Fresh full gate built `dist/assets/*.css`; `motion-css.mjs` scanned the emitted CSS, not source utilities alone. Global reduce zeroes animations/transitions; generated `motion-safe:animate-spin`, `motion-reduce:hidden`, and `motion-reduce:transition-none` are present. Header's only new motion is spinner/fill; panel introduces no animation/transition. Four reduced-motion probes show header spinner **display:none**, fill **0s**, row spinner **none 0s**, **zero running animations**. Text/counts remain. Remaining shimmer keyframes serve the pre-existing skeleton, not an overlay residue.

Impeccable detector for the new panel/header and changed Dialog: **`[]`** (`detector.json`). Product-specific token/primitive composition is coherent; no new styling system or motion machinery. Indicative scoped scores: accessibility **2/4**, performance **4/4**, responsive **3/4**, theming **4/4**, integrity **2/4** = **15/20**. The score is not acceptance; A1–A4 are hard interaction defects.

## Architecture and standards checks

- **HeaderProgress:** memoized view of reporter pending + its existing step message, no local progress state, no Dialog/layer/inert calls. App places it inline/bar/dialog-foot. A4 is a missing active-layer placement, not a second lifecycle.
- **Overlay cleanup:** source grep finds no `LoadingOverlay`, `sheetOpenFor`, `whenSettled`, `showsProgressSurface`, or 12-frame hold. Door places once with one-frame re-enable grace. App effect cleanup cancels its frame; late Settings completion stays on Settings. Layer-stack `handOverReturn`/`returnChains` remain attached to live layer records; only terminology changed there.
- **History:** exactly one production `recordRun(` call in reporter applyOutcome. `useNotificationHistory` owns the bounded session ring; `record` still intentionally handles standalone notifications. `notificationRuns` is pure/type-only dependencies, nine tests, no DOM/IPC. It does not retain skill object snapshots or callback repairs. Route erasure is A3.
- **Actions:** no `invokeTauri(` in the panel. Mutations dispatch through actions.run; Show/Settings through surface.focus. No new per-verb callback prop API. Local expansion/copy feedback is presentation state, not another world.
- **Toasts:** reporter owns operational calls, suppression, history recording and run opening. UI specimen has an intentional non-product toast demonstration. Opening panel dismisses existing toasts; events produced while it is open are recorded immediately. Copy success is deliberately only local/courtesy feedback. A1 exposes the library focus-integration gap, not lost outcome data.
- **Dialog:** menu-item opener resolves to menu trigger via aria-labelledby; primitive test pins it, DESIGN.md documents it. Real menu→Re-point→Escape returns to More actions. Existing layer handover preserved.
- **Update:** no offerDialog in useAppUpdate; explicit `open()` uses the surface door. `offerDialog` correctly remains for **new Tools**, not update startup.
- **Styling:** no notif-* rules in App.css; ceiling **1882**, **26 migrated**; no new stylesheet/token/raw-color regressions; no binding diff against parent or current tree. EN literal-key test and TS build pass.
- **Shared-seam caution:** do not reintroduce overlay waits to fix A1; coordinate toast focus once. Do not make ordinary Retry ignore disabled target admission to fix A3.

## Gate output

Ran **`npm run version:check && npm run check` myself**, without changing source. First attempt (`gate.txt`) reported **886 passed / one 5-second-timeout failure** in the Restore consolidated-ask test (`useSkillActions.test.ts:910`); reported wall duration was ~1,050 s. Cause was not established. Repeated the **entire unchanged-tree gate**, not just the failed test:

```text
Version OK (1.2.18)
ESLint: passed
styles:check ok — App.css 1882 counted lines (ceiling), 26 migrated file(s)
Test Files  41 passed (41)
Tests       887 passed (887)
tsc -b && vite build: passed (2.65 s)
cargo fmt --check: passed
cargo clippy --all-targets --all-features -- -D warnings: passed
cargo test: 807 passed; 0 failed
GATE_EXIT=0
```

`gate-rerun.txt`, `final-checks.txt`; bindings unchanged. Usual Node localStorage experimental and Vite chunk-size warnings remain nonblocking. `verify.ts` independently asserts the key successes **and the reproduced failures**, all eight axe checks, reduced motion and gate exit; `verified-summary.txt` ends **all audit evidence assertions passed**. That means the evidence is consistent, not that D31 passed.

## Acceptance

**FIX-THEN-PASS; ticket 14's D31 gate is not granted on `86bb753`.** Fix **A1–A4**. N1–N3 are bounded notes, not additional P0/P1 blockers. The orchestrator owns any explicit deferral and the missing 22 Comment; Import's existing ticket-21 deferral must remain visible. Operator native/VoiceOver release smoke remains outstanding.

Exact bounded re-check:

1. Both themes/sizes: keyboard and pointer **toast View report → requested run focused**, close → usable fallback, at 0/300/2000 ms; no BODY. Repeat while another report/layer exists and after toast removal. Preserve one toast and suppression/history policy.
2. Both sizes: **Clear last run**, empty-state focus, Tab/Escape; no BODY at 0/300/2000 ms.
3. **Enable collision → dismiss → history Overwrite… → one ask**; Keep/Overwrite/dismiss settle correct pair, preserve unrelated targets and foreign data on dismissal. Include a changed-current-state/stale-history case. Retain ordinary global sync, project and install ask tests.
4. **Panel source Retry**: progress + usable Cancel in active layer; early cancel settles honestly and returns focus. Target Retry and overwrite retries offer no Cancel. Opening panel/sheet mid-Refresh must not orphan its cancellation affordance.
5. Re-run affected hook/primitive/pure-fold tests and **version + full check**; targeted axe/contrast/reduced-motion on any changed report/progress placement. Recheck menu→dialog and child-close/resize return seam (N2) if shared focus code changes. No reason to repeat unrelated backend/browser matrices if those seams stay untouched.
