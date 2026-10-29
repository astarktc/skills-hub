# Handoff — 2026-10-29 · 1.3.0 shipped and smoked, 19a closed; next session opens **19b**

The queue is `.scratch/BACKLOG.md` (next free **#83**). Read it, then this note. Round 19's record: `round19/spec.md`
(D1–D33, ticket table, `## Closure — 2026-10-29 (19a)`), `round19/issues/01`–`29`, `round19/review/`. `round18/` is
archived.

## State of `main` (`50c3b12`, pushed, clean tree, = `origin/main`)

- **1.3.0 is released**: tag `v1.3.0` = `457085e`, Release published 2026-10-28, run ids on ticket 19 § Release run ids.
  Operator smoke passed on the main Mac 2026-10-29 (ticket 19 § Smoke outcome). **One smoke item is still open: item 8
  on the work Mac** (Refresh `impeccable` twice; `ls -l ~/.skillshub/impeccable/scripts/impeccable` → `-rwxr-xr-x`).
  Append the result to ticket 19's smoke table when Alex reports it.
- Tickets 01–19, 27, 28, 29 are `done — <sha>`. 20–26 (19b) are `ready-for-agent`.
- Last full gate: `457085e` tree — vitest 1043, cargo 836, App.css 1108 / 36 migrated; CI green on `main`.
- The 10-27 handoff is superseded by this one (its whole plan ran); delete it in the first 19b commit.

## 19b — what it is and the order

Tickets 20–26 (`round19/spec.md` § Tickets, phase 19b → 1.3.x): **20 ∥ 22 ∥ 23 → 21 → 24 → 25 → 26**.
Before building:

1. **Read 20's and 21's `## Comments` dated 2026-10-27** — binding think-aloud findings that change their Work.
2. **BACKLOG #71** ("not synced" vs Disabled, and Refresh's re-assert) **needs a `grilling` session with Alex** before
   19b builds anything near it (20's settings panes touch auto-sync's visibility; 21 touches import). Don't dispatch 20
   until #71 has a decision recorded on it.
3. **#78 (restrained colour pass)** — Alex raised it again on 10-29 ("very monochrome … light is weaker"). Offered three
   slots (pre-push / first in 19b / later); he chose to ship 1.3.0 first and gave no slot for 19b. **Ask where it goes**
   at the start of the session; if "first", open it as ticket 30 scoped to `src/styles/theme.css` + DESIGN.md's measured
   contrast table, Opus build + Astra audit, with one round of Alex's eyes on screenshots before the audit.
4. Small 1.3.1 candidates that could ride with 19b's first wave, all grep-verified: **#81** (`<title>` "Vite + React + TS";
   `autoCorrect="off"` on the `type="search"` inputs; Enter/↓ from search to the first result; one-result group label),
   **#82** (dead i18n keys), **#80** (toast stack over the header — compounds the toaster work 29 did).

## Operator decisions in force

- Spec and 19b tickets are pre-accepted; children are dispatched only on Alex's explicit go per wave.
- **Routing**, all on Pi, `runtimeMode: "full-access"`, `mode: "async"`, own `clientRequestId`:
  implementation Opus 5.5 `thinking: high`; audits Astra `thinking: high` (fix-then-pass, one bounded re-check;
  reviewer ≠ implementer); mechanical migrations only → GPT-6 Luna `high`. Confirm per session — it shifts with usage
  windows.
- The push is the release (`auto-tag.yml` → `release.yml`); a version bump push needs Alex's explicit "push".
- Pause around Alex's usage windows.

## Release-process facts learned on 10-27/28 (also on ticket 19)

- **The repo is a fork** (`qufei1993/skills-hub`). GitHub's idle-fork sweep disabled every workflow on 2026-10-14 with no
  repo-level signal (settings and the API both said "enabled"; dispatch answered 422). Alex re-enabled it from the Actions
  tab. **Recommended before the next release: Settings → General → "Leave fork network"** (irreversible, keeps
  everything). Not yet done — raise it.
- `auto-tag.yml` can't retro-fire once the bump push has landed; the manual equivalent is
  `git tag vX.Y.Z <sha> && git push origin vX.Y.Z` — a tag pushed with operator credentials triggers `release.yml` directly.
- Hosted **macOS runners on the Free plan** can take an hour to a night to acquire. `assemble-updater-json` now runs on
  `ubuntu-latest` (`457085e`); the two macOS *build* jobs still need them — push a release off-peak.
- `ci.yml` has no `workflow_dispatch`; only a push to `main` runs it.

## Live-app smoke procedure (new; worked, with two slips)

With Alex present and VoiceOver on, the orchestrator can drive the **release build** through macOS Accessibility
(System Events keystrokes + `AXFocusedUIElement` reads + `screencapture`), Alex reports what VoiceOver said. The release
app is a native WKWebView — no URL, no CDP; ego/agent-browser cannot reach it. Rules that made it safe:

- Accessibility and Screen Recording must be granted to the **T3 Code (Beta)** app (ad-hoc-signed Electron, bundle id
  `Electron`): remove + re-add under Accessibility after any T3 update; `swift` `AXIsProcessTrustedWithOptions` with
  prompt registers the right binary. Python ctypes segfaults — don't.
- **In one script: verify the focused control's `name` AND `enabled = false` (held), then press Enter.** Two slips came
  from breaking this: an AX `click` silently failed and the Enter toggled a *live* row switch (skill-grain Disable on 4
  Tools — reverted with Enable); a retry landed after the Refresh had ended (a real Disable — reverted with Undo/Enable).
- Pace Tabs at ≥ 0.3 s with VoiceOver on; `after`, `rd` are reserved words in AppleScript; screenshot from bash, not
  `do shell script`.
- A held window only exists during a long action: **Refresh all** (~90 s on 86 skills) is the one to use.
- Evidence: gitignored `.scratch/round19/evidence/19-smoke/` (notes + s1–s12 screenshots).

## Briefing rules that proved out (unchanged, abbreviated)

Make the first tool call immediately; AGENTS.md Workflow step 1 pre-approved ("state the approach in two lines and
continue"); explicit file ownership; never revert/stash/commit; `.scratch/` is tracked — children edit their ticket and
write under gitignored `evidence/<NN>/`, never BACKLOG, never archive; `resources.ts` keys appended at the end of `en`;
gate `npm run version:check && npm run check`; no CHANGELOG edits by children. Fixture proofs: `localhost:5175` (check
`lsof`), trusted CDP input via `.scratch/round19/evidence/13-fix/keys-cdp.ts` + `cdp.ts` (#69 — still gitignored),
every tracked-file save reloads the page, one browser session, bash calls < 120 s. Astra briefs avoid "TOCTOU",
"symlink", "adversarial", "dangling", "interposer". Children on the 5-hour limit are resumed on their own thread
with `t3_thread_send`; silent deaths are re-briefed fresh. Audits found real defects every time (27: label clicks; 28:
slash-ref encoding; 29: clean) — keep the discipline.

## Suggested skills for the next session

- `grilling` — BACKLOG #71 with Alex, before 19b builds.
- `impeccable` — if #78 opens as ticket 30 (colorize pass within ADR-0006's contrast table).
- `to-tickets` — if the #71 grill or #78 produce new tickets.
- `handoff` — end of session, to `.scratch/handoffs/`.
- `code-review` — optional standards/spec pass over a 19b wave before its audit.
- `web-design-guidelines` — for 19b audit briefs (20, 22, 23 are all surfaces).
- `agent-browser` — fixture proofs, with the CDP-driver caveat.
