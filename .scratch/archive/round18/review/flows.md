# Round 18 · ticket 07 — Task-flow and state review of the six core flows

Method: `product-design-and-ux` (SKILL.md workflow steps 3–5, 7; `references/task-flows-and-state-models.md`,
`references/content-and-cognitive-demand.md`, `references/interaction-pattern-selection.md`; templates
`task-flow-state-model.md` and `screen-state-inventory.md`, applied per flow below). Judged: the **incumbent** app
(main + the fixture browser mode of ticket 04), not the prototypes. Date: 2026-09-26.

## Evidence boundary — read this first

- **Validated user: one — the operator (Alex).** `PRODUCT.md` § Users names no other confirmed audience; no usability
  study, analytics, support data or external users exist. No personas are invented here. Every "the operator will…"
  below is a *hypothesis* from heuristic review plus source reading, and the § "Usability questions" per flow are what
  a think-aloud session with the operator would need to confirm or kill.
- **Observed** = driven in the fixture app at `http://localhost:5175` (`?scenario=failures`, `latency=0|3`, and
  `first-run`) through `preview_evaluate`; the copy quoted in "…" is what the app rendered. **Described from code, not
  observed** = the fixture cannot produce the state, or produces it differently from the backend; the Rust or TS producer
  is cited. Fixture fidelity gaps that matter are listed in § Fixture gaps.
- Screenshots (gitignored, local): `/Users/alexstark/Projects/skills-hub/.scratch/round18/evidence/flows/`
  `f1-overwrite-ask.png` (four stacked layers during Add), `f3-import-review.png` (Review discovered skills),
  `f4-matrix.png` (project matrix, colour-only cell states, raw tool keys), `f6-delete-kept.png` (Delete kept by a
  locked target, same confirm re-shown). Ticket 04's `…/evidence/app/failures-refresh-report-dark.png` shows the
  Refresh toast stack. Other screenshots were lost to full-page reloads caused by a sibling child regenerating
  `src/bindings/index.ts`; the textual captures in this file are the primary evidence.
- A usability session must **not** run on `npm run tauri:dev` — it mutates the operator's live library (AGENTS.md
  § Environment gotchas). Use the fixture (`failures` / `first-run`) or a release build pointed at a scratch
  `central_repo_path` on a test account.

---

## 0. Cross-cutting findings (they recur in every flow; ranked)

| # | Finding | Evidence | Why it matters to the outcome |
|---|---|---|---|
| X1 | **Outcome memory is toast-shaped and session-only.** Every report is folded into toasts + a flat in-memory Notifications list (no run grouping, lost on restart). Failures that leave **no row** — a declined/failed *first* sync (`global_sync.rs` records only `Synced` targets), a failed Refresh acquisition, a failed import group — leave **no trace on any card**: after "Keep existing", `ts-reset`'s card simply lacks a Claude Code pill; after Refresh, `deprecated-helper` looks identical to a healthy skill. | Observed (cards before/after); `useStatusReporter` + CONTEXT § Notification | Principle 5 ("every failure is shown, typed, and recoverable") holds only until the toast is closed or the app restarts. The operator cannot answer "is my library healthy?" at a glance (Principle 1). |
| X2 | **The loading overlay's Cancel lies for most actions.** Only 5 commands take the `CancelToken` (`install_git_selection`, `refresh_managed_skills`, `update_managed_skill`, `repoint_skill_source`, `clone_explore_skill` — `commands/mod.rs`). `cancelLoading` hides the overlay for *every* action; sync, import, delete, unsync-all and project ops keep running and their toasts arrive later. During the overwrite ask, Cancel removes the overlay but the ask stays and the Add loop continues unblocked. | Observed (Add + ask + Cancel); code `useStatusReporter.cancelLoading` | The operator believes work stopped; it did not. The UI unlocks mid-mutation (the Mutation guard serialises backend writes, but the frontend now permits a second action). |
| X3 | **One blocking modal overlay for every action, titled "Installing Skills..."** with "Cloning repository and syncing to tools" / "First fetch may take a while…" — shown for Refresh, Delete, Restore, Re-point, Import, project resync. The whole window is blocked for the duration (Refresh of ~80 skills ≈ minutes on the real library). | Observed (`processingTitle`, `processingTipShort`) | Wrong status copy (heuristic: truthful status), and a daily-driver utility cannot be browsed while its longest batch runs. |
| X4 | **Tool keys leak where labels belong.** Matrix headers `CLAUDE_CODE AGENTS_SKILLS WINDSURF`; errors "Cannot sync to amp: …", "Cannot sync to kimi_cli…"; import review "Found in cursor", "Found in gemini_cli"; `.agents/skills` used as a tool name in Refresh errors with no project named. Also "Please … run as administrator" on macOS, "missing_name" raw reason, "1 targets". | Observed | Domain language mismatch; the operator has to translate keys → tools while reading failure lists. |
| X5 | **The operator's own choice is reported as a failure.** "Keep existing" in the overwrite ask ends as an *error* toast "Sync failed: ts-reset -> Claude Code — Target folder already exists and was left in place." inside "Some actions did not complete". | Observed | Trains the operator to ignore red; a decline is a decision, not a fault. |
| X6 | **Auto-sync silently re-deploys what the operator removed.** Unsync of `tdd` ("Remove this skill from all tool directories") → Refresh → `tdd` is back on Cursor, Claude Code, Codex, Gemini CLI. Re-point also fanned `deprecated-helper` out to three *new* tools (the re-assert). Nothing in either flow says so. | Observed | Unsync has no durable meaning while auto-sync is on — exactly the gap #52 (enable/disable) exists to close. |
| X7 | **Destructive/wide actions have inconsistent barriers.** "Uninstall from tool directories" removed **68 deployments with no confirmation**; per-skill unsync and Detach are one click with no confirm or undo; Delete has a generic confirm that omits project assignments and Edits. Project bulk unassign is deliberately unconfirmed (round 16 D2: reversible). | Observed; `FilterBar.tsx:136`, `ProjectsPage.tsx:195` | Blast radius, not reversibility alone, should pick the barrier (§ pattern decision P2). |

