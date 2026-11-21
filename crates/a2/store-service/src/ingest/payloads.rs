//! The payloads consensus offers before their blocks are decided: this instance takes its share,
//! makes each durable in the store by its commitment, and acknowledges it, which lets consensus
//! vote on the block.

use std::net::SocketAddr;

use anyhow::{Context, anyhow, ensure};
use bytes::BytesMut;
use chain_store::ChainStore;
use consensus_api::{
    DecideLogClient,
    proto::{AckPayloadRequest, PayloadHead, TakePayloadsRequest, payload_offer::Event},
};
use espresso_types::NsTable;
use futures::StreamExt;

use super::RECONNECT_DELAY;

/// Takes payloads from the consensus API at `addr` into `store`, reconnecting whenever the
/// connection fails.
pub async fn run(addr: SocketAddr, store: &ChainStore) -> anyhow::Error {
    loop {
        match take(addr, store).await {
            Ok(()) => tracing::warn!(%addr, "payload stream ended; reconnecting"),
            Err(err) => tracing::warn!(%addr, "taking payloads failed; reconnecting: {err:#}"),
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

/// Takes payloads until the stream ends or fails. A payload taken and not acknowledged goes to
/// another subscription, or to this one after it reconnects.
async fn take(addr: SocketAddr, store: &ChainStore) -> anyhow::Result<()> {
    let mut client = DecideLogClient::connect(format!("http://{addr}"))
        .await
        .context("connecting to the consensus API")?;
    let mut offers = client
        .take_payloads(TakePayloadsRequest {})
        .await?
        .into_inner();
    let mut receiving: Option<(PayloadHead, BytesMut)> = None;
    while let Some(offer) = offers.next().await {
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
            let ns_table = NsTable::from_bytes_unchecked(&head.ns_table);
            store
                .put_payload(&head.commitment, payload.freeze(), Some(&ns_table))
                .await?;
            client
                .ack_payload(AckPayloadRequest {
                    view: head.view,
                    commitment: head.commitment,
                })
                .await?;
        }
    }
    Ok(())
}
