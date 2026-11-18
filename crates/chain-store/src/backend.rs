use std::ops::Range;

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::watch;

/// Where a [`ChainStore`](crate::ChainStore) keeps its data: bytes by height, with the
/// commitments to find them by.
///
/// Every method is the backend's whole contract; [`testing`](crate::testing) checks it.
#[async_trait]
pub trait Backend: Send + Sync + 'static {
    /// Stores `row` and `parts` in one commit. A row or part already stored is kept, and nothing
    /// is written before the floor.
    async fn put(&self, row: Row, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()>;

    /// Stores `part` of `height`, unless the height has it or is before the floor.
    async fn put_part(&self, height: u64, part: Part, data: Bytes) -> anyhow::Result<()>;

    async fn item(&self, height: u64) -> anyhow::Result<Option<Bytes>>;

    async fn part(&self, height: u64, part: Part) -> anyhow::Result<Option<Bytes>>;

    /// The height of the row whose `column` is `value`.
    async fn height_of(&self, column: Column, value: &str) -> anyhow::Result<Option<u64>>;

    async fn newest(&self) -> anyhow::Result<Option<u64>>;

    async fn floor(&self) -> anyhow::Result<u64>;

    /// See [`ChainStore::missing_items`](crate::ChainStore::missing_items).
    async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>>;

    /// See [`ChainStore::missing_parts`](crate::ChainStore::missing_parts).
    async fn missing_parts(&self, part: Part, from: u64, limit: usize) -> anyhow::Result<Vec<u64>>;

    /// Deletes rows and parts before `height`, and raises the floor to it.
    async fn prune_below(&self, height: u64) -> anyhow::Result<()>;

    /// A value that changes after each committed write by any writer.
    fn changes(&self) -> watch::Receiver<u64>;
}

/// An item as a backend stores it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub height: u64,

    /// The commitments of the leaf, the block header and the payload, as tagged base64.
    pub leaf: String,
    pub block: String,
    pub payload: String,

    /// The encoded item.
    pub item: Bytes,
}

/// A commitment column of a [`Row`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Leaf,
    Block,
    Payload,
}

/// A part of a height that is stored apart from its row, and can arrive after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Part {
    Payload,
    VidCommon,
}

impl Part {
    /// The number a backend stores the part under.
    pub fn id(self) -> i16 {
        match self {
            Self::Payload => 0,
            Self::VidCommon => 1,
        }
    }
}
