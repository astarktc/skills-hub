//! Secrets the app keeps for the operator (today: the GitHub token) live in
//! the OS credential store, never in the SQLite settings table.
//!
//! `CredentialStore` is the seam: `KeyringStore` is the production adapter
//! (macOS Keychain, Windows Credential Manager, Linux Secret Service, via the
//! `keyring` crate's default `v1` feature); `MemoryStore` is the in-process
//! adapter tests use. Core never constructs a `KeyringStore` — the command
//! seam does (`commands::credential_store`), exactly as it resolves
//! `InstallerPaths`, so no core test can touch the operator's real keychain.
//!
//! Every store failure is raised as the typed
//! `SignalError::CredentialStoreUnavailable`; nothing here falls back to
//! plaintext.

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::Mutex;

use anyhow::Result;

use super::errors::SignalError;

/// Keychain service name: the app identifier, so the entry is recognisable in
/// Keychain Access and stable across releases (ad-hoc-signed macOS builds
/// change signature every release, which may re-prompt for access — the
/// service/account pair must not also change).
pub const SERVICE: &str = "com.skillshub.app";

/// Credential keys (the store's account names) — spelled here and nowhere else.
pub mod keys {
    pub const GITHUB_TOKEN: &str = "github_token";
}

/// A key/value store for secrets. `get` answers `Ok(None)` for a key that
/// was never set (or was deleted); `delete` of an absent key is a no-op.
pub trait CredentialStore: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<String>>;
    fn set(&self, key: &str, value: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}

fn unavailable(err: impl std::fmt::Display) -> anyhow::Error {
    anyhow::Error::new(SignalError::CredentialStoreUnavailable {
        detail: err.to_string(),
    })
}

/// The OS credential store, one entry per key under [`SERVICE`].
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyringStore;

impl KeyringStore {
    pub fn new() -> Self {
        Self
    }

    /// `keyring`'s v1 wrapper initialises the platform store once per process
    /// and caches the result (`Entry::store_status`); a failed initialisation
    /// (no Secret Service on Linux) makes every later `Entry::new` answer the
    /// generic `NoDefaultStore` without retrying — recovery needs an app
    /// restart. The cached initialisation error is the useful diagnostic, so
    /// it is preferred for `detail`.
    fn entry(key: &str) -> Result<keyring::Entry> {
        keyring::Entry::new(SERVICE, key).map_err(|err| match keyring::Entry::store_status() {
            Err(init) if matches!(err, keyring::Error::NoDefaultStore) => unavailable(format!(
                "credential store initialisation failed (restart required after fixing): {init}"
            )),
            _ => unavailable(err),
        })
    }
}

impl CredentialStore for KeyringStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        match Self::entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(unavailable(err)),
        }
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        Self::entry(key)?.set_password(value).map_err(unavailable)
    }

    fn delete(&self, key: &str) -> Result<()> {
        match Self::entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(unavailable(err)),
        }
    }
}

/// In-process store for tests. `failing()` builds one whose every call
/// fails like a locked keychain; `set_reads_fail(true)` scripts a store whose
/// writes land but whose reads are denied (a keychain that accepts the item
/// and then refuses the read prompt).
#[cfg(test)]
#[derive(Debug, Default)]
pub struct MemoryStore {
    entries: Mutex<HashMap<String, String>>,
    fail: bool,
    fail_reads: AtomicBool,
}

#[cfg(test)]
impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn failing() -> Self {
        Self {
            fail: true,
            ..Self::default()
        }
    }

    /// Toggle read denial; writes and deletes keep working.
    pub fn set_reads_fail(&self, fail: bool) {
        self.fail_reads.store(fail, Ordering::SeqCst);
    }

    fn check(&self) -> Result<()> {
        if self.fail {
            anyhow::bail!(SignalError::CredentialStoreUnavailable {
                detail: "memory store configured to fail".to_string(),
            });
        }
        Ok(())
    }

    fn check_read(&self) -> Result<()> {
        self.check()?;
        if self.fail_reads.load(Ordering::SeqCst) {
            anyhow::bail!(SignalError::CredentialStoreUnavailable {
                detail: "memory store configured to deny reads".to_string(),
            });
        }
        Ok(())
    }

    fn entries(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.entries.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
impl CredentialStore for MemoryStore {
    fn get(&self, key: &str) -> Result<Option<String>> {
        self.check_read()?;
        Ok(self.entries().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> Result<()> {
        self.check()?;
        self.entries().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, key: &str) -> Result<()> {
        self.check()?;
        self.entries().remove(key);
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/credentials.rs"]
mod tests;
