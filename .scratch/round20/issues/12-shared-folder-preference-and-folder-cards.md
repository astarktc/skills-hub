# 12: Shared-folder preference and the Harnesses pane's folder cards

Status: ready-for-agent

**Blocked by:** 10.

**Spec:** `.scratch/round20/spec.md` § Wave B spec → § Placement (the serving rule), § Settings and the Harnesses pane;
grill Q14–Q17, Q20, Q24, Q26, Q30, Q31; user stories 20–32, 38–41. Absorbs BACKLOG **#109** (global scope) and
**#116**.

**What to build:** with the preference on (the default), each harness gets each skill from exactly one folder. The
Harnesses pane shows one **folder card** per folder written: one switch per card, harness names first, and every
installed harness that sees the folder listed. Changing the preference, the both-folders choice or a card switch
previews "would move N" and runs as one redeploy action with a report.

## Acceptance criteria

- [ ] **The serving rule in placement (Q15/Q16).**
  - Reads `.agents` by default and not `.claude` → `.agents`.
  - Reads `.claude` and not `.agents` → `.claude`.
  - Reads both → the both-folders choice (default `.agents`).
  - Anything else → its own folder.
  - Preference off → its own folder.
  - Table tests cover every branch, plus a snapshot of the operator's machine (Claude Code, Codex, Pi → two cards).
- [ ] **Settings.**
  - `prefer_shared_folders` (default on) and the both-folders choice are typed `AppSettings` fields.
  - Each is written only through its own command, which previews and then runs the redeploy's unlocked seam inside
    one guarded section, never through `update_setting`.
  - Existing installs start with the preference on. The launch check (10) offers the move (Q17). Declining never
    turns the preference off.
- [ ] **Turning the preference off (Q30).** The confirmation lists every installed harness currently served by an on
      folder, all ticked. Unticking keeps that harness's own-folder card off. The answer becomes the card selection in
      the same action.
- [ ] **Folder catalog on the wire.** Per card:
  - folder key, display path, on/off;
  - served harnesses, other installed readers and reader-only harnesses, **detected only**;
  - undetected members, separately;
  - the duplicate lines it contributes.

  Plus a pane-level "Seen twice" list resolved per installed harness, selected or not (Q20, #116). `useToolCatalog`
  exposes cards; no surface rebuilds them.
- [ ] **Harnesses pane = C** (prototype branch `proto/waveb-harness-cards`, verdict 2026-11-06), with Q31:
  - a card's title is its folder ("Shared folder — `~/.agents/skills`", "Claude Code — `~/.claude/skills`"), with
    "Serves …" underneath;
  - "Also seen by" lists names only;
  - reader-only harnesses appear in their card with no switch;
  - each card has a closed "Not detected (N)" group;
  - own cards with no detected harness sit in the pane's "Not detected" group;
  - every consequence is in one "Seen twice (N)" section;
  - amber only for `LoadsNone` (fx);
  - the shadowed-own-copy flag and `FlagsConflict` are notes.
- [ ] **Confirmations** state "would move N" from the preview.
- [ ] **Primitives.** New variants are documented in DESIGN.md (ADR-0006), with their `ui/*.test.tsx` contract kept.
      `npm run styles:check` is green.
- [ ] **Frontend tests:** the folder-card presentation fold; the "Seen twice" fold; settings-world confirm → preview →
      run; Q30's all-ticked default; i18n coverage.
- [ ] **Fixture.** The `registry` scenario shows cards, and a new scenario carries the prototype's data set (the
      operator's machine plus Cursor, Gemini CLI, Cortex Code, fx, Deep Agents Code, Antigravity, Zed and Warp).
- [ ] **CONTEXT.md** gains **Shared-folder preference**, **both-folders choice**, **Served harness** / **reader**.
- BACKLOG #109 and #116 left the queue when this ticket opened.
- [ ] `npm run version:check && npm run check` is green.

## Notes

- Copy goes through `en` only. The orchestrator runs the copy pass with `impeccable` at merge (a worktree child has no
  `.pi/` skills).
- Verify in the fixture with a browser (`npm run dev:fixture`, never `tauri:dev`).

## Comments

- 2026-11-06 (orchestrator): **release split.** This ticket now also carries the moves of Codex (`.codex/skills`),
  Goose (`.config/goose/skills`), OpenHands (`.openhands/skills`) and Zencoder (`.zencoder/skills`) into
  `~/.agents/skills`, taken from ticket 10's Q12 table. Each old dir goes into `former_skills_dirs`, and the redeploy
  (ticket 10) moves their Deployments. They land together with the serving rule, so no harness reading both folders
  (Pi) ever lists them twice. This ticket's tests cover the operator's machine: Codex and Pi are both served from
  `.agents` with no duplicate.
- 2026-11-07 (orchestrator): carried over from the ticket 10 review, round 1
  (`reviews/round20-t10-redeploy-review.md`).
  - **Sol 101:** surface a `REDEPLOY_PENDING` skip in a normal sync with an actionable line that points to Settings →
    Harnesses, never a bare "skipped" plus Retry.
  - **Sol 103:** the launch offer remembers a declined offer by an opaque work fingerprint computed in core (planned
    row, destination, mode and pending journal identities), not by counts.
  - **Sol 105 / Astra 5:** a move blocked by an unfinished Disable gets truthful pending copy and a recovery action
    ("finish the Disable" or Enable), never "re-recorded".
  - **Sol 106:** bring the fixture's registry snapshot to the production facts as they stand after the release split.
    Derive edited-copy readers from it. Add a regression test that a moved target and its catalog folder agree.
- 2026-11-07 (orchestrator): carried over from the ticket 10 review, round 2.
  - **Sol 111:** `NOT_FOUND` of kind `redeploy_question` reads "question changed, run Move again".
  - **Sol 113:** a replacement undone at startup reads "discarded an unfinished replacement; still pending", not "old
    copy removed".
  - **Astra:** the fixture uses stable row-instance ids, not `${skillId}:${tool}`.
