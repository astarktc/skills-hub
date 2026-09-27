# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Tauri 2 desktop shell around a React 19 web UI; ships macOS ×2, Windows ×2, Linux. Native title bar. The design
language is web, not per-OS native.)

## Users

Primary: Alex — a solo developer who runs many AI coding harnesses (Claude Code, Codex, Pi, Cursor, Kimi, Amp, …)
across a Mac (primary) and a Windows host, curating a personal skill library (`~/Projects/agent-skills`) and using
Skills Hub as the deployment layer for it.

Also: people like Alex — anyone who curates a skill library across machines or harnesses, whether their primary use
is coding, knowledge work or creative work, done through AI coding harnesses for their customizability and
composability, and who prefers a GUI over a TUI (they operate their harnesses through a tool such as T3 Code).

No other audience is confirmed. The GitHub repo is public (2 stars, no forks, no issues); there is no external user
base to protect.

## Product Purpose

Install an AI Agent Skill once and keep it deployed, correctly, to every AI coding tool on the machine and to every
project that should have it — with a typed record of anything that failed.

Success for the operator: the current state of the library is understood at a glance; deploying and undeploying is
ergonomic; skills are easy to find; tailored groupings can be deployed from across multiple source repos; a skill
with decent bones can be edited (progressive disclosure, concise directed descriptions) while staying updatable from
its source; nothing under `~/.claude/skills`, `~/.pi/agent/skills`, … is ever touched by hand.

## Positioning

One skill library for every AI harness: deploy tailored sets globally or per project, and tune any skill locally
without losing its upstream updates.

## Operating Context

- Scene: Alex at his desk, day and evening, on a Mac, the app docked beside a terminal and T3 Code, both dark; light
  mode exists for bright rooms and the Windows host.
- A daily-driver utility used in short sessions many times a day; long-running batch actions (Refresh all, sync,
  import) stream progress and settle with a report.
- Sources are GitHub repos (whole repos or subpaths, one repo often contributing many skills), local folders, and
  onboarding imports of skills already present in a tool's directory.
- Deployment targets are Tools (harnesses) — a registry of ~45 adapters with global skills dirs, detect dirs and
  project-relative dirs — and Projects (repos on disk), each with an assignment matrix of skills × tools.
- Sync materialises symlink → junction → copy; Propagation keeps every target of a changed skill in line; every
  fan-out answers a per-target report (failures and skips are data, not errors).
- Two windows into the same state exist for the operator: this GUI and the harnesses' own skill directories.

## Capabilities and Constraints

Confirmed capabilities: add from GitHub URL / local folder / Explore catalog; per-tool global sync with overwrite
policy; project assignment matrix, bulk assign/unassign, resync; Refresh (all) and per-skill Update / Restore /
Re-point; direct Edit with replay on Update; onboarding import scoped to installed or selected tools; delete with
artifact removal; settings (theme light/dark/system, zoom, central repo path, GitHub token, scan scope); update
checker; notification history.

Constraints future work preserves:
- Sync semantics, the Tool adapter registry, the mutation guard, Propagation, artifact removal, and the typed
  `CommandError` wire contract (ADRs 0001–0005) are untouchable by UI work.
- No state-management library; per-world hooks + `App.tsx` binder; hook/pure-function tests only, no component
  rendering tests; every backend call through the typed `invokeTauri` seam.
- All copy lives in the i18n catalog (`t()` everywhere). Locale: **EN only** (decision 2026-09-24: ZH removed as
  unverifiable upstream inheritance; broader localisation is a parked backlog item).
- Five shipping targets; releases are built on GitHub.
- Keyboard zoom hotkeys (Cmd/Ctrl +/−) stay.

Terminology (see `CONTEXT.md`): Skill, Tool (harness adapter), Project, Sync target, Propagation, Edit, Re-point,
Onboarding import, Refresh, Explore.

Undecided product facts: whether a home/overview ("doctor") surface exists (deferred with backlog #58); scheduled
refresh (#53); a CLI (#54); a declarative library manifest (#55).

## Brand Commitments

- Name: Skills Hub. Voice: plain, precise, unhurried; a tool, not a brand.
- Practical, not flashy: no marketing-page devices; "beautiful" means quietly right, never over the top.
- Quality-bar peers (binding for craft level, not for copying a look): T3 Code (density, dark palette, typography,
  tone — colour only where it means something), Linear (cleanliness), Raycast settings panes. The upstream
  `qufei1993/skills-hub` is a domain reference; a specific item, concept or page may be borrowed when it is genuinely
  the ideal choice, with our own spin.
- Dark is the canonical designed-first theme; light is a first-class derivation.
- Motion: crisp and near-invisible on frequent actions; deliberate delight only on rare moments (first run, a large
  import completing, an install landing).
- Icons: Lucide; brand marks via simple-icons.

## Evidence on Hand

- Operator frictions (2026-09-24): visual clutter on skill cards (many tool pills per skill; repo pills repeated
  inside repo-grouped lists); grouped layout wastes space when one huge repo sits beside several single-skill repos;
  no bulk assignment; no project assignment from the My Skills page; Explore "could be better".
- `docs/design-inputs/2026-09-24-upstream-v0.10-and-field-survey.md` §E — UI borrows judged worth having.
- `.scratch/BACKLOG.md` — numbered, source-pointed queue.
- Real library: ~80 skills across ~10 source repos, 8+ installed tools, several projects on the operator's machine.
- No usability studies, analytics, testimonials or external users exist; do not fabricate them.

## Product Principles

1. State at a glance — the library's health is visible without opening anything.
2. Deploy and undeploy are ergonomic — one selection model, several entry points (row, detail, group, bulk bar).
3. Grouping serves finding — sources organise the library, but never at the cost of density or provenance.
4. Edits survive updates — tuning a skill never severs it from its source.
5. Every failure is shown, typed, and recoverable — never a silent success.

## Accessibility & Inclusion

Hard gate for UI work (decision 2026-09-24): keyboard-only completion of every core flow, visible focus, WCAG AA
contrast in both themes, `prefers-reduced-motion` honoured, minimum window 960×640 with one structural collapse.
