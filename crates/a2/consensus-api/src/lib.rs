//! The interface between a consensus node and its API service.
//!
//! A consensus node serves two gRPC services. The
//! [`DecideLog`](proto::decide_log_server::DecideLog), served from a [`Source`], its store, with
//! [`server`], gives the API service's ingester the decided chain; the ingester connects with
//! [`DecideLogClient`]. The [`ConsensusNode`](proto::consensus_node_server::ConsensusNode), served
//! from a [`Node`] with [`node_server`], takes what only consensus can act on, such as
//! transactions; the API service connects with [`ConsensusNodeClient`]. See the proto files in
//! `proto/espresso/consensus/v1/` for the services.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    ops::Range,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt, stream};
use tokio::{
    sync::{mpsc, watch},
    time::Instant,
};
use tonic::{
    Request, Response, Status,
    transport::{Channel, Endpoint},
};

pub mod discovery;

/// Code generated from the proto definition.
#[allow(clippy::large_enum_variant)]
pub mod proto {
    tonic::include_proto!("espresso.consensus.v1");
}

use proto::{
    AckRequest, AckResponse, Decided, PayloadAdded, PayloadChunk, ServiceRecords, StakeTable,
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

/// How many views behind the newest decide an entry is final: no height before it is decided
/// any more, and its payload no longer joins the decide log. Consensus decides a missing view
/// only within this many views of the newest decide, and adds a late payload within fewer.
pub const FINALITY_VIEWS: u64 = 20;

/// How often the stream looks again for more discovery records of the newest epoch, which
/// validators send as the epoch starts.
const RECORDS_INTERVAL: Duration = match cfg!(test) {
    true => Duration::from_millis(100),
    false => Duration::from_secs(5),
};

/// How long a request to the consensus node may take.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the connection to the consensus node is pinged, and how long a ping may go
/// unanswered before the connection is dropped.
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(10);
const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(20);

/// A channel to the consensus node's API at `addr`, for its unary requests. It connects on first
/// use and again whenever the connection fails, so all requests share one connection.
pub fn channel(addr: SocketAddr) -> Channel {
    Endpoint::from_shared(format!("http://{addr}"))
        .expect("a socket address makes a valid URI")
        .connect_timeout(REQUEST_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .http2_keep_alive_interval(KEEP_ALIVE_INTERVAL)
        .keep_alive_timeout(KEEP_ALIVE_TIMEOUT)
        .keep_alive_while_idle(true)
        .connect_lazy()
}

/// What the service serves: the consensus node's decide log and payloads.
#[async_trait]
pub trait Source: Send + Sync + 'static {
    /// The entries from height `from` on, at most `limit`, in height order.
    async fn decided(&self, from: u64, limit: usize) -> Result<Vec<Decided>, Error>;

    /// The earliest height whose entry is still kept, if any.
    async fn earliest(&self) -> Result<Option<u64>, Error>;

    /// Changes after each durable write to the decide log, including entries of heights decided
    /// late and payloads added to entries.
    fn changes(&self) -> watch::Receiver<u64>;

    /// Frees the entries of the heights in `ranges`, and their payloads.
    async fn ack(&self, ranges: Vec<Range<u64>>) -> Result<(), Error>;

    /// The payload of the entry at `height`, if this node has it and still keeps the entry.
    async fn payload(&self, height: u64) -> Result<Option<Bytes>, Error>;

    /// The encoded stake table of `epoch`, if this node has built it.
    async fn stake_table(&self, epoch: u64) -> Result<Option<Bytes>, Error>;

    /// The encoded service records this node holds for `epoch`.
    async fn service_records(&self, epoch: u64) -> Result<Vec<Bytes>, Error>;
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
        // The feed runs as part of the stream, and stops with it when the subscriber goes away.
        let feed = Feed::new(self.source.clone(), tx);
        let feed = stream::once(feed.run(from)).filter_map(|()| async { None });
        Ok(Response::new(Box::pin(stream::select(
            receiver_stream(rx),
            feed,
        ))))
    }

    async fn ack(&self, request: Request<AckRequest>) -> Result<Response<AckResponse>, Status> {
        let ranges = request
            .into_inner()
            .ranges
            .into_iter()
            .map(|range| range.start..range.end)
            .collect();
        self.source.ack(ranges).await.map_err(internal)?;
        Ok(Response::new(AckResponse {}))
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
        let transaction = request.into_inner().transaction;
        match self.node.submit_transaction(transaction).await {
            Ok(()) => Ok(Response::new(SubmitTransactionResponse {})),
            Err(SubmitError::Invalid(reason)) => Err(Status::invalid_argument(reason)),
            Err(SubmitError::Overloaded(reason)) => Err(Status::resource_exhausted(reason)),
            Err(SubmitError::Internal(err)) => Err(internal(err)),
        }
    }
}

