//! The record of the chain an API service keeps.
//!
//! The store holds one [`Item`] per block height: the leaf and the certificate that certifies it,
//! which the hash chain fixes. Everything else about a height is kept apart, as [`Parts`]: the
//! payload, the VID common data, the `Certificate2` that decided the block directly and, at an
//! epoch root, the light client state certificate. A height can have its item without them, and
//! they are filled in later, each when it can be checked. The store also finds the height of a
//! leaf, a block or a payload by its commitment, and records the epochs whose certificates have
//! been checked.
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
    data::{EpochNumber, VidCommitment, VidCommon},
    simple_certificate::{Certificate1, Certificate2, LightClientStateUpdateCertificateV2},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::watch;

mod backend;
pub mod mem;
pub mod postgres;
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use backend::{Backend, Column, Part, Row};

/// The store's record of one block height.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Item {
    /// The leaf, without its payload.
    pub leaf: Leaf2,

    /// The certificate that certifies the leaf: its own if it was the newest leaf of a decide,
    /// otherwise the justify QC of the leaf after it.
    pub cert1: Certificate1<SeqTypes>,
}

impl Item {
    pub fn height(&self) -> u64 {
        self.leaf.height()
    }
}

/// What the store keeps of a height apart from its item. Each is absent until it is known.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parts {
    /// The payload, as the block encodes it.
    pub payload: Option<Bytes>,

    pub vid_common: Option<VidCommon>,

    /// The certificate that decided the block directly, if one did.
    pub cert2: Option<Certificate2<SeqTypes>>,

    /// The light client state certificate of the epoch the block is the root of.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
}

impl Parts {
    fn encode(&self) -> anyhow::Result<Vec<(Part, Bytes)>> {
        let mut parts = Vec::new();
        if let Some(payload) = &self.payload {
            parts.push((Part::Payload, payload.clone()));
        }
        if let Some(common) = &self.vid_common {
            parts.push((Part::VidCommon, encode(common)?));
        }
        if let Some(cert2) = &self.cert2 {
            parts.push((Part::Cert2, encode(cert2)?));
        }
        if let Some(cert) = &self.state_cert {
            parts.push((Part::StateCert, encode(cert)?));
        }
        Ok(parts)
    }

    fn decode(parts: &[(Part, Bytes)]) -> anyhow::Result<Self> {
        let mut decoded = Self::default();
        for (part, bytes) in parts {
            match part {
                Part::Payload => decoded.payload = Some(bytes.clone()),
                Part::VidCommon => decoded.vid_common = Some(decode(bytes)?),
                Part::Cert2 => decoded.cert2 = Some(decode(bytes)?),
                Part::StateCert => decoded.state_cert = Some(decode(bytes)?),
            }
        }
        Ok(decoded)
    }
}

/// A height as the store encodes it: the item, and the parts it has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub height: u64,
    pub item: Bytes,
    pub parts: Vec<(Part, Bytes)>,
}

impl Record {
    pub fn item(&self) -> anyhow::Result<Item> {
        decode(&self.item)
    }

    pub fn parts(&self) -> anyhow::Result<Parts> {
        Parts::decode(&self.parts)
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

/// The version of the encoding of an [`Item`] and of each part but the payload. A store refuses
/// values of other versions.
const FORMAT: u8 = 1;

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

    /// Stores `item` with `parts`. An item or part already stored is kept, so a write fills only
    /// what is missing. Heights the store has pruned are not written.
    pub async fn put(&self, item: &Item, parts: &Parts) -> anyhow::Result<()> {
        self.backend.put(row(item)?, parts.encode()?).await
    }

    /// Stores `parts` of `height`, each unless the height has it or is pruned.
    pub async fn put_parts(&self, height: u64, parts: &Parts) -> anyhow::Result<()> {
        self.backend.put_parts(height, parts.encode()?).await
    }

    pub async fn item(&self, height: u64) -> anyhow::Result<Option<Item>> {
        match self.backend.item(height).await? {
            Some(bytes) => Ok(Some(decode(&bytes)?)),
            None => Ok(None),
        }
    }

    pub async fn payload(&self, height: u64) -> anyhow::Result<Option<Bytes>> {
        self.backend.part(height, Part::Payload).await
    }

    pub async fn vid_common(&self, height: u64) -> anyhow::Result<Option<VidCommon>> {
        self.decoded_part(height, Part::VidCommon).await
    }

    pub async fn cert2(&self, height: u64) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        self.decoded_part(height, Part::Cert2).await
    }

