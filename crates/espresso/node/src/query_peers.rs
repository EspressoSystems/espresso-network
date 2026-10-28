//! Query service requests between staked peers over the cliquenet query lane, so large replies
//! never share a connection with consensus messages.

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result, bail, ensure};
use espresso_types::{Certificate2, PubKey, SeqTypes};
use hotshot_new_protocol::network::{QueryReceiver, QuerySender};
use hotshot_query_service::{
    availability::{BlockHash, LeafHash, LeafQueryData},
    types::HeightIndexed,
};
use hotshot_types::data::VidCommitment;
use light_client::consensus::{leaf::LeafProof, payload::PayloadProof};
use parking_lot::Mutex;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tokio::{sync::oneshot, time::timeout};
use tracing::debug;

use crate::{
    api::context::ConsensusSource, context::TaskList, request_response::data_source::RequestStorage,
};

/// What a light client asks a peer for. Each is answered from stored data only.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueryRequest {
    /// A finality proof for a leaf, as the light client API serves it.
    LeafProof {
        leaf: LeafRef,
        finalized: Option<u64>,
    },
    /// The leaves in a range, unproven.
    Leaves(Range<u64>),
    /// The payload and VID common at a height.
    PayloadProof(u64),
    /// The certificate2 at a height, unverified.
    Cert2(u64),
}

/// The ways a light client names a leaf.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LeafRef {
    Height(u64),
    Hash(LeafHash<SeqTypes>),
    BlockHash(BlockHash<SeqTypes>),
    PayloadHash(VidCommitment),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueryResponse {
    LeafProof(LeafProof),
    Leaves(Vec<LeafQueryData<SeqTypes>>),
    PayloadProof(PayloadProof),
    Cert2(Option<Certificate2<SeqTypes>>),
    /// The peer does not have the data.
    Unavailable,
}

/// A message on the query lane. The requester picks `id`, and only the asked peer's reply with
/// that id is accepted.
#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Request { id: u64, request: QueryRequest },
    Reply { id: u64, response: QueryResponse },
}

impl QueryResponse {
    /// Whether this is the kind of response `request` asks for, covering exactly its heights.
    ///
    /// Proofs are verified by the light client. This only rejects a response of the wrong shape.
    fn answers(&self, request: &QueryRequest) -> bool {
        match (request, self) {
            (QueryRequest::LeafProof { .. }, QueryResponse::LeafProof(_))
            | (QueryRequest::PayloadProof(_), QueryResponse::PayloadProof(_))
            | (QueryRequest::Cert2(_), QueryResponse::Cert2(_)) => true,
            (QueryRequest::Leaves(range), QueryResponse::Leaves(leaves)) => {
                leaves.iter().map(|leaf| leaf.height()).eq(range.clone())
            },
            (
                QueryRequest::LeafProof { .. }
                | QueryRequest::Leaves(_)
                | QueryRequest::PayloadProof(_)
                | QueryRequest::Cert2(_),
                _,
            ) => false,
        }
    }
}

/// Asks staked peers over the query lane, and answers theirs from this node's query storage.
#[derive(Clone)]
pub struct QueryPeers(Arc<Inner>);

struct Inner {
    sender: QuerySender<SeqTypes>,
    consensus: Arc<dyn ConsensusSource>,
    public_key: PubKey,
    pending: Mutex<HashMap<(PubKey, u64), oneshot::Sender<QueryResponse>>>,
    next_id: AtomicU64,
    /// Peers that answered before, asked first since most validators may not serve queries.
    serving: Mutex<HashSet<PubKey>>,
}

impl QueryPeers {
    /// Start answering peers from `storage` and return the handle that asks them.
    pub fn start(
        (sender, receiver): (QuerySender<SeqTypes>, QueryReceiver<SeqTypes>),
        storage: Arc<dyn RequestStorage>,
        consensus: Arc<dyn ConsensusSource>,
        public_key: PubKey,
        tasks: &mut TaskList,
    ) -> QueryPeers {
        let peers = QueryPeers(Arc::new(Inner {
            sender,
            consensus,
            public_key,
            pending: Default::default(),
            next_id: AtomicU64::new(0),
            serving: Default::default(),
        }));
        tasks.spawn("query peers", peers.clone().run(receiver, storage));
        peers
    }

