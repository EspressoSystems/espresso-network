#![doc = include_str!("../README.md")]

use std::{
    collections::HashMap,
    fmt::Display,
    future::pending,
    hash::Hash,
    iter::{once, repeat},
    sync::Arc,
    time::Duration,
};

use bimap::BiHashMap;
use bytes::{Bytes, BytesMut};
use parking_lot::Mutex;
use snow::{Builder, HandshakeState, TransportState};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    spawn,
    sync::{
        Mutex as AsyncMutex, OwnedSemaphorePermit, Semaphore,
        mpsc::{self, Receiver, Sender},
    },
    task::{self, AbortHandle, JoinHandle, JoinSet},
    time::{Interval, MissedTickBehavior, sleep, timeout},
};
use tracing::{debug, error, info, trace, warn};

#[cfg(feature = "metrics")]
use crate::metrics::NetworkMetrics;
use crate::{
    Address, Id, Keypair, LAST_DELAY, NUM_DELAYS, NetConf, NetworkError, PublicKey, Role, chan,
    error::Empty,
    frame::{Header, Type},
    time::{Countdown, Timestamp},
};

type Budget = Arc<Semaphore>;
type Result<T> = std::result::Result<T, NetworkError>;

/// Max. message size using noise handshake.
const MAX_NOISE_HANDSHAKE_SIZE: usize = 1024;

/// Max. message size using noise protocol.
const MAX_NOISE_MESSAGE_SIZE: usize = 64 * 1024;

/// Max. number of bytes for payload data.
const MAX_PAYLOAD_SIZE: usize = MAX_NOISE_MESSAGE_SIZE - 32;

/// Noise parameters to initialize the builders.
const NOISE_PARAMS: &str = "Noise_IK_25519_AESGCM_BLAKE2s";

/// Interval between ping protocol.
const PING_INTERVAL: Duration = Duration::from_secs(15);

/// Max. allowed duration of a single TCP connect attempt.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Max. allowed duration of a Noise handshake.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Max. allowed duration to wait for a peer to answer.
///
/// This is started when we have sent a ping. Unless we receive
/// some data back within this duration, the connection times
/// out and is dropped.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);

/// `Network` is the API facade of this crate.
#[derive(Debug)]
pub struct Network<K> {
    /// Name of this network.
    name: &'static str,

    /// Log label.
    label: K,

    /// The network participants.
    parties: Mutex<HashMap<K, Role>>,

    /// MPSC sender of server task instructions.
    tx: Sender<Command<K>>,

    /// MPSC receiver of messages from a remote party.
    ///
    /// The public key identifies the remote.
    rx: AsyncMutex<Receiver<(K, Bytes, Option<OwnedSemaphorePermit>)>>,

    /// Handle of the server task that has been spawned by `Network`.
    srv: JoinHandle<Result<Empty>>,

    /// Max. number of bytes per message.
    max_message_size: usize,
}

impl<K> Drop for Network<K> {
    fn drop(&mut self) {
        self.srv.abort()
    }
}

/// Server task instructions.
#[derive(Debug)]
pub(crate) enum Command<K> {
    /// Add the given peers.
    Add(Vec<(K, PublicKey, Address)>),
    /// Remove the given peers.
    Remove(Vec<K>),
    /// Assign a `Role` to the given peers.
    Assign(Role, Vec<K>),
    /// Send a message to one peer.
    Unicast(K, Option<Id>, Bytes),
    /// Send a message to some peers.
    Multicast(Vec<K>, Option<Id>, Bytes),
    /// Send a message to all peers with `Role::Active`.
    Broadcast(Option<Id>, Bytes),
}

/// The `Server` is accepting connections and also establishing and
/// maintaining connections with all parties.
#[derive(Debug)]
struct Server<K> {
    conf: NetConf<K>,

    /// This server's role.
    role: Role,

    /// MPSC sender for messages received over a connection to a party.
    ///
    /// (see `Network` for the accompanying receiver).
    ibound: Sender<(K, Bytes, Option<OwnedSemaphorePermit>)>,

    /// MPSC receiver for server task instructions.
    ///
    /// (see `Network` for the accompanying sender).
    obound: Receiver<Command<K>>,

    /// All parties of the network and their addresses.
    peers: HashMap<K, Peer>,

    /// Bi-directional mapping of signing key and X25519 keys to identify
    /// remote parties.
    index: BiHashMap<K, PublicKey>,

    /// Find the public key given a tokio task ID.
    task2key: HashMap<task::Id, K>,

    /// Currently active connect attempts.
    connecting: HashMap<K, ConnectTask>,

    /// Currently active connections (post handshake).
    active: HashMap<K, IoTask>,

