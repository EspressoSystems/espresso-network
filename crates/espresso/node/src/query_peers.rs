//! Query service requests between staked peers over the cliquenet query lane, so large replies
//! never share a connection with consensus messages.
//!
//! Requests are answered by the handlers the node's HTTP API runs, so a peer gets exactly what an
//! HTTP client of the node would.

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result, bail};
use espresso_api::{
    DynHotShotAvailabilityApi, DynLightClientApi, Erased,
    error::{AvailabilityError, classify},
    v1,
};
use espresso_types::{PubKey, SeqTypes};
use hotshot_new_protocol::network::{QueryReceiver, QuerySender};
use hotshot_query_service::Error;
use http_client::{StatusCode, error::ClientError as _};
use parking_lot::Mutex;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tokio::{select, sync::oneshot, task::JoinSet, time::timeout};
use tracing::{debug, error};

use crate::{api::context::ConsensusSource, context::TaskList};

/// What a light client asks a peer for, one variant per HTTP API handler it runs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueryRequest {
    LeafProof {
        leaf: v1::LeafQuery,
        finalized: Option<u64>,
    },
    Leaves(Vec<Range<u64>>),
    PayloadProof(u64),
    StakeTableEvents(u64),
    Cert2(u64),
}

/// The handler's response, bincode encoded, or its error as the HTTP API reports it.
type Reply = Result<Vec<u8>, Error>;

/// A message on the query lane. The requester picks `id`, and only the asked peer's reply with
/// that id is accepted.
#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Request { id: u64, request: QueryRequest },
    Reply { id: u64, response: Reply },
}

/// Asks staked peers over the query lane, and answers theirs with this node's API handlers.
#[derive(Clone)]
pub struct QueryPeers(Arc<Inner>);

struct Inner {
    sender: QuerySender<SeqTypes>,
    consensus: Arc<dyn ConsensusSource>,
    public_key: PubKey,
    /// Attached by the query module once the node context exists. Requests before that fail
    /// with 503.
    handlers: OnceLock<(
        Arc<dyn DynLightClientApi>,
        Arc<dyn DynHotShotAvailabilityApi>,
    )>,
    pending: Mutex<HashMap<(PubKey, u64), oneshot::Sender<Reply>>>,
    next_id: AtomicU64,
    /// Peers whose last answer was a success. They are asked first, since most validators may
    /// not serve queries.
    serving: Mutex<HashSet<PubKey>>,
    /// Peers that could not be reached or did not answer. They are asked again only once every
    /// other staked peer has joined them.
    silent: Mutex<HashSet<PubKey>>,
}

impl QueryPeers {
    /// Start answering peers and return the handle that asks them.
    pub fn start(
        (sender, receiver): (QuerySender<SeqTypes>, QueryReceiver<SeqTypes>),
        consensus: Arc<dyn ConsensusSource>,
        public_key: PubKey,
        tasks: &mut TaskList,
    ) -> QueryPeers {
        let peers = QueryPeers(Arc::new(Inner {
            sender,
            consensus,
            public_key,
            handlers: OnceLock::new(),
            pending: Default::default(),
            next_id: AtomicU64::new(0),
            serving: Default::default(),
            silent: Default::default(),
        }));
        tasks.spawn("query peers", peers.clone().run(receiver));
        peers
    }

    /// Answer peers with these HTTP API handlers from now on.
    pub fn serve_with(
        &self,
        light_client: Arc<dyn DynLightClientApi>,
        availability: Arc<dyn DynHotShotAvailabilityApi>,
    ) {
        _ = self.0.handlers.set((light_client, availability));
    }

    /// Ask one staked peer, returning the bincode encoded response of its handler. The response
    /// is unverified.
    pub async fn ask(&self, request: QueryRequest) -> Result<Vec<u8>> {
        let peer = self.choose_peer().await.context("no staked peer to ask")?;
        let id = self.0.next_id.fetch_add(1, Ordering::Relaxed);
        let (waiter, reply) = oneshot::channel();
        let _pending = Pending::new(self, (peer, id), waiter);
        let message = Message::Request { id, request };
        if let Err(err) = self.0.sender.send(&peer, bincode::serialize(&message)?) {
            self.0.serving.lock().remove(&peer);
            self.0.silent.lock().insert(peer);
            return Err(err.into());
        }

        let wait = self.0.sender.receive_timeout();
        let Ok(Ok(response)) = timeout(wait, reply).await else {
            self.0.serving.lock().remove(&peer);
            self.0.silent.lock().insert(peer);
            bail!("peer {peer} did not answer");
        };
        match response {
            Ok(response) => {
                self.0.serving.lock().insert(peer);
                Ok(response)
            },
            Err(err) => {
                self.0.serving.lock().remove(&peer);
                Err(err).with_context(|| format!("peer {peer} failed"))
            },
        }
    }