/// Sends a subscriber the decide log, and what joins it late.
struct Feed<S> {
    source: Arc<S>,
    tx: mpsc::Sender<Result<SubscribeResponse, Status>>,

    /// The next height to read in order.
    cursor: u64,

    /// The view of the newest entry sent.
    newest_view: u64,

    /// Entries sent without their payload, by height, with their view.
    awaiting: BTreeMap<u64, u64>,

    /// Heights skipped as missing, by first height, with the end and the view of the entry
    /// after them, which bounds theirs.
    missing: BTreeMap<u64, (u64, u64)>,

    /// The epochs of the first and of the newest entry sent.
    epochs: Option<(u64, u64)>,

    /// The epochs whose stake tables are sent.
    stake_tables: BTreeSet<u64>,

    /// The epoch whose records were last sent, how many, and when they were last looked for.
    records: Option<(u64, usize, Instant)>,
}

/// The subscriber went away, or the source failed and the subscriber was told.
struct Stop;

impl<S: Source> Feed<S> {
    fn new(source: Arc<S>, tx: mpsc::Sender<Result<SubscribeResponse, Status>>) -> Self {
        Self {
            source,
            tx,
            cursor: 0,
            newest_view: 0,
            awaiting: BTreeMap::new(),
            missing: BTreeMap::new(),
            epochs: None,
            stake_tables: BTreeSet::new(),
            records: None,
        }
    }

    /// Sends `Start`, the entries from `from` on, then whatever the decide log gets, until the
    /// subscriber goes away.
    async fn run(mut self, from: u64) {
        let mut changes = self.source.changes();
        if self.start(from).await.is_err() {
            return;
        }
        loop {
            changes.borrow_and_update();
            if self.pass().await.is_err() {
                return;
            }
            tokio::select! {
                changed = changes.changed() => if changed.is_err() {
                    return;
                },
                () = self.tx.closed() => return,
            }
        }
    }

    /// Sends `Start`. The cursor stays at `from`, so that heights before the earliest entry are
    /// noted as missing too: one of them can still be decided late.
    async fn start(&mut self, from: u64) -> Result<(), Stop> {
        let earliest = self.ok(self.source.earliest().await).await?;
        self.cursor = from;
        let start = Event::Start(Start {
            earliest_height: earliest.map_or(from, |earliest| earliest.max(from)),
        });
        self.send(start).await
    }

    /// Sends the new entries, then the late ones, then what the epochs of the entries need,
    /// then forgets what is final.
    async fn pass(&mut self) -> Result<(), Stop> {
        self.send_new().await?;
        self.send_late_payloads().await?;
        self.send_late_entries().await?;
        self.send_stake_tables().await?;
        self.send_records().await?;
        let newest = self.newest_view;
        let is_final = |view: u64| view + FINALITY_VIEWS <= newest;
        self.awaiting.retain(|_, view| !is_final(*view));
        self.missing.retain(|_, (_, view)| !is_final(*view));
        Ok(())
    }

    /// Sends the entries from the cursor on, noting the heights skipped.
    async fn send_new(&mut self) -> Result<(), Stop> {
        loop {
            let entries = self
                .ok(self.source.decided(self.cursor, BATCH).await)
                .await?;
            let read_all = entries.len() < BATCH;
            for entry in entries {
                if entry.height > self.cursor {
                    self.missing.insert(self.cursor, (entry.height, entry.view));
                }
                self.cursor = entry.height + 1;
                self.send_entry(entry).await?;
            }
            if read_all {
                return Ok(());
            }
        }
    }

    /// Sends the payloads that joined entries sent without them.
    async fn send_late_payloads(&mut self) -> Result<(), Stop> {
        let heights: Vec<u64> = self.awaiting.keys().copied().collect();
        for height in heights {
            let Some(payload) = self.ok(self.source.payload(height).await).await? else {
                continue;
            };
            self.awaiting.remove(&height);
            let added = Event::PayloadAdded(PayloadAdded {
                height,
                payload_size: payload.len() as u64,
            });
            self.send(added).await?;
            self.send_chunks(&payload).await?;
        }
        Ok(())
    }