    /// Tasks performing a handshake with a remote party.
    handshake_tasks: JoinSet<Result<(TcpStream, TransportState)>>,

    /// Tasks connecting to a remote party and performing a handshake.
    connect_tasks: JoinSet<(TcpStream, TransportState)>,

    /// Active I/O tasks, exchanging data with remote parties.
    io_tasks: JoinSet<Result<()>>,

    /// Interval at which to ping peers.
    ping_interval: Interval,

    /// For gathering network metrics.
    #[cfg(feature = "metrics")]
    metrics: Arc<NetworkMetrics<K>>,
}

#[derive(Debug)]
struct Peer {
    addr: Address,
    role: Role,
    budget: Budget,
}

/// A connect task.
#[derive(Debug)]
struct ConnectTask {
    h: AbortHandle,
}

// Make sure the task is stopped when `ConnectTask` is dropped.
impl Drop for ConnectTask {
    fn drop(&mut self) {
        self.h.abort();
    }
}

/// An I/O task, reading data from and writing data to a remote party.
#[derive(Debug)]
struct IoTask {
    /// Abort handle of the read-half of the connection.
    rh: AbortHandle,

    /// Abort handle of the write-half of the connection.
    wh: AbortHandle,

    /// MPSC sender of outgoing messages to the remote.
    tx: chan::Sender<Message>,
}

// Make sure all tasks are stopped when `IoTask` is dropped.
impl Drop for IoTask {
    fn drop(&mut self) {
        self.rh.abort();
        self.wh.abort();
    }
}

/// Unify the various data types we want to send to the writer task.
#[derive(Debug)]
enum Message {
    Data(Bytes),
    Ping(Timestamp),
    Pong(Timestamp),
}

