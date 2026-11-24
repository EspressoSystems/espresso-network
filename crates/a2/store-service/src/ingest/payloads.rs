//! The payloads consensus offers before their blocks are decided: this instance takes its share,
//! makes each durable in the store by its commitment, and acknowledges it, which lets consensus
//! vote on the block, then indexes its transactions.
//!
//! The payloads come over the consensus API, or, for an instance in the consensus node's own
//! process, straight from its payload queue, which shares each payload with consensus rather than
//! copying it.

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, anyhow, ensure};
use async_trait::async_trait;
use bytes::BytesMut;
use chain_store::ChainStore;
use consensus_api::{
    DecideLogClient,
    payloads::{Offer, PayloadQueue, Subscription},
    proto::{
        AckPayloadRequest, PayloadHead, PayloadOffer, TakePayloadsRequest, payload_offer::Event,
    },
};
use espresso_types::NsTable;
use futures::StreamExt;
use tonic::{Streaming, transport::Channel};

use super::RECONNECT_DELAY;

/// Takes payloads from the consensus API at `addr` into `store`, reconnecting whenever the
/// connection fails.
pub async fn run(addr: SocketAddr, store: &ChainStore) -> anyhow::Error {
    loop {
        let taken = async {
            let mut source = Remote::connect(addr).await?;
            take(&mut source, store).await
        };
        match taken.await {
            Ok(()) => tracing::warn!(%addr, "payload stream ended; reconnecting"),
            Err(err) => tracing::warn!(%addr, "taking payloads failed; reconnecting: {err:#}"),
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Takes payloads from `queue`, the payload queue of the consensus node in this process, into
/// `store`, subscribing again whenever storing one fails.
pub async fn run_local(queue: PayloadQueue, store: &ChainStore) -> anyhow::Error {
    loop {
        let mut source = Local {
            subscription: queue.subscribe(),
            queue: queue.clone(),
        };
        match take(&mut source, store).await {
            Ok(()) => return anyhow!("the payload queue ended"),
            Err(err) => tracing::warn!("taking payloads failed; subscribing again: {err:#}"),
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Takes payloads until `source` ends or fails. A payload taken and not acknowledged goes to
/// another subscription, or to this one after it subscribes again.
///
/// Each payload is indexed after it is acknowledged, beside taking the next ones, since consensus
/// waits for the acknowledgement and not for the index.
async fn take(source: &mut dyn Source, store: &ChainStore) -> anyhow::Result<()> {
    while let Some(offer) = source.next().await? {
        let ns_table = NsTable::from_bytes_unchecked(&offer.ns_table);
        store
            .put_payload(&offer.commitment, offer.payload.clone(), Some(&ns_table))
            .await?;
        source.ack(&offer).await?;
        let store = store.clone();
        tokio::spawn(async move {
            let indexed = store
                .index_payload(&offer.commitment, offer.payload.clone(), ns_table)
                .await;
            if let Err(err) = indexed {
                tracing::warn!(commitment = %offer.commitment, "indexing a payload failed: {err:#}");
            }
        });
    }
    Ok(())
}

/// Where this instance takes its share of the payloads from.
#[async_trait]
trait Source: Send {
    /// The next payload for this instance, or `None` once the offers end.
    async fn next(&mut self) -> anyhow::Result<Option<Arc<Offer>>>;

    /// Reports `offer` durable.
    async fn ack(&mut self, offer: &Offer) -> anyhow::Result<()>;
}

/// The payloads over the consensus API, each sent as a head and then its chunks.
struct Remote {
    client: DecideLogClient<Channel>,
    offers: Streaming<PayloadOffer>,
}

impl Remote {
    async fn connect(addr: SocketAddr) -> anyhow::Result<Self> {
        let mut client = DecideLogClient::connect(format!("http://{addr}"))
            .await
            .context("connecting to the consensus API")?;
        let offers = client
            .take_payloads(TakePayloadsRequest {})
            .await?
            .into_inner();
        Ok(Self { client, offers })
    }
}

#[async_trait]
impl Source for Remote {
    async fn next(&mut self) -> anyhow::Result<Option<Arc<Offer>>> {
        let mut receiving: Option<(PayloadHead, BytesMut)> = None;
        while let Some(offer) = self.offers.next().await {
            let event = offer?
                .event
                .ok_or_else(|| anyhow!("a payload offer without an event"))?;
            match (event, receiving.as_mut()) {
                (Event::Head(head), None) => {
                    let size = usize::try_from(head.size)?;
                    receiving = Some((head, BytesMut::with_capacity(size)));
                },
                (Event::Chunk(chunk), Some((head, payload))) => {
                    payload.extend_from_slice(&chunk.data);
                    ensure!(
                        payload.len() as u64 <= head.size,
                        "a payload longer than announced"
                    );
                },
                (Event::Head(_), Some(_)) => anyhow::bail!("a payload ended early"),
                (Event::Chunk(_), None) => anyhow::bail!("a payload chunk without a head"),
            }
            if let Some((head, payload)) = &receiving
                && payload.len() as u64 == head.size
            {
                let (head, payload) = receiving.take().expect("a payload is being received");
                return Ok(Some(Arc::new(Offer {
                    view: head.view,
                    commitment: head.commitment,
                    ns_table: head.ns_table,
                    payload: payload.freeze(),
                })));
            }
        }
        ensure!(
            receiving.is_none(),
            "the payload stream ended within a payload"
        );
        Ok(None)
    }

    async fn ack(&mut self, offer: &Offer) -> anyhow::Result<()> {
        self.client
            .ack_payload(AckPayloadRequest {
                view: offer.view,
                commitment: offer.commitment.clone(),
            })
            .await?;
        Ok(())
    }
}

/// The payload queue of the consensus node in this process.
struct Local {
    subscription: Subscription,
    queue: PayloadQueue,
}

#[async_trait]
impl Source for Local {
    async fn next(&mut self) -> anyhow::Result<Option<Arc<Offer>>> {
        Ok(self.subscription.next().await)
    }

    async fn ack(&mut self, offer: &Offer) -> anyhow::Result<()> {
        self.queue.ack((offer.view, offer.commitment.clone()));
        Ok(())
    }
}
