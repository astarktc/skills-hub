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

Implemented 2026-09-27 by a delegated child (uncommitted; working tree only).

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
- AGENTS.md/CONTEXT.md do not yet mention the credential store rule ("core never constructs `KeyringStore`; the command
  seam's `credential_store()` does") — orchestrator's call whether to add it to "Core never resolves filesystem roots…".
