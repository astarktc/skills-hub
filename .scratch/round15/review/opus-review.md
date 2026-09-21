# Round 15 adversarial review — Claude Opus 5 (high), read-only, on `8cfe657..b42dccd`

Verdict: **fix-then-ship**. Disposition by the parent: Standards #1 + Spec #1 fixed together at the root by ticket 04
(toggle-unassign returns its `RemovalReport`; `from_anyhow` carve-out deleted); Standards #2 and #4 folded into
ticket 04; Standards #3 (log lines render `CommandError` JSON; DB `last_error` unaffected) and Spec #2 (`merge` keeps
the first scope; not logged today) and Spec #3 (disclosed extras) — accepted as-is, no follow-up.

## Standards

1. `should` — core propagates a wire type (`CommandError`, JSON `Display`) as an internal `anyhow` error in
   `project_sync::unassign_and_remove_artifacts`; only producer of the `from_anyhow` `CommandError` downcast carve-out.
2. `nit` — blanket `#[allow(unused_imports)]` on the `CommandError, GitCloneFailureKind` re-export.
3. `nit` — removal `Display`/`failures()` log lines render `CommandError` JSON for typed failures; no DB loss.
4. `nit` — `InvocationEditReport` still defined in `commands/`.

Verified clean: (a) no `from_anyhow`/`From` on a `?`-propagated error in core (18 call sites, all report rows);
(b) every `{:#}` chain still written to `last_error` before classification; (c) `commands/` is wiring; (d) no
`serde(rename)`; (e) counters gone from the wire (`ResyncSummary` is R1's carve-out); (f) shims respected; (g) i18n
EN+ZH complete; (h) no new `remove_path_any` caller; (i) `satisfies Record<CommandError["code"], true>` intact;
(j) no test-coverage regression — the three deleted mirror tests have stronger core replacements; (k) the
`CommandError`-first downcast is sound and cannot mask `FinalizeRollbackFailed`'s detail capture.

## Spec

1. `should` — project unassign-toggle failure copy regressed from localized framing to the raw `OTHER` chain
   (`ProjectsPage.handleToggleAssignment` → bare `notifyError`).
2. `nit` — `RemovalReport::merge` keeps the first input's scope; `merge([])` invents `EveryGlobalTarget`.
3. `nit` — disclosed extras: `Display` gained row counts, `RowRef::project_id()` deleted, `failures()` re-rendered.

Verified clean: D1 (TS union byte-identical minus the retired variant, 31→30 codes), D2 (six status families
classify at settlement), D3 (wire shape matches ticket 01's claims, checked against the bindings), **D4 (the fold's
`refreshCounts`/`removalCounts`/`importCounts` reproduce the deleted Rust mappers' rules exactly, including the
reassert term and both skip kinds)**, D5, D6 (delete fold + hook completion pinned end-to-end), D7, D8, R1;
out-of-scope respected; nothing asked for is missing. Gate green: cargo 648, vitest 361, bindings do not drift.
