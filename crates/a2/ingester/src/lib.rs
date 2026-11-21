//! The ingester: copies the chain a consensus node decides into its deployment's store, with what
//! consensus derived from it, and repairs what the store lacks, once however many API instances
//! serve the store.
//!
//! It reads the decide log over the ingester API (`consensus-api`), writes each block to a
//! [`Sink`], usually a [`ChainStore`], with the state after it as consensus sends it, and the stake
//! tables and discovery records the stream carries, and acknowledges each block, which lets consensus free it: once it
//! has the block's payload, or once the block is final and no payload can come any more. Data from
//! the node's own consensus is not verified again; the ingester checks only that each leaf fits
//! its neighbours, so that one pointed at the wrong node or a store restored to the wrong place
//! does not mix two chains.
//!
//! Beside the decide log it runs the [`GapFiller`], which fills final gaps and checks the
//! certificates of filled heights from other API services, and the [`ChainFollower`], which
//! writes the state of the recent heights consensus sent none for. Membership, derived from L1,
//! serves both.

use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use anyhow::{Context, anyhow, ensure};
use async_trait::async_trait;
use bytes::Bytes;
use committable::{Commitment, Committable};
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

mod certs;
mod gaps;
mod membership;
mod peers;
mod state;
mod state_cert;
mod store;

pub use gaps::GapFiller;
pub use membership::{Membership, node_state};
pub use peers::Peers;
pub use state::ChainFollower;

/// How long the ingester waits before reconnecting after the connection fails.
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// How often the ingester acknowledges what it stored, in one call for all of it.
const ACK_INTERVAL: Duration = Duration::from_millis(50);

/// Where the ingester writes.
#[async_trait]
pub trait Sink<Types: NodeType>: Send + Sync {
    /// Makes `block` durable.
    async fn append(&self, block: Block<Types>) -> anyhow::Result<()>;

    /// Adds `payload` to the block stored at `height`, which was stored without it.
    async fn add_payload(&self, height: u64, payload: Vec<u8>) -> anyhow::Result<()>;

    /// The leaf of the newest block the sink holds, which entries after it must follow.
    async fn newest_leaf(&self) -> anyhow::Result<Option<Leaf2<Types>>>;

    /// Makes the encoded stake table of `epoch` durable.
    async fn put_stake_table(&self, epoch: u64, table: Bytes) -> anyhow::Result<()>;

    /// Makes the encoded discovery records of `epoch` durable.
    async fn put_service_records(&self, epoch: u64, records: Vec<Bytes>) -> anyhow::Result<()>;
}

/// A decided block, as the ingester hands it to its sink.
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

/// Ingests from the ingester API at `addr` into `sink`, reconnecting whenever the connection
/// fails, until an entry breaks the chain the sink holds or cannot be taken at all. Such an
/// entry would come again after every reconnect.
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

