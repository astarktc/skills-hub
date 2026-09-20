# 03 ADR-0005: row is source of truth for content identity

Status: open
Lane: A
Source: BACKLOG #05

Write `docs/adr/0005-content-identity-row-is-source-of-truth.md` in the style of the existing ADRs (read 0001–0004 first). Decision: the skill row's `content_hash` is authoritative; hand-editing `~/.skillshub/<skill>` is unsupported; `same_content` trusts the row deliberately. Include the table: git-sourced → upstream/fork, rewritten by Update/Restore; local-path-sourced → source path, rewritten by Re-point/Update; imported/central-only (ADR-0003) → in-app Edit. Alternative rejected: reconcile-driven re-hash (reintroduces the read-time hashing round 10 removed). Add a one-line pointer in CONTEXT.md **Content identity**. Use the domain-modeling skill.

## Done when

ADR file exists, CONTEXT.md pointer added, no code change.

## Comments
