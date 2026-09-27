use super::{CredentialStore, MemoryStore, SERVICE};
use crate::core::errors::SignalError;

#[test]
fn memory_store_round_trips_set_get_delete() {
    let store = MemoryStore::new();
    assert_eq!(store.get("github_token").unwrap(), None);

    store.set("github_token", "ghp_one").unwrap();
    assert_eq!(
        store.get("github_token").unwrap().as_deref(),
        Some("ghp_one")
    );

    store.set("github_token", "ghp_two").unwrap();
    assert_eq!(
        store.get("github_token").unwrap().as_deref(),
        Some("ghp_two")
    );
    assert_eq!(store.get("other").unwrap(), None);

    store.delete("github_token").unwrap();
    assert_eq!(store.get("github_token").unwrap(), None);
    // Deleting an absent key is a no-op, as for the keychain adapter.
    store.delete("github_token").unwrap();
}

#[test]
fn failing_memory_store_raises_the_typed_signal() {
    let store = MemoryStore::failing();
    for err in [
        store.get("k").unwrap_err(),
        store.set("k", "v").unwrap_err(),
        store.delete("k").unwrap_err(),
    ] {
        assert!(matches!(
            err.downcast_ref::<SignalError>(),
            Some(SignalError::CredentialStoreUnavailable { .. })
        ));
    }
}

/// The Keychain entry is found by service + account; changing either orphans
/// every operator's stored token (and re-prompts), so both are pinned.
#[test]
fn keychain_coordinates_are_stable() {
    assert_eq!(SERVICE, "com.skillshub.app");
    assert_eq!(super::keys::GITHUB_TOKEN, "github_token");
}
