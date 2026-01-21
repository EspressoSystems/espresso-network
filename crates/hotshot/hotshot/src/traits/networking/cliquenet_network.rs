#[cfg(feature = "hotshot-testing")]
use std::sync::Arc;
#[cfg(feature = "hotshot-testing")]
use std::time::Duration;

use async_trait::async_trait;
pub use cliquenet::{Address, PublicKey};
use cliquenet::{Keypair, NetConf, Retry, SecretKey};

use futures::future::ready;
#[cfg(feature = "hotshot-testing")]
use hotshot_types::traits::network::{
    AsyncGenerator, NetworkReliability, TestableNetworkingImplementation,
};
use hotshot_types::{
    boxed_sync,
    traits::{
        network::{BroadcastDelay, ConnectedNetwork, NetworkError, Topic},
        node_implementation::NodeType,
        signature_key::{PrivateSignatureKey, SignatureKey},
    },
    BoxSyncFuture,
};
#[derive(Clone)]
pub struct Cliquenet<T: NodeType> {
    net: Retry<T::SignatureKey>,
}

impl<T: NodeType> Cliquenet<T> {
    pub async fn create<A, B, P>(
        name: &'static str,
        key: T::SignatureKey,
        keypair: Keypair,
        addr: A,
        parties: P,
    ) -> Result<Self, NetworkError>
    where
        A: Into<Address>,
        B: Into<Address>,
        P: IntoIterator<Item = (T::SignatureKey, PublicKey, B)>,
    {
        let cfg = NetConf::builder()
            .name(name)
            .label(key)
            .keypair(keypair)
            .bind(addr.into())
            .parties(parties.into_iter().map(|(k, x, a)| (k, x, a.into())))
            .build();
        let net = Retry::create(cfg)
            .await
            .map_err(|e| NetworkError::ListenError(format!("cliquenet creation failed: {e}")))?;
        Ok(Self { net })
    }
}

pub fn derive_keypair<K: SignatureKey>(k: &K::PrivateKey) -> Keypair {
    SecretKey::from(blake3::derive_key("cliquenet key", &k.to_bytes())).into()
}

#[async_trait]
impl<T: NodeType> ConnectedNetwork<T::SignatureKey> for Cliquenet<T> {
    async fn broadcast_message(
        &self,
        m: Vec<u8>,
        _: Topic,
        _: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        self.net.broadcast(0, m).await.map_err(|e| {
            NetworkError::MessageSendError(format!("cliquenet broadcast error: {e}"))
        })?;
        Ok(())
    }

    async fn da_broadcast_message(
        &self,
        m: Vec<u8>,
        recipients: Vec<T::SignatureKey>,
        _: BroadcastDelay,
    ) -> Result<(), NetworkError> {
        self.net.multicast(recipients, 0, m).await.map_err(|e| {
            NetworkError::MessageSendError(format!("cliquenet da_broadcast error: {e}"))
        })?;
        Ok(())
    }

    async fn direct_message(
        &self,
        m: Vec<u8>,
        recipient: T::SignatureKey,
    ) -> Result<(), NetworkError> {
        self.net
            .unicast(recipient, 0, m)
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

    async fn wait_for_ready(&self) {}

    fn pause(&self) {
        unimplemented!("Pausing not implemented for cliquenet");
    }

    fn resume(&self) {
        unimplemented!("Resuming not implemented for cliquenet");
    }

    fn shut_down<'a, 'b>(&'a self) -> BoxSyncFuture<'b, ()>
    where
        'a: 'b,
        Self: 'b,
    {
        boxed_sync(ready(()))
    }
}

#[cfg(feature = "hotshot-testing")]
impl<T: NodeType> TestableNetworkingImplementation<T> for Cliquenet<T> {
    fn generator(
        expected_node_count: usize,
        _num_bootstrap: usize,
        _network_id: usize,
        _da_committee_size: usize,
        _reliability_config: Option<Box<dyn NetworkReliability>>,
        _secondary_network_delay: Duration,
    ) -> AsyncGenerator<Arc<Self>> {
        let mut parties = Vec::new();
        for i in 0..expected_node_count {
            use std::net::Ipv4Addr;

            use cliquenet::Address;

            let secret = T::SignatureKey::generated_from_seed_indexed([0u8; 32], i as u64).1;
            let public = T::SignatureKey::from_private(&secret);
            let kpair = derive_keypair::<<T as NodeType>::SignatureKey>(&secret);
            let port = portpicker::pick_unused_port().expect("an unused port is available");
            let addr = Address::Inet(Ipv4Addr::LOCALHOST.into(), port);

            parties.push((kpair, public, addr));
        }

        let parties = Arc::new(parties);

        Box::pin(move |i| {
            let parties = parties.clone();
            let future = async move {
                let (s, k, a) = &parties[i as usize];
                let it = parties
                    .iter()
                    .map(|(s, k, a)| (k.clone(), s.public_key(), a.clone()));
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
