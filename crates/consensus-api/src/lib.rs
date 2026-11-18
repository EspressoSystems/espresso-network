//! The interface between a consensus node and its API service.
//!
//! A consensus node serves two gRPC services. The
//! [`DecideLog`](proto::decide_log_server::DecideLog), served from a [`Source`], its store, with
//! [`server`], gives the API service's ingester the decided chain; the ingester connects with
//! [`DecideLogClient`]. The [`ConsensusNode`](proto::consensus_node_server::ConsensusNode), served
//! from a [`Node`] with [`node_server`], takes what only consensus can act on, such as
//! transactions; the API service connects with [`ConsensusNodeClient`]. See the proto files in
//! `proto/espresso/consensus/v1/` for the services.

use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, stream};
use tokio::sync::{mpsc, watch};
use tonic::{Request, Response, Status};

/// Code generated from the proto definition.
#[allow(clippy::large_enum_variant)]
pub mod proto {
    tonic::include_proto!("espresso.consensus.v1");
}

use proto::{
    AckRequest, AckResponse, Decided, PayloadChunk, ServiceRecordsRequest, ServiceRecordsResponse,
    Start, SubmitTransactionRequest, SubmitTransactionResponse, SubscribeRequest,
    SubscribeResponse,
    consensus_node_server::{ConsensusNode, ConsensusNodeServer},
    decide_log_server::{DecideLog, DecideLogServer},
    subscribe_response::Event,
};
pub use proto::{consensus_node_client::ConsensusNodeClient, decide_log_client::DecideLogClient};

/// The version of [`proto::Body`] this crate produces.
pub const BODY_FORMAT: u32 = 1;

/// The largest payload chunk [`server`] sends.
const CHUNK_SIZE: usize = 1 << 20;

/// How many entries the server reads from its source at a time.
const BATCH: usize = 64;

/// What the service serves: the consensus node's decide log and payloads.
#[async_trait]
pub trait Source: Send + Sync + 'static {
    /// The entries from height `from` on, at most `limit`, in height order.
    async fn decided(&self, from: u64, limit: usize) -> Result<Vec<Decided>, Error>;

    /// The earliest height whose entry is still kept, if any.
    async fn earliest(&self) -> Result<Option<u64>, Error>;

    /// The height after the newest decided entry. It only rises.
    fn next_height(&self) -> watch::Receiver<u64>;

    /// Frees the entries below `height`, and their payloads.
    async fn ack(&self, height: u64) -> Result<(), Error>;

    /// The payload of the entry at `height`, if this node has it and still keeps the entry.
    async fn payload(&self, height: u64) -> Result<Option<Bytes>, Error>;

    /// The encoded service records of `epoch`, or of the latest epoch with records if `epoch` is
    /// `None`, with the epoch they are for.
    async fn service_records(&self, epoch: Option<u64>)
    -> Result<Option<(u64, Vec<Bytes>)>, Error>;
}

/// What the [`ConsensusNode`](proto::consensus_node_server::ConsensusNode) service does: hand
/// transactions to consensus.
#[async_trait]
pub trait Node: Send + Sync + 'static {
    /// Hands an encoded transaction to consensus.
    async fn submit_transaction(&self, transaction: Bytes) -> Result<(), SubmitError>;
}

/// Why [`Source::submit_transaction`] did not take a transaction.
#[derive(Debug, thiserror::Error)]
pub enum SubmitError {
    /// No block can take it.
    #[error("invalid transaction: {0}")]
    Invalid(String),

    /// Consensus has no room for it now; trying again later can succeed.
    #[error("consensus is overloaded: {0}")]
    Overloaded(String),

    #[error(transparent)]
    Internal(Error),
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

/// The gRPC service over `source`.
pub fn server<S: Source>(source: Arc<S>) -> DecideLogServer<Service<S>> {
    DecideLogServer::new(Service { source })
}

pub struct Service<S> {
    source: Arc<S>,
}

type BoxStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[async_trait]
impl<S: Source> DecideLog for Service<S> {
    type SubscribeStream = BoxStream<SubscribeResponse>;

    async fn subscribe(
        &self,
        request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let from = request.into_inner().from_height;
        let (tx, rx) = mpsc::channel(BATCH);
        tokio::spawn(feed(self.source.clone(), from, tx));
        Ok(Response::new(receiver_stream(rx)))
    }

    async fn ack(&self, request: Request<AckRequest>) -> Result<Response<AckResponse>, Status> {
        self.source
            .ack(request.into_inner().height)
            .await
            .map_err(internal)?;
        Ok(Response::new(AckResponse {}))
    }

