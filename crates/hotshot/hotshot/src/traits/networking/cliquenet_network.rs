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
    BoxSyncFuture, PeerConnectInfo,
    addr::NetAddr,
    boxed_sync,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    stake_table::HSStakeTable,
    traits::{
        metrics::Metrics,
        network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    x25519::Keypair,
};
use tokio::sync::Mutex;
use tracing::{error, info};

#[derive(Clone)]
pub struct Cliquenet<K> {
    net: Retry<K>,
    inner: Arc<Mutex<Inner<K>>>,
}

#[derive(Clone)]
struct Inner<K> {
    epoch: EpochNumber,
    peers: HashMap<K, PeerConnectInfo>,
}

impl<K: SignatureKey + 'static> Cliquenet<K> {
    pub async fn create<A, P>(
        name: &'static str,
        key: K,
        keypair: Keypair,
        addr: A,
        parties: P,
        metrics: Box<dyn Metrics>,
    ) -> Result<Self, NetworkError>
    where
        A: Into<NetAddr>,
        P: IntoIterator<Item = (K, PeerConnectInfo)>,
    {
        let parties: HashMap<K, PeerConnectInfo> = parties.into_iter().collect();

        let cfg = NetConf::builder()
            .name(name)
            .label(key)
            .keypair(keypair)
            .bind(addr.into())
            .parties(
                parties
                    .iter()
                    .map(|(k, info)| (k.clone(), info.x25519_key, info.p2p_addr.clone())),
            )
            .metrics(metrics)
            .build();

        let net = Retry::create(cfg)
            .await
            .map_err(|e| NetworkError::ListenError(format!("cliquenet creation failed: {e}")))?;

        info!(peers = %parties.len(), "cliquenet created");

        Ok(Self {
            net,
            inner: Arc::new(Mutex::new(Inner {
                epoch: EpochNumber::genesis(),
                peers: parties,
            })),
        })
    }

    /// Get the current network peers.
    pub fn peers(&self) -> Vec<K> {
        self.net.parties(None)
    }

    /// Update peers on every epoch change.
    ///
    /// For any given epoch `e` we collect the validators of `e`, `e-1` and
    /// `e+1` from the stake tables and merge their connection information.
    ///
    /// We keep validator that were in `e-1` but not in `e` for one additional
    /// epoch and eagerly connect to new validators of `e+1`.
    async fn on_epoch_change<U>(&self, epoch: EpochNumber, coord: &EpochMembershipCoordinator<U>)
    where
        U: NodeType<SignatureKey = K>,
    {
        // Collect peer connect infos from stake table.
        let connect_infos = |a: HSStakeTable<U>, b: HSStakeTable<U>| {
            a.0.into_iter()
                .chain(b.0)
                .map(|m| (m.stake_table_entry.public_key(), m.connect_info))
                .collect()
        };

        let mut inner = self.inner.lock().await;

        if epoch <= inner.epoch {
            info!(%epoch, ours = %inner.epoch, "epoch already seen");
            return;
        }

        // Validators of the new epoch.
        let curr_infos = {
            let Ok(membership) = coord.stake_table_for_epoch(Some(epoch)).await else {
                error!(%epoch, "no stake table available");
                return;
            };
            let st = membership.stake_table().await;
            let da = membership.da_stake_table().await;
            connect_infos(st, da)
        };

        // Validators leaving are retained as peers for one additional epoch.
        let prev_infos = if *epoch > 0 {
            if let Ok(membership) = coord.stake_table_for_epoch(Some(epoch - 1)).await {
                let st = membership.stake_table().await;
                let da = membership.da_stake_table().await;
                connect_infos(st, da)
            } else {
                info!(%epoch, "previous epoch's stake table unavailable");
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        // Validators joining in the next epoch are connected to early.
        let next_infos = {
            if let Ok(membership) = coord.stake_table_for_epoch(Some(epoch + 1)).await {
                let st = membership.stake_table().await;
                let da = membership.da_stake_table().await;
                connect_infos(st, da)
            } else {
                info!(%epoch, "next epoch's stake table not available");
                HashMap::new()
            }
        };

        // Since connection information may be updated, we need to merge them,
        // preferring the newest epoch's data, i.e. `next(curr(prev))`.
        let mut merged_infos = prev_infos.clone();
        for (k, v) in curr_infos.iter().chain(&next_infos) {
            merged_infos.insert(k.clone(), v.clone());
        }

        let wanted: HashSet<K> = curr_infos
            .keys()
            .chain(next_infos.keys())
            .cloned()
            .collect();

        let retained: HashSet<K> = curr_infos
            .keys()
            .chain(prev_infos.keys())
            .cloned()
            .collect();

        let mut to_add = Vec::new();
        let mut to_del = Vec::new();

        for k in &wanted {
            if let Some(Some(new_info)) = merged_infos.get(k) {
                if Some(new_info) != inner.peers.get(k) {
                    info!(%epoch, peer = %k, "adding/updating network peer");
                    to_add.push((k.clone(), new_info.x25519_key, new_info.p2p_addr.clone()));
                } else {
                    info!(%epoch, peer = %k, "peer unchanged");
                }
            } else {
                info!(%epoch, peer  = %k, "ignoring peer without connection info");
            }
        }

        // Remove peers that have left both the current and previous epoch's stake tables.
        for p in inner.peers.keys() {
            if !(retained.contains(p) || wanted.contains(p)) {
                info!(%epoch, peer = %p, "removing network peer");
                to_del.push(p.clone());
            }
        }

        // Perform the updates:

        for k in &to_del {
            inner.peers.remove(k);
        }

        for (k, x, a) in to_add.iter().cloned() {
            inner.peers.insert(
                k,
                PeerConnectInfo {
                    x25519_key: x,
                    p2p_addr: a,
                },
            );
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

        info!(%epoch, peers = %inner.peers.len());

        inner.epoch = EpochNumber::from(*epoch);
    }
}

/// Collect the union of `prev`, `curr`, and `next` epoch stake tables (each
/// merged with its DA committee) as a flat map of peers to dial.
///
/// Used at startup to seed cliquenet with the same window `on_epoch_change`
/// would build for `epoch`, before any epoch transition has occurred.
/// Newest-wins ordering for `connect_info`: next overrides curr overrides prev.
/// Entries with no `connect_info` are filtered out.
pub async fn collect_window_peers<U>(
    coord: &EpochMembershipCoordinator<U>,
    epoch: EpochNumber,
) -> HashMap<U::SignatureKey, PeerConnectInfo>
where
    U: NodeType,
{
    let curr = fetch_epoch_peers(coord, Some(epoch)).await;
    let prev = if *epoch > 0 {
        fetch_epoch_peers(coord, Some(epoch - 1)).await
    } else {
        HashMap::new()
    };
    let next = fetch_epoch_peers(coord, Some(epoch + 1)).await;

    // Newest-wins merge: start from prev, overlay curr and next.
    let mut merged: HashMap<U::SignatureKey, Option<PeerConnectInfo>> = prev;
    for (k, v) in curr.into_iter().chain(next) {
        merged.insert(k, v);
    }

    merged
        .into_iter()
        .filter_map(|(k, v)| v.map(|info| (k, info)))
        .collect()
}

async fn fetch_epoch_peers<U>(
    coord: &EpochMembershipCoordinator<U>,
    epoch: Option<EpochNumber>,
) -> HashMap<U::SignatureKey, Option<PeerConnectInfo>>
where
    U: NodeType,
{
    let Ok(membership) = coord.stake_table_for_epoch(epoch).await else {
        return HashMap::new();
    };
    let st = membership.stake_table().await;
    let da = membership.da_stake_table().await;
    st.0.into_iter()
        .chain(da.0)
        .map(|m| (m.stake_table_entry.public_key(), m.connect_info))
        .collect()
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
        connect_infos: &mut HashMap<T::SignatureKey, PeerConnectInfo>,
    ) -> AsyncGenerator<Arc<Self>> {
        let parties = {
            let p = gen_parties::<T::SignatureKey>()
                .take(nodes)
                .collect::<Vec<_>>();
            Arc::new(p)
        };
        for (s, k, a) in &*parties {
            connect_infos.insert(
                k.clone(),
                PeerConnectInfo {
                    x25519_key: s.public_key(),
                    p2p_addr: a.clone(),
                },
            );
        }
        Box::pin(move |i| {
            let parties = parties.clone();
            let future = async move {
                use hotshot_types::traits::metrics::NoMetrics;

                let (s, k, a) = &parties[i as usize];
                let it = parties.iter().map(|(s, k, a)| {
                    (
                        k.clone(),
                        PeerConnectInfo {
                            x25519_key: s.public_key(),
                            p2p_addr: a.clone(),
                        },
                    )
                });
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
        let kpair = Keypair::derive_from::<K>(&secret);
        let port =
            test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
        let addr = NetAddr::Inet(std::net::Ipv4Addr::LOCALHOST.into(), port);
        i += 1;
        (kpair, public, addr)
    })
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use hotshot_types::{signature_key::BLSKeyPair, traits::metrics::NoMetrics, x25519};
    use rand::thread_rng;
    use test_utils::reserve_tcp_port;

    use super::*;

    #[tokio::test]
    async fn test_create_empty_network() {
        let port = reserve_tcp_port().unwrap();
        Cliquenet::create(
            "test",
            BLSKeyPair::generate(&mut thread_rng()).ver_key(),
            x25519::Keypair::generate().unwrap(),
            NetAddr::from_str(&format!("0.0.0.0:{port}")).unwrap(),
            [],
            Box::new(NoMetrics),
        )
        .await
        .unwrap();
    }
}
