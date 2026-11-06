# Prototype: the Harnesses pane, A vs C (Wave B, Q14 / Q25 / Q27)

Throwaway. Lives only on branch `proto/waveb-harness-cards`. It answers one question: once shared folders serve several harnesses, which Harnesses-pane model is clearer? It does **not** pick a winner.

## Run

```bash
npm ci            # once, in a fresh worktree
npm run dev:fixture
```

| Variant | URL |
|---|---|
| A: per-harness switches, folders derived | http://localhost:5175/?proto=harnesses&variant=a |
| B: flat folder list, paths first (extra) | http://localhost:5175/?proto=harnesses&variant=b |
| C: folder cards, one switch per thing written | http://localhost:5175/?proto=harnesses&variant=c |

- Add `&frame=full` to unclip the pane. The default is the real 880 × 640 Settings frame, with the pane scrolling.
- The floating bar switches variants (arrows, or ← → on the page) and toggles dark/light.
- Nothing is persisted, and it touches no disk: the route is a fixture-only branch in `src/main.tsx`, like `?specimen`.
- Code: `src/components/settings/prototype/HarnessesPrototype.tsx`.

## What it renders

Left: a copy of the Settings frame with the Harnesses pane in the variant's model. Right, top: the Add dialog's "Edit harnesses → Deploy after adding" popover (w-80) in the same model (Q25). Right, bottom: the derived state, meaning which folders get written, in which mode, and which detected harnesses see each one.

Data: the operator's machine.

- Selected: Claude Code, Codex and Pi. `.agents/skills` folds into Codex after #108.
- Also detected: Cursor, Gemini CLI, Cortex Code, Deep Agents Code (copy-only), Antigravity, and the reader-only harnesses Zed, Warp and fx.
- Not detected: OpenCode, Goose, OpenHands, Zencoder, Neovate, Augment, Dexto, crow-cli.

Read facts are hand-copied from `core/tool_adapters/mod.rs`. Every variant has the global preference ("Use shared folders when a harness can read them") and the dropdown ("When a harness reads both shared folders, use .agents / .claude"). Each folder has a Link/Copy control.

Folders are worked out as in Q15. A copy-only harness is served by its own folder unless its shared folder is set to Copy (Q21). Toggling the preference, the dropdown or a folder's mode regroups the folders live. The #116 duplicate lines are computed as Q20 specifies: one line per detected reader that sees two or more written folders, saying what its duplicate behaviour does.

## Screenshots

These are under `.scratch/round20/evidence/prototype/`, which is gitignored and local only.

- `a-dark-full.png`, `b-dark-full.png`, `c-dark-full.png`: the default state, with the pane unclipped.
- `a-dark-frame.png`, `c-dark-frame.png`: the real 640 px frame.
- `a-light-full.png`, `c-light-full.png`: light theme.
- `c-dark-both-claude.png`: dropdown set to `.claude`. Cursor moves to the `.claude` card.
- `c-dark-agents-copy.png`: `.agents` set to Copy. Deep Agents Code joins the `.agents` card and its own-folder card disappears.
- `c-dark-pref-off.png`, `a-dark-pref-off.png`: preference off.

## Trade-offs observed while building (no verdict)

**A (per-harness switches)**
- Every row needs a "via `<folder>` · also seen by …" line. A shared row's "also seen by" list runs to six or seven names, often wrapping, and repeats across Codex, Pi, Gemini CLI and Cursor.
- Every switched-off detected harness that reads a written folder carries a warning: "Off, but still sees your skills: .agents is on for Codex, Pi." With today's selection that is Cursor, Gemini CLI, Cortex Code and Deep Agents Code, so four of the eight detected rows are amber. Cursor and Deep Agents Code each get two such lines plus a duplicate line.
- A switched-on harness that shares a folder needs "Switching this off alone changes nothing on disk while Pi is on." The switch does not do what it looks like it does.
- Link/Copy cannot live on a shared harness's row, because it is a folder fact. A shows it only on own-folder rows. A shared folder's mode would need a separate home. The prototype has none, so A has no way to set `.agents` to Copy.
- Reader-only harnesses need a separate "no switch" section.
- Add picker: the familiar 2-column harness grid. With Claude Code, Codex and Pi checked, Cursor, Gemini CLI, Cortex Code and Deep Agents Code read "gets it anyway". The checkbox is a fiction for half the rows.
- Turning the preference off is simple in A. Each selected harness keeps its switch and gets its own folder. Nothing new is switched on.

**C (folder cards)**
- One switch per thing written, so what the switch does is literal. There are no "off but still sees" lines.
- The card title carries the harness names: "Codex, Pi, Cursor, Gemini CLI". A card's title changes when the dropdown or a mode changes, because harnesses move between cards. Watching that regroup live is informative, but the titles are unstable.
- Crowding sits in the shared cards: "Also seen by" (with duplicate notes), the reader-only list, a copy-only note ("Deep Agents Code needs copies …") and a nested "Not detected (N)" disclosure. With both shared folders on, nearly every line in both cards is amber, and the "Seen twice (4)" section repeats the same facts at pane level.
- Own-folder cards look like today's rows and stay calm.
- **Finding:** what does turning the preference off do to C's switches? The prototype carries "served" over: every harness served by an on folder gets its own folder switched on. On this machine that turns Pi, Cursor, Gemini CLI and Cortex Code on, writing 6 folders instead of 3 (`c-dark-pref-off.png`). C has no record of which harnesses the operator originally wanted, so preference-off either fans out to every reader or loses intent. A does not have this problem. The spec needs an answer here: a remembered per-harness intent under the cards, or an explicit choice on preference-off.
- Add picker: one checkbox per folder. The first two labels are names lists ("Codex, Pi, Cursor, Gemini CLI"), with "shared .agents · also Deep Agents Code, Zed, Warp, fx" under them. That is honest and short (4 rows instead of 8), but it is a single column of long labels, and a harness's name moves between rows as the folder rule changes.

**B (flat folder list)**: the most compact layout. It works only if the operator thinks in paths. The harness names sit in the secondary "Read by" line.
