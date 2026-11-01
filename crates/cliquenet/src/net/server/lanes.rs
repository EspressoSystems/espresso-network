use std::{collections::HashMap, mem, sync::Arc};

use bytes::Bytes;
use tokio::{select, task::JoinError};
use tokio_util::{sync::CancellationToken, task::JoinMap};
use tracing::{debug, error, warn};

use super::remove_trailer;
use crate::{
    Config, LaneId, Metrics, NetAddr, PublicKey,
    connection::Connection,
    delay::DelayQueue,
    error::NetworkError,
    metrics::LaneMetrics,
    msg::{MsgId, Slot},
    net::{LaneDelivery, RetryPolicy, peer::Peer},
    queue::Queue,
};

/// The extra lanes to every party.
///
/// Unlike the main connection, a lane is opened by a send and not kept up:
/// once its connection ends it is dialed again only for messages still
/// waiting on it, and a dial that fails drops them and reports the party
/// unreachable.
pub(super) struct Lanes {
    key: PublicKey,
    conf: Arc<Config>,
    deliveries: HashMap<LaneId, LaneDelivery>,
    links: HashMap<(PublicKey, LaneId), Link>,
    connect_tasks: JoinMap<(PublicKey, LaneId), Option<Connection>>,
    peer_tasks: JoinMap<(PublicKey, LaneId), Peer>,
    msgid: MsgId,
    metrics: Arc<dyn Metrics>,
}

struct Link {
    addr: NetAddr,
    outbox: Queue<(RetryPolicy, Bytes)>,
    state: LinkState,
}

enum LinkState {
    Idle,
    Dialing,
    /// A peer task runs the connection. A dial may still be running, when
    /// the party opened the lane while we dialed it.
    Connected(CancellationToken),
    /// The peer task was cancelled, and this connection takes over once it
    /// ends.
    Replacing(Box<Connection>),
}

/// Who opened a connection, which decides the one kept when both sides open
/// a lane at once.
#[derive(Clone, Copy)]
enum Origin {
    Accepted,
    Dialed,
}

pub(super) enum Event {
    Dialed((PublicKey, LaneId), Result<Option<Connection>, JoinError>),
    Ended((PublicKey, LaneId), Result<Peer, JoinError>),
}

impl Lanes {
    pub(super) fn new(
        conf: Arc<Config>,
        deliveries: HashMap<LaneId, LaneDelivery>,
        metrics: Arc<dyn Metrics>,
    ) -> Self {
        Self {
            key: conf.keypair.public_key(),
            conf,
            deliveries,
            links: HashMap::new(),
            connect_tasks: JoinMap::new(),
            peer_tasks: JoinMap::new(),
            msgid: MsgId::new(0),
            metrics,
        }
    }

    /// Queue `bytes` for `to` on lane `id`, opening the lane if it is idle.
    ///
    /// `addr` is `None` when `to` is not a party.
    pub(super) fn send(&mut self, id: LaneId, to: PublicKey, addr: Option<&NetAddr>, bytes: Bytes) {
        if to == self.key {
            let message = (to, remove_trailer(bytes), None);
            if self.delivery(id).messages.send(message).is_err() {
                warn!(name = %self.conf.name, node = %self.key, lane = %id, "lane channel closed");
            }
            return;
        }
        let Some(addr) = addr else {
            debug!(name = %self.conf.name, node = %self.key, peer = %to, lane = %id, "lane target not found");
            self.unreachable(id, to);
            return;
        };
        let msgid = self.next_msgid();
        let budget = self.conf.peer_budget.get();
        let max_bytes = self.conf.max_message_size.get();
        let link = self
            .links
            .entry((to, id))
            .or_insert_with(|| Link::new(addr.clone()));
        // Small messages count against the budget only, so a lane full of large
        // replies still takes requests, and small messages alone never hold more
        // than the maximum size.
        let small = bytes.len() <= max_bytes / budget;
        if link.outbox.len() >= budget || (!small && link.outbox.bytes() >= max_bytes) {
            debug!(name = %self.conf.name, node = %self.key, peer = %to, lane = %id, "lane full, message dropped");
            return;
        }
        link.outbox
            .enqueue(Slot::MIN, msgid, (RetryPolicy::NoRetry, bytes));
        if !matches!(link.state, LinkState::Idle) {
            return;
        }
        link.state = LinkState::Dialing;
        self.dial(to, id, addr.clone());
    }

