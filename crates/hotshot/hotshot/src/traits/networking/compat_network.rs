use std::{collections::HashSet, sync::Arc};

use async_trait::async_trait;
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
use parking_lot::RwLock;
use tokio::{join, select};

use crate::traits::networking::Cliquenet;

#[derive(Clone)]
pub struct CompatNetwork<A, K> {
    cliquenet: Cliquenet<K>,
    cliquenet_peers: Arc<RwLock<HashSet<K>>>,
    fallback: A,
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
        let (a, b) = join! {
            self.cliquenet.broadcast_message(v, m.clone(), t, d.clone()),
            self.fallback.broadcast_message(v, m, t, d)
        };
        merge(a, b)
    }

    async fn da_broadcast_message(
        &self,
        v: ViewNumber,
        m: Vec<u8>,
        recipients: Vec<K>,
        d: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        let cliquenet_peers = self.cliquenet_peers.read();
        let (c, f): (Vec<_>, Vec<_>) = recipients
            .into_iter()
            .partition(|k| cliquenet_peers.contains(k));
        drop(cliquenet_peers);

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
        if self.cliquenet_peers.read().contains(&recipient) {
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

fn merge(a: Result<(), NetworkError>, b: Result<(), NetworkError>) -> Result<(), NetworkError> {
    match (a, b) {
        (Err(e1), Err(e2)) => Err(NetworkError::Multiple(vec![e1, e2])),
        (Err(e1), _) => Err(e1),
        (_, Err(e2)) => Err(e2),
        (Ok(()), Ok(())) => Ok(()),
    }
}
