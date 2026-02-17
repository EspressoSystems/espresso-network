#[cfg(feature = "hotshot-testing")]
use std::time::Duration;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use cliquenet::{NetConf, Retry, Role};
#[cfg(feature = "hotshot-testing")]
use hotshot_types::traits::network::{
    AsyncGenerator, NetworkReliability, TestableNetworkingImplementation,
};
use hotshot_types::{
    addr::NetAddr,
    boxed_sync,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        metrics::Metrics,
        network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
        node_implementation::{ConsensusTime, NodeType},
        signature_key::{PrivateSignatureKey, SignatureKey, StakeTableEntryType},
    },
    x25519::{Keypair, PublicKey, SecretKey},
    BoxSyncFuture,
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct Cliquenet<K> {
    net: Retry<K>,
    inner: Arc<Mutex<Inner>>,
}

#[derive(Clone)]
struct Inner {
    epoch: EpochNumber,
}

impl<K: SignatureKey + 'static> Cliquenet<K> {
    pub async fn create<A, B, P>(
        name: &'static str,
        key: K,
        keypair: Keypair,
        addr: A,
        parties: P,
        metrics: Box<dyn Metrics>,
    ) -> Result<Self, NetworkError>
    where
        A: Into<NetAddr>,
        B: Into<NetAddr>,
        P: IntoIterator<Item = (K, PublicKey, B)>,
    {
        let cfg = NetConf::builder()
            .name(name)
            .label(key)
            .keypair(keypair)
            .bind(addr.into())
            .parties(parties.into_iter().map(|(k, x, a)| (k, x, a.into())))
            .metrics(metrics)
            .build();
        let net = Retry::create(cfg)
            .await
            .map_err(|e| NetworkError::ListenError(format!("cliquenet creation failed: {e}")))?;
        Ok(Self {
            net,
            inner: Arc::new(Mutex::new(Inner {
                epoch: EpochNumber::genesis(),
            })),
        })
    }

    /// Get the current network peers.
    pub fn peers(&self) -> Vec<K> {
        self.net.parties(None)
    }

    /// Handle an epoch change.
    ///
    /// We are at epoch e and if this method is applied to an epoch e' (> e),
    /// we perform the following steps:
    ///
    /// - Add new peers from epoch e' + 1.
    /// - Remove peers which are in e but not e'.
    async fn on_epoch_change<U>(&self, epoch: EpochNumber, coord: &EpochMembershipCoordinator<U>)
    where
        U: NodeType<SignatureKey = K>,
    {
        let mut inner = self.inner.lock().await;

        if epoch <= inner.epoch {
            return;
        }

        let current_peers: HashSet<K> = HashSet::from_iter(self.peers());

        // Remove current peers which are not in this new epoch:

        if let Ok(membership) = coord.stake_table_for_epoch(None).await {
            let parties: HashSet<_> = HashSet::from_iter(
                membership
                    .stake_table()
                    .await
                    .0
                    .into_iter()
                    .map(|m| m.stake_table_entry.public_key()),
            );
            let mut to_del = Vec::new();
            for p in &current_peers {
                if !parties.contains(p) {
                    debug!(%epoch, peer = %p, "removing network peer");
                    to_del.push(p.clone())
                }
            }
            if let Err(err) = self.net.remove(to_del).await {
                error!(%epoch, %err, "could not remove peers from network");
            }
        } else {
            warn!(%epoch, "no stake table available");
        }

        // Add peers from the next epoch:

        let next_epoch = <<U as NodeType>::Epoch as ConsensusTime>::new(u64::from(epoch) + 1);

        if let Ok(membership) = coord.stake_table_for_epoch(Some(next_epoch)).await {
            let stake_tbl: HashMap<_, _> = HashMap::from_iter(
                membership
                    .stake_table()
                    .await
                    .0
                    .into_iter()
                    .map(|m| (m.stake_table_entry.public_key(), (m.x25519_key, m.p2p_addr))),
            );
            let mut to_add = Vec::new();
            for (k, v) in stake_tbl {
                if current_peers.contains(&k) {
                    continue;
                }
                let (Some(x25519), Some(addr)) = v else {
                    info!(%epoch, peer = %k, "ignoring peer without x25519 key or p2p address");
                    continue;
                };
                debug!(%epoch, peer = %k, "adding network peer");
                to_add.push((k, x25519, addr))
            }
            if let Err(err) = self.net.add(Role::Active, to_add).await {
                error!(%epoch, %err, "could not add peers to network");
            }
        } else {
            warn!(epoch = %next_epoch, "no stake table available");
        }

        inner.epoch = epoch
    }
}

