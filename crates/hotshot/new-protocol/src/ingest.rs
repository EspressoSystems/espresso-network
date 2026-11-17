//! The decide log as the ingester API serves it.

use std::{net::SocketAddr, str::FromStr, sync::Arc};

use async_trait::async_trait;
use committable::Committable;
use consensus_api::{
    BODY_FORMAT, Error, PayloadKey, Source,
    proto::{Body, Decided as DecidedEntry, PayloadReady},
};
use consensus_store::{Bytes, Store};
use hotshot_types::{
    data::{EpochNumber, VidCommitment, ViewNumber},
    traits::node_implementation::NodeType,
};
use tokio::sync::{broadcast, watch};

use crate::store::{ConsensusStore, Decided};

/// Binds the ingester API over `store` to `addr`, and returns the address it is bound to,
/// which differs from `addr` for port 0, with the server, which runs until it is dropped or
/// fails.
pub fn bind<T: NodeType, S: Store>(
    store: ConsensusStore<T, S>,
    epoch_height: u64,
    addr: SocketAddr,
) -> std::io::Result<(
    SocketAddr,
    impl Future<Output = Result<(), tonic::transport::Error>> + Send,
)> {
    let incoming = tonic::transport::server::TcpIncoming::bind(addr)?;
    let bound = incoming.local_addr()?;
    let server = tonic::transport::Server::builder()
        .add_service(consensus_api::server(IngestSource::new(
            store,
            epoch_height,
        )))
        .serve_with_incoming(incoming);
    Ok((bound, server))
}

/// A [`Source`] over a node's consensus store.
pub struct IngestSource<T, S> {
    store: ConsensusStore<T, S>,
    epoch_height: u64,

    /// Payload announcements in the API's form, fed from the store's.
    ready: broadcast::Sender<PayloadReady>,
}

impl<T: NodeType, S: Store> IngestSource<T, S> {
    /// A source over `store`, whose chain has epochs of `epoch_height` blocks.
    pub fn new(store: ConsensusStore<T, S>, epoch_height: u64) -> Arc<Self> {
        let (ready, _) = broadcast::channel(64);
        let mut stored = store.stored_payloads();
        let forward = ready.clone();
        tokio::spawn(async move {
            loop {
                match stored.recv().await {
                    Ok(p) => {
                        let _ = forward.send(PayloadReady {
                            view: *p.view,
                            payload_commitment: VidCommitment::V2(p.commitment).to_string(),
                            size: p.size,
                        });
                    },
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        });
        Arc::new(Self {
            store,
            epoch_height,
            ready,
        })
    }

    /// Runs `f` on the store on a blocking thread.
    async fn blocking<R: Send + 'static>(
        &self,
        f: impl FnOnce(&ConsensusStore<T, S>) -> anyhow::Result<R> + Send + 'static,
    ) -> Result<R, Error> {
        let store = self.store.clone();
        Ok(tokio::task::spawn_blocking(move || f(&store)).await??)
    }
}

#[async_trait]
impl<T: NodeType, S: Store> Source for IngestSource<T, S> {
    async fn decided(&self, from: u64, limit: usize) -> Result<Vec<DecidedEntry>, Error> {
        let epoch_height = self.epoch_height;
        let entries = self.blocking(move |s| s.decided_from(from, limit)).await?;
        entries
            .into_iter()
            .map(|(decided, size)| entry(decided, size, epoch_height))
            .collect()
    }

    async fn earliest(&self) -> Result<Option<u64>, Error> {
        self.blocking(|s| s.earliest_decided()).await
    }

    fn next_height(&self) -> watch::Receiver<u64> {
        self.store.next_height()
    }

    async fn ack(&self, height: u64) -> Result<(), Error> {
        self.blocking(move |s| s.ack(height)).await
    }

    async fn payload(&self, key: PayloadKey) -> Result<Option<Bytes>, Error> {
        match key {
            PayloadKey::Height(height) => self.blocking(move |s| s.decided_payload(height)).await,
            PayloadKey::Block {
                view,
                payload_commitment,
            } => {
                let VidCommitment::V2(commitment) = VidCommitment::from_str(&payload_commitment)?
                else {
                    return Ok(None);
                };
                self.blocking(move |s| s.payload(ViewNumber::new(view), &commitment))
                    .await
            },
        }
    }

    fn payloads_ready(&self) -> broadcast::Receiver<PayloadReady> {
        self.ready.subscribe()
    }

    async fn service_records(
        &self,
        epoch: Option<u64>,
    ) -> Result<Option<(u64, Vec<Bytes>)>, Error> {
        let found = self
            .blocking(move |s| s.service_records(epoch.map(EpochNumber::new)))
            .await?;
        Ok(found.map(|(epoch, records)| (*epoch, records)))
    }
}

/// A decide log entry in the API's form.
fn entry<T: NodeType>(
    decided: Decided<T>,
    payload_size: Option<u64>,
    epoch_height: u64,
) -> Result<DecidedEntry, Error> {
    let leaf = &decided.leaf;
    let body = Body {
        format: BODY_FORMAT,
        leaf: bincode::serialize(leaf)?,
        cert1: decided
            .cert1
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default(),
        cert2: decided
            .cert2
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default(),
        vid_common: decided
            .vid_common
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default(),
        state_cert: decided
            .state_cert
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default(),
    };
    Ok(DecidedEntry {
        height: leaf.height(),
        view: *leaf.view_number(),
        epoch: leaf.epoch(epoch_height).map_or(0, |e| *e),
        leaf_commitment: leaf.commit().to_string(),
        parent_commitment: leaf.parent_commitment().to_string(),
        payload_commitment: leaf.payload_commitment().to_string(),
        payload_size,
        body: Some(body),
    })
}
