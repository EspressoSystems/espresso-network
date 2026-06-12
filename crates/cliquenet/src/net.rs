pub mod peer;
pub mod server;

use std::{fmt, ops::Deref, sync::Arc};

use bon::Builder;
use bytes::Bytes;
use tokio::{
    net::TcpListener,
    sync::{
        OwnedSemaphorePermit,
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, watch,
    },
};
use tracing::{debug, info, warn};

use crate::{
    Config, Metrics, Role, addr::NetAddr, error::NetworkError, metrics::NoMetrics, msg::Slot,
    net::server::Server, x25519::PublicKey,
};

type PeerMessage = (PublicKey, Bytes, Option<OwnedSemaphorePermit>);

#[derive(Debug)]
pub struct Network {
    recv: NetworkReceiver,
    send: NetworkSender,
}

#[derive(Debug)]
pub struct NetworkReceiver {
    rx: UnboundedReceiver<PeerMessage>,
}

#[derive(Clone)]
pub struct NetworkSender {
    conf: Arc<Config>,
    node: PublicKey,
    tx: UnboundedSender<Command>,
    next_slot: watch::Sender<Slot>,
    metrics: Arc<dyn Metrics>,
}

impl fmt::Debug for NetworkSender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetworkSender")
            .field("node", &self.node)
            .field("lower_bound", &*self.next_slot.borrow())
            .field("conf", &self.conf)
            .finish()
    }
}

/// Server task instructions.
#[derive(Debug)]
enum Command {
    Peer(PeerCommand),
    Send(SendCommand),
    Shutdown(oneshot::Sender<()>),
}

/// Update network peers.
#[derive(Debug)]
enum PeerCommand {
    /// Add the given peers.
    Add(Role, Vec<(PublicKey, NetAddr)>),
    /// Remove the given peers.
    Remove(Vec<PublicKey>),
    /// Assign a `Role` to the given peers.
    Assign(Role, Vec<PublicKey>),
}

/// Send to peer(s).
#[derive(Clone, Debug, Builder)]
pub struct SendCommand {
    slot: Slot,
    action: SendAction,
    #[builder(default)]
    retry: RetryPolicy,
}

/// Specify if a message should be retried if no ACK is received.
#[derive(Clone, Copy, Debug, Default)]
pub enum RetryPolicy {
    #[default]
    Default,
    NoRetry,
}

impl RetryPolicy {
    pub fn is_retry(self) -> bool {
        matches!(self, Self::Default)
    }
}

#[derive(Clone, Debug)]
pub enum SendAction {
    /// Send a message to one peer.
    Unicast(PublicKey, Vec<u8>),
    /// Send a message to some peers.
    Multicast(Vec<PublicKey>, Vec<u8>),
    /// Send a message to all peers with `Role::Active`.
    Broadcast(Vec<u8>),
}

impl Network {
    pub async fn create(conf: Config) -> Result<Self, NetworkError> {
        let listener = TcpListener::bind(conf.bind.to_string())
            .await
            .map_err(|e| NetworkError::Bind(conf.bind.clone(), e))?;

        let addr = listener.local_addr()?;
        let node = conf.keypair.public_key();

        // Command channel from application to network.
        let (otx, orx) = mpsc::unbounded_channel();

        // Channel of messages from peers to the application.
        let (itx, irx) = mpsc::unbounded_channel();

        let (etx, erx) = watch::channel(Slot::MIN);

        let metr = conf.metrics.clone().unwrap_or_else(|| Arc::new(NoMetrics));
        let conf = Arc::new(conf);

        // The server ends when all `NetworkSender`s are dropped.
        Server::spawn(
            conf.clone(),
            listener,
            Role::Active,
            itx,
            orx,
            erx,
            metr.clone(),
        );

        let recv = NetworkReceiver { rx: irx };
        let send = NetworkSender {
            conf: conf.clone(),
            node,
            tx: otx,
            next_slot: etx,
            metrics: metr,
        };

        info!(name = %conf.name, %node, %addr, "listening");

        Ok(Self { recv, send })
    }

    pub fn sender(&self) -> &NetworkSender {
        &self.send
    }

    pub fn receiver(&self) -> &NetworkReceiver {
        &self.recv
    }

    pub fn receiver_mut(&mut self) -> &mut NetworkReceiver {
        &mut self.recv
    }

    pub fn split_into(self) -> (NetworkSender, NetworkReceiver) {
        (self.send, self.recv)
    }

    pub async fn receive(&mut self) -> Option<(PublicKey, Bytes)> {
        self.recv.receive().await
    }
}

impl Deref for Network {
    type Target = NetworkSender;

    fn deref(&self) -> &Self::Target {
        &self.send
    }
}

impl NetworkReceiver {
    /// Receive the next incoming message.
    ///
    /// The returned public key denotes the source where the message came from.
    pub async fn receive(&mut self) -> Option<(PublicKey, Bytes)> {
        let (k, b, _) = self.rx.recv().await?;
        debug!(peer = %k, len = b.len(), "message received");
        Some((k, b))
    }
}

impl NetworkSender {
    pub fn config(&self) -> &Config {
        &self.conf
    }

