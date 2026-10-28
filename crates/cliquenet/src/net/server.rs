use std::{
    collections::{HashMap, VecDeque},
    mem,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use bytes::{Bytes, BytesMut};
use tokio::{
    net::{TcpListener, TcpStream},
    select, spawn,
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        watch,
    },
    task::{JoinHandle, JoinSet},
};
use tokio_util::{sync::CancellationToken, task::JoinMap};
use tracing::{debug, error, info, trace, warn};

use crate::{
    Config, Lane, Metrics, NetAddr, PublicKey, Role,
    connection::Connection,
    delay::DelayQueue,
    error::NetworkError,
    metrics::QueryMetrics,
    msg::{MsgId, Slot, Trailer, hello::Hello},
    net::{Command, PeerCommand, PeerMessage, RetryPolicy, SendAction, peer::Peer},
    queue::Queue,
    util::until,
};

pub struct Server {
    key: PublicKey,
    conf: Arc<Config>,
    role: Role,
    msgid: MsgId,
    lower_bound: Slot,
    parties: HashMap<PublicKey, Party>,
    ibound: UnboundedSender<PeerMessage>,
    query_ibound: UnboundedSender<PeerMessage>,
    obound: UnboundedReceiver<Command>,
    next_slot: watch::Receiver<Slot>,
    accept_tasks: JoinSet<Result<Connection, NetworkError>>,
    /// Keyed by a per-connection id because the lane is only known once the hello is read.
    #[allow(clippy::type_complexity)]
    hello_tasks:
        JoinMap<(PublicKey, u64), Result<(Hello, Connection, Option<Hello>), NetworkError>>,
    next_hello_id: u64,
    connect_tasks: JoinMap<(PublicKey, Lane), Option<Connection>>,
    peer_tasks: JoinMap<(PublicKey, Lane), Peer>,
    accept_query: bool,
    /// When each lower bound arrived. Query links are collected by age, not
    /// by view, since a query message is useful for as long as its sender
    /// waits for the reply.
    bounds: VecDeque<(Instant, Slot)>,
    metrics: Arc<dyn Metrics>,
}

struct Party {
    role: Role,
    addr: NetAddr,
    consensus: Link,
    query: Link,
}

/// The state of one connection to a party.
struct Link {
    outbox: Queue<(RetryPolicy, Bytes)>,
    retry: DelayQueue,
    peer: PeerState,
}

/// Which side opened a connection. This decides which one is kept when both
/// sides connect at once.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Accepted,
    Connected,
}

/// The states of a peer.
///
/// When a party is created there exists no peer yet. After a connection has
/// been accepted or a connect attempt succeeded, a peer task is created and
/// the state transitions to `Connected`. The cancellation token is used if
/// the peer task should stop in order to replace the peer's connection. When
/// cancelled the next connection is stored in `Replace` and once the peer
/// task finishes and returns the peer, its connection is set and a new peer
/// task (with the same peer object) is spawned and the state transitions to
/// `Connected` again.
///
/// The `Reconnect` state is entered if the peer itself errors and its task
/// finishes. We keep the peer here until we have a new connection to resume.
/// Once a connect or accept task finishes we set the connection and spawn
/// a new peer task again.
enum PeerState {
    /// Initial state.
    None,
    /// A peer task is running.
    ///
    /// The cancellation token can be used to interrupt the peer. The task
    /// will end and the peer object is returned.
    Connected(CancellationToken),
    /// A peer has errored and wants a fresh connection.
    ///
    /// We store the peer here until a new connection is available.
    Reconnect(Peer),
    /// The server wants to replace a peer's connection.
    ///
    /// Only entered from `Connected` after the server has cancelled the
    /// peer task. While waiting for the task to return the peer object
    /// we park the connection here.
    Replace(Connection),
}