impl<K> Network<K>
where
    K: Eq + Ord + Clone + Display + Hash + Send + Sync + 'static,
{
    pub async fn create(cfg: NetConf<K>) -> Result<Self> {
        let listener = TcpListener::bind(cfg.bind.to_string())
            .await
            .map_err(|e| NetworkError::Bind(cfg.bind.clone(), e))?;

        debug!(
            name = %cfg.name,
            node = %cfg.label,
            addr = %listener.local_addr()?,
            "listening"
        );

        let mut parties = HashMap::new();
        let mut peers = HashMap::new();
        let mut index = BiHashMap::new();

        for (k, x, a) in cfg.parties.iter().cloned() {
            parties.insert(k.clone(), Role::Active);
            index.insert(k.clone(), x);
            peers.insert(
                k,
                Peer {
                    addr: a,
                    role: Role::Active,
                    budget: cfg.new_budget(),
                },
            );
        }

        // Command channel from application to network.
        let (otx, orx) = mpsc::channel(cfg.total_capacity_egress);

        // Channel of messages from peers to the application.
        let (itx, irx) = mpsc::channel(cfg.total_capacity_ingress);

        let mut interval = tokio::time::interval(PING_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        let name = cfg.name;
        let label = cfg.label.clone();
        let mmsze = cfg.max_message_size;

        #[cfg(feature = "metrics")]
        let metrics = {
            let it = parties.keys().filter(|k| **k != label).cloned();
            NetworkMetrics::new(name, &*cfg.metrics, it)
        };

        let server = Server {
            conf: cfg,
            role: Role::Active,
            ibound: itx,
            obound: orx,
            peers,
            index,
            connecting: HashMap::new(),
            active: HashMap::new(),
            task2key: HashMap::new(),
            handshake_tasks: JoinSet::new(),
            connect_tasks: JoinSet::new(),
            io_tasks: JoinSet::new(),
            ping_interval: interval,
            #[cfg(feature = "metrics")]
            metrics: Arc::new(metrics),
        };

        Ok(Self {
            name,
            label,
            parties: Mutex::new(parties),
            rx: AsyncMutex::new(irx),
            tx: otx,
            srv: spawn(server.run(listener)),
            max_message_size: mmsze,
        })
    }

    pub fn public_key(&self) -> &K {
        &self.label
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn parties(&self, r: Role) -> Vec<K> {
        self.parties
            .lock()
            .iter()
            .filter(|&(_, x)| r == *x)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Send a message to a party, identified by the given public key.
    pub async fn unicast(&self, to: K, msg: Bytes) -> Result<()> {
        if msg.len() > self.max_message_size {
            warn!(
                name = %self.name,
                node = %self.label,
                to   = %to,
                len  = %msg.len(),
                max  = %self.max_message_size,
                "message too large to send"
            );
            return Err(NetworkError::MessageTooLarge);
        }
        self.tx
            .send(Command::Unicast(to, None, msg))
            .await
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Send a message to all parties.
    pub async fn broadcast(&self, msg: Bytes) -> Result<()> {
        if msg.len() > self.max_message_size {
            warn!(
                name = %self.name,
                node = %self.label,
                len  = %msg.len(),
                max  = %self.max_message_size,
                "message too large to broadcast"
            );
            return Err(NetworkError::MessageTooLarge);
        }
        self.tx
            .send(Command::Broadcast(None, msg))
            .await
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Receive a message from a remote party.
    pub async fn receive(&self) -> Result<(K, Bytes)> {
        let mut rx = self.rx.lock().await;
        let (k, b, _) = rx.recv().await.ok_or(NetworkError::ChannelClosed)?;
        Ok((k, b))
    }

    /// Add the given peers to the network.
    ///
    /// NB that peers added here are passive. See `Network::assign` for
    /// giving peers a different `Role`.
    pub async fn add(&self, peers: Vec<(K, PublicKey, Address)>) -> Result<()> {
        self.parties
            .lock()
            .extend(peers.iter().map(|(p, ..)| (p.clone(), Role::Passive)));
        self.tx
            .send(Command::Add(peers))
            .await
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Remove the given peers from the network.
    pub async fn remove(&self, peers: Vec<K>) -> Result<()> {
        {
            let mut parties = self.parties.lock();
            for p in &peers {
                parties.remove(p);
            }
        }
        self.tx
            .send(Command::Remove(peers))
            .await
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Assign the given role to the given peers.
    pub async fn assign(&self, r: Role, peers: Vec<K>) -> Result<()> {
        {
            let mut parties = self.parties.lock();
            for p in &peers {
                if let Some(role) = parties.get_mut(p) {
                    *role = r
                }
            }
        }
        self.tx
            .send(Command::Assign(r, peers))
            .await
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Get a clone of the MPSC sender.
    pub(crate) fn sender(&self) -> Sender<Command<K>> {
        self.tx.clone()
    }
}

impl<K> Server<K>
where
    K: Eq + Ord + Clone + Display + Hash + Send + Sync + 'static,
{
    /// Runs the main loop of this network node.
    ///
    /// This function:
    ///
    /// - Tries to connect to each remote peer in the committee.
    /// - Handles tasks that have been completed or terminated.
    /// - Processes new messages we received on the network.
    async fn run(mut self, listener: TcpListener) -> Result<Empty> {
        self.handshake_tasks.spawn(pending());
        self.io_tasks.spawn(pending());

        // Connect to all peers.
        for k in self
            .peers
            .keys()
            .filter(|k| **k != self.conf.label)
            .cloned()
            .collect::<Vec<_>>()
        {
            self.spawn_connect(k)
        }

        loop {
            trace!(
                name       = %self.conf.name,
                node       = %self.conf.label,
                active     = %self.active.len(),
                connects   = %self.connect_tasks.len(),
                handshakes = %self.handshake_tasks.len().saturating_sub(1), // -1 for `pending()`
                io_tasks   = %self.io_tasks.len().saturating_sub(1), // -1 for `pending()`
                tasks_ids  = %self.task2key.len(),
                iqueue     = %self.ibound.capacity(),
                oqueue     = %self.obound.capacity(),
            );

            #[cfg(feature = "metrics")]
            {
                self.metrics.iqueue.set(self.ibound.capacity());
                self.metrics.oqueue.set(self.obound.capacity());
            }

            tokio::select! {
                // Accepted a new connection.
                i = listener.accept() => match i {
                    Ok((s, a)) => {
                        debug!(
                            name = %self.conf.name,
                            node = %self.conf.label,
                            addr = %a,
                            "accepted connection"
                        );
                        self.spawn_handshake(s)
                    }
                    Err(e) => {
                        warn!(
                            name = %self.conf.name,
                            node = %self.conf.label,
                            err  = %e,
                            "error accepting connection"
                        )
                    }
                },
                // The handshake of an inbound connection completed.
                Some(h) = self.handshake_tasks.join_next() => match h {
                    Ok(Ok((s, t))) => {
                        let Some((k, peer)) = self.lookup_peer(&t) else {
                            info!(
                                name = %self.conf.name,
                                node = %self.conf.label,
                                peer = ?t.get_remote_static().and_then(|k| PublicKey::try_from(k).ok()),
                                addr = ?s.peer_addr().ok(),
                                "unknown peer"
                            );
                            continue
                        };
                        if !self.is_valid_ip(&k, &s) {
                            warn!(
                                name = %self.conf.name,
                                node = %self.conf.label,
                                peer = %k,
                                addr = ?s.peer_addr().ok(), "invalid peer ip addr"
                            );
                            continue
                        }
                        // We only accept connections whose party has a public key that
                        // is larger than ours, or if we do not have a connection for
                        // that key at the moment.
                        if k > self.conf.label || !self.active.contains_key(&k) {
                            self.spawn_io(k, s, t, peer.budget.clone())
                        } else {
                            debug!(
                                name = %self.conf.name,
                                node = %self.conf.label,
                                peer = %k,
                                "dropping accepted connection"
                            );
                        }
                    }
                    Ok(Err(e)) => {
                        warn!(
                            name = %self.conf.name,
                            node = %self.conf.label,
                            err  = %e,
                            "handshake failed"
                        )
                    }
                    Err(e) => {
                        if !e.is_cancelled() {
                            error!(
                                name = %self.conf.name,
                                node = %self.conf.label,
                                err  = %e,
                                "handshake task panic"
                            )
                        }
                    }
                },
                // One of our connection attempts completed.
                Some(tt) = self.connect_tasks.join_next_with_id() => {
                    match tt {
                        Ok((id, (s, t))) => {
                            self.on_connect_task_end(id);
                            let Some((k, peer)) = self.lookup_peer(&t) else {
                                warn!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    peer = ?t.get_remote_static().and_then(|k| PublicKey::try_from(k).ok()),
                                    addr = ?s.peer_addr().ok(),
                                    "connected to unknown peer"
                                );
                                continue
                            };
                            // We only keep the connection if our key is larger than the remote,
                            // or if we do not have a connection for that key at the moment.
                            if k < self.conf.label || !self.active.contains_key(&k) {
                                self.spawn_io(k, s, t, peer.budget.clone())
                            } else {
                                debug!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    peer = %k,
                                    "dropping new connection"
                                )
                            }
                        }
                        Err(e) => {
                            if !e.is_cancelled() {
                                error!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    err  = %e,
                                    "connect task panic"
                                )
                            }
                            self.on_connect_task_end(e.id());
                        }
                    }
                },
                // A read or write task completed.
                Some(io) = self.io_tasks.join_next_with_id() => {
                    match io {
                        Ok((id, r)) => {
                            if let Err(e) = r {
                                warn!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    err  = %e,
                                    "i/o error"
                                )
                            }
                            self.on_io_task_end(id);
                        }
                        Err(e) => {
                            if e.is_cancelled() {
                                // If one half completes we cancel the other, so there is
                                // nothing else to do here, except to remove the cancelled
                                // tasks's ID. Same if we kill the connection, both tasks
                                // get cancelled.
                                self.task2key.remove(&e.id());
                                continue
                            }
                            // If the task has not been cancelled, it must have panicked.
                            error!(
                                name = %self.conf.name,
                                node = %self.conf.label,
                                err  = %e,
                                "i/o task panic"
                            );
                            self.on_io_task_end(e.id())
                        }
                    };
                },
                cmd = self.obound.recv() => match cmd {
                    Some(Command::Add(peers)) => {
                        #[cfg(feature = "metrics")]
                        Arc::make_mut(&mut self.metrics).add_parties(peers.iter().map(|(k, ..)| k).cloned());
                        for (k, x, a) in peers {
                            if self.peers.contains_key(&k) {
                                warn!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    peer = %k,
                                    "peer to add already exists"
                                );
                                continue
                            }
                            info!(
                                name = %self.conf.name,
                                node = %self.conf.label,
                                peer = %k,
                                "adding peer"
                            );
                            let p = Peer {
                                addr: a,
                                role: Role::Passive,
                                budget: self.conf.new_budget()
                            };
                            self.peers.insert(k.clone(), p);
                            self.index.insert(k.clone(), x);
                            self.spawn_connect(k)
                        }
                    }
                    Some(Command::Remove(peers)) => {
                        for k in &peers {
                            info!(
                                name = %self.conf.name,
                                node = %self.conf.label,
                                peer = %k,
                                "removing peer"
                            );
                            self.peers.remove(k);
                            self.index.remove_by_left(k);
                            self.connecting.remove(k);
                            self.active.remove(k);
                        }
                        #[cfg(feature = "metrics")]
                        Arc::make_mut(&mut self.metrics).remove_parties(&peers)
                    }
                    Some(Command::Assign(role, peers)) => {
                        for k in &peers {
                            if let Some(p) = self.peers.get_mut(k) {
                                p.role = role
                            } else {
                                warn!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    peer = %k,
                                    role = ?role,
                                    "peer to assign role to not found"
                                );
                            }
                        }
                    }
                    Some(Command::Unicast(to, id, m)) => {
                        if to == self.conf.label {
                            trace!(
                                name  = %self.conf.name,
                                node  = %self.conf.label,
                                to    = %to,
                                len   = %m.len(),
                                queue = self.ibound.capacity(),
                                "sending message"
                            );
                            if let Err(err) = self.ibound.try_send((self.conf.label.clone(), m, None)) {
                                warn!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    err  = %err,
                                    cap  = %self.ibound.capacity(),
                                    "channel full => dropping unicast message"
                                )
                            }
                            continue
                        }
                        if let Some(task) = self.active.get(&to) {
                            trace!(
                                name  = %self.conf.name,
                                node  = %self.conf.label,
                                to    = %to,
                                len   = %m.len(),
                                queue = task.tx.capacity(),
                                "sending message"
                            );
                            #[cfg(feature = "metrics")]
                            self.metrics.set_peer_oqueue_cap(&to, task.tx.capacity());
                            task.tx.send(id, Message::Data(m))
                        }
                    }
                    Some(Command::Multicast(peers, id, m)) => {
                        if peers.contains(&self.conf.label) {
                            trace!(
                                name  = %self.conf.name,
                                node  = %self.conf.label,
                                to    = %self.conf.label,
                                len   = %m.len(),
                                queue = self.ibound.capacity(),
                                "sending message"
                            );
                            if let Err(err) = self.ibound.try_send((self.conf.label.clone(), m.clone(), None)) {
                                warn!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    err  = %err,
                                    cap  = %self.ibound.capacity(),
                                    "channel full => dropping multicast message"
                                )
                            }
                        }
                        for (to, task) in &self.active {
                            if !peers.contains(to) {
                                continue
                            }
                            trace!(
                                name  = %self.conf.name,
                                node  = %self.conf.label,
                                to    = %to,
                                len   = %m.len(),
                                queue = task.tx.capacity(),
                                "sending message"
                            );
                            #[cfg(feature = "metrics")]
                            self.metrics.set_peer_oqueue_cap(to, task.tx.capacity());
                            task.tx.send(id, Message::Data(m.clone()))
                        }
                    }
                    Some(Command::Broadcast(id, m)) => {
                        if self.role.is_active() {
                            trace!(
                                name  = %self.conf.name,
                                node  = %self.conf.label,
                                to    = %self.conf.label,
                                len   = %m.len(),
                                queue = self.ibound.capacity(),
                                "sending message"
                            );
                            if let Err(err) = self.ibound.try_send((self.conf.label.clone(), m.clone(), None)) {
                                warn!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    err  = %err,
                                    cap  = %self.ibound.capacity(),
                                    "channel full => dropping broadcast message"
                                )
                            }
                        }
                        for (to, task) in &self.active {
                            if Some(Role::Active) != self.peers.get(to).map(|p| p.role) {
                                continue
                            }
                            trace!(
                                name  = %self.conf.name,
                                node  = %self.conf.label,
                                to    = %to,
                                len   = %m.len(),
                                queue = task.tx.capacity(),
                                "sending message"
                            );
                            #[cfg(feature = "metrics")]
                            self.metrics.set_peer_oqueue_cap(to, task.tx.capacity());
                            task.tx.send(id, Message::Data(m.clone()))
                        }
                    }
                    None => {
                        return Err(NetworkError::ChannelClosed)
                    }
                },
                _ = self.ping_interval.tick() => {
                    let now = Timestamp::now();
                    for task in self.active.values() {
                        task.tx.send(None, Message::Ping(now))
                    }
                }
            }
        }
    }

    /// Handles a completed connect task.
    fn on_connect_task_end(&mut self, id: task::Id) {
        let Some(k) = self.task2key.remove(&id) else {
            error!(name = %self.conf.name, node = %self.conf.label, "no key for connect task");
            return;
        };
        self.connecting.remove(&k);
    }

    /// Handles a completed I/O task.
    ///
    /// This function will get the public key of the task that was terminated
    /// and then cleanly removes the associated I/O task data and re-connects
    /// to the peer node it was interacting with.
    fn on_io_task_end(&mut self, id: task::Id) {
        let Some(k) = self.task2key.remove(&id) else {
            error!(name = %self.conf.name, node = %self.conf.label, "no key for i/o task");
            return;
        };
        let Some(task) = self.active.get(&k) else {
            return;
        };
        if task.rh.id() == id {
            debug!(
                name = %self.conf.name,
                node = %self.conf.label,
                peer = %k,
                "read-half closed => dropping connection"
            );
            self.active.remove(&k);
            self.spawn_connect(k)
        } else if task.wh.id() == id {
            debug!(
                name = %self.conf.name,
                node = %self.conf.label,
                peer = %k,
                "write-half closed => dropping connection"
            );
            self.active.remove(&k);
            self.spawn_connect(k)
        } else {
            debug!(
                name = %self.conf.name,
                node = %self.conf.label,
                peer = %k,
                "i/o task was previously replaced"
            );
        }
    }

    /// Spawns a new connection task to a peer identified by public key.
    ///
    /// This function will look up the x25519 public key of the ed25519 key
    /// and the remote address and then spawn a connection task.
    fn spawn_connect(&mut self, k: K) {
        if self.connecting.contains_key(&k) {
            debug!(
                name = %self.conf.name,
                node = %self.conf.label,
                peer = %k,
                "connect task already started"
            );
            return;
        }
        let x = self.index.get_by_left(&k).expect("known public key");
        let p = self.peers.get(&k).expect("known peer");
        let h = self.connect_tasks.spawn(connect(
            self.conf.name,
            (self.conf.label.clone(), self.conf.keypair.clone()),
            (k.clone(), *x),
            p.addr.clone(),
            self.conf.retry_delays,
            #[cfg(feature = "metrics")]
            self.metrics.clone(),
        ));
        assert!(self.task2key.insert(h.id(), k.clone()).is_none());
        self.connecting.insert(k, ConnectTask { h });
    }

    /// Spawns a new `Noise` responder handshake task using the IK pattern.
    ///
    /// This function will create the responder handshake machine using its
    /// own private key and then spawn a task that awaits an initiator handshake
    /// to which it will respond.
    fn spawn_handshake(&mut self, s: TcpStream) {
        let h = Builder::new(NOISE_PARAMS.parse().expect("valid noise params"))
            .local_private_key(&self.conf.keypair.secret_key().as_bytes())
            .expect("valid private key")
            .prologue(self.conf.name.as_bytes())
            .expect("1st time we set the prologue")
            .build_responder()
            .expect("valid noise params yield valid handshake state");
        self.handshake_tasks.spawn(async move {
            timeout(HANDSHAKE_TIMEOUT, on_handshake(h, s))
                .await
                .or(Err(NetworkError::Timeout))?
        });
    }

    /// Spawns a new I/O task for handling communication with a remote peer over
    /// a TCP connection using the noise framework to create an authenticated
    /// secure link.
    fn spawn_io(&mut self, k: K, s: TcpStream, t: TransportState, b: Budget) {
        debug!(
            name = %self.conf.name,
            node = %self.conf.label,
            peer = %k,
            addr = ?s.peer_addr().ok(),
            "starting i/o tasks"
        );
        let (to_remote, from_remote) = chan::channel(self.conf.peer_capacity_egress);
        let (r, w) = s.into_split();
        let t1 = Arc::new(Mutex::new(t));
        let t2 = t1.clone();
        let ibound = self.ibound.clone();
        let to_write = to_remote.clone();
        let countdown = Countdown::new();
        let rh = self.io_tasks.spawn(recv_loop(
            self.conf.name,
            k.clone(),
            r,
            t1,
            ibound,
            to_write,
            #[cfg(feature = "metrics")]
            self.metrics.clone(),
            b,
            countdown.clone(),
            self.conf.max_message_size,
        ));
        let wh = self
            .io_tasks
            .spawn(send_loop(w, t2, from_remote, countdown));
        assert!(self.task2key.insert(rh.id(), k.clone()).is_none());
        assert!(self.task2key.insert(wh.id(), k.clone()).is_none());
        let io = IoTask {
            rh,
            wh,
            tx: to_remote,
        };
        self.active.insert(k, io);
        #[cfg(feature = "metrics")]
        self.metrics.connections.set(self.active.len());
    }

    /// Get the public key of a party by their static X25519 public key.
    fn lookup_peer(&self, t: &TransportState) -> Option<(K, &Peer)> {
        let x = t.get_remote_static()?;
        let x = PublicKey::try_from(x).ok()?;
        let k = self.index.get_by_right(&x)?;
        self.peers.get(k).map(|p| (k.clone(), p))
    }

    /// Check if the socket's peer IP address corresponds to the configured one.
    fn is_valid_ip(&self, k: &K, s: &TcpStream) -> bool {
        self.peers
            .get(k)
            .map(|p| {
                let Address::Inet(ip, _) = p.addr else {
                    return true;
                };
                Some(ip) == s.peer_addr().ok().map(|a| a.ip())
            })
            .unwrap_or(false)
    }
}

