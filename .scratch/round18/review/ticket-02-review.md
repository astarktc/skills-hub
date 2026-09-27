# Ticket 02 adversarial review — `2b60c7f`

## Verdict

**fix-then-ship** — two blocking findings, three should-fix findings.

Reviewed exactly `2b60c7f~1..2b60c7f` (`2b60c7f9ac04e9fede4df6f56b5f3c876d9993ee`). All file/line references below refer to that commit. Standards: AGENTS.md and ADR-0001. Spec: ticket 02 **Work**, not its Result. No source edits, commits, staging, app startup, or real-keychain calls were performed. Concurrent ticket-04 work and the subsequent ADR commit are not findings.

The main architecture is sound: an injected credential store, no secret in the settings response, generated IPC types, and an automatic read-back-before-delete migration. However, failure recovery can resurrect an explicitly removed credential, and the successful migration leaves the token recoverable from the database file.

## Blocking findings

### B1 — P1 · A pending migration overrides later Save/Remove actions

**Where:** `src-tauri/src/core/settings.rs:345–348`, `:405–420`. **Axis:** Spec / credential lifecycle correctness.

A failed startup migration deliberately preserves the legacy `github_token` row. But a subsequent successful `SettingUpdate::GithubToken` only changes the credential store; it never settles that pending row. The next launch unconditionally writes the old row into the keychain.

Concrete sequence:

1. SQLite holds token A; startup keychain write or read-back fails, so A stays in SQLite.
2. The operator later permits access and saves token B. Save succeeds and Settings reports a token saved; SQLite still holds A.
3. Restart: migration overwrites B with A, verifies A, and deletes the row.

Likewise, if migration wrote A but its read-back failed, the keychain can contain A while the row remains. A later successful Remove deletes the keychain item only; the next startup recreates it from SQLite. This also keeps plaintext around after an apparently successful manual replacement/removal. Existing tests only apply settings to databases without a pending legacy row.

**Concrete fix:** make explicit token replacement/removal settle the pending legacy migration as part of the same core operation. For replacement, verify the new credential before removing the legacy row; for an explicit removal, ensure no legacy row can restore the credential. Propagate cleanup failures rather than report unconditional success. Coordinate this with B2's secure cleanup. Add regression tests for migration failure → successful replacement → restart, and partial migration/read-back failure → successful removal → restart. Cover cleanup failure as well.

### B2 — P1 · SQL DELETE does not remove the plaintext token from the database bytes

**Where:** `src-tauri/src/core/skill_store.rs:532–536`; called by `src-tauri/src/core/settings.rs:420`. **Axis:** Spec / security outcome.

The migration's cleanup is a plain `DELETE`. `SkillStore::with_conn` (`:1430–1440`) does not enable secure deletion or vacuum the database. With this project's bundled SQLite, `secure_delete` is **off**. The logical row disappears, but the plaintext secret remains in the freed portion of the database file. Copying or inspecting the migrated DB can therefore still recover it, despite the stated goal of moving the token out of that database.

**Fresh reproduction:** compiled a source-free stdin Rust probe against the project's built `librusqlite-b62fa4a7a2689373.rlib`, using the same settings-table schema, connection pragma and DELETE statement. Inserted only a synthetic sentinel, deleted its row, closed the connection, and scanned the DB bytes. Output:

```text
bundled SQLite: secure_delete=0, logical_rows=0, secret_bytes_remain=true
```

No operator DB or token was read. The synthetic DB was removed. Probe binary is in ignored `.scratch/round18/review/evidence/token-delete-probe`.

**Concrete fix:** implement secret-specific DB cleanup, with `secure_delete=ON` on the connection performing the deletion and an appropriate one-time compaction strategy to remove historical freed copies. Handle cleanup/retry semantics explicitly; do not mark the security migration complete while required cleanup failed. Account for journals/WAL if enabled. Add a file-backed test that checks both logical absence and absence of the synthetic secret in the live database bytes. This does not promise erasure from pre-existing backups, APFS snapshots, or SSD remnants; document those separate limits.

