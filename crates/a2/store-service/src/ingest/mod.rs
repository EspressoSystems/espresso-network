//! The storage instance's share of the decide log: it copies the entries consensus hands it into
//! the store, with the state after each block as consensus sends it, and the stake tables and
//! discovery records the stream carries, and acknowledges each entry, which lets consensus free
//! it: once the store has its payload, or once the entry is final and no payload can come any
//! more.
//!
//! Consensus shares its entries out among the storage instances of the deployment, which all
//! write to one store, so each instance sees only some heights. It checks each entry against the
//! leaves the store holds before and after it, so that one pointed at the wrong node, or a store
//! restored to the wrong place, does not mix two chains. Data from the node's own consensus is not
//! verified otherwise. A payload that joins an entry later can go to another instance than the
//! entry did; whichever stores the payload acknowledges the entry once the store also has it.

use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use anyhow::{Context, anyhow, ensure};
use async_trait::async_trait;
use bytes::Bytes;
use committable::Committable;
use consensus_api::{
    BODY_FORMAT, DecideLogClient, FINALITY_VIEWS,
    proto::{AckRequest, Decided, HeightRange, State, SubscribeRequest, subscribe_response::Event},
};
use futures::StreamExt;
use hotshot_types::{
    data::{Leaf2, VidCommitment, VidCommon, ViewNumber, ns_table::parse_ns_table},
    simple_certificate::{Certificate1, Certificate2, LightClientStateUpdateCertificateV2},
    traits::{
        block_contents::{BlockHeader, BlockPayload, EncodeBytes, GENESIS_VID_NUM_STORAGE_NODES},
        node_implementation::NodeType,
    },
    vid::{
        advz::advz_scheme,
        avidm::{AvidMScheme, init_avidm_param},
        avidm_gf2::{AvidmGf2Common, AvidmGf2Scheme, init_avidm_gf2_param},
    },
};
use jf_advz::VidScheme;
use tokio::time::interval;

pub mod payloads;
mod store;

/// How long to wait before reconnecting after the connection fails.
pub(crate) const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// How often to acknowledge what was stored, in one call for all of it.
const ACK_INTERVAL: Duration = Duration::from_millis(50);

/// Where the entries are written.
#[async_trait]
pub trait Sink<Types: NodeType>: Send + Sync {
    /// Makes `block` durable.
    async fn append(&self, block: Block<Types>) -> anyhow::Result<()>;

    /// Adds `payload` to the block stored at `height`, which was stored without it.
    async fn add_payload(&self, height: u64, payload: Vec<u8>) -> anyhow::Result<()>;

    /// The leaf of the newest block the sink holds.
    async fn newest_leaf(&self) -> anyhow::Result<Option<Leaf2<Types>>>;

    /// The leaf of the block the sink holds at `height`.
    async fn leaf(&self, height: u64) -> anyhow::Result<Option<Leaf2<Types>>>;

    /// Whether the sink holds the payload of the block at `height`.
    async fn has_payload(&self, height: u64) -> anyhow::Result<bool>;

    /// Makes the encoded stake table of `epoch` durable.
    async fn put_stake_table(&self, epoch: u64, table: Bytes) -> anyhow::Result<()>;

    /// Makes the encoded discovery records of `epoch` durable.
    async fn put_service_records(&self, epoch: u64, records: Vec<Bytes>) -> anyhow::Result<()>;
}

/// A decided block, as it goes to the sink.
#[derive(Clone, Debug, PartialEq)]
pub struct Block<Types: NodeType> {
    pub leaf: Leaf2<Types>,

    /// The certificate that certifies the leaf.
    pub cert1: Certificate1<Types>,

    /// The certificate that decided the leaf directly, if one did.
    pub cert2: Option<Certificate2<Types>>,

    pub vid_common: Option<VidCommon>,

    /// The light client state certificate of the epoch the block is the root of.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<Types>>,

    /// The encoded payload; absent if consensus did not have it.
    pub payload: Option<Vec<u8>>,

    /// The state after the block as consensus sends it; absent if consensus did not validate the
    /// block.
    pub state: Option<State>,
}

