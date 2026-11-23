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
//! verified otherwise. Entries carry no payloads: those come through the payload offers
//! ([`payloads`]), to whichever instance takes them, so an entry whose payload the store does not
//! have yet is acknowledged once it is final.

use std::{collections::BTreeMap, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context, anyhow, ensure};
use async_trait::async_trait;
use bytes::Bytes;
use committable::Committable;
use consensus_api::{
    BODY_FORMAT, DecideLogClient, FINALITY_VIEWS,
    proto::{
        AckRequest, Decided, HeightRange, State, SubscribeRequest, SubscribeResponse,
        decide_log_server::DecideLog, subscribe_response::Event,
    },
};
use futures::{StreamExt, stream::BoxStream};
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
use tonic::{Request, Status, transport::Channel};

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

    /// The state after the block as consensus sends it; absent if consensus did not validate the
    /// block.
    pub state: Option<State>,
}

/// Ingests from the decide log at `addr` into `sink`, reconnecting whenever the connection fails,
/// until an entry breaks the chain the sink holds or cannot be taken at all. Such an entry would
/// come again after every reconnect.
pub async fn run<Types: NodeType>(addr: SocketAddr, sink: &dyn Sink<Types>) -> anyhow::Error {
    loop {
        let ingested = async {
            let mut client = DecideLogClient::connect(format!("http://{addr}"))
                .await
                .context("connecting to the decide log")?;
            ingest(&mut client, sink).await
        };
        if let Some(err) = stopped(ingested.await) {
            return err;
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Ingests from `log`, the decide log of the consensus node in this process, into `sink`, as
/// [`run`] does over the consensus API.
pub async fn run_local<Types: NodeType, D: DecideLog>(
    log: Arc<D>,
    sink: &dyn Sink<Types>,
) -> anyhow::Error {
    loop {
        if let Some(err) = stopped(ingest(&mut Local(log.clone()), sink).await) {
            return err;
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// The error to stop ingesting with after `ingested`, if it is one that would come again after
/// every reconnect; any other end is logged, to be followed by a reconnect.
fn stopped(ingested: anyhow::Result<()>) -> Option<anyhow::Error> {
    match ingested {
        Ok(()) => tracing::warn!("decide log stream ended; reconnecting"),
        Err(err) if err.is::<ChainBroken>() || err.is::<BadEntry>() => {
            tracing::error!("stopped ingesting: {err:#}");
            return Some(err);
        },
        Err(err) => tracing::warn!("ingesting failed; reconnecting: {err:#}"),
    }
    None
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
async fn ingest<Types: NodeType>(log: &mut dyn Log, sink: &dyn Sink<Types>) -> anyhow::Result<()> {
    let mut stream = log.subscribe().await?;
    let mut chain = Chain::default();
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
                receive(sink, &mut chain, event).await?;
            },
            _ = acks.tick() => {
                let ranges = chain.take_ackable();
                if !ranges.is_empty() {
                    log.ack(ranges).await?;
                }
            },
        }
    }
}

/// Where this instance takes its share of the decide log from.
#[async_trait]
trait Log: Send {
    /// The stream of this instance's share of the entries, with the stake tables and records.
    async fn subscribe(
        &mut self,
    ) -> anyhow::Result<BoxStream<'static, Result<SubscribeResponse, Status>>>;

    /// Frees the entries of `ranges`, which the sink holds.
    async fn ack(&mut self, ranges: Vec<HeightRange>) -> anyhow::Result<()>;
}

#[async_trait]
impl Log for DecideLogClient<Channel> {
    async fn subscribe(
        &mut self,
    ) -> anyhow::Result<BoxStream<'static, Result<SubscribeResponse, Status>>> {
        let stream = DecideLogClient::subscribe(self, SubscribeRequest {}).await?;
        Ok(stream.into_inner().boxed())
    }

    async fn ack(&mut self, ranges: Vec<HeightRange>) -> anyhow::Result<()> {
        DecideLogClient::ack(self, AckRequest { ranges }).await?;
        Ok(())
    }
}

/// The decide log of the consensus node in this process, called directly.
struct Local<D>(Arc<D>);

#[async_trait]
impl<D: DecideLog> Log for Local<D> {
    async fn subscribe(
        &mut self,
    ) -> anyhow::Result<BoxStream<'static, Result<SubscribeResponse, Status>>> {
        let stream = self.0.subscribe(Request::new(SubscribeRequest {})).await?;
        Ok(stream.into_inner().boxed())
    }

    async fn ack(&mut self, ranges: Vec<HeightRange>) -> anyhow::Result<()> {
        self.0.ack(Request::new(AckRequest { ranges })).await?;
        Ok(())
    }
}

/// Takes `event` into the chain and the sink.
async fn receive<Types: NodeType>(
    sink: &dyn Sink<Types>,
    chain: &mut Chain,
    event: Event,
) -> anyhow::Result<()> {
    match event {
        Event::Start(start) => {
            if let Some(earliest) = start.earliest_height {
                note_missing(sink, earliest).await?;
            }
        },
        Event::Progress(progress) => chain.progress(progress.newest_view),
        Event::Decided(decided) => {
            let entry = decode(decided).map_err(BadEntry)?;
            check(sink, &entry).await?;
            store(sink, chain, entry).await?;
        },
        Event::StakeTable(table) => sink.put_stake_table(table.epoch, table.table).await?,
        Event::ServiceRecords(records) => {
            sink.put_service_records(records.epoch, records.records)
                .await?
        },
    }
    Ok(())
}

/// Stores `entry`. Its payload may have reached the store already, through any instance.
async fn store<Types: NodeType>(
    sink: &dyn Sink<Types>,
    chain: &mut Chain,
    entry: Entry<Types>,
) -> anyhow::Result<()> {
    let (height, view) = (entry.leaf.height(), entry.view);
    chain.progress(view);
    let block = finish(entry.into_block()).map_err(BadEntry)?;
    sink.append(block).await?;
    let complete = sink.has_payload(height).await?;
    chain.stored(height, view, complete);
    Ok(())
}

/// A decide log entry with its consensus objects decoded.
struct Entry<Types: NodeType> {
    leaf: Leaf2<Types>,
    view: u64,
    cert1: Certificate1<Types>,
    cert2: Option<Certificate2<Types>>,
    vid_common: Option<AvidmGf2Common>,
    state_cert: Option<LightClientStateUpdateCertificateV2<Types>>,
    state: Option<State>,
}

impl<Types: NodeType> Entry<Types> {
    fn into_block(self) -> Block<Types> {
        Block {
            leaf: self.leaf,
            cert1: self.cert1,
            cert2: self.cert2,
            vid_common: self.vid_common.map(VidCommon::V2),
            state_cert: self.state_cert,
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
