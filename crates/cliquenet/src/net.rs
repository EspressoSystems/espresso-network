#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::fmt::Display;
use std::future::pending;
use std::hash::Hash;
use std::iter::repeat;
use std::sync::Arc;
use std::time::Duration;

use bon::Builder;
use bytes::{Bytes, BytesMut};
use minicbor::{Decode, Encode};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{sleep, timeout, Interval, MissedTickBehavior};
use tokio::{
    spawn,
    task::{self, AbortHandle, JoinHandle, JoinSet},
};
use tracing::{debug, error, info, trace, warn};

use crate::chan;
use crate::error::Empty;
use crate::frame::{Header, Type};
use crate::time::{Countdown, Timestamp};
use crate::{Address, Id, NetworkError, Role, MAX_MESSAGE_SIZE};

type Budget = Arc<Semaphore>;
type Result<T> = std::result::Result<T, NetworkError>;

/// Max. number of bytes for payload data.
const MAX_PAYLOAD_SIZE: usize = u16::MAX as usize;

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
    parties: HashMap<K, Role>,

    /// MPSC sender of server task instructions.
    tx: Sender<Command<K>>,

    /// MPSC receiver of messages from a remote party.
    ///
    /// The public key identifies the remote.
    rx: Receiver<(K, Bytes, Option<OwnedSemaphorePermit>)>,

    /// Handle of the server task that has been spawned by `Network`.
    srv: JoinHandle<Result<Empty>>,
}

impl<K> Drop for Network<K> {
    fn drop(&mut self) {
        self.srv.abort()
    }
}

#[derive(Debug, Builder)]
pub struct NetConf<K> {
    /// Network name.
    name: &'static str,

    /// Network public key.
    label: K,

    /// Address to bind to.
    bind: Address,

    /// Committee members with key material and bind address.
    #[builder(with = <_>::from_iter)]
    parties: Vec<(K, Address)>,

    /// Total egress channel capacity.
    ///
    /// Default is n⁴ with n = number of parties.
    #[builder(default = parties.len() * parties.len() * parties.len() * parties.len())]
    total_capacity_egress: usize,

    /// Total ingress channel capacity.
    ///
    /// Default is n⁴ with n = number of parties.
    #[builder(default = parties.len() * parties.len() * parties.len() * parties.len())]
    total_capacity_ingress: usize,

    /// Egress channel capacity per peer.
    ///
    /// Default is n³ with n = number of parties.
    #[builder(default = parties.len() * parties.len() * parties.len())]
    peer_capacity_egress: usize,

    /// Ingress channel capacity per peer.
    ///
    /// Default is 2n² with n = number of parties.
    #[builder(default = 2 * parties.len() * parties.len())]
    peer_capacity_ingress: usize,
}

impl<K> NetConf<K> {
    fn new_budget(&self) -> Budget {
        Arc::new(Semaphore::new(self.peer_capacity_ingress))
    }
}

/// Server task instructions.
#[derive(Debug)]
pub(crate) enum Command<K> {
    /// Add the given peers.
    Add(Vec<(K, Address)>),
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

    /// Find the public key given a tokio task ID.
    task2key: HashMap<task::Id, K>,

    /// Currently active connect attempts.
    connecting: HashMap<K, ConnectTask>,

    /// Currently active connections (post handshake).
    active: HashMap<K, IoTask>,

    /// Tasks performing a handshake with a remote party.
    handshake_tasks: JoinSet<Result<(TcpStream, K)>>,

    /// Tasks connecting to a remote party and performing a handshake.
    connect_tasks: JoinSet<(TcpStream, K)>,

    /// Active I/O tasks, exchanging data with remote parties.
    io_tasks: JoinSet<Result<()>>,

    /// Interval at which to ping peers.
    ping_interval: Interval,
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
    K: Encode<()>
        + for<'a> Decode<'a, ()>
        + Eq
        + Ord
        + Send
        + Clone
        + Copy
        + Display
        + Hash
        + 'static,
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

