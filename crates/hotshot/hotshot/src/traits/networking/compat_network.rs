#[cfg(feature = "hotshot-testing")]
use std::{collections::HashMap, time::Duration};
use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
use futures::future::join_all;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
        node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    BoxSyncFuture,
};
#[cfg(feature = "hotshot-testing")]
use hotshot_types::{
    traits::network::{AsyncGenerator, NetworkReliability, TestableNetworkingImplementation},
    PeerConnectInfo,
};
use parking_lot::RwLock;
use tokio::{join, select};

use crate::traits::networking::Cliquenet;

#[derive(Clone)]
pub struct CompatNetwork<A, K> {
    cliquenet: Cliquenet<K>,
    peers: Arc<RwLock<Peers<K>>>,
    fallback: A,
}

struct Peers<K> {
    members: HashSet<K>,
    others: HashSet<K>,
}

impl<A, K> CompatNetwork<A, K>
where
    K: SignatureKey + 'static,
{
    pub async fn new(cliquenet: Cliquenet<K>, fallback: A) -> Self {
        let peers = cliquenet.peers();
        let non_peers = cliquenet.non_peers().await;
        Self {
            cliquenet,
            peers: Arc::new(RwLock::new(Peers {
                members: HashSet::from_iter(peers),
                others: non_peers,
            })),
            fallback,
        }
    }

    pub fn cliquenet(&self) -> &Cliquenet<K> {
        &self.cliquenet
    }

    pub fn fallback(&self) -> &A {
        &self.fallback
    }
}

#[async_trait]
impl<A, K> ConnectedNetwork<K> for CompatNetwork<A, K>
where
    A: ConnectedNetwork<K>,
    K: SignatureKey + 'static,
{
    async fn broadcast_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        t: Topic,
        d: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        let f1 = self.cliquenet.broadcast_message(v, m.clone(), t, d.clone());

        let f2 = join_all(
            self.peers
                .read()
                .others
                .iter()
                .map(|k| self.fallback.direct_message(v, m.clone(), k.clone())),
        );

        let results = join!(f1, f2);

        let mut errors = Vec::new();
        if let Err(e) = results.0 {
            errors.push(e);
        }
        errors.extend(results.1.into_iter().filter_map(|r| r.err()));

        if errors.is_empty() {
            Ok(())
        } else {
            Err(NetworkError::Multiple(errors))
        }
    }

    async fn da_broadcast_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipients: Vec<K>,
        d: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        let (c, f): (Vec<_>, Vec<_>) = {
            let cliquenet_peers = &self.peers.read().members;
            recipients
                .into_iter()
                .partition(|k| cliquenet_peers.contains(k))
        };
        let (a, b) = join! {
            self.cliquenet.da_broadcast_message(v, m.clone(), c, d.clone()),
            self.fallback.da_broadcast_message(v, m, f, d)
        };
        merge(a, b)
    }

    async fn direct_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipient: K,
    ) -> Result<(), NetworkError> {
        if self.peers.read().members.contains(&recipient) {
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
        U: NodeType<SignatureKey = K>,
    {
        join! {
            self.cliquenet.update_view(v, e, m.clone()),
            self.fallback.update_view(v, e, m)
        };

        let others = self.cliquenet.non_peers().await;

        let mut peers = self.peers.write();
        peers.members.clear();
        peers.members.extend(self.cliquenet.peers());
        peers.others = others;
    }

    async fn wait_for_ready(&self) {
        join! {
            self.cliquenet.wait_for_ready(),
            self.fallback.wait_for_ready()
        };
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
        Box::pin(async {
            join!(a, b);
        })
    }
}

#[cfg(feature = "hotshot-testing")]
impl<T, A> TestableNetworkingImplementation<T> for CompatNetwork<A, T::SignatureKey>
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

fn merge(a: Result<(), NetworkError>, b: Result<(), NetworkError>) -> Result<(), NetworkError> {
    match (a, b) {
        (Err(e1), Err(e2)) => Err(NetworkError::Multiple(vec![e1, e2])),
        (Err(e1), _) => Err(e1),
        (_, Err(e2)) => Err(e2),
        (Ok(()), Ok(())) => Ok(()),
    }
}
