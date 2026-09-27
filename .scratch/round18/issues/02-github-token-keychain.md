# 02 — GitHub token → OS keychain behind `CredentialStore` (#44)

Status: ready-for-agent
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
  move it into the keychain and delete the row; log once. Idempotent.
- Errors: keychain failure is a typed `CommandError` variant (`CREDENTIAL_STORE_UNAVAILABLE { detail }`) with a
  `describeCommandError` branch — the operator can cause it (locked keychain, no secret service). Never fall back
  to plaintext silently.
- Tests: memory store round-trip; migration moves and deletes; `set_setting` for the token writes the store not
  the row. `cargo test --all` regenerates bindings — commit them.
