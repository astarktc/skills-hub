//! The single wall-clock reading used across the app.
//!
//! Timestamps stored in the database (`created_at`, `updated_at`,
//! `last_fetched_ms`, …) are Unix epoch milliseconds. Core functions that
//! record a timestamp take it as a parameter so tests can substitute a fixed
//! value; this module is where the seams that *do* read the clock get it, so
//! there is one definition rather than one per module.
use std::time::SystemTime;

/// Milliseconds since the Unix epoch, saturating to 0 before the epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