/// Ingests until the stream ends or fails. The stream starts at the earliest entry consensus
/// still holds, all of which are unacknowledged, so nothing stored but unacknowledged is missed.
async fn ingest<Types: NodeType>(addr: SocketAddr, sink: &dyn Sink<Types>) -> anyhow::Result<()> {
    let mut client = DecideLogClient::connect(format!("http://{addr}"))
        .await
        .context("connecting to the ingester API")?;
    let newest = sink.newest_leaf().await?;
    let mut stream = client
        .subscribe(SubscribeRequest { from_height: 0 })
        .await?
        .into_inner();
    let mut chain = Chain::new(newest.as_ref());
    let mut receiving: Option<Receiving<Types>> = None;
    let mut acks = interval(ACK_INTERVAL);
    loop {
        tokio::select! {
            response = stream.next() => {
                let Some(response) = response else {
                    return Ok(());
                };
                // An event this ingester does not know decodes to none, and would come again.
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
    chain: &mut Chain<Types>,
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
        Event::Start(start) => chain.start(start.earliest_height),
        Event::Decided(decided) => {
            let size = decided.payload_size;
            let entry = decode(decided).map_err(BadEntry)?;
            chain.check(&entry)?;
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
    chain: &mut Chain<Types>,
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
            chain.payload_added(height);
            Ok(())
        },
    }
}

/// Stores `entry`, with `payload` if consensus had it.
async fn store<Types: NodeType>(
    sink: &dyn Sink<Types>,
    chain: &mut Chain<Types>,
    entry: Entry<Types>,
    payload: Option<Vec<u8>>,
) -> anyhow::Result<()> {
    let (height, view) = (entry.leaf.height(), entry.view);
    let complete = payload.is_some();
    let block = finish(entry.into_block(payload)).map_err(BadEntry)?;
    sink.append(block).await?;
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

/// What the ingester knows of the chain: the leaves of recent heights, to check each entry
/// against its neighbours, and the stored heights not acknowledged yet.
///
/// Entries arrive mostly in height order, but a missing height can be decided later, and a
/// payload can follow its entry. A height is final once its view is
/// [`FINALITY_VIEWS`] behind the newest view seen; then nothing more comes for it.
struct Chain<Types: NodeType> {
    /// The leaves of recent heights.
    known: BTreeMap<u64, Known<Types>>,

    /// Stored heights not acknowledged yet, with their view and whether their payload is stored.
    unacked: BTreeMap<u64, (u64, bool)>,

    newest_view: u64,
}

impl<Types: NodeType> Chain<Types> {
    /// The chain as the sink holds it, whose newest leaf is `newest`.
    fn new(newest: Option<&Leaf2<Types>>) -> Self {
        let mut known = BTreeMap::new();
        let mut newest_view = 0;
        if let Some(leaf) = newest {
            let leaf = Known::of(leaf, *leaf.view_number());
            newest_view = leaf.view;
            known.insert(newest.map_or(0, Leaf2::height), leaf);
        }
        Self {
            known,
            unacked: BTreeMap::new(),
            newest_view,
        }
    }

    /// Notes where the decide log starts.
    fn start(&self, earliest: u64) {
        let Some((&newest, _)) = self.known.last_key_value() else {
            return;
        };
        if earliest > newest + 1 {
            tracing::warn!(
                missing = ?(newest + 1..earliest),
                "consensus no longer holds these heights; the API service fills them"
            );
        }
    }

    /// Checks that `entry` fits the leaves of the heights before and after it, if known, and
    /// any leaf known at its own height.
    fn check(&mut self, entry: &Entry<Types>) -> Result<(), ChainBroken> {
        let height = entry.leaf.height();
        let leaf = Known::of(&entry.leaf, entry.view);
        if let Some(known) = self.known.get(&height)
            && known.commitment != leaf.commitment
        {
            return Err(ChainBroken(format!(
                "leaf {} at height {height}, but the chain has {}",
                leaf.commitment, known.commitment
            )));
        }
        if let Some(before) = height.checked_sub(1)
            && let Some(known) = self.known.get(&before)
            && known.commitment != leaf.parent
        {
            return Err(ChainBroken(format!(
                "leaf at height {height} names parent {}, but the leaf before it is {}",
                leaf.parent, known.commitment
            )));
        }
        if let Some(child) = self.known.get(&(height + 1))
            && child.parent != leaf.commitment
        {
            return Err(ChainBroken(format!(
                "leaf {} at height {height} is not the parent of the leaf after it",
                leaf.commitment
            )));
        }
        self.known.insert(height, leaf);
        self.newest_view = self.newest_view.max(entry.view);
        // Keep the leaves an entry still to come can neighbour.
        let horizon = self.newest_view.saturating_sub(2 * FINALITY_VIEWS);
        let newest = self.known.keys().next_back().copied();
        self.known
            .retain(|height, leaf| leaf.view >= horizon || Some(*height) == newest);
        Ok(())
    }

    fn stored(&mut self, height: u64, view: u64, complete: bool) {
        self.unacked.insert(height, (view, complete));
    }

    fn payload_added(&mut self, height: u64) {
        if let Some((_, complete)) = self.unacked.get_mut(&height) {
            *complete = true;
        }
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
                tracing::warn!(height, "consensus does not have the payload");
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

/// What the neighbour check needs of a leaf.
struct Known<Types: NodeType> {
    commitment: Commitment<Leaf2<Types>>,
    parent: Commitment<Leaf2<Types>>,
    view: u64,
}

impl<Types: NodeType> Known<Types> {
    fn of(leaf: &Leaf2<Types>, view: u64) -> Self {
        Self {
            commitment: leaf.commit(),
            parent: leaf.parent_commitment(),
            view,
        }
    }
}

fn decode<Types: NodeType>(entry: Decided) -> anyhow::Result<Entry<Types>> {
    let body = entry.body.context("entry without a body")?;
    ensure!(
        body.format == BODY_FORMAT,
        "entry body has format {}, this ingester reads {BODY_FORMAT}",
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
    if block.payload.is_none() {
        tracing::warn!(height, "consensus does not have the payload");
    }
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
