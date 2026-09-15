# 11: Typed sync-engine errors — kill internal prose sniffing

Status: resolved

Type: task
Blocked by: None (can start immediately)

## What to build

The epic review's unanimous #1 finding (verified): `core/global_sync.rs::classify_sync_error` recovers `GlobalSyncError::TargetExists` by `msg.contains("target already exists")` — decoding the prose of our *own* `anyhow::bail!` calls in `core/sync_engine.rs` (lines 41/91/114 at `be9a74c`). The frontend's overwrite-retry UX (`TARGET_EXISTS` → `targetExistsDetail` toggle) hangs off this; rewording the bail silently degrades it to `Other` with no compiler signal. This violates the repo's own rule (AGENTS.md / ADR 0001: conditions are recovered by downcast, never by string matching).

- `sync_engine` raises a typed value for target-exists (a small typed error, or a `SignalError` variant in `core/errors.rs` — your call, downcastable through the `anyhow` chain), and `classify_sync_error` recovers it by downcast.
- The permission branch (`"os error 5"` / `"Access is denied"` / `"Permission denied"` sniffs) classifies *external* OS errors — replace with a `std::io::Error` downcast checking `ErrorKind::PermissionDenied` (keep a raw-OS-code check for Windows error 5 if `ErrorKind` doesn't cover a case; justify any remaining string check in a comment).
- Wire behavior must not change: the same failure scenarios must still produce `TARGET_EXISTS` / `TOOL_NOT_WRITABLE` on the wire. Existing Rust batch-sync tests must keep passing; add/adjust a test proving classification survives a message reword (i.e. it no longer depends on prose).

## Acceptance criteria

- [x] Zero `.contains()` prose matching against our own emitted messages anywhere in the sync classification path.
- [x] Permission classification is `io::Error`-kind-based, not substring-based (any residue documented inline).
- [x] Wire codes unchanged; existing + new `cargo test` coverage green.
- [x] `npm run version:check && npm run check` green (`> /tmp/gate.log 2>&1; echo $?`).

## Answer

Landed green in `de649b6` (Fable 5 low subagent; orchestrator-verified, rebased, combined-tree gate green). `sync_engine` raises typed `TargetExistsError { target }` (Display keeps the human prose); `classify_sync_error` has zero `.contains()` — target-exists via `err.chain()` downcast, permission via `io::Error` `ErrorKind::PermissionDenied` with a Windows-gated `raw_os_error()==5` residue documented inline (raw 5 = EIO on Unix, hence the gate). 3 new tests incl. the key regression: an unrelated error *containing* the literal prose stays `Other`. Wire codes untouched; bindings untouched. Discovered, left as-is (out of scope): analogous `"skill already exists"` prose + test sniff in `installer.rs` — candidate for the deferred installer-unification work.

## Comments

- 2026-09-15 — Metadata normalization only: resolved retained; plain Status moved to the first metadata line under the title. Existing resolution evidence is unchanged.
