//! The record of the chain an API service keeps.
//!
//! The store holds one [`Item`] per block height: the leaf and the certificate that certifies it,
//! which the hash chain fixes. Everything else about a height is kept apart, as [`Parts`]: the
//! payload, the VID common data, the `Certificate2` that decided the block directly and, at an
//! epoch root, the light client state certificate. A height can have its item without them, and
//! they are filled in later, each when it can be checked. The store also finds the height of a
//! leaf, a block or a payload by its commitment.
//!
//! The storage instances write to the store, copying the decide log of their consensus node, and
//! repairing API instances write the heights they fill from other API services. Writes are idempotent by height, so
//! writers need not coordinate. Every committed write is announced through
//! [`ChainStore::changes`], whichever process made it, so readers learn about new heights from the
//! store alone and need no connection to the writers.
//!
//! The store checks nothing it is given; whoever writes has checked it.

use std::{fmt, ops::Range, sync::Arc, time::Duration};

use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use committable::{Commitment, Committable};
use espresso_types::{Header, Leaf2, NsTable, Payload, SeqTypes};
use hotshot_types::{
    data::{VidCommitment, VidCommon},
    simple_certificate::{Certificate1, Certificate2, LightClientStateUpdateCertificateV2},
    traits::{BlockPayload, block_contents::BlockHeader},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::watch;

mod backend;
pub mod mem;
pub mod payloads;
#[cfg(feature = "postgres")]
pub mod postgres;
mod recent;
#[cfg(feature = "remote")]
pub mod remote;
#[cfg(feature = "service")]
pub mod service;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod tip;
#[cfg(any(feature = "remote", feature = "service"))]
pub mod wire;

pub use app_tables::{AppTables, AppWrite, TableName};
pub use backend::{Backend, Column, KeyBounds, Part, Row, TipUpdate};

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

    /// The BLAKE3 hash and size of the payload, which the store computes when the payload is
    /// written, so that readers need not read the payload to describe it.
    pub payload_info: Option<PayloadInfo>,

    pub vid_common: Option<VidCommon>,

    /// The certificate that decided the block directly, if one did.
    pub cert2: Option<Certificate2<SeqTypes>>,

    /// The light client state certificate of the epoch the block is the root of.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,

    /// Whether the payload is in the store's object store, for a store that keeps its payloads
    /// there.
    pub payload_stored: bool,

    /// Whether `cert2` and `state_cert` are as complete as consensus made them: true for a height
    /// from the decide log, or one whose certificates were checked from a peer that knew them. A
    /// height without it may lack certificates it has.
    pub certs_known: bool,
}

/// What readers need to know of a payload without reading it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadInfo {
    /// The BLAKE3 hash of the payload's bytes.
    pub blake3: [u8; 32],

    pub size: u64,

    /// How many transactions the payload holds, which its block's header tells how to count;
    /// absent if the header was not stored when the payload was.
    pub num_transactions: Option<u64>,
}

impl PayloadInfo {
    /// The info of `payload`, whose namespace table, which its block's header holds, is
    /// `ns_table`, if known.
    pub fn of(payload: &[u8], ns_table: Option<&NsTable>) -> Self {
        let num_transactions = ns_table.map(|ns_table| {
            Payload::from_bytes(payload, ns_table).num_transactions(ns_table) as u64
        });
        Self {
            blake3: *blake3::hash(payload).as_bytes(),
            size: payload.len() as u64,
            num_transactions,
        }
    }
}

