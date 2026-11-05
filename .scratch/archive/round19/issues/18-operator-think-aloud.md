# 18 — Operator think-aloud on the fixture (45 minutes)

Status: done — 9aeec8b
Blocked by: 11 (done)
Spec: `.scratch/round19/spec.md` — D30 (grill Q23). Question source: `archive/round18/review/flows.md` § "Usability questions" per
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

Session 2026-10-27, from 10:35 CDT. Operator: Alex. Facilitator: orchestrator. `?scenario=failures&latency=3`,
1280 × 800, dark theme, recorded (`preview_recording_start` 15:35:37Z). The operator started `npm run dev:fixture`
himself before the session.

- **P1·Q1** (★, which skills are in trouble). *"It looks like seven skills are in trouble. One of them is missing a
  central copy. A few others failed to deploy to their target location(s). And then for the remainder, one of them was
  unable to be pointed to a new location. Another one failed to update from its upstream source, and the last one must
  have had a customized edit, and the upstream was updated, therefore there's now an edit conflict between my custom
  version and the upstream."*
  - On screen: list unscrolled (`scrollTop` 0). Header: "7 failed targets · View 11 skills with issues". Seven
    badge rows were above the fold: docx, deprecated-helper, pr-review, tdd, using-git-worktrees, api-style,
    json-canvas. Four issue rows were below it: release-train (Delete incomplete), local-notes (Source folder
    missing), scratchpad (Restore failed), shell-aliases (Import incomplete).
  - What happened: **every badge he read, he read correctly**: Central copy missing, targets failed, Change source
    failed, Update failed, and the Edit conflict's cause. **He didn't scroll, and missed 4 of the 11.** "Seven"
    matches both the seven visible badge rows and the header's "7 failed targets". Ask in the debrief which one it
    came from. If the header's target count was read as a skill count, the summary's pairing of two different units
    is the finding.
  - Severity: 1.3.x (My Skills → BACKLOG). The list is grouped by source, not sorted by severity, so "problems first"
    depends on the summary line and the Issues chip. He didn't use either.