    /// Ask one staked peer. The response is unverified.
    pub async fn ask(&self, request: QueryRequest) -> Result<QueryResponse> {
        let peer = self.choose_peer().await.context("no staked peer to ask")?;
        let id = self.0.next_id.fetch_add(1, Ordering::Relaxed);
        let (waiter, reply) = oneshot::channel();
        self.0.pending.lock().insert((peer, id), waiter);
        let _pending = Pending {
            peers: self,
            key: (peer, id),
        };
        let message = Message::Request {
            id,
            request: request.clone(),
        };
        self.0.sender.send(&peer, bincode::serialize(&message)?)?;

        let wait = self.0.sender.receive_timeout();
        let Ok(Ok(response)) = timeout(wait, reply).await else {
            self.0.serving.lock().remove(&peer);
            bail!("peer {peer} did not answer");
        };
        self.0.serving.lock().insert(peer);
        ensure!(
            response.answers(&request),
            "peer {peer} could not answer {request:?}"
        );
        Ok(response)
    }

    async fn choose_peer(&self) -> Option<PubKey> {
        let serving = self.0.serving.lock().iter().copied().collect::<Vec<_>>();
        if let Some(peer) = serving.choose(&mut rand::thread_rng()) {
            return Some(*peer);
        }
        let epoch = self.0.consensus.current_epoch().await;
        let membership = self.0.consensus.membership_coordinator().await;
        let stake_table = membership.stake_table_for_epoch(epoch).ok()?;
        let peers = stake_table
            .stake_table()
            .map(|entry| entry.stake_table_entry.stake_key)
            .filter(|key| *key != self.0.public_key)
            .collect::<Vec<_>>();
        peers.choose(&mut rand::thread_rng()).copied()
    }

    /// Answer requests and route replies, one message at a time, until the network shuts down.
    async fn run(self, mut receiver: QueryReceiver<SeqTypes>, storage: Arc<dyn RequestStorage>) {
        while let Some((from, bytes)) = receiver.receive().await {
            match bincode::deserialize(&bytes) {
                Ok(Message::Request { id, request }) => {
                    let response = answer(&*storage, request).await.unwrap_or_else(|err| {
                        debug!(%from, id, err = %format_args!("{err:#}"), "cannot answer query");
                        QueryResponse::Unavailable
                    });
                    let reply = bincode::serialize(&Message::Reply { id, response });
                    if let Err(err) = reply
                        .map_err(anyhow::Error::from)
                        .and_then(|reply| Ok(self.0.sender.send(&from, reply)?))
                    {
                        debug!(%from, id, err = %format_args!("{err:#}"), "cannot send query reply");
                    }
                },
                Ok(Message::Reply { id, response }) => {
                    let waiter = self.0.pending.lock().remove(&(from, id));
                    if let Some(waiter) = waiter {
                        _ = waiter.send(response);
                    }
                },
                Err(err) => debug!(%from, %err, "undecodable query message"),
            }
        }
    }
}

async fn answer(storage: &dyn RequestStorage, request: QueryRequest) -> Result<QueryResponse> {
    Ok(match request {
        QueryRequest::LeafProof { leaf, finalized } => {
            QueryResponse::LeafProof(storage.get_leaf_proof(leaf, finalized).await?)
        },
        QueryRequest::Leaves(range) => QueryResponse::Leaves(storage.get_leaves(range).await?),
        QueryRequest::PayloadProof(height) => {
            QueryResponse::PayloadProof(storage.get_payload_proof(height).await?)
        },
        QueryRequest::Cert2(height) => QueryResponse::Cert2(storage.load_cert2(height).await?),
    })
}

/// Removes a call's waiter when the call ends, however it ends.
struct Pending<'a> {
    peers: &'a QueryPeers,
    key: (PubKey, u64),
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        self.peers.0.pending.lock().remove(&self.key);
    }
}
