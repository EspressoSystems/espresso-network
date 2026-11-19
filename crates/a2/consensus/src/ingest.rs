//! The decide log as the ingester API serves it.

use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use committable::Committable;
use consensus_api::{
    BODY_FORMAT, Error, Node, Source, SubmitError,
    proto::{Body, Decided as DecidedEntry},
};
use consensus_store::{Bytes, Store};
use hotshot_types::{data::EpochNumber, traits::node_implementation::NodeType};
use tokio::sync::watch;

use crate::{
    block::SubmitError as BlockSubmitError,
    client::{ClientApi, QueryError},
    store::{ConsensusStore, Decided},
};

/// Binds the consensus node's API to `addr`: the decide log over `store` for the ingester, and
/// the node service over the coordinator behind `client` for the API service. Returns the address
/// it is bound to, which differs from `addr` for port 0, with the server, which runs until it is
/// dropped or fails.
pub fn bind<T: NodeType, S: Store>(
    store: ConsensusStore<T, S>,
    epoch_height: u64,
    client: ClientApi<T>,
    addr: SocketAddr,
) -> std::io::Result<(
    SocketAddr,
    impl Future<Output = Result<(), tonic::transport::Error>> + Send,
)> {
    // Without TCP_NODELAY a small answer waits for the client's delayed acknowledgement.
    let incoming = tonic::transport::server::TcpIncoming::bind(addr)?.with_nodelay(Some(true));
    let bound = incoming.local_addr()?;
    let server = tonic::transport::Server::builder()
        .add_service(consensus_api::server(IngestSource::new(
            store,
            epoch_height,
        )))
        .add_service(consensus_api::node_server(Arc::new(NodeService { client })))
        .serve_with_incoming(incoming);
    Ok((bound, server))
}

/// A [`Source`] over a node's consensus store.
pub struct IngestSource<T, S> {
    store: ConsensusStore<T, S>,
    epoch_height: u64,
}

impl<T: NodeType, S: Store> IngestSource<T, S> {
    /// A source over `store`, whose chain has epochs of `epoch_height` blocks.
    pub fn new(store: ConsensusStore<T, S>, epoch_height: u64) -> Arc<Self> {
        Arc::new(Self {
            store,
            epoch_height,
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

    async fn payload(&self, height: u64) -> Result<Option<Bytes>, Error> {
        self.blocking(move |s| s.decided_payload(height)).await
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

/// The node service: what only consensus can do for the API service.
struct NodeService<T: NodeType> {
    client: ClientApi<T>,
}

#[async_trait]
impl<T: NodeType> Node for NodeService<T> {
    async fn submit_transaction(&self, transaction: Bytes) -> Result<(), SubmitError> {
        let transaction = bincode::deserialize(&transaction)
            .map_err(|err| SubmitError::Invalid(format!("undecodable transaction: {err}")))?;
        match self.client.submit_transaction(transaction).await {
            Ok(()) => Ok(()),
            Err(QueryError::Rejected(rejection @ BlockSubmitError::TooLarge { .. })) => {
                Err(SubmitError::Invalid(rejection.to_string()))
            },
            Err(QueryError::Rejected(rejection @ BlockSubmitError::RetryBufferFull)) => {
                Err(SubmitError::Overloaded(rejection.to_string()))
            },
            Err(err) => Err(SubmitError::Internal(err.into())),
        }
    }
}
