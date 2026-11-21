//! Application tables: keyed values, per named table and ordered bytewise by key, that a store
//! keeps for the service running over it next to its own data.
//!
//! Both stores of a deployment have them: the consensus store, for the membership data and the
//! decided state of consensus, and the chain store, for what the API instances serve
//! besides blocks. Code that keeps its data in either, such as the membership store, is written once
//! against [`AppTables`].

use std::{fmt, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;

/// The longest name of an application table, in bytes. A store can prefix the name with a few
/// bytes of its own and stay within the 64 its tables allow.
const MAX_LEN: usize = 60;

/// The name of an application table: 1 to [`MAX_LEN`] bytes from `[a-z0-9_-]`, not starting with
/// `_`. It is checked once, when it is made, so that no read or write can fail on it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableName(Arc<str>);

impl TableName {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        let valid = !name.is_empty()
            && name.len() <= MAX_LEN
            && !name.starts_with('_')
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        anyhow::ensure!(
            valid,
            "invalid application table name {name:?}: expected 1 to {MAX_LEN} bytes of \
             [a-z0-9_-], not starting with _"
        );
        Ok(Self(name.into()))
    }

    /// The table `name`, fixed in code. Panics if it is not a valid name.
    pub fn of(name: &'static str) -> Self {
        Self::new(name).expect("table names in code are valid")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A write to the application tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppWrite {
    /// Sets `key` to `value`.
    Put {
        table: TableName,
        key: Bytes,
        value: Bytes,
    },

    /// Deletes the keys before `key`.
    DeleteBefore { table: TableName, key: Bytes },

    /// Deletes every key.
    Clear { table: TableName },
}

/// A store with application tables.
#[async_trait]
pub trait AppTables: Clone + Send + Sync + 'static {
    /// The value of `key` in `table`.
    async fn get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>>;

    /// The entries of `table` with keys from `from` on, and before `until` if set, in key order.
    async fn entries(
        &self,
        table: &TableName,
        from: &[u8],
        until: Option<&[u8]>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>>;

    /// Applies `writes` in order, in one commit.
    async fn write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_checked() {
        for name in ["a", "stake-events", "epoch_17", &"x".repeat(MAX_LEN)] {
            assert_eq!(TableName::new(name).unwrap().as_str(), name);
        }
        for name in [
            "",
            "_hidden",
            "Upper",
            "a b",
            "a/b",
            &"x".repeat(MAX_LEN + 1),
        ] {
            assert!(TableName::new(name).is_err(), "{name:?}");
        }
    }
}
