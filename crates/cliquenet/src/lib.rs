mod addr;
mod connection;
mod metrics;
mod msg;
mod net;
mod queue;
mod time;

pub mod error;
pub mod x25519;

use std::{
    collections::BTreeMap,
    fmt,
    num::NonZeroUsize,
    sync::{Arc, LazyLock},
    time::Duration,
};

pub use addr::NetAddr;
use bon::Builder;
pub use error::NetworkError;
pub use metrics::Metrics;
pub use msg::Slot;
pub use net::{
    Network, NetworkController, NetworkReceiver, RetryPolicy, SendAction, SendCommand,
    SendCommandBuilder,
};
use snow::params::NoiseParams;

use crate::x25519::{Keypair, PublicKey};

pub static NOISE_IK_25519_AESGCM_BLAKE2S: LazyLock<NoiseParams> = LazyLock::new(|| {
    "Noise_IK_25519_AESGCM_BLAKE2s"
        .parse()
        .expect("valid noise params")
});

#[derive(Builder)]
#[non_exhaustive]
pub struct Config {
    /// Network name.
    #[builder(with = |s: impl Into<String>| Arc::new(s.into()))]
    pub name: Arc<String>,

    #[builder(with = |xs: impl IntoIterator<Item = (Version, NoiseParams)>| {
        let m = BTreeMap::from_iter(xs);
        assert! {
            !m.is_empty(),
            "noise configs must not be empty"
        }
        assert! {
            m.keys().zip(m.keys().skip(1)).all(|(a, b)| u16::from(*a) + 1 == u16::from(*b)),
            "noise config versions must be consecutive"
        }
        m
    })]
    noise_configs: BTreeMap<Version, NoiseParams>,

    /// DH keypair
    pub keypair: Keypair,

    /// Address to bind to.
    pub bind: NetAddr,

    /// Network members with public key and network address.
    #[builder(with = <_>::from_iter)]
    pub parties: Vec<(PublicKey, NetAddr)>,

    #[builder(default = NonZeroUsize::new(100).expect("100 > 0"))]
    pub peer_budget: NonZeroUsize,

    /// Max. number of bytes per message to send or receive.
    #[builder(default = NonZeroUsize::new(10485760).expect("10485760 > 0"))]
    pub max_message_size: NonZeroUsize,

    /// Retry delays in seconds.
    #[builder(default = vec![1, 3, 5, 15, 30])]
    pub retry_delays: Vec<u8>,

    #[builder(default = Duration::from_secs(30))]
    pub max_retry_delay: Duration,

    /// Randomly delay the initial connect attempt between 0 and 1s.
    #[builder(default = true)]
    pub random_connect_delay: bool,

    #[builder(default = Duration::from_secs(30))]
    pub connect_timeout: Duration,

    #[builder(default = Duration::from_secs(10))]
    pub handshake_timeout: Duration,

    #[builder(default = Duration::from_secs(30))]
    pub receive_timeout: Duration,

    #[builder(default = Duration::from_secs(30))]
    pub backoff_duration: Duration,

    pub metrics: Option<Arc<dyn Metrics>>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("name", &self.name)
            .field("key", &self.keypair.public_key())
            .field("bind", &self.bind)
            .field("parties", &self.parties)
            .field("peer_budget", &self.peer_budget)
            .field("max_message_size", &self.max_message_size)
            .field("retry_delays", &self.retry_delays)
            .field("max_retry_delay", &self.max_retry_delay)
            .field("random_connect_delay", &self.random_connect_delay)
            .field("connect_timeout", &self.connect_timeout)
            .field("handshake_timeout", &self.handshake_timeout)
            .field("receive_timeout", &self.receive_timeout)
            .field("backoff_duration", &self.backoff_duration)
            .finish()
    }
}

impl Config {
    pub fn public_key(&self) -> PublicKey {
        self.keypair.public_key()
    }
}

/// Network peer role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// Active peers receive broadcast messages.
    Active,
    /// Passive peers are excluded from broadcasts.
    ///
    /// Note however that passive peers can be addressed directly in
    /// unicast or multicast operations.
    Passive,
}

impl Role {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => f.write_str("active"),
            Self::Passive => f.write_str("passive"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(u16);

impl From<u16> for Version {
    fn from(v: u16) -> Self {
        Self(v)
    }
}

impl From<Version> for u16 {
    fn from(v: Version) -> Self {
        v.0
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A variant of `timeout` that merges the timeout error into network error.
async fn until<F, A, E>(t: Duration, fut: F) -> Result<A, NetworkError>
where
    F: Future<Output = Result<A, E>>,
    E: Into<NetworkError>,
{
    match tokio::time::timeout(t, fut).await {
        Ok(Ok(a)) => Ok(a),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => Err(NetworkError::Timeout),
    }
}