/// Connect to the given socket address.
///
/// This function will only return, when a connection has been established and the handshake
/// has been completed.
async fn connect<K>(
    name: &'static str,
    this: (K, Keypair),
    to: (K, PublicKey),
    addr: Address,
    delays: [u8; NUM_DELAYS],
    #[cfg(feature = "metrics")] metrics: Arc<NetworkMetrics<K>>,
) -> (TcpStream, TransportState)
where
    K: Eq + Hash + Display + Clone,
{
    use rand::prelude::*;

    let new_handshake_state = || {
        Builder::new(NOISE_PARAMS.parse().expect("valid noise params"))
            .local_private_key(this.1.secret_key().as_slice())
            .expect("valid private key")
            .remote_public_key(to.1.as_slice())
            .expect("valid remote pub key")
            .prologue(name.as_bytes())
            .expect("1st time we set the prologue")
            .build_initiator()
            .expect("valid noise params yield valid handshake state")
    };

    let delays = once(rand::rng().random_range(0..=1000))
        .chain(delays.into_iter().map(|d| u64::from(d) * 1000))
        .chain(repeat(u64::from(delays[LAST_DELAY]) * 1000));

    let addr = addr.to_string();

    for d in delays {
        sleep(Duration::from_millis(d)).await;
        debug!(%name, node = %this.0, peer = %to.0, %addr, "connecting");
        #[cfg(feature = "metrics")]
        metrics.add_connect_attempt(&to.0);
        match timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr)).await {
            Ok(Ok(s)) => {
                if let Err(err) = s.set_nodelay(true) {
                    error!(%name, node = %this.0, %err, "failed to set NO_DELAY socket option");
                    continue;
                }
                match timeout(HANDSHAKE_TIMEOUT, handshake(new_handshake_state(), s)).await {
                    Ok(Ok(x)) => {
                        debug!(%name, node = %this.0, peer = %to.0, %addr, "connection established");
                        return x;
                    },
                    Ok(Err(err)) => {
                        warn!(%name, node = %this.0, peer = %to.0, %addr, %err, "handshake failure");
                    },
                    Err(_) => {
                        warn!(%name, node = %this.0, peer = %to.0, %addr, "handshake timeout");
                    },
                }
            },
            Ok(Err(err)) => {
                warn!(%name, node = %this.0, peer = %to.0, %addr, %err, "failed to connect");
            },
            Err(_) => {
                warn!(%name, node = %this.0, peer = %to.0, %addr, "connect timeout");
            },
        }
    }

    unreachable!("for loop repeats forever")
}

