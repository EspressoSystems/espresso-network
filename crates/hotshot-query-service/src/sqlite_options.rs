//! Shared SQLite connection defaults.
//!
//! Used by both the embedded-db variant of this crate and by external clients (e.g. the light
//! client) that build their own pool. Keeping the pragma choices in one place ensures every
//! SQLite database in the workspace runs with the same journaling, locking, and vacuum settings.

use std::time::Duration;

use sqlx::sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode};

/// Default [`SqliteConnectOptions`] for SQLite databases in this workspace.
///
/// WAL journaling is the load-bearing choice: under the default rollback journal, any writer
/// holds an exclusive lock that blocks readers on other connections, which produces spurious
/// `SQLITE_BUSY` ("database is locked") errors under concurrent access. WAL lets readers and
/// the single writer proceed in parallel.
///
/// Callers add `.filename(...)` (or use `:memory:`) on top.
pub fn sqlite_options() -> SqliteConnectOptions {
    SqliteConnectOptions::default()
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(30))
        .auto_vacuum(SqliteAutoVacuum::Incremental)
        .create_if_missing(true)
}
