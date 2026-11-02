# 32 — Vocabulary: Harness · Deploy · Update · state words · Library — across the glossary, every UI string, README, DESIGN.md, the fixture and the tickets

Status: ready-for-agent
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

(child writes: renamed keys, allow-list, screenshots, sha)

## Comments