/// Ingests from the decide log at `addr` into `sink`, reconnecting whenever the connection fails,
/// until an entry breaks the chain the sink holds or cannot be taken at all. Such an entry would
/// come again after every reconnect.
pub async fn run<Types: NodeType>(addr: SocketAddr, sink: &dyn Sink<Types>) -> anyhow::Error {
    loop {
        match ingest(addr, sink).await {
            Ok(()) => tracing::warn!(%addr, "decide log stream ended; reconnecting"),
            Err(err) if err.is::<ChainBroken>() || err.is::<BadEntry>() => {
                tracing::error!(%addr, "stopped ingesting: {err:#}");
                return err;
            },
            Err(err) => tracing::warn!(%addr, "ingesting failed; reconnecting: {err:#}"),
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// An entry that does not fit the chain the sink holds.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ChainBroken(String);

/// An entry that cannot be taken, such as one that does not decode, or a stream that breaks the
/// decide log's protocol.
#[derive(Debug, thiserror::Error)]
#[error("{0:#}")]
pub struct BadEntry(anyhow::Error);

/// Ingests until the stream ends or fails. What this instance was sent and did not acknowledge
/// goes to another subscription, or to this one after it reconnects, so nothing is missed.
async fn ingest<Types: NodeType>(addr: SocketAddr, sink: &dyn Sink<Types>) -> anyhow::Result<()> {
    let mut client = DecideLogClient::connect(format!("http://{addr}"))
        .await
        .context("connecting to the decide log")?;
    let mut stream = client.subscribe(SubscribeRequest {}).await?.into_inner();
    let mut chain = Chain::default();
    let mut receiving: Option<Receiving<Types>> = None;
    let mut acks = interval(ACK_INTERVAL);
    loop {
        tokio::select! {
            response = stream.next() => {
                let Some(response) = response else {
                    return Ok(());
                };
                // An event this instance does not know decodes to none, and would come again.
                let event = response?
                    .event
                    .ok_or_else(|| BadEntry(anyhow!("a subscribe response without an event")))?;
                receive(sink, &mut chain, &mut receiving, event).await?;
            },
            _ = acks.tick() => {
                let ranges = chain.take_ackable();
                if !ranges.is_empty() {
                    client.ack(AckRequest { ranges }).await?;
                }
            },
        }
    }
}

/// What the stream is sending in chunks: the payload of an entry, or one added to an entry
/// stored earlier.
enum Receiving<Types: NodeType> {
    Entry(Box<Entry<Types>>, Vec<u8>),
    Payload {
        height: u64,
        size: u64,
        data: Vec<u8>,
    },
}

/// Takes `event` into the chain and the sink.
async fn receive<Types: NodeType>(
    sink: &dyn Sink<Types>,
    chain: &mut Chain,
    receiving: &mut Option<Receiving<Types>>,
    event: Event,
) -> anyhow::Result<()> {
    if receiving.is_some() {
        let Event::PayloadChunk(chunk) = event else {
            return Err(BadEntry(anyhow!("a payload ended early")).into());
        };
        return receive_chunk(sink, chain, receiving, chunk.data).await;
    }
    match event {
        Event::Start(start) => {
            if let Some(earliest) = start.earliest_height {
                note_missing(sink, earliest).await?;
            }
        },
        Event::Progress(progress) => chain.progress(progress.newest_view),
        Event::Decided(decided) => {
            let size = decided.payload_size;
            let entry = decode(decided).map_err(BadEntry)?;
            check(sink, &entry).await?;
            match size {
                Some(size) => {
                    *receiving = Some(Receiving::Entry(
                        Box::new(entry),
                        Vec::with_capacity(size as usize),
                    ));
                    return receive_chunk(sink, chain, receiving, Bytes::new()).await;
                },
                None => store(sink, chain, entry, None).await?,
            }
        },
        Event::PayloadAdded(added) => {
            *receiving = Some(Receiving::Payload {
                height: added.height,
                size: added.payload_size,
                data: Vec::with_capacity(added.payload_size as usize),
            });
            return receive_chunk(sink, chain, receiving, Bytes::new()).await;
        },
        Event::PayloadChunk(_) => {
            return Err(BadEntry(anyhow!("a payload chunk without an entry")).into());
        },
        Event::StakeTable(table) => sink.put_stake_table(table.epoch, table.table).await?,
        Event::ServiceRecords(records) => {
            sink.put_service_records(records.epoch, records.records)
                .await?
        },
    }
    Ok(())
}

/// Adds `data` to the payload being received, and stores it once it is complete.
async fn receive_chunk<Types: NodeType>(
    sink: &dyn Sink<Types>,
    chain: &mut Chain,
    receiving: &mut Option<Receiving<Types>>,
    data: Bytes,
) -> anyhow::Result<()> {
    let (received, expected) = match receiving.as_mut().expect("a payload is being received") {
        Receiving::Entry(entry, payload) => {
            payload.extend_from_slice(&data);
            (payload.len(), entry.payload_size)
        },
        Receiving::Payload {
            size,
            data: payload,
            ..
        } => {
            payload.extend_from_slice(&data);
            (payload.len(), *size)
        },
    };
    if received as u64 > expected {
        return Err(BadEntry(anyhow!("a payload is longer than announced")).into());
    }
    if (received as u64) < expected {
        return Ok(());
    }
    match receiving.take().expect("a payload is being received") {
        Receiving::Entry(entry, payload) => store(sink, chain, *entry, Some(payload)).await,
        Receiving::Payload { height, data, .. } => {
            sink.add_payload(height, data).await?;
            // The entry may have gone to another instance; it is complete once the store has it.
            if sink.leaf(height).await?.is_some() {
                chain.complete(height);
            }
            Ok(())
        },
    }
}

/// Stores `entry`, with `payload` if consensus had it.
async fn store<Types: NodeType>(
    sink: &dyn Sink<Types>,
    chain: &mut Chain,
    entry: Entry<Types>,
    payload: Option<Vec<u8>>,
) -> anyhow::Result<()> {
    let (height, view) = (entry.leaf.height(), entry.view);
    let with_payload = payload.is_some();
    chain.progress(view);
    let block = finish(entry.into_block(payload)).map_err(BadEntry)?;
    sink.append(block).await?;
    // A payload that joined the entry later may have reached the store through another instance.
    let complete = with_payload || sink.has_payload(height).await?;
    chain.stored(height, view, complete);
    Ok(())
}

/// A decide log entry with its consensus objects decoded.
struct Entry<Types: NodeType> {
    leaf: Leaf2<Types>,
    view: u64,
    payload_size: u64,
    cert1: Certificate1<Types>,
    cert2: Option<Certificate2<Types>>,
    vid_common: Option<AvidmGf2Common>,
    state_cert: Option<LightClientStateUpdateCertificateV2<Types>>,
    state: Option<State>,
}

impl<Types: NodeType> Entry<Types> {
    fn into_block(self, payload: Option<Vec<u8>>) -> Block<Types> {
        Block {
            leaf: self.leaf,
            cert1: self.cert1,
            cert2: self.cert2,
            vid_common: self.vid_common.map(VidCommon::V2),
            state_cert: self.state_cert,
            payload,
            state: self.state,
        }
    }
}

/// Logs the heights before `earliest`, the earliest entry consensus keeps, that the store lacks:
/// they must come from elsewhere.
async fn note_missing<Types: NodeType>(
    sink: &dyn Sink<Types>,
    earliest: u64,
) -> anyhow::Result<()> {
    let Some(newest) = sink.newest_leaf().await? else {
        return Ok(());
    };
    if earliest > newest.height() + 1 {
        tracing::warn!(
            missing = ?(newest.height() + 1..earliest),
            "consensus no longer holds these heights; the API service fills them"
        );
    }
    Ok(())
}

/// Checks that `entry` fits the leaves the sink holds at the heights before and after it, and any
/// leaf it holds at its own height.
async fn check<Types: NodeType>(
    sink: &dyn Sink<Types>,
    entry: &Entry<Types>,
) -> anyhow::Result<()> {
    let height = entry.leaf.height();
    let commitment = entry.leaf.commit();
    if let Some(stored) = sink.leaf(height).await?
        && stored.commit() != commitment
    {
        return Err(ChainBroken(format!(
            "leaf {commitment} at height {height}, but the chain has {}",
            stored.commit()
        ))
        .into());
    }
    if let Some(before) = height.checked_sub(1)
        && let Some(parent) = sink.leaf(before).await?
        && parent.commit() != entry.leaf.parent_commitment()
    {
        return Err(ChainBroken(format!(
            "leaf at height {height} names parent {}, but the leaf before it is {}",
            entry.leaf.parent_commitment(),
            parent.commit()
        ))
        .into());
    }
    if let Some(child) = sink.leaf(height + 1).await?
        && child.parent_commitment() != commitment
    {
        return Err(ChainBroken(format!(
            "leaf {commitment} at height {height} is not the parent of the leaf after it"
        ))
        .into());
    }
    Ok(())
}

/// The heights this instance stored and has not acknowledged yet, and the newest view, which
/// tells when they are final.
///
/// A height is final once its view is [`FINALITY_VIEWS`] behind the newest view; then nothing
/// more comes for it.
#[derive(Default)]
struct Chain {
    /// Stored heights not acknowledged yet, with their view and whether the store has their
    /// payload.
    unacked: BTreeMap<u64, (u64, bool)>,

    newest_view: u64,
}

impl Chain {
    fn progress(&mut self, view: u64) {
        self.newest_view = self.newest_view.max(view);
    }

    fn stored(&mut self, height: u64, view: u64, complete: bool) {
        self.unacked.insert(height, (view, complete));
    }

    /// Notes that the store has the payload of `height`, and its entry.
    fn complete(&mut self, height: u64) {
        self.unacked
            .entry(height)
            .and_modify(|(_, complete)| *complete = true)
            .or_insert((0, true));
    }

    /// The ranges of consecutive stored heights that have their payloads or are final, which
    /// it then forgets. A final height without its payload is left to the API service.
    fn take_ackable(&mut self) -> Vec<HeightRange> {
        let newest = self.newest_view;
        let ackable: Vec<u64> = self
            .unacked
            .iter()
            .filter(|(_, (view, complete))| *complete || view + FINALITY_VIEWS <= newest)
            .map(|(height, _)| *height)
            .collect();
        let mut ranges: Vec<HeightRange> = Vec::new();
        for height in ackable {
            if !self
                .unacked
                .remove(&height)
                .expect("ackable heights are unacked")
                .1
            {
                tracing::warn!(height, "the payload of a final height is not stored yet");
            }
            match ranges.last_mut() {
                Some(range) if range.end == height => range.end += 1,
                _ => ranges.push(HeightRange {
                    start: height,
                    end: height + 1,
                }),
            }
        }
        ranges
    }
}

fn decode<Types: NodeType>(entry: Decided) -> anyhow::Result<Entry<Types>> {
    let body = entry.body.context("entry without a body")?;
    ensure!(
        body.format == BODY_FORMAT,
        "entry body has format {}, this instance reads {BODY_FORMAT}",
        body.format
    );
    let leaf: Leaf2<Types> = bincode::deserialize(&body.leaf).context("decoding the leaf")?;
    ensure!(
        leaf.height() == entry.height,
        "entry at height {} holds a leaf of height {}",
        entry.height,
        leaf.height()
    );
    ensure!(
        leaf.commit().to_string() == entry.leaf_commitment,
        "entry at height {} names a leaf commitment its leaf does not have",
        entry.height
    );
    Ok(Entry {
        leaf,
        view: entry.view,
        payload_size: entry.payload_size.unwrap_or_default(),
        cert1: bincode::deserialize(&body.cert1).context("decoding the cert1")?,
        cert2: optional(&body.cert2).context("decoding the cert2")?,
        vid_common: optional(&body.vid_common).context("decoding the VID common data")?,
        state_cert: optional(&body.state_cert).context("decoding the state certificate")?,
        state: body.state,
    })
}

/// An optional value, empty if absent.
fn optional<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<Option<T>> {
    if bytes.is_empty() {
        Ok(None)
    } else {
        Ok(Some(bincode::deserialize(bytes)?))
    }
}

/// `block`, checked that its certificate certifies its leaf, with the VID common data of the
/// genesis block, which consensus does not disperse.
fn finish<Types: NodeType>(mut block: Block<Types>) -> anyhow::Result<Block<Types>> {
    let height = block.leaf.height();
    ensure!(
        block.cert1.data.leaf_commit == block.leaf.commit(),
        "the certificate of height {height} does not certify its leaf"
    );
    if block.vid_common.is_none() && block.leaf.view_number() == ViewNumber::genesis() {
        match genesis_vid_common(&block.leaf) {
            Ok(common) => block.vid_common = Some(common),
            Err(err) => tracing::warn!("failed to compute the genesis VID common data: {err:#}"),
        }
    }
    Ok(block)
}

/// The VID common data of the genesis block, computed as consensus would have dispersed it.
fn genesis_vid_common<Types: NodeType>(leaf: &Leaf2<Types>) -> anyhow::Result<VidCommon> {
    let bytes = Types::BlockPayload::empty().0.encode();
    let header = leaf.block_header();
    let ns_table = || parse_ns_table(bytes.len(), &header.metadata().encode());
    let (commit, common) = match header.payload_commitment() {
        VidCommitment::V0(_) => {
            let disperse = advz_scheme(GENESIS_VID_NUM_STORAGE_NODES).disperse(&bytes)?;
            (
                VidCommitment::V0(disperse.commit),
                VidCommon::V0(disperse.common),
            )
        },
        VidCommitment::V1(_) => {
            let param = init_avidm_param(GENESIS_VID_NUM_STORAGE_NODES)?;
            let weights = vec![1; GENESIS_VID_NUM_STORAGE_NODES];
            let (commit, _) = AvidMScheme::ns_disperse(&param, &weights, &bytes, ns_table())
                .map_err(|err| anyhow::anyhow!("{err:?}"))?;
            (VidCommitment::V1(commit), VidCommon::V1(param))
        },
        VidCommitment::V2(_) => {
            let param = init_avidm_gf2_param(GENESIS_VID_NUM_STORAGE_NODES)?;
            let (commit, common) = AvidmGf2Scheme::commit(&param, &bytes, ns_table())
                .map_err(|err| anyhow::anyhow!("{err:?}"))?;
            (VidCommitment::V2(commit), VidCommon::V2(common))
        },
    };
    ensure!(
        commit == header.payload_commitment(),
        "the genesis VID commitment {commit} does not match the header's"
    );
    Ok(common)
}

#[cfg(test)]
mod tests;
