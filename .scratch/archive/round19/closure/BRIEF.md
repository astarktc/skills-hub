# Round 19 closure — extract-walk brief (shared by every slice child)

You are one of several read-only extraction children for closing the round-19 effort of Skills Hub
(`/Users/alexstark/Projects/skills-hub`). Read `AGENTS.md` and `docs/agents/issue-tracker.md` § Lifecycle → "When an
effort is finished" step 1 (Extract) first. Workflow step 1 is pre-approved: state your approach in one line and continue.

## Your job
Walk **only your slice's files** (named in your task) and find every line that is unfinished business:
- every `## Comments` entry, Result / "Deliberately left" / "Not done" / follow-up / deferred / later / candidate /
  "out of scope" / "known limitation" / BACKLOG-pointer line in the tickets;
- every review/audit finding (P0–P3, F-numbers, I-numbers, N-numbers, "should", "nit", "consider") — and for each,
  find out whether it was **applied** (a later fix commit / recheck says so, or the code shows it) or **left**;
- in `spec.md` (slice A only): every decision with a deferred half, every "later"/"1.3.x"/"19c"/"BACKLOG" line, and
  whether the 19a closure block already carried it.

For every **left** item, **re-derive its evidence from the current code on `main`** (file:line you opened yourself —
a carried line is a snapshot, not a fact). An item a later ticket already fixed is `closed` (name the sha/ticket and the
code line). Then check `.scratch/BACKLOG.md`: if an existing item already covers it, say `covered by #NN` (and whether
#NN's text needs amending).

## Output
Write exactly one file: `.scratch/round19/closure/extract-<SLICE>.md` (your slice letter). Nothing else is written —
no code edits, no ticket edits, no commits, no `git` writes. Do not run the app (`tauri:dev` mutates the operator's real
library); reading code, `rg`, `git log`/`git show` are all you need. Re-read your file after writing it.

Format — a header line with the slice and files walked, then one table per verdict group:

1. **Carry** — real, still open, not in BACKLOG: `| id | title (≤12 words) | source (path:line) | current evidence (path:line, what it shows) | proposed home: now / later / parked / future / ADR / CONTEXT / live-ticket | why it matters (1 line) |`
2. **Covered** — already a BACKLOG item: `| id | title | source | #NN | amend #NN? (what to add, or "no") |`
3. **Closed** — fixed since: `| id | title | source | closed by (sha / ticket) | code evidence |`
4. **Drop candidates** — not worth carrying: `| id | title | source | reason to drop by name |`
5. **Operator decisions** — only for items where the home or the drop is a *product* judgment you cannot make from the
   code: numbered, each `question — your recommended answer — one-line reasoning`.

Ids are `<SLICE><n>` (A1, A2, …). Be exhaustive but terse; one row per distinct item; merge duplicates across files and
cite every source. Prefer under-claiming "closed": if you could not find the code proving a fix, it is not closed.
End with a 3-line summary: counts per group, and anything that looked dangerous (data loss, destructive path, P0/P1 left open).
