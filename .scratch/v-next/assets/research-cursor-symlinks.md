# Research: Does Cursor discover symlinked Agent Skills?

Question: does the current Cursor release (IDE and/or Cursor CLI / `cursor-agent`) discover Agent
Skills placed as **symlinks** under `~/.cursor/skills`, project `.cursor/skills`, and `.agents/skills`?
Researched **2026-09-02** against primary sources only (cursor.com docs/changelog, Cursor's own
download API, and Cursor staff posts on forum.cursor.com). Repo claims cite files in this checkout.

**Bottom line: yes — symlinked skill directories are discovered by both the IDE and the CLI in current
releases.** The IDE bug was fixed in Cursor **2.5** (Feb 2026, staff-confirmed + reporter-confirmed);
the CLI shipped the equivalent fix in the **Jun 22, 2026** CLI release. Recommendation and caveats in
§6.

---

## 1. Version baseline (as of 2026-09-02)

| Surface | Latest version | Source |
|---|---|---|
| Cursor IDE (stable) | **3.18.25**, commit `280eca2911f1774689696e5f1efa5a4f97a87af3` | Cursor's own release API: `curl -sL 'https://www.cursor.com/api/download?platform=darwin-arm64&releaseTrack=stable'` → `{"version":"3.18.25", …}` (queried 2026-09-02) |
| Cursor CLI (`cursor-agent` / `agent`) | **2026.08.31-4057e58** | Official install script <https://cursor.com/install> pins `~/.local/share/cursor-agent/versions/2026.08.31-4057e58` (queried 2026-09-02) |
| Latest IDE feature changelog entry | "Start from scratch, without a repo", **Aug 27, 2026** | <https://cursor.com/changelog> |
| Latest CLI changelog entry | **August 26, 2026 release** | <https://cursor.com/docs/cli/changelog> |

Note on version numbering: the marketing changelog is organised by **minor feature release**, not by
patch build — patch bumps (3.11.10 → 3.12.10 …) are not written up individually. Staff statement:
<https://forum.cursor.com/t/changelog-or-release-notes-not-updated-why/165984>. So the *absence* of a
symlink note on a `cursor.com/changelog/<x-y>` page does not mean the fix wasn't in that release.

## 2. Official docs: no statement about symlinks (either way)

- **Agent Skills reference** — <https://cursor.com/docs/skills> (raw: <https://cursor.com/docs/skills.md>).
  Full-text search for "symlink": **no matches**. It documents discovery locations only:
  > "Skills are automatically loaded from these locations: `.agents/skills/` (Project-level),
  > `.cursor/skills/` (Project-level), `~/.agents/skills/` (User-level (global) on the local machine),
  > `~/.cursor/skills/` (User-level (global) on the local machine)". Plus, "For compatibility, Cursor
  > also loads skills from Claude and Codex directories: `.claude/skills/`, `.codex/skills/`,
  > `~/.claude/skills/`, `~/.codex/skills/`."
- **Help page** — <https://cursor.com/help/customization/skills>. No symlink statement either. Confirms
  the skills root is walked **recursively** ("Cursor walks the skills root recursively, so category
  folders work") and that nested project skill dirs (`apps/web/.cursor/skills/`) are picked up and
  path-scoped.

**Conclusion:** symlink support is not a documented contract on either side. It is changelog + staff-
statement behaviour, not a specification. That matters for the recommendation (§6).

## 3. Changelogs

### 3.1 CLI — the verbatim "symlinks" entry

Source: **CLI Changelog**, <https://cursor.com/docs/cli/changelog>, section
`### MCP and skills` under the **June 22, 2026 release** (anchor `#jun-22-2026-mcp-and-skills`):

> **Skills found through symlinks.** The skills menu follows symlinked directories when discovering skills.

Two things to note precisely:
- This is the **CLI** changelog and it is dated **June 22, 2026** — *not* Cursor IDE 2.5 (Feb 17, 2026).
  The premise in the brief ("Cursor 2.5 (Feb 2026) changelog entry: Skills found through symlinks") is a
  conflation: the *IDE* fix landed in 2.5 per staff/forum (§4), while the wording "Skills found through
  symlinks" is the *CLI* entry from June.
- It says **symlinked directories**. Nothing in any Cursor changelog addresses a symlinked
  **`SKILL.md` file** inside a real directory.

### 3.2 IDE — 2.5 page has no symlink line

<https://cursor.com/changelog/2-5> ("Plugins, Sandbox Access Controls, and Async Subagents", **2.5,
Feb 17, 2026**) covers marketplace plugins, sandbox network controls, async subagents. **No mention of
symlinks or skills discovery** — consistent with §1's note that patch-level fixes are not written up.
The IDE fix is therefore evidenced by staff statements and user confirmation (§4), not by the changelog.

