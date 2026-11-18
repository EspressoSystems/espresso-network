//! The record of the chain an API service keeps.
//!
//! The store holds one [`Item`] per block height: the leaf, the certificates that decided it, its
//! VID common data and, at an epoch root, the light client state certificate. Payloads are kept
//! apart, since they can be several megabytes and a height can have its item without its payload.
//! The store also finds the height of a leaf, a block or a payload by its commitment.
//!
//! The ingester writes to the store, copying the decide log of its consensus node, and the API
//! service writes the heights it fills from other API services. Writes are idempotent by height, so
//! writers need not coordinate. Every committed write is announced through
//! [`ChainStore::changes`], whichever process made it, so readers learn about new heights from the
//! store alone and need no connection to the writers.
//!
//! The store checks nothing it is given; whoever writes has checked it.

use std::{fmt, ops::Range, sync::Arc};

use bytes::Bytes;
use committable::{Commitment, Committable};
use espresso_types::{Header, Leaf2, SeqTypes};
use hotshot_types::{
    data::{VidCommitment, VidCommon},
    simple_certificate::{Certificate1, Certificate2, LightClientStateUpdateCertificateV2},
};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

mod backend;
pub mod mem;
pub mod postgres;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use backend::{Backend, Column, Row};

/// The store's record of one block height.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Item {
    /// The leaf, without its payload.
    pub leaf: Leaf2,

    /// The certificate that certifies the leaf: its own if it was the newest leaf of a decide,
    /// otherwise the justify QC of the leaf after it.
    pub cert1: Certificate1<SeqTypes>,

    /// The certificate that decided the leaf directly, if one did.
    pub cert2: Option<Certificate2<SeqTypes>>,

    pub vid_common: Option<VidCommon>,

    /// The light client state certificate of the epoch the block is the root of.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
}

impl Item {
    pub fn height(&self) -> u64 {
        self.leaf.height()
    }
}

/// A commitment the store finds a height by. Blocks with the same payload share a payload
/// commitment; a lookup by it finds the earliest of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lookup {
    Leaf(Commitment<Leaf2>),
    Block(Commitment<Header>),
    Payload(VidCommitment),
}

/// The version of the encoding of an [`Item`]. A store refuses items of other versions.
const ITEM_FORMAT: u8 = 1;

/// A store over a [`Backend`]. Clones share the backend.
#[derive(Clone)]
pub struct ChainStore {
    backend: Arc<dyn Backend>,
}

impl fmt::Debug for ChainStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChainStore").finish_non_exhaustive()
    }
}

impl ChainStore {
    pub fn new(backend: impl Backend) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// Stores `item`, and `payload` if given. An item already stored at the height is kept, and a
    /// payload is only added to a height that lacks one. Heights the store has pruned are not
    /// written.
    pub async fn put(&self, item: &Item, payload: Option<Bytes>) -> anyhow::Result<()> {
        self.backend.put(row(item)?, payload).await
    }

    /// Stores the payload of `height`, unless the height has one or is pruned.
    pub async fn put_payload(&self, height: u64, payload: Bytes) -> anyhow::Result<()> {
        self.backend.put_payload(height, payload).await
    }

    pub async fn item(&self, height: u64) -> anyhow::Result<Option<Item>> {
        match self.backend.item(height).await? {
            Some(bytes) => Ok(Some(decode(&bytes)?)),
            None => Ok(None),
        }
    }

    pub async fn payload(&self, height: u64) -> anyhow::Result<Option<Bytes>> {
        self.backend.payload(height).await
    }

    pub async fn height_of(&self, lookup: &Lookup) -> anyhow::Result<Option<u64>> {
        let (column, value) = match lookup {
            Lookup::Leaf(commit) => (Column::Leaf, commit.to_string()),
            Lookup::Block(commit) => (Column::Block, commit.to_string()),
            Lookup::Payload(commit) => (Column::Payload, commit.to_string()),
        };
        self.backend.height_of(column, &value).await
    }

    /// The newest height with an item, if any.
    pub async fn newest(&self) -> anyhow::Result<Option<u64>> {
        self.backend.newest().await
    }

    /// The first height the store keeps; everything before it is pruned.
    pub async fn floor(&self) -> anyhow::Result<u64> {
        self.backend.floor().await
    }

    /// Up to `limit` ranges of heights without an item, from `from` or the floor, whichever is
    /// later, up to the newest height with one, in height order.
    pub async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>> {
        self.backend.missing_items(from, limit).await
    }

    /// Up to `limit` heights from `from` on that have an item but no payload, in height order.
    pub async fn missing_payloads(&self, from: u64, limit: usize) -> anyhow::Result<Vec<u64>> {
        self.backend.missing_payloads(from, limit).await
    }

    /// Deletes everything before `height` and stops accepting writes there.
    pub async fn prune_below(&self, height: u64) -> anyhow::Result<()> {
        self.backend.prune_below(height).await
    }

    /// A value that changes after every committed write, by any writer. A reader waiting for a
    /// height looks again whenever it changes.
    pub fn changes(&self) -> watch::Receiver<u64> {
        self.backend.changes()
    }
}

/// `item` as its backend stores it.
fn row(item: &Item) -> anyhow::Result<Row> {
    let mut bytes = vec![ITEM_FORMAT];
    bincode::serialize_into(&mut bytes, item)?;
    Ok(Row {
        height: item.height(),
        leaf: item.leaf.commit().to_string(),
        block: item.leaf.block_header().commit().to_string(),
        payload: item.leaf.payload_commitment().to_string(),
        item: bytes.into(),
    })
}

fn decode(bytes: &[u8]) -> anyhow::Result<Item> {
    match bytes.split_first() {
        Some((&ITEM_FORMAT, rest)) => Ok(bincode::deserialize(rest)?),
        Some((format, _)) => {
            anyhow::bail!("item of format {format}, this store reads {ITEM_FORMAT}")
        },
        None => anyhow::bail!("empty item"),
    }
}

#[cfg(test)]
mod tests {
    use espresso_types::{NodeState, ValidatedState};
    use hotshot_types::simple_certificate::QuorumCertificate2;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    async fn genesis() -> Item {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
        Item {
            leaf: Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await,
            cert1: QuorumCertificate2::genesis(&state, &instance, upgrade).await,
            cert2: None,
            vid_common: None,
            state_cert: None,
        }
    }

    #[tokio::test]
    async fn an_item_is_found_by_height_and_by_its_commitments() {
        let store = ChainStore::new(mem::MemBackend::default());
        let item = genesis().await;
        store
            .put(&item, Some(Bytes::from_static(b"payload")))
            .await
            .unwrap();

        assert_eq!(store.item(0).await.unwrap(), Some(item.clone()));
        assert_eq!(
            store.payload(0).await.unwrap(),
            Some(Bytes::from_static(b"payload"))
        );
        for lookup in [
            Lookup::Leaf(item.leaf.commit()),
            Lookup::Block(item.leaf.block_header().commit()),
            Lookup::Payload(item.leaf.payload_commitment()),
        ] {
            assert_eq!(
                store.height_of(&lookup).await.unwrap(),
                Some(0),
                "{lookup:?}"
            );
        }
    }

    #[test]
    fn refuses_an_item_of_another_format() {
        assert!(decode(&[ITEM_FORMAT + 1, 0, 0]).is_err());
        assert!(decode(&[]).is_err());
    }
}
