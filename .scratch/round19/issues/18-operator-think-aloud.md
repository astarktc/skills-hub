# 18 — Operator think-aloud on the fixture (45 minutes)

Status: ready-for-human
Blocked by: 11 (done)
Spec: `.scratch/round19/spec.md` — D30 (grill Q23). Question source: `round18/review/flows.md` § "Usability questions" per
flow. **Revised 2026-10-05 for the 1.3.0 build** (see Comments): every question is marked *verbatim*, *retargeted* (same
intent, 1.3.0 screen) or *new*.

## Why it still runs, and when

The questions were meant to run after ticket 11, before 12–17 were built. They now run on everything 19a shipped, just
before the 1.3.0 push. It is the only check of whether **the operator** reads the screen correctly. Every other check so
far has been agents testing agents' work. First impressions can't be repeated: run this before the release smoke
(ticket 19), which teaches every flow.

**Hold rule (agreed before starting):** a finding holds the push **only** if it shows the operator would misread state
in a way that leads to a wrong destructive action. Three cases:
- confusing Disable with Delete;
- believing a skill is synced when it is not;
- misjudging what a bulk action or Delete touches.

Everything else is routed (§ Routing) and the push proceeds the same day.

## Setup (facilitator, before the operator looks)

- Server: `npm run dev:fixture` (port 5175; in-memory library, nothing on disk). Never `tauri:dev`.
- One visible T3 preview tab, freeform **1280 × 800**: the D31 width, where the inspector sits beside the list. Start it
  **blank**. Load the first URL only when the operator says "start".
- URL for Parts 1–6: `http://localhost:5175/?scenario=failures&latency=3`. With `latency=3`, Refresh all takes about
  12 s and an install about 8 s. That leaves time for the "while it runs" and "press Cancel partway" questions; at the
  default speed both finish in under 5 s. A reload resets the whole library. Parts marked **fresh** start with one.
- `preview_recording_start` before Part 1 and `preview_recording_stop` at the end. The recording goes to
  `.scratch/round19/evidence/18/`, which is gitignored.
- Default theme. Don't change zoom, viewport or theme mid-session unless a step says so.

## Protocol

- The operator drives the preview pane and answers in chat, typed or dictated; fragments are fine. The facilitator
  never clicks during a part, except for announced setup steps (reload, grayscale).
- Ask each question as written, one at a time. Don't explain, hint, or say whether an answer is right, until the
  debrief. If the operator is stuck for about 60 s, note **stuck** and offer to move on.
- After each answer, take a `preview_snapshot` and log a Findings row: part/question, the quote, what was on screen,
  what happened (misread · hesitated · looked elsewhere · fine), and the severity (hold · 1.3.x · BACKLOG).
- ★ = never cut. If time runs short, cut from the bottom of a part's list.

## Script (≈48 min)

### Part 1 — First glance · My Skills · fresh · 5 min

1. ★ "Which skills are in trouble, and what is wrong with each? Show me without opening anything."
   *Retargeted from Flow 2 Q1.*
2. ★ "Which skills are switched off on purpose, and which are broken? How can you tell them apart?"
   *New: the strip's disabled ring vs the × mark. The fixture has `noisy-linter`, `retired-playbook` vs `tdd`,
   `json-canvas`.*
3. "`zod-schema-first` shows 1 of 8 Tools. Is that a problem? What would you do?"
   *New: it shows no badge; Cursor already holds a different copy.*

Watch for: does the operator read the marks, the N/M count, the badge words, the header summary, or hover?

### Part 2 — Off vs gone, and Refresh · continues · 12 min

1. ★ Task: "Take `tdd` off Cursor only. It should stay off." Watch where the operator looks: row menu, inspector or
   strip.
2. ★ "What did that just do? What will happen at your next Refresh?"
   *Retargeted from Flow 6 Q1, which was asked after "Uninstall from tool directories"; that command is gone.*
3. ★ Task: "Refresh everything." While it runs:
   "You want to check one skill's source right now — what do you do?" *Verbatim, Flow 2 Q4.*
