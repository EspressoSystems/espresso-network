#[cfg(feature = "hotshot-testing")]
use std::sync::Arc;
#[cfg(feature = "hotshot-testing")]
use std::time::Duration;

use async_trait::async_trait;
use cliquenet::{NetConf, Retry};
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
        signature_key::{PrivateSignatureKey, SignatureKey},
    },
    x25519::{Keypair, PublicKey, SecretKey},
    BoxSyncFuture,
};
use tokio::sync::Mutex;
use tracing::warn;

#[derive(Clone)]
pub struct Cliquenet<K> {
    net: Retry<K>,
    epoch: Arc<Mutex<EpochNumber>>,
}

impl<K: SignatureKey + 'static> Cliquenet<K> {
    async fn on_epoch_change<U>(&self, epoch: EpochNumber, coord: &EpochMembershipCoordinator<U>)
    where
        U: NodeType<SignatureKey = K>,
    {
        let ours = self.epoch.lock().await;
        if epoch <= *ours {
            return;
        }
        let next = <<U as NodeType>::Epoch as ConsensusTime>::new(u64::from(epoch) + 1);
        let _prev =
            <<U as NodeType>::Epoch as ConsensusTime>::new(u64::from(epoch).saturating_sub(1));
        let Ok(_membership) = coord.stake_table_for_epoch(Some(next)).await else {
            warn!(epoch = %next, "no stake table available");
            return;
        };
    }
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
            epoch: Arc::new(Mutex::new(EpochNumber::genesis())),
        })
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
        expected_node_count: usize,
        _num_bootstrap: usize,
        _network_id: usize,
        _da_committee_size: usize,
        _reliability_config: Option<Box<dyn NetworkReliability>>,
        _secondary_network_delay: Duration,
    ) -> AsyncGenerator<Arc<Self>> {
        use std::net::Ipv4Addr;

        let mut parties: Vec<(Keypair, T::SignatureKey, Address)> = Vec::new();
        for i in 0..expected_node_count {
            let secret = T::SignatureKey::generated_from_seed_indexed([0u8; 32], i as u64).1;
            let public = T::SignatureKey::from_private(&secret);
            let kpair = derive_keypair::<<T as NodeType>::SignatureKey>(&secret);
            let port =
                test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
            let addr = NetAddr::Inet(Ipv4Addr::LOCALHOST.into(), port);

            parties.push((kpair, public, addr));
        }

        let parties = Arc::new(parties);

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
