#[cfg(feature = "hotshot-testing")]
use std::time::Duration;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use cliquenet::{NetConf, NetworkDown, Retry, Role};
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
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    x25519::{Keypair, PublicKey},
    BoxSyncFuture,
};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct Cliquenet<K> {
    net: Retry<K>,
    inner: Arc<Mutex<Inner<K>>>,
}

#[derive(Clone)]
struct Inner<K> {
    epoch: EpochNumber,
    peers: HashMap<K, (PublicKey, NetAddr)>,
    non_peers: HashSet<K>,
}

impl<K: SignatureKey + 'static> Cliquenet<K> {
    pub async fn create<A, B, P, Q>(
        name: &'static str,
        key: K,
        keypair: Keypair,
        addr: A,
        parties: P,
        others: Q,
        metrics: Box<dyn Metrics>,
    ) -> Result<Self, NetworkError>
    where
        A: Into<NetAddr>,
        B: Into<NetAddr>,
        P: IntoIterator<Item = (K, PublicKey, B)>,
        Q: IntoIterator<Item = K>,
    {
        let parties: HashMap<K, (PublicKey, NetAddr)> = parties
            .into_iter()
            .map(|(k, x, a)| (k, (x, a.into())))
            .collect();

        let others: HashSet<K> = HashSet::from_iter(others);

        let cfg = NetConf::builder()
            .name(name)
            .label(key)
            .keypair(keypair)
            .bind(addr.into())
            .parties(parties.iter().map(|(k, (x, a))| (k.clone(), *x, a.clone())))
            .metrics(metrics)
            .build();

        let net = Retry::create(cfg)
            .await
            .map_err(|e| NetworkError::ListenError(format!("cliquenet creation failed: {e}")))?;

        info!(peers = %parties.len(), non_peers = %others.len(), "cliquenet created");

        Ok(Self {
            net,
            inner: Arc::new(Mutex::new(Inner {
                epoch: EpochNumber::genesis(),
                peers: parties,
                non_peers: others,
            })),
        })
    }

    /// Get the current network peers.
    pub fn peers(&self) -> Vec<K> {
        self.net.parties(None)
    }

    /// Get keys of peers not in this network.
    pub async fn non_peers(&self) -> HashSet<K> {
        self.inner.lock().await.non_peers.clone()
    }

    async fn on_epoch_change<U>(&self, epoch: EpochNumber, coord: &EpochMembershipCoordinator<U>)
    where
        U: NodeType<SignatureKey = K>,
    {
        let mut inner = self.inner.lock().await;

        if epoch <= inner.epoch {
            info!(%epoch, ours = %inner.epoch, "epoch <= ours");
            return;
        }

        let epoch = <<U as NodeType>::Epoch as ConsensusTime>::new(u64::from(epoch));

        let mut non_peers = HashSet::new();
        let mut to_add = Vec::new();
        let mut to_del = Vec::new();

        let Ok(membership) = coord.stake_table_for_epoch(Some(epoch)).await else {
            error!(%epoch, ours = %inner.epoch, "no stake table available");
            return;
        };

        let stake_tbl: HashMap<_, _> = HashMap::from_iter(
            membership
                .stake_table()
                .await
                .0
                .into_iter()
                .map(|m| (m.stake_table_entry.public_key(), (m.x25519_key, m.p2p_addr))),
        );

        // Collect peers to add or update, i.e. stake table members which are
        // not already network peers.
        for (k, v) in stake_tbl.iter() {
            info!(%epoch, peer = %k, "checking stake table member");
            let (Some(x25519), Some(addr)) = v else {
                info!(%epoch, peer  = %k, "ignoring peer without x25519 key or p2p address");
                non_peers.insert(k.clone());
                continue;
            };
            if let Some((x, a)) = inner.peers.get(k) {
                if x == x25519 && a == addr {
                    debug!(%epoch, peer = %k, "peer unchanged");
                    continue;
                }
            }
            info!(%epoch, peer = %k, "adding network peer");
            to_add.push((k.clone(), *x25519, addr.clone()))
        }

        // Collect peers to remove from the network, i.e. peers which are no
        // longer stake table members.
        for p in inner.peers.keys() {
            if !stake_tbl.contains_key(p) {
                info!(%epoch, peer = %p, "removing network peer");
                to_del.push(p.clone())
            }
        }

        // Perform the updates:

        for k in &to_del {
            inner.peers.remove(k);
        }

        for (k, x, a) in to_add.iter().cloned() {
            inner.peers.insert(k, (x, a));
        }

        if let Err(err) = self.net.add(Role::Active, to_add).await {
            let _: NetworkDown = err;
            error!(%epoch, "network down; could not add peers to network");
            return;
        }

        if let Err(err) = self.net.remove(to_del).await {
            let _: NetworkDown = err;
            error!(%epoch, "network down; could not remove peers from network");
            return;
        }

        debug_assert_eq! {
            HashSet::<K>::from_iter(self.net.parties(None)),
            HashSet::<K>::from_iter(inner.peers.keys().cloned())
        }

        info!(%epoch, peers = %inner.peers.len(), non_peers = %non_peers.len());

        inner.epoch = EpochNumber::from(*epoch);
        inner.non_peers = non_peers;
    }
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
                use std::iter::empty;

                use hotshot_types::traits::metrics::NoMetrics;

                let (s, k, a) = &parties[i as usize];
                let it = parties
                    .iter()
                    .map(|(s, k, a)| (k.clone(), s.public_key(), a.clone()));
                let met = Box::new(NoMetrics);
                let net =
                    Cliquenet::create("test", k.clone(), s.clone(), a.clone(), it, empty(), met)
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
        let kpair = Keypair::derive_from::<K>(&secret);
        let port =
            test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
        let addr = NetAddr::Inet(std::net::Ipv4Addr::LOCALHOST.into(), port);
        i += 1;
        (kpair, public, addr)
    })
}
