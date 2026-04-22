use std::collections::HashMap;

use cliquenet::{NetAddr, Role, Slot, x25519::PublicKey};
use hotshot::traits::NetworkError;
use hotshot_types::{
    PeerConnectInfo,
    data::ViewNumber,
    message::{EXTERNAL_MESSAGE_VERSION, UpgradeLock},
    traits::node_implementation::NodeType,
    x25519::Keypair,
};
use tracing::{error, info};

use crate::{
    message::{Message, Unchecked, Validated},
    network::{Network, PeerManagement, PeerRole},
};

pub struct Cliquenet<T: NodeType> {
    my_keys: (T::SignatureKey, PublicKey),
    inner: cliquenet::Network,
    peers: HashMap<T::SignatureKey, PublicKey>,
    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType> Cliquenet<T> {
    pub async fn create<A, P>(
        name: &'static str,
        singing_key: T::SignatureKey,
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

        let public_key = keypair.public_key();

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

        let net = cliquenet::Network::create(cfg)
            .await
            .map_err(|e| NetworkError::ListenError(format!("cliquenet creation failed: {e}")))?;

        info!(peers = %parties.len(), "cliquenet created");

        let peers = parties
            .into_iter()
            .map(|(k, info)| (k, info.x25519_key.into()))
            .collect();

        Ok(Self {
            my_keys: (singing_key, public_key.into()),
            inner: net,
            peers,
            upgrade_lock,
        })
    }
}

impl<T: NodeType> Network<T> for Cliquenet<T> {
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
            .map_err(|e| NetworkError::MessageSendError(format!("unicast failed: {e}")))?;
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
            .map_err(|e| NetworkError::MessageSendError(format!("multicast failed: {e}")))?;
        Ok(())
    }

    fn broadcast(&mut self, v: ViewNumber, m: &Message<T, Validated>) -> Result<(), NetworkError> {
        let bytes = self.serialize(m)?;
        self.inner
            .broadcast(Slot::new(*v), bytes)
            .map_err(|e| NetworkError::MessageSendError(format!("broadcast failed: {e}")))?;
        Ok(())
    }

    fn receive(&mut self) -> impl Future<Output = Result<Message<T, Unchecked>, NetworkError>> {
        async {
            let (_src, bytes) = self.inner.receive().await.ok_or_else(|| {
                NetworkError::MessageReceiveError("cliquenet receive channel closed".to_string())
            })?;
            let m = self.deserialize(&bytes)?;
            Ok(m)
        }
    }

    fn gc(&mut self, v: ViewNumber) -> Result<(), NetworkError> {
        self.inner
            .gc(Slot::new(*v))
            .map_err(|e| NetworkError::ConfigError(format!("gc failed: {e}")))?;
        Ok(())
    }
}

impl<T: NodeType> PeerManagement<T> for Cliquenet<T> {
    type Data = (PublicKey, NetAddr);

    fn add_peers(
        &mut self,
        r: PeerRole,
        ps: Vec<(T::SignatureKey, Self::Data)>,
    ) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for (k, (x, a)) in ps {
            self.peers.insert(k, x);
            targets.push((x, a))
        }
        self.inner
            .add_peers(map_peer_role(r), targets)
            .map_err(|e| NetworkError::ConfigError(format!("add_peers failed: {e}")))?;
        Ok(())
    }

    fn remove_peers(&mut self, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for k in ps {
            if let Some(x) = self.peers.remove(k) {
                targets.push(x)
            }
        }
        self.inner
            .remove_peers(targets)
            .map_err(|e| NetworkError::ConfigError(format!("remove_peers failed: {e}")))?;
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
            .map_err(|e| NetworkError::ConfigError(format!("assign_peers failed: {e}")))?;
        Ok(())
    }
}

impl<T: NodeType> Cliquenet<T> {
    fn serialize(&self, m: &Message<T, Validated>) -> Result<Vec<u8>, NetworkError> {
        self.upgrade_lock
            .serialize(m)
            .map_err(|e| NetworkError::FailedToSerialize(e.to_string()))
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<Message<T, Unchecked>, NetworkError> {
        match self
            .upgrade_lock
            .deserialize::<Message<T, Unchecked>>(bytes)
        {
            Ok((m, v)) => {
                if v == EXTERNAL_MESSAGE_VERSION && !m.is_external() {
                    let e = "received a non-external message with version 0.0".to_string();
                    return Err(NetworkError::FailedToDeserialize(e));
                }
                Ok(m)
            },
            Err(err) => Err(NetworkError::FailedToDeserialize(err.to_string())),
        }
    }
}

fn map_peer_role(r: PeerRole) -> Role {
    match r {
        PeerRole::Active => Role::Active,
        PeerRole::Passive => Role::Passive,
    }
}