    /// Send a message to a party, identified by the given public key.
    pub fn unicast(&self, s: Slot, to: PublicKey, msg: Vec<u8>) -> Result<(), NetworkError> {
        debug!(slot = %s, %to, len = msg.len(), "unicast");
        self.length_check(&msg)?;
        if self.lt_lower_bound(s) {
            return Ok(());
        }
        let cmd = SendCommand::builder()
            .slot(s)
            .action(SendAction::Unicast(to, msg))
            .build();
        self.tx
            .send(Command::Send(cmd))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Send a message to all parties.
    pub fn broadcast(&self, s: Slot, msg: Vec<u8>) -> Result<(), NetworkError> {
        debug!(slot = %s, len = msg.len(), "broadcast");
        self.length_check(&msg)?;
        if self.lt_lower_bound(s) {
            return Ok(());
        }
        let cmd = SendCommand::builder()
            .slot(s)
            .action(SendAction::Broadcast(msg))
            .build();
        self.tx
            .send(Command::Send(cmd))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Send a message to several parties, identified by their public keys.
    pub fn multicast<P>(&self, s: Slot, to: P, msg: Vec<u8>) -> Result<(), NetworkError>
    where
        P: IntoIterator<Item = PublicKey>,
    {
        debug!(slot = %s, len = msg.len(), "multicast");
        self.length_check(&msg)?;
        if self.lt_lower_bound(s) {
            return Ok(());
        }
        let cmd = SendCommand::builder()
            .slot(s)
            .action(SendAction::Multicast(to.into_iter().collect(), msg))
            .build();
        self.tx
            .send(Command::Send(cmd))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// General send operation, supporting custom retry policies.
    pub fn send(&self, cmd: SendCommand) -> Result<(), NetworkError> {
        let bytes = msg_bytes(&cmd);
        debug!(slot = %cmd.slot, len = %bytes.len(), "send");
        self.length_check(bytes)?;
        if self.lt_lower_bound(cmd.slot) {
            return Ok(());
        }
        self.tx
            .send(Command::Send(cmd))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Add the given peers to the network.
    pub fn add_peers<P>(&self, r: Role, peers: P) -> Result<(), NetworkError>
    where
        P: IntoIterator<Item = (PublicKey, NetAddr)>,
    {
        debug!(role = %r, "add_peers");
        let peers = peers.into_iter().collect::<Vec<_>>();
        self.tx
            .send(Command::Peer(PeerCommand::Add(r, peers)))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Remove the given peers from the network.
    pub fn remove_peers<P>(&self, peers: P) -> Result<(), NetworkError>
    where
        P: IntoIterator<Item = PublicKey>,
    {
        debug!("remove_peers");
        let peers = peers.into_iter().collect::<Vec<_>>();
        for p in &peers {
            self.metrics.del(p);
        }
        self.tx
            .send(Command::Peer(PeerCommand::Remove(peers)))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Assign the given role to the given peers.
    pub fn assign_peers<P>(&self, r: Role, peers: P) -> Result<(), NetworkError>
    where
        P: IntoIterator<Item = PublicKey>,
    {
        debug!(role = %r, "assign_peers");
        let peers = peers.into_iter().collect::<Vec<_>>();
        self.tx
            .send(Command::Peer(PeerCommand::Assign(r, peers)))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Trigger garbage collection of messages below the given slot.
    pub fn gc(&self, s: Slot) -> Result<(), NetworkError> {
        debug!(slot = %s, "gc");
        if self.next_slot.is_closed() {
            return Err(NetworkError::ChannelClosed);
        }
        self.next_slot.send_if_modified(|lower_bound| {
            if s > *lower_bound {
                *lower_bound = s;
                true
            } else {
                false
            }
        });
        Ok(())
    }

    /// Trigger network shutdown.
    ///
    /// The returned future will resolve once the server task finished.
    pub fn shutdown(&self) -> Result<impl Future<Output = ()> + use<>, NetworkError> {
        debug!("shutdown");
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Command::Shutdown(tx))
            .map_err(|_| NetworkError::ChannelClosed)?;
        Ok(async move {
            let _ = rx.await;
        })
    }

    /// Check the number of message bytes does not exceed the configured maximum.
    fn length_check(&self, msg: &[u8]) -> Result<(), NetworkError> {
        if msg.len() > self.conf.max_message_size.get() {
            warn!(
                name = %self.conf.name,
                node = %self.node,
                len  = %msg.len(),
                max  = %self.conf.max_message_size,
                "message too large to send"
            );
            return Err(NetworkError::MessageTooLarge);
        }
        Ok(())
    }

    /// Check if the given slot is less than our lower bound.
    fn lt_lower_bound(&self, s: Slot) -> bool {
        let lower_bound = *self.next_slot.borrow();
        if s < lower_bound {
            warn!(
                name = %self.conf.name,
                node = %self.node,
                slot = %s,
                %lower_bound,
                "slot below lower bound"
            );
            return true;
        }
        false
    }
}

fn msg_bytes(cmd: &SendCommand) -> &[u8] {
    match &cmd.action {
        SendAction::Unicast(_, b) => b,
        SendAction::Multicast(_, b) => b,
        SendAction::Broadcast(b) => b,
    }
}