pub fn derive_keypair<K: SignatureKey>(k: &K::PrivateKey) -> Keypair {
    SecretKey::from(blake3::derive_key("cliquenet key", &k.to_bytes())).into()
}

#[async_trait]
impl<K: SignatureKey + 'static> ConnectedNetwork<K> for Cliquenet<K> {
    async fn broadcast_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        _: Topic,
        _: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        self.net.broadcast(*v, m).await.map_err(|e| {
            NetworkError::MessageSendError(format!("cliquenet broadcast error: {e}"))
        })?;
        Ok(())
    }

    async fn da_broadcast_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipients: Vec<K>,
        _: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        self.net.multicast(recipients, *v, m).await.map_err(|e| {
            NetworkError::MessageSendError(format!("cliquenet da_broadcast error: {e}"))
        })?;
        Ok(())
    }

    async fn direct_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipient: K,
    ) -> Result<(), NetworkError> {
        self.net
            .unicast(recipient, *v, m)
            .await
            .map_err(|e| NetworkError::MessageSendError(format!("cliquenet unicast error: {e}")))?;
        Ok(())
    }

    async fn recv_message(&self) -> Result<Vec<u8>, NetworkError> {
        let (_src, data) =
            self.net.receive().await.map_err(|e| {
                NetworkError::MessageSendError(format!("cliquenet receive error: {e}"))
            })?;
        Ok(Vec::from(&data[..]))
    }

    async fn update_view<U>(
        &self,
        v: ViewNumber,
        e: Option<EpochNumber>,
        m: EpochMembershipCoordinator<U>,
    ) where
        U: NodeType<SignatureKey = K>,
    {
        self.net.gc(*v);

        if let Some(e) = e {
            self.on_epoch_change(e, &m).await
        }
    }

    async fn wait_for_ready(&self) {}

    fn pause(&self) {}

    fn resume(&self) {}

    fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
    where
        'a: 'b,
        Self: 'b,
    {
        boxed_sync(self.net.close())
    }
}

#[cfg(feature = "hotshot-testing")]
impl<T: NodeType> TestableNetworkingImplementation<T> for Cliquenet<T::SignatureKey> {
    fn generator(
        nodes: usize,
        _num_bootstrap: usize,
        _network_id: usize,
        _da_committee_size: usize,
        _reliability_config: Option<Box<dyn NetworkReliability>>,
        _secondary_network_delay: Duration,
    ) -> AsyncGenerator<Arc<Self>> {
        let parties = {
            let p = gen_parties::<T::SignatureKey>()
                .take(nodes)
                .collect::<Vec<_>>();
            Arc::new(p)
        };
        Box::pin(move |i| {
            let parties = parties.clone();
            let future = async move {
                use hotshot_types::traits::metrics::NoMetrics;

                let (s, k, a) = &parties[i as usize];
                let it = parties
                    .iter()
                    .map(|(s, k, a)| (k.clone(), s.public_key(), a.clone()));
                let met = Box::new(NoMetrics);
                let net = Cliquenet::create("test", k.clone(), s.clone(), a.clone(), it, met)
                    .await
                    .unwrap();
                Arc::new(net)
            };
            Box::pin(future)
        })
    }

    fn in_flight_message_count(&self) -> Option<usize> {
        None
    }
}

/// Generate an arbitrary number or network parties.
///
/// A party is defined by its X25519 keypair, public signing key and network address.
#[cfg(feature = "hotshot-testing")]
fn gen_parties<K: SignatureKey>() -> impl Iterator<Item = (Keypair, K, NetAddr)> {
    let mut i = 0u64;
    std::iter::repeat_with(move || {
        let secret = K::generated_from_seed_indexed([0u8; 32], i).1;
        let public = K::from_private(&secret);
        let kpair = derive_keypair::<K>(&secret);
        let port =
            test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
        let addr = NetAddr::Inet(std::net::Ipv4Addr::LOCALHOST.into(), port);
        i += 1;
        (kpair, public, addr)
    })
}