    async fn choose_peer(&self) -> Option<PubKey> {
        let serving = self.0.serving.lock().iter().copied().collect::<Vec<_>>();
        if let Some(peer) = serving.choose(&mut rand::thread_rng()) {
            return Some(*peer);
        }
        let epoch = self.0.consensus.current_epoch().await;
        let membership = self.0.consensus.membership_coordinator().await;
        let stake_table = membership.stake_table_for_epoch(epoch).ok()?;
        let staked = stake_table
            .stake_table()
            .map(|entry| entry.stake_table_entry.stake_key)
            .filter(|key| *key != self.0.public_key)
            .collect::<Vec<_>>();
        let mut silent = self.0.silent.lock();
        let fresh = staked
            .iter()
            .filter(|key| !silent.contains(*key))
            .copied()
            .collect::<Vec<_>>();
        if !fresh.is_empty() {
            return fresh.choose(&mut rand::thread_rng()).copied();
        }
        // Every staked peer has failed once, so each is worth another try.
        silent.clear();
        staked.choose(&mut rand::thread_rng()).copied()
    }

    /// Route replies and answer requests until the network shuts down.
    async fn run(self, mut receiver: QueryReceiver<SeqTypes>) {
        // Capped by the network's per peer budget, so a burst of requests can neither pile up
        // handler work nor hold back the replies this node is waiting for.
        let budget = self.0.sender.peer_budget().get();
        let mut answering = JoinSet::new();
        loop {
            select! {
                received = receiver.receive() => {
                    let Some((from, bytes)) = received else { break };
                    // The network gave up on the peer's lane, so nothing asked of it gets answered.
                    if bytes.is_empty() {
                        self.0.pending.lock().retain(|(peer, _), _| *peer != from);
                        continue;
                    }
                    match bincode::deserialize(&bytes) {
                        Ok(Message::Request { id, request }) if answering.len() < budget => {
                            answering.spawn(self.clone().serve(from, id, request));
                        },
                        Ok(Message::Request { id, request: _ }) => {
                            debug!(%from, id, "too many query requests in flight");
                            let busy = "too many requests in flight".to_owned();
                            self.reply(from, id, Err(Error::catch_all(StatusCode::SERVICE_UNAVAILABLE, busy)));
                        },
                        Ok(Message::Reply { id, response }) => {
                            let waiter = self.0.pending.lock().remove(&(from, id));
                            if let Some(waiter) = waiter {
                                _ = waiter.send(response);
                            }
                        },
                        Err(err) => debug!(%from, %err, "undecodable query message"),
                    }
                },
                Some(served) = answering.join_next(), if !answering.is_empty() => {
                    if let Err(err) = served
                        && err.is_panic()
                    {
                        error!(%err, "query answer panicked");
                    }
                },
            }
        }
    }

    async fn serve(self, from: PubKey, id: u64, request: QueryRequest) {
        let response = match self.0.handlers.get() {
            Some((light_client, availability)) => {
                match answer(&**light_client, &**availability, request).await {
                    Ok(value) => bincode::serialize(&value).map_err(|err| {
                        Error::catch_all(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
                    }),
                    Err(err) => {
                        let err = classify(err);
                        Err(Error::catch_all(err.status(), err.to_string()))
                    },
                }
            },
            None => Err(Error::catch_all(
                StatusCode::SERVICE_UNAVAILABLE,
                "query service is starting".to_owned(),
            )),
        };
        self.reply(from, id, response);
    }

    fn reply(&self, to: PubKey, id: u64, response: Reply) {
        if let Err(err) = self.send_reply(to, id, response) {
            debug!(%to, id, err = %format_args!("{err:#}"), "cannot send query reply");
            let too_large = Error::catch_all(
                StatusCode::INTERNAL_SERVER_ERROR,
                "reply does not fit a message".to_owned(),
            );
            _ = self.send_reply(to, id, Err(too_large));
        }
    }

    fn send_reply(&self, to: PubKey, id: u64, response: Reply) -> Result<()> {
        let reply = bincode::serialize(&Message::Reply { id, response })?;
        Ok(self.0.sender.send(&to, reply)?)
    }
}

async fn answer(
    light_client: &dyn DynLightClientApi,
    availability: &dyn DynHotShotAvailabilityApi,
    request: QueryRequest,
) -> Result<Erased> {
    match request {
        QueryRequest::LeafProof { leaf, finalized } => {
            light_client.get_leaf_proof(leaf, finalized).await
        },
        QueryRequest::Leaves(ranges) => availability.get_leaf_ranges(ranges).await,
        QueryRequest::PayloadProof(height) => light_client.get_payload_proof(height).await,
        QueryRequest::StakeTableEvents(epoch) => {
            light_client.get_light_client_stake_table(epoch).await
        },
        // The HTTP API answers a missing cert2 with a 404 too.
        QueryRequest::Cert2(height) => availability.get_cert2(height).await?.ok_or_else(|| {
            AvailabilityError::NotFound(format!("no cert2 available for height {height}")).into()
        }),
    }
}

/// Registers a call's waiter and removes it when the call ends, however it ends.
struct Pending<'a> {
    peers: &'a QueryPeers,
    key: (PubKey, u64),
}

impl<'a> Pending<'a> {
    fn new(
        peers: &'a QueryPeers,
        key: (PubKey, u64),
        waiter: oneshot::Sender<Reply>,
    ) -> Pending<'a> {
        peers.0.pending.lock().insert(key, waiter);
        Pending { peers, key }
    }
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        self.peers.0.pending.lock().remove(&self.key);
    }
}