    /// Take over a lane connection the party opened.
    pub(super) fn accept(&mut self, key: PublicKey, id: LaneId, addr: &NetAddr, conn: Connection) {
        self.links
            .entry((key, id))
            .or_insert_with(|| Link::new(addr.clone()));
        self.connected(key, id, conn, Origin::Accepted);
    }

    /// Close every lane of a party that left.
    pub(super) fn remove(&mut self, key: &PublicKey) {
        self.links.retain(|(k, _), _| k != key);
        self.connect_tasks.abort_matching(|(k, _)| k == key);
        self.peer_tasks.abort_matching(|(k, _)| k == key);
    }

    /// Move a party's lanes to its new address.
    pub(super) fn readdress(&mut self, key: &PublicKey, addr: &NetAddr) {
        for (&(party, id), link) in self.links.iter_mut().filter(|((k, _), _)| k == key) {
            link.addr = addr.clone();
            match &link.state {
                LinkState::Connected(cancel) => cancel.cancel(),
                LinkState::Dialing => {
                    // Replaces the dial to the old address, which `dial` would keep.
                    let dial = Connection::connect_lane(self.conf.clone(), party, addr.clone(), id);
                    self.connect_tasks.spawn((party, id), dial);
                },
                LinkState::Idle | LinkState::Replacing(_) => {},
            }
        }
    }

    /// The next lane task to finish, or `None` when no task runs.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel safe.
    pub(super) async fn next(&mut self) -> Option<Event> {
        select! {
            Some((key, result)) = self.connect_tasks.join_next() => Some(Event::Dialed(key, result)),
            Some((key, result)) = self.peer_tasks.join_next() => Some(Event::Ended(key, result)),
            else => None,
        }
    }

    pub(super) fn handle(&mut self, event: Event) {
        match event {
            Event::Dialed((key, id), Ok(Some(conn))) => {
                self.connected(key, id, conn, Origin::Dialed)
            },
            Event::Dialed((key, id), Ok(None)) => self.dial_failed(key, id),
            Event::Dialed((key, id), Err(err)) => {
                if err.is_panic() {
                    error!(name = %self.conf.name, node = %self.key, peer = %key, lane = %id, %err, "lane connect task panic");
                }
                self.dial_failed(key, id)
            },
            Event::Ended((key, id), Ok(peer)) => self.ended(key, id, peer),
            Event::Ended((key, id), Err(err)) => {
                if err.is_panic() {
                    error!(name = %self.conf.name, node = %self.key, peer = %key, lane = %id, %err, "lane peer task panic");
                }
                self.disconnected(key, id)
            },
        }
    }

    fn connected(&mut self, key: PublicKey, id: LaneId, conn: Connection, origin: Origin) {
        let inbound = self.delivery(id).messages.clone();
        let Some(link) = self.links.get_mut(&(key, id)) else {
            return;
        };
        let peer = match mem::replace(&mut link.state, LinkState::Idle) {
            LinkState::Idle | LinkState::Dialing => Peer::builder()
                .config(self.conf.clone())
                .budget(self.conf.peer_budget)
                .inbound(inbound)
                .messages(link.outbox.clone())
                .retry(DelayQueue::new(self.conf.clone()))
                .metrics(Arc::new(LaneMetrics::new(self.metrics.clone(), id)))
                .build(),
            LinkState::Connected(cancel) => {
                // Both sides apply the same rule, so they keep the same connection.
                let replace = match origin {
                    Origin::Accepted => key > self.key,
                    Origin::Dialed => key < self.key,
                };
                if !replace {
                    link.state = LinkState::Connected(cancel);
                    return;
                }
                debug!(name = %self.conf.name, node = %self.key, peer = %key, lane = %id, "replacing lane connection");
                cancel.cancel();
                link.state = LinkState::Replacing(Box::new(conn));
                // What was sent on the replaced connection may never be read.
                self.unreachable(id, key);
                return;
            },
            LinkState::Replacing(_) => {
                link.state = LinkState::Replacing(Box::new(conn));
                return;
            },
        };
        let cancel = CancellationToken::new();
        link.state = LinkState::Connected(cancel.clone());
        self.spawn_peer(key, id, peer, conn, cancel);
    }

