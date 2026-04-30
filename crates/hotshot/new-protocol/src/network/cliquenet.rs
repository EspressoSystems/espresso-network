use std::collections::HashMap;

use cliquenet::{NetAddr, NetworkError as CliquenetError, Role, Slot, x25519::PublicKey};
use hotshot_types::{
    PeerConnectInfo,
    data::ViewNumber,
    message::{EXTERNAL_MESSAGE_VERSION, MessageKind, UpgradeLock},
    traits::node_implementation::NodeType,
    x25519::Keypair,
};
use tracing::{error, info};

use crate::{
    message::{Message, MessageType, Unchecked, Validated},
    network::{Network, NetworkError, PeerRole},
};

pub struct Cliquenet<T: NodeType> {
    my_keys: (T::SignatureKey, PublicKey),
    inner: cliquenet::Network,
    peers: HashMap<T::SignatureKey, PublicKey>,
    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType> Cliquenet<T> {
    pub async fn create<A, P>(
        name: &str,
        signing_key: T::SignatureKey,
        keypair: Keypair,
        addr: A,
        parties: P,
        upgrade_lock: UpgradeLock<T>,
    ) -> Result<Self, NetworkError>
    where
        A: Into<cliquenet::NetAddr>,
        P: IntoIterator<Item = (T::SignatureKey, PeerConnectInfo)>,
    {
        let parties: HashMap<T::SignatureKey, PeerConnectInfo> = parties.into_iter().collect();

        let cfg = cliquenet::Config::builder()
            .name(name)
            .keypair(keypair.into())
            .bind(addr.into())
            .parties(
                parties
                    .values()
                    .map(|info| (info.x25519_key.into(), info.p2p_addr.clone())),
            )
            .build();

        Self::create_with_config(signing_key, upgrade_lock, cfg, parties).await
    }

    pub async fn create_with_config<P>(
        signing_key: T::SignatureKey,
        upgrade_lock: UpgradeLock<T>,
        config: cliquenet::Config,
        parties: P,
    ) -> Result<Self, NetworkError>
    where
        P: IntoIterator<Item = (T::SignatureKey, PeerConnectInfo)>,
    {
        let public_key = config.public_key();
        let network = cliquenet::Network::create(config)
            .await
            .map_err(to_network_error)?;

        let peers: HashMap<_, _> = parties
            .into_iter()
            .map(|(k, info)| (k, info.x25519_key.into()))
            .collect();

        info!(peers = %peers.len(), "cliquenet created");

        Ok(Self {
            my_keys: (signing_key, public_key),
            inner: network,
            peers,
            upgrade_lock,
        })
    }
}

impl<T: NodeType> Network<T> for Cliquenet<T> {
    type PeerData = (PublicKey, NetAddr);

    fn unicast(
        &mut self,
        v: ViewNumber,
        to: &T::SignatureKey,
        m: &Message<T, Validated>,
    ) -> Result<(), NetworkError> {
        let target = if *to == self.my_keys.0 {
            self.my_keys.1
        } else if let Some(target) = self.peers.get(to) {
            *target
        } else {
            error!(peer = %to, "unicast target not found");
            return Ok(());
        };
        let bytes = self.serialize(m)?;
        self.inner
            .unicast(Slot::new(*v), target, bytes)
            .map_err(to_network_error)?;
        Ok(())
    }

    fn multicast(
        &mut self,
        v: ViewNumber,
        to: Vec<&T::SignatureKey>,
        m: &Message<T, Validated>,
    ) -> Result<(), NetworkError> {
        let bytes = self.serialize(m)?;
        let mut targets = Vec::new();
        for t in to {
            if let Some(target) = self.peers.get(t) {
                targets.push(*target)
            } else if *t == self.my_keys.0 {
                targets.push(self.my_keys.1)
            } else {
                error!(peer = %t, "multicast target not found");
            }
        }
        self.inner
            .multicast(Slot::new(*v), targets, bytes)
            .map_err(to_network_error)?;
        Ok(())
    }

    fn broadcast(&mut self, v: ViewNumber, m: &Message<T, Validated>) -> Result<(), NetworkError> {
        let bytes = self.serialize(m)?;
        self.inner
            .broadcast(Slot::new(*v), bytes)
            .map_err(to_network_error)?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Message<T, Unchecked>, NetworkError> {
        let (_src, bytes) = self
            .inner
            .receive()
            .await
            .ok_or_else(|| NetworkError::Critical("cliquenet has shutdown".into()))?;
        let m = self.deserialize(&bytes)?;
        Ok(m)
    }

    fn gc(&mut self, v: ViewNumber) -> Result<(), NetworkError> {
        self.inner.gc(Slot::new(*v)).map_err(to_network_error)
    }

    fn add_peers(
        &mut self,
        r: PeerRole,
        ps: Vec<(T::SignatureKey, Self::PeerData)>,
    ) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for (k, (x, a)) in ps {
            self.peers.insert(k, x);
            targets.push((x, a))
        }
        self.inner
            .add_peers(map_peer_role(r), targets)
            .map_err(to_network_error)?;
        Ok(())
    }

    fn remove_peers(&mut self, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for k in ps {
            if let Some(x) = self.peers.remove(k) {
                targets.push(x)
            }
        }
        self.inner.remove_peers(targets).map_err(to_network_error)?;
        Ok(())
    }

    fn assign_role(&mut self, r: PeerRole, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for k in ps {
            if let Some(x) = self.peers.get(k) {
                targets.push(*x)
            }
        }
        self.inner
            .assign_peers(map_peer_role(r), targets)
            .map_err(to_network_error)?;
        Ok(())
    }
}

impl<T: NodeType> Cliquenet<T> {
    fn serialize(&self, m: &Message<T, Validated>) -> Result<Vec<u8>, NetworkError> {
        self.upgrade_lock
            .serialize(m)
            .map_err(|e| NetworkError::Io(format!("serialization error: {e}").into()))
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<Message<T, Unchecked>, NetworkError> {
        match self
            .upgrade_lock
            .deserialize::<Message<T, Unchecked>>(bytes)
        {
            Ok((m, v)) => {
                if v == EXTERNAL_MESSAGE_VERSION && !m.is_external() {
                    let e = "received a non-external message with version 0.0".to_string();
                    return Err(NetworkError::Io(e.into()));
                }
                Ok(m)
            },
            Err(primary_err) => {
                // Fallback: bytes may be a hotshot-types `Message<T>` carrying
                // an `External` payload (this is how `Leaf2Fetcher` in the
                // membership layer frames leaf-catchup requests/responses).
                // If so, surface it as `MessageType::External` so the
                // Coordinator can route it to the membership external
                // channel just like a native new-protocol external message.
                if let Ok((_v, hs_msg)) =
                    versions::decode::<hotshot_types::message::Message<T>>(bytes)
                    && let MessageKind::External(data) = hs_msg.kind
                {
                    return Ok(Message {
                        sender: hs_msg.sender,
                        message_type: MessageType::External(data),
                    });
                }
                Err(NetworkError::Io(primary_err.to_string().into()))
            },
        }
    }
}

fn map_peer_role(r: PeerRole) -> Role {
    match r {
        PeerRole::Active => Role::Active,
        PeerRole::Passive => Role::Passive,
    }
}

fn to_network_error(e: CliquenetError) -> NetworkError {
    match e {
        e @ CliquenetError::Bind(..) => NetworkError::Critical(e.into()),
        e @ CliquenetError::ChannelClosed => NetworkError::Critical(e.into()),
        e @ CliquenetError::BudgetClosed => NetworkError::Critical(e.into()),
        e => NetworkError::Io(e.into()),
    }
}