    pub async fn state_cert(
        &self,
        height: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        self.decoded_part(height, Part::StateCert).await
    }

    /// The item and parts of `height` as the store encodes them, for passing on to another store
    /// unchanged.
    pub async fn record(&self, height: u64) -> anyhow::Result<Option<Record>> {
        let Some(item) = self.backend.item(height).await? else {
            return Ok(None);
        };
        Ok(Some(Record {
            height,
            item,
            parts: self.backend.parts(height).await?,
        }))
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

    /// Up to `limit` heights from `from` on that have an item but not `part`, in height order.
    pub async fn missing_parts(
        &self,
        part: Part,
        from: u64,
        limit: usize,
    ) -> anyhow::Result<Vec<u64>> {
        self.backend.missing_parts(part, from, limit).await
    }

    /// Whether the certificates of every height of `epoch` have been checked and stored.
    pub async fn certs_checked(&self, epoch: EpochNumber) -> anyhow::Result<bool> {
        self.backend.certs_checked(*epoch).await
    }

    /// Records that the certificates of every height of `epoch` have been checked and stored.
    pub async fn mark_certs_checked(&self, epoch: EpochNumber) -> anyhow::Result<()> {
        self.backend.mark_certs_checked(*epoch).await
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

    async fn decoded_part<T: DeserializeOwned>(
        &self,
        height: u64,
        part: Part,
    ) -> anyhow::Result<Option<T>> {
        match self.backend.part(height, part).await? {
            Some(bytes) => Ok(Some(decode(&bytes)?)),
            None => Ok(None),
        }
    }
}

/// `item` as its backend stores it.
fn row(item: &Item) -> anyhow::Result<Row> {
    Ok(Row {
        height: item.height(),
        leaf: item.leaf.commit().to_string(),
        block: item.leaf.block_header().commit().to_string(),
        payload: item.leaf.payload_commitment().to_string(),
        item: encode(item)?,
    })
}

fn encode<T: Serialize>(value: &T) -> anyhow::Result<Bytes> {
    let mut bytes = vec![FORMAT];
    bincode::serialize_into(&mut bytes, value)?;
    Ok(bytes.into())
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<T> {
    match bytes.split_first() {
        Some((&FORMAT, rest)) => Ok(bincode::deserialize(rest)?),
        Some((format, _)) => anyhow::bail!("a value of format {format}, this store reads {FORMAT}"),
        None => anyhow::bail!("an empty value"),
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
        }
    }

    fn payload_only(payload: &'static [u8]) -> Parts {
        Parts {
            payload: Some(Bytes::from_static(payload)),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn an_item_is_found_by_height_and_by_its_commitments() {
        let store = ChainStore::new(mem::MemBackend::default());
        let item = genesis().await;
        store.put(&item, &payload_only(b"payload")).await.unwrap();

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

    /// A record carries the item and parts as stored, and decodes to them.
    #[tokio::test]
    async fn a_record_decodes_to_what_was_stored() {
        let store = ChainStore::new(mem::MemBackend::default());
        let item = genesis().await;
        store.put(&item, &payload_only(b"payload")).await.unwrap();
        let record = store.record(0).await.unwrap().unwrap();
        assert_eq!(record.item().unwrap(), item);
        assert_eq!(record.parts().unwrap(), payload_only(b"payload"));
        assert_eq!(store.record(1).await.unwrap(), None);
    }

    #[test]
    fn refuses_an_item_of_another_format() {
        assert!(decode::<Item>(&[FORMAT + 1, 0, 0]).is_err());
        assert!(decode::<Item>(&[]).is_err());
    }
}
