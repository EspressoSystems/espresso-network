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
use tokio::sync::{broadcast, mpsc, watch};
use tonic::{Request, Response, Status};

/// Code generated from the proto definition.
#[allow(clippy::large_enum_variant)]
pub mod proto {
    tonic::include_proto!("espresso.consensus.v1");
}

use proto::{
    AckRequest, AckResponse, Decided, GetPayloadRequest, PayloadChunk, PayloadReady,
    PayloadsReadyRequest, ServiceRecordsRequest, ServiceRecordsResponse, Start,
    SubmitTransactionRequest, SubmitTransactionResponse, SubscribeRequest, SubscribeResponse,
    consensus_node_server::{ConsensusNode, ConsensusNodeServer},
    decide_log_server::{DecideLog, DecideLogServer},
    get_payload_request::Key,
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

    /// A payload, if this node has it.
    async fn payload(&self, key: PayloadKey) -> Result<Option<Bytes>, Error>;

    /// Payloads as they are stored, before their block is decided.
    fn payloads_ready(&self) -> broadcast::Receiver<PayloadReady>;

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

/// Which payload [`Source::payload`] returns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadKey {
    /// The payload of the block decided at a height.
    Height(u64),

    /// The payload of a block, by the view it was proposed in and its payload commitment.
    Block {
        view: u64,
        payload_commitment: String,
    },
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
    type GetPayloadStream = BoxStream<PayloadChunk>;
    type PayloadsReadyStream = BoxStream<PayloadReady>;

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

    async fn get_payload(
        &self,
        request: Request<GetPayloadRequest>,
    ) -> Result<Response<Self::GetPayloadStream>, Status> {
        let key = match request.into_inner().key {
            Some(Key::Height(height)) => PayloadKey::Height(height),
            Some(Key::Block(block)) => PayloadKey::Block {
                view: block.view,
                payload_commitment: block.payload_commitment,
            },
            None => return Err(Status::invalid_argument("no payload key")),
        };
        let payload = self
            .source
            .payload(key)
            .await
            .map_err(internal)?
            .ok_or_else(|| Status::not_found("no such payload"))?;
        let chunks: Vec<_> = (0..payload.len().max(1))
            .step_by(CHUNK_SIZE)
            .map(|start| {
                let end = (start + CHUNK_SIZE).min(payload.len());
                Ok(PayloadChunk {
                    data: payload.slice(start..end).to_vec(),
                })
            })
            .collect();
        Ok(Response::new(Box::pin(stream::iter(chunks))))
    }

    async fn payloads_ready(
        &self,
        _: Request<PayloadsReadyRequest>,
    ) -> Result<Response<Self::PayloadsReadyStream>, Status> {
        let ready = stream::unfold(self.source.payloads_ready(), |mut rx| async move {
            loop {
                match rx.recv().await {
                    Ok(ready) => return Some((Ok(ready), rx)),
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        });
        Ok(Response::new(Box::pin(ready)))
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
            if !send(&tx, Event::Decided(entry)).await {
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