/// Perform a noise handshake as initiator with the remote party.
async fn handshake(
    mut hs: HandshakeState,
    mut stream: TcpStream,
) -> Result<(TcpStream, TransportState)> {
    let mut b = vec![0; MAX_NOISE_HANDSHAKE_SIZE];
    let n = hs.write_message(&[], &mut b[Header::SIZE..])?;
    let h = Header::data(n as u16);
    send_frame(&mut stream, h, &mut b[..Header::SIZE + n]).await?;
    let (h, m) = recv_frame(&mut stream).await?;
    if !h.is_data() || h.is_partial() {
        return Err(NetworkError::InvalidHandshakeMessage);
    }
    hs.read_message(&m, &mut b)?;
    Ok((stream, hs.into_transport_mode()?))
}

/// Perform a noise handshake as responder with a remote party.
async fn on_handshake(
    mut hs: HandshakeState,
    mut stream: TcpStream,
) -> Result<(TcpStream, TransportState)> {
    stream.set_nodelay(true)?;
    let (h, m) = recv_frame(&mut stream).await?;
    if !h.is_data() || h.is_partial() {
        return Err(NetworkError::InvalidHandshakeMessage);
    }
    let mut b = vec![0; MAX_NOISE_HANDSHAKE_SIZE];
    hs.read_message(&m, &mut b)?;
    let n = hs.write_message(&[], &mut b[Header::SIZE..])?;
    let h = Header::data(n as u16);
    send_frame(&mut stream, h, &mut b[..Header::SIZE + n]).await?;
    Ok((stream, hs.into_transport_mode()?))
}