### 3.3 Adjacent CLI entries that shape the risk picture

All from <https://cursor.com/docs/cli/changelog>:

- **Aug 11, 2026 release**, "Skills, custom modes, and goals":
  > **Skill discovery skips hidden directories.** Skill and subagent scans no longer descend into hidden
  > dot-directories, avoiding slow loads from large nested folders.
- **Aug 11, 2026**: "**Managed skills can ship Markdown resources.** Managed-skill sync now materializes
  a skill's nested Markdown files under `~/.cursor/skills-cursor` …" (i.e. Cursor has its own managed-skill
  sync mechanism writing to `~/.cursor/skills-cursor`).
- Earlier CLI entry: "**Nested rules and skills.** `.cursor/rules` and `.cursor/skills` in subdirectories
  are discovered everywhere, matching the IDE."
- Earlier CLI entry: "**Skills from other tools' directories.** Skills are also discovered in
  `.claude/skills`, `.agents/skills`, and `.codex/skills`."
- Jul 6, 2026: `user-invocable: false` frontmatter hides a skill from `/` autocomplete.

The hidden-directory skip is the one live risk for Skills Hub specifically — see §6 caveat 2.

## 4. Forum evidence with staff confirmation (chronological)

### 4.1 The original IDE bug (global scope, directory symlink)

<https://forum.cursor.com/t/cursor-doesnt-follow-symlinks-to-discover-skills/149693> — filed Jan 23,
2026 against IDE **2.4.21**: "I symlinked a skill folder to `~/.cursor/skills` but the `SKILL.md` inside
this directory is not discovered by Cursor." Repro = symlink a skill *directory* into `~/.cursor/skills`.

- Staff **Colin**, Jan 23, 2026 (post 4): "Thanks for reporting this. This is a known issue on our side
  (and also affects Rules), and we have a ticket open for this already."
- User **Tom**, Jan 23 (post 5): "I just tested a symlink from `<repo-root>/.cursor/skills` to
  `<repo-root>/.codex/skills`, and it seemed to work fine in v2.4.21. Does this issue only affect
  symlinks in the home directory skills folder?" → Staff **Colin** (post 6): "The other reported issue
  was for home directory rules, so it sounds like that's the case!"
  ⇒ **Even in the broken era, project-scope symlinks worked; global (`~/.cursor`) scope was broken.**
- Staff **Colin**, Feb 6 (post 14): "A fix is in the pipe!"; Feb 10 (post 16): "**This will be fixed in
  2.5**, which is just around the corner."
- Original reporter **borekb**, Feb 19, 2026 (post 17): "**I can confirm that this is now indeed fixed in
  2.5** 🎉" — followed by a report that `agents/` and `rules/` symlinks still misbehaved on **2.5.20**
  (i.e. skills fixed, other symlinked config dirs not).

Corroborating duplicate threads with the same staff line:
- <https://forum.cursor.com/t/global-symlinked-skills-are-not-discovered-by-cursor/150028/6> — staff:
  "This will be fixed in 2.5, which is just around the corner!"
- <https://forum.cursor.com/t/skills-installed-by-skill-sh-will-not-be-found-if-use-symlink-method/151014/6>
  — staff **Colin**: "**This is fixed now in 2.5!**" (repro was `npx skills add … → Symlink (Recommended)`).
- <https://forum.cursor.com/t/bug-report-skills-in-symlinked-directories-not-detected-after-cursor-restart/150029>
  — staff **deanrie**, Jan 27, 2026: "Cursor doesn't follow symlinks yet when indexing skills (and rules
  too). … Other users say symlinks in the project-level `.cursor/skills` (in the project root) seem to
  work in v2.4.21, but symlinks in the home directory `~/.cursor/skills` don't. … Workaround for now:
  copy the files directly instead of using symlinks until it's fixed."
  This is the thread whose title matches the "Settings tab drops symlinked skills after restart" symptom.
- Feature request thread (context only, no staff verdict):
  <https://forum.cursor.com/t/agent-skills-must-see-symlinks/150093>.

### 4.2 The 2.5.22 "still broken" report — and its status

In thread 149693, a user (Feb 2026): "I'm on **v2.5.22**, and skills installed via symlink still
disappear from the Skills settings tab after restarting Cursor. Also, when trying to install skills by
copying (instead of symlink) they are installed as Rules instead of Skills." Staff **Colin**, Feb 24,
2026 (post 20): "Please raise new threads."