impl Parts {
    /// The parts to write, of a block whose header is `header`, if known. The info of a payload
    /// is computed here, so every writer stores it.
    fn encode(&self, header: Option<&Header>) -> anyhow::Result<Vec<(Part, Bytes)>> {
        let mut parts = Vec::new();
        if let Some(payload) = &self.payload {
            parts.push((Part::Payload, payload.clone()));
        }
        let info = match (&self.payload, self.payload_info) {
            (_, Some(info)) => Some(info),
            (Some(payload), None) => Some(PayloadInfo::of(payload, header.map(|h| h.metadata()))),
            (None, None) => None,
        };
        if let Some(info) = info {
            parts.push((Part::PayloadInfo, encode(&info)?));
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
        if self.certs_known {
            parts.push((Part::CertsKnown, Bytes::new()));
        }
        if self.payload_stored {
            parts.push((Part::PayloadStored, Bytes::new()));
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
                Part::CertsKnown => decoded.certs_known = true,
                Part::PayloadInfo => decoded.payload_info = Some(decode(bytes)?),
                Part::PayloadStored => decoded.payload_stored = true,
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

/// The application tables of the store, which hold what the API instances serve besides blocks.
#[async_trait]
impl AppTables for ChainStore {
    async fn get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>> {
        self.app_get(table, key).await
    }

    async fn entries(
        &self,
        table: &TableName,
        from: &[u8],
        until: Option<&[u8]>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>> {
        self.app_entries(table, from, until).await
    }

    async fn write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()> {
        self.app_write(writes).await
    }
}

/// The version of the encoding of an [`Item`] and of each part but the payload. A store refuses
/// values of other versions.
const FORMAT: u8 = 1;

/// How many application table entries [`ChainStore::app_entries`] asks the backend for at a time.
const APP_PAGE: usize = 1024;

/// A store over a [`Backend`]. Clones share the backend.
#[derive(Clone)]
pub struct ChainStore {
    backend: Arc<dyn Backend>,

    /// Where the payloads live, if not in the backend: an object store, which a writer may stage
    /// payloads for.
    payloads: Option<payloads::PayloadStore>,
    staging: Option<payloads::Staging>,

    /// The newest payloads written through this store, for readers in the same process.
    recent: Option<Arc<recent::RecentPayloads>>,
}

impl fmt::Debug for ChainStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChainStore").finish_non_exhaustive()
    }
}

impl ChainStore {
    pub fn new(backend: impl Backend) -> Self {
        Self::shared(Arc::new(backend))
    }

    /// A store over `backend`, which others use too.
    pub fn shared(backend: Arc<dyn Backend>) -> Self {
        Self {
            backend,
            payloads: None,
            staging: None,
            recent: None,
        }
    }

    /// The store, keeping its payloads in `payloads` instead of the backend, uploading each one
    /// as it is written.
    pub fn with_payloads(mut self, payloads: payloads::PayloadStore) -> Self {
        self.payloads = Some(payloads);
        self
    }

    /// The store, keeping its payloads in `payloads` instead of the backend, staging each one in
    /// `dir` as it is written, so that a write is durable before the upload, with the uploader
    /// that uploads them, which must run as long as the store is written to.
    pub async fn with_staged_payloads(
        mut self,
        payloads: payloads::PayloadStore,
        dir: std::path::PathBuf,
    ) -> anyhow::Result<(Self, payloads::Uploader)> {
        let (staging, uploader) =
            payloads::staging(dir, payloads.clone(), self.backend.clone()).await?;
        self.payloads = Some(payloads);
        self.staging = Some(staging);
        Ok((self, uploader))
    }

    /// The store, keeping the newest payloads written through it in memory, up to `bytes` of
    /// them, for its clones to read without reading the object store. For a process that both
    /// writes payloads and serves them.
    pub fn with_recent_payloads(mut self, bytes: usize) -> Self {
        self.recent = Some(Arc::new(recent::RecentPayloads::new(bytes)));
        self
    }

    /// The object store the payloads live in, if not in the backend.
    pub fn payload_store(&self) -> Option<&payloads::PayloadStore> {
        self.payloads.as_ref()
    }

    /// Makes `payload`, whose VID commitment is `commitment` and whose namespace table is
    /// `ns_table`, if known, durable in the object store: staged, or uploaded if this store does
    /// not stage. Records what describes it under its commitment, so that every height whose
    /// block has it learns its info, and whether it is uploaded, when its item is written. A
    /// payload can be stored before any block that has it is decided.
    pub async fn put_payload(
        &self,
        commitment: &str,
        payload: Bytes,
        ns_table: Option<&NsTable>,
    ) -> anyhow::Result<PayloadInfo> {
        let payloads = self
            .payloads
            .as_ref()
            .context("this store keeps no payloads by commitment")?;
        let info = PayloadInfo::of(&payload, ns_table);
        let kept = payload.clone();
        let uploaded = match &self.staging {
            Some(staging) => {
                staging.stage(commitment, &payload).await?;
                false
            },
            None => {
                payloads.put(commitment, payload).await?;
                true
            },
        };
        payloads::put_record(
            self.backend.as_ref(),
            commitment,
            &payloads::PayloadRecord { info, uploaded },
        )
        .await?;
        if let Some(recent) = &self.recent {
            recent.insert(commitment, kept);
        }
        // A block with the payload may be stored already, without its info; see `put`.
        if let Some(height) = self.backend.height_of(Column::Payload, commitment).await? {
            self.describe_payload(height, commitment).await?;
        }
        Ok(info)
    }

    /// `parts` of the block at `height`, whose header is `header`, as the backend keeps them: for a
    /// store whose payloads live in the object store, a payload goes there, and the parts carry
    /// what the payload's record says of it.
    async fn externalize(&self, header: &Header, parts: &Parts) -> anyhow::Result<Parts> {
        if self.payloads.is_none() {
            return Ok(parts.clone());
        }
        let commitment = header.payload_commitment().to_string();
        let record = match &parts.payload {
            Some(payload) => {
                self.put_payload(&commitment, payload.clone(), Some(header.metadata()))
                    .await?;
                payloads::record(self.backend.as_ref(), &commitment).await?
            },
            None => payloads::record(self.backend.as_ref(), &commitment).await?,
        };
        let mut kept = Parts {
            payload: None,
            ..parts.clone()
        };
        if let Some(record) = record {
            kept.payload_info = kept.payload_info.or(Some(record.info));
            kept.payload_stored |= record.uploaded;
        }
        Ok(kept)
    }

    /// Stores `item` with `parts`. An item or part already stored is kept, so a write fills only
    /// what is missing. Heights the store has pruned are not written.
    pub async fn put(&self, item: &Item, parts: &Parts) -> anyhow::Result<()> {
        let header = item.leaf.block_header();
        let kept = self.externalize(header, parts).await?;
        self.backend
            .put(row(item)?, kept.encode(Some(header))?)
            .await?;
        // A payload whose record was written after this item read it, but before the item was
        // stored, would leave the item without its payload's info for good: the writer of the
        // record looked for the item too early. Looking again after the write closes that gap.
        if self.payloads.is_some() && kept.payload_info.is_none() {
            let commitment = header.payload_commitment().to_string();
            self.describe_payload(item.height(), &commitment).await?;
        }
        Ok(())
    }

    /// Gives the stored height `height` its payload's info, and whether it is uploaded, from the
    /// record under `commitment`, if both are there.
    async fn describe_payload(&self, height: u64, commitment: &str) -> anyhow::Result<()> {
        let Some(record) = payloads::record(self.backend.as_ref(), commitment).await? else {
            return Ok(());
        };
        let mut parts = vec![(Part::PayloadInfo, encode(&record.info)?)];
        if record.uploaded {
            parts.push((Part::PayloadStored, Bytes::new()));
        }
        self.backend.put_parts(height, parts).await
    }

    /// Stores `parts` of `height`, each unless the height has it or is pruned.
    pub async fn put_parts(&self, height: u64, parts: &Parts) -> anyhow::Result<()> {
        // A payload's info counts its transactions by the header, if the store has it.
        let item = match parts.payload.is_some()
            && (parts.payload_info.is_none() || self.payloads.is_some())
        {
            true => self.item(height).await?,
            false => None,
        };
        let header = item.as_ref().map(|item| item.leaf.block_header());
        let parts = match (header, self.payloads.is_some() && parts.payload.is_some()) {
            (Some(header), true) => self.externalize(header, parts).await?,
            // The object's key is the payload commitment, which the item names.
            (None, true) => anyhow::bail!("the payload of height {height} came before its block"),
            (_, false) => parts.clone(),
        };
        self.backend.put_parts(height, parts.encode(header)?).await
    }

    pub async fn item(&self, height: u64) -> anyhow::Result<Option<Item>> {
        match self.backend.item(height).await? {
            Some(bytes) => Ok(Some(decode(&bytes)?)),
            None => Ok(None),
        }
    }

    /// The bytes in `range` of the payload of `height`: read from the object store alone for a
    /// store that keeps its payloads there, once uploaded, and cut from the whole payload
    /// otherwise.
    pub async fn payload_range(
        &self,
        height: u64,
        range: Range<u64>,
    ) -> anyhow::Result<Option<Bytes>> {
        let Some(payloads) = &self.payloads else {
            let Some(payload) = self.backend.part(height, Part::Payload).await? else {
                return Ok(None);
            };
            let (start, end) = (usize::try_from(range.start)?, usize::try_from(range.end)?);
            anyhow::ensure!(end <= payload.len(), "a range past the end of the payload");
            return Ok(Some(payload.slice(start..end)));
        };
        let Some(item) = self.item(height).await? else {
            return Ok(None);
        };
        let commitment = item.leaf.block_header().payload_commitment().to_string();
        if let Some(payload) = self.recent.as_ref().and_then(|r| r.get(&commitment)) {
            let (start, end) = (usize::try_from(range.start)?, usize::try_from(range.end)?);
            anyhow::ensure!(
                start <= end && end <= payload.len(),
                "a range past the end of the payload"
            );
            return Ok(Some(payload.slice(start..end)));
        }
        payloads.get_range(&commitment, range).await
    }

    /// The payload of `height`, from the object store for a store that keeps its payloads there,
    /// once it is uploaded.
    pub async fn payload(&self, height: u64) -> anyhow::Result<Option<Bytes>> {
        let Some(payloads) = &self.payloads else {
            return self.backend.part(height, Part::Payload).await;
        };
        let Some(item) = self.item(height).await? else {
            return Ok(None);
        };
        let commitment = item.leaf.block_header().payload_commitment().to_string();
        if let Some(payload) = self.recent.as_ref().and_then(|r| r.get(&commitment)) {
            return Ok(Some(payload));
        }
        payloads.get(&commitment).await
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
        self.read_record(height, true).await
    }

    /// The record of `height` without its payload, which [`Parts::payload_info`] describes.
    pub async fn summary(&self, height: u64) -> anyhow::Result<Option<Record>> {
        self.read_record(height, false).await
    }

    async fn read_record(&self, height: u64, payload: bool) -> anyhow::Result<Option<Record>> {
        let Some((item, mut parts)) = self.backend.record(height, payload).await? else {
            return Ok(None);
        };
        // A payload in the object store joins the record from there, once uploaded.
        if payload
            && self.payloads.is_some()
            && let Some(bytes) = self.payload(height).await?
        {
            parts.insert(0, (Part::Payload, bytes));
        }
        Ok(Some(Record {
            height,
            item,
            parts,
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

    /// The newest range of heights without an item, at or above the floor and below the newest
    /// height with one. Every height from its end up to the newest has an item.
    pub async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>> {
        self.backend.newest_gap().await
    }

    /// Up to `limit` heights from `from` on that have an item but not `part`, in height order.
    pub async fn missing_parts(
        &self,
        part: Part,
        from: u64,
        limit: usize,
    ) -> anyhow::Result<Vec<u64>> {
        if part != Part::Payload || self.payloads.is_none() {
            return self.backend.missing_parts(part, from, limit).await;
        }
        // A store whose payloads live in the object store has them once they are uploaded. A
        // height is marked when its item is written after the upload; one written before is
        // marked here, when its payload's record says the upload has happened since.
        let unmarked = self
            .backend
            .missing_parts(Part::PayloadStored, from, limit)
            .await?;
        let mut missing = Vec::new();
        for height in unmarked {
            let uploaded = match self.item(height).await? {
                Some(item) => {
                    let commitment = item.leaf.block_header().payload_commitment().to_string();
                    payloads::record(self.backend.as_ref(), &commitment)
                        .await?
                        .is_some_and(|record| record.uploaded)
                },
                None => false,
            };
            match uploaded {
                true => {
                    self.backend
                        .put_parts(height, vec![(Part::PayloadStored, Bytes::new())])
                        .await?
                },
                false => missing.push(height),
            }
        }
        Ok(missing)
    }

    /// The value of `key` in the application table `table`.
    pub async fn app_get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>> {
        self.backend.app_get(table, key).await
    }

    /// The entries of the application table `table` with keys from `from` on, and before
    /// `until` if set, in bytewise key order. They are read a page at a time, so no single answer
    /// grows with the table.
    pub async fn app_entries(
        &self,
        table: &TableName,
        from: &[u8],
        until: Option<&[u8]>,
    ) -> anyhow::Result<Vec<(Bytes, Bytes)>> {
        let mut entries: Vec<(Bytes, Bytes)> = Vec::new();
        let mut next = from.to_vec();
        loop {
            let keys = KeyBounds {
                from: &next,
                until,
                limit: APP_PAGE,
            };
            // A backend may answer fewer than asked for; only an empty page is the end.
            let page = self.backend.app_entries(table, keys).await?;
            let Some((last, _)) = page.last() else {
                return Ok(entries);
            };
            // The least key after the last one read.
            next = [&last[..], &[0]].concat();
            entries.extend(page);
        }
    }

    /// Applies `writes` to the application tables, in order, in one commit.
    pub async fn app_write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()> {
        self.backend.app_write(writes).await
    }

    /// Deletes everything before `height` and stops accepting writes there.
    pub async fn prune_below(&self, height: u64) -> anyhow::Result<()> {
        self.backend.prune_below(height).await
    }

    /// The backend's updates of the newest heights from `from` on, if it sends them.
    pub(crate) async fn tip_updates(
        &self,
        from: u64,
    ) -> anyhow::Result<Option<futures::stream::BoxStream<'static, anyhow::Result<TipUpdate>>>>
    {
        self.backend.tip_updates(from).await
    }

    /// Takes or renews the lease `name` for `holder` for `ttl`; see [`Backend::lease`].
    pub async fn lease(&self, name: &str, holder: &str, ttl: Duration) -> anyhow::Result<bool> {
        self.backend.lease(name, holder, ttl).await
    }

    /// A value that changes after every committed write to the chain, by any writer: items,
    /// parts and pruning, but not the application tables. A reader waiting for a height looks
    /// again whenever it changes.
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

    /// A table longer than a page is read whole, and only up to `until` if set.
    #[tokio::test]
    async fn app_entries_read_every_page() {
        let store = ChainStore::new(mem::MemBackend::default());
        let t = TableName::of("t");
        let count = 2 * APP_PAGE as u32 + 5;
        let writes = (0..count)
            .map(|i| AppWrite::Put {
                table: t.clone(),
                key: Bytes::copy_from_slice(&i.to_be_bytes()),
                value: Bytes::new(),
            })
            .collect();
        store.app_write(writes).await.unwrap();

        let all = store.app_entries(&t, &[], None).await.unwrap();
        assert_eq!(all.len(), count as usize);
        let until = 1500u32.to_be_bytes();
        let before = store.app_entries(&t, &[], Some(&until)).await.unwrap();
        assert_eq!(before.len(), 1500);
        assert_eq!(before.last().unwrap().0[..], 1499u32.to_be_bytes());
    }

    /// A record carries the item and parts as stored, and decodes to them, with the payload's
    /// info computed when it was written; a summary leaves out only the payload.
    #[tokio::test]
    async fn a_record_decodes_to_what_was_stored() {
        let store = ChainStore::new(mem::MemBackend::default());
        let item = genesis().await;
        store.put(&item, &payload_only(b"payload")).await.unwrap();
        let record = store.record(0).await.unwrap().unwrap();
        assert_eq!(record.item().unwrap(), item);
        let info = PayloadInfo::of(b"payload", Some(item.leaf.block_header().metadata()));
        assert_eq!(info.size, 7);
        assert_eq!(
            record.parts().unwrap(),
            Parts {
                payload_info: Some(info),
                ..payload_only(b"payload")
            }
        );
        let summary = store.summary(0).await.unwrap().unwrap();
        assert_eq!(
            summary.parts().unwrap(),
            Parts {
                payload_info: Some(info),
                ..Default::default()
            }
        );
        assert_eq!(store.record(1).await.unwrap(), None);
    }

    #[test]
    fn refuses_an_item_of_another_format() {
        assert!(decode::<Item>(&[FORMAT + 1, 0, 0]).is_err());
        assert!(decode::<Item>(&[]).is_err());
    }
}