        for (k, a) in cfg.parties.iter().cloned() {
            parties.insert(k, Role::Active);
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
        let label = cfg.label;
        let server = Server {
            conf: cfg,
            role: Role::Active,
            ibound: itx,
            obound: orx,
            peers,
            connecting: HashMap::new(),
            active: HashMap::new(),
            task2key: HashMap::new(),
            handshake_tasks: JoinSet::new(),
            connect_tasks: JoinSet::new(),
            io_tasks: JoinSet::new(),
            ping_interval: interval,
        };

        Ok(Self {
            name,
            label,
            parties,
            rx: irx,
            tx: otx,
            srv: spawn(server.run(listener)),
        })
    }

    pub fn public_key(&self) -> &K {
        &self.label
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn parties(&self) -> impl Iterator<Item = (&K, &Role)> {
        self.parties.iter()
    }

    /// Send a message to a party, identified by the given public key.
    pub async fn unicast(&self, to: K, msg: Bytes) -> Result<()> {
        if msg.len() > MAX_MESSAGE_SIZE {
            warn!(
                name = %self.name,
                node = %self.label,
                %to,
                len = %msg.len(),
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
        if msg.len() > MAX_MESSAGE_SIZE {
            warn!(
                name = %self.name,
                node = %self.label,
                len = %msg.len(),
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
    pub async fn receive(&mut self) -> Result<(K, Bytes)> {
        let (k, b, _) = self.rx.recv().await.ok_or(NetworkError::ChannelClosed)?;
        Ok((k, b))
    }

    /// Add the given peers to the network.
    ///
    /// NB that peers added here are passive. See `Network::assign` for
    /// giving peers a different `Role`.
    pub async fn add(&mut self, peers: Vec<(K, Address)>) -> Result<()> {
        self.parties
            .extend(peers.iter().map(|(p, ..)| (p.clone(), Role::Passive)));
        self.tx
            .send(Command::Add(peers))
            .await
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Remove the given peers from the network.
    pub async fn remove(&mut self, peers: Vec<K>) -> Result<()> {
        for p in &peers {
            self.parties.remove(p);
        }
        self.tx
            .send(Command::Remove(peers))
            .await
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Assign the given role to the given peers.
    pub async fn assign(&mut self, r: Role, peers: Vec<K>) -> Result<()> {
        for p in &peers {
            if let Some(role) = self.parties.get_mut(p) {
                *role = r
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
    K: Encode<()>
        + for<'a> Decode<'a, ()>
        + Eq
        + Ord
        + Send
        + Clone
        + Copy
        + Display
        + Hash
        + 'static,
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
            .copied()
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
                    Ok(Ok((s, k))) => {
                        let Some(peer) = self.lookup_peer(&k) else {
                            info!(
                                name = %self.conf.name,
                                node = %self.conf.label,
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
                            self.spawn_io(k, s, peer.budget.clone())
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
                        Ok((id, (s, k))) => {
                            self.on_connect_task_end(id);
                            let Some(peer) = self.lookup_peer(&k) else {
                                warn!(
                                    name = %self.conf.name,
                                    node = %self.conf.label,
                                    addr = ?s.peer_addr().ok(),
                                    "connected to unknown peer"
                                );
                                continue
                            };
                            // We only keep the connection if our key is larger than the remote,
                            // or if we do not have a connection for that key at the moment.
                            if k < self.conf.label || !self.active.contains_key(&k) {
                                self.spawn_io(k, s, peer.budget.clone())
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
                        for (k, a) in peers {
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
                            self.connecting.remove(k);
                            self.active.remove(k);
                        }
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
                            if let Err(err) = self.ibound.try_send((self.conf.label, m, None)) {
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
                            if let Err(err) = self.ibound.try_send((self.conf.label, m.clone(), None)) {
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
                            if let Err(err) = self.ibound.try_send((self.conf.label, m.clone(), None)) {
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
        let p = self.peers.get(&k).expect("known peer");
        let h =
            self.connect_tasks
                .spawn(connect(self.conf.name, self.conf.label, k, p.addr.clone()));
        assert!(self.task2key.insert(h.id(), k).is_none());
        self.connecting.insert(k, ConnectTask { h });
    }

    /// Spawns a new `Noise` responder handshake task using the IK pattern.
    ///
    /// This function will create the responder handshake machine using its
    /// own private key and then spawn a task that awaits an initiator handshake
    /// to which it will respond.
    fn spawn_handshake(&mut self, s: TcpStream) {
        let ours = self.conf.label;
        self.handshake_tasks.spawn(async move {
            timeout(HANDSHAKE_TIMEOUT, on_handshake(ours, s))
                .await
                .or(Err(NetworkError::Timeout))?
        });
    }

    /// Spawns a new I/O task for handling communication with a remote peer over
    /// a TCP connection using the noise framework to create an authenticated
    /// secure link.
    fn spawn_io(&mut self, k: K, s: TcpStream, b: Budget) {
        debug!(
            name = %self.conf.name,
            node = %self.conf.label,
            peer = %k,
            addr = ?s.peer_addr().ok(),
            "starting i/o tasks"
        );
        let (to_remote, from_remote) = chan::channel(self.conf.peer_capacity_egress);
        let (r, w) = s.into_split();
        let ibound = self.ibound.clone();
        let to_write = to_remote.clone();
        let countdown = Countdown::new();
        let rh = self.io_tasks.spawn(recv_loop(
            self.conf.name,
            k,
            r,
            ibound,
            to_write,
            b,
            countdown.clone(),
        ));
        let wh = self.io_tasks.spawn(send_loop(w, from_remote, countdown));
        assert!(self.task2key.insert(rh.id(), k).is_none());
        assert!(self.task2key.insert(wh.id(), k).is_none());
        let io = IoTask {
            rh,
            wh,
            tx: to_remote,
        };
        self.active.insert(k, io);
    }

    /// Get the public key of a party by their static X25519 public key.
    fn lookup_peer(&self, k: &K) -> Option<&Peer> {
        self.peers.get(k)
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
async fn connect<K>(name: &'static str, this: K, to: K, addr: Address) -> (TcpStream, K)
where
    K: Display + Encode<()> + for<'a> Decode<'a, ()> + Copy + PartialEq + Send,
{
    use rand::prelude::*;

    let i = rand::rng().random_range(0..=1000);
    let addr = addr.to_string();

    for d in [i, 1000, 3000, 6000, 10_000, 15_000]
        .into_iter()
        .chain(repeat(30_000))
    {
        sleep(Duration::from_millis(d)).await;
        debug!(%name, node = %this, peer = %to, %addr, "connecting");
        match timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr)).await {
            Ok(Ok(s)) => {
                if let Err(err) = s.set_nodelay(true) {
                    error!(%name, node = %this, %err, "failed to set NO_DELAY socket option");
                    continue;
                }
                match timeout(HANDSHAKE_TIMEOUT, handshake(this, s)).await {
                    Ok(Ok((s, x))) if x == to => {
                        debug!(%name, node = %this, peer = %to, %addr, "connection established");
                        return (s, x);
                    },
                    Ok(Ok((_, x))) => {
                        error!(%name, node = %this, peer = %to, actual = %x, %addr, "peer id mismatch");
                    },
                    Ok(Err(err)) => {
                        warn!(%name, node = %this, peer = %to, %addr, %err, "handshake failure");
                    },
                    Err(_) => {
                        warn!(%name, node = %this, peer = %to, %addr, "handshake timeout");
                    },
                }
            },
            Ok(Err(err)) => {
                warn!(%name, node = %this, peer = %to, %addr, %err, "failed to connect");
            },
            Err(_) => {
                warn!(%name, node = %this, peer = %to, %addr, "connect timeout");
            },
        }
    }

    unreachable!("for loop repeats forever")
}

/// Perform a handshake as initiator with the remote party.
async fn handshake<K>(ours: K, mut stream: TcpStream) -> Result<(TcpStream, K)>
where
    K: Encode<()> + for<'a> Decode<'a, ()>,
{
    let hello = minicbor::to_vec(ours)?;
    send_frame(&mut stream, Header::data(hello.len() as u16), &hello).await?;
    let (h, m) = recv_frame(&mut stream).await?;
    if !h.is_data() || h.is_partial() {
        return Err(NetworkError::InvalidHandshakeMessage);
    }
    let theirs = minicbor::decode(&m)?;
    Ok((stream, theirs))
}

/// Perform a handshake as responder with a remote party.
async fn on_handshake<K>(ours: K, mut stream: TcpStream) -> Result<(TcpStream, K)>
where
    K: Encode<()> + for<'a> Decode<'a, ()>,
{
    stream.set_nodelay(true)?;
    let (h, m) = recv_frame(&mut stream).await?;
    if !h.is_data() || h.is_partial() {
        return Err(NetworkError::InvalidHandshakeMessage);
    }
    let theirs = minicbor::decode(&m)?;
    let hello = minicbor::to_vec(ours)?;
    send_frame(&mut stream, Header::data(hello.len() as u16), &hello).await?;
    Ok((stream, theirs))
}

/// Read messages from the remote by assembling frames together.
///
/// Once complete the message will be handed over to the given MPSC sender.
#[allow(clippy::too_many_arguments)]
async fn recv_loop<R, K>(
    name: &'static str,
    id: K,
    mut reader: R,
    to_deliver: Sender<(K, Bytes, Option<OwnedSemaphorePermit>)>,
    to_writer: chan::Sender<Message>,
    budget: Arc<Semaphore>,
    mut countdown: Countdown,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    K: Display + Copy,
{
    loop {
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
                                    if let Some(ping) = Timestamp::try_from_slice(&f) {
                                        to_writer.send(None, Message::Pong(ping))
                                    }
                                }
                                Ok(Type::Pong) => {
                                    if let Some(_) = Timestamp::try_from_slice(&f) {
                                        // update metrics
                                    }
                                }
                                Ok(Type::Data) => {
                                    msg.extend_from_slice(&f);
                                    if !h.is_partial() {
                                        break;
                                    }
                                    if msg.len() > MAX_MESSAGE_SIZE {
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
            .send((id, msg.freeze(), Some(permit)))
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
async fn send_loop<W>(mut writer: W, rx: chan::Receiver<Message>, ctr: Countdown) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    while let Some(msg) = rx.recv().await {
        match msg {
            Message::Ping(ping) => {
                let b = ping.to_bytes();
                let h = Header::ping(b.len() as u16);
                send_frame(&mut writer, h, &b).await?;
                ctr.start(REPLY_TIMEOUT)
            },
            Message::Pong(pong) => {
                let b = pong.to_bytes();
                let h = Header::pong(b.len() as u16);
                send_frame(&mut writer, h, &b).await?;
            },
            Message::Data(msg) => {
                let mut it = msg.chunks(MAX_PAYLOAD_SIZE).peekable();
                while let Some(m) = it.next() {
                    let h = if it.peek().is_some() {
                        Header::data(m.len() as u16).partial()
                    } else {
                        Header::data(m.len() as u16)
                    };
                    send_frame(&mut writer, h, &m).await?
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
async fn send_frame<W>(w: &mut W, hdr: Header, msg: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    debug_assert_eq!(usize::from(hdr.len()), msg.len());
    w.write_all(&hdr.to_bytes()).await?;
    w.write_all(msg).await?;
    Ok(())
}