- **P1·Q2** (★, off on purpose vs broken). *"My bad, I guess I forgot to scroll down to see the rest. I'm guessing that
  retired-playbook is the only one switched off on purpose because of the toggle indicating as such, which means we
  chose not to deploy it to any tool or project. It depends on what you mean by broken, but obviously there are the
  failed targets and stale items that we previously looked at in the last question, but then on a tool-by-tool basis,
  looking toward the right side The red X's tell me that there was an issue with deployment to those locations, and
  for the ones that said two or three targets failed, obviously I see that many X's while I see dots indicating some
  successful ones. But the first one, which is docx, every single deployed harness has an X because all of them do
  not have the central copy that they would be symlinking to at the target location. Did I get that right?"*
  - On screen: list back at `scrollTop` 0 when sampled. **That was the facilitator's reload** (see "Facilitator
    incident"), not the operator: he had scrolled, since he named retired-playbook (top 895). Two skills are off on purpose:
    - retired-playbook: switch off, every mark "disabled — the skill is off everywhere";
    - noisy-linter: switch on, Cursor and Codex marks "disabled" (the slashed ring).

    The Disabled chip reads 2.
  - What happened:
    - The skill-grain Disable was read correctly from the switch, including what it means ("not deploy it to any
      tool or project").
    - **The pair-grain Disable on noisy-linter was not named.** It's unclear whether he didn't see it or saw the
      rings and didn't read them as deliberate. Ask in the debrief.
    - × = failed and dot = synced were read correctly. The N-targets-failed badge was matched to its × count.
    - docx's all-× strip was explained correctly (the links point at a central copy that is gone).
    - "Stale" was named in passing.
    - The Disabled chip wasn't used.
  - Severity: 1.3.x (My Skills → BACKLOG) if the debrief shows the slashed ring didn't register as "off on purpose".
    Not a hold: he didn't mistake broken for synced, or Disable for Delete.
  - He asked "Did I get that right?" The facilitator deferred it to the debrief.

- **P1·Q3** (`zod-schema-first` 1 of 8). *"That would not be a problem because clearly that's the only tool that we
  synced to."*
  - On screen: zod-schema-first is synced to Claude Code only. The other marks are hollow ("not synced"), and there is
    no badge. Cursor already holds a different copy (the fixture's foreign folder).
  - What happened: he reads **"not synced" as a deliberate choice**. In the system it isn't one. Only Disable is the
    durable "not on this Tool" (D8). With auto-sync on (the fixture's default), Refresh's re-assert
    (`core/refresh.rs::reassert_auto_sync_unlocked`) syncs every managed skill to every Tool in the selection. So the
    next Refresh all puts zod-schema-first on the other 6 detected Tools, and raises the overwrite ask for Cursor. The
    hollow mark doesn't tell a fresh "nothing here yet" apart from an operator's intent. The system has no unrecorded
    intent, so the screen can't show one.
  - Severity: **to be decided after P2·Q3.** Refresh all runs there and will deploy it live; watch his reaction. Not a
    hold as worded: deploying adds copies, and the one collision is asked first. But the model gap it shows is what D8
    was meant to close.

- **P2·Q1** (★ task: take `tdd` off Cursor only, and keep it off). *"I looked for TDD in the list, clicked on its line,
  the right sidebar opened up, and I clicked on disable next to cursor."*
  - Result: `#/skills/skill-001-tdd`; tdd's Cursor row is `disabled` and the skill isn't disabled. The other rows are
    unchanged (Claude Code, Codex, Pi, Gemini synced; Amp and Kimi error).
  - What happened: **done directly and correctly.** Path: list → row → inspector → Sync targets → Disable beside
    Cursor. No hesitation reported, and he didn't try the row menu or the strip.
  - Severity: none (a pass).

- **P2·Q2** (★ what did that do / next Refresh). *"It removed the skill from the cursor tool directory's skill folder.
  At my next refresh, the skill should be updated from its upstream source and continue to no longer be deployed to
  Cursor."*
  - What happened: **correct on both counts.** The artifact was removed, and Refresh updates the skill but skips the
    disabled pair (`skipped: disabled`). The 1.2.x worry (Flow 6 Q1, "Uninstall … what will happen at your next
    Refresh?") is answered right: Disable is understood as durable.
  - Severity: none (a pass).

- **Facilitator incident (found 15:48Z).** Every save of the tracked ticket file made the fixture page do a **full
  reload**, which resets the in-memory library. `@tailwindcss/vite` scans tracked non-module files, including
  `.scratch/`, and reloads the page when one changes. Evidence: the page's `performance.timeOrigin` is 15:46:35.506Z
  with navigation type `reload`, and the ticket file's mtime is 10:46:35 CDT (the P2·Q2 append).
  - Effects:
    - The P2·Q1 Disable of tdd/Cursor (verified `disabled` before the append) was wiped before Refresh all. tdd's
      Cursor row reads `synced` again with its *seed* `synced_at`, not a fresh one.
    - P1·Q2's note "list back at scrollTop 0" was the reload, not the operator.
  - Fix: live notes go to this gitignored file (Tailwind skips gitignored paths). The ticket isn't touched until
    the session ends.
  - Process lesson for any fixture-driven session or audit: never write tracked files under the repo while a
    stateful fixture page is in use.
- **P2·Q3** (★ Refresh all; check a source while it runs). *"I hit refresh all and I could see that everything started
  to refresh with that indicator telling me which ones are being checked and downloaded. I clicked on front-end design
  to see its source, and in the sidebar I saw that it was from Anthropic/skills. Along the way, a pop-up appeared,
  telling me that three folders already exist with different content, and I could choose to either keep my current
  copy or overwrite them with the new version that came from the online source."*
  - On screen: header progress "Refreshing 19/19 · Waiting for your confirmation…". The overwrite ask: "These 3
    folders already exist with different content. Overwrite them with the Skills Hub copies?" Its rows:
    - zod-schema-first → Cursor;
    - prisma-review → Cursor;
    - prisma-review → Codex.

    Each row reads "Modified 3d ago · differs from the skill", with Keep / Overwrite. Inspector at
    `#/skills/skill-003-frontend-design`.
  - What happened:
    - The source check during the run works: row → inspector → "anthropics/skills", with no refusal.
    - He noticed the header progress and read it as per-skill status.
    - He described the ask correctly in outline. But he framed the folders as **"my current copy" vs "the new version
      from the online source"**. These folders are copies Skills Hub never deployed. The ask exists only because the
      re-assert is putting zod-schema-first and prisma-review on Tools they weren't on.
    - **He didn't connect it to P1·Q3**, where he said zod-schema-first was on Claude Code only on purpose. Here
      Refresh is trying to deploy it to Cursor (and, silently, to Codex, Pi and Gemini, now `synced`).
  - Severity: candidate for the D8 model gap with P1·Q3; decide after he answers the ask and reads the result. **The
    tdd/Cursor part of this Refresh is invalid** (facilitator incident). It will be redone.
- **P2·Q4** (answer the ask; read the result). Long answer; quotes trimmed.
  - On the ask: *"maybe this pop-up is actually saying that there could either be Skills Hub managed content in the
    existing folders that was custom or edited, and the upstream is different … Or option two might be that there are
    unmanaged copies or skills with the same name in those target locations … the situation is ambiguous … I've also
    had situations where some third-party tool on my machine did its own deploy of a skill to a location that causes
    such conflicts. So, for the sake of this exercise, I'm going to choose to overwrite all three … it gives me no way
    to see them whatsoever, and I can't even see the diff."*
    - Fact: all three are folders **Skills Hub never deployed**. A re-assert collision means no target row exists for
      the pair, so ownership is known in the backend. The ask doesn't say so. It shows path, "Modified 3d ago · differs
      from the skill" and Keep / Overwrite, with no content view, diff or Reveal.
    - This answers round 18's Flow 1 Q2 ("what do you need to decide?"): **who put it there, and its content as a diff
      against the skill.**
    - He chose **Overwrite ×3 without being able to judge**, which destroyed three unmanaged folders: zod-schema-first
      and prisma-review in Cursor, and prisma-review in Codex, now `synced`.
    - Severity: **hold candidate under the hold rule's letter** (a destructive action taken on state he couldn't read).
      But the ask has shipped unchanged since 1.2.17, so holding 1.3.0 removes no exposure. Put to the operator at the
      debrief. Fix shape: the ask says "not deployed by Skills Hub" (or names the managed state), with a content view
      or diff and Reveal in Finder per row. → BACKLOG, high.
  - After the ask: *"I clicked on overwrite, the window closed, and I already forgot which skills were listed in the
    window in the first place."*
    - The answered rows go nowhere he can find. The run report counts failures; overwritten pairs settle as plain
      `synced` and aren't itemised. Same family as BACKLOG #64 (3): an answered collision is dropped from the report.
    - Severity: 1.3.x → BACKLOG (an audit line, "Overwrote 3 folders: …", in the run).
  - On the library after Refresh: *"suddenly there are a lot more skills that have failed than before … twenty-three
    failed targets, … twenty skills with issues."*
    - Fact: before, 7 failed targets and 11 skills with issues. After, 23 and 20. **16 new failed targets, all on Amp
      and Kimi** (11 each, plus using-git-worktrees/Cursor). The re-assert deployed every skill onto every selected Tool,
      and Amp and Kimi share the unwritable `~/.config/agents/skills`.
    - Refresh, which he understands as "update from upstream" (P2·Q2), also **deploys everywhere** (auto-sync), and
      here made the library look much worse.
    - The rows read **"Could not update the synced copy: zod-schema-first -> Amp"**. There was no synced copy: this was
      a first deploy by the re-assert. The copy misdescribes it.
    - Run 1 does carry "prisma-review also deployed to Claude Code, Pi, Gemini CLI" (ticket 14's announced re-assert
      additions), but not as a headline.
    - The "import incomplete / source folder missing / update failed" rows he lists as new were there from the start,
      below the fold (P1·Q1).
    - Severity: joins P1·Q3 as **one model-gap finding: Refresh = update + deploy to every selected Tool.** The screen
      never says it. 1.3.x → BACKLOG, high. Candidates: the run headline counts re-assert deploys as their own kind
      ("deployed to 8 more Tools"); re-assert failures get their own wording; consider whether Refresh should re-assert
      at all, or ask first.
  - Toast: *"I also just happened to have clicked on refresh all again, and for a short moment I saw the toast … it was
    so short-lived I didn't even get the opportunity to click on the report button."*
    - A second Refresh ran (run 2). The failure tone should be `Infinity` (`useStatusReporter` `TOAST_DURATION_MS`), so
      a fast vanish is unexpected. **Reproduce after the session** (failures, latency=3, two Refresh alls; does opening
      the panel or a second run dismiss it?).
    - Severity: 1.3.x until reproduced.
  - On the report: *"13 refreshed, 5 failed, 22 targets failed … I'm assuming 5 failed means upstream failure … targets
    failed would be that they weren't able to copy into the location … It also says four were skipped. … API rate limit
    reached. It will reset in 12 minutes, and I can manually retry it … another one was no longer found on GitHub, so
    the upstream might have been deleted or moved."*
    - He found the Notifications panel on his own, after the toast. **The count line was read correctly** (skill
      failures vs target failures; "mildly ambiguous, but not really"), and so were the per-item causes and repairs.
    - Panel facts: run 2 (top) is "13 refreshed · 5 failed · 22 targets failed · 4 skipped" (31 items); run 1 is "… 14
      targets failed · 4 skipped · 1 Edit to review" (24 items).
    - Severity: none (a pass), apart from the toast.
- **P2·Q5** (the "to review" item). *"Once I opened up that first one, because it was collapsed by default, I had to
  scroll all the way down and I saw that it was design-tokens invocation edit conflict. The upstream changed its
  invocation mode to model only, and my override is kept, which sounds like I don't need to do anything unless I wanted
  to change it to match the upstream. I'm also assuming that the system must have tracked what the upstream's previous
  invocation mode was, and that it was neither model only nor user only … Therefore … the system tracks the diff of the
  upstreams when they update."*
  - What happened:
    - **Correct**: the Edit is kept, nothing is required, and he inferred the recorded base (user-and-model) correctly.
    - Finding: the run's headline says "1 Edit to review", but the item sits **last** in a run that is **collapsed by
      default**. He had to expand it and scroll past 20+ target failures.
  - Severity: 1.3.x → BACKLOG (low). Within a run, order "to review" items (and skill-level failures) before
    target-level rows, or link the headline's "1 Edit to review" to its item.
- **P2·Q6** (★ redo: tdd/Cursor disabled again, Refresh all a third time; is tdd back on Cursor?). *"It is not back on
  cursor, and I can tell because of two ways. One, if I open up the skill, I can see that it still shows that it is
  disabled … only because I opened it up and saw the cursor was first in the list do I know that in the line in the
  table with the eight little dots that the first one with the crossed-out surface circle is the cursor one."*
  - Verified: tdd is `cursor:disabled` after the third Refresh, with no page reload since 15:46:35. The inspector shows
    Health "Disabled on 1 Tool · On Cursor · Enable", and Sync targets "Cursor · link · Disabled · Enable".
  - What happened:
    - **Pass: Disable survives Refresh**, read correctly from the inspector and the strip.
    - Finding: **the strip's marks are positional and unlabelled.** He could only tell which mark was Cursor by matching
      the order to the inspector's list. Hover titles exist, but he didn't use them.
  - Severity: 1.3.x → BACKLOG (low): a Tool legend or initials over the strip column, or the list header naming the
    order.
  - The redo was announced after the facilitator incident, so a little was primed: he knew the earlier Disable had been
    reset by the reload, not by the app.
- **P2·Q6, also volunteered (Windsurf, and Disabled vs not synced).** *"I also saw that WindSurf is in the list, but
  not detected and therefore not synced. When I click on sync, it says sync failed. I'm assuming that's because … we
  actually have some logic that prevents it from syncing to a tool that is deterministically found to not be installed.
  I'm assuming that's the difference between something that is merely not synced versus disabled, because when I
  disable something, there is never again an option for it to say sync … disabled and not synced is kind of the same
  thing and is a little bit redundant outside of that context … the sync failure is easily confirmed by me opening up
  the report in the toast notification that does in fact say that tool is uninstalled."*
  - Finding A: **the inspector offers "Sync" on a Tool it labels "(not detected)"**, and it fails (`TOOL_NOT_INSTALLED`).
    A dead-end control. It should be refused in place with its reason ("Windsurf isn't installed — remove it from
    Configure Tools"), per D21's refused-control pattern. Severity: 1.3.x → BACKLOG.
  - Finding B, **the strongest so far, third confirmation after P1·Q3 and P2·Q4**: he concludes **"disabled and not
    synced is kind of the same thing … redundant"**. He reached this right after watching Refresh deploy
    zod-schema-first and prisma-review to five more Tools each. In the system, "not synced" is not durable: with
    auto-sync on, the next Refresh's re-assert (and any first sync) puts the skill on every selected Tool. Only Disable
    stays.
    - Consequence on the real library: he would leave a skill "not synced" on a harness he doesn't want it in, and the
      next Refresh would deploy it there. That isn't data loss (additive, and a collision is asked first), so it isn't
      a hold under the rule as worded. But it is the D8 premise ("removal is Disable, and it lasts") not reaching him.
    - Severity: **put to the operator at the debrief.** Candidate fixes: say in the strip/inspector that "Not synced"
      means "will be deployed at the next Refresh (auto-sync on)"; name re-assert deploys in the run headline (see
      P2·Q4); or reconsider the re-assert so that "not synced" stays put until asked.
- **P2·Q7** (★ tomorrow morning). *"I'm guessing I would just go back to the notifications menu and see the history."*
  - Fact: Notifications history is session memory only (`useNotificationHistory`, no persistence; BACKLOG "Dropped by
    name": #21, "per-row `last_error` is the durable record"). After a quit and relaunch the panel is empty. What
    survives is each skill's health and target state: row badges, strip marks, the summary line and the Issues chip.
  - What happened: **he'd look somewhere that is empty after a restart.** If the app stays open overnight, he's right.
  - Severity: 1.3.x → BACKLOG, and it **reopens the #21 decision with operator evidence.** Either persist the last N
    runs, or have the empty panel point at "View N skills with issues" after a relaunch.
- **P2·Q8** (★ task: take noisy-linter, flaky-upstream and mirror-tools off every Tool). *"I found those skills, and since
  it was the most convenient way to take them off of every tool, I just clicked on that deploy toggle to turn the skill
  off everywhere, and there was no confirmation. It just went ahead and did it, which I'm fine with … However, clicking
  on the toggle to turn it back on did pop up a thing that said, 'Enable flaky upstream and sync it to two tools
  again,' and then I could either say enable or not now."*
  - What happened:
    - He used **three row switches, not selection plus the bulk rail**, so the bulk rail and its count confirmation
      weren't exercised. The switch is the discoverable "off everywhere".
    - Off without a confirmation: he's explicitly fine with it. On asks inline ("Enable … and sync it to 2 Tools
      again"), which was understood. He re-enabled flaky-upstream ("Enabled and synced.").
  - **State check (16:11Z): mirror-tools was never disabled.** Its switch is on and Claude Code is synced with its
    *seed* `synced_at`, so no Disable/Enable cycle ever ran on it. Possible causes: the click was refused because the
    previous switch's Disable was still running (one mutation at a time; about 600 ms at latency=3), or he missed the
    row. Probe next with a neutral state question.
- **P2·Q8 follow-up** (state of the three, no clicks). *"Due to the fact that I then decided to click on the three
  checkboxes on the side, look around, and eventually saw at the bottom of the screen that is where the bulk options
  live. I did click on disable all tools and confirm, so therefore … the current state they are in is disabled
  everywhere."*
  - Verified (16:12Z): all three are disabled at skill grain, every row `disabled`. **Correct.**
  - What happened:
    - **He found the bulk rail himself, after "looking around"**: it's at the bottom of the screen, which isn't where
      he looked first. He used checkboxes, then the rail's "Disable on all Tools", then confirmed. The count
      confirmation was answered without the facilitator's stop-and-ask, so ask him what it said (from memory) next.
    - **Open: in his first switch pass, mirror-tools never turned off.** It still had its seed `synced_at`, and he had
      reported all three off. The bulk action then masked it. Ask in the debrief whether he flipped mirror-tools'
      switch. If he did, the refusal (one mutation at a time) went unnoticed, and **that is a misread state: he believed
      a skill was off when it was on.**
  - Severity: bulk rail placement, 1.3.x → BACKLOG (low). The mirror-tools question is **pending the debrief**: a hold
    candidate if a refused switch can be mistaken for a completed Disable.
- **P2·Q8 recall** (what the bulk confirmation said). He had forgotten, so he re-ran Enable and then Disable to read
  them.
  - **Enable confirmation** (`resources.ts` ~:994, `bodyNoAssignments`: "Enable {{skills}} — {{targets}} on {{tools}} sync
    again, a first sync each."), rendered "Enable 3 skills — 10 Sync targets on 7 Tools sync again, a first sync
    each". *"That phrasing was so confusing at first. Actually, that is still confusing. I have no clue what the hell
    that thing is saying."* After it ran, he decoded it correctly: 7 = the union of Tools, 10 = 7 + 1 + 2 pairs.
    - Finding: **the count sentence is unparseable without hindsight.** Problems: noun-stacked ("10 Sync targets on 7
      Tools sync again"), product jargon ("a first sync each"), and two different units in one clause.
    - Severity: 1.3.x copy → BACKLOG. Suggested shape: "Turn on 3 skills. They'll be synced to 7 Tools again (10
      copies in all)."
  - **Disable confirmation** (~:985, `bodyNoAssignments`), rendered "Disable 3 skills on 5 Tools — 8 Sync targets are
    removed and kept disabled; no project assignment changes. Refresh will not re-add them." He read it correctly:
    - 5 because two of noisy-linter's rows (Amp/Kimi) had failed and hold nothing;
    - "kept disabled";
    - "Refresh will not re-add them".
  - **Misread: *"But then again, if a project specifically has an assignment, it will still receive an update."*** He
    takes "no project assignment changes" to mean that a skill-grain Disable leaves project assignments live. In fact
    these three skills have no assignments, so there was nothing to change. A skill-grain Disable *does* disable every
    assignment, and the non-zero copy (~:984) says so with a count.
    - Not a hold: when assignments exist the sentence states the count, so the misjudgment can't lead to an
      unexpected removal.
    - Severity: 1.3.x copy → BACKLOG. Zero case: "None of these skills is assigned to a project", not "no project
      assignment changes".
  - The P2·Q8 stop-before-confirm question was answered by this re-run; on the second reading the confirmation was
    read accurately.
- **P2·Q9** (★ Delete legacy-lint: what goes, what stays, what can you get back). *"It's going to be removed from Cursor
  and Cloud Code. It's not in any project, and the copy within Skill Hub will be fully deleted and not recoverable."*
  ("Cloud Code" is dictation for Claude Code.)
  - What happened: **correct and complete.** Both Tools, no projects, and the only-copy warning (an imported skill)
    understood as unrecoverable.
  - Severity: none (a pass). D16's Delete dialog works.
- **P2·Q10** (release-train "Delete incomplete": what exists, where, how to finish). *"I can currently see that it is
  synced to Cursor, Cloud Code, Codecs Pi, and Gemini, with failed syncs to AMP and Kimmy Code. The delete itself says
  that permission was denied, and there's a button that says 'Finish Removing.' … I don't know whether or not it will
  fail again because it sounds like it's an OS-level permission thing. So I then clicked on Finish Removing, and it says
  that it removed from Codex, Pi, Cursor, Gemini, and Claude code. Yet, right below that, it says that it is still in
  Claude code and still in agents/skills-monorepo. There's an option to retry or reveal in Finder, but clearly the issue
  is not going to resolve itself … I have no way of knowing what that permission issue is in the first place, and I
  don't know how to solve it from here."*
  - What happened:
    - The state was read correctly from the strip, and the repair (Finish removing) was found from the health row.
    - **Finding A: the report says "removed from … Claude Code" and "still in Claude Code" together.** Most likely a
      global row and a project row (monorepo's Claude Code assignment) without their scope named. Verify the strings
      after the session; the human had the tab, so the facilitator couldn't read it.
    - **Finding B: "Permission denied" comes with no explanation and no next step.** Retry and Reveal in Finder are
      offered, but nothing says what to check (ownership, read-only, a process holding it) or that Retry will fail the
      same way.
    - **Finding C, from the facts:** before the session's Refreshes, release-train was on Claude Code, Codex and Pi
      only. **Refresh's re-assert deployed a skill whose Delete was incomplete to Cursor and Gemini, and tried Amp and
      Kimi.** The operator had asked for it to be gone, and Refresh spread it.
  - Severity:
    - A and B: 1.3.x → BACKLOG. Scope labels in the removal report; plain-language guidance for a permission failure.
    - C: BACKLOG, high, joined to the re-assert model gap (P1·Q3 / P2·Q4 / P2·Q6). Re-assert should skip a skill
      carrying `delete_incomplete` health, and arguably any skill with a pending removal.
    - Not holds: nothing was destroyed against his intent.
- **Part 3 setup.** The operator reloaded the page himself (⌘R) for a fresh `failures` library; he had control of the
  tab. Option **B** was chosen at 12:46: ★ questions only from here (P3 Q4 sql-review, P4 Q4–Q5, P5 Q3, P6 Q2 and Q4
  are skipped).
- **P3·Q1** (★ task: turn auto-sync off). *"I looked in the header first; I didn't see anything. I then clicked on
  Configure Tools because I thought maybe it was moved there … Then I looked toward the bottom and ended up clicking on
  settings … And lo and behold, there it is. The only reason I looked near the top is because our old version had it
  there, but it does make sense that it could be in settings instead and not clutter up the main UI."*
  - What happened: found on the third try (header → Configure Tools → Settings). The old location was the anchor, and he
    agrees with the move.
  - Severity: none for the move itself. Possibly a one-time "Auto-sync moved to Settings" hint for upgraders, or a
    Settings link from Configure Tools. → ticket 20, low.
  - Note for the model-gap finding: auto-sync is what drives the re-assert behind P1·Q3, P2·Q4 and P2·Q6, and it now
    lives out of sight. The main screen never shows it's on.
- **P3·Q2** (★ auto-sync off: what happens to `~/.cursor/skills/zod-schema-first`). *"If autosync is off, then what will
  happen is simply that the skill will be imported into the central library without being deployed to any tools. It
  would be deleted from the original source locations. I'm assuming both locations would be deleted in the case of a
  skill that was detected in multiple tools … I assume the match is identical and is truly safe to both be deleted,
  versus when it's a conflict, you have to pick which one you want, and the fate of the one you don't choose is kind of
  unknown. I would assume, though, it's going to be deleted, but that is not specifically stated, nor is there a choice
  being given as to whether you want to migrate the one you choose and then keep versus delete the other."*
  - Facts (`core/onboarding_import.rs` module doc; `useAddSkillFlow.ts` ~:908, which hardcodes `remove_originals:
    true` "until the review offers the per-run choice (ticket 21)"):
    - For an ordinary group with auto-sync off: finalize, then remove each original **only if byte-identical** to the
      central copy. His general model is right.
    - **zod-schema-first is a name he already manages** (it's in his library, synced to Claude Code). A managed-name
      group is never finalized: without the operator's "replace" it is `Skipped` and nothing is touched. The legacy
      review shows it as an ordinary "Match" and never says the name is taken, so **he expects an import plus removal
      where the run will skip it.**
    - **A Conflict's unchosen variant is left in place and reported**, not deleted. He assumes deletion, an
      over-estimate in the safe direction. He is right that the screen states neither fate and offers no choice.
  - Severity: **ticket 21**, all of it, and exactly its scope (managed-name groups with Replace/Leave; the
    remove-originals checkbox; each variant's fate stated). Not a hold: the misjudgment is toward *more* removal than
    happens.
- **P3·Q3** (★ tdd "Match", already managed). *"Well, if match means that the copy within Pi is identical to the one
  in the central library, then I simply want the physical copy in Pi to be deleted and a symlink will take its place."*
  - Facts:
    - The legacy badge "Match" is `consistent` (`ImportModal.tsx` ~:93, `resources.ts` ~:94). It means **the
      discovered copies agree with each other** (`has_conflict: false`). It says nothing about the managed copy.
    - In this scenario Pi's tdd is a fork ("pi's tdd fork"), different from the managed tdd.
    - The run will **skip** the group: the name is managed, and the legacy review can't send "replace".
  - What happened: **he reads "Match" as "identical to my central copy"** and expects a replace-with-link. Today
    nothing happens, which is safe. Under ticket 21's Replace this misreading would be dangerous: he would replace a fork
    believing it was identical. The backend raises the ask for exactly this (`TARGET_EXISTS`).
  - Severity: **ticket 21, binding.**
    - The badge must name what it compares: "copies agree" vs "same as your managed copy" / "differs from your managed
      copy".
    - A managed-name group must say the name is taken, before the Replace choice.
    - Not a hold for 1.3.0, which skips the group.
- **P3·Q4** (★, the ones that didn't import). *"If I were in the mood to address all of the discovered skills, I would
  make choices one by one. If I choose not to implement anything else, the other skills would remain in the review page
  for me to revisit later because I do want to always be able to know when I have unmanaged skills deployed in my
  environment."*
  - Snapshot right after: 25 skills, banner still "Review 9 discovered skills", no dialog open. **No import appears to
    have run.** The facilitator can't evaluate in the tab (the human has control). Asked neutrally whether the button
    was pressed.
  - Stated need, regardless of the run: **unmanaged skills must stay visible and revisitable**, a standing inventory,
    not a one-shot import. → ticket 21 (and the banner's persistence).
- **P3·Q4, continued** (he then pressed the button). *"in the settings menu I never actually disabled auto sync, so the
  button said import and sync, and I clicked it. Now I saw some stuff process, but in the foreground the window remains
  open and it still shows me the skills. But they have reduced down to six discovered skills. And if I am to assume that
  the reason why this happened was an error, I will need to close the window and look for these skills in my library to
  see what happened."*
  - **Correction to P3·Q1–Q3:** he found the Auto-sync switch in Settings but **didn't turn it off**. Q2 and Q3 were
    asked "with auto-sync off" and answered as stated; the run itself happened with auto-sync **on**. The button label
    ("Import & Sync") told him so.
  - What happened: after the run **the review stays open with no result**. Six rows remain (from 9), with no word on
    whether each was skipped (a managed name), kept (a divergent sibling), failed (e.g. Amp's unwritable original), or
    simply not chosen. He would have to close it and search the library to find out what imported. The 2026-10-05
    pre-check saw the same: 5 imported, the review left open.
  - Severity: **ticket 21** (a Result step: imported · skipped with reason · failed · left in place, the D11 count
    line). Not a hold: nothing destructive happened.
- **P4·Q1** (★ local-notes, source folder missing: where was it, how to re-point). *"It was located within
  /Users/alex/Projects/notes-skill I would point it by clicking on the Change Source button and navigating to a new
  place or choosing a Git repo or whatever I want the actual new source to be."*
  - What happened: **correct.** The recorded folder was read off the inspector's Source line (D18), and Change source…
    was found as the repair, including the option of switching to a Git source (ADR-0003's Re-point to any
    provenance).
  - Severity: none (a pass).
- **P4·Q2** (★ task: re-point deprecated-helper to the ts-error-fixer URL). *"I found a deprecated helper in the list. I
  clicked on it to open the right side sidebar and clicked on change source, pasted in the GitHub URL, and clicked on
  check source. It says that it will replace deprecated helper bytes with TS Error Fixer, and I will click on continue
  with TS Error Fixer. And it says that it will keep the name deprecated helper and take the bytes of TSErrorFixer.
  Personally, I don't like that because hypothetically, if this was a situation in which Matt Pocock deliberately
  renamed a skill … I should either have it rename by default or give me a choice to override that behavior and not
  rename it. But the renaming should be default for sure …"*
  - What happened:
    - **The preview caught the mismatch before anything was replaced** (D-feature, ticket 06/12: Check source → "replace
      deprecated-helper's bytes with ts-error-fixer" → an explicit "Continue with ts-error-fixer"). He noticed it there,
      which answers round 18's Flow 5 Q2 ("when would you notice?"): **at the check, before replacing.** A pass.
    - The task's wording ("now lives at") framed the URL as a deliberate upstream rename, so continuing was reasonable.
      It doesn't show he'd miss a genuinely wrong paste.
    - **Product request:** when the re-pointed skill has a different name, **rename to the upstream name by default**,
      with an option to keep the old name. Today Re-point always keeps the managed name.
  - Severity: BACKLOG (Parked, needs a product decision). A rename moves every target folder (global and project) and
    the central dir, and touches Edits and health keys. Decide the default with the operator. Not a hold.
- **P4·Q3** (★ api-style conflict). *"Upstream changed it to model only, and I can click on the resolve button, choose to
  either keep my setting or use the skills setting, or choose another setting … I decided to keep it user-only and save.
  In my mind, what would happen during the next update is that the conflict will no longer get flagged because I
  already chose that I want this override."*
  - What happened: **correct.** Upstream wants model-only, he kept user-only, and keeping re-bases the Edit on the new
    upstream, so the next Update stays quiet unless upstream moves again. Resolve… found from the inspector.
  - Severity: none (a pass). This answers round 18's Flow 5 Q3 cleanly.
- **Part 5 setup.** The operator reloaded (⌘R): a fresh `failures` library.
- **P5·Q1** (★ task: add acme/collide-kit to every Tool). *"I clicked on Add Skill, the window popped up. I pasted in the
  URL. I decided to check the Edit Tools button just to see what would be deployed, but I didn't change anything. It has
  all tools in it. Then I clicked on the button where it detected all the skills. I could choose individual skills or
  select all, so I kept all and clicked on Add Selected. Now it says a folder already exists with different content …
  I will choose to override everything. Three of the skills were added, and then the release notes one had an API rate
  limit on GitHub. I'm going to retry. That time it succeeded without a rate limit error. At the bottom, there's also a
  warning that it cannot sync to AMP because of permission denied on the config/agents/skills, and to please check the
  directory permissions."*
  - What happened: **the whole ticket-16 flow worked as designed**:
    - Source → Skills → Result;
    - he inspected the Tool-scope popover;
    - one overwrite ask covered both collisions;
    - the Result showed the rate-limited row, with Retry re-sending only it;
    - the distinct deploy reason (Amp, permission denied) appeared once beneath the rows.

    No hesitation reported.
  - **"Overwrite everything" again, without seeing the folders' content.** This is the same pattern as P2·Q4 and
    confirms that finding: the ask gives him nothing to judge with, so he defaults to the managed copy. P5·Q2 ("what do
    you need to decide?") was **skipped as already answered** in P2·Q4: who put it there, plus content and a diff.
  - Severity: none for the Add flow (a pass). The overwrite ask's missing provenance and diff stays at P2·Q4's
    severity.
- **P5·Q3 (script Q4)** (★ where do you look after Add; what would "Keep existing" have shown for Claude Code).
  - Where: *"I will look for the name of that skill in the list … findable by their repo name … I can see five solid
    dots, two X's, and a blank indicating five tools succeeded, two failed, and one is not synced, which I recall from
    before is WindSurf because it's not installed, and yet it is in my configure tool list. If it were not in that list,
    it would not appear there as not synced because it's not a tool I'm managing."* **Correct**: the repo group, then
    the strip.
    - Side note: a selected-but-uninstalled Tool (Windsurf) puts a permanent hollow mark in every strip. He understands
      why, but it's standing noise. Configure Tools could flag "Windsurf: not detected — remove from the selection?".
      → ticket 20 / BACKLOG, low.
  - Keep: *"I'm not sure what I would expect if I chose to keep existing … I'm guessing that if it would either import
    the cloned code copy into the library and redeploy as a symlink, or maybe it would simply skip that import and it
    would not appear under the Acme repo …"* Both guesses are wrong.
    - Fact (D9/D12; CHANGELOG 1.3.0 "Answering 'Keep existing' … disables the skill on that tool"): the skill **is**
      added from acme/collide-kit. The Claude Code folder is left untouched, and the pair is recorded **disabled**
      (slashed ring), so it isn't asked again.
    - **The ask doesn't state Keep's consequence.** Together with P2·Q4: the ask needs provenance, a diff, and
      per-button consequences ("Keep theirs — {skill} stays off {Tool}; you can Enable it later").
    - Also restated: *"in theory, there would be an option to see a diff if there's ever any doubt"*.
  - Severity: joins P2·Q4's overwrite-ask BACKLOG item (high). Not a hold.
- **P5·Q4 (script Q5)** (★ Cancel partway through total-typescript/skills; what was installed). *"I started the
  install, then I clicked cancel, and for whatever reason, it popped up that the folder already exists with different
  content … So I clicked on keep, and then when that pop-up disappeared, that particular skill ts-reset finished what it
  was doing because it already started before I canceled. But since I chose to keep the existing folder in Cloud Code,
  it deployed to four tools and did not deploy to Cloud Code, apparently. Meanwhile, the other three did not get
  started, so nothing was kept."*
  - What happened: **correct**, and it answers round 18's Flow 1 Q4.
    - The in-flight candidate finishes; Cancel is honoured between candidates (ticket 16, D20).
    - The rest were cancelled with nothing written.
    - **Keep's effect was understood once seen**: not on Claude Code. That makes the P5·Q3 gap a matter of *stating*
      it in the ask beforehand.
  - Minor: an overwrite ask arriving *after* Cancel felt odd ("for whatever reason"). The ask could say "ts-reset was
    already being added when you cancelled". → BACKLOG, low (Add, with the overwrite-ask item).
  - Severity: none (a pass).
- **P6·Q1** (★ every cell needing attention, monorepo, colour first). *"it would appear that Doc X failed. All I know is
  that it failed. Front-end design is missing in Windsor. Agent skills for TDD failed. Also … for the last two, I saw the
  message by hovering, but for DocX, I actually accidentally clicked on it, which undeployed it. So I clicked on it again
  to try to redeploy, and that's why it says failed … I also see that Design Tokens says that it is stale, which I have
  to assume is an indicator that the upstream central copy is stale because there's a new update in the GitHub repo and
  we have not refreshed."*
  - Seed for monorepo (`scenarios.ts`, failures):
    - tdd/Claude Code synced; tdd/.agents/skills error (permission);
    - frontend-design/Claude Code **stale**; frontend-design/Windsurf missing;
    - release-train/Claude Code **pending**; release-train/.agents/skills synced;
    - docx/Claude Code **missing**;
    - design-tokens/Claude Code stale.
  - What happened:
    - Read correctly: frontend-design/Windsurf missing, tdd/.agents/skills failed (both from hover text).
    - **Accidental unassign.** One click on docx's cell un-assigned it. That removes the project's copy when one exists,
      with no confirmation or Undo. His re-click re-assigned it and it failed: docx's central copy is missing, so it
      can't be re-deployed. The original state was "missing", which he never read.
      - Severity: BACKLOG, medium: a 5 s Undo on a cell's toggle-off, as the inspector's targets list already has (D21).
        Check whether ticket 17's larger cell hit area made misclicks likelier than in 1.2.x.
      - Not a hold: unassign is reversible whenever the skill has a central copy. Here it cost nothing, since the
        artifact was already gone.
    - **"Stale" misread as "a newer version upstream, not yet refreshed".** In fact it is **project-copy drift**: a
      copy-mode assignment whose content differs from the central copy. The fix is Sync this, not Refresh, and the
      backend has no "update available" state (`scenarios.ts` header; BACKLOG #62). This is the misreading the round-18
      bake-off warned about.
      - Severity: 1.3.x → BACKLOG: a legend/word change, e.g. "Out of date copy — Sync this" instead of "stale", here
        and on the strip ("stale — the copy differs from the skill").
      - Not a hold: no destructive consequence. Refresh wouldn't fix it, Sync this would, and nothing is lost.
    - **Not mentioned:** frontend-design/Claude Code stale, and release-train/Claude Code pending ("not synced yet").
      Couldn't verify on screen (the human has the tab). Ask in the debrief whether he saw them.
- **P6·Q1, grayscale** (macOS Color Filters → Grayscale, applied by the operator). *"I can tell that a solid gray dot is
  probably just the green dots that say synced. And then the errors are X's, stale is a half-filled circle, and an empty
  circle is pending. So from a symbology standpoint, the symbols are able to distinguish the statuses without color
  being visible."*
  - What happened: **pass.** All four shapes were told apart without colour (ticket 17's cell marks, D22). Here he names
    the hollow "pending" mark, so release-train/Claude Code was seen after all.
  - Severity: none. This closes round 18's colour-only matrix finding (`f4-matrix.png`).
- **P6·Q2 (script Q3)** (★ archived-app gone from disk; what to do, what Sync All says). *"If I wanted to fix it, I would
  need to point it to the right folder, and if I don't want to fix it, I would just delete the archived app from the
  projects entirely. In the meanwhile, I expect that sync all would simply skip deploying there because the location
  does not exist. It might flag an error in the notifications, perhaps. Oh yes, I see it. There is a warning that says
  the folder not found; nothing in it was synced. Okay, yeah, that makes sense."*
  - What happened: **correct, and confirmed live.** Repairs: update the path, or remove the project. The expectation
    "skip, not recreate" matches ticket 17's core fix (`skipped_projects: folder_missing`, no folder recreated). The
    warning "{project}: folder not found — Update path or Remove · Nothing in it was synced" was read as intended. It
    was a warning, not an error: right.
  - Severity: none (a pass).
- **P7·Q1** (★ free use on `?scenario=rich&latency=3`: what would you do first each morning?). *"Well, hypothetically, if
  I did not set up an automatic schedule to update and refresh automatically, then the first thing I would do in the
  morning is hit refresh all. After that, I would notice the failed targets and skills with issues and look into them to
  diagnose what's going on there."*
  - What happened: **the morning loop is Refresh all → the summary line / Issues → diagnose.** That's the flow 1.3.0's
    header summary and Issues chip were built for.
  - **Operator evidence for BACKLOG #53** (scheduled Refresh all): he assumes he'd set up an automatic schedule.
  - Interacts with the re-assert model gap: every morning Refresh also deploys to every selected Tool.
- **P7, volunteered (visual).** *"in both light and dark mode, it's very, very monochrome. The only splash of color is the
  little bit of purple in some of the top areas. I wouldn't mind adding just a little tiny bit of extra color, perhaps,
  but we can think about that later … I think that it looks better in dark mode than in light … the monochrome chrome
  look seems to feel more bland in the light mode than it does in the dark mode."*
  - Severity: BACKLOG, low (design, after 19b). A restrained colour pass on the token world, the `impeccable` colorize
    mode within ADR-0006 / DESIGN.md contrast rules, with the light theme as the weaker of the two. Not a hold.
- **Debrief** (the operator's answers).
  1. Most confusing: *"some of the phrasing things we talked about, and a lack of clarity as to what the consequences
     would be when choosing from multiple versions of a discovered skill."* That is the confirmation copy (P2·Q8) and
     the Import Conflict variants' fates (P3·Q2) → ticket 21.
  2. Missing: *"That was probably it"*: the diff in the overwrite ask (P2·Q4).
  3. "Seven" in P1·Q1: *"I think it was because of the seven failed targets in the header, and it was also coincidental
     that … there were 7 items visible because I forgot to scroll down, so I probably counted both."* **The header's
     target count was read as a skill count.** The summary line puts two units side by side ("7 failed targets · View
     11 skills with issues"), and the first number got taken as the answer. → BACKLOG (My Skills summary: lead with
     the skill count, or make the units unmissable).
  4. noisy-linter's slashed rings: *"I might not have noticed them at first."* Then, unprompted: *"we have three
     statuses: not synced, enabled, and disabled. The distinction between disabled and not synced is unclear … it just
     seems like as soon as you ever have synced something for the first time, from that point moving forward, it would
     only ever be an enabled status or a disabled status … Maybe you can remind me why we have three statuses instead of
     just a binary."* **The fourth confirmation of the model gap** (P1·Q3, P2·Q4, P2·Q6): the operator can't tell "not
     synced" from "disabled" and asks why both exist.
  5. mirror-tools' switch in the first pass: *"I don't remember what I did."* Unresolved. **Reproduce after the
     session**: does a second row switch clicked while the first Disable runs get refused, and is the refusal visible?
- Session recording: `preview_recording_stop` answered "No recording is active for this thread". The recording started
  at 15:35:37Z was lost, most likely when the operator took control of the tab or reloaded. These notes are the record.
- **mirror-tools reproduced (13:47 CDT, facilitator's own background tab, fresh `failures&latency=3`).** Clicked three
  row switches 250 ms apart: noisy-linter, then flaky-upstream, then mirror-tools.
  - **Only the first turned off.** While its Disable ran, the other two switches were refused: `aria-disabled="true"`,
    with the reason "Disable is running — wait for it to finish" only as a description/title. They stayed on, with no
    toast and no visible feedback beyond the switch not moving.
  - This is exactly the operator's state: he believed all three were off and one stayed on. One mutation at a time
    (D19) and the row switch are both **new in 1.3.0**.
  - **Hold candidate:** he believed a skill was off when it was on, and the cause is new in this release. The fix is
    small: a refused click shows its reason visibly, e.g. a toast or inline note "Still disabling noisy-linter — try
    again in a moment".
- **P2·Q10 Finding A, verified (13:55 CDT, background tab, fresh `failures`).** Finish removing on release-train gave
  "release-train was not deleted · Removed from Codex, Pi. · Still in Claude Code: ~/.claude/skills/release-train ·
  Still in .agents/skills · monorepo: …".
  - The "Still in" lines carry the project scope ("· monorepo"); **the "Removed from" summary doesn't.**
  - In the operator's run, Refresh had first synced monorepo's pending Claude Code assignment, so "Removed from …
    Claude Code" (the project row) printed next to "Still in Claude Code" (the global row). The scope label is missing
    on the removed side.
  - Severity: BACKLOG, low. Name the project on the "Removed from" side as the "Still in" lines already do.

### Outcome and routing (2026-10-27)

**Hold decision (operator, at the debrief).** One finding holds the 1.3.0 push. Separately, the operator added an
outside defect to 1.3.0:

| Item | Why | Where |
|---|---|---|
| A refused switch click looks like nothing happened, so the operator believed a skill was off when it was on (P2·Q8, reproduced) | New in 1.3.0: the row switch and one-mutation-at-a-time (D19) | **ticket 27**, blocks 19 |
| Executable bits dropped on the GitHub Contents fast path (UNI-163, raised by the operator from another agent) | Not from this session; script-driven skills fail on every Refresh | **ticket 28**, blocks 19 |

Not holds, by the operator's decision:
- The overwrite ask's missing provenance and diff (P2·Q4): unchanged since 1.2.17.
- The "not synced" vs Disabled model gap (P1·Q3, P2·Q4, P2·Q6, debrief 4): the behaviour predates 1.3.0 and needs a
  design decision.

**Routed:**
- **BACKLOG** (operator evidence on #53 and #21):

  | Item | Findings |
  |---|---|
  | #70 Overwrite ask: who put the folder there, its content, what Keep does, and a record of what was answered | P2·Q4, P5·Q3, P5·Q4 |
  | #71 "Not synced" vs Disabled, and Refresh's re-assert | P1·Q3, P2·Q4, P2·Q6, P2·Q10 C, debrief 4 |
  | #72 My Skills reading fixes: summary units, problems below the fold, unlabelled strip marks, "stale", Sync offered on an uninstalled Tool, a buried "to review" item | P1·Q1, P2·Q5, P2·Q6, P6·Q1 |
  | #73 Bulk confirmation copy | P2·Q8 |
  | #74 Notifications after a relaunch, and the vanishing run toast | P2·Q4, P2·Q7 |
  | #75 Matrix cell Undo | P6·Q1 |
  | #76 Removal report scope labels and permission guidance | P2·Q10 |
  | #77 Re-point renames to the upstream name | P4·Q2 |
  | #78 A restrained colour pass | P7 |

- **Ticket 21:** P3·Q2–Q4 and debrief 1 (binding Comments).
- **Ticket 20:** P3·Q1, and the not-detected selected Tool from P5·Q3.
- **AGENTS.md:** the fixture-reload gotcha (facilitator incident).

**Passes** (1.3.0 behaviour the operator read correctly, first time):
- Disable durable across Refresh;
- inspector targets;
- the Refresh count line and per-item repairs;
- Edit conflicts;
- Change source's mismatch preview;
- Delete's what-goes / what-stays;
- the Add flow with Retry and Cancel;
- Projects' shape-coded cells in grayscale;
- the missing-folder skip.

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

- 2026-10-27 — **Session run** (orchestrator + operator, about 2 h 15 min including breaks, option B: ★ questions
  only from Part 3). § Findings holds every answer verbatim, with the screen state and a severity, and § Outcome and
  routing holds the dispositions.
  - Facilitator incident: writing to this tracked file during the session full-reloaded the fixture page (Tailwind
    scans tracked files). The affected step was redone, and live notes moved to the gitignored
    `evidence/18/findings-live.md`.
  - The recording was lost when the operator took over the tab, so the notes are the record.
  - Status → `implemented` (round 19's interim word; normalised to `done — <sha>` at 19a closure).
- 2026-10-29 — **done — 9aeec8b (think-aloud run and routing)** (post-rebase SHAs on `origin/main`; the pre-rebase SHAs cited above exist only in the orchestrator's local history). Shipped in 1.3.0 (`v1.3.0`, published 2026-10-28; operator smoke on the main Mac passed 2026-10-29, ticket 19 § Smoke outcome). 19a closure: `round19/spec.md` § Closure — 2026-10-29.