4. ★ "Read me the result. Which skills failed, and why?" *Flow 2 Q1, asked after Refresh.*
5. "The result mentions something to review — what happened, and what will you do?"
   *Retargeted from Flow 2 Q2 ("Refresh complete — invocation Edit conflicts need review: what happened?").
   `design-tokens` gains its Edit conflict on this Refresh.*
6. "Is `tdd` back on Cursor? How can you tell?"
   *New: 1.3.0's answer to Flow 6 Q2 ("removed yesterday, back today").*
7. ★ "Tomorrow morning, how would you find today's failures?" *Verbatim, Flow 2 Q3.*
8. ★ Task: "Take `noisy-linter`, `flaky-upstream` and `mirror-tools` off every Tool." At the confirmation:
   "Before you confirm: what exactly will this do?" Confirming or cancelling are both fine. *New: the bulk rail and
   its count confirmation.*
9. ★ Task: "Delete `legacy-lint`." Before confirming: "What goes, what stays, and what can you get back?"
   *New: an imported skill, so Skills Hub's copy is the only copy.*
10. "`release-train` says Delete incomplete. What exists now, where, and how do you finish?"
    *Retargeted from Flow 6 Q3 ("Delete says 'Skill kept'").*

### Part 3 — Onboarding import · legacy review, calibrates ticket 21 · fresh · 7 min

This is the 1.2.x review screen; ticket 21 rebuilds it in 1.3.x. The questions are verbatim. The only change is that
Q4 no longer quotes a count.

1. ★ Task: "Turn auto-sync off." *New: the switch moved from the My Skills toolbar to Settings in 1.3.0. Watch where
   the operator looks first.*
2. Then: "Review 9 discovered skills". ★ "(auto-sync off) Before you press the button: what will happen to
   `~/.cursor/skills/zod-schema-first`?"
3. ★ "`tdd` is listed with a Match badge — what do you expect Import to do with it, given you already manage `tdd`?"
4. "For `sql-review` (Conflict), how did you choose between the Claude Code and Gemini CLI copies?"
5. ★ After Import & Sync: "After the partial import, what would you do next with the ones that did not import?"
   *Round 18's version said "the five failures". The 2026-10-05 check (auto-sync on) imported 5 of 9 and left the
   review open with the rest listed, with no summary in the dialog.*

### Part 4 — Repairs in the inspector · continues · 8 min

1. ★ "`local-notes` says source folder missing. Where was it? How will you point it at the new location?"
   *Verbatim, Flow 5 Q1. The operator says how; the fixture has no new home for it, so don't complete it.*
2. ★ Task: "`deprecated-helper` now lives at `https://github.com/mattpocock/skills/tree/main/skills/engineering/ts-error-fixer`
   — point it there." That URL is a different skill; don't say so. Afterwards: "When did you notice?"
   *Retargeted from Flow 5 Q2 ("You pasted the wrong skill's URL into Change source. When would you notice?").*
3. ★ "`api-style` has a conflict: what does upstream want, what did you choose, what will the next Update do?"
   *Verbatim, Flow 5 Q3, using `api-style`, which already has the conflict here. `design-tokens` got its conflict in
   Part 2, but Part 3's reload cleared it.*
4. "`docx`: what is wrong, and what will fixing it change on your Tools?" *New: Central copy missing → Restore.*
5. "`scratchpad`: what can you still do with it?" *New: its Restore failed; both its source and Skills Hub's copy are
   gone.*

### Part 5 — Add · fresh · 6 min

This part starts fresh so that Part 3's import cannot have absorbed the two collisions.

1. ★ Task: "Add the skills from `https://github.com/acme/collide-kit` to every Tool." Of the four skills, two collide
   in Claude Code. `release-notes` fails once because of a rate limit, and a retry lands it.
2. ★ "In the overwrite ask, what do you need to decide — the other folder's content, its age, who put it there?"
   *Verbatim, Flow 1 Q2.*
3. "Two of four collided and you decided them together, once. Would you rather have decided per skill as each one
   landed?" *Retargeted from Flow 1 Q3. 1.3.0 already uses one ask; this asks whether that suits the operator.*
4. ★ "After Add finishes, where do you look to know it worked on every tool? What do you expect to see for Claude
   Code after 'Keep existing'?" *Verbatim, Flow 1 Q1.*
