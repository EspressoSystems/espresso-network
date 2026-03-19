use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
#[cfg(feature = "hotshot-testing")]
use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use hotshot_types::{
    BoxSyncFuture, boxed_sync,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
        node_implementation::NodeType,
    },
};
#[cfg(feature = "hotshot-testing")]
use hotshot_types::{
    PeerConnectInfo,
    traits::network::{AsyncGenerator, NetworkReliability, TestableNetworkingImplementation},
};
use tokio::{join, select};
use tracing::info;
use versions::CLIQUENET_VERSION;

use crate::traits::networking::Cliquenet;

/// Compatibility network.
///
/// Uses either a fallback network (any impl of `ConnectedNetwork`), or else
/// `Cliquenet` once the protocol has been upgraded to `CLIQUENET_VERSION`.
///
/// Receiving listens on both networks simultaneously so that messages arriving
/// on either side are never lost. Sending is routed to the active network only.
#[derive(Clone)]
pub struct CompatNetwork<A, TYPES: NodeType> {
    cliquenet: Cliquenet<TYPES::SignatureKey>,
    fallback: A,
    use_cliquenet: Arc<AtomicBool>,
    upgrade_lock: Arc<OnceLock<UpgradeLock<TYPES>>>,
}

impl<A, TYPES> CompatNetwork<A, TYPES>
where
    TYPES: NodeType,
{
    pub async fn new(cliquenet: Cliquenet<TYPES::SignatureKey>, fallback: A) -> Self {
        Self {
            cliquenet,
            fallback,
            use_cliquenet: Arc::new(AtomicBool::new(false)),
            upgrade_lock: Arc::new(OnceLock::new()),
        }
    }

    pub fn cliquenet(&self) -> &Cliquenet<TYPES::SignatureKey> {
        &self.cliquenet
    }

    pub fn fallback(&self) -> &A {
        &self.fallback
    }

    pub fn set_upgrade_lock(&self, lock: UpgradeLock<TYPES>) {
        let _ = self.upgrade_lock.set(lock);
    }

    pub fn use_cliquenet(&self) {
        self.use_cliquenet.store(true, Ordering::Relaxed)
    }

    pub fn is_cliquenet(&self) -> bool {
        self.use_cliquenet.load(Ordering::Relaxed)
    }

    fn maybe_switch_to_cliquenet(&self, view: ViewNumber) {
        if self.is_cliquenet() {
            return;
        }
        if let Some(lock) = self.upgrade_lock.get()
            && lock.version_infallible(view) >= CLIQUENET_VERSION
        {
            info!("switching to cliquenet network");
            self.use_cliquenet();
        }
    }
}

#[async_trait]
impl<A, TYPES> ConnectedNetwork<TYPES::SignatureKey> for CompatNetwork<A, TYPES>
where
    A: ConnectedNetwork<TYPES::SignatureKey>,
    TYPES: NodeType,
{
    async fn broadcast_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        t: Topic,
        d: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        if self.is_cliquenet() {
            self.cliquenet.broadcast_message(v, m, t, d).await
        } else {
            self.fallback.broadcast_message(v, m, t, d).await
        }
    }

    async fn da_broadcast_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipients: Vec<TYPES::SignatureKey>,
        d: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        if self.is_cliquenet() {
            self.cliquenet
                .da_broadcast_message(v, m, recipients, d)
                .await
        } else {
            self.fallback
                .da_broadcast_message(v, m, recipients, d)
                .await
        }
    }

    async fn direct_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipient: TYPES::SignatureKey,
    ) -> Result<(), NetworkError> {
        if self.is_cliquenet() {
            self.cliquenet.direct_message(v, m, recipient).await
        } else {
            self.fallback.direct_message(v, m, recipient).await
        }
    }

    async fn recv_message(&self) -> Result<Vec<u8>, NetworkError> {
        select! {
            m = self.cliquenet.recv_message() => m,
            m = self.fallback.recv_message() => m
        }
    }

    async fn update_view<U>(
        &self,
        v: ViewNumber,
        e: Option<EpochNumber>,
        m: EpochMembershipCoordinator<U>,
    ) where
        U: NodeType<SignatureKey = TYPES::SignatureKey>,
    {
        self.maybe_switch_to_cliquenet(v);
        join! {
            self.cliquenet.update_view(v, e, m.clone()),
            self.fallback.update_view(v, e, m)
        };
    }

    async fn wait_for_ready(&self) {
        if self.is_cliquenet() {
            self.cliquenet.wait_for_ready().await
        } else {
            self.fallback.wait_for_ready().await
        }
    }

    fn pause(&self) {
        self.cliquenet.pause();
        self.fallback.pause()
    }

    fn resume(&self) {
        self.cliquenet.resume();
        self.fallback.resume()
    }

    fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
    where
        'a: 'b,
        Self: 'b,
    {
        let a = self.cliquenet.shut_down();
        let b = self.fallback.shut_down();
        boxed_sync(async {
            join!(a, b);
        })
    }
}

#[cfg(feature = "hotshot-testing")]
impl<T, A> TestableNetworkingImplementation<T> for CompatNetwork<A, T>
where
    T: NodeType,
    A: TestableNetworkingImplementation<T> + Clone + Send + 'static,
{
    fn generator(
        nodes: usize,
        num_bootstrap: usize,
        network_id: usize,
        da_committee_size: usize,
        reliability_config: Option<Box<dyn NetworkReliability>>,
        secondary_network_delay: Duration,
        connect_infos: &mut HashMap<T::SignatureKey, PeerConnectInfo>,
    ) -> AsyncGenerator<Arc<Self>> {
        let cliquenet =
            <Cliquenet<T::SignatureKey> as TestableNetworkingImplementation<T>>::generator(
                nodes,
                num_bootstrap,
                network_id,
                da_committee_size,
                reliability_config.clone(),
                secondary_network_delay,
                connect_infos,
            );

        let fallback = <A as TestableNetworkingImplementation<T>>::generator(
            nodes,
            num_bootstrap,
            network_id,
            da_committee_size,
            reliability_config.clone(),
            secondary_network_delay,
            connect_infos,
        );

        Box::pin(move |i: u64| {
            let cliquenet = cliquenet(i);
            let fallback = fallback(i);

            let future = async move {
                let cliquenet = Arc::unwrap_or_clone(cliquenet.await);
                let fallback = Arc::unwrap_or_clone(fallback.await);
                Arc::new(Self::new(cliquenet, fallback).await)
            };

            Box::pin(future)
        })
    }

    fn in_flight_message_count(&self) -> Option<usize> {
        None
    }
}