/// Read messages from the remote by assembling frames together.
///
/// Once complete the message will be handed over to the given MPSC sender.
#[allow(clippy::too_many_arguments)]
async fn recv_loop<R, K>(
    name: &'static str,
    id: K,
    mut reader: R,
    state: Arc<Mutex<TransportState>>,
    to_deliver: Sender<(K, Bytes, Option<OwnedSemaphorePermit>)>,
    to_writer: chan::Sender<Message>,
    #[cfg(feature = "metrics")] metrics: Arc<NetworkMetrics<K>>,
    budget: Arc<Semaphore>,
    mut countdown: Countdown,
    max_message_size: usize,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    K: Eq + Hash + Display + Clone,
{
    let mut buf = vec![0; MAX_NOISE_MESSAGE_SIZE];
    loop {
        #[cfg(feature = "metrics")]
        metrics.set_peer_iqueue_cap(&id, budget.available_permits());
        let permit = budget
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| NetworkError::BudgetClosed)?;
        let mut msg = BytesMut::new();
        loop {
            tokio::select! {
                val = recv_frame(&mut reader) => {
                    countdown.stop();
                    match val {
                        Ok((h, f)) => {
                            match h.frame_type() {
                                Ok(Type::Ping) => {
                                    // Received ping message; sending pong to writer
                                    let n = state.lock().read_message(&f, &mut buf)?;
                                    if let Some(ping) = Timestamp::try_from_slice(&buf[..n]) {
                                        to_writer.send(None, Message::Pong(ping))
                                    }
                                }
                                Ok(Type::Pong) => {
                                    // Received pong message; measure elapsed time
                                    let _n = state.lock().read_message(&f, &mut buf)?;
                                    #[cfg(feature = "metrics")]
                                    if let Some(ping) = Timestamp::try_from_slice(&buf[.._n])
                                        && let Some(delay) = Timestamp::now().diff(ping)
                                    {
                                        metrics.set_latency(&id, delay)
                                    }
                                }
                                Ok(Type::Data) => {
                                    let n = state.lock().read_message(&f, &mut buf)?;
                                    msg.extend_from_slice(&buf[..n]);
                                    if !h.is_partial() {
                                        break;
                                    }
                                    if msg.len() > max_message_size {
                                        return Err(NetworkError::MessageTooLarge);
                                    }
                                }
                                Err(t) => return Err(NetworkError::UnknownFrameType(t)),
                            }
                        }
                        Err(e) => return Err(e)
                    }
                },
                () = &mut countdown => {
                    warn!(%name, node = %id, "timeout waiting for peer");
                    return Err(NetworkError::Timeout)
                }
            }
        }
        if to_deliver
            .send((id.clone(), msg.freeze(), Some(permit)))
            .await
            .is_err()
        {
            break;
        }
    }
    Ok(())
}

