# 02 — GitHub token → OS keychain behind `CredentialStore` (#44)

Status: implemented
Spec: `.scratch/round18/spec.md` — ticket 02.

## Work

- New `core/credentials.rs`: `pub trait CredentialStore { fn get(&self, key) -> Result<Option<String>>; fn set(..);
  fn delete(..) }`; `KeyringStore` (crate `keyring` — check its current backend story on macOS/Windows/Linux and pin
  a version; Linux may need the `sync-secret-service` or file fallback feature — document the choice) and
  `MemoryStore` for tests. Core never resolves the store from the environment: commands construct it once at the
  seam (`commands/mod.rs`), like `installer_paths`.
- `settings::github_token(store)` / `github_token_or_none(store)` become `github_token(credentials)` — five core
  call sites (`installer.rs` ×4, `refresh.rs:158`, `git_acquisition.rs` doc) and `github_download.rs` tests. Keep
  the function names so the diff is mechanical. `AppSettings.github_token` on the wire: keep the field but make it
  a **presence flag** (`github_token_set: bool`) — the frontend never needs the secret back; update
  `SettingsPage.tsx` + `useSettingsState.ts` + i18n accordingly (EN only after ticket 03; coordinate).
- **Migration**: on startup (where `migrate_legacy_db_if_needed` runs), if the `github_token` settings row exists,
  write it to the keychain, **read it back**, and only then delete the row; log once. Idempotent. Operator decision
  2026-09-24: migrate automatically, no opt-in button.
- macOS gotcha: ad-hoc-signed builds change signature every release, so the "Skills Hub wants to access…"
  Keychain prompt can recur after updates. Use one stable service name (`com.skillshub.app`) and account
  (`github_token`); a denied/locked prompt yields the typed error below and leaves the settings row untouched. Note
  the prompt in CHANGELOG.
- Errors: keychain failure is a typed `CommandError` variant (`CREDENTIAL_STORE_UNAVAILABLE { detail }`) with a
  `describeCommandError` branch — the operator can cause it (locked keychain, no secret service). Never fall back
  to plaintext silently.
- Tests: memory store round-trip; migration moves and deletes; `set_setting` for the token writes the store not
  the row. `cargo test --all` regenerates bindings — commit them.

## Result

Implemented 2026-09-27 by a delegated child; committed as `2b60c7f`. (Corrected after review N2: this line
originally said "uncommitted; working tree only", true only before the commit.) Superseded in part by
**Review fixes** below.

### What changed

- **`core/credentials.rs`** (new, declared in `core/mod.rs`): `trait CredentialStore: Send + Sync { get / set / delete }`
  (`get` → `Ok(None)` when absent; `delete` of an absent key is a no-op). `KeyringStore` (production, stateless,
  service `com.skillshub.app`, account `github_token` — pinned by a test) and `#[cfg(test)] MemoryStore`
  (+ `MemoryStore::failing()` that behaves like a locked keychain). Every store failure is raised as
  `SignalError::CredentialStoreUnavailable { detail }`; nothing falls back to plaintext.
- **Seam**: `commands::credential_store()` constructs the `KeyringStore` (like `installer_paths`); core never does.
- **`core/settings.rs`**: `github_token(credentials)` / `github_token_or_none(credentials)` (names kept). The settings
  key is renamed internally to `keys::LEGACY_GITHUB_TOKEN` (read only by the migration). `load_settings` and
  `apply_setting` take `credentials: &dyn CredentialStore`; `SettingUpdate::GithubToken` writes the store
  (blank → `delete`). `AppSettings.github_token_set` presence flag: a store that cannot be read reports `false`
  (logged warn) so Settings still loads; writes surface the typed error. New
  `migrate_github_token_to_credential_store(store, credentials) -> TokenMigration { NothingToMigrate | Migrated | DroppedBlank }`:
  set → read back → compare → only then delete the row; any failure leaves the row (idempotent, re-run every launch;
  no row ⇒ no keychain call). New `onboarding_scan_selection(store)` so the onboarding scan scope no longer calls
  `load_settings` (avoids a needless keychain read / macOS prompt during onboarding).