### The fold's precedence contract (`src/lib/reportOutcome.ts`) — judgement

The fold picks **one headline** by `conflict › failure › skipped › success` and pushes every item into toasts +
history. Judged per flow:

- **Refresh (all): wrong for the headline.** Observed: 5 skills failed, 6 targets failed, 4 skipped, 1 Edit conflict →
  the headline read "**Refresh complete — invocation Edit conflicts need review**" (warning). The word "complete" and
  the absent counts hide five failures behind the rarest, least urgent outcome. A conflict is *additive information about
  a skill that refreshed successfully* (ADR-0004: the Edit wins); it must not outrank failure. `refreshSummary` also
  drops the skipped count whenever anything failed.
- **Cancelled Refresh: wrong (described from code).** A cancel returns every selected skill `Failed { Cancelled }`
  (`core/refresh.rs:219-233`); the entries have empty messages and are dropped, but `summary.failed > 0` → warning
  "**0 skills refreshed, 15 failed.**" for an operator who pressed Cancel. (Fixture throws `CANCELLED` instead and shows
  nothing — § Fixture gaps.)
- **Batch-of-one (Update/Restore/Re-point): right.** The single error entry *is* the headline; no duplicate summary.
- **Edit: acceptable, but lossy.** With target failures the fold shows only the errors and **no** "saved" — the Edit
  *did* save centrally (ADR-0004); the operator cannot tell saved-with-drift from not-saved.
- **Install / sync / import: precedence is not the problem, vocabulary is.** Headline "Some actions did not complete"
  (`partialFailure`) says nothing about what *did* complete (4 skills installed and deployed to 5 tools).
- **Project sync/removal:** failure › success with counts ("Synced 7, 4 failed") — the best-shaped of the folds.