    /// Sends the entries of heights skipped as missing that have been decided since.
    async fn send_late_entries(&mut self) -> Result<(), Stop> {
        for (start, (end, view)) in std::mem::take(&mut self.missing) {
            let mut next = start;
            for entry in self.decided_before(start, end).await? {
                if entry.height > next {
                    self.missing.insert(next, (entry.height, view));
                }
                next = entry.height + 1;
                self.send_entry(entry).await?;
            }
            if next < end {
                self.missing.insert(next, (end, view));
            }
        }
        Ok(())
    }

    /// The entries from `start` up to `end`, read a batch at a time, so that a long range with
    /// few entries costs a single read.
    async fn decided_before(&self, start: u64, end: u64) -> Result<Vec<Decided>, Stop> {
        let mut found = Vec::new();
        let mut from = start;
        loop {
            let entries = self.ok(self.source.decided(from, BATCH).await).await?;
            let read_all = entries.len() < BATCH;
            for entry in entries {
                if entry.height >= end {
                    return Ok(found);
                }
                from = entry.height + 1;
                found.push(entry);
            }
            if read_all {
                return Ok(found);
            }
        }
    }

    /// Sends the stake tables of the epochs from the first entry's to two after the newest
    /// entry's, the furthest one the node can know, as the node builds them.
    async fn send_stake_tables(&mut self) -> Result<(), Stop> {
        let Some((first, newest)) = self.epochs else {
            return Ok(());
        };
        for epoch in first..=newest + 2 {
            if self.stake_tables.contains(&epoch) {
                continue;
            }
            let Some(table) = self.ok(self.source.stake_table(epoch).await).await? else {
                continue;
            };
            self.send(Event::StakeTable(StakeTable { epoch, table }))
                .await?;
            self.stake_tables.insert(epoch);
        }
        Ok(())
    }

    /// Sends the records of the newest entry's epoch once the node holds any, and again
    /// whenever it holds more, looking at most every [`RECORDS_INTERVAL`].
    async fn send_records(&mut self) -> Result<(), Stop> {
        let Some((_, epoch)) = self.epochs else {
            return Ok(());
        };
        let sent = match self.records {
            Some((sent_epoch, count, at)) if sent_epoch == epoch => {
                if at.elapsed() < RECORDS_INTERVAL {
                    return Ok(());
                }
                count
            },
            _ => 0,
        };
        let records = self.ok(self.source.service_records(epoch).await).await?;
        let count = records.len();
        if count > sent {
            self.send(Event::ServiceRecords(ServiceRecords { epoch, records }))
                .await?;
        }
        self.records = Some((epoch, count.max(sent), Instant::now()));
        Ok(())
    }

    /// Sends `entry` and then its payload in chunks, reading the payload only now so that at
    /// most one is held at a time. A payload freed since the entry was read leaves the entry
    /// without one.
    async fn send_entry(&mut self, mut entry: Decided) -> Result<(), Stop> {
        let payload = match entry.payload_size {
            Some(_) => self.ok(self.source.payload(entry.height).await).await?,
            None => None,
        };
        entry.payload_size = payload.as_ref().map(|p| p.len() as u64);
        self.newest_view = self.newest_view.max(entry.view);
        self.epochs = Some(match self.epochs {
            Some((first, newest)) => (first.min(entry.epoch), newest.max(entry.epoch)),
            None => (entry.epoch, entry.epoch),
        });
        if payload.is_none() {
            self.awaiting.insert(entry.height, entry.view);
        }
        self.send(Event::Decided(entry)).await?;
        match payload {
            Some(payload) => self.send_chunks(&payload).await,
            None => Ok(()),
        }
    }

    async fn send_chunks(&self, payload: &Bytes) -> Result<(), Stop> {
        let mut start = 0;
        while start < payload.len() {
            let end = (start + CHUNK_SIZE).min(payload.len());
            let chunk = PayloadChunk {
                data: payload.slice(start..end),
            };
            self.send(Event::PayloadChunk(chunk)).await?;
            start = end;
        }
        Ok(())
    }

    async fn send(&self, event: Event) -> Result<(), Stop> {
        self.tx
            .send(Ok(SubscribeResponse { event: Some(event) }))
            .await
            .map_err(|_| Stop)
    }

    /// The value of `result`; an error is sent to the subscriber and stops the feed.
    async fn ok<T>(&self, result: Result<T, Error>) -> Result<T, Stop> {
        match result {
            Ok(value) => Ok(value),
            Err(err) => {
                let _ = self.tx.send(Err(internal(err))).await;
                Err(Stop)
            },
        }
    }
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
