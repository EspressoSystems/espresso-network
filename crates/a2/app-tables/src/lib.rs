//! Application tables: keyed values, per named table and ordered bytewise by key, that a store
//! keeps for the service running over it next to its own data.
//!
//! Both stores of a deployment have them: the consensus store, for the membership data and the
//! decided state of consensus, and the chain store, for the data the ingester derives for the API
//! instances. Code that keeps its data in either, such as the membership store, is written once
//! against [`AppTables`].

use async_trait::async_trait;
use bytes::Bytes;

/// A write to the application tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppWrite {
    /// Sets `key` to `value`.
    Put {
        table: String,
        key: Bytes,
        value: Bytes,
    },

    /// Deletes the keys before `key`.
    DeleteBefore { table: String, key: Bytes },

    /// Deletes every key.
    Clear { table: String },
}

/// A store with application tables.
#[async_trait]
pub trait AppTables: Clone + Send + Sync + 'static {
    /// The value of `key` in `table`.
    async fn get(&self, table: &str, key: &[u8]) -> anyhow::Result<Option<Bytes>>;

    /// The entries of `table` with keys from `from` on, and before `until` if set, in key order.
    async fn entries(
        &self,
        table: &str,
        from: &[u8],
        until: Option<&[u8]>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>>;

    /// Applies `writes` in order, in one commit.
    async fn write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()>;
}