/// Consume messages to be delivered to remote parties and send them.
///
/// The function automatically splits large messages into chunks that fit into
/// a noise package.
async fn send_loop<W>(
    mut writer: W,
    state: Arc<Mutex<TransportState>>,
    rx: chan::Receiver<Message>,
    countdown: Countdown,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut buf = vec![0; MAX_NOISE_MESSAGE_SIZE];

    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Ping(ping) => {
                let n = state
                    .lock()
                    .write_message(&ping.to_bytes()[..], &mut buf[Header::SIZE..])?;
                let h = Header::ping(n as u16);
                send_frame(&mut writer, h, &mut buf[..Header::SIZE + n]).await?;
                countdown.start(REPLY_TIMEOUT)
            },
            Message::Pong(pong) => {
                let n = state
                    .lock()
                    .write_message(&pong.to_bytes()[..], &mut buf[Header::SIZE..])?;
                let h = Header::pong(n as u16);
                send_frame(&mut writer, h, &mut buf[..Header::SIZE + n]).await?
            },
            Message::Data(msg) => {
                let mut it = msg.chunks(MAX_PAYLOAD_SIZE).peekable();
                while let Some(m) = it.next() {
                    let n = state.lock().write_message(m, &mut buf[Header::SIZE..])?;
                    let h = if it.peek().is_some() {
                        Header::data(n as u16).partial()
                    } else {
                        Header::data(n as u16)
                    };
                    send_frame(&mut writer, h, &mut buf[..Header::SIZE + n]).await?
                }
            },
        }
    }
    Ok(())
}

/// Read a single frame (header + payload) from the remote.
async fn recv_frame<R>(r: &mut R) -> Result<(Header, Vec<u8>)>
where
    R: AsyncRead + Unpin,
{
    let b = r.read_u32().await?;
    let h = Header::try_from(b.to_be_bytes())?;
    let mut v = vec![0; h.len().into()];
    r.read_exact(&mut v).await?;
    Ok((h, v))
}

/// Write a single frame (header + payload) to the remote.
///
/// The header is serialised into the first 4 bytes of `msg`. It is the
/// caller's responsibility to ensure there is room at the beginning.
async fn send_frame<W>(w: &mut W, hdr: Header, msg: &mut [u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    debug_assert!(msg.len() <= MAX_NOISE_MESSAGE_SIZE);
    msg[..Header::SIZE].copy_from_slice(&hdr.to_bytes());
    w.write_all(msg).await?;
    Ok(())
}
