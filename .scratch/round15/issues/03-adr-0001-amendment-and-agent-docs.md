# 03 — ADR-0001 amendment "report rows classify at settlement" + AGENTS.md / CONTEXT.md

Status: done — b09616f
Blocked by: 01, 02
Owner: parent (spec D8)

## Work

- `docs/adr/0001-tagged-command-error-contract.md`: add `## Amendment (round 15, 2026): report rows classify at
  settlement; reports cross the wire as themselves` — `CommandError` is a core type (`core/errors.rs`);
  classification-by-downcast happens where a fan-out row settles (chain richest, `last_error` already written)
  and at the command seam only for whole-command failures; no `*ReportDto` mirrors; counters are derived by the
  consumer; `DELETE_CLEANUP_FAILED` retired (delete returns its `RemovalReport`). Name the rejected alternative
  (generic `Report<E>` mapped at the seam).
- `AGENTS.md` **Error wire contract**: path `core/errors.rs`, the two-timing rule. **Target fan-out**: add the
  "core report types cross the wire directly; per-item failures are `CommandError` at settlement; no `*ReportDto`;
  counters derived in `reportOutcome.ts`" rule. **IPC types** bullet: unchanged (already sanctions core DTOs).
- `CONTEXT.md`: check **Artifact removal** / any term that names `DELETE_CLEANUP_FAILED` or the DTO mirrors.
- `CHANGELOG.md` `[Unreleased]` entry for 1.2.15.
