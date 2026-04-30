pub mod peer;
pub mod server;

use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use bon::Builder;
use bytes::Bytes;
use tokio::{
    net::TcpListener,
    sync::{
        OwnedSemaphorePermit,
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, watch,
    },
    task::JoinHandle,
};
use tracing::{debug, info, warn};

use crate::{
    Config, Role, addr::NetAddr, error::NetworkError, msg::Slot, net::server::Server,
    x25519::PublicKey,
};

type PeerMessage = (PublicKey, Bytes, Option<OwnedSemaphorePermit>);

#[derive(Debug)]
pub struct Network {
    recv: NetworkReceiver,
    ctrl: NetworkController,
}

#[derive(Debug)]
pub struct NetworkReceiver {
    rx: UnboundedReceiver<PeerMessage>,
}

#[derive(Debug)]
pub struct NetworkController {
    conf: Arc<Config>,
    node: PublicKey,
    parties: HashMap<PublicKey, Role>,
    tx: UnboundedSender<Command>,
    next_slot: watch::Sender<Slot>,
    lower_bound: Slot,
    task: JoinHandle<()>,
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

        let _addr = listener.local_addr()?;
        let node = conf.keypair.public_key();
        let parties = HashMap::from_iter(conf.parties.iter().map(|(k, _)| (*k, Role::Active)));

        // Command channel from application to network.
        let (otx, orx) = mpsc::unbounded_channel();

        // Channel of messages from peers to the application.
        let (itx, irx) = mpsc::unbounded_channel();

        let (etx, erx) = watch::channel(Slot::MIN);

        let conf = Arc::new(conf);
        let serv = Server::spawn(conf.clone(), listener, Role::Active, itx, orx, erx);
        let recv = NetworkReceiver { rx: irx };
        let ctrl = NetworkController {
            conf: conf.clone(),
            node,
            parties,
            tx: otx,
            task: serv,
            next_slot: etx,
            lower_bound: Slot::MIN,
        };

        info!(name = %conf.name, %node, addr = %_addr, "listening");

        Ok(Self { recv, ctrl })
    }

    pub fn controller(&mut self) -> &mut NetworkController {
        &mut self.ctrl
    }

    pub fn receiver(&mut self) -> &mut NetworkReceiver {
        &mut self.recv
    }

    pub fn split_into(self) -> (NetworkController, NetworkReceiver) {
        (self.ctrl, self.recv)
    }

    pub async fn receive(&mut self) -> Option<(PublicKey, Bytes)> {
        self.recv.receive().await
    }
}

impl Deref for Network {
    type Target = NetworkController;

    fn deref(&self) -> &Self::Target {
        &self.ctrl
    }
}

impl DerefMut for Network {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctrl
    }
}

impl NetworkReceiver {
    /// Receive the next incoming message.
    ///
    /// The returned public key denotes the source where the message came from.
    pub async fn receive(&mut self) -> Option<(PublicKey, Bytes)> {
        let (k, b, _) = self.rx.recv().await?;
        Some((k, b))
    }
}

impl NetworkController {
    /// Iterate over all parties.
    pub fn parties(&self) -> impl Iterator<Item = (&PublicKey, &Role)> {
        self.parties.iter()
    }

    /// Send a message to a party, identified by the given public key.
    pub fn unicast(&mut self, s: Slot, to: PublicKey, msg: Vec<u8>) -> Result<(), NetworkError> {
        debug!(slot = %s, %to, "unicast");
        self.length_check(&msg)?;
        let cmd = SendCommand::builder()
            .slot(s)
            .action(SendAction::Unicast(to, msg))
            .build();
        self.tx
            .send(Command::Send(cmd))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Send a message to all parties.
    pub fn broadcast(&mut self, s: Slot, msg: Vec<u8>) -> Result<(), NetworkError> {
        debug!(slot = %s, "broadcast");
        self.length_check(&msg)?;
        let cmd = SendCommand::builder()
            .slot(s)
            .action(SendAction::Broadcast(msg))
            .build();
        self.tx
            .send(Command::Send(cmd))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Send a message to several parties, identified by their public keys.
    pub fn multicast<P>(&mut self, s: Slot, to: P, msg: Vec<u8>) -> Result<(), NetworkError>
    where
        P: IntoIterator<Item = PublicKey>,
    {
        debug!(slot = %s, "multicast");
        self.length_check(&msg)?;
        let cmd = SendCommand::builder()
            .slot(s)
            .action(SendAction::Multicast(to.into_iter().collect(), msg))
            .build();
        self.tx
            .send(Command::Send(cmd))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// General send operation, supporting custom retry policies.
    pub fn send(&mut self, cmd: SendCommand) -> Result<(), NetworkError> {
        debug!(slot = %cmd.slot, "send");
        self.length_check(msg_bytes(&cmd))?;
        self.tx
            .send(Command::Send(cmd))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Add the given peers to the network.
    pub fn add_peers<P>(&mut self, r: Role, peers: P) -> Result<(), NetworkError>
    where
        P: IntoIterator<Item = (PublicKey, NetAddr)>,
    {
        debug!(role = %r, "add_peers");
        let peers = peers.into_iter().collect::<Vec<_>>();
        self.parties.extend(peers.iter().map(|(p, ..)| (*p, r)));
        self.tx
            .send(Command::Peer(PeerCommand::Add(r, peers)))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Remove the given peers from the network.
    pub fn remove_peers<P>(&mut self, peers: P) -> Result<(), NetworkError>
    where
        P: IntoIterator<Item = PublicKey>,
    {
        debug!("remove_peers");
        let peers = peers.into_iter().collect::<Vec<_>>();
        for p in &peers {
            self.parties.remove(p);
        }
        self.tx
            .send(Command::Peer(PeerCommand::Remove(peers)))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    /// Assign the given role to the given peers.
    pub fn assign_peers<P>(&mut self, r: Role, peers: P) -> Result<(), NetworkError>
    where
        P: IntoIterator<Item = PublicKey>,
    {
        debug!(role = %r, "assign_peers");
        let peers = peers.into_iter().collect::<Vec<_>>();
        for p in &peers {
            if let Some(role) = self.parties.get_mut(p) {
                *role = r
            }
        }
        self.tx
            .send(Command::Peer(PeerCommand::Assign(r, peers)))
            .map_err(|_| NetworkError::ChannelClosed)
    }

    pub fn gc(&mut self, s: Slot) -> Result<(), NetworkError> {
        debug!(slot = %s, "gc");
        if s <= self.lower_bound {
            return Ok(());
        }
        self.next_slot
            .send(s)
            .map_err(|_| NetworkError::ChannelClosed)?;
        self.lower_bound = s;
        Ok(())
    }

    /// Trigger network shutdown.
    ///
    /// The returned future will resolve once the server task finished.
    pub fn shutdown(&mut self) -> Result<impl Future<Output = ()> + use<>, NetworkError> {
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
}

fn msg_bytes(cmd: &SendCommand) -> &[u8] {
    match &cmd.action {
        SendAction::Unicast(_, b) => b,
        SendAction::Multicast(_, b) => b,
        SendAction::Broadcast(b) => b,
    }
}

impl Drop for NetworkController {
    fn drop(&mut self) {
        self.task.abort();
    }
}