impl Server {
    #[expect(clippy::too_many_arguments)]
    pub(super) fn spawn(
        conf: Arc<Config>,
        listener: TcpListener,
        role: Role,
        tx: UnboundedSender<PeerMessage>,
        query_tx: UnboundedSender<PeerMessage>,
        rx: UnboundedReceiver<Command>,
        sx: watch::Receiver<Slot>,
        metrics: Arc<dyn Metrics>,
    ) -> JoinHandle<()> {
        let our_key = conf.keypair.public_key();
        let parties = conf
            .parties
            .iter()
            .filter(|&(k, _)| *k != our_key)
            .map(|(k, a)| {
                let p = Party::new(conf.clone(), Role::Active, a.clone());
                (*k, p)
            })
            .collect();

        let this = Self {
            key: our_key,
            conf,
            role,
            ibound: tx,
            query_ibound: query_tx,
            obound: rx,
            parties,
            accept_tasks: JoinSet::new(),
            connect_tasks: JoinMap::new(),
            hello_tasks: JoinMap::new(),
            next_hello_id: 0,
            peer_tasks: JoinMap::new(),
            accept_query: false,
            bounds: VecDeque::new(),
            msgid: MsgId::new(0),
            next_slot: sx,
            lower_bound: Slot::MIN,
            metrics,
        };

        spawn(this.run(listener))
    }

    async fn run(mut self, listener: TcpListener) {
        // Connect to all peers. Query links connect on first use.
        for (k, a) in self
            .parties
            .iter()
            .map(|(k, p)| (*k, p.addr.clone()))
            .collect::<Vec<_>>()
        {
            self.spawn_connect(k, Lane::Consensus, a)
        }

        loop {
            select! {
                x = listener.accept(), if self.accept_tasks.len() < self.conf.max_accept_tasks => {
                    match x {
                        Ok((stream, addr)) => {
                            debug!(
                                name = %self.conf.name,
                                node = %self.key,
                                %addr,
                                "accepted new tcp connection"
                            );
                            self.spawn_accept(stream)
                        }
                        Err(err) => {
                            warn!(
                                name = %self.conf.name,
                                node = %self.key,
                                %err,
                                "error accepting tcp connection"
                            )
                        }
                    }
                }

                Some(h) = self.accept_tasks.join_next() => match h {
                    Ok(Ok(conn)) => {
                        self.metrics.set(&self.key, ACCEPT_TASKS, self.accept_tasks.len());
                        if conn.key == self.key {
                            warn!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %conn.key,
                                addr = %conn.addr,
                                "rejecting connection with the same key"
                            );
                            self.spawn_hello(conn, Hello::BackOff(Duration::MAX));
                            continue
                        }
                        let Some(party) = self.parties.get_mut(&conn.key) else {
                            info!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %conn.key,
                                addr = %conn.addr,
                                "unknown party"
                            );
                            self.spawn_hello(conn, Hello::BackOff(self.conf.backoff_duration));
                            continue
                        };
                        if party.ip_addr_mismatch(conn.addr.ip()) {
                            warn!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %conn.key,
                                addr = %conn.addr,
                                "party has invalid ip addr"
                            );
                            self.spawn_hello(conn, Hello::BackOff(self.conf.backoff_duration));
                            continue
                        }
                        self.spawn_hello(conn, Hello::Ok);
                    }
                    Ok(Err(err)) => {
                        self.metrics.set(&self.key, ACCEPT_TASKS, self.accept_tasks.len());
                        warn!(name = %self.conf.name, node = %self.key, %err, "handshake failed")
                    }
                    Err(err) => {
                        self.metrics.set(&self.key, ACCEPT_TASKS, self.accept_tasks.len());
                        if err.is_panic() {
                            error!(
                                name = %self.conf.name,
                                node = %self.key,
                                %err,
                                "handshake task panic"
                            )
                        }
                    }
                },