    async fn get_service_records(
        &self,
        request: Request<ServiceRecordsRequest>,
    ) -> Result<Response<ServiceRecordsResponse>, Status> {
        let (epoch, records) = self
            .source
            .service_records(request.into_inner().epoch)
            .await
            .map_err(internal)?
            .ok_or_else(|| Status::not_found("no service records"))?;
        Ok(Response::new(ServiceRecordsResponse {
            epoch,
            records: records.into_iter().map(|r| r.to_vec()).collect(),
        }))
    }
}

/// The [`ConsensusNode`](proto::consensus_node_server::ConsensusNode) gRPC service over `node`.
pub fn node_server<N: Node>(node: Arc<N>) -> ConsensusNodeServer<NodeService<N>> {
    ConsensusNodeServer::new(NodeService { node })
}

pub struct NodeService<N> {
    node: Arc<N>,
}

#[async_trait]
impl<N: Node> ConsensusNode for NodeService<N> {
    async fn submit_transaction(
        &self,
        request: Request<SubmitTransactionRequest>,
    ) -> Result<Response<SubmitTransactionResponse>, Status> {
        let transaction = Bytes::from(request.into_inner().transaction);
        match self.node.submit_transaction(transaction).await {
            Ok(()) => Ok(Response::new(SubmitTransactionResponse {})),
            Err(SubmitError::Invalid(reason)) => Err(Status::invalid_argument(reason)),
            Err(SubmitError::Overloaded(reason)) => Err(Status::resource_exhausted(reason)),
            Err(SubmitError::Internal(err)) => Err(internal(err)),
        }
    }
}

/// Sends the entries from `from` on to `tx`, then each new one as it is decided, until the
/// subscriber goes away.
async fn feed<S: Source>(
    source: Arc<S>,
    from: u64,
    tx: mpsc::Sender<Result<SubscribeResponse, Status>>,
) {
    let mut next_height = source.next_height();
    let mut cursor = from;
    match source.earliest().await {
        Ok(Some(earliest)) if earliest > from => {
            let start = Event::Start(Start {
                earliest_height: earliest,
            });
            if !send(&tx, start).await {
                return;
            }
            cursor = earliest;
        },
        Ok(_) => {},
        Err(err) => {
            let _ = tx.send(Err(internal(err))).await;
            return;
        },
    }
    loop {
        let entries = match source.decided(cursor, BATCH).await {
            Ok(entries) => entries,
            Err(err) => {
                let _ = tx.send(Err(internal(err))).await;
                return;
            },
        };
        let read_all = entries.len() < BATCH;
        for entry in entries {
            cursor = entry.height + 1;
            if !send_entry(&*source, &tx, entry).await {
                return;
            }
        }
        if read_all {
            tokio::select! {
                changed = next_height.wait_for(|next| *next > cursor) => {
                    if changed.is_err() {
                        return;
                    }
                },
                () = tx.closed() => return,
            }
        }
    }
}

/// Sends `entry` and then its payload in chunks, reading the payload only now so that at most one
/// is held at a time. A payload freed since the entry was read leaves the entry without one.
/// Returns whether the subscriber is still there.
async fn send_entry<S: Source>(
    source: &S,
    tx: &mpsc::Sender<Result<SubscribeResponse, Status>>,
    mut entry: Decided,
) -> bool {
    let payload = match entry.payload_size {
        Some(_) => match source.payload(entry.height).await {
            Ok(payload) => payload,
            Err(err) => {
                let _ = tx.send(Err(internal(err))).await;
                return false;
            },
        },
        None => None,
    };
    entry.payload_size = payload.as_ref().map(|p| p.len() as u64);
    if !send(tx, Event::Decided(entry)).await {
        return false;
    }
    let Some(payload) = payload else {
        return true;
    };
    let mut start = 0;
    while start < payload.len() {
        let end = (start + CHUNK_SIZE).min(payload.len());
        let chunk = PayloadChunk {
            data: payload.slice(start..end).to_vec(),
        };
        if !send(tx, Event::PayloadChunk(chunk)).await {
            return false;
        }
        start = end;
    }
    true
}

async fn send(tx: &mpsc::Sender<Result<SubscribeResponse, Status>>, event: Event) -> bool {
    tx.send(Ok(SubscribeResponse { event: Some(event) }))
        .await
        .is_ok()
}

fn receiver_stream<T: Send + 'static>(rx: mpsc::Receiver<Result<T, Status>>) -> BoxStream<T> {
    Box::pin(stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    }))
}

fn internal(err: Error) -> Status {
    Status::internal(err.to_string())
}

#[cfg(test)]
mod tests;
