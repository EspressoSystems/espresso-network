//! Query service requests between peers over a cliquenet lane, so large replies never share a
//! connection with consensus messages.
//!
//! A request runs one of this node's HTTP API handlers, so a peer gets what an HTTP client of the
//! node would. The query module builds [`QueryHandlers`] when the lane is on, consensus init
//! configures the lane for them and starts [`QueryLane`] as soon as the network exists, and the
//! light client asks through it.

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Context, Result, anyhow, bail};
use espresso_api::{DynCatchupApi, DynHotShotAvailabilityApi, DynLightClientApi, v1};
use espresso_types::{PubKey, SeqTypes};
use hotshot_new_protocol::network::{Cliquenet, LaneEvent, LaneId, LaneReceiver, LaneSender};
use parking_lot::Mutex;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use tokio::{
    select,
    sync::{Semaphore, oneshot},
    task::JoinSet,
    time::timeout,
};
use tracing::{debug, error};

use crate::context::TaskList;

/// The cliquenet lane query requests and replies travel on.
///
/// Every node must use the same id. A change to [`QueryRequest`], or to what a request answers,
/// needs a new id: a node refuses a lane it does not serve, so its peers move on at once instead
/// of misreading each other.
pub(crate) const QUERY_LANE: LaneId = LaneId::new(1);

/// What a light client asks a peer for, one variant per HTTP API handler it runs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueryRequest {
    LeafProof {
        leaf: v1::LeafQuery,
        finalized: Option<u64>,
    },
    HeaderProof {
        root: u64,
        header: v1::HeaderQuery,
    },
    Leaves(Vec<Range<u64>>),
    PayloadProof(u64),
    PayloadProofs(Vec<Range<u64>>),
    StakeTableEvents(u64),
    Cert2(u64),
}

/// The handler's output, bincode encoded, or its error.
type Response = Result<Vec<u8>, String>;

/// A message on the query lane. The requester picks `id`, and only the asked peer's reply with
/// that id is accepted.
#[derive(Debug, Serialize, Deserialize)]
enum Message {
    Request { id: u64, request: QueryRequest },
    Reply { id: u64, response: Response },
}

/// The HTTP API handlers that answer peers.
///
/// They exist only when the query lane is on, so having them turns the lane on.
#[derive(Clone)]
pub struct QueryHandlers {
    light_client: Arc<dyn DynLightClientApi>,
    availability: Arc<dyn DynHotShotAvailabilityApi>,
    catchup: Arc<dyn DynCatchupApi>,
    limit: Option<usize>,
}

impl QueryHandlers {
    /// Handlers running at most `limit` answers at once, or the network's peer budget when
    /// `None`.
    pub fn new<S>(api: S, limit: Option<usize>) -> QueryHandlers
    where
        S: DynLightClientApi + DynHotShotAvailabilityApi + DynCatchupApi + 'static,
    {
        let api = Arc::new(api);
        QueryHandlers {
            light_client: api.clone(),
            availability: api.clone(),
            catchup: api,
            limit,
        }
    }
}

/// This node's end of the query lane: it asks peers, and answers them with its handlers.
#[derive(Clone)]
pub struct QueryLane(Arc<Requester>);

impl QueryLane {
    /// Start answering peers on the query lane of `network`, and return the handle that asks
    /// them, or `None` when the network has no query lane.
    pub fn start(
        network: &mut Cliquenet<SeqTypes>,
        handlers: QueryHandlers,
        tasks: &mut TaskList,
    ) -> Option<QueryLane> {
        let (sender, receiver) = network.take_lane(QUERY_LANE)?;
        let limit = handlers.limit.unwrap_or(sender.peer_budget().get());
        let requester = Arc::new(Requester {
            sender: sender.clone(),
            pending: Default::default(),
            next_id: AtomicU64::new(0),
            silent: Default::default(),
        });
        let responder = Responder {
            sender,
            handlers,
            slots: Arc::new(Semaphore::new(limit)),
            answering: JoinSet::new(),
        };
        tasks.spawn("query lane", run(receiver, requester.clone(), responder));
        Some(QueryLane(requester))
    }

    /// Ask one peer, chosen at random, returning the bincode encoded output of its handler.
    /// The output is unverified.
    ///
    /// The choice is random on every ask, so a peer that lacks data or answers wrongly gets only
    /// its share of the asks, and the next attempt goes to someone else.
    pub async fn ask(&self, request: QueryRequest) -> Result<Vec<u8>> {
        self.0.ask(request).await
    }
}

/// Route replies to their waiters and admit requests, until the network shuts down.
async fn run(
    mut receiver: LaneReceiver<SeqTypes>,
    requester: Arc<Requester>,
    mut responder: Responder,
) {
    loop {
        select! {
            event = receiver.receive() => match event {
                Some(LaneEvent::Message(from, bytes)) => match bincode::deserialize(&bytes) {
                    Ok(Message::Request { id, request }) => responder.on_request(from, id, request),
                    Ok(Message::Reply { id, response }) => requester.on_reply(from, id, response),
                    Err(err) => debug!(%from, %err, "undecodable query message"),
                },
                Some(LaneEvent::Unreachable(peer)) => requester.on_unreachable(&peer),
                None => break,
            },
            Some(answered) = responder.answering.join_next() => {
                if let Err(err) = answered
                    && err.is_panic()
                {
                    error!(%err, "query answer panicked");
                }
            },
        }
    }
}