Assessment: this is a **single unreproduced tail report on a Feb-2026 build**, closed out procedurally
rather than confirmed, contradicted in the same thread by the original reporter's 2.5 confirmation. It
is six months and ~13 minor releases stale relative to 3.18.25. I found **no post-2.5 thread claiming
global symlinked skills are undiscovered in the IDE**; the later symlink threads assume discovery works
(§4.4).

### 4.3 CLI: symlinked-target scope, tested by staff (Jun 2026)

<https://forum.cursor.com/t/discovery-of-symlinked-skills-not-working-for-all-cases-in-cli/163569> —
filed Jun 18, 2026 (CLI `2026.06.15-18-00-12-6f5a2cf`). Claim: "When skills are symlinked into
`.cursor/skills/`, **the IDE agent discovers all of them**, but the CLI agent only discovers skills whose
symlink targets live under `.cursor/` or `.claude/`. Symlinks pointing to repo root, `.agents/`, or other
paths (e.g. `.hidden/`) are ignored by the CLI." The report ships an 8-case repro script (targets at repo
root, `.agents/`, `.cursor/`, `.claude/`, nested variants, and a `.hidden/` dir).

Staff **deanrie**, Jun 18, 2026 (post 5): "I ran your repo exactly as-is on the latest CLI. **Discovery
finds all 8 skills no matter where the symlink target lives (root, `.agents/`, `.hidden/`, etc.)**. So
either this was already fixed compared to build `2026.06.15`, or the actual discovered set is different
from what you described."

⇒ On the CLI, as of late Jun 2026, symlinked skill dirs are followed **regardless of where the target
lives, including a hidden dot-directory target**. (This predates the Aug 11 "skips hidden directories"
change — see caveat 2.)

Related CLI parity gaps (not symlink-specific, but relevant if the Cursor CLI matters to users):
<https://forum.cursor.com/t/skills-not-shown-in-cursor-cli-while-visible-and-working-in-desktop-app/159218>
— staff **deanrie**, Apr 28 2026: nested skills not picked up by CLI, plugin-shipped skills not
registered in CLI, "there's no file watcher in the CLI, so skills added during a session won't show up
until you restart". In the same thread staff *recommend symlinking* as a workaround: "you can copy or
**symlink** the skill folders you need from `~/.agents/skills/…` into `~/.cursor/skills/`."

### 4.4 Most recent symlink datapoint: IDE 3.15.19, Aug 2026 — discovery works

<https://forum.cursor.com/t/cannot-open-symlinked-global-skill-rule-in-editor/168425> — filed Aug 14,
2026 on IDE **3.15.19** (build date 2026-08-11): "In Customize → Skills **I see my global skills. Some of
these are symlinks** to files in my workspace. When I click on one I get: The editor could not be opened
because the file was not found."

Staff **kevinn**, Aug 14, 2026 (post 7): "What you're seeing isn't intended, and **it isn't caused by the
symlink** or anything in your setup. In a remote SSH window, Customize is opening global skills and rules
against the local Windows machine instead of the Linux home. … **You can leave the symlinks as they are.**"

⇒ Strongest current-build evidence: on 3.15.x, **symlinked global skills are listed in Customize →
Skills**; the only residual symlink-adjacent bug is a *remote-SSH editor-open path* issue (clicking a
skill to edit it), not discovery. Note this user's symlinks were "symlinks to **files**" — the closest
thing to file-symlink evidence found, though the wording is ambiguous and it may be dir symlinks to
files-in-workspace.

## 5. Matrix of what is established

| Surface | Scope | Directory symlink | Symlinked `SKILL.md` file |
|---|---|---|---|
| IDE ≥ 2.5 (current 3.18.25) | global `~/.cursor/skills`, `~/.agents/skills` | **Works** — staff "fixed in 2.5" (149693/16, 151014/6, 150028/6), reporter-confirmed on 2.5 (149693/17), and listed in Customize → Skills on 3.15.19 (168425) | **Unverified** — no primary source either way |
| IDE (all eras) | project `.cursor/skills`, `.agents/skills` | **Works** — worked even in the broken 2.4.21 era (149693/5–6; 150029/3) | Unverified |
| CLI ≥ Jun 22 2026 (current 2026.08.31) | both | **Works** — changelog "Skills found through symlinks" + staff test of 8 target locations incl. `.hidden/` (163569/5) | Unverified |
| Docs contract | — | **Not documented** (`cursor.com/docs/skills` has zero "symlink" hits) | Not documented |

