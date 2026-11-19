//! The ingester: copies the chain a consensus node decides into the API service's store.
//!
//! It reads the decide log over the ingester API (`consensus-api`), writes each block to a
//! [`Sink`], usually a [`ChainStore`], and acknowledges it, which lets consensus free it. Data from
//! the node's own consensus is not verified again; the ingester checks only the structure of the
//! chain, so that one pointed at the wrong node or a store restored to the wrong place does not mix
//! two chains.

use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use committable::Committable;
use consensus_api::{
    BODY_FORMAT, DecideLogClient,
    proto::{AckRequest, Decided, SubscribeRequest, subscribe_response::Event},
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

mod store;

/// How long the ingester waits before reconnecting after the connection fails.
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// Where the ingester writes.
#[async_trait]
pub trait Sink<Types: NodeType>: Send + Sync {
    /// Makes `block` durable.
    async fn append(&self, block: Block<Types>) -> anyhow::Result<()>;

    /// The height after the newest block the sink holds. The ingester resumes from it; heights
    /// below it can be missing, and are the API service's to fill.
    async fn block_height(&self) -> anyhow::Result<u64>;
}

/// A decided block, as the ingester hands it to its sink.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block<Types: NodeType> {
    pub leaf: Leaf2<Types>,

    /// The certificate that certifies the leaf: its own if it was the newest leaf of a decide,
    /// otherwise the justify QC of the leaf after it.
    pub cert1: Certificate1<Types>,

    /// The certificate that decided the leaf directly, if one did.
    pub cert2: Option<Certificate2<Types>>,

    pub vid_common: Option<VidCommon>,

    /// The light client state certificate of the epoch the block is the root of.
    pub state_cert: Option<LightClientStateUpdateCertificateV2<Types>>,

    /// The encoded payload; absent if consensus did not have it.
    pub payload: Option<Vec<u8>>,
}