                Some(r) = self.hello_tasks.join_next() => match r {
                    ((key, _), Ok(Ok((our_hello, conn, their_hello)))) => {
                        self.metrics.set(&self.key, HELLO_TASKS, self.hello_tasks.len());
                        if key == self.key {
                            // This case has been addressed already by rejecting the peer,
                            // i.e. we told the peer to backoff forever.
                            continue
                        }
                        if !self.parties.contains_key(&key) {
                            info!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %key,
                                addr = %conn.addr,
                                "unknown party"
                            );
                            continue
                        }
                        if their_hello == Some(Hello::OkQuery) && our_hello != Hello::OkQuery {
                            debug!(name = %self.conf.name, node = %self.key, peer = %key, "refused query lane");
                            continue
                        }
                        let lane = match our_hello.lane() {
                            Some(lane) if their_hello == Some(our_hello) => lane,
                            Some(_) | None => {
                                warn!(
                                    name   = %self.conf.name,
                                    node   = %self.key,
                                    peer   = %key,
                                    addr   = %conn.addr,
                                    ours   = ?our_hello,
                                    theirs = ?their_hello,
                                    "hello failed"
                                );
                                continue
                            }
                        };
                        self.on_connection(key, lane, conn, Origin::Accepted);
                    }
                    ((key, _), Ok(Err(err))) => {
                        self.metrics.set(&self.key, HELLO_TASKS, self.hello_tasks.len());
                        warn!(
                            name = %self.conf.name,
                            node = %self.key,
                            peer = %key,
                            %err,
                            "hello task error"
                        )
                    }
                    ((key, _), Err(err)) => {
                        self.metrics.set(&self.key, HELLO_TASKS, self.hello_tasks.len());
                        if err.is_panic() {
                            error!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %key,
                                %err,
                                "hello task panic"
                            )
                        }
                    }
                },

                Some(x) = self.connect_tasks.join_next() => match x {
                    ((key, lane), Ok(Some(conn))) => {
                        self.metrics.set(&self.key, CONNECT_TASKS, self.connect_tasks.len());
                        self.on_connection(key, lane, conn, Origin::Connected);
                    }
                    ((key, lane), Ok(None)) => {
                        self.metrics.set(&self.key, CONNECT_TASKS, self.connect_tasks.len());
                        debug!(name = %self.conf.name, node = %self.key, peer = %key, ?lane, "lane unavailable");
                        if let Some(party) = self.parties.get_mut(&key) {
                            party.link_mut(lane).outbox.clear();
                        }
                        // The empty message tells the application nothing queued for the party was sent.
                        if self.query_ibound.send((key, Bytes::new(), None)).is_err() {
                            debug!(name = %self.conf.name, node = %self.key, "query channel closed");
                        }
                    }
                    ((key, lane), Err(err)) => {
                        self.metrics.set(&self.key, CONNECT_TASKS, self.connect_tasks.len());
                        if err.is_panic() {
                            error!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %key,
                                ?lane,
                                %err,
                                "connect task panic"
                            );
                            if let Some(party) = self.parties.get_mut(&key)
                                && matches!(party.link_mut(lane).peer, PeerState::None | PeerState::Reconnect(_))
                            {
                                let addr = party.addr.clone();
                                self.spawn_connect(key, lane, addr);
                            }
                        }
                    }
                },

                Some(p) = self.peer_tasks.join_next() => match p {
                    ((key, lane), Ok(peer)) => {
                        self.metrics.set(&self.key, PEER_TASKS, self.peer_tasks.len());
                        if self.ibound.is_closed() {
                            return
                        }
                        let Some(party) = self.parties.get_mut(&key) else {
                            debug!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %key,
                                "party has been removed"
                            );
                            continue
                        };
                        let addr = party.addr.clone();
                        let link = party.link_mut(lane);
                        match (lane, link.peer.take()) {
                            (_, PeerState::Replace(conn)) => {
                                let cancel = CancellationToken::new();
                                link.peer = PeerState::Connected(cancel.clone());
                                self.spawn_peer(key, lane, peer, conn, cancel);
                            }
                            (Lane::Consensus, PeerState::None | PeerState::Connected(_) | PeerState::Reconnect(_)) => {
                                link.peer = PeerState::Reconnect(peer);
                                self.spawn_connect(key, lane, addr);
                            }
                            // A query link is reopened by the next send, so nothing is kept.
                            (Lane::Query, PeerState::None | PeerState::Connected(_) | PeerState::Reconnect(_)) => {}
                        }
                    }
                    ((key, lane), Err(err)) => {
                        self.metrics.set(&self.key, PEER_TASKS, self.peer_tasks.len());
                        if err.is_panic() {
                            error!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %key,
                                ?lane,
                                %err,
                                "peer task panic"
                            );
                            if self.ibound.is_closed() {
                                return
                            }
                            if let Some(party) = self.parties.get_mut(&key) {
                                let addr = party.addr.clone();
                                party.link_mut(lane).peer = PeerState::None;
                                self.spawn_connect(key, lane, addr);
                            }
                        }
                    }
                },

                r = self.next_slot.changed() => {
                    if r.is_err() {
                        return
                    }
                    let s = *self.next_slot.borrow_and_update();
                    debug_assert!(s > self.lower_bound); // ensured by `NetworkSender::gc`
                    self.lower_bound = s;
                    self.metrics.set(&self.key, LOWER_BOUND, u64::from(s) as usize);
                    self.bounds.push_back((Instant::now(), s));
                    // Everything tagged below a bound was queued before that bound arrived.
                    let aged = self
                        .bounds
                        .iter()
                        .rposition(|(at, _)| at.elapsed() >= self.conf.receive_timeout)
                        .map(|i| {
                            let (_, bound) = self.bounds[i];
                            self.bounds.drain(..=i);
                            bound
                        });
                    for party in self.parties.values() {
                        party.consensus.outbox.gc(s);
                        party.consensus.retry.gc(s);
                        if let Some(bound) = aged {
                            party.query.outbox.gc(bound);
                            party.query.retry.gc(bound);
                        }
                    }
                }

                cmd = self.obound.recv() => {
                    self.metrics.set(&self.key, CHANNEL_SIZE, self.obound.len());
                    match cmd {
                        Some(Command::Peer(PeerCommand::Add(role, parties))) => {
                            for (k, a) in parties {
                                if k == self.key {
                                    self.role = role;
                                    continue
                                }
                                if let Some(p) = self.parties.get_mut(&k) {
                                    if p.addr == a {
                                        p.role = role;
                                    } else {
                                        info!(
                                            name = %self.conf.name,
                                            node = %self.key,
                                            peer = %k,
                                            addr = %a,
                                            "updating party address"
                                        );
                                        p.addr = a.clone();
                                        p.role = role;
                                        self.connect_tasks.abort_matching(|(key, _)| *key == k);
                                        let redial_query = match &p.query.peer {
                                            PeerState::Connected(cancel) => {
                                                cancel.cancel();
                                                false
                                            }
                                            PeerState::None | PeerState::Reconnect(_) | PeerState::Replace(_) => {
                                                p.query.outbox.len() > 0
                                            }
                                        };
                                        if let PeerState::Connected(cancel) = &p.consensus.peer {
                                            cancel.cancel()
                                        } else {
                                            self.spawn_connect(k, Lane::Consensus, a.clone())
                                        }
                                        if redial_query {
                                            self.spawn_connect(k, Lane::Query, a)
                                        }
                                    }
                                    continue
                                }
                                info!(
                                    name = %self.conf.name,
                                    node = %self.key,
                                    peer = %k,
                                    addr = %a,
                                    "adding new peer"
                                );
                                self.parties.insert(k, Party::new(self.conf.clone(), role, a.clone()));
                                self.spawn_connect(k, Lane::Consensus, a)
                            }
                        }
                        Some(Command::Peer(PeerCommand::Remove(peers))) => {
                            for k in &peers {
                                if *k == self.key {
                                    info!(
                                        name = %self.conf.name,
                                        node = %self.key,
                                        "removing self sets role to passive"
                                    );
                                    self.role = Role::Passive;
                                    continue
                                }
                                info!(
                                    name = %self.conf.name,
                                    node = %self.key,
                                    peer = %k,
                                    "removing peer"
                                );
                                self.parties.remove(k);
                                self.connect_tasks.abort_matching(|(key, _)| key == k);
                                self.peer_tasks.abort_matching(|(key, _)| key == k);
                            }
                        }
                        Some(Command::Peer(PeerCommand::Assign(role, peers))) => {
                            for k in &peers {
                                if *k == self.key {
                                    self.role = role;
                                    continue
                                }
                                if let Some(p) = self.parties.get_mut(k) {
                                    info!(
                                        name = %self.conf.name,
                                        node = %self.key,
                                        peer = %k,
                                        %role,
                                        "assigning role to peer"
                                    );
                                    p.role = role
                                } else {
                                    warn!(
                                        name = %self.conf.name,
                                        node = %self.key,
                                        peer = %k,
                                        role = %role,
                                        "peer to assign role to not found"
                                    );
                                }
                            }
                        }
                        Some(Command::Send(cmd)) => match cmd.action {
                            SendAction::Unicast(to, m) => {
                                if cmd.slot < self.lower_bound {
                                    continue
                                }

                                if to == self.key {
                                    trace!(name = %self.conf.name, node = %self.key, "sending message");
                                    if let Err(err) = self.ibound.send((self.key, m.into(), None)) {
                                        warn!(
                                            name = %self.conf.name,
                                            node = %self.key,
                                            err  = %err,
                                            "channel closed"
                                        );
                                        return
                                    }
                                    trace!(name = %self.conf.name, node = %self.key, "message delivered");
                                    continue
                                }

                                let msgid = self.next_msgid();
                                let bytes = append_trailer(cmd.retry, cmd.slot, msgid, m);

                                if let Some(party) = self.parties.get(&to) {
                                    party.consensus.outbox.enqueue(cmd.slot, msgid, (cmd.retry, bytes));
                                } else {
                                    warn!(
                                        name = %self.conf.name,
                                        node = %self.key,
                                        peer = %to,
                                        "unicast target not found"
                                    );
                                }
                            }
                            SendAction::Multicast(parties, m) => {
                                if cmd.slot < self.lower_bound {
                                    continue
                                }

                                let msgid = self.next_msgid();
                                let bytes = append_trailer(cmd.retry, cmd.slot, msgid, m);

                                if parties.contains(&self.key) {
                                    let bytes = remove_trailer(bytes.clone());
                                    trace!(name = %self.conf.name, node = %self.key, "sending message");
                                    if let Err(err) = self.ibound.send((self.key, bytes, None)) {
                                        warn!(
                                            name = %self.conf.name,
                                            node = %self.key,
                                            err  = %err,
                                            "channel closed"
                                        );
                                        return
                                    }
                                    trace!(name = %self.conf.name, node = %self.key, "message delivered");
                                }

                                for (to, party) in &self.parties {
                                    if !parties.contains(to) {
                                        continue
                                    }
                                    trace!(name = %self.conf.name, node = %self.key, %to, "sending message");
                                    party.consensus.outbox.enqueue(cmd.slot, msgid, (cmd.retry, bytes.clone()));
                                }
                            }
                            SendAction::Broadcast(m) => {
                                if cmd.slot < self.lower_bound {
                                    continue
                                }

                                let msgid = self.next_msgid();
                                let bytes = append_trailer(cmd.retry, cmd.slot, msgid, m);

                                if self.role.is_active() {
                                    let bytes = remove_trailer(bytes.clone());
                                    trace!(name = %self.conf.name, node = %self.key, "sending message");
                                    if let Err(err) = self.ibound.send((self.key, bytes, None)) {
                                        warn!(
                                            name = %self.conf.name,
                                            node = %self.key,
                                            err  = %err,
                                            "channel closed"
                                        );
                                        return
                                    }
                                    trace!(name = %self.conf.name, node = %self.key, "message delivered");
                                }
                                for (key, party) in &self.parties {
                                    if party.role.is_active() {
                                        trace!(
                                            name  = %self.conf.name,
                                            node  = %self.key,
                                            to    = %key,
                                            "sending message"
                                        );
                                        party.consensus.outbox.enqueue(cmd.slot, msgid, (cmd.retry, bytes.clone()));
                                    }
                                }
                            }
                        }
                        Some(Command::SendQuery(to, bytes)) => self.send_query(to, bytes),
                        Some(Command::AcceptQuery) => self.accept_query = true,
                        Some(Command::Shutdown(tx)) => {
                            debug!(name = %self.conf.name, node = %self.key, "shutting down");
                            let _ = tx.send(());
                            return
                        }
                        None => return
                    }
                }
            }
        }
    }

    /// Run a newly opened connection on its link, or keep the existing one
    /// when both sides connected at once.
    fn on_connection(&mut self, key: PublicKey, lane: Lane, conn: Connection, origin: Origin) {
        let Some(party) = self.parties.get_mut(&key) else {
            debug!(
                name = %self.conf.name,
                node = %self.key,
                peer = %key,
                addr = %conn.addr,
                "party has been removed"
            );
            return;
        };
        let link = party.link_mut(lane);
        let peer = match link.peer.take() {
            PeerState::None => {
                let (inbound, metrics) = match lane {
                    Lane::Consensus => (self.ibound.clone(), Arc::clone(&self.metrics)),
                    Lane::Query => (
                        self.query_ibound.clone(),
                        Arc::new(QueryMetrics(Arc::clone(&self.metrics))) as Arc<dyn Metrics>,
                    ),
                };
                Peer::builder()
                    .config(self.conf.clone())
                    .budget(self.conf.peer_budget)
                    .inbound(inbound)
                    .messages(link.outbox.clone())
                    .retry(link.retry.clone())
                    .metrics(metrics)
                    .build()
            },
            PeerState::Reconnect(peer) => peer,
            PeerState::Connected(cancel) => {
                // Both sides apply the same rule, so they keep the same connection.
                let replace = match origin {
                    Origin::Accepted => key > self.key,
                    Origin::Connected => key < self.key,
                };
                if replace {
                    info!(
                        name = %self.conf.name,
                        node = %self.key,
                        peer = %key,
                        addr = %conn.addr,
                        ?lane,
                        "replacing connection"
                    );
                    cancel.cancel();
                    link.peer = PeerState::Replace(conn);
                } else {
                    link.peer = PeerState::Connected(cancel);
                }
                return;
            },
            PeerState::Replace(_) => {
                link.peer = PeerState::Replace(conn);
                return;
            },
        };
        if origin == Origin::Accepted {
            self.connect_tasks.abort(&(key, lane));
        }
        let cancel = CancellationToken::new();
        link.peer = PeerState::Connected(cancel.clone());
        self.spawn_peer(key, lane, peer, conn, cancel);
    }

    /// Queue a query message for a party, opening its query link if needed.
    fn send_query(&mut self, to: PublicKey, bytes: Bytes) {
        if to == self.key {
            if self
                .query_ibound
                .send((self.key, remove_trailer(bytes), None))
                .is_err()
            {
                warn!(name = %self.conf.name, node = %self.key, "query channel closed");
            }
            return;
        }
        let msgid = self.next_msgid();
        let Some(party) = self.parties.get(&to) else {
            warn!(
                name = %self.conf.name,
                node = %self.key,
                peer = %to,
                "query target not found"
            );
            return;
        };
        // The same budget the peer applies to its own messages in flight, so one party cannot
        // pile up replies in memory.
        if party.query.outbox.len() >= self.conf.peer_budget.get() {
            debug!(name = %self.conf.name, node = %self.key, peer = %to, "query outbox full");
            return;
        }
        // The tag is the queueing time as the age based GC of query links reads it.
        party
            .query
            .outbox
            .enqueue(self.lower_bound, msgid, (RetryPolicy::NoRetry, bytes));
        let dial = match party.query.peer {
            PeerState::None | PeerState::Reconnect(_) => {
                !self.connect_tasks.contains_key(&(to, Lane::Query))
            },
            PeerState::Connected(_) | PeerState::Replace(_) => false,
        };
        if dial {
            let addr = party.addr.clone();
            self.spawn_connect(to, Lane::Query, addr);
        }
    }

    fn spawn_connect(&mut self, key: PublicKey, lane: Lane, addr: NetAddr) {
        if self.key == key {
            return;
        }
        debug!(
            name = %self.conf.name,
            node = %self.key,
            peer = %key,
            addr = %addr,
            ?lane,
            "spawning connect task"
        );
        let conn = Connection::connect(self.conf.clone(), key, addr, lane);
        self.connect_tasks.spawn((key, lane), conn);
        self.metrics.add(&key, CONNECT_ATTEMPTS, 1);
        self.metrics
            .set(&self.key, CONNECT_TASKS, self.connect_tasks.len());
    }

    fn spawn_accept(&mut self, stream: TcpStream) {
        debug!(name = %self.conf.name, node = %self.key, "spawning accept task");
        let conn = Connection::accept(self.conf.clone(), stream);
        self.accept_tasks.spawn(conn);
        self.metrics
            .set(&self.key, ACCEPT_TASKS, self.accept_tasks.len());
    }

    fn spawn_hello(&mut self, mut conn: Connection, ours: Hello) {
        debug!(
            name = %self.conf.name,
            node = %self.key,
            peer = %conn.key,
            addr = %conn.addr,
            "spawning hello task"
        );

        self.metrics.add(&conn.key, HELLOS, 1);

        // Under normal circumstances we should never witness more hellos than parties.
        // An adversary may nevertheless pass the accept stage and since we do not
        // know the party we inform it to back off. If the remote does not send us
        // their hello in due time this will occupy the hello task (and a file
        // descriptor). To put a cap on that we make the reading of hellos contingent
        // on how many ongoing hellos we already process. Under attack we do not
        // bother with reading the remote's (unless it is a peer we want), but only
        // send our hello and drop the connection. NB that this may cause the remote
        // to see a connection reset.
        //
        // Each party may open one connection per lane, hence two hellos per party.
        let under_pressure = self.hello_tasks.len() > 4 * self.parties.len();

        // A third pending hello of a party replaces its oldest, as a party reconnecting a lane
        // would want, and caps what one party can hold open.
        let mut pending = self
            .hello_tasks
            .keys()
            .filter(|(key, _)| *key == conn.key)
            .map(|(_, id)| *id)
            .collect::<Vec<_>>();
        if pending.len() >= 2 {
            pending.sort_unstable();
            self.hello_tasks.abort(&(conn.key, pending[0]));
        }

        let id = self.next_hello_id;
        self.next_hello_id = self.next_hello_id.wrapping_add(1);
        let accept_query = self.accept_query;
        let backoff = self.conf.backoff_duration;
        self.hello_tasks.spawn(
            (conn.key, id),
            until(self.conf.handshake_timeout, async move {
                let theirs = if ours == Hello::Ok || !under_pressure {
                    Some(conn.recv_hello().await?)
                } else {
                    None
                };
                // An accepted party opens the lane it asked for.
                let ours = match (ours, theirs) {
                    (Hello::Ok, Some(Hello::OkQuery)) if accept_query => Hello::OkQuery,
                    (Hello::Ok, Some(Hello::OkQuery)) => Hello::BackOff(backoff),
                    (ours, _) => ours,
                };
                conn.send_hello(ours).await?;
                Ok::<_, NetworkError>((ours, conn, theirs))
            }),
        );

        self.metrics
            .set(&self.key, HELLO_TASKS, self.hello_tasks.len());
    }

    fn spawn_peer(
        &mut self,
        key: PublicKey,
        lane: Lane,
        mut peer: Peer,
        conn: Connection,
        cancel: CancellationToken,
    ) {
        debug!(
            name = %self.conf.name,
            node = %self.key,
            peer = %key,
            addr = %conn.addr,
            ?lane,
            "spawning peer task"
        );
        let node = self.key;
        let name = self.conf.name.clone();
        let metrics = self.metrics.clone();
        let addr = conn.addr;
        self.peer_tasks.spawn((key, lane), async move {
            let Err(err) = peer.start(conn, cancel).await;
            match (lane, err) {
                (_, NetworkError::PeerInterrupt) => {},
                (Lane::Consensus, err) => {
                    warn!(%name, %node, peer = %key, %addr, %err, "peer failure");
                    metrics.add(&key, ERRORS, 1)
                },
                // A query link closing is routine, unlike a consensus link.
                (Lane::Query, err) => {
                    debug!(%name, %node, peer = %key, %addr, %err, "query peer ended")
                },
            }
            peer
        });
        self.metrics
            .set(&self.key, PEER_TASKS, self.peer_tasks.len());
    }

    fn next_msgid(&mut self) -> MsgId {
        let current = self.msgid;
        self.msgid = MsgId::new(self.msgid.0.wrapping_add(1));
        current
    }
}