- **`core/skill_store.rs`**: `pub(super) fn delete_setting` (raw adapter, used only by the migration).
- **Core call sites** now take `credentials: &dyn CredentialStore` after `store`: `installer::list_git_skills`,
  `install_git_skill_from_listing`, `clone_for_explore_preview`, `refresh::refresh_managed_skills` (+ `#[allow(clippy::too_many_arguments)]`),
  `repoint::repoint_skill_source`. The `#[cfg(test)]` legacy `install_git_skill_from_selection` fixture adapter uses
  `HttpGithubApi::new(None)` (fixtures carry no token) so its ~10 test call sites stay unchanged. `git_acquisition.rs` doc updated.
- **Startup** (`lib.rs`, right after `ensure_schema`, where `migrate_legacy_db_if_needed` runs): migration runs once per
  launch, logs `info` once on move/blank-drop, `warn` on failure (row kept).
- **Error**: `SignalError::CredentialStoreUnavailable` → `CommandError::CredentialStoreUnavailable { detail }`
  (`CREDENTIAL_STORE_UNAVAILABLE`), regenerated binding, `describeCommandError` branch (copy + `detail` line),
  `errors.credentialStoreUnavailable` i18n key, `commandError.test.ts` case.
- **Frontend**: `useSettingsState` exposes `githubTokenSet: boolean` (was `githubToken: string`);
  `handleGithubTokenChange(token)` now returns `Promise<boolean>` (true when the write landed) and adopts presence from
  the echo. `SettingsPage` shows a write-only password draft (never prefilled) + **Save token** (button/Enter; clears
  the draft on success) + **Remove token** (only when set) + "A token is saved…" / "No token saved." status line;
  hint mentions the system keychain. New `en` keys: `githubTokenSet`, `githubTokenUnset`, `githubTokenSave`,
  `githubTokenClear`, `errors.credentialStoreUnavailable`; `githubTokenHint` extended.

### keyring crate

