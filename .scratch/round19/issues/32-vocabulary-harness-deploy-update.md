# 32 — Vocabulary: Harness · Deploy · Update · state words · Library — across the glossary, every UI string, README, DESIGN.md, the fixture and the tickets

Status: done — 60dc80e
Blocked by: 33 (it rewrites strings 33 touches)
Blocks: 30, 31, 20, 21, 22, 23, 24
Source: grill 2026-11-02, D45 + D46 (operator: "go through our vocabulary from scratch and make sure we're perfectly
clear on what different terms mean"). Spec: D45, D46, D34 (the model the words describe).

## Problem

Three operator-facing words are wrong for the people who use the app:

- **"Tool"** is what every user of an AI coding agent calls a model-callable function (MCP tools, tool calls). The
  thing that reads a skills folder is a **harness** — the word the operator uses unprompted. ("Agent" was rejected: it
  collides with sub-agents and the model itself; "Configure agents" reads as managing sub-agents.)
- **"Sync"** names materialising a skill into a harness or project (Sync targets, Synced strip, "5 of 8 Tools synced",
  Sync this, Sync All, Auto-sync, Resync, Assign to project) — and reads as "pull updates". The word is **Deploy**.
- **"Refresh (all)"** and **"Update"** are one operation at two grains. The word is **Update** / **Update all**.

State words were misread in the think-aloud: "Stale" as "upstream has a newer version" (it is a deployed copy that
drifted from the Library's copy); "not synced" as a choice.

## The vocabulary (D45)

| Human-facing (canonical) | Replaces | Code keeps |
|---|---|---|
| **Harness** / harnesses; "Detected" stays | Tool, AI coding tool, tool | `ToolId`, `tool_adapters`, `ToolInfoDto`, DB `tool` columns, `TOOL_*` error codes |
| **Deploy** (verb) · **Deployed** (state) · **Deployment** (one skill in one place, global or project) | Sync, Synced, Sync target, assignment (as a UI noun) | `sync_*`, `SyncStatus`, `skill_targets`, `project_assignments` |
| **Deploy to project** · **Re-deploy** · **Deploy all** · **Deploy this** | Assign to project · Resync / Sync All · Deploy all (unchanged) · Sync this | — |
| **Auto-deploy** — "Deploy added and imported skills to your selected harnesses" | Auto-sync | `auto_sync_enabled` |
| **Update** (one) · **Update all** (batch) · Restore · Change source · Detach — unchanged verbs | Refresh (all) | `refresh.rs`, `refresh_managed_skills` |
| **App updates** (Settings pane) — unchanged | — | — |
| States: **Deployed** · **Not deployed** (hollow) · **Not deployed yet** (`pending`) · **Out of date copy** (`stale`) · **Copy missing** (`missing`) · **Failed** (`error`) · **Disabled** | synced / not synced / pending / stale / missing / error / disabled | the stored strings |
| **Library** (the managed skills; the area stays "My Skills") · **Library folder** · central copy (unchanged) | Skills Storage Path, central repo (UI) | `central_repo_path`, `central_path` |
| **Global** · **Project** (scope words, unchanged) | — | — |
| Health rows unchanged: Source missing · Central copy missing · Update failed · Edit conflict · Delete incomplete · Import failed | — | — |

Where a Harness first appears on a surface that a newcomer meets (empty state, Harnesses pane header, Add's scope,
Onboarding), the sentence names examples once: "harnesses — Claude Code, Cursor, Codex, …".

## Work

- **`CONTEXT.md`** — rewrite the Language section in the canonical words: rename the entries (Tool → Harness, Sync
  target → Deployment, Refresh (all) → Update all, …), keep every rule they state (after ticket 33's edits), add a
  **Code name** line to each entry whose code identifier differs, and keep the old term in each `_Avoid_` line so a
  reader coming from the code finds it. The 2026-11-02 "Vocabulary decisions" block at the top is folded in and removed.
- **`src/i18n/resources.ts`** — every string: nouns, verbs, titles, tooltips, confirmations, count lines, error copy in
  `describeCommandError`'s keys, `relative.*` untouched. Pluralisation keys follow (`_one` / `_other`). Keys are
  **renamed only when their name would mislead** (`syncedCount` → `deployedCount`); otherwise values change, keys stay —
  say which in `## Result`. `resources.test.ts` gains a **retired-words test**: no `en` leaf contains `Tool`, `Tools`,
  `Sync`, `Synced`, `Resync`, `Refresh` as a standalone word (case-insensitive, word-boundary; an allow-list for the
  few legitimate uses — "GitHub token", "tool" inside a quoted path — is explicit and short).
- **Components** — any literal label outside the catalog (there should be none; `resources.test.ts` guards the other
  direction), `aria-label`s built from keys, `title`s. `DESIGN.md` prose and examples. `README.md` (title line,
  supported-harnesses table heading, FAQ). `index.html` `<title>` is ticket 31's.
- **Fixture** (`src/fixtures/`) — scenario labels and any copy it renders itself.
- **Tickets 20–26, 30, 31** — a one-line "Vocabulary: read Tool/Sync/Refresh as Harness/Deploy/Update (ticket 32)"
  comment each; no rewrite of their bodies.
- **Not touched**: Rust identifiers, DB columns, error codes, `src/bindings/index.ts`, hook/module names, CHANGELOG
  history. AGENTS.md: only the sentences that quote UI copy; its architecture prose keeps code names.

## Proof

- `npm run test`: the retired-words test green and **meaningful** — show it fails when one retired word is reinserted.
- `grep -nwiE 'tool|tools|sync|synced|resync|refresh' src/i18n/resources.ts` → only allow-listed lines.
- Fixture, `?scenario=rich`, both themes, 1280×800: screenshots of My Skills, the inspector, Add's summary, the
  Projects matrix legend, Settings — every visible word is from the table above. Recorded under
  `evidence/32/`.
- `npm run version:check && npm run check` green.

## Result

Implemented, uncommitted, in worktree `/Users/alexstark/.t3/worktrees/skills-hub/t3-1af8d9fa`, branch `t3/1af8d9fa`
(base `3862c75`; the orchestrator reviews and commits).

**Renamed keys** (only where the key named the retired label; every other key keeps its name, value changed):
`library.refreshAll` → `library.updateAll` · `library.row.syncedCount` → `library.row.deployedCount` (its
interpolation `{{synced}}` → `{{deployed}}`) · `settingsAutoSync` / `settingsAutoSyncHint` → `settingsAutoDeploy` /
`settingsAutoDeployHint` · `skillsStoragePath` / `skillsStorageHint` → `libraryFolder` / `libraryFolderHint`. Callers:
`LibraryHeader.tsx`, `SyncedStrip.tsx`, `SettingsPage.tsx`. Keys such as `bulk.sync`, `outcome.done.synced`,
`strip.stale`, `assignProject.*` stay: they name code verbs / stored status strings (D45 "code keeps").

**Retired-words test** (`src/i18n/resources.test.ts`): no `en` leaf's rendered text (interpolation names like
`{{tool}}` stripped; keys never checked) contains, case-insensitive at word boundaries, tool · tools · sync · synced ·
syncing · syncs · resync · resynced · unsync · refresh · refreshed · refreshing · assign · assigned · assigning ·
assignment · assignments · unassign · unassigned · stale. A second test guards the check itself (catches "Sync to
Tools", "Refresh all", "Not assigned", "stale"; ignores keys, `{{tool}}`, "Toolbar", "GitHub token", "synchronous").
**Allow-list: empty** (`RETIRED_WORD_ALLOW_LIST = {}`) — no value needed a retired word: the GitHub token is "token",
paths in the catalog hold none, and "Configure Tools" became "Configure harnesses". Meaningful: with
`library.updateAll` set back to "Refresh all" the test fails (`library.updateAll: Refresh all`), restored it passes —
`evidence/32/retired-words-{red,green}.txt`.

**grep** `grep -nwiE 'tool|tools|sync|synced|resync|refresh|assign' src/i18n/resources.ts` → 88 lines, every one a
key (`tools: {`, `sync: "Deploy"`, `synced: "Deployed"`, …) or a `{{tool}}` / `{{tools}}` interpolation name; with keys
and interpolation names stripped (`sed -E 's/\{\{[^}]*\}\}//g; s/^([[:space:]]*)[A-Za-z_"0-9]+:/\1/' … | grep …`)
→ no match. Raw output: `evidence/32/grep-raw.txt`.

**Files changed**: `src/i18n/resources.ts` (every retired word; state words Deployed · Not deployed · Not deployed yet ·
Out of date copy · Copy missing · Failed · Disabled; "Sync target(s)" → global / project deployment(s);
examples "— Claude Code, Cursor, Codex, …" once on the empty library, discovered-skills row, Configure harnesses
description, Add's harness scope and the import summary) · `src/i18n/resources.test.ts` · the three renamed-key
callers · `src/lib/{reportOutcome,projectPresentation}.test.ts` (copy pins) · `src/fixtures/{registry,scenarios}.ts`
(the `.agents/skills (9 harnesses)` label, four fixture descriptions incl. `mirror-tools` → `mirror-scripts`, header
comment) · `src-tauri/src/core/tool_adapters/mod.rs` + `core/tests/{tool_adapters,tool_catalog}.rs` (the virtual
group's project label `".agents/skills (9 tools)"` → `"(9 harnesses)"` — a rendered string literal, not an
identifier; its doc comment and the two tests pinning it) · `CONTEXT.md` (rewritten in the new words, 26 **Code name**
lines, the "Vocabulary decisions" block folded in and gone, every former word kept in its _Avoid_ line) · `DESIGN.md`
(prose and quoted copy; token / tone names such as `stale` and stored status strings stay) · `README.md` (title line,
features, "Supported AI Coding Harnesses" table heading, `.agents/skills` paragraph, FAQ) · `AGENTS.md` (one quote:
"(Sync this)" → "(Deploy this)") · tickets 20–26, 30, 31 (one dated comment line each) · this ticket.

**Deliberately left**: `index.html` (ticket 31's); CHANGELOG; Rust identifiers, DB columns, error codes,
`src/bindings/index.ts` (no drift); hook / module / file names (`SyncedStrip.tsx`, `useSyncLight`, `AssignProjectModal`);
code comments outside the catalog; the primitives' own `*.test.tsx` fixtures ("Edit Tools", "Tool scope", "Refresh
all" are test-local labels, not product copy); AGENTS.md architecture prose ("not on this Tool", `Sync target`, …)
which keeps code names; health-row words (unchanged per the table). `projectsSurface.list.status.stale` reads "out of
date" (a project aggregate, not one copy). "Refresh view" in the Projects header became **Reload view** (a re-read,
not Update).

**Proof**: `npm run version:check && npm run check` green (see Comments for the run) — vitest 49 files / 1045 tests
(the two new tests included). Fixture `?scenario=rich` on port 5176, 1280×800, dark and light: My Skills (Grouped),
the inspector (`using-git-worktrees`: Failed / Deployed / Not deployed), Add's summary and its harness-scope popover,
the Projects matrix + legend (`skills-hub`), Settings (Auto-deploy, Library folder) —
`evidence/32/{dark,light}-0{1..6}-*.png`, with each surface's rendered text in `evidence/32/{dark,light}-texts.json`
scanned for the retired words: zero hits. Vite server stopped; ego task space closed.

## Comments
- 2026-11-02 — claimed and implemented in worktree `t3/1af8d9fa`. Gate after the last edit: `npm run version:check && npm run check` → EXIT 0 (Version OK 1.3.0; eslint clean; styles:check ok; vitest 49 files / 1045 tests; build ok; cargo fmt/clippy clean; cargo test 830 passed; `src/bindings/index.ts` unchanged). Log: `evidence/32/gate.log`.

- 2026-11-02 — **done.** Built in worktree `t3/1af8d9fa`, rebased onto `23938db` (one CONTEXT.md conflict: 32's rewrite vs the ticket-33 fix's sibling-record sentences — resolved by taking the rewrite and re-stating the nuance in the new words in the Auto-deploy, Update and Propagation entries). Orchestrator reviewed `git diff main...HEAD` (vocabulary-only hunks; the `(9 harnesses)` group label in `tool_adapters/mod.rs` kept) and re-ran the gate in the worktree: vitest 1053, cargo 837, clippy clean. Squash-merged.
