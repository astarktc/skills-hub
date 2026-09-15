# 10: Prefactor — compiler-check the error-code whitelist

Status: resolved

Type: task
Blocked by: None (can start immediately)

## What to build

Make it impossible to add a `CommandError` variant that the frontend silently mishandles. Today `src/commandError.ts` keeps a hand-typed `COMMAND_ERROR_CODES: ReadonlySet<string>` next to the generated union; a new Rust variant compiles clean everywhere while `isCommandError` rejects its code, so `toCommandError` falls through to `{ code: "OTHER", message: String(err) }` and the user sees `[object Object]`.

From the 3-model epic review (verified): the ADR's "adding a variant" checklist omits this list entirely.

- Derive the set from the generated union so the TypeScript compiler forces an update, e.g. a `satisfies Record<CommandError["code"], true>` map (or equivalent) whose keys feed the runtime `Set`. Adding a 13th variant to `src/bindings/CommandError.ts` must fail `npm run build` until the frontend handles it.
- Update `docs/adr/0001-tagged-command-error-contract.md`: the variant checklist gains this step (or, better, notes the list is now compiler-derived and needs no manual step).
- `isCommandError` is exported but has no importers outside its own module and its test. Un-export it if the test can exercise it through `toCommandError`; otherwise keep the export and leave a one-line comment saying the test is the only consumer.

## Acceptance criteria

- [x] `COMMAND_ERROR_CODES` (or its replacement) is compiler-derived from `CommandError["code"]` — no free-floating string list.
- [x] Manually adding a fake variant to the union locally breaks the build (verify, then revert the experiment).
- [x] ADR 0001 checklist updated.
- [x] `npm run version:check && npm run check` green (check via `> /tmp/gate.log 2>&1; echo $?` — pipes mask exit codes).

## Answer

Landed green in `18c1aa6` (fast-forward merge to main; implemented by a Fable 5 low-effort subagent, verified by orchestrator). `COMMAND_ERROR_CODE_MAP` is `as const satisfies Record<CommandError["code"], true>` — the runtime `Set` derives from its keys, so a new Rust variant now fails `npm run build` twice over (TS2741 missing map key + TS2366 non-exhaustive `describeCommandError` switch; proven experimentally with a fake variant, then reverted). `isCommandError` un-exported; its behavior is covered through `toCommandError` tests. ADR 0001's variant checklist now records the whitelist as compiler-derived (no manual step).

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
