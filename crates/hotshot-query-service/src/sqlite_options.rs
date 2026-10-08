//! Shared SQLite connection defaults.
//!
//! Used by both the embedded-db variant of this crate and by external clients (e.g. the light
//! client) that build their own pool. Keeping the pragma choices in one place ensures every
//! SQLite database in the workspace runs with the same journaling, locking, and vacuum settings.

use std::{env, time::Duration};

use sqlx::sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode};
use tracing::warn;

/// Unofficial, for benchmarks: comma-separated `key=value` SQLite pragmas applied after the
/// defaults, e.g. `synchronous=normal,cache_size=-1048576`. Malformed input panics.
const PRAGMAS_ENV: &str = "EXPERIMENTAL_SQLITE_PRAGMAS";

/// Default [`SqliteConnectOptions`] for SQLite databases in this workspace.
///
/// WAL journaling is the load-bearing choice: under the default rollback journal, any writer
/// holds an exclusive lock that blocks readers on other connections, which produces spurious
/// `SQLITE_BUSY` ("database is locked") errors under concurrent access. WAL lets readers and
/// the single writer proceed in parallel.
///
/// Callers add `.filename(...)` (or use `:memory:`) on top.
///
/// Pragmas from `EXPERIMENTAL_SQLITE_PRAGMAS` override the defaults.
pub fn sqlite_options() -> SqliteConnectOptions {
    let mut options = SqliteConnectOptions::default()
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(30))
        .auto_vacuum(SqliteAutoVacuum::Incremental)
        .create_if_missing(true);

    if let Ok(raw) = env::var(PRAGMAS_ENV) {
        let pragmas = parse_pragmas(&raw);
        warn!(?pragmas, "applying {PRAGMAS_ENV}");
        for (key, value) in pragmas {
            options = options.pragma(key, value);
        }
    }
    options
}

fn parse_pragmas(raw: &str) -> Vec<(String, String)> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let (key, value) = entry
                .split_once('=')
                .unwrap_or_else(|| panic!("{PRAGMAS_ENV}: entry {entry:?} is not key=value"));
            let (key, value) = (key.trim(), value.trim());
            assert!(!key.is_empty(), "{PRAGMAS_ENV}: empty key in {entry:?}");
            (key.to_string(), value.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list() {
        assert_eq!(
            parse_pragmas("synchronous=normal, cache_size = -1048576,"),
            vec![
                ("synchronous".to_string(), "normal".to_string()),
                ("cache_size".to_string(), "-1048576".to_string()),
            ]
        );
    }

    #[test]
    fn empty_is_empty() {
        assert!(parse_pragmas("").is_empty());
        assert!(parse_pragmas(" , ").is_empty());
    }

    #[test]
    #[should_panic(expected = "EXPERIMENTAL_SQLITE_PRAGMAS")]
    fn missing_equals_panics() {
        parse_pragmas("synchronous");
    }

    #[test]
    #[should_panic(expected = "empty key")]
    fn empty_key_panics() {
        parse_pragmas("=normal");
    }
}
