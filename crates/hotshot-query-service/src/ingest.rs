//! The ingester: moves the chain a consensus node decides into the query service.
//!
//! It reads the decide log over the ingester API (`consensus-api`), writes each block to a
//! [`Sink`], and acknowledges it, which lets consensus free it. Data from the node's own
//! consensus is not verified again; the ingester checks only the structure of the chain, so that
//! one pointed at the wrong node or a store restored to the wrong place does not mix two chains.

use std::{net::SocketAddr, time::Duration};

use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use committable::Committable;
use consensus_api::{
    BODY_FORMAT, DecideLogClient,
    proto::{
        AckRequest, Decided, GetPayloadRequest, SubscribeRequest, get_payload_request::Key,
        subscribe_response::Event,
    },
};
use futures::StreamExt;
use hotshot_types::{
    data::{Leaf2, VidCommon},
    simple_certificate::{Certificate1, Certificate2},
    traits::{
        block_contents::{BlockHeader, BlockPayload},
        node_implementation::NodeType,
    },
    vid::avidm_gf2::AvidmGf2Common,
};
use tonic::transport::Channel;

use crate::{
    Header, Payload,
    availability::{
        BlockInfo, BlockQueryData, LeafQueryData, QueryableHeader, QueryablePayload,
        VidCommonQueryData,
    },
};

/// How long the ingester waits before reconnecting after the connection fails.
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// Where the ingester writes.
#[async_trait]
pub trait Sink<Types: NodeType>: Send + Sync {
    /// Makes the block durable.
    async fn append(&self, info: BlockInfo<Types>) -> anyhow::Result<()>;

    /// The height of the first block the sink does not hold. Everything below it is durable.
    async fn block_height(&self) -> anyhow::Result<u64>;
}

/// Ingests from the ingester API at `addr` into `sink`, reconnecting whenever the connection
/// fails, until an entry breaks the chain the sink holds.
pub async fn run<Types>(addr: SocketAddr, sink: &dyn Sink<Types>) -> anyhow::Error
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
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
async fn ingest<Types>(addr: SocketAddr, sink: &dyn Sink<Types>) -> anyhow::Result<()>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    let mut client = DecideLogClient::connect(format!("http://{addr}"))
        .await
        .context("connecting to the ingester API")?;
    let from = sink.block_height().await?;
    // Everything below `from` is in the sink already.
    client.ack(AckRequest { height: from }).await?;
    let mut stream = client
        .subscribe(SubscribeRequest { from_height: from })
        .await?
        .into_inner();
    let mut chain = Chain::new(from);
    while let Some(response) = stream.next().await {
        match response?.event {
            Some(Event::Start(start)) => {
                tracing::warn!(
                    from = chain.next_height,
                    earliest = start.earliest_height,
                    "consensus no longer holds the heights before its earliest entry; they must \
                     come from elsewhere"
                );
                chain.skip_to(start.earliest_height);
            },
            Some(Event::Decided(entry)) => {
                for ready in chain.push(decode(entry)?)? {
                    let height = ready.leaf.height();
                    sink.append(block_info(&mut client, ready).await?).await?;
                    client.ack(AckRequest { height: height + 1 }).await?;
                }
            },
            None => bail!("subscribe response without an event"),
        }
    }
    Ok(())
}

/// A decide log entry with its consensus objects decoded.
struct Entry<Types: NodeType> {
    leaf: Leaf2<Types>,
    leaf_commitment: String,
    parent_commitment: String,
    payload_size: Option<u64>,
    cert1: Option<Certificate1<Types>>,
    cert2: Option<Certificate2<Types>>,
    vid_common: Option<AvidmGf2Common>,
}

/// An entry with the QC that certifies its leaf, ready to append.
struct Ready<Types: NodeType> {
    leaf: Leaf2<Types>,
    qc: Certificate1<Types>,
    payload_size: Option<u64>,
    cert2: Option<Certificate2<Types>>,
    vid_common: Option<AvidmGf2Common>,
}

/// The ingester's position in the chain.
///
/// An entry is ready once the QC that certifies its leaf is known: its own `cert1` if it is
/// the newest leaf of a decide, otherwise the next leaf's justify QC.
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

    /// Continues after a gap, from `height`.
    fn skip_to(&mut self, height: u64) {
        self.next_height = height;
        self.last_commitment = None;
        self.waiting = None;
    }

    /// Takes the next entry, and returns the entries that are now ready, in height order.
    fn push(&mut self, entry: Entry<Types>) -> Result<Vec<Ready<Types>>, ChainBroken> {
        let height = entry.leaf.height();
        if height != self.next_height {
            return Err(ChainBroken(format!(
                "expected height {}, got {height}",
                self.next_height
            )));
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
            ready.push(Ready::new(waiting, qc));
        }
        self.next_height = height + 1;
        self.last_commitment = Some(entry.leaf_commitment.clone());
        match entry.cert1.clone() {
            Some(qc) => ready.push(Ready::new(entry, qc)),
            None => self.waiting = Some(entry),
        }
        Ok(ready)
    }
}

impl<Types: NodeType> Ready<Types> {
    fn new(entry: Entry<Types>, qc: Certificate1<Types>) -> Self {
        Self {
            leaf: entry.leaf,
            qc,
            payload_size: entry.payload_size,
            cert2: entry.cert2,
            vid_common: entry.vid_common,
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
        leaf_commitment: entry.leaf_commitment,
        parent_commitment: entry.parent_commitment,
        payload_size: entry.payload_size,
        cert1: optional(&body.cert1).context("decoding the cert1")?,
        cert2: optional(&body.cert2).context("decoding the cert2")?,
        vid_common: optional(&body.vid_common).context("decoding the VID common data")?,
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

/// What the query service stores for `ready`, with its payload fetched if consensus has it.
async fn block_info<Types>(
    client: &mut DecideLogClient<Channel>,
    ready: Ready<Types>,
) -> anyhow::Result<BlockInfo<Types>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    let height = ready.leaf.height();
    let header = ready.leaf.block_header().clone();
    let block = match ready.payload_size {
        Some(_) => {
            let bytes = fetch_payload(client, height).await?;
            let payload = Payload::<Types>::from_bytes(&bytes, header.metadata());
            Some(BlockQueryData::new(header.clone(), payload))
        },
        None => {
            tracing::warn!(height, "consensus does not have the payload");
            None
        },
    };
    let vid_common = ready
        .vid_common
        .map(|common| VidCommonQueryData::new(header, VidCommon::V2(common)));
    let leaf = LeafQueryData::new(ready.leaf, ready.qc)
        .with_context(|| format!("the QC does not certify the leaf at height {height}"))?;
    let mut info = BlockInfo::new(leaf, block, vid_common, None);
    if let Some(cert2) = ready.cert2 {
        info = info.with_cert2(cert2);
    }
    Ok(info)
}

async fn fetch_payload(
    client: &mut DecideLogClient<Channel>,
    height: u64,
) -> anyhow::Result<Vec<u8>> {
    let mut chunks = client
        .get_payload(GetPayloadRequest {
            key: Some(Key::Height(height)),
        })
        .await?
        .into_inner();
    let mut bytes = Vec::new();
    while let Some(chunk) = chunks.next().await {
        bytes.extend(chunk?.data);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
