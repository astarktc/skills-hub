# Round 19 closure — extract D

Slice **D** · walked: `issues/15-emil-pass.md`, `issues/16-add-one-surface.md`, `issues/17-projects-surface.md`,
`issues/18-operator-think-aloud.md` (every § Findings row, debrief and outcome line), `review/ticket-15-audit.md`,
`review/ticket-16-audit.md`, `review/ticket-17-audit.md`. Evidence re-derived on `main` @ `19e896c` (read-only; paths
below are repo-relative; `.scratch/round19/` shortened to `r19/`).

## 1. Carry — real, still open, not in BACKLOG

| id | title | source | current evidence | proposed home | why it matters |
|---|---|---|---|---|---|
| D1 | Triggerless anchored Popover: hidden focus-guard Tab stop, stays open on Tab-out (R1) | `r19/review/ticket-16-audit.md:205` (R1); `r19/issues/16-add-one-surface.md` Comments 2026-10-01 "R1 … carried to ticket 17"; `r19/issues/17-projects-surface.md` Comments 2026-10-01 (R1 deferred); `r19/issues/23-command-palette.md:129` ("not needed, not fixed … stays where ticket 17 left it") | `src/components/skills/SkillRow.tsx:299-305` still opens the inline Enable confirmation as `<PopoverContent anchor={switchRef} …>`; `src/components/ui/popover.tsx:99-131` passes `anchor` to the Positioner only — no focus-manager return position, so the defect the audit reproduced (also on the pre-fix primitive) is unchanged | later (BACKLOG, primitive fix + browser regression) | Every My Skills row's Enable ask leaves an unnamed `aria-hidden` Tab stop and a stale layer record after Tab-out; three tickets passed it on and none owns it now |
| D2 | Bulk rail at the window foot was found only "after looking around" | `r19/issues/18-operator-think-aloud.md:402-415` (P2·Q8 follow-up: "bulk rail placement, 1.3.x → BACKLOG (low)") — not in § Outcome's routing table | `src/App.tsx:686-695` still renders `BulkRail` `order-last` (foot of the list column); `DESIGN.md:659` documents it there | later — add as a bullet to #72 (My Skills reading fixes) | The only finding of the think-aloud the routing table dropped silently; low, but the operator reached for row switches first because the rail was out of view |
| D3 | Popover tab-return relies on Base UI's internal `data-tabindex` protocol; no browser regression pins it | `r19/review/ticket-16-audit.md:226` (concern (b): "Pin the browser regression when upgrading Base UI; this is an internal attribute dependency") | `src/components/ui/popover.tsx:80-93` (`restoreTabOrder` reads/rewrites `[data-tabindex]`); only a jsdom test simulates the attribute (`popover.test.tsx:119`, :213); `package.json:37` `"@base-ui/react": "^1.8.0"` — a caret range, so a minor bump can change the protocol unnoticed | later (BACKLOG; pairs with #69's tracked browser driver) | A silent Base UI minor upgrade can re-break ticket 16's A1 (focus return inside a popover under the shared-dir ask) with every test green |

## 2. Covered — already a BACKLOG item

