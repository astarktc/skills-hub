# Tagged command error contract (structured `CommandError`, prose frontend-only)

The Rust→TS error interface was "a string that may start with one of N pipe prefixes"
(`TOOL_NOT_WRITABLE|tool|path`), parsed in three frontend dialects and drifted (8 shipped
prefixes vs 5 documented, plus 2 more matched with `.contains()` inside `installer.rs`).
We replaced it: every `#[tauri::command]` returns `Result<T, CommandError>`
(`commands/error.rs` then; `core/errors.rs` since the round-15 amendment below) — a serde internally-tagged enum (`{ code: "TOOL_NOT_WRITABLE",
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

## Amendment (round 15, 2026): report rows classify at settlement; reports cross the wire as themselves

Every fan-out report (global sync, artifact removal, Refresh/Update + Propagation, Onboarding import,
Edit) used to exist twice: a core type whose failures were `anyhow::Error`s, and a `…Dto` mirror in
`commands/` with a hand-written total-match mapper that classified each failure with `from_anyhow`
and computed the report's counters. One propagation skip reason meant four edits; two counter rules
coexisted (core method vs mapper arithmetic); and one path still smuggled report data out as an
*error* (`DELETE_CLEANUP_FAILED { failures: Vec<String> }` — rendered prose, the shape this ADR
retired everywhere else).

Decided:

- **`CommandError` is a core type** (`core/errors.rs`, beside `SignalError`). Its TS name and every
  `code` tag are unchanged; `commands::CommandError` is a re-export.
- **Classification has two timings.** A fan-out row is classified **where it settles** — inside core,
  after the `{:#}` chain has been written to the row's `last_error`, where the chain is richest — so
  `RemovalTargetStatus::Failed`, `PropagationStatus::Failed`, `SkillRefreshStatus::Failed`,
  `BatchTargetStatus::{Skipped, Failed}`, `ImportGroupStatus::Failed` and `OriginalStatus::Failed`
  carry a `CommandError`. The command seam's `from_anyhow` remains only for errors that fail a *whole
  command*; core never classifies an error it then `?`-propagates. (`from_anyhow` also recovers an
  already-classified `CommandError` by downcast, for the one single-target caller — the unassign
  toggle — that raises a settled row failure as its command failure.)
- **Core report types derive `Serialize + specta::Type` and are what the command returns.** No
  `*ReportDto`, no mapper; serde tags (`status` / `scope` / `reason`, snake_case) live on the core
  enum. The wire vocabulary follows core names (`tool_key`, `RemovalReport.targets[].rows[]`) — no
  `#[serde(rename)]` to preserve a retired dialect.
- **Counters are derived by the consumer.** The frontend fold (`src/lib/reportOutcome.ts`) already
  walks every item; it derives `refreshed/failed/skipped/target_failures`, `removed/failed`,
  `imported/failed` itself. Core keeps its own counting methods for `Display` and tests.
- **Delete returns its `RemovalReport`**; `DELETE_CLEANUP_FAILED` is retired. The report's
  `record_deleted: false` plus its kept targets say what stayed (ADR-0002 unchanged in substance).

Rejected: a generic `Report<E>` mapped at the seam (`RemovalReport<anyhow::Error>` →
`RemovalReport<CommandError>`). It keeps classification at the seam but keeps a per-type `map_error`
transcription for five nested trees and leans on RC-line specta generics; it buys nothing the
two-timing rule does not.

Consequence: adding a report status or skip reason is one Rust edit plus the regenerated binding
plus the fold branch; `cargo test` and `npm run build` catch the rest.