**Verdict:** keep the fold as the single place reports become presentation (AGENTS.md invariant), and keep precedence
for the *toast tone* only. Replace the single-headline message with a **composite count line** ("12 refreshed · 5 failed
· 4 skipped · 1 Edit to review") and move the per-item list off toasts into a **persistent last-run report** addressed
from the toast (see R-X1). Separately classify *declined* and *cancelled* as their own outcome kinds, never failures.

### Cross-cutting recommendations

- **R-X1** Persistent outcome surfaces: a last-run report per batch action (opened from its toast) plus stored
  skill-level health rendered by #49 and the detail health ladder; Notifications grouped by run. (X1, P1)
- **R-X2** Cancel only where the command honours the token; otherwise no Cancel, or extend cancellation (X2).
- **R-X3** Action-specific progress copy and a non-blocking progress affordance (X3, P3).
- **R-X4** Tool labels everywhere a key leaks; OS-appropriate permission copy; human reasons for manifest errors (X4).
- **R-X5** `declined` and `cancelled` outcome kinds in the fold (X5).
- **R-X6** Announce auto-sync re-assert effects in every report that triggers it; #52 makes unsync durable (X6).
- **R-X7** Barrier chosen by blast radius and reversibility (P2) (X7).

---

## Flow 1 — Add → pick → sync → overwrite ask

**Outcome (the operator's, not "report shown"):** the chosen skills exist as Managed skills **and** are live in every
Tool of the chosen target set, with any occupied target resolved by an informed decision — and the operator can see,
later, which Tools do *not* carry them and why.

Entry: My Skills → **Add skill** (Git Repository / Local Folder tab; "Install to tools" checklist of *detected* tools;
"Sync to selected tools after creation"). Also Explore → install (hands a URL into the same flow).

### Flow

| Step | Actor/system | Decision / alternative | Side effect | Recovery / cancel / re-entry |
|---|---|---|---|---|
| 1 | Paste URL / choose folder, pick target tools | Local vs git; custom name only for a single skill | none | Close modal (disabled while an action runs) |
| 2 | `listGitSkillsCmd` / `listLocalSkillsCmd` | 0 → error; 1 valid → straight install; many → picker | git: clone cache widened | Listing failure → error toast, modal stays |
| 3 | Picker "Select skills to import" stacks **over** the Add modal; all preselected | Deselect; "Select all" | none | Cancel discards the listing |
| 4 | **Frontend loop**, one candidate at a time: `installGitSelection` → finalize → `deploy()` = one sync batch per skill | — | **Central copy + record written per skill** (irreversible short of Delete) | Overlay Cancel: backend honours it only inside the current `install_git_selection`; the loop continues (X2) |
| 5 | A batch with `TARGET_EXISTS` rows raises the **overwrite ask** ("Folder already exists … ts-reset → Claude Code  /Users/alex/.claude/skills/ts-reset  [Keep existing] [Overwrite]") | Keep / Overwrite, per ask; **one ask per candidate** (code: `deploy` per skill) | Overwrite = second batch with per-pair override; replaces foreign bytes (irreversible) | No ✕; overlay Cancel leaves the ask up (observed) |
| 6 | `installOutcome` → toasts; any install ⇒ modal closes, picker resets, library reloads | — | — | Failed candidates must be re-listed from the URL; nothing remembers them |

### States (force-driven)

| State | Force | User-visible behaviour (observed unless noted) | Exit / recovery |
|---|---|---|---|
| Listing | fetch | Overlay "Installing Skills… / Cloning repository and syncing to tools" | Cancel honoured for git listing |
| Listing failed | network/auth/rate limit | Error toast; modal keeps URL | Edit URL, retry |
| Picker | many candidates | Two modals stacked; count "4 / 4 selected"; no search (#47) | Cancel |
| Installing n/N | background work | "Import (2/4) name …" — *Import* is Onboarding import's word | Cancel partially honoured |
| Awaiting overwrite decision | conflict | Four layers: overlay ("Waiting for your confirmation…"), Add modal, picker, ask (`f1-overwrite-ask.png`) | Keep / Overwrite only |
| Partial success | per-target failure | "Some actions did not complete" + error toast "Sync failed: ts-reset -> Amp … permission denied … run as administrator (+3 more)" | Notifications panel; no retry affordance |
| Declined | operator choice | Reported as error (X5) | none — no later "sync it anyway" path from the card except toggling the tool in detail |
| Installed, not deployed to some tools | failed first sync | **No row, no pill, no mark** (X1) | Operator must remember |
| Name taken | validation | "skillAlreadyExists" before any write | Rename (single only) |
| Auto-sync off / no targets | settings | "Skill not synced — no sync targets" error entry | Configure Tools |

### Gaps (ranked) → round-19 recommendation

1. **A failed or declined first sync leaves no durable trace** (X1). → R1.1: the post-install state is the card, not
   the toast: show the skill's per-tool coverage as the **"Synced N/M" per-tool strip** (D8) with hollow = not deployed,
   × = failed, and let the strip cell itself be the retry/overwrite entry. Backend note for the grill: a declined or
   failed first sync currently has no row to render — decide whether to record a `failed` target row (ADR-0002 already
   keeps error rows for removal) or derive "not deployed" from the effective target set. *D8: Synced N/M strip, #49 chip.*
2. **One overwrite ask per candidate, mid-loop, under four layers.** A 10-skill pick with collisions interrupts ten
   times, each time after bytes already landed. → R1.2: move the fan-out into one backend install batch (the
   AGENTS.md "one batch command per operator action" rule currently holds for sync but not for the picker's install
   loop) and raise **one** ask listing every occupied pair across the pick, with per-row Keep/Overwrite and a
   "view existing" path link. *D8: sticky install summary in Add.*
3. **"Keep existing" is an error.** → R1.3: a new outcome kind `declined` (neither failed nor skipped): quiet line in the
   summary ("ts-reset kept your existing Claude Code folder"), no error toast, and the strip shows the tool as
   "occupied — yours" rather than failed.
4. **Picker partial failure resets the picker.** `closeModal = any installed`, so failed candidates vanish. → R1.4:
   keep the picker open on partial failure with installed rows ticked-off and failed rows retryable; this is the
   "sticky install summary" D8 reserves.
5. **Vocabulary**: picker title "Select skills to import", progress "Import (i/n)", overlay "Installing Skills…" used
   everywhere. → R1.5: "Add" family for this flow; "Import" reserved for Onboarding import (CONTEXT § Language).
6. **Cancel during the loop does not stop it** (X2). → R1.6: either make the batch cancellable end-to-end (stop
   dispatching the next candidate, report the rest as `cancelled`) or remove Cancel from non-cancellable phases.
7. **No picker search** at 28-skill repos. *D8: #47.* **Invalid folders listed** in local pick are filtered; git lists
   manifest-less dirs only if they carry a `SKILL.md` — consistent with *#46* for onboarding.
8. After installing a whole repo, deploying the *group* is not offered. *D8: #39 group-level Deploy all.*

### Usability questions (operator walkthrough)

- After Add finishes, where do you look to know it worked on every tool? What do you expect to see for Claude Code after
  "Keep existing"?
- In the overwrite ask, what do you need to decide — the other folder's content, its age, who put it there?
- When you pick 6 skills and 3 collide, would you rather decide per skill as they land, or once at the end?
- You pressed Cancel during install — what do you believe was installed?

---

## Flow 2 — Refresh (all) and its report

**Outcome:** every refreshable skill carries its upstream's current bytes on every target, operator Edits intact; every
skill that could *not* be refreshed is known, with its reason and the next action — still visible tomorrow.

Entry: My Skills → **Refresh** (toolbar). Per skill: **Update** icon (batch of one). No scheduled refresh (#53, out).

### Flow

| Step | System | Alternative | Side effect | Recovery |
|---|---|---|---|---|
| 1 | Select members (`provenance::refresh_eligibility`); imported skills are not members; Unlocatable are members reported skipped | — | none | — |
| 2 | Phase 1 acquire, pool of 4, ticks "Fetched (i/N) name …" | Cancel stops dispatch; nothing reaches phase 2 | Staging dirs, git cache | Cancelled report (see fold judgement) |
| 3 | Phase 2 per skill under the guard: finalize + Edit replay + Propagation; ticks "Updating (i/N) …" | stale acquisition → skipped | **central copies replaced**, targets re-materialised | ADR-0004 rollback per skill |
| 4 | Re-assert (auto-sync on): sync each refreshed skill to every tool of the effective target set it lacks | `TARGET_EXISTS` here → reported failed, **no overwrite ask** (code, `refresh.rs:460-526`) | **new targets created** | none offered |
| 5 | `refreshOutcome` → headline + one entry per item; library reload | — | — | Notifications panel; "Change source…" action only on `GITHUB_SKILL_NOT_FOUND` |

### States

| State | Force | Observed behaviour | Exit |
|---|---|---|---|
| Acquiring i/N | background | Blocking overlay; "Fetched (3/15) frontend-design …" (latency 3) | Cancel (honoured) |
| Applying i/N | background | "Updating (i/N) …" | not cancellable (by design) |
| Complete, all healthy | — | "All skills refreshed." (success, 2 s) | — |
| Complete with failures / skips / conflict | partial | Headline "Refresh complete — invocation Edit conflicts need review"; toasts "Skipped: docx … (+4 more)", "Update failed: pr-review … (+10 more)"; history holds 19 lines | toasts; panel |
| Per-skill failed (rate limit, 404, auth, TLS, symlink escape) | network/upstream | Typed, well-written messages; only 404 carries an action | none on the card (X1) |
| Target failed (unwritable shared dir; project `.agents/skills`) | permissions | "Could not update the synced copy: tdd -> .agents/skills" — no project named; also used for re-assert targets that never existed | none |
| Re-assert store failure | DB | "Could not check which tools still need store-hiccup — database is locked" | none |
| Cancelled | operator | Code: "0 skills refreshed, 15 failed." warning. Fixture: silent | — |
| Unlocatable member | disk | "Skipped: docx — Its central copy is missing. Restore or remove it from its card." — the card *does* show it (only skill-level state that is persistent) | Restore / Re-point / Detach / Remove |

### Gaps → recommendations

1. **Headline hides failure** (fold judgement). → R2.1: composite count headline; conflict as an addendum; skipped
   always counted; `cancelled` as its own kind ("Refresh cancelled — nothing was changed").
2. **Skill-level health is not persisted** (the spec § Bake-off finding (b)). Failed acquisitions, Edit conflicts and
   target errors all vanish into history. → R2.2: a skill-level health channel distinct from target health, rendered as
   the **issues banner + Issues chip (#49)** on My Skills and the **detail health ladder** (Health / Upstream / Edits /
   Last synced) with **Retry failed**; the last refresh outcome per skill needs a stored home (backend decision for
   the grill: a `last_refresh_error` on the skill row vs a run log table). *D8: #49, detail health ladder.*
3. **Blocking overlay for a minutes-long batch** (X3). → R2.3: non-blocking progress — a **passive title-bar
   affordance** (D8) showing "Refreshing 12/80" with Cancel, the library stays browsable (the guard is per skill, so
   reads and listing are not blocked backend-side), and per-row spinners on the skill being applied.
4. **Re-assert creates targets silently and can hit `TARGET_EXISTS` with no ask.** → R2.4: report re-assert additions
   as "newly deployed to N tools" and route re-assert `TARGET_EXISTS` into the same overwrite ask as Flow 1 (one ask
   per run), or into the Issues list with an Overwrite action.
5. **Recovery affordances are uneven**: only `GITHUB_SKILL_NOT_FOUND` offers "Change source…"; rate-limit copy points at
   Settings with no link; auth failure offers nothing. → R2.5: every typed failure maps to a next action in the report
   row (Open Settings → GitHub token; Change source…; Retry).
6. **Project targets reported without the project** ("tdd -> .agents/skills"). → name the project, as
   `resyncAll` already does ("Sync failed in monorepo: …").
7. **History is a flat list** — two Refresh runs interleave 38 identical lines with no run boundary. → group
   Notifications by run with the headline as the group row.

### Usability questions

- After Refresh, which skills do you believe failed? Show me without opening Notifications.
- "Refresh complete — invocation Edit conflicts need review": what happened?
- The next morning, how would you find yesterday's failures?
- While Refresh runs you want to check one skill's source — what do you do?

---

## Flow 3 — Onboarding import

**Outcome:** every skill already sitting in a Tool's directory that the operator wants is now a Managed skill, every
Tool that held it now carries the managed copy (auto-sync on) or no longer carries an untracked copy (auto-sync off),
and anything left behind is left *on purpose* and visible.

Entry: banner "Discovered skills — Found N skills ready for review. [Review & Import]" on My Skills (also the
first-run entry: `first-run` shows "Found 22 skills ready for review." above "No managed skills yet." — no guidance
that import is the natural first step).

### Flow

| Step | System / operator | Decision | Side effect | Recovery |
|---|---|---|---|---|
| 1 | Plan fetched afresh under the scan scope | — | none | Plan reloads on tool-config save |
| 2 | "Review discovered skills": groups, all preselected; conflicting groups get radio variants, first preselected | select groups; pick variant | none | Close |
| 3 | **Import & Sync** (label unchanged when auto-sync is off) | — | finalize (central copy, `imported` provenance) | overlay Cancel is **not honoured** by `import_onboarding_selection` (X2) |
| 4a | auto-sync on: sync batch; takes over byte-identical originals in every Tool of the group | — | **Tool copies overwritten by links** | divergent siblings kept + reported |
| 4b | auto-sync off: **byte-identical originals removed** | — | **Tool copies deleted** (irreversible in the sense that the Tool loses the skill until a sync) | — |
| 5 | `importOutcome`; modal stays open on any warning/error with a re-fetched plan | — | — | failed groups still preselected |

### States

| State | Force | Observed | Exit |
|---|---|---|---|
| Review, consistent group | data | "commit-message · Match" with two paths | select |
| Review, conflicting group | data | "sql-review · Conflict", radios; **no content preview or diff** between variants | pick blind |
| Name already managed | data | Listed as "Match", preselected — e.g. `tdd` (a *divergent fork* in Pi), `frontend-design`, `prisma-review`, `zod-schema-first` | fails on import (below) |
| Manifest invalid | data | `broken-skill` listed "Match", preselected | fails "This path is not a valid skill: missing_name." |
| Partial import | per group | "3 skills imported, 5 failed." + "Kept a different copy: sql-review -> Gemini CLI …" + "Import failed: … (+11 more)" | modal stays, "Skills found: 7" |
| Name collision failure | data | '"tdd" already exists in Hub. Go to My Skills to update it.' — the advice does not resolve it | none |
| Divergent sibling | data | Reported **twice**: warning "Kept a different copy…" and error "Sync failed: sql-review -> Gemini CLI  Target folder already exists" | none |
| Stale selection | settings | "Skipped Windsurf: not detected — 3 skills were not deployed there. Install the tool, or untick it under Configure Tools." (good) | Configure Tools |
| Auto-sync off import | settings | Same button "Import & Sync", same intro "Select skills to import and sync to your tools."; originals removed; **no line says which Tools lost their copy** | — |

### Gaps → recommendations

1. **Auto-sync-off import deletes Tool copies behind an "Import & Sync" label.** Highest risk in this flow. → R3.1:
   the review states the policy per mode before the button — "Your N originals in Cursor, Claude Code… will be
   removed; the skills stay in Skills Hub and can be synced back" — and the button reads "Import and remove
   originals"; the report lists removed originals per Tool.
2. **The review cannot see Managed-skill collisions.** Four of eight groups were guaranteed failures, preselected. The
   real choices for such a group are "Replace this Tool's copy with the managed skill" (an overwrite — Flow 1's ask)
   or "Leave it"; neither is offered, and the copy "Go to My Skills to update it" is wrong. → R3.2: the plan marks
   groups whose name is managed; they are not preselected and offer "Replace with managed" / "Leave". (Backend question
   for the grill: the plan does not carry "name is managed" today.)
3. **Invalid manifests are importable.** → *D8: #46 `SKILL.md`-gated scan*; show invalid candidates disabled with the
   reason in words ("SKILL.md has no name"), never raw `missing_name`.
4. **Conflict variants are chosen blind.** → R3.3: per-variant description, modified time and a "Compare" (diff of
   `SKILL.md`) before the radio; the default stays the real-directory variant (CONTEXT § Onboarding import).
5. **Counts disagree**: banner "Found 11 skills", review "Skills found: 11" over 8 name-groups. → count names, state
   variants separately ("8 skills · 11 copies in 8 tools").
6. **Partial import keeps failed groups preselected**, so "Import & Sync" again repeats the same failures. → after a
   run, failed groups are deselected and annotated with their failure; imported groups disappear (they do).
7. **Double-reported divergent sibling; raw tool keys** ("Found in gemini_cli"). → one warning row; tool labels (X4).
8. First run: the banner competes with an empty list that says only "No managed skills yet." → the empty state
   *is* the import invitation; this is D6's "rare delight" moment (a large import completing).

### Usability questions

- (auto-sync off) Before you press the button: what will happen to `~/.cursor/skills/zod-schema-first`?
- `tdd` is listed with a Match badge — what do you expect Import to do with it, given you already manage `tdd`?
- For `sql-review` (Conflict), how did you choose between the Claude Code and Gemini CLI copies?
- After the partial import, what would you do next with the five failures?

---

## Flow 4 — Project assignment / bulk / resync

**Outcome:** the Project's repo carries exactly the chosen skills in each configured Tool's project dir, drift (`stale`)
and missing artifacts are healed, and any assignment that could not be materialised is visible as such in the matrix.

Entry: **Projects** → select a project → matrix (skills × the project's configured Tools). Per cell: toggle. Per row:
"All Tools" (bulk assign), "Unassign All" (bulk unassign, unconfirmed by decision — round 16 D2). Header: **Sync
Project** (resync), **Sync All** (resync every project), Configure Tools, Remove Project (sidebar gear/trash).
There is no project assignment from My Skills (operator friction, `PRODUCT.md` § Evidence on Hand).

### States

| State | Force | Observed | Exit |
|---|---|---|---|
| Cell synced / stale / missing / pending | reconcile | **Colour only** (pale yellow, pink); stale/missing/pending cells carry no text or tooltip; only `error` has one ("Sync failed: Permission denied (os error 13): …/.agents/skills/tdd") (`f4-matrix.png`) | Sync Project |
| Cell "global" | global target exists | Disabled checkbox + globe; title "This skill is already deployed globally for this tool. Assign it here only to manage it per-project." — but disabled, so it cannot be assigned here | **"All Tools" bypasses it** (observed: alias-skill → Claude Code assigned via bulk) |
| Toggle on fails | permissions | Error toast only: "Sync failed: design-tokens -> .agents/skills …"; cell turns red (row kept `error`) | toggle off / resync |
| Bulk assign partial | permissions | "2 assigned, 1 failed" (one of the two was the undetected Windsurf) | — |
| Resync partial | mixed | "Synced 7, 4 failed" + entries | — |
| Resync all | many projects | "Synced 9 across all projects, 6 failed"; includes **"Sync failed in archived-app: tdd -> Claude Code — project directory not found"** per assignment | — |
| Project folder missing | disk | Sidebar ⚠ + "(not found)"; "Sync is disabled because the project directory was not found."; **Update Path** icon; cells show the not-found error after Sync All | Update Path / Remove |
| Tool not detected in project config | settings | `WINDSURF` column, no mark; assignments to it succeed | — |
| Remove project | destructive | Confirm lists "All deployed symlinks/copies will be removed from the project · Skill assignments for this project will be deleted"; success "Project removed" | — |
| Remove project with a stuck artifact | ADR-0002 | **Described from code, not observed** (fixture's unwritable dirs affect writes only): project kept, `projects.removeKept` warning, failure-path `refreshView` re-shows the red cells | retry Remove |
| Reconcile skipped | guard busy | **Described from code**: `reconciled: false` when a mutation holds the guard — the matrix may show pre-reconcile statuses with no indication | — |

### Gaps → recommendations

1. **Status is colour-only and unexplained** (also a D5 accessibility gate failure). → R4.1: shape + word per state,
   `stale` worded as drift ("copy differs from the skill — Sync to fix"), never "update available"
   (spec § Bake-off (a)); a legend; per-cell "Sync this" on stale/missing/error instead of whole-project resync only.
2. **Header shows raw keys** (`CLAUDE_CODE`). → labels + icon; the undetected Windsurf column says "not detected".
3. **Sync All hammers a project whose folder is gone** and reports one failure per assignment. → skip it with one line
   "archived-app: folder not found — Update path or Remove", matching the disabled Sync Project.
4. **"Global" cells: disabled for the toggle, assignable by bulk.** Decide one rule (the tooltip says assignment is
   allowed). → make the cell enabled with a "also global" mark, or have bulk skip it and say so.
5. **Assignment only from inside the project.** → *D8: per-row / inspector project assignment* (assign from My Skills
   and the detail inspector) and **#52 multi-select bulk "assign to project"**; the matrix remains the per-project view.
6. `reconciled: false` is invisible. → a quiet "statuses may be out of date — Refresh view" line when it happens.

### Usability questions

- Point at every cell that needs attention and tell me what is wrong with each. (Colour-blind variant: grayscale.)
- You want `tdd` in three projects — where do you start?
- `archived-app` is gone from disk. What do you do, and what do you expect Sync All to say about it?

---

## Flow 5 — Re-point + Edit + Restore (repairs and tuning)

**Outcome:** (Re-point) the skill follows the intended new source with its targets, assignments and Edits intact;
(Edit) the skill behaves the way the operator chose in every Tool that honours the key, and keeps doing so across
Updates; (Restore) an Unlocatable skill is whole again on every target.

Entries: card icons **Update**, **Change source…**; Unlocatable inline strip "Source folder missing · Re-point ·
Detach · Remove" / "Central copy missing · Restore · Remove"; Refresh/Update 404 toast action **Change source…**;
invocation badge ("User only — Overridden") opens the Edit modal.

### States

| State | Force | Observed | Exit |
|---|---|---|---|
| Change source (git) | operator / 404 | "Change the source of deprecated-helper … Paste a repository or skill folder URL. Your source changes only after acquisition succeeds; existing sync targets and project assignments are preserved." Empty field; **current source not shown** | Cancel / submit |
| Re-point refused | validation | Error **toast** "Update failed: deprecated-helper — This repository contains multiple skills…"; modal stays, input kept | edit URL |
| Re-point to a whole repo | data | Refused; no candidate picker (Add has one) | paste subfolder URL |
| Re-point to a different skill | data | **Accepted**: `deprecated-helper` re-pointed at `…/ts-error-fixer`, name kept, success flash only (code: no identity check in `core/repoint.rs`) | Change source again |
| Re-point side effect | auto-sync | Re-assert deployed the skill to Cursor, Pi, Gemini CLI — unannounced (X6) | — |
| Source folder missing | disk | Card label shows **`/Users/alex/.skillshub/local-notes` — the central path, not the missing folder** (`skillSourceLabel` returns `central_path` for every non-git skill); the Re-point modal also omits the old path | — |
| Detach | operator | One click, no confirm; toast "local-notes detached: the copy in Skills Hub is now its source."; card then says "**Imported from unknown**" | Re-point |
| Restore | disk | "docx restored." (success) | — |
| Restore/Update failed | upstream | Single error entry (fold: batch-of-one, correct) | retry |
| Edit, conflict | upstream changed | Badge dot + tooltip "Overridden; upstream changed its setting to …"; modal "…Keeping your override re-bases it; 'Follow the skill's own setting' clears it." | Save / Follow |
| Edit saved, propagation failed | permissions | **Described from code**: errors only, no "saved" (fold judgement) | — |

### Gaps → recommendations

1. **Local skills never show their source folder** — the one fact the source-missing repair needs. → R5.1: show the
   recorded source (`source_ref`) for local skills, copyable, and "was: ~/Downloads/scratchpad" in the Re-point modal.
   *D8: detail health ladder + copyable source.* (This is a presentation bug worth fixing even before round 19.)
2. **Re-point has no preview of what will land.** A wrong URL silently swaps a skill's identity. → R5.2: resolve the
   target first and show "This will replace deprecated-helper's bytes with ts-error-fixer (skills/engineering/ts-error-fixer
   @ main)" with name mismatch highlighted; offer a candidate picker for multi-skill repos (reuse the Add picker, #47
   search). Pre-fill the field with the current source.
3. **Re-point / Restore errors arrive as toasts over an open modal.** → inline error in the modal, next to the field.
4. **Detach is unconfirmed and relabels history wrongly** ("Imported from unknown" for a skill that was never
   imported). → confirm stating the consequence ("stops following ~/Downloads/scratchpad; Skills Hub's copy becomes the
   source; Refresh will skip it"), and a provenance line "Detached from ~/Downloads/scratchpad" (needs the found-in /
   detached-from field — backend question).
5. **Edit conflict is a 6-px dot and a tooltip; jargon "re-bases".** → conflict is skill-level health (R2.2) with the
   plain choice "Keep yours (User only)" / "Use the skill's (User & model)"; the report's Edit-conflict line links
   straight into this modal.
6. Re-point/Restore trigger auto-sync re-assert → say "also deployed to Cursor, Pi, Gemini CLI" (X6).

### Usability questions

- `local-notes` says source folder missing. Where was it? How will you point it at the new location?
- You pasted the wrong skill's URL into Change source. When would you notice?
- `design-tokens` has a conflict: what does upstream want, what did you choose, what will the next Update do?

---

## Flow 6 — Delete / unsync (Artifact removal)

**Outcome:** (unsync) the chosen Tools no longer carry the skill **and stay that way**; (delete) the skill and every
artifact of it are gone — or, where an artifact could not be removed, the operator knows exactly what remains, where,
and how to finish (ADR-0002).

Entries: card link icon "Remove this skill from all tool directories" (unsync skill, `SkillGlobal`); toolbar
"Uninstall from tool directories" (unsync all, `EveryGlobalTarget`); tool pill click (unsync one tool; undetected pill
"Click to remove it"); card trash **Remove** → confirm (delete, `Skill`); project Unassign All / cell toggle-off.

### States

| State | Force | Observed | Exit |
|---|---|---|---|
| Unsync skill | operator | **No confirm, no toast**; pills disappear | none (no undo) |
| Unsync persists? | auto-sync | **No** — next Refresh re-deployed `tdd` to 4 tools (X6) | — |
| Unsync all | operator | **No confirm**: "Removed 68 tool directory deployments; 1 could not be removed" + error "Could not remove from Claude Code — Permission denied …release-train" | row kept `error` (red pill) |
| Delete confirm | destructive | "Remove skill? Are you sure you want to delete release-train? · Remove from synced tools · Delete local copy from Hub" — no tool list, no mention of project assignments or Edits, same copy for an imported skill whose central copy is its only copy | Cancel / Yes, Delete |
| Delete partially done (kept) | locked target | Codex + Pi removed, Claude Code kept red; warning "Skill kept: 1 targets could not be removed. You can retry." + error; **the same confirm stays open unchanged** (`f6-delete-kept.png`) | Yes, Delete again after fixing permissions |
| Removal of a Tool no longer detected | registry | Pill "Windsurf (not detected) — Click to remove it" (good) | click |
| `PATH_OUTSIDE_TOOL_DIRS` refusal | stored path | **Described from code** (ADR-0002 note): row survives, typed refusal | — |
| Nothing planned | empty | **Described from code**: "unsyncNothingPlanned" warning | — |

### Gaps → recommendations

1. **"Uninstall from tool directories" removes everything in one unconfirmed click**, and auto-sync will re-add it all
   on the next Refresh. → R6.1: this action should not live in the everyday toolbar; with #52 it becomes "Disable all"
   (reversible, survives Refresh) in a menu, with the count in the confirmation ("Remove 69 deployments from 7 tools")
   or an Undo toast that re-syncs exactly the removed set. *D8: #52 enable/disable + multi-select bulk.*
2. **Unsync does not survive auto-sync.** → R6.2: the operator's "not on this Tool" must be recorded — #52's `disabled`
   transition (rows kept, Propagation `Skipped`), per skill and per (skill, Tool); the Synced N/M strip shows disabled
   as distinct from failed.
3. **The kept-delete retry is the original confirm, unchanged.** → R6.3: after a partial delete, the dialog becomes a
   report: "release-train was **not** deleted. Removed from Codex, Pi. Still in Claude Code: /Users/alex/.claude/skills/
   release-train — permission denied. [Reveal in Finder] [Retry] [Close]". The card shows "Delete incomplete" as
   skill-level health (#49).
4. **Delete confirm understates scope.** → list tools and projects affected, the number of Edits that will be lost,
   and — for `imported` or detached skills — "Skills Hub's copy is the only copy; deleting it cannot be undone".
5. **Unsync skill gives no feedback at all** and bypasses `runAction` (no overlay, errors only). → quiet success
   ("Removed from 4 tools · Undo") — undo is real here because the central copy remains.

### Usability questions

- (After Uninstall from tool directories) What did that just do? What will happen at your next Refresh?
- You removed `tdd` from all tools yesterday; today it is back. What do you think happened?
- Delete says "Skill kept". What exists now, where, and how do you finish?

---

## Screen/state inventory (consolidated; states the flows force)

| ID | Surface / state | Flow | Applicable force | Recovery / next action today | Round-19 contract link |
|---|---|---|---|---|---|
| SS-01 | Add modal + picker (stacked modals) | 1 | many candidates | Cancel | R1.2, R1.4, #47 |
| SS-02 | Overwrite ask (4 layers) | 1 (2, 3 by R2.4/R3.2) | occupied target | Keep / Overwrite | R1.2, R1.3 |
| SS-03 | Blocking overlay "Installing Skills…" | all | background work | Cancel (often not honoured) | X2, X3, R2.3 |
| SS-04 | Toast stack (+N more) | all | partial outcome | Notifications panel | R-X1 |
| SS-05 | Notifications panel (flat, session) | all | history | Copy all / Clear | R-X1, R2 grouping |
| SS-06 | Skill card: Unlocatable strip | 2, 5 | disk | Restore / Re-point / Detach / Remove | keep; extend to all skill-level health |
| SS-07 | Skill card: target pills (active / error / undetected) | 1, 2, 6 | target state | click pill | Synced N/M strip |
| SS-08 | Skill card: *absence* of a pill (never deployed / failed first sync / declined) | 1, 6 | missing row | none | R1.1 |
| SS-09 | Invocation badge + conflict dot | 5 | Edit conflict | Edit modal | R5.5, health ladder |
| SS-10 | Change source modal | 5 | repair | inline input; errors as toasts | R5.2, R5.3 |
| SS-11 | Review discovered skills | 3 | onboarding plan | select / variant | R3.1–R3.3, #46 |
| SS-12 | Project matrix cells (5 statuses + global) | 4 | reconcile | Sync Project | R4.1 |
| SS-13 | Project missing folder | 4 | disk | Update Path / Remove | R4 gap 3 |
| SS-14 | Delete confirm → kept (unchanged dialog) | 6 | ADR-0002 | Yes, Delete again | R6.3 |
| SS-15 | First-run empty + discovered banner | 3 | first use | Review & Import | R3 gap 8 |

### Omission review

| Potential state omitted | Why it is (not) applicable | Risk if wrong | Owner / gate |
|---|---|---|---|
| Persistent per-skill failure (acquisition, first sync, import) | Applicable — omitted today | Silent drift of library health (Principle 1, 5) | round 19 + backend decision (grill Q1) |
| `declined` / `cancelled` outcome kinds | Applicable — collapsed into failed | Red noise; cancelled Refresh reads as 15 failures | reportOutcome fold (grill Q3) |
| Offline | Applicable to Add/Refresh/Re-point; handled as typed network errors per skill | Low | — |
| Permission / ownership roles | Not applicable — single local operator | — | — |
| Concurrent mutation while overlay cancelled | Applicable via X2 — backend serialises (Mutation guard), frontend does not show "busy" | Operator starts a second action believing the first stopped | round 19 progress affordance |
| Reconcile skipped (`reconciled:false`) | Applicable, invisible | Stale matrix trusted | R4 gap 6 |
| Upstream "update available" | Not applicable — backend cannot produce it (#62, needs #59) | Prototypes rendered it; must not reappear as `stale` | spec § Bake-off |
| Undo | Applicable for unsync / unsync-all / detach (central copy remains) | Reversible actions carried with no undo | pattern decision P2 |

---

## Pattern decisions (hypotheses, with revisit triggers)

- **P1 — Where a report lives.** Alternatives: (a) toasts + history (today); (b) persistent last-run report sheet per
  action, opened from the toast; (c) skill-level health persisted on rows + issues banner/chip (#49). Recommendation:
  **(b) + (c)** — (b) answers "what did this run do", (c) answers "what is wrong with my library now"; toasts carry only
  the composite headline. Disconfirming: the operator reads the Notifications panel after every batch and never misses
  a failure (ask in the session). Revisit if the operator never opens (b).
- **P2 — Barrier for wide removals.** Alternatives: confirm with counts; typed intent; undo; disable (#52). For actions
  whose effect the central copy can reverse (unsync, unsync all, bulk unassign, detach), prefer **undo / disable** over
  confirmation (repeated confirms become automatic); for loss the app cannot reverse (Delete of an imported/detached
  skill, Edits lost on delete, import-with-removal) a **consequence-stating confirm** is warranted. Round 16 D2's "no
  confirm for bulk unassign" is consistent with this; "Uninstall from tool directories" at 68 targets is not.
- **P3 — Batch progress.** Alternatives: blocking modal (today), non-blocking title-bar progress (D8 passive affordance),
  per-row inline state. Recommendation: title-bar progress + per-row spinner, Cancel only where honoured. Disconfirming:
  concurrent-action bugs the guard cannot absorb (it can: entry points serialise).
- **P4 — Overwrite ask timing.** Per-skill mid-loop (today) vs one consolidated ask per operator action. Recommend
  consolidated; revisit if the operator wants to see each skill's collision in isolation.

---

## D8 reserved features → the flows they serve

| D8 feature | Serves flow(s) | Gap(s) it closes |
|---|---|---|
| #52 enable/disable + multi-select bulk (sync-to-tools, assign-to-project) | 6, 4, 1 | Unsync that survives auto-sync (X6, R6.1–R6.2); bulk assign from My Skills; reversible "Uninstall all" |
| #39 group-level Deploy all | 1, 4 | Deploying a just-added repo group; tailored groupings (PRODUCT success criterion) |
| Per-row / inspector project assignment | 4 | Assignment outside the project matrix (operator friction) |
| #49 issues banner + Issues chip | 2, 1, 3, 5, 6 | Persistent skill-level health (X1, R2.2); "Delete incomplete"; Edit conflicts |
| "Synced N/M" per-tool strip | 1, 2, 6 | Coverage incl. never-deployed / failed / declined / disabled (R1.1, SS-08) |
| Detail health ladder + copyable source | 5, 2 | Local source path (R5.1); Health/Upstream/Edits/Last synced with Retry failed |
| Sticky install summary in Add | 1 | Picker partial failure, consolidated overwrite ask (R1.2, R1.4) |
| #47 picker search | 1, 5 (Re-point picker), 3 | Finding candidates in 28-skill repos |
| #46 `SKILL.md`-gated scan | 3 | `broken-skill` importable (R3 gap 3) |
| Passive title-bar update affordance | 2 (and every batch) | Non-blocking progress, honest Cancel (X2, X3, R2.3) |
| Singleton fold + flat mode | (library browsing) | Not a flow gap found here; helps locate the skill a report names |
| ⌘K palette | all (re-entry) | Keyboard route to Refresh / Update / Change source / Edit for a named skill after reading a report (D5 keyboard-only gate) |

---

## Fixture gaps found (for ticket 04's owner; none blocks this review)

- Cancelled Refresh throws `CANCELLED` (silent) instead of returning an all-`Failed{Cancelled}` report as
  `core/refresh.rs` does — the fixture hides the misleading "0 refreshed, N failed" toast.
- Unwritable dirs affect writes only, so ADR-0002's kept row on **project removal** and on **unsync** of an unwritable
  shared dir are not reproducible (`release-train`'s locked path does reproduce it for Delete / unsync all).
- Re-assert does not produce `TARGET_EXISTS` for some occupied targets the scenario seeds (prisma-review → Cursor/Codex).
- Invocation-conflict data differs between the card ("upstream … User & model") and the Refresh report ("Model only").

---

## Open questions for the operator's grill (ticket 09)

1. **Persisted health.** Should a failed *first* sync, a failed Refresh acquisition and an incomplete Delete become
   stored skill-level state (new column/table) so they survive restart — or is session-only acceptable? This decides
   whether #49 and the health ladder have data to show.
2. **Unsync vs auto-sync.** Today unsync is undone by the next Refresh. Is #52's `disabled` the *only* unsync in 1.3.0
   (retiring "Uninstall from tool directories" as a toolbar button), or do both exist?
3. **Outcome kinds.** Accept `declined` (operator kept an existing folder) and `cancelled` as first-class outcomes in
   the fold, distinct from failed? And the composite headline ("12 refreshed · 5 failed · 4 skipped · 1 Edit") replacing
   single-headline precedence?
4. **Where does a run's report live** — a persistent last-run sheet (P1-b), the Notifications panel grouped by run, or
   only the issues surface (#49)?
5. **Overwrite ask**: one consolidated ask per action (P4) including Refresh re-assert and Onboarding import
   collisions with managed names — yes/no? Does the ask need a content diff?
6. **Import with auto-sync off** deletes Tool copies. Keep that policy, or make "remove originals" an explicit per-run
   choice in the review?
7. **Re-point identity**: should Re-point refuse or warn when the target's `SKILL.md` name differs from the skill's?
8. **Blocking overlay**: accept a non-blocking progress affordance for Refresh/sync/import in 1.3.0 (with the library
   browsable and mutations queued by the guard)?
9. **Cancel**: extend cancellation to sync, import and project resync, or remove Cancel where it cannot be honoured?
10. **Delete scope copy**: must the confirm list projects and Edits, and treat imported/detached skills (only copy) as a
    stronger confirm?
11. **Global-vs-project cell**: may a project assignment coexist with a global target for the same Tool (tooltip says
    yes, toggle says no, bulk does it)?
12. **Usability session**: will you do one 45-minute think-aloud on the fixture (`failures` + `first-run`) using the
    per-flow questions above before the round-19 spec is locked? It is the only way to turn these hypotheses into
    evidence.
