use std::{collections::HashMap, mem, net::IpAddr, sync::Arc, time::Duration};

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
    Config, Metrics, PublicKey, Role,
    addr::NetAddr,
    connection::Connection,
    error::NetworkError,
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
    obound: UnboundedReceiver<Command>,
    next_slot: watch::Receiver<Slot>,
    accept_tasks: JoinSet<Result<Connection, NetworkError>>,
    hello_tasks: JoinMap<PublicKey, Result<(Hello, Connection, Hello), NetworkError>>,
    connect_tasks: JoinMap<PublicKey, Connection>,
    peer_tasks: JoinMap<PublicKey, Peer>,
    metrics: Arc<dyn Metrics>,
}

struct Party {
    role: Role,
    addr: NetAddr,
    outbox: Queue<(RetryPolicy, Bytes)>,
    peer: PeerState,
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
    pub(super) fn spawn(
        conf: Arc<Config>,
        listener: TcpListener,
        role: Role,
        tx: UnboundedSender<PeerMessage>,
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
                let p = Party::new(Role::Active, a.clone());
                (*k, p)
            })
            .collect();

        let this = Self {
            key: our_key,
            conf,
            role,
            ibound: tx,
            obound: rx,
            parties,
            accept_tasks: JoinSet::new(),
            connect_tasks: JoinMap::new(),
            hello_tasks: JoinMap::new(),
            peer_tasks: JoinMap::new(),
            msgid: MsgId::new(0),
            next_slot: sx,
            lower_bound: Slot::MIN,
            metrics,
        };

        spawn(this.run(listener))
    }

    async fn run(mut self, listener: TcpListener) {
        // Connect to all peers.
        for (k, a) in self
            .parties
            .iter()
            .map(|(k, p)| (*k, p.addr.clone()))
            .collect::<Vec<_>>()
        {
            self.spawn_connect(k, a)
        }

        loop {
            select! {
                x = listener.accept() => match x {
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
                },

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
                        if !err.is_cancelled() {
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
                    (_, Ok(Ok((our_hello, conn, their_hello)))) => {
                        self.metrics.set(&self.key, HELLO_TASKS, self.hello_tasks.len());
                        if conn.key == self.key {
                            // This case has been addressed already by rejecting the peer,
                            // i.e. we told the peer to backoff forever.
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
                            continue
                        };
                        if !(our_hello.is_ok() && their_hello.is_ok()) {
                            warn!(
                                name   = %self.conf.name,
                                node   = %self.key,
                                peer   = %conn.key,
                                addr   = %conn.addr,
                                ours   = ?our_hello,
                                theirs = ?their_hello,
                                "hello failed"
                            );
                            continue
                        }
                        match party.peer.take() {
                            PeerState::None => {
                                self.connect_tasks.abort(&conn.key);
                                let key = conn.key;
                                let peer = Peer::builder()
                                    .config(self.conf.clone())
                                    .budget(self.conf.peer_budget)
                                    .next_slot(self.next_slot.clone())
                                    .inbound(self.ibound.clone())
                                    .messages(party.outbox.clone())
                                    .connection(conn)
                                    .metrics(self.metrics.clone())
                                    .build();
                                party.peer = PeerState::Connected(peer.cancel_token());
                                self.spawn_peer(key, peer);
                            }
                            PeerState::Reconnect(mut peer) => {
                                self.connect_tasks.abort(&conn.key);
                                let key = conn.key;
                                peer.set_connection(conn);
                                party.peer = PeerState::Connected(peer.cancel_token());
                                self.spawn_peer(key, peer);
                            }
                            PeerState::Connected(cancel) => {
                                if conn.key > self.key {
                                    info!(
                                        name = %self.conf.name,
                                        node = %self.key,
                                        peer = %conn.key,
                                        addr = %conn.addr,
                                        "replacing connection with accepted one"
                                    );
                                    cancel.cancel();
                                    party.peer = PeerState::Replace(conn);
                                } else {
                                    party.peer = PeerState::Connected(cancel);
                                }
                            }
                            PeerState::Replace(_) => {
                                party.peer = PeerState::Replace(conn);
                            }
                        }
                    }
                    (key, Ok(Err(err))) => {
                        self.metrics.set(&self.key, HELLO_TASKS, self.hello_tasks.len());
                        warn!(
                            name = %self.conf.name,
                            node = %self.key,
                            peer = %key,
                            %err,
                            "hello task error"
                        )
                    }
                    (key, Err(err)) => {
                        self.metrics.set(&self.key, HELLO_TASKS, self.hello_tasks.len());
                        if !err.is_cancelled() {
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
                    (_, Ok(conn)) => {
                        self.metrics.set(&self.key, CONNECT_TASKS, self.connect_tasks.len());
                        let Some(party) = self.parties.get_mut(&conn.key) else {
                            debug!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %conn.key,
                                addr = %conn.addr,
                                "party has been removed"
                            );
                            continue
                        };
                        match party.peer.take() {
                            PeerState::None => {
                                let key = conn.key;
                                let peer = Peer::builder()
                                    .config(self.conf.clone())
                                    .budget(self.conf.peer_budget)
                                    .next_slot(self.next_slot.clone())
                                    .inbound(self.ibound.clone())
                                    .messages(party.outbox.clone())
                                    .connection(conn)
                                    .metrics(self.metrics.clone())
                                    .build();
                                party.peer = PeerState::Connected(peer.cancel_token());
                                self.spawn_peer(key, peer);
                            }
                            PeerState::Reconnect(mut peer) => {
                                let key = conn.key;
                                peer.set_connection(conn);
                                party.peer = PeerState::Connected(peer.cancel_token());
                                self.spawn_peer(key, peer);
                            }
                            PeerState::Connected(cancel) => {
                                if conn.key < self.key {
                                    info!(
                                        name = %self.conf.name,
                                        node = %self.key,
                                        peer = %conn.key,
                                        addr = %conn.addr,
                                        "replacing connection with outgoing one"
                                    );
                                    cancel.cancel();
                                    party.peer = PeerState::Replace(conn);
                                } else {
                                    party.peer = PeerState::Connected(cancel);
                                }
                            }
                            PeerState::Replace(_) => {
                                party.peer = PeerState::Replace(conn);
                            }
                        }
                    }
                    (key, Err(err)) => {
                        self.metrics.set(&self.key, CONNECT_TASKS, self.connect_tasks.len());
                        if !err.is_cancelled() {
                            error!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %key,
                                %err,
                                "connect task panic"
                            )
                        }
                    }
                },

                Some(p) = self.peer_tasks.join_next() => match p {
                    (key, Ok(mut peer)) => {
                        self.metrics.set(&self.key, PEER_TASKS, self.peer_tasks.len());
                        if self.ibound.is_closed() {
                            return
                        }
                        let Some(party) = self.parties.get_mut(peer.public_key()) else {
                            debug!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %peer.public_key(),
                                addr = %peer.socket_addr(),
                                "party has been removed"
                            );
                            continue
                        };
                        if let PeerState::Replace(conn) = party.peer.take() {
                            let key = conn.key;
                            peer.set_connection(conn);
                            party.peer = PeerState::Connected(peer.cancel_token());
                            self.spawn_peer(key, peer);
                        } else {
                            let addr = party.addr.clone();
                            party.peer = PeerState::Reconnect(peer);
                            self.spawn_connect(key, addr);
                        }
                    }
                    (key, Err(err)) => {
                        self.metrics.set(&self.key, PEER_TASKS, self.peer_tasks.len());
                        if !err.is_cancelled() {
                            error!(
                                name = %self.conf.name,
                                node = %self.key,
                                peer = %key,
                                %err,
                                "peer task panic"
                            );
                            if self.ibound.is_closed() {
                                return
                            }
                            if let Some(party) = self.parties.get_mut(&key) {
                                let addr = party.addr.clone();
                                party.peer = PeerState::None;
                                self.spawn_connect(key, addr);
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
                    for party in self.parties.values() {
                        party.outbox.gc(s)
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
                                        self.connect_tasks.abort(&k);
                                        if let PeerState::Connected(cancel) = &p.peer {
                                            cancel.cancel()
                                        } else {
                                            self.spawn_connect(k, a)
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
                                self.parties.insert(k, Party::new(role, a.clone()));
                                self.spawn_connect(k, a)
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
                                self.connect_tasks.abort(k);
                                self.peer_tasks.abort(k);
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
                                    party.outbox.enqueue(cmd.slot, msgid, (cmd.retry, bytes));
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
                                    party.outbox.enqueue(cmd.slot, msgid, (cmd.retry, bytes.clone()));
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
                                        party.outbox.enqueue(cmd.slot, msgid, (cmd.retry, bytes.clone()));
                                    }
                                }
                            }
                        }
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

    fn spawn_connect(&mut self, key: PublicKey, addr: NetAddr) {
        if self.key == key {
            return;
        }
        debug!(
            name = %self.conf.name,
            node = %self.key,
            peer = %key,
            addr = %addr,
            "spawning connect task"
        );
        let conn = Connection::connect(self.conf.clone(), key, addr);
        self.connect_tasks.spawn(key, conn);
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

        self.hello_tasks.abort(&conn.key);
        self.hello_tasks.spawn(
            conn.key,
            until(self.conf.handshake_timeout, async move {
                let theirs = conn.recv_hello().await?;
                conn.send_hello(ours.clone()).await?;
                Ok::<_, NetworkError>((ours, conn, theirs))
            }),
        );

        self.metrics
            .set(&self.key, HELLO_TASKS, self.hello_tasks.len());
    }

    fn spawn_peer(&mut self, key: PublicKey, mut peer: Peer) {
        debug!(
            name = %self.conf.name,
            node = %self.key,
            peer = %peer.public_key(),
            addr = %peer.socket_addr(),
            "spawning peer task"
        );
        let node = self.key;
        let name = self.conf.name.clone();
        let metrics = self.metrics.clone();
        self.peer_tasks.spawn(key, async move {
            let Err(err) = peer.start().await;
            if !matches!(err, NetworkError::PeerInterrupt) {
                warn!(
                    %name,
                    %node,
                    peer = %peer.public_key(),
                    addr = %peer.socket_addr(),
                    %err,
                    "peer failure"
                );
                metrics.add(peer.public_key(), ERRORS, 1)
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
    fn new(r: Role, a: NetAddr) -> Self {
        Self {
            addr: a,
            role: r,
            outbox: Queue::new(),
            peer: PeerState::None,
        }
    }

    fn ip_addr_mismatch(&self, addr: IpAddr) -> bool {
        let NetAddr::Inet(ip, _) = &self.addr else {
            return false;
        };
        *ip != addr
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
