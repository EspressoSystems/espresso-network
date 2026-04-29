#[cfg(feature = "hotshot-testing")]
use std::time::Duration;
use std::{
    collections::{HashMap, HashSet},
    future::ready,
    sync::Arc,
};

use async_trait::async_trait;
use cliquenet::{self, Network, NetworkController, NetworkReceiver, Role, Slot};
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
        network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
        node_implementation::NodeType,
        signature_key::{SignatureKey, StakeTableEntryType},
    },
    x25519::{Keypair, PublicKey},
};
use parking_lot::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct Cliquenet<K> {
    my_keys: (K, PublicKey),
    gc_window: ViewNumber,
    sender: Arc<Mutex<Sender<K>>>,
    receiver: Arc<AsyncMutex<NetworkReceiver>>,
    epoch: Arc<AsyncMutex<EpochNumber>>,
}

struct Sender<K> {
    controller: NetworkController,
    peers: HashMap<K, PeerConnectInfo>,
    last_gc: ViewNumber,
}

impl<K: SignatureKey + 'static> Cliquenet<K> {
    pub async fn create<A, P>(
        name: &'static str,
        key: K,
        keypair: Keypair,
        addr: A,
        parties: P,
    ) -> Result<Self, NetworkError>
    where
        A: Into<NetAddr>,
        P: IntoIterator<Item = (K, PeerConnectInfo)>,
    {
        let this = (key, keypair.public_key());
        let parties: HashMap<K, PeerConnectInfo> = parties.into_iter().collect();

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

        let net = Network::create(cfg)
            .await
            .map_err(|e| NetworkError::ListenError(format!("cliquenet creation failed: {e}")))?;

        info!(peers = %parties.len(), "cliquenet created");

        let (control, recv) = net.split_into();

        Ok(Self {
            my_keys: this,
            sender: Arc::new(Mutex::new(Sender {
                controller: control,
                peers: parties,
                last_gc: ViewNumber::genesis(),
            })),
            receiver: Arc::new(AsyncMutex::new(recv)),
            epoch: Arc::new(AsyncMutex::new(EpochNumber::genesis())),
            gc_window: ViewNumber::new(100),
        })
    }

    /// How many views should potentially be resend to unresponsive peers?
    ///
    /// After that, messages from older views are discarded.
    ///
    /// NB: This is a workaround. The `ConnectedNetwork` trait should be
    /// augmented to have a GC method that is invoked when it is safe to
    /// discard old views.
    pub fn set_gc_window(&mut self, v: ViewNumber) {
        self.gc_window = v
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

        let mut our_epoch = self.epoch.lock().await;

        if epoch <= *our_epoch {
            info!(%epoch, ours = %our_epoch, "epoch already seen");
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
                if Some(new_info) != self.sender.lock().peers.get(k) {
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
        for (k, p) in &self.sender.lock().peers {
            if !(retained.contains(k) || wanted.contains(k)) {
                info!(%epoch, peer = %k, "removing network peer");
                to_del.push((k.clone(), p.x25519_key));
            }
        }

        // Perform the updates:
        {
            let peers = &mut self.sender.lock().peers;
            for (k, _) in &to_del {
                peers.remove(k);
            }

            for (k, x, a) in to_add.iter().cloned() {
                peers.insert(
                    k,
                    PeerConnectInfo {
                        x25519_key: x,
                        p2p_addr: a,
                    },
                );
            }
        }

        {
            let to_add = to_add.iter().map(|(_, k, a)| ((*k).into(), a.clone()));
            let to_del = to_del.iter().map(|(_, k)| (*k).into());

            let mut sender = self.sender.lock();

            if let Err(err) = sender.controller.add_peers(Role::Active, to_add) {
                error!(%epoch, %err, "network down; could not add peers to network");
                return;
            }

            if let Err(err) = sender.controller.remove_peers(to_del) {
                error!(%epoch, %err, "network down; could not remove peers from network");
                return;
            }
        }

        info!(%epoch, peers = %self.sender.lock().peers.len());

        *our_epoch = EpochNumber::from(*epoch);
    }

    /// Get the current network peers.
    #[cfg(feature = "hotshot-testing")]
    pub fn peers(&self) -> Vec<(PublicKey, Role)> {
        self.sender
            .lock()
            .controller
            .parties()
            .map(|(k, r)| ((*k).into(), *r))
            .collect()
    }

    #[cfg(feature = "hotshot-testing")]
    pub fn reverse_lookup(&self, k: &PublicKey) -> Option<K> {
        for (x, p) in &self.sender.lock().peers {
            if p.x25519_key == *k {
                return Some(x.clone());
            }
        }
        None
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
        self.sender
            .lock()
            .controller
            .broadcast(Slot::new(*v), m)
            .map_err(|e| NetworkError::MessageSendError(format!("cliquenet broadcast error: {e}")))
    }

    async fn da_broadcast_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipients: Vec<K>,
        _: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        let mut sender = self.sender.lock();
        let mut targets = Vec::new();
        for r in recipients {
            if let Some(p) = sender.peers.get(&r) {
                targets.push(p.x25519_key.into())
            } else if r == self.my_keys.0 {
                targets.push(self.my_keys.1.into())
            } else {
                warn!(node = %self.my_keys.1, recipient = %r, "unknown da broadcast target");
            }
        }
        sender
            .controller
            .multicast(Slot::new(*v), targets, m)
            .map_err(|e| {
                NetworkError::MessageSendError(format!("cliquenet da_broadcast error: {e}"))
            })
    }

    async fn direct_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipient: K,
    ) -> Result<(), NetworkError> {
        let mut sender = self.sender.lock();
        let target = if recipient == self.my_keys.0 {
            self.my_keys.1
        } else if let Some(k) = sender.peers.get(&recipient).map(|p| p.x25519_key) {
            k
        } else {
            warn!(node = %self.my_keys.1, %recipient, "unknown direct message target");
            return Ok(());
        };
        sender
            .controller
            .unicast(Slot::new(*v), target.into(), m)
            .map_err(|e| NetworkError::MessageSendError(format!("cliquenet unicast error: {e}")))
    }

    async fn recv_message(&self) -> Result<Vec<u8>, NetworkError> {
        let (_src, data) = self
            .receiver
            .lock()
            .await
            .receive()
            .await
            .ok_or(NetworkError::ShutDown)?;
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
        {
            let mut sender = self.sender.lock();
            if v.saturating_sub(*sender.last_gc) >= *self.gc_window {
                let _ = sender.controller.gc(Slot::new(*v));
                sender.last_gc = v
            }
        }
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
        let result = self.sender.lock().controller.shutdown();
        if let Ok(future) = result {
            boxed_sync(future)
        } else {
            boxed_sync(ready(()))
        }
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
                let net = Cliquenet::create("test", k.clone(), s.clone(), a.clone(), it)
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
        let kpair = Keypair::derive_from::<K>(&secret).unwrap();
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

    use hotshot_types::{signature_key::BLSKeyPair, x25519};
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
        )
        .await
        .unwrap();
    }
}