Primary references: [SQLite secure_delete](https://www.sqlite.org/pragma.html#pragma_secure_delete), [VACUUM — deleted content can otherwise be recovered](https://www.sqlite.org/lang_vacuum.html).

## Should-fix

### S1 — P2 · An unreadable credential is reported as definitely absent

**Where:** `src-tauri/src/core/settings.rs:180–185`, `:380`; `src/components/skills/SettingsPage.tsx:330–341`. **Axis:** Standards (typed operator-caused errors) / Spec (truthful presence).

`load_settings` swallows `CredentialStoreUnavailable`, returns `github_token_set: false`, and logs only. Settings then asserts **“No token saved”** and hides Remove even when a token exists but access was denied. This also affects the read after a successful token write: if writing succeeds but reading fails, the command reports success with false presence, the frontend clears the draft, and the saved credential is represented as absent. This is not the same as the intentionally unauthenticated acquisition fallback.

**Concrete fix:** do not encode read failure as absence. Surface the typed error through the settings path, or explicitly represent unavailable/unknown credential status alongside the boolean so the rest of Settings can remain usable. Show actionable recovery instead of “No token saved”; do not claim a verified post-write state when verification failed. Add existing-token/read-denied and write-success/read-denied tests. The current test explicitly expecting false on read failure should change.

### S2 — P2 · Linux recovery instructions omit the restart required by keyring v1

**Where:** `src-tauri/src/core/credentials.rs:58–59`; `src/i18n/resources.ts:351–352`. **Axis:** Spec / recovery behavior.

The error copy tells the operator to start Secret Service and “try again.” In keyring **4.2.0**, the first store initialization result is permanently cached in `static SET_CREDENTIAL_STORE_RESULT: LazyLock<Result<()>>`. If initialization fails because the session bus/Secret Service is unavailable, subsequent `Entry::new` calls return `NoDefaultStore` without retrying initialization. Constructing another stateless `KeyringStore` does not reset it. Starting the service and pressing Save again in the same process cannot recover that failure.

**Concrete fix:** at minimum, tell the operator to restart Skills Hub after starting the missing service, and include this limitation in the Linux release note. Prefer preserving `Entry::store_status()`'s original diagnostic rather than replacing it with the generic “No default store” error. If in-process recovery is desired, use explicitly managed backend initialization rather than the v1 global wrapper. Validate cold-start-without-service → start-service → retry/restart on Linux.

Primary sources: [keyring 4.2.0 v1 source](https://docs.rs/keyring/4.2.0/src/keyring/v1.rs.html) (`Entry::new`, `store_status`, `SET_CREDENTIAL_STORE_RESULT`); [Secret Service initialization](https://docs.rs/zbus-secret-service-keyring-store/1.0.1/src/zbus_secret_service_keyring_store/service.rs.html) (`SecretService::connect`).

### S3 — P2 · A slow Save clears a newer unsaved draft; credential writes can overlap

**Where:** `src/components/skills/SettingsPage.tsx:63–67`, `:317–334`. **Axis:** Spec / frontend behavior.

There is no pending-state guard. While saving draft A awaits IPC/keychain access, the user can type draft B. When A resolves, `setTokenDraft("")` discards B even though B was never saved. Save/Enter/Remove also remain available during the request, allowing overlapping mutations of the same credential and out-of-order presence responses. OS prompts make these waits materially longer than ordinary local setting writes.

**Concrete fix:** make token mutations single-flight and disable the draft, Save, Remove, and Enter submission while one is pending, or track draft revisions and clear only the submitted draft while serializing writes. Prefer owning the single-flight action in the hook so it can be tested without component rendering. Add a deferred-promise test covering repeated submissions/removal and preservation of a newer draft if editing remains enabled.

## Nits

- **N1 — missing focused regression coverage.** `src-tauri/src/core/tests/settings.rs:493–576` covers set failure and read-back mismatch, but not **set succeeds → get returns Err**, nor DB cleanup failure. `src-tauri/src/core/settings.rs:205–212` introduces an untested `onboarding_scan_selection` entry point; add the off/on × absent/empty/valid/corrupt-selection table. `src-tauri/src/core/errors.rs:508–510` has the new mapping but no Rust test specifically recovering this variant through an anyhow context and asserting its serialized tag. Existing generic downcast tests and the frontend formatting test do not exercise that entire new path. These are coverage gaps, not evidence that those branches currently fail.
- **N2 — stale descriptions.** `src/hooks/useSettingsState.ts:95–98` still says the token writes on every keystroke; it now writes on explicit Save. Update that comment. Ticket Result says “uncommitted; working tree only” and that AGENTS.md does not contain the credential rule, although this commit includes that rule; correct those historical-status statements when recording the review fixes.

## Verified claims

### Implementation and spec claims

| Claim | Verification / qualification |
| --- | --- |
| New `CredentialStore: Send + Sync`, production adapter, test-only memory adapter; stable coordinates | Read `core/credentials.rs` and `core/mod.rs` at the commit. `MemoryStore` and its impls are `#[cfg(test)]`; tests pin `com.skillshub.app` / `github_token`. Round-trip and failing-memory-store tests pass. `NoEntry` alone maps to absent/no-op deletion; other errors become the typed signal. |
| Commands construct the adapter; core/tests never reach for the real keychain | Commit-scoped search for `KeyringStore` found the only construction call in `commands::credential_store`. Startup uses that seam. All changed core/test call sites receive explicit stores; no test constructs `KeyringStore`. |
| Token readers retain their names and receive injected credentials | Traced installer listing/selected install/Explore, Refresh and Re-point. The test-only legacy install helper now uses `HttpGithubApi::new(None)`. `github_download`'s failure test uses the failing memory store and asserts no bearer header. |
| AppSettings returns only presence; SettingUpdate stays compatible | Rust has nonoptional `github_token_set: bool`; generated TS has `github_token_set: boolean`, no secret response field. `SettingUpdate` remains `{ key: "github_token", value: string }`. Save necessarily sends the draft frontend→backend; no backend→frontend secret echo. |
| New values are stored only in the keychain | Token update branches invoke `set`/`delete`, not `set_setting`. Passing tests verify trimming, clear, and no newly written DB row. **Pending old rows are not settled (B1), and migration doesn't purge database bytes (B2).** |
| Normal migration is read-back-before-delete and idempotent | Control flow is set → get → exact comparison → delete. No row skips all keychain calls; blank row skips the keychain. A set/get error or mismatch exits before DELETE. Happy-path rerun, blank, failing-store and mismatch tests pass. This does **not** prove recovery after later user actions (B1), or physical deletion (B2). |
| Startup ordering and “log once” | In `lib.rs:98–124`, legacy DB adoption and `ensure_schema` precede migration; migration precedes `app.manage`/normal command access. One match arm logs a successful move or blank drop; no row logs nothing on future launches. Failure warns once per migration attempt/launch. No explicit retry loop. |
| Error path is wired end-to-end | `SignalError` → `CommandError::from_anyhow` downcast → `From<SignalError>` → generated `CREDENTIAL_STORE_UNAVAILABLE` union → whitelist/`describeCommandError` → EN catalog. The generic classifier preserves typed signals through contexts. Frontend branch test passes; new-variant-specific Rust context/tag test is absent (N1). Settings reads deliberately suppress this error (S1). |
| No token interpolation in new logs/errors | Examined migration logs, settings warnings, adapter error conversion, classifier and token consumers. No explicit token value is interpolated or serialized in responses. Mismatch diagnostics are constant text. Keyring-core's `Display` for `BadEncoding` does not print its secret bytes, and set/get debug logging prints credential identity, not password contents. This is a source review, not an exhaustive runtime audit of every platform diagnostic. |
| Refresh can continue unauthenticated | It reads the credential once before the batch through `github_token_or_none`, preserving the prior best-effort acquisition policy. This is **not** plaintext fallback. I do not recommend failing unrelated/local refresh rows merely because optional authentication is unavailable. Downstream acquisition failures still settle as report data; the settings recovery surface must be truthful (S1). |
| Onboarding selection behavior remains the same | Compared old `onboarding_scan_scope` against the new helper. Both use `read_tool_selection(...).configured()` and the same `read_bool` default. Flag off → Installed; corrupt/absent selection → Installed; selected empty array with flag on → Selected(empty); unknown tool keys are pruned by the same reader. Avoiding unrelated settings/keychain reads is intentional. No dedicated helper test (N1). |
| Frontend input is write-only; draft clears after save; clear uses same command | Input starts at `""`, type password, never populated from backend. Hook holds only presence and returns boolean success. Save calls the same `SettingUpdate` as Remove (blank). Success clears the submitted draft in the ordinary case; failures retain it. Concurrent edits have S3. Hook save/replace/clear/failure tests and error formatting test pass; no component-rendering tests are claimed. |
| Sync test change is only its fixture field | Exact diff in `useSyncOrchestration.test.ts` is `github_token: ""` → `github_token_set: false`. App binder change is the corresponding prop rename. |
| CHANGELOG/macOS note exists | The ticket Result contains release-ready security, recurring macOS prompt, and Linux service notes. **CHANGELOG.md itself is not changed by this commit.** Deferral to ticket 10 is explicit and reasonable; the release owner must carry those notes forward, amended for the findings. |
| Binding generated and committed | `cargo test --all` ran `bindings_export::export_bindings`; subsequent `git diff --exit-code 2b60c7f -- src/bindings/index.ts` was empty. This proves reproducibility, not how the implementer originally produced the file. |

### Registry / crate facts (primary-source verified)

- `Cargo.toml` specifies **`keyring = "4.2"`**, a caret range (`>=4.2.0, <5.0.0`), not an exact `=4.2.0` manifest pin. The committed lockfile resolves **4.2.0**. The release was published **2026-08-29**, confirmed by [upstream v4.2.0 release/changelog](https://github.com/open-source-cooperative/keyring-rs/releases/tag/v4.2.0). A root `CHANGELOG.md` fetch returned 404; upstream release notes supply the changelog.
- The [versioned README](https://github.com/open-source-cooperative/keyring-rs/blob/v4.2.0/README.md), [versioned Cargo.toml](https://github.com/open-source-cooperative/keyring-rs/blob/v4.2.0/Cargo.toml), [4.2.0 docs](https://docs.rs/keyring/4.2.0/keyring/), and locally downloaded 4.2.0 `Cargo.toml`/`src/v1.rs` agree: **default = v1**, enabling Apple `keychain`, Windows native store, and zbus Secret Service. The target-specific dependency tables select the corresponding platform; no extra platform feature is needed by this application.
- v1 selects macOS Keychain Services, Windows Credential Manager, and non-iOS/non-Android Unix Secret Service. The Secret Service dependency enables `crypto-rust`. `cli`-only DB/sample/keyutils alternatives are not enabled, and v1 does not fall back to them. Missing-service initialization returns an error, with the lifetime caveat in S2.
- [Entry v1 API](https://docs.rs/keyring/4.2.0/keyring/v1/struct.Entry.html): `Entry::new(service, username)` is the correct argument order; `delete_credential`, not `delete_password`, is correct for 4.2. [NoEntry](https://docs.rs/keyring-core/1.0.0/keyring_core/error/enum.Error.html) means no matching underlying credential (never set/deleted), not an inaccessible store.
- macOS replacement is supported, not an unconditional duplicate-item insert: the locked [apple-native-keyring-store 1.0.2 implementation](https://docs.rs/apple-native-keyring-store/1.0.2/src/apple_native_keyring_store/keychain.rs.html) calls `SecKeychain::set_generic_password`; the locked [security-framework 3.7.0 implementation](https://docs.rs/security-framework/3.7.0/src/security_framework/os/macos/passwords.rs.html) finds and updates an existing item, otherwise adds it. A denied lookup may lead to an add error, but that still propagates and does not authorize legacy-row deletion.
- The ad-hoc-signing warning is justified, not a crate backend omission. [Apple TN2206](https://developer.apple.com/library/archive/technotes/tn2206/_index.html) describes Keychain ACL tracking by designated requirement. GitHub CLI's maintainer-authored [macOS keyring security note](https://github.com/cli/cli/blob/trunk/docs/macos-keyring.md) explicitly explains that unsigned/ad-hoc native clients are identified by `cdhash`, which changes on upgrade and invalidates prior ACL authorization. A stable service/account locates the item; it does **not** prevent re-prompts across ad-hoc builds.

### Fresh command results

- `cd src-tauri && cargo test --all`: **692 passed**, zero failed; main/doc tests passed.
- `bun run test` (the project's `vitest run` script): **17 files / 426 tests passed**. This includes concurrent fixture tests; do not confuse it with the implementer's earlier 16/410 count.
- `bun run build`: **`tsc -b` and Vite build passed**. Existing chunk-size/mixed-static-dynamic-import warnings remain; not attributed to this commit.
- `bun run version:check`: **Version OK (1.2.17)**.
- `bun run lint`: passed.
- `cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings`: passed.
- Regenerated bindings match the reviewed commit byte-for-byte. Tests ran in the shared working tree; the reviewed Rust/settings/binding files matched the commit, while unrelated ticket-04 web files were in flight.

## Not verifiable here

- Real macOS prompt timing, Deny/Allow/Always Allow behavior, locked-keychain recovery, and installed ad-hoc release upgrade behavior. No live keychain or application startup was exercised. The migration is synchronous in `.setup`, so startup waits for the native operation/prompt; source shows no infinite retry loop, but cannot bound OS prompt duration. Subsequent Settings/acquisition reads can prompt again after denial.
- Windows/Linux compilation and actual credential stores, including unavailable D-Bus/Secret Service and restart recovery. Source/dependency facts were verified, not platform runtime behavior.
- Complete erasure from historical backups, snapshots, other legacy-identifier database copies or storage-device remnants. B2 establishes recoverable plaintext in the **current** migrated DB, without relying on those broader concerns.
- Visual/UI smoke and native IPC prompt interaction. Unit tests verify hook logic; S3 is a code-level asynchronous interleaving, not a recorded UI reproduction.
- The implementer's historical gate run or whether generated bindings were originally hand-edited. Fresh gates and a fresh generator comparison are the evidence above.

## Re-review of bc84841

**Verdict: fix-then-ship.** B1, S1–S3 and the requested N1 coverage are closed. B2's happy path is fixed, but post-DELETE failure recovery and WAL checkpoint completion are not. N2's original statements are corrected, with one new stale status nit.

Reviewed `bc84841~1..bc84841` (`bc8484135d8adb8fee49c1efca5bc7e1385552e8`) against the prior findings, the ticket's **Review fixes**, and the orchestrator's stated decisions—not alternative credential-store designs. Read `git show` and the diff. Source/bindings matched that commit before and after verification. No product source edits, staging, commits, app startup, browser work, real-keychain calls or operator-database access. Only this report is edited; synthetic probe binaries/logs are in ignored `review/evidence/`. Concurrent ticket-04 review output is unrelated.

### Per-finding disposition

| Finding | Closed? | Evidence | Residual risk |
| --- | --- | --- | --- |
| **B1** | **Yes, as decided** | `core/settings.rs:394–413`: Save checks for the legacy row, writes and verifies the replacement before secure cleanup; Remove deletes the keychain item before cleanup. Every error propagates. Passing tests at `core/tests/settings.rs:733,772,805,854` cover replacement/restart, partial-migration removal/restart, read denial and failed row deletion. | If keychain deletion succeeds but DELETE fails, the old row can still resurrect the token on restart. This is explicitly documented in `settings.rs:378–393` and the ticket's B1(c) residual; the action returns an error, not success. This is the accepted behavior, not a reopened B1. Compaction failure is separately B2 below. |
| **B2** | **No—partial** | `core/skill_store.rs:551–563` correctly sets `secure_delete=ON` **before DELETE on the same connection**, then VACUUMs. Migration returns `Migrated` only after that call returns. The file-backed raw-byte test at `core/tests/settings.rs:642` passes. | A failure after DELETE loses the cleanup retry signal; a busy WAL checkpoint is silently accepted. See B2a/B2b below. |
| **S1** | **Yes** | `settings.rs:121,181–187`, generated `src/bindings/index.ts:157`: `Option<bool>` / required `boolean \| null`. Hook adopts null unchanged and warns/keeps the draft on an unconfirmed echo (`useSettingsState.ts:272–297`). `SettingsPage.tsx:339–356` handles null before boolean rendering and retains Remove. Rust and hook denial tests pass. | No post-load path found coercing null to false. The initial pre-load placeholder is false, not a coercion of an unreadable response. Ticket-04's `fixtures/model.ts:438` false remains type-valid. |
| **S2** | **Yes (source/copy)** | `credentials.rs:66–73` preserves `Entry::store_status()`'s initialization diagnostic for `NoDefaultStore`. Rechecked keyring 4.2.0 `src/v1.rs:47–68,107`. Recovery strings and the amended Linux release note say restart. | Actual Linux cold-start/service-start/restart behavior was not exercised; no real credential store was accessed. |
| **S3** | **Yes** | Hook ref gate is synchronous; pending state disables input/Save/Remove (`SettingsPage.tsx:323,332,343`), and Enter reaches the same gated action. Functional draft update clears only the submitted unchanged value. Deferred test `useSettingsState.test.ts:291–346` asserts exactly one A submission while both Save and Remove are attempted, then asserts B survives A's completion and is sent only by the subsequent Save. | No new overlap/draft-loss regression found. |
| **N1** | **Yes, requested cases** | Set-success/read-error tests at `core/tests/settings.rs:711,805,829`; DELETE-failure tests at `:686,854`; off/on × absent/empty/valid/corrupt selection table at `:903`. `commands/tests/commands.rs:16–29` wraps the signal in two anyhow contexts and asserts the **serialized JSON tag and detail**, not just its Rust variant. | DELETE-trigger failure does not exercise a failure after DELETE or the WAL busy status. Those missing cases expose B2a/B2b. |
| **N2** | **Original fixes yes; small follow-up** | Hook comment now describes explicit Save. Original Result and AGENTS.md statements are corrected. | `.scratch/round18/issues/02-github-token-keychain.md:130` newly labels the fix itself “uncommitted, working tree only”; replace with `bc84841` or explicitly mark it as the pre-commit historical state. Non-blocking documentation nit. |

Paths in the table beginning `core/` or `commands/` are under `src-tauri/src/`.

### Remaining findings

#### B2a — P1 · Failed compaction becomes permanently ineligible for retry

**Where:** `src-tauri/src/core/skill_store.rs:554–555`, `src-tauri/src/core/settings.rs:399–410,454–455`.

DELETE autocommits before VACUUM. If VACUUM fails (busy database, disk/resource failure, interruption), the immediate call correctly errors, but the legacy row is already gone. The next startup returns `NothingToMigrate`; later Save/Remove also skip cleanup because `legacy_row_pending` is false. Historical plaintext in freed pages can therefore remain indefinitely in the **live** database. This is not the explicitly excluded backup/snapshot/device-remnant risk, nor merely a database previously touched by the unreleased plain-DELETE build. The ticket's `:170–171` assertion “retry via Save” is false for this case.

**Connection/concurrency check:** `with_conn` (`skill_store.rs:1456–1467`) opens a fresh connection per call; there is no enclosing transaction or connection pool mutex. VACUUM is legal here. SQLite serializes active writers and obtains the locks required by VACUUM; the application does **not** reserve the writer across the autocommit DELETE→VACUUM gap. Other connections/readers can enter that gap. The existing `concurrent_readers_and_writers_all_succeed` test (`core/tests/skill_store.rs:1852`) passed, but runs ordinary upsert/read pairs, not secure cleanup or VACUUM.

**Fresh reproduction:** stdin-compiled Rust probe linked to the project's bundled rusqlite. In a synthetic rollback-journal database, inserted/replaced a synthetic legacy value to leave historical bytes, ran the cleanup's exact PRAGMA→DELETE order, then let an independent reader hold a transaction before VACUUM. With the production 5-second timeout:

```text
concurrent reader: vacuum=Err(... DatabaseBusy ... "database is locked"); elapsed=5.17619375s; row_count=0; old_bytes=true
reader released: next migration skips=true, old_bytes=true
explicit VACUUM retry: old_bytes=false
```

The probe exercises the SQL sequence and evaluates the row-presence retry condition; it does not call the private production method. An independent injected VACUUM failure produced the same missing-row/remaining-bytes state. Evidence: `evidence/bc84841-vacuum-reader-probe.log` and `evidence/bc84841-cleanup-probe.log`. Synthetic DBs were deleted.

**Concrete fix:** persist a non-secret cleanup-pending marker atomically with the legacy-row deletion, and have startup and explicit token operations finish pending compaction/checkpointing even when the token row is absent. Clear that marker only after the required cleanup succeeds. Do not restore plaintext merely to obtain a retry signal. Add a post-DELETE/VACUUM-failure test that releases the fault, retries with no legacy row, verifies historical bytes disappear, and verifies the chosen keychain value is untouched. Correct the ticket's retry claim.

#### B2b — P2 · A busy WAL checkpoint is treated as completed erasure

**Where:** `src-tauri/src/core/skill_store.rs:558–559`.

`PRAGMA wal_checkpoint(TRUNCATE)` reports contention in its **result row** (first column `1`); this is not a rusqlite query error. The closure `|_| Ok(())` discards that status, so the method and migration report success even when old token frames remain readable in the WAL.

**Fresh reproduction:** synthetic WAL DB, reader retaining a pre-delete snapshot, then the exact secure-delete→VACUUM→checkpoint sequence:

```text
wal checkpoint_row=(1, 10, 5), production_row_mapper=Ok(()), token_in_wal=true
```

Verified against bundled SQLite's `OP_Checkpoint` implementation (`libsqlite3-sys-0.37.0/sqlite3/sqlite3.c:103264`): `SQLITE_BUSY` becomes `SQLITE_OK` plus result-column `1`. The application does not enable WAL today, so this is conditional, not a claim that default installations use WAL. Nevertheless, the explicitly required WAL branch cannot claim cleanup completion on this result.

**Concrete fix:** inspect the checkpoint status and propagate incomplete/busy cleanup as an error; retain the non-secret pending marker from B2a so a later retry can finish it. Test with a WAL reader held through the first attempt, then released, and assert no `Migrated` on the busy attempt and no synthetic token bytes after successful retry.

### Other regression checks and fresh verification

- Migration of a nonblank row still requires a successful keychain write and matching read-back before deleting it. Blank rows intentionally require no keychain write. Explicit removal intentionally authorizes deleting the row without a replacement write. No new unverified nonblank migration-delete path found.
- Skipping value read-back when no legacy row is pending follows the orchestrator's decision; the settings read returns null on access failure and the hook warns/retains the draft. No new hole found in that accepted policy.
- No new non-test `unwrap`/`expect` calls found. New `MemoryStore` helpers remain test-only.
- `cd src-tauri && cargo test --all`: **703 passed**, zero failures; main/doc tests passed. Full output: `evidence/bc84841-rust-tests.log`.
- `npm run test`: **17 files / 429 tests passed**.
- `npm run build`: **TypeScript build and Vite passed**; existing chunk-size/mixed-import warnings only.
- `cd src-tauri && cargo clippy --all-targets -- -D warnings`: **passed**.
- `git diff --exit-code bc84841 -- src src-tauri`: **clean**, including freshly generated bindings.
- Targeted LSP probe: no reported diagnostics; one file confirmed clean and one unconfirmed (push-only server). The fresh TypeScript build is the definitive type-check evidence.

**Goal-backward check:** ordinary migration now removes current/historical plaintext and frontend recovery is truthful, but required cleanup can still become unretryable or be falsely marked successful. Resolve B2a/B2b before shipping; the status nit is non-blocking. **Verdict: fix-then-ship.**