    fn dial_failed(&mut self, key: PublicKey, id: LaneId) {
        let Some(link) = self.links.get_mut(&(key, id)) else {
            return;
        };
        // Otherwise the party opened the lane meanwhile, or nothing waits.
        if !matches!(link.state, LinkState::Dialing) {
            return;
        }
        link.state = LinkState::Idle;
        link.outbox.clear();
        debug!(name = %self.conf.name, node = %self.key, peer = %key, lane = %id, "lane unavailable");
        self.unreachable(id, key);
    }

    fn ended(&mut self, key: PublicKey, id: LaneId, peer: Peer) {
        let Some(link) = self.links.get_mut(&(key, id)) else {
            return;
        };
        if let LinkState::Replacing(conn) = mem::replace(&mut link.state, LinkState::Idle) {
            let cancel = CancellationToken::new();
            link.state = LinkState::Connected(cancel.clone());
            self.spawn_peer(key, id, peer, *conn, cancel);
        } else {
            self.disconnected(key, id);
        }
    }

    /// Report a link's lost connection, and leave the link idle or dial it
    /// again for the messages still waiting on it.
    fn disconnected(&mut self, key: PublicKey, id: LaneId) {
        let Some(link) = self.links.get_mut(&(key, id)) else {
            return;
        };
        let redial = (link.outbox.len() > 0).then(|| link.addr.clone());
        link.state = if redial.is_some() {
            LinkState::Dialing
        } else {
            LinkState::Idle
        };
        if let Some(addr) = redial {
            self.dial(key, id, addr);
        }
        // What was sent on the lost connection may never be read.
        self.unreachable(id, key);
    }

    /// Dial a link, unless a dial for it still runs: that dial may already be
    /// the connection the party accepted and writes on.
    fn dial(&mut self, key: PublicKey, id: LaneId, addr: NetAddr) {
        if self.connect_tasks.contains_key(&(key, id)) {
            return;
        }
        let dial = Connection::connect_lane(self.conf.clone(), key, addr, id);
        self.connect_tasks.spawn((key, id), dial);
    }

    fn spawn_peer(
        &mut self,
        key: PublicKey,
        id: LaneId,
        mut peer: Peer,
        conn: Connection,
        cancel: CancellationToken,
    ) {
        debug!(
            name = %self.conf.name,
            node = %self.key,
            peer = %key,
            addr = %conn.addr,
            lane = %id,
            "spawning lane peer task"
        );
        let name = self.conf.name.clone();
        let node = self.key;
        let addr = conn.addr;
        self.peer_tasks.spawn((key, id), async move {
            let Err(err) = peer.start(conn, cancel).await;
            if !matches!(err, NetworkError::PeerInterrupt) {
                debug!(%name, %node, peer = %key, %addr, lane = %id, %err, "lane peer ended");
            }
            peer
        });
    }

    fn unreachable(&self, id: LaneId, key: PublicKey) {
        if self.delivery(id).unreachable.send(key).is_err() {
            debug!(name = %self.conf.name, node = %self.key, lane = %id, "lane channel closed");
        }
    }

    fn delivery(&self, id: LaneId) -> &LaneDelivery {
        self.deliveries
            .get(&id)
            .expect("only configured lanes reach the server")
    }

    fn next_msgid(&mut self) -> MsgId {
        let current = self.msgid;
        self.msgid = MsgId::new(self.msgid.0.wrapping_add(1));
        current
    }
}

impl Link {
    fn new(addr: NetAddr) -> Self {
        Self {
            addr,
            outbox: Queue::new(),
            state: LinkState::Idle,
        }
    }
}