impl Party {
    fn new(c: Arc<Config>, r: Role, a: NetAddr) -> Self {
        Self {
            addr: a,
            role: r,
            consensus: Link::new(c.clone()),
            query: Link::new(c),
        }
    }

    fn link_mut(&mut self, lane: Lane) -> &mut Link {
        match lane {
            Lane::Consensus => &mut self.consensus,
            Lane::Query => &mut self.query,
        }
    }

    fn ip_addr_mismatch(&self, addr: IpAddr) -> bool {
        let NetAddr::Inet(ip, _) = &self.addr else {
            return false;
        };
        ip.to_canonical() != addr.to_canonical()
    }
}

impl Link {
    fn new(c: Arc<Config>) -> Link {
        Link {
            outbox: Queue::new(),
            retry: DelayQueue::new(c),
            peer: PeerState::None,
        }
    }
}

impl PeerState {
    fn take(&mut self) -> Self {
        mem::replace(self, Self::None)
    }
}

fn append_trailer(pol: RetryPolicy, slot: Slot, id: MsgId, bytes: Vec<u8>) -> Bytes {
    let t = match pol {
        RetryPolicy::Default => Trailer::Std { slot, id },
        RetryPolicy::NoRetry => Trailer::NoAck { slot },
    };
    let mut msg = BytesMut::from(Bytes::from(bytes));
    msg.extend_from_slice(t.to_bytes().as_ref());
    msg.freeze()
}

fn remove_trailer(mut bytes: Bytes) -> Bytes {
    let _t = Trailer::from_bytes(&mut bytes);
    debug_assert!(_t.is_some());
    bytes
}

// Metrics labels /////////////////////////////////////////////////////////////

/// Current number of accept tasks.
const ACCEPT_TASKS: &str = "accept_tasks";

/// Current number of channel items.
const CHANNEL_SIZE: &str = "channel_size";

/// Total number of connect attempts.
const CONNECT_ATTEMPTS: &str = "connect_attempts";

/// Current number of connect tasks.
const CONNECT_TASKS: &str = "connect_tasks";

/// Total number of peer errors.
const ERRORS: &str = "errors";

/// Total number of hello exchanges.
const HELLOS: &str = "hellos";

/// Current number of hello tasks.
const HELLO_TASKS: &str = "hello_tasks";

/// Current GC lower bound.
const LOWER_BOUND: &str = "lower_bound";

/// Current number of peer tasks.
const PEER_TASKS: &str = "peer_tasks";