- `keyring = "4.2"` (resolved **4.2.0**, 2026-08-29), default feature **`v1`** = macOS Keychain Services
  (`apple-native-keyring-store/keychain`), Windows Credential Manager (`windows-native-keyring-store`), Linux/*nix
  Secret Service over **zbus** with `crypto-rust` (pure Rust — no libdbus/openssl system dep, keeps the
  "no system SSL" rule). No file/plaintext fallback feature enabled on purpose: a Linux box with no Secret Service
  gets `CREDENTIAL_STORE_UNAVAILABLE` (token features degrade to unauthenticated) rather than a silent plaintext file.
- Sources: https://crates.io/crates/keyring (API `https://crates.io/api/v1/crates/keyring`, readme 4.2.0),
  https://github.com/open-source-cooperative/keyring-rs ; verified the v1 module source (`src/v1.rs`) in the
  downloaded 4.2.0 crate. Note keyring 4 split into `keyring-core` + per-store crates; the `v1` feature is the
  maintained all-in-one wrapper for exactly this use.

### ⚠ AppSettings wire change (relay to ticket 04)

`AppSettings.github_token: string` → **`AppSettings.github_token_set: boolean`**. `SettingUpdate` is unchanged
(`{ key: "github_token", value: string }`, blank clears). New `CommandError` code `CREDENTIAL_STORE_UNAVAILABLE { detail }`.
No command added/renamed; no arity change on the wire. (At the time of the final gate, `src/fixtures/model.ts` /
`backend.ts` already carried `github_token_set` — ticket 04 caught up.) Also touched `src/hooks/useSyncOrchestration.test.ts`
(one fixture field) because the type change breaks it.

### CHANGELOG lines (for ticket 10's 1.2.18 block)

- Security: the GitHub token now lives in the OS credential store (macOS Keychain, Windows Credential Manager, Linux
  Secret Service) instead of the app database. An existing token is moved automatically on first launch of 1.2.18 and
  the database copy is deleted only after the keychain copy is verified (#44). Settings shows whether a token is
  saved; it can be replaced or removed but is never displayed.
- Note (macOS): Skills Hub builds are ad-hoc signed, so after installing an update macOS may ask "Skills Hub wants to
  access … in your keychain" again. Choose **Always Allow**. If access is denied or the keychain is locked, GitHub
  requests run unauthenticated (60 req/hr) and saving a token reports an error; nothing falls back to plaintext.
- Note (Linux): a running Secret Service (GNOME Keyring, KWallet, …) is required to store a token.

### Verification

- `npm run version:check` → `Version OK (1.2.17)`.
- `npm run check` → lint clean, vitest 16 files / 410 tests pass, `tsc -b && vite build` ok, `cargo fmt --check` clean,
  clippy `-D warnings` clean, `cargo test`: **692 passed, 0 failed**. (One earlier full run hit a flake in
  `skill_store::concurrent_readers_and_writers_all_succeed` under load; passes alone and in the final gate.)
- New Rust tests: `credentials::memory_store_round_trips_set_get_delete`, `failing_memory_store_raises_the_typed_signal`,
  `keychain_coordinates_are_stable`; settings: `apply_github_token_writes_the_credential_store_not_the_settings_row`,
  `apply_github_token_store_failure_is_typed_and_writes_nothing`, `token_migration_moves_the_row_then_deletes_it_and_is_idempotent`,
  `token_migration_drops_a_blank_row_without_touching_the_store`, `token_migration_store_failure_keeps_the_row`,
  `token_migration_read_back_mismatch_keeps_the_row`, `token_read_failure_is_typed_degrades_for_acquisition_and_settings_presence`;
  `github_download::credential_store_failure_sends_no_bearer_to_github`; installer degrade test now uses a failing store.
- Frontend: `useSettingsState.test.ts` (presence load, set/replace by presence never holding the secret, clear,
  keychain failure keeps presence + reports), `commandError.test.ts` new branch.

### Open questions

- The real `KeyringStore` is not exercised by any automated test (by design — no test may touch the operator's
  keychain). Operator smoke test on the installed 1.2.18 release: with a pre-existing token, first launch should log
  "moved the GitHub token…", `sqlite3 skills_hub.db "select * from settings where key='github_token'"` → empty, and
  Keychain Access shows service `com.skillshub.app` / account `github_token`.
- Linux/Windows keychain adapters compile only on their CI runners; not built locally.
- ~~AGENTS.md/CONTEXT.md do not yet mention the credential store rule~~ (corrected after review N2): `2b60c7f` added it
  to AGENTS.md "Core never resolves filesystem roots…" ("The same rule covers secrets…"). CONTEXT.md does not mention it.

## Review fixes

Fixes for `.scratch/round18/review/ticket-02-review.md` (review of `2b60c7f`), implemented 2026-09-27 by a delegated
child on `main` @ `a3672bc`; committed as `bc84841`. Implemented as the orchestrator decided.

| Finding | What changed | Test that pins it |
| --- | --- | --- |
| **B1** — a pending migration overrode a later Save/Remove | `settings::apply_setting`'s `GithubToken` arm → new private `set_github_token`. If a legacy `github_token` row is pending: Save = `set` → read back (shared `verify_stored_github_token`, also used by the migration) → secure-delete row; Remove = `delete` → secure-delete row. Every failure propagates. A row that cannot be removed fails the operation, even though the keychain already holds what the operator asked for. The row then survives, so the next launch's migration is the remaining risk, reported and not hidden. With no pending row, a read-back failure is not an error: presence reads `None` (S1). | `core::settings::tests::save_after_failed_migration_settles_the_row_so_restart_keeps_the_new_token` (a), `remove_after_partial_migration_settles_the_row_so_restart_stays_empty` (b), `legacy_row_cleanup_failure_fails_save_and_remove` (c, via a `BEFORE DELETE` trigger), `save_with_pending_row_and_denied_read_back_fails_and_keeps_the_row` |
| **B2** — plain DELETE left the plaintext in freed pages | `SkillStore::delete_setting` (plain) is replaced by `delete_setting_securely(key)`. On one connection it runs `PRAGMA secure_delete = ON` → DELETE → `VACUUM`, then `PRAGMA wal_checkpoint(TRUNCATE)` if `journal_mode` is `wal`. The store never enables WAL: it uses the default rollback journal, which is deleted at commit. The WAL branch is only a defensive guard. Both the migration (including its blank-row path) and B1's settlement use it. The migration returns `Migrated` only after the cleanup succeeds. Limits are documented on the method and the migration. | `token_migration_removes_the_plaintext_from_the_database_bytes`: file-backed DB with two synthetic sentinels (an earlier value plus the current row). It checks that the sentinel is on disk before migration, then that neither sentinel is in `test.db`/`-journal`/`-wal`/`-shm` afterwards and the row is logically gone. Sensitivity was checked by temporarily removing `secure_delete` + `VACUUM`: the test then fails with "still present in test.db". Also `token_migration_cleanup_failure_is_an_error_not_migrated`. B1 (a) also asserts that the legacy sentinel is gone from disk. |
| **S1** — unreadable reported as absent | `AppSettings.github_token_set: bool` → `Option<bool>` (wire `boolean \| null`, regenerated binding; no `?`). `None` = store unreadable (logged); the rest of Settings loads. `useSettingsState.githubTokenSet: boolean \| null`. `SettingsPage`: `null` shows `githubTokenUnknown` (actionable recovery copy), Save stays available, and Remove is shown (`githubTokenSet !== false`). If a Save/Remove echo does not confirm the expected presence, the hook calls `notify("warning", githubTokenUnconfirmedTitle, …Message)`, makes no success claim, and keeps the draft. | Rust: `token_read_failure_is_typed_degrades_for_acquisition_and_reads_as_unknown_presence` (was: expected `false`), `existing_token_with_denied_read_reports_unknown_presence`, `save_with_denied_read_back_reports_unknown_presence`. Hook: "reports an unreadable keychain as unknown presence, not absent", "warns, keeps the draft and claims nothing when a save cannot be read back" |
| **S2** — Linux: in-process retry cannot recover | `errors.credentialStoreUnavailable` now says to unlock the keychain and allow access, and on Linux to start the secret service **and restart Skills Hub**. It adds that retrying without a restart cannot reach the service. `KeyringStore::entry`: when `Entry::new` fails with `NoDefaultStore` and `keyring::Entry::store_status()` is `Err` (verified in keyring 4.2.0 `src/v1.rs`: `pub fn store_status() -> &'static Result<()>`, the cached `SET_CREDENTIAL_STORE_RESULT`), `detail` carries the original initialisation error plus "(restart required after fixing)". | Copy only; the real adapter stays untested by design, because no test may touch the keychain. Linux runtime is not verifiable here. |
| **S3** — slow Save clobbered a newer draft; overlapping writes | The draft moved from `SettingsPage` into `useSettingsState` (`githubTokenDraft`, `handleGithubTokenDraftChange`). Token mutations are single-flight: a ref gate plus `githubTokenPending` state. While pending, `handleGithubTokenSave`/`handleGithubTokenRemove` (and so Enter) are no-ops; the page disables the input, Save and Remove. Save clears the draft only if the echo confirms and the draft still equals the value that was sent. The old `handleGithubTokenChange(token): Promise<boolean>` API is gone. | Hook: "ignores Save/Remove while a token write is pending and keeps a newer draft" (deferred promise: A pending → draft B + Save + Remove ignored → A resolves → draft B kept → next Save sends B and clears) |
| **N1** — coverage | set ok → get errors: `token_migration_read_back_denied_keeps_the_row` and the S1/B1 tests (`MemoryStore::set_reads_fail`, a new scripted failure mode: writes land, reads are denied). DB cleanup failure: the B1(c) and B2 cleanup tests. `onboarding_scan_selection_table`: flag off / on / default × selection absent / `[]` / valid / corrupt. Command seam: `commands::tests::credential_store_failure_crosses_the_wire_as_a_typed_error` (two `anyhow` contexts → `{ code: "CREDENTIAL_STORE_UNAVAILABLE", detail }`). | as listed |
| **N2** — stale descriptions | `useSettingsState.ts` `writeSetting` doc no longer says the token writes on every keystroke. This ticket's Result no longer says "uncommitted" or that AGENTS.md lacks the credential rule. | — |

Also: the startup comment and warn log in `lib.rs` now describe the secure cleanup ("a remaining settings row is retried
next launch").

### CHANGELOG lines (amended — replace the three lines above in ticket 10's 1.2.18 block)

- Security: the GitHub token now lives in the OS credential store (macOS Keychain, Windows Credential Manager, Linux
  Secret Service) instead of the app database. An existing token moves automatically on the first launch of 1.2.18.
  The database copy is removed only after the keychain copy is verified, and it is removed securely: overwritten, and
  the database compacted, so the plaintext no longer sits in the database file. This cannot erase copies made before
  the upgrade: backups, APFS / Time Machine snapshots, or SSD remnants. Rotate the token on GitHub if that matters to
  you (#44). Saving or removing a token in Settings also removes any database copy still waiting to be migrated.
  Settings shows whether a token is saved, or that the keychain could not be read. A token can be replaced or
  removed but is never displayed.
- Note (macOS): Skills Hub builds are ad-hoc signed, so after installing an update macOS may ask "Skills Hub wants to
  access … in your keychain" again. Choose **Always Allow**. If access is denied or the keychain is locked, GitHub
  requests run unauthenticated (60 req/hr) and saving a token reports an error. Nothing falls back to plaintext.
- Note (Linux): storing a token requires a running Secret Service (GNOME Keyring, KWallet, …). If it was not running
  when Skills Hub started, start it and **restart Skills Hub**: the keychain connection is set up once per launch.

### Open questions

- A database that an **unreleased** `2b60c7f` dev build already migrated (e.g. an operator `tauri:dev` run) used the
  plain DELETE, so the token's bytes may remain in freed pages. That migration left no row, so the new migration never
  runs `VACUUM` on it. A one-off `sqlite3 skills_hub.db 'VACUUM;'` fixes it (the operator's call). Released 1.2.17
  databases are unaffected: they still hold the row, and the new migration cleans it.
- B1(c) residual: if the legacy row cannot be deleted, the next launch's migration will write that row's token over
  the operator's explicit choice. The operation reports the failure. A row that cannot be deleted means the database
  is not writable, which blocks most of the app anyway.
- `VACUUM` needs an exclusive lock. Startup runs before any command. From Settings it waits out other writers under
  the 5 s `busy_timeout`, and a timeout surfaces as an error. If the DELETE itself failed, the row is kept and the
  next launch or Save/Remove retries it. If the DELETE committed but the compaction failed, the row is gone (its live
  bytes zeroed) and the non-secret `github_token.cleanup_pending` marker, committed in the DELETE's transaction, makes
  the next launch (`SkillStore::finish_pending_secure_cleanup`, before the migration) or the next token Save/Remove
  finish the compaction — see **Review fixes (round 2)**.

### Verification

- `npm run version:check && npm run check` → exit 0: `Version OK (1.2.17)`; lint clean; vitest **17 files / 429
  tests**; `tsc -b && vite build` ok; `cargo fmt --check` clean; clippy `-D warnings` clean; `cargo test` **703
  passed, 0 failed** (bindings regenerated: `github_token_set: boolean | null`).

## Review fixes (round 2)

Fixes for `.scratch/round18/review/ticket-02-review.md` § "Re-review of bc84841", implemented 2026-09-27 by a
delegated child on `main` @ `6740a10`; committed as `4f6e312`. Implemented as the orchestrator decided. Astra final check (ship): one
non-blocking follow-up — the zeroing assertion in `core/tests/settings.rs` (~:768) also passes without `secure_delete`; tighten it
with a page-spanning sentinel and a sensitivity check (BACKLOG candidate, not a release blocker).

| Finding | What changed | Test that pins it |
| --- | --- | --- |
| **B2a** — a compaction failing after the DELETE lost its retry signal | `SkillStore::delete_setting_securely(key)`: `PRAGMA secure_delete = ON` (set before the transaction, so it covers it), then ONE transaction: DELETE the row + upsert the non-secret marker `<key>.cleanup_pending` = `1`; commit; then the shared private `compact_and_clear_cleanup_markers` (`VACUUM`, the checked WAL checkpoint, and only then a plain delete of every `*.cleanup_pending` marker). New `SkillStore::finish_pending_secure_cleanup() -> Result<bool>`: with any marker present, compacts and clears; otherwise does nothing. Called at startup in `lib.rs` right after `ensure_schema`, before the token migration (logs info/warn), and by `set_github_token` when no legacy row is pending (with a row pending, the secure delete's own compaction covers it). The migration returns `Migrated` only after the compaction succeeded; otherwise it errors, the next startup finishes the cleanup and the migration then reports `NothingToMigrate`. Plaintext is never restored. Documented on the method, `TokenMigration::Migrated`, the migration and the module doc. | `failed_compaction_keeps_a_marker_that_the_next_startup_completes`: an earlier token left in freelist pages + the current row; a `#[cfg(test)]` per-thread fault (`skill_store::compaction_fault`) fails the compaction → migration errors, row gone, marker `1`, keychain holds the current token, the live row's bytes are already zeroed (proves `secure_delete` was in force inside the transaction), the historical copy is still on disk; then startup order (`finish_pending_secure_cleanup` → `true`, migration → `NothingToMigrate`) → marker gone, no sentinel in any DB file, keychain unchanged. A fault hook rather than a busy reader here: in rollback-journal mode a reader holding its lock before the deletion blocks the DELETE's commit, not the `VACUUM`. `save_and_remove_finish_a_stale_cleanup_marker`: stale marker + freed sentinel bytes, no legacy row → Save, then Remove, each clears the marker and the bytes. `token_migration_cleanup_failure_is_an_error_not_migrated` now also asserts no marker after a failed DELETE (atomicity). |
| **B2b** — a busy WAL checkpoint counted as completed erasure | The checkpoint's result row `(busy, log, checkpointed)` is read; `busy != 0` is an error ("WAL checkpoint could not complete …") and the marker stays. | `busy_wal_checkpoint_is_an_error_and_the_retry_completes_it`: file-backed DB switched to WAL, a real reader holding a pre-deletion snapshot → migration errors after the 5 s busy timeout, row gone, marker kept, token bytes still on disk; reader released → `finish_pending_secure_cleanup` → marker gone, no sentinel in `test.db`/`-wal`/`-shm`/`-journal`. |
| **N2 follow-up** — stale status | § Review fixes now names commit `bc84841`; Open questions' "retry via Save" now describes the marker mechanism. | — |

Sensitivity: disabling the `busy` check fails the B2b test; dropping the `finish_pending_secure_cleanup` call from
`set_github_token` fails the stale-marker test ("marker left by \"ghp_B\""). Both restored.

Verification: `npm run version:check` → `Version OK (1.2.17)`; `cargo fmt --check` clean; clippy `-D warnings`
clean; `cargo test --all` **706 passed, 0 failed**; `npm run check` exit 0 (vitest **17 files / 446 tests**, which
includes a concurrent child's `src/fixtures/**` work in this tree). `git diff --exit-code -- src/bindings/index.ts`
→ exit 0 (no wire change).
