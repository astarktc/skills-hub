# One ToolConfigModal, not two copies

Status: resolved

Type: grilling

## Question

`src/components/skills/modals/ToolConfigModal.tsx` (158 lines) is a literal copy of `src/components/projects/ToolConfigModal.tsx` (145 lines) — `git log --follow` resolves the former to the latter's history (copied in commit 698968b). ~120/150 lines byte-identical; every fix must land twice.

Decide, then implement (per map Notes):

- The single module's interface. Scan's sketch: `initialSelection: Set<string>` (caller computes baseline — the only genuinely different logic), `onConfirm(selected: string[])` (global caller closes over `scanSelectedOnly` itself), plus a `labels`/i18n-prefix prop. Variant extras: global has the scan-only checkbox; projects has the hardcoded 9-tool AGENTS-standard subtitle (which duplicates Rust's `AGENTS_STANDARD_KEYS` — take the list from backend data instead of prose?).
- Where the merged module lives (components/shared/?), and which i18n keys survive (`globalToolConfig*` vs `projects.toolConfig*` — both EN & ZH).
- Wiring diffs: App.tsx:1110–1150, 2583–2591 vs ProjectsPage.tsx:63–100, 311–318 (projects confirm also diffs against `state.tools` and triggers `update_project_gitignore`).

Context: scan finding 1 (incl. interface-diff table and both fact chains); report card #1. Independent of the Rust tickets — safe to work in parallel.

## Answer

Implemented and landed green on main as **`6ba5ae4`** "refactor: merge the two copied ToolConfigModals into one shared module (v-next ticket 04)". One grilling round (Q1–Q6), all recommendations accepted.

**Decisions:**

1. **Baseline interface (Q1)**: `savedSelection: string[] | null` — `null` means "nothing saved yet, default to installed tools"; the identical fallback logic lives once inside the modal. Global passes `globalSelectedTools` as-is; projects passes `state.tools.length ? state.tools.map(t => t.tool) : null`.
2. **Scan-only checkbox (Q2)**: explicit optional feature prop `scanSelectedOnly?: boolean` — when provided, the modal renders the checkbox (draft state stays inside; unmount-on-close resets it for free) and passes the draft as `onConfirm(selected, scanSelectedOnly?)`. No generic slot/toggle: one adapter means a hypothetical seam.
3. **i18n (Q3)**: resolved-strings `labels` prop (`title`, `description`, `confirmLabel`, optional `scanToggleLabel`); modal keeps `t` only for common chrome (`cancel`, `close`) and the shared `toolConfigDetectedOnly` key (new, top-level, EN+ZH). Deleted `globalToolConfigDetectedOnly` (EN+ZH) and `projects.toolConfigDetectedOnly` (EN). Side effect: the projects modal's detected-only toggle is now translated in ZH.
4. **Subtitle (Q4)**: backend-owned. `ToolInfoDto` gained `constituents: Vec<String>` (display labels of tools absorbed into a virtual group entry; empty for real tools), filled from the adapters in `get_project_tool_status`; the modal renders a subtitle for any tool with non-empty constituents — no `agents_skills` key check and no prose copy of `AGENTS_STANDARD_KEYS` in the frontend. Follows ticket 03's `shared_with` precedent: backend owns the invariant, frontend presents it.
5. **Location (Q5)**: `src/components/shared/ToolConfigModal.tsx` — first resident of `components/shared/`; both old files deleted.
6. **Unified drift (Q6)**: cancel button now `disabled={loading}` in both callers (invisible to projects, which passes `loading={false}`); redundant inline `installed.includes` collapsed.

**Term crystallised**: *constituent tools* / *virtual group* — recorded in the repo's first `CONTEXT.md` (created this session per domain-modeling's lazy-creation rule, alongside tool / shared skills dir group / managed skill / sync target).

**Gate**: `npm run version:check` exit 0; `npm run check` exit 0 (unmasked); `cargo test --all` 212 passed. Net **−272 lines**. No ADR: reversible, unsurprising, no real trade-off.

**Not done here**: the `<Modal>` shell extraction (~9 modals share the `.modal*` skeleton) — graduated from fog to its own ticket now that `components/shared/` exists.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
