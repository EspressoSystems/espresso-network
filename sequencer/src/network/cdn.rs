/// Trait implementations for the CDN
use std::marker::PhantomData;

use bincode::Options;
use cdn_broker::reexports::{
    connection::protocols::{Quic, Tcp, TcpTls},
    crypto::signature::{Serializable, SignatureScheme},
    def::{hook::NoMessageHook, ConnectionDef, RunDef, Topic as TopicTrait},
    discovery::{Embedded, Redis},
};
use hotshot::types::SignatureKey;
use hotshot_types::{
    traits::{network::Topic as HotShotTopic, node_implementation::NodeType},
    utils::bincode_opts,
};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use static_assertions::const_assert_eq;

/// The enum for the topics we can subscribe to in the Push CDN
#[repr(u8)]
#[derive(IntoPrimitive, TryFromPrimitive, Clone, PartialEq, Eq)]
pub enum Topic {
    /// The global topic
    Global = 0,
    /// The DA topic
    Da = 1,
}

pub enum Namespace {}

// Make sure the topics are the same as defined in `HotShot`.
const_assert_eq!(Topic::Global as u8, HotShotTopic::Global as u8);
const_assert_eq!(Topic::Da as u8, HotShotTopic::Da as u8);

/// Implement the `TopicTrait` for our `Topic` enum. This lets us define
/// compatible topics at the broker-level. Others will be rejected.
impl TopicTrait for Topic {}

/// A wrapped `SignatureKey`. We need to implement the Push CDN's `SignatureScheme`
/// trait in order to sign and verify messages to/from the CDN.
#[derive(Clone, Eq, PartialEq)]
pub struct WrappedSignatureKey<T: SignatureKey + 'static>(pub T);
impl<T: SignatureKey> SignatureScheme for WrappedSignatureKey<T> {
    type PrivateKey = T::PrivateKey;
    type PublicKey = Self;

    /// Sign a message of arbitrary data and return the serialized signature
    ///
    /// The namespace is prefixed to the message before signing to prevent
    /// signature replay attacks in different parts of the system.
    fn sign(
        private_key: &Self::PrivateKey,
        namespace: &str,
        message: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        // Combine the namespace and message into a single byte array
        let message = [namespace.as_bytes(), message].concat();

        let signature = T::sign(private_key, &message)?;
        Ok(bincode_opts().serialize(&signature)?)
    }

    /// Verify a message of arbitrary data and return the result
    ///
    /// The namespace is prefixed to the message before verification to prevent
    /// signature replay attacks in different parts of the system.
    fn verify(
        public_key: &Self::PublicKey,
        namespace: &str,
        message: &[u8],
        signature: &[u8],
    ) -> bool {
        // Combine the namespace and message into a single byte array
        let namespaced_message = [namespace.as_bytes(), message].concat();

        let signature: T::PureAssembledSignatureType = match bincode_opts().deserialize(signature) {
            Ok(key) => key,
            Err(_) => return false,
        };

        public_key.0.validate(&signature, &namespaced_message)
    }
}

/// We need to implement the `Serializable` so the Push CDN can serialize the signatures
/// and public keys and send them over the wire.
impl<T: SignatureKey> Serializable for WrappedSignatureKey<T> {
    fn serialize(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.0.to_bytes())
    }

    fn deserialize(serialized: &[u8]) -> anyhow::Result<Self> {
        Ok(WrappedSignatureKey(T::from_bytes(serialized)?))
    }
}

/// The production run definition for the Push CDN.
/// Uses the real protocols and a Redis discovery client.
pub struct ProductionDef<TYPES: NodeType>(PhantomData<TYPES>);
impl<TYPES: NodeType> RunDef for ProductionDef<TYPES> {
    type User = UserDefQuic<TYPES>;
    type User2 = UserDefTcp<TYPES>;
    type Broker = BrokerDef<TYPES>;
    type DiscoveryClientType = Redis;
    type Topic = Topic;
}

/// The user definition for the Push CDN.
/// Uses the Quic protocol and untrusted middleware.
/// RM TODO: Remove this, switching to TCP+TLS singularly when everyone has updated
pub struct UserDefQuic<TYPES: NodeType>(PhantomData<TYPES>);
impl<TYPES: NodeType> ConnectionDef for UserDefQuic<TYPES> {
    type Scheme = WrappedSignatureKey<TYPES::SignatureKey>;
    type Protocol = Quic;
    type MessageHook = NoMessageHook;
}

/// The (parallel, TCP) user definition for the Push CDN.
/// Uses the TCP+TLS protocol and untrusted middleware.
pub struct UserDefTcp<TYPES: NodeType>(PhantomData<TYPES>);
impl<TYPES: NodeType> ConnectionDef for UserDefTcp<TYPES> {
    type Scheme = WrappedSignatureKey<TYPES::SignatureKey>;
    type Protocol = TcpTls;
    type MessageHook = NoMessageHook;
}

/// The broker definition for the Push CDN.
/// Uses the TCP protocol and trusted middleware.
pub struct BrokerDef<TYPES: NodeType>(PhantomData<TYPES>);
impl<TYPES: NodeType> ConnectionDef for BrokerDef<TYPES> {
    type Scheme = WrappedSignatureKey<TYPES::SignatureKey>;
    type Protocol = Tcp;
    type MessageHook = NoMessageHook;
}

/// The client definition for the Push CDN. Uses the Quic
/// protocol and no middleware. Differs from the user
/// definition in that is on the client-side.
#[derive(Clone)]
pub struct ClientDef<TYPES: NodeType>(PhantomData<TYPES>);
impl<TYPES: NodeType> ConnectionDef for ClientDef<TYPES> {
    type Scheme = WrappedSignatureKey<TYPES::SignatureKey>;
    type Protocol = Quic;
    type MessageHook = NoMessageHook;
}

/// The testing run definition for the Push CDN.
/// Uses the real protocols, but with an embedded discovery client.
pub struct TestingDef<TYPES: NodeType>(PhantomData<TYPES>);
impl<TYPES: NodeType> RunDef for TestingDef<TYPES> {
    type User = UserDefQuic<TYPES>;
    type User2 = UserDefTcp<TYPES>;
    type Broker = BrokerDef<TYPES>;
    type DiscoveryClientType = Embedded;
    type Topic = Topic;
}