struct Requester {
    sender: LaneSender<SeqTypes>,
    pending: Mutex<HashMap<(PubKey, u64), oneshot::Sender<Response>>>,
    next_id: AtomicU64,
    /// Peers that could not be reached or did not answer. They are asked again only once every
    /// other peer has joined them.
    silent: Mutex<HashSet<PubKey>>,
}

impl Requester {
    async fn ask(&self, request: QueryRequest) -> Result<Vec<u8>> {
        let peer = self.choose_peer().context("no peer to ask")?;
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (waiter, reply) = oneshot::channel();
        let _pending = Pending::new(self, (peer, id), waiter);
        let message = bincode::serialize(&Message::Request { id, request })?;
        if let Err(err) = self.sender.send(&peer, message) {
            self.silent.lock().insert(peer);
            return Err(err.into());
        }

        let Ok(Ok(response)) = timeout(self.sender.receive_timeout(), reply).await else {
            self.silent.lock().insert(peer);
            bail!("peer {peer} did not answer");
        };
        // Kept as text, so `FallbackClient` takes one peer's miss as that peer failing, not as
        // the data being absent, which would stop it asking anyone else.
        response.map_err(|err| anyhow!("peer {peer} failed: {err}"))
    }

    fn choose_peer(&self) -> Option<PubKey> {
        let peers = self.sender.peers();
        let mut silent = self.silent.lock();
        let fresh = peers
            .iter()
            .filter(|key| !silent.contains(*key))
            .copied()
            .collect::<Vec<_>>();
        if let Some(peer) = fresh.choose(&mut rand::thread_rng()) {
            return Some(*peer);
        }
        // Every peer has failed once, so each is worth another try.
        silent.clear();
        peers.choose(&mut rand::thread_rng()).copied()
    }

    fn on_reply(&self, from: PubKey, id: u64, response: Response) {
        let waiter = self.pending.lock().remove(&(from, id));
        if let Some(waiter) = waiter {
            _ = waiter.send(response);
        }
    }

    /// What was sent to `peer` may be lost, so its calls fail now instead of at their timeout.
    fn on_unreachable(&self, peer: &PubKey) {
        self.pending.lock().retain(|(key, _), _| key != peer);
    }
}

struct Responder {
    sender: LaneSender<SeqTypes>,
    handlers: QueryHandlers,
    slots: Arc<Semaphore>,
    answering: JoinSet<()>,
}

impl Responder {
    fn on_request(&mut self, from: PubKey, id: u64, request: QueryRequest) {
        let Ok(slot) = self.slots.clone().try_acquire_owned() else {
            debug!(%from, id, "too many query requests in flight");
            reply(
                &self.sender,
                from,
                id,
                Err("too many requests in flight".to_owned()),
            );
            return;
        };
        let sender = self.sender.clone();
        let handlers = self.handlers.clone();
        self.answering.spawn(async move {
            let response = answer(&handlers, request).await;
            reply(&sender, from, id, response);
            drop(slot);
        });
    }
}

async fn answer(handlers: &QueryHandlers, request: QueryRequest) -> Response {
    let QueryHandlers {
        light_client,
        availability,
        catchup,
        ..
    } = handlers;
    let output = match request {
        QueryRequest::LeafProof { leaf, finalized } => {
            light_client.get_leaf_proof(leaf, finalized).await
        },
        QueryRequest::HeaderProof { root, header } => {
            light_client.get_header_proof(root, header).await
        },
        QueryRequest::Leaves(ranges) => availability.get_leaf_ranges(ranges).await,
        QueryRequest::PayloadProof(height) => light_client.get_payload_proof(height).await,
        QueryRequest::PayloadProofs(ranges) => light_client.get_payload_proof_ranges(ranges).await,
        QueryRequest::StakeTableEvents(epoch) => {
            light_client.get_light_client_stake_table(epoch).await
        },
        // The catchup handler reads only this node's storage. The availability one fetches a
        // missing cert2 from peers, so a request for a cert2 nobody has would hop from node to
        // node.
        QueryRequest::Cert2(height) => catchup.get_cert2(height).await,
    };
    let output = output.map_err(|err| format!("{err:#}"))?;
    bincode::serialize(&output).map_err(|err| format!("{err:#}"))
}

/// Send a reply, or the reason it could not be sent in its place.
fn reply(sender: &LaneSender<SeqTypes>, to: PubKey, id: u64, response: Response) {
    let send = |response| -> Result<()> {
        let message = bincode::serialize(&Message::Reply { id, response })?;
        Ok(sender.send(&to, message)?)
    };
    if let Err(err) = send(response) {
        debug!(%to, id, err = %format_args!("{err:#}"), "cannot send query reply");
        _ = send(Err(format!("{err:#}")));
    }
}

/// Registers a call's waiter and removes it when the call ends, however it ends.
struct Pending<'a> {
    requester: &'a Requester,
    key: (PubKey, u64),
}

impl<'a> Pending<'a> {
    fn new(
        requester: &'a Requester,
        key: (PubKey, u64),
        waiter: oneshot::Sender<Response>,
    ) -> Pending<'a> {
        requester.pending.lock().insert(key, waiter);
        Pending { requester, key }
    }
}

impl Drop for Pending<'_> {
    fn drop(&mut self) {
        self.requester.pending.lock().remove(&self.key);
    }
}