Outstanding/unresolved, all non-blocking for skills discovery: symlinked **rules**/`agents/` dirs were
still reported broken on 2.5.20 (149693/17); remote-SSH click-to-edit of a symlinked global skill is
broken on 3.15.19 (168425); one unreproduced 2.5.22 "disappears after restart" tail report (§4.2).

## 6. What this means for Skills Hub, and the recommendation

Current state in this checkout: `src-tauri/src/core/tool_adapters/mod.rs:191–199` sets the Cursor entry
to `supports_symlink: false` with the comment "Cursor cannot read a symlinked/junctioned skills dir:
every sync path copies", and `core/sync_engine.rs:149–152` (`sync_dir_for_tool_with_overwrite`) hard-routes
`!supports_symlink` to `sync_dir_copy_with_overwrite`. The central repo is `~/.skillshub`
(`core/settings.rs:37`, `CENTRAL_DIR_NAME = ".skillshub"`), i.e. symlink targets would resolve into a
**hidden dot-directory**. Also relevant: at project scope Cursor is a constituent of the `agents_skills`
virtual group (`project_relative_skills_dir: ".agents/skills"`), whose dir is symlinked regardless of the
Cursor entry's capability (per AGENTS.md, v-next ticket 36) — so **project scope is already effectively
symlinked today and evidently fine**, which is itself corroborating evidence.

### Recommendation: **flip to symlink — with caveats** (confidence: moderate-high, ~80%)

Why flip:
1. The exact failure mode the flag encodes (dir symlink in `~/.cursor/skills` not discovered) is
   **staff-declared fixed in 2.5** and independently confirmed by the original reporter — six months and
   ~13 minor versions before current 3.18.25.
2. The CLI has an explicit changelog line for it (Jun 22, 2026) plus a staff-run 8-case symlink-target test.
3. The most recent primary datapoint (IDE 3.15.19, Aug 2026) shows symlinked global skills **listed in
   Customize → Skills**, with staff saying "you can leave the symlinks as they are".
4. Cursor staff themselves recommend symlinking skill dirs into `~/.cursor/skills` as a supported workaround.
5. Project scope already ships symlinked via the `agents_skills` group with no known Cursor complaints.

Caveats to attach to the flip (these are why it is not an unconditional "flip"):
1. **Not a documented contract.** `cursor.com/docs/skills` never mentions symlinks, so this is
   changelog/staff behaviour that could regress silently. Whatever we do, keep the per-adapter capability
   flag as the single lever (never a `"cursor"` string check) so a regression is a one-line revert.
2. **Hidden-target risk, untested for the IDE.** The CLI changelog (Aug 11, 2026) says "Skill discovery
   skips hidden directories… scans no longer descend into hidden dot-directories." Our targets live under
   `~/.skillshub/…`. Staff's Jun 2026 test found a `.hidden/`-targeted symlink fine, but that predates the
   Aug 11 change, and no source states whether the skip is applied to the symlink's *name* (ours is a
   plain skill name — safe) or to the *resolved* path (potentially unsafe). **This is the one thing worth
   an empirical check before flipping**: create one dir symlink `~/.cursor/skills/<name>` →
   `~/.skillshub/<name>/…` and confirm it appears in Customize → Skills after a full IDE restart *and* in
   `/skills` in `cursor-agent`. Doing that check would move confidence to ~95%.
3. **Restart-persistence was the historical symptom** (150029, and the 2.5.22 tail report) — so the
   verification must include a **full quit-and-relaunch**, not just a window reload.
4. **Symlinked `SKILL.md` files are unverified** by any source. If Skills Hub ever symlinks individual
   files rather than the skill directory, keep that on copy mode; the evidence only covers directory symlinks.
5. **Windows junctions are separately unverified.** Every source above is macOS/Linux (one WSL2 report).
   The hybrid path falls back symlink → junction → copy, so the junction leg is untested against Cursor;
   if we want zero risk there, the flip could be gated to non-Windows.

If we want a zero-risk posture instead, **keep copy mode** — the cost is duplicated bytes and updates
that require a re-sync, and copy mode is known-good. But the evidence no longer supports the *reason*
written in the code comment, so at minimum that comment/flag should be re-labelled from "Cursor cannot
read a symlinked skills dir" to something version-scoped (e.g. "historically broken before Cursor 2.5;
kept on copy pending verification"), so the next reader doesn't inherit a stale fact.
