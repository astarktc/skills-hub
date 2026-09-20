# 03 ADR-0005: row is source of truth for content identity

Status: done — 49f2f9a
Lane: A
Source: BACKLOG #05

Write `docs/adr/0005-content-identity-row-is-source-of-truth.md` in the style of the existing ADRs (read 0001–0004 first). Decision: the skill row's `content_hash` is authoritative; hand-editing `~/.skillshub/<skill>` is unsupported; `same_content` trusts the row deliberately. Include the table: git-sourced → upstream/fork, rewritten by Update/Restore; local-path-sourced → source path, rewritten by Re-point/Update; imported/central-only (ADR-0003) → in-app Edit. Alternative rejected: reconcile-driven re-hash (reintroduces the read-time hashing round 10 removed). Add a one-line pointer in CONTEXT.md **Content identity**. Use the domain-modeling skill.

## Done when

ADR file exists, CONTEXT.md pointer added, no code change.

## Comments

- 2026-09-16 (lane A child) — Wrote `docs/adr/0005-content-identity-row-is-source-of-truth.md` (format per domain-modeling ADR-FORMAT: decision paragraph + Considered options + Consequences, matching 0002–0004). Includes the three-row source-kind → edit door → rewriter table, the accepted `overwrite_if_same_content` consequence from BACKLOG #05, and rejects reconcile-driven re-hash (the read-time hashing round 10 Q2 removed) plus two further alternatives. CONTEXT.md **Content identity** entry gained the pointer sentence + ADR link (and "hand edit (of the central copy)" in _Avoid_). No code change; cross-checked against `core/content_identity.rs` (`same_content` re-hashes only the target; managed side reads the row) and `global_sync.rs:113`.

- 2026-09-16 (parent) — closed `done — 49f2f9a`; review fixes in fa4ae9f; released as 1.2.13 (c927683).
