# Tagged command error contract (structured `CommandError`, prose frontend-only)

The Rust→TS error interface was "a string that may start with one of N pipe prefixes"
(`TOOL_NOT_WRITABLE|tool|path`), parsed in three frontend dialects and drifted (8 shipped
prefixes vs 5 documented, plus 2 more matched with `.contains()` inside `installer.rs`).
We replaced it: every `#[tauri::command]` returns `Result<T, CommandError>`
(`commands/error.rs`) — a serde internally-tagged enum (`{ code: "TOOL_NOT_WRITABLE",
tool, path }`) whose TS mirror is generated from the Rust enum (ts-rs originally; tauri-specta
since ticket 30 — see the amendment below), so both compilers check every variant. Core raises discriminable conditions as typed `SignalError` values
(`core/errors.rs`) through `anyhow` chains and they are recovered by downcast, never by
string matching. All user-facing copy (EN & ZH) is composed in the frontend's single
`describeCommandError` module; the backend composes no localized prose (it previously
emitted Chinese-only hints to all locales).

## Considered options

- **Compatibility `Display` keeping the prefix strings while call sites migrate** —
  rejected: every command already funneled through one `map_err`, so the flip was
  mechanical, and a compat layer would have kept the string dialect alive indefinitely.
- **Classifying GitHub failures frontend-side** — rejected: the backend has the full
  error chain; it classifies (`GitCloneFailed { kind }`), the frontend owns the copy.

## Consequences

- Adding an error variant = Rust variant + regenerated `src/bindings/` + a
  `describeCommandError` branch + i18n keys (EN & ZH). CI diff-guards the bindings.
  The runtime code whitelist in `src/commandError.ts` is compiler-derived from the
  generated union (`satisfies Record<CommandError["code"], true>`), so a new variant
  fails `npm run build` until the frontend handles it — no manual list to update.
- `CommandError::Other { message }` is the deliberate safety valve for unclassified
  failures; raw prose reaching users through it is a smell that a typed variant is due.

## Amendment (ticket 30, 2026): generator is tauri-specta; commands are typed end to end

ts-rs generated the DTO mirror but nothing typed the *calls*: `invoke("name", { args })` was
string-keyed at every site. Spiked `tauri-specta =2.0.0-rc.25` against the three hard shapes
(the `Channel`-streaming `sync_skills_to_tools`, this internally-tagged `CommandError`, and
`Option<T>`): all three export byte-for-byte equivalent unions (`{ code: "…"; …fields }`,
`T | null`, `Channel<SyncProgressDto>`), so it replaced ts-rs as the single generator.
`src/bindings/index.ts` now carries every DTO **and** one typed function per command; the
frontend seam `invokeTauri(name, ...args)` (`src/lib/tauri.ts`) is generic over that table, so a
misspelled command or a wrong argument fails `npm run build`, and `collect_commands!` in
`lib.rs` is the single registration list (an unlisted command has no binding).

Decisions that keep this ADR's contract intact:

- **Errors stay thrown.** specta's default `ErrorHandlingMode::Result` would wrap every command
  in `{ status: "ok" | "error", … }`; we use `Throw` so `runAction`/`toCommandError` keep owning
  the catch and `describeCommandError` keeps consuming the same union. The `satisfies
  Record<CommandError["code"], true>` guard still fails the build on an unhandled variant.
- **Numeric fidelity is explicit.** specta refuses `i64`/`u64`/`usize` by default; the builder
  opts into `number` (`dangerously_cast_bigints_to_number`) because every such field is a
  timestamp or count. specta types `f64` as `number | null` (serde_json writes NaN/∞ as null);
  the three finite-by-construction settings floats override to `number` per field.
- **Pinned, bumped deliberately.** `tauri-specta`/`specta` `=2.0.0-rc.25`, `specta-typescript`
  `0.0.12`; the RC line offers no semver guarantee, so bumps are reviewed through the bindings diff.