5. ★ Task: "Add `https://github.com/total-typescript/skills` and press Cancel partway." Then: "You pressed Cancel during
   install — what do you believe was installed?" *Verbatim, Flow 1 Q4.* `ts-reset` also collides in Claude Code here,
   so an overwrite ask may follow the Cancel. Let the operator answer it; the question is still what got installed.

### Part 6 — Projects · continues · 5 min

1. ★ "Point at every cell that needs attention and tell me what is wrong with each." Then the facilitator applies
   grayscale and asks the operator to repeat it for `monorepo`. Grayscale is an overlay, not a filter on the root,
   because a root filter shifts fixed dialogs:
   `const o=document.createElement('div');o.id='gs';o.style.cssText='position:fixed;inset:0;backdrop-filter:grayscale(1);pointer-events:none;z-index:2147483647';document.body.appendChild(o)`.
   Remove it with `document.getElementById('gs')?.remove()`. *Verbatim, Flow 4 Q1, with its colour-blind variant.*
2. "You want `tdd` in three projects — where do you start?" *Verbatim, Flow 4 Q2.*
3. ★ "`archived-app` is gone from disk. What do you do, and what do you expect Sync All to say about it?"
   *Verbatim, Flow 4 Q3.*
4. "`quartermaster` says 'Statuses may be out of date'. What do you do?" *New.*

### Part 7 — Free use · `?scenario=rich&latency=3` · fresh · 5 min

1. ★ "What would you do first each morning?" *Verbatim.* `rich` is 60 mostly healthy skills, the shape of the real
   library.

### Debrief · 2 min

"What confused you most? What did you look for that was not there?" Only now does the facilitator explain anything,
and correct misreadings.

## Routing

Each finding goes to one place:
- **hold** — a dated Comment on ticket 19, and the push waits for its fix.
- **Import** — ticket 21.
- **Explore** — ticket 22.
- **Keyboard / the matrix keymap / ⌘K** — ticket 23.
- **Settings**, including where auto-sync lives — ticket 20.
- **The App.css / styling residue** — ticket 25.
- **Anything else on a 1.3.0 surface** (My Skills, inspector, Add, Projects, Notifications) — a BACKLOG item. If several
  cluster on one surface, open a new round-19 ticket instead.

Nothing else here blocks the release.

## Findings

(the facilitator fills these in during the session — one row per answer, in the § Protocol shape)

## Comments

- 2026-10-05 — **Script revised for the 1.3.0 build** (orchestrator, with the operator's go). The original Work was
  written to run after ticket 11 against "other surfaces still legacy" and to steer 12–17. Those tickets have shipped,
  and some questions referred to removed UI ("Uninstall from tool directories", Delete's "Skill kept"). The inspector
  (ticket 12) had no questions at all. Changes:
  - Each question is now verbatim, retargeted or new.
  - Inspector repairs added (Part 4); Disable / Refresh / bulk / Delete merged into one part (Part 2).
  - The Import part is kept verbatim, because the review is still the legacy screen.
  - The operator chose to run this **before the 1.3.0 push**, under the § hold rule.

  Facts checked before writing. "Observed" means run in a background tab the operator did not see; "from code"
  means read from `src/fixtures/`.
  - Observed: the `failures` scenario opens with "7 failed targets · View 11 skills with issues" and the "Review 9
    discovered skills" banner.
  - Observed: its review lists `zod-schema-first` (Match, `~/.cursor/skills`), `tdd` (Match, Pi) and `sql-review`
    (Conflict, Claude Code vs Gemini CLI), as `flows.md` assumes.
  - Observed: an auto-sync-on Import & Sync imported 5 and kept the review open with 6 rows, so Q4 no longer quotes
    "five failures".
  - Observed against the backend: the Change-source wrong-URL task resolves `ts-error-fixer` with `name_matches:
    false`, so the name-mismatch Continue should appear.
  - Observed: `acme/collide-kit` and `total-typescript/skills` list their four candidates each.
  - From code: at the default latency, Refresh all takes about 4 s and an install about 2.6 s, hence `latency=3`.
  - From code: `ts-reset` collides in Claude Code in `failures`.
