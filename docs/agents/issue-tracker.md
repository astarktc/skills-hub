# Issue tracker: Local Markdown

Issues and specs for this repo live as markdown files in `.scratch/`.

## Conventions

- One feature per directory: `.scratch/<feature-slug>/`
- The spec is `.scratch/<feature-slug>/spec.md`
- Implementation issues are one file per ticket at `.scratch/<feature-slug>/issues/<NN>-<slug>.md`, numbered from `01` — never a single combined tickets file
- Triage state is one plain `Status: <value>` line at column 0, the first metadata line under the title (see `triage-labels.md` for the role strings and § Lifecycle for the transitions)
- Comments and conversation history append to the bottom of the file under a `## Comments` heading, each entry dated

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<feature-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path. The user will normally pass the path or the issue number directly.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `.scratch/<effort>/map.md` — the Notes / Decisions-so-far / Fog body.
- **Child ticket**: `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `Type:` line records the ticket type (`research`/`prototype`/`grilling`/`task`); a `Status:` line records `claimed`/`resolved`.
- **Blocking**: a `Blocked by: NN, NN` line near the top. A ticket is unblocked when every file it lists is `resolved`.
- **Frontier**: scan `.scratch/<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim**: set `Status: claimed` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.

## Lifecycle

The directory is the tracker state. Two files carry what the directories cannot say:

- **`.scratch/BACKLOG.md`** is the only cross-effort live queue. Every session starts by reading it, then the newest note (by filename date) in
  `.scratch/handoffs/`. An item is a numbered line with a source pointer (`effort/file:line`, review, or session); numbers
  are never reused. An item leaves the file **in the commit that closes it** — or the commit that opens the ticket/effort
  absorbing it, which says so. Handoffs point at BACKLOG.md; they never embed the queue.
- **`.scratch/archive/`** holds finished efforts, moved intact. **Live** = any `.scratch/` directory outside `archive/`
  (and outside `handoffs/`).

### Status transitions

- Task ticket: `ready-for-agent` → `claimed` → `done — <sha>`. Terminal alternatives: `wontfix — <reason>`,
  `superseded — <effort>/issues/NN-slug` or `superseded — BACKLOG #NN`. Pre-terminal triage values: `needs-triage`,
  `needs-info`, `ready-for-human`.
- Wayfinder ticket (has a `Type:` line): `claimed` → `resolved`.
- Terminal set: `done`, `resolved`, `wontfix`, `superseded`. Every status change gets a dated `## Comments` entry naming
  its evidence (sha, tag, file:line, or the superseding pointer). A module merely existing is not evidence.

### When an effort is finished

An effort is finished when every ticket carries a terminal status **and** its closure doc (`spec.md`, else `map.md`,
else `decisions.md`, else `closure.md`) carries a `## Closure — <date>` block naming the shipped release/sha and the
residue. Retire it with **extract-then-archive**, in this order:

1. **Extract.** Walk the map's Fog, every follow-up / not-scheduled / deferred / candidate / "later" line, every ticket
   comment, and every review "should" that was never applied. Each one becomes a BACKLOG item, a ticket in a live effort,
   or an ADR/CONTEXT edit — or is **dropped by name** in the closure block. Nothing is dropped silently.
2. **Prune walk evidence.** Delete logs, diffs, screenshots and JSON captures (`git rm` if tracked; `.gitignore` keeps
   `**/evidence/`, `**/worktrees/`, `*.log`, `*.diff` out of history). Reports and reviews stay.
3. **Move intact.** `git mv .scratch/<effort> .scratch/archive/<effort>`.
4. **Log it.** Add a row to `.scratch/archive/README.md`: `date · effort · what it was · residue (BACKLOG #NN / ADR / none)`.
5. **Rewrite inbound paths** in live files (BACKLOG.md, handoffs, AGENTS.md, CONTEXT.md, ADRs) to `archive/<effort>/…`.
6. **Commit** the moves and the BACKLOG edit together.

### Handoffs

A session that ends mid-effort writes `.scratch/handoffs/<YYYY-MM-DD>-<slug>.md`: where the work stands, the exact next
step, and pointers — never a copy of BACKLOG.md. The note with the latest filename date is the one the next session reads.
