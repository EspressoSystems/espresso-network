use std::{ops::Range, time::Duration};

use app_tables::{AppWrite, TableName};
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
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

    /// Stores `parts` of `height` in one commit, each unless the height has it or is before the
    /// floor.
    async fn put_parts(&self, height: u64, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()>;

    async fn item(&self, height: u64) -> anyhow::Result<Option<Bytes>>;

    async fn part(&self, height: u64, part: Part) -> anyhow::Result<Option<Bytes>>;

    /// The item of `height` with the parts it has, in [`Part`] order, read together; the payload
    /// only if `payload`.
    async fn record(
        &self,
        height: u64,
        payload: bool,
    ) -> anyhow::Result<Option<(Bytes, Vec<(Part, Bytes)>)>>;

    /// The height of the row whose `column` is `value`.
    async fn height_of(&self, column: Column, value: &str) -> anyhow::Result<Option<u64>>;

    async fn newest(&self) -> anyhow::Result<Option<u64>>;

    async fn floor(&self) -> anyhow::Result<u64>;

    /// See [`ChainStore::missing_items`](crate::ChainStore::missing_items).
    async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>>;

    /// See [`ChainStore::newest_gap`](crate::ChainStore::newest_gap).
    async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>>;

    /// See [`ChainStore::missing_parts`](crate::ChainStore::missing_parts).
    async fn missing_parts(&self, part: Part, from: u64, limit: usize) -> anyhow::Result<Vec<u64>>;

    /// The value of `key` in the application table `table`.
    async fn app_get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>>;

    /// Up to `keys.limit` entries of the application table `table` with keys from `keys.from`
    /// and before `keys.until`, in bytewise key order.
    async fn app_entries(
        &self,
        table: &TableName,
        keys: KeyBounds<'_>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>>;

    /// Applies `writes` to the application tables, in order, in one commit.
    async fn app_write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()>;

    /// Deletes rows and parts before `height`, and raises the floor to it.
    async fn prune_below(&self, height: u64) -> anyhow::Result<()>;

    /// Takes or renews the lease `name` for `holder` for `ttl`, and says whether `holder` holds
    /// it now: it does unless another holder took it and its time has not run out. Leases tell
    /// processes sharing the store which of them does a piece of work; they are not part of the
    /// chain and change nothing readers see.
    async fn lease(&self, name: &str, holder: &str, ttl: Duration) -> anyhow::Result<bool>;

    /// The updates of the newest heights from `from` on, as a backend that keeps them itself
    /// sends them; see [`tip`](crate::tip). `None` from a backend that does not, whose readers
    /// follow the store by reading it.
    async fn tip_updates(
        &self,
        from: u64,
    ) -> anyhow::Result<Option<BoxStream<'static, anyhow::Result<TipUpdate>>>> {
        let _ = from;
        Ok(None)
    }

    /// A value that changes after each committed write to the chain by any writer: items,
    /// parts and pruning. Writes to the application tables do not change it.
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
    Cert2,
    StateCert,

    /// An empty marker: the height's `Certificate2` and state certificate are as complete as
    /// consensus made them, so a height without them has none.
    CertsKnown,

    /// What the payload's readers need to know of it without reading it: its BLAKE3 hash and its
    /// size, written with the payload.
    PayloadInfo,

    /// An empty marker: the payload is in the store's object store, for a store that keeps its
    /// payloads there; see [`payloads`](crate::payloads).
    PayloadStored,
}

impl Part {
    pub const ALL: [Part; 7] = [
        Self::Payload,
        Self::VidCommon,
        Self::Cert2,
        Self::StateCert,
        Self::CertsKnown,
        Self::PayloadInfo,
        Self::PayloadStored,
    ];

    /// The number a backend stores the part under.
    pub fn id(self) -> i16 {
        match self {
            Self::Payload => 0,
            Self::VidCommon => 1,
            Self::Cert2 => 2,
            Self::StateCert => 3,
            Self::CertsKnown => 4,
            Self::PayloadInfo => 5,
            Self::PayloadStored => 6,
        }
    }

    pub fn from_id(id: i16) -> Option<Self> {
        Self::ALL.into_iter().find(|part| part.id() == id)
    }
}

/// An update of the newest heights a store holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TipUpdate {
    /// A height that is new, or has new parts: its item and its parts but the payload.
    Height {
        height: u64,
        item: Bytes,
        parts: Vec<(Part, Bytes)>,
    },

    /// The newest height and the floor, after the heights sent before them.
    Bounds { newest: Option<u64>, floor: u64 },
}

/// Which keys of an application table to read: from `from` on, before `until` if set, at most
/// `limit` of them.
#[derive(Clone, Copy, Debug)]
pub struct KeyBounds<'a> {
    pub from: &'a [u8],
    pub until: Option<&'a [u8]>,
    pub limit: usize,
}