/// Ingests from the ingester API at `addr` into `sink`, reconnecting whenever the connection
/// fails, until an entry breaks the chain the sink holds.
pub async fn run<Types: NodeType>(addr: SocketAddr, sink: &dyn Sink<Types>) -> anyhow::Error {
    loop {
        match ingest(addr, sink).await {
            Ok(()) => tracing::warn!(%addr, "decide log stream ended; reconnecting"),
            Err(err) if err.is::<ChainBroken>() => {
                tracing::error!(%addr, "stopped ingesting: {err:#}");
                return err;
            },
            Err(err) => tracing::warn!(%addr, "ingesting failed; reconnecting: {err:#}"),
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// An entry that does not continue the chain the sink holds.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ChainBroken(String);

/// Ingests until the stream ends or fails.
async fn ingest<Types: NodeType>(addr: SocketAddr, sink: &dyn Sink<Types>) -> anyhow::Result<()> {
    let mut client = DecideLogClient::connect(format!("http://{addr}"))
        .await
        .context("connecting to the ingester API")?;
    let from = sink.block_height().await?;
    // The sink has stored up to `from`; any gap below it is the API service's to fill, not
    // consensus's to keep.
    client.ack(AckRequest { height: from }).await?;
    let mut stream = client
        .subscribe(SubscribeRequest { from_height: from })
        .await?
        .into_inner();
    let mut chain = Chain::new(from);
    // The entry whose payload is still arriving, with the bytes so far.
    let mut receiving: Option<(Entry<Types>, Vec<u8>)> = None;
    while let Some(response) = stream.next().await {
        let event = response?.event;
        if let Some((entry, mut payload)) = receiving.take() {
            let Some(Event::PayloadChunk(chunk)) = event else {
                bail!("the payload of height {} ended early", entry.leaf.height());
            };
            payload.extend(chunk.data);
            receiving = Some((entry, payload));
            if let Some(entry) = complete(&mut receiving)? {
                append(&mut client, sink, &mut chain, entry).await?;
            }
            continue;
        }
        match event {
            Some(Event::Start(start)) => {
                tracing::warn!(
                    from = chain.next_height,
                    earliest = start.earliest_height,
                    "consensus no longer holds the heights before its earliest entry; they must \
                     come from elsewhere"
                );
                chain.skip_to(start.earliest_height);
            },
            Some(Event::Decided(decided)) => {
                let size = decided.payload_size;
                let entry = decode(decided)?;
                match size {
                    Some(size) => {
                        receiving = Some((entry, Vec::with_capacity(size as usize)));
                        if let Some(entry) = complete(&mut receiving)? {
                            append(&mut client, sink, &mut chain, entry).await?;
                        }
                    },
                    None => append(&mut client, sink, &mut chain, entry).await?,
                }
            },
            Some(Event::PayloadChunk(_)) => bail!("a payload chunk without an entry"),
            None => bail!("subscribe response without an event"),
        }
    }
    Ok(())
}

/// The entry of `receiving` once its whole payload has arrived, which leaves `receiving` empty.
fn complete<Types: NodeType>(
    receiving: &mut Option<(Entry<Types>, Vec<u8>)>,
) -> anyhow::Result<Option<Entry<Types>>> {
    let Some((entry, payload)) = receiving.as_ref() else {
        return Ok(None);
    };
    let expected = entry.payload_size.unwrap_or_default() as usize;
    ensure!(
        payload.len() <= expected,
        "the payload of height {} is longer than announced",
        entry.leaf.height()
    );
    if payload.len() < expected {
        return Ok(None);
    }
    let (mut entry, payload) = receiving.take().expect("checked above");
    entry.payload = Some(payload);
    Ok(Some(entry))
}

/// Takes `entry` into the chain, and stores and acknowledges every block that is ready.
async fn append<Types: NodeType>(
    client: &mut DecideLogClient<tonic::transport::Channel>,
    sink: &dyn Sink<Types>,
    chain: &mut Chain<Types>,
    entry: Entry<Types>,
) -> anyhow::Result<()> {
    for block in chain.push(entry)? {
        let height = block.leaf.height();
        sink.append(finish(block)?).await?;
        client.ack(AckRequest { height: height + 1 }).await?;
    }
    Ok(())
}

/// A decide log entry with its consensus objects decoded.
struct Entry<Types: NodeType> {
    leaf: Leaf2<Types>,
    leaf_commitment: String,
    parent_commitment: String,
    payload_size: Option<u64>,
    payload: Option<Vec<u8>>,
    cert1: Option<Certificate1<Types>>,
    cert2: Option<Certificate2<Types>>,
    vid_common: Option<AvidmGf2Common>,
    state_cert: Option<LightClientStateUpdateCertificateV2<Types>>,
}

/// The ingester's position in the chain.
///
/// An entry is ready once the QC that certifies its leaf is known: its own `cert1` if it is
/// the newest leaf of a decide, otherwise the next leaf's justify QC.
///
/// Heights can be missing from the decide log, where consensus decided a block without an
/// ancestor it never received. The ingester stores what follows and leaves the missing heights to
/// the API service, which fills them from other API services; waiting for them here would make
/// consensus wait on other operators.
struct Chain<Types: NodeType> {
    next_height: u64,

    /// The commitment of the leaf before `next_height`, once the ingester has seen it.
    last_commitment: Option<String>,

    /// An entry waiting for the next leaf to certify it.
    waiting: Option<Entry<Types>>,
}

impl<Types: NodeType> Chain<Types> {
    fn new(next_height: u64) -> Self {
        Self {
            next_height,
            last_commitment: None,
            waiting: None,
        }
    }

    /// Continues at `height`, past heights the decide log lacks. An entry waiting for its QC
    /// cannot get it across the hole, so it becomes part of the hole.
    fn skip_hole(&mut self, height: u64) {
        let missing = match &self.waiting {
            Some(waiting) => waiting.leaf.height()..height,
            None => self.next_height..height,
        };
        tracing::warn!(
            ?missing,
            "the decide log lacks these heights; the API service fills them"
        );
        self.skip_to(height);
    }

    /// Continues after a gap, from `height`.
    fn skip_to(&mut self, height: u64) {
        self.next_height = height;
        self.last_commitment = None;
        self.waiting = None;
    }

    /// Takes the next entry, and returns the entries that are now ready, in height order.
    fn push(&mut self, entry: Entry<Types>) -> Result<Vec<Block<Types>>, ChainBroken> {
        let height = entry.leaf.height();
        if height < self.next_height {
            return Err(ChainBroken(format!(
                "expected height {}, got {height}",
                self.next_height
            )));
        }
        if height > self.next_height {
            self.skip_hole(height);
        }
        if let Some(last) = &self.last_commitment
            && *last != entry.parent_commitment
        {
            return Err(ChainBroken(format!(
                "leaf at height {height} names parent {}, but the leaf before it is {last}",
                entry.parent_commitment
            )));
        }
        let mut ready = Vec::new();
        if let Some(waiting) = self.waiting.take() {
            let qc = entry.leaf.justify_qc().clone();
            ready.push(certified(waiting, qc));
        }
        self.next_height = height + 1;
        self.last_commitment = Some(entry.leaf_commitment.clone());
        match entry.cert1.clone() {
            Some(qc) => ready.push(certified(entry, qc)),
            None => self.waiting = Some(entry),
        }
        Ok(ready)
    }
}

/// `entry` with `qc`, the certificate that certifies its leaf.
fn certified<Types: NodeType>(entry: Entry<Types>, qc: Certificate1<Types>) -> Block<Types> {
    Block {
        leaf: entry.leaf,
        cert1: qc,
        cert2: entry.cert2,
        vid_common: entry.vid_common.map(VidCommon::V2),
        state_cert: entry.state_cert,
        payload: entry.payload,
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
        leaf_commitment: entry.leaf_commitment,
        parent_commitment: entry.parent_commitment,
        payload_size: entry.payload_size,
        payload: None,
        cert1: optional(&body.cert1).context("decoding the cert1")?,
        cert2: optional(&body.cert2).context("decoding the cert2")?,
        vid_common: optional(&body.vid_common).context("decoding the VID common data")?,
        state_cert: optional(&body.state_cert).context("decoding the state certificate")?,
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