| id | title | source | # | amend? |
|---|---|---|---|---|
| D4 | Overwrite ask: who put the folder there, content/diff, Reveal, per-button consequence, answered rows recorded, ask-after-Cancel wording | `18…:279-296` (P2·Q4), `:297-301` (forgot what he overwrote), `:256-278` (P2·Q3 "my current copy" framing), `:596-614` (P5·Q3 Keep), `:626` (P5·Q4 minor), `:694` (debrief 2) | #70 | **yes**: drop "re-assert" from "A re-assert, Add or Enable collision has no target row" — ticket 33 deleted the re-assert (`src-tauri/src/core/refresh.rs:22-24`); the ask copy is unchanged (`src/i18n/resources.ts:439-447`, `:575-583`) |
| D5 | Summary pairs two units, target count first (read "7" as skills) | `18…:170-186` (P1·Q1), `:695-699` (debrief 3) | #72 | **yes**: the words are now "N failed deployments · View N skills with issues" (ticket 32), still target count first — `src/components/skills/LibraryHeader.tsx:82-91`, `resources.ts:601-606` |
| D6 | Issue rows below the fold (list grouped by source, no severity order) | `18…:181-186` (P1·Q1) | #72 | no — still true: `SortMode = "name" \| "updated" \| "added"` (`src/lib/skillPresentation.ts:56`) |
| D7 | Inspector offers Sync on a "(not detected)" Tool, which fails `TOOL_NOT_INSTALLED` | `18…:367-369` (P2·Q6 Finding A) | #72 | **yes**: cite `src/components/skills/inspector/TargetsSection.tsx:46-47` (`not_synced` → `["sync"]` regardless of `mark.detected`, read at :173); the verb is now "Deploy" in the vocabulary |
| D8 | "1 Edit to review" item sits last in a collapsed run | `18…:334-345` (P2·Q5) | #72 | no — runs keep report order (`src/lib/notificationRuns.ts:29`, :52-65) |
| D9 | Bulk Enable sentence unparseable; Disable zero case misread; "removed and kept disabled" | `18…:417-441` (P2·Q8 recall), `:691-693` (debrief 1, phrasing half) | #73 | **yes**: ticket 32 rewrote the words but not the shape — now `resources.ts:903` "Enable {{skills}} — {{targets}} on {{tools}} are deployed again, a first deploy each." and `:894` "…; no project deployment changes. Updates will not re-add them."; update the `~:994/~:985` line refs |
| D10 | Notifications empty after relaunch (reopens #21) | `18…:381-387` (P2·Q7) | #74 | no — still session-only (`src/hooks/useNotificationHistory.ts` has no persistence) |
| D11 | Run toast vanished before View report (failure tone should be `Infinity`) | `18…:319-325` (P2·Q4 toast) | #74 | no — still unreproduced; `useStatusReporter.ts:49-54` unchanged |
| D12 | Matrix cell toggle-off has no Undo; check whether the larger hit area invites misclicks | `18…:643-649` (P6·Q1 accidental unassign) | #75 | no — no undo in `src/components/projects/*.tsx` |
| D13 | Removal report: "Removed from" lacks project scope; permission failure has no next step | `18…:458-463` (P2·Q10 A, B), `:720-726` (A verified) | #76 | no — `resources.ts:978` `removedFrom: "Removed from {{tools}}."` |
| D14 | Re-point should rename to the upstream name by default | `18…:550-566` (P4·Q2) | #77 | no |
| D15 | Operator assumes a scheduled morning Refresh | `18…:676-683` (P7·Q1) | #53 | no — evidence already quoted in #53 (its "watch #71" clause is moot since ticket 33; optional trim) |
| D16 | Projects matrix keyboard model (127 Tab presses) | `r19/review/ticket-17-audit.md:53` (N1); `17…:171` (open concern) | #85 | no |
| D17 | No matrix multi-select / selection producer | `17…` § Decisions "No matrix multi-select / keymap this round" | #86 | no |

## 3. Closed — fixed since

| id | title | source | closed by | code evidence |
|---|---|---|---|---|
| D18 | N4 equal-text arrivals silent in the open panel (15 audit A1, P1) | `review/ticket-15-audit.md:41` | 15 (be367c8 + ab98c40) | `src/lib/notificationRuns.ts:222` `arrivalAnnouncement` (keyed), used `NotificationsModal.tsx:290` |
| D19 | Button test spy unbound (15 A2) | `ticket-15-audit.md:53` | 15 | `src/components/ui/button.test.tsx:28`, `:45` bind `onClick={onClick}` |
| D20 | Reduced-motion "first frame" overclaim (15 A3) | `ticket-15-audit.md:59` | 15 | `DESIGN.md:698` "settled on its first *visible* frame" |
| D21 | Generated `.transition*` utilities from prose scans; dormant `Layout.tsx` (15 A4) | `ticket-15-audit.md:65`; `15…` Comments 2026-09-30 "Open:" | 25 (20d3fb4) | `src/index.css:12` `@import "tailwindcss" source(".")`; `src/components/Layout.tsx` and `src/App.css` no longer exist |
| D22 | Legacy loops `.spinner` / `.cell-spinner` / `.skeleton-row` shimmer | `ticket-15-audit.md:97-99`; `17…` Comments (from 15) | 17 + 25 | no `.css` rule for any of them in `src/` (App.css deleted) |
| D23 | Permitted rare moments not built (install landing, large import) | `15…` § motion table "not built yet (tickets 21, 16)" | 16 (landing) + 21 (first-run cascade) | `r19/issues/16…` § Decisions "Install landing moves"; `r19/issues/21-import-review.md:100` § First-run cascade |
| D24 | 14→15 handoffs: Switch `focusableWhenDisabled`, N2 reparent BODY frames, N5 duplicate progress | `15…:116-124` Comments | 15 | `focusableWhenDisabled` on the row switch `SkillRow.tsx:293`; `HeaderProgress yielded` / `progressInDialog` (`App.tsx:675`) |
| D25 | Add audit A1–A4 (shared ask eats popover; Retry auto-closes; tab race; repeated prose) and R2 (stale tabindex) | `review/ticket-16-audit.md:47,59,71,81,213` | 16 (a20a58b + db27534, 3cdb33e) | R2: `popover.tsx:86-89` skips natively/aria-disabled controls without `data-focusable-when-disabled` |
| D26 | Missing project root recreated by assign/toggle/Enable/collision retry, and by the check-then-mkdir race (17 A1, P1) | `review/ticket-17-audit.md:39`, `:142`; `17…:173` ("refused only in the UI") | 17 (f648961 + 2450e61, e66da2c) | `src-tauri/src/core/project_sync.rs:379-387` (`folder_missing` skip), `:481`, `:728`, `:824`, `:947` (`project_folder_exists`); `sync_engine.rs:336` `create_dirs_within`; picker refusal `resources.ts:1297` |
| D27 | Filtered repair drops focus to MAIN (17 N2) | `ticket-17-audit.md:61` | 17 | `src/components/projects/useMatrixFocusReconciliation.ts`, used by `AssignmentMatrix.tsx`; re-check closed N2 |
| D28 | Open-menu axe `region` | `17…:169`; `ticket-17-audit.md` § Rulings 1 | 23 (359bb3b) | `DESIGN.md:560-563` records the narrow accepted exception |
| D29 | Refused row switch looks like nothing happened (mirror-tools stayed on — the one hold) | `18…:388-416`, `:706-719` | 27 (1c7d031) | `src/lib/actionLifecycle.ts:157` refusal notice; `resources.ts:1299-1301` "Nothing was changed." |
| D30 | Executable bits dropped on the Contents fast path (UNI-163, second hold) | `18…` § Outcome table | 28 (cd33e1e) | `src-tauri/src/core/git_acquisition.rs:16`, `:184` (adapter restores mode bits) |
| D31 | Refresh re-assert: "not synced" not durable; spread a Delete-incomplete skill; first deploys worded "Could not update the synced copy"; slashed ring not read as deliberate (#71) | `18…:214-227` (P1·Q3), `:302-318` (P2·Q4), `:370-380` (P2·Q6 B), `:464-468` (P2·Q10 C), `:700-705` (debrief 4), `:187-213` (P1·Q2) | 33 (23938db over 948f12b; D34/D35) | `src-tauri/src/core/refresh.rs:22-24` (arrival-only, re-assert gone), `:410-460` (`delete_incomplete` skipped as `RefreshSkipReason::DeleteIncomplete`); the "Could not update the synced copy" string no longer exists in `resources.ts` |
| D32 | Strip marks positional and unlabelled | `18…:346-359` (P2·Q6) | 31 (a2eed57; D42) | `src/lib/skillPresentation.ts:1217` `harnessInitials`, rendered `SkillsList.tsx:134`; `DESIGN.md:829` |
| D33 | "Stale" read as "newer upstream" | `18…:650-658` (P6·Q1) | 32 (60dc80e; D45) | `resources.ts:697`, `:753`, `:1199-1207` "out of date copy — … Deploy this to fix" |
| D34 | Selected-but-undetected Tool (Windsurf) is standing hollow noise | `18…:602-605` (P5·Q3 side note); `20…:216-219` | 20 (1c2cc4b; D47) | `src/components/settings/HarnessesPane.tsx:251-259` ("Selected, but not detected …" + "Remove from selection") |
| D35 | Auto-sync hard to find; "nothing on the main screen says it is on" | `18…:475-484` (P3·Q1) | 20 (D49) + 33 | hint dropped by name in `r19/issues/20-settings-panes.md:236-238`; moot once auto-deploy changes nothing on Update (`refresh.rs:22`) |
| D36 | Import: managed name vs "Match"; remove-originals choice; Conflict variants' fate; no Result step; unmanaged skills stay revisitable | `18…:485-542` (P3·Q2–Q4), `:691-693` (debrief 1, Import half) | 21 (e6be057) | `resources.ts:1492-1494` (same/differs from your managed copy), `:1469-1481` (left in place), `:1508-1510` (remove-originals choice), `:455-458` (name managed); `r19/issues/21-import-review.md:88` § Result step, `:97` banner persists |
| D37 | Monochrome UI, light theme blander | `18…:684-689` (P7) | 30 (9ca0bc8; #78) | `src/styles/theme.css:104` identity tokens |
| D38 | Writing tracked files reloads the fixture page (facilitator incident) | `18…:244-255` | AGENTS.md | `AGENTS.md:71` (Environment gotchas, fixture reload) |
| D39 | Release smoke's VoiceOver / reduced-motion half (15/16/17 audits' "operator half of D31") | `ticket-15-audit.md` § Acceptance 4; `ticket-16-audit.md` § Acceptance 5; `ticket-17-audit.md` § Acceptance 5 | 19e896c (D31 amended) | `r19/spec.md:166` "VoiceOver and reduced motion are **never** part of a release smoke"; `AGENTS.md:53` |

## 4. Drop candidates

| id | title | source | reason to drop |
|---|---|---|---|
| D40 | Per-skill backend read instead of N `get_project_view` calls for the inspector's Projects section | `17…:184-186` (from ticket 12) | `useProjectState.ts:575-599` still fans out, but `get_project_view` is a plain DB read (`commands/projects.rs:111-119`, no reconcile, no filesystem) — no cost worth a new command at the operator's project count |
| D41 | Enable skipped for a missing folder leaves rows `disabled`; recovery is Enable or uncheck/recheck | `ticket-17-audit.md:187` (Question (a)); `17…` Comments "Re-check focus" | Ruled correct by the re-check under D8/ADR-0002 (a skip must not undo Disable/Keep); `CONTEXT.md:227-229` documents the missing-folder rule |
| D42 | Mid-copy partial files when a root vanishes during copy | `ticket-17-audit.md:211` | Same exposure as any mid-copy failure; no new risk (orchestrator's own note) |
| D43 | Native header `Select` type-ahead / macOS arrow behaviour | `17…:174` | Platform behaviour of the native control, by design |
| D44 | Keyboard-triggered switch/rail motion vs Emil's no-keyboard-animation rule | `ticket-15-audit.md:105` | The world contract names both motions; audit accepted them as an explicit exception — see Operator decision 1 |
| D45 | Recording lost / ~1 fps / missing segment | `15…` § Recording; `ticket-15-audit.md:16`; `18…:708-709` | Evidence-capture artefacts; rAF/WAAPI numbers and the written notes are the record |
| D46 | Documentation-count corrections ("17 all + 3" transitions; "33" vs 40 i18n keys) | `ticket-15-audit.md:100`; `ticket-17-audit.md:32` | Historical wording in closed tickets; no code consequence |
| D47 | `CountConfirmModal`'s Tool pick as Add's Tool-scope step | `16…:165-168` (from 13) | Rejected in 16 § Decisions (`bulkCounts.syncCounts` counts existing pairs; Add says "N skills to M Tools") |
| D48 | Run toast is headline-only with "View report" — "worth an Emil look" | `15…:119` (from 14) | 15 did not change it; the headline + panel split is D10/ticket 14's design and the think-aloud read the count line and panel correctly (`18…:326-333`) |
| D49 | Summary ↔ progress swap instant | `15…:119`; `ticket-15-audit.md` § Emil rubric | Decided "stays instant", `DESIGN.md` § Motion "What does not move" |

## 5. Operator decisions

1. Should switch-thumb and bulk-rail entry motion be suppressed when triggered from the keyboard (Emil's literal rule)?
   — **No; drop D44.** — Both are 120 ms, interruptible, contract-named, and the audit found the rail fires once per
   selection, not per keystroke.
2. D2 (bulk rail at the foot): fold into #72 as a reading fix, or drop as a deliberate placement?
   — **Fold into #72 (low).** — The operator found it only after trying three other paths; a cheap check (rail
   placement or a selection hint at the header) belongs with the other My Skills reading fixes.

**Summary** — Carry 3 · Covered 14 (amend #70, #72, #73) · Closed 22 · Drop 10 · Operator decisions 2.
Nothing in slice D is a P0/P1 left open; both holds (27, 28) and every P1 (15 A1, 16 A1–A3, 17 A1 + its race) are closed in code.
Most exposed: D1 (R1, every row's Enable popover, passed through 16 → 17 → 23 with no owner) and D3 (an unpinned Base UI internal under a caret range).
