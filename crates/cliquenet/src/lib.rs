mod addr;
mod connection;
mod metrics;
mod msg;
mod net;
mod queue;
mod time;
mod util;

pub mod error;
pub mod noise;
pub mod x25519;

use std::{collections::BTreeMap, fmt, num::NonZeroUsize, sync::Arc, time::Duration};

pub use addr::NetAddr;
use bon::Builder;
pub use error::NetworkError;
pub use metrics::Metrics;
pub use msg::Slot;
pub use net::{
    CmdSender, Network, NetworkController, NetworkReceiver, RetryPolicy, SendAction, SendCommand,
    SendCommandBuilder,
};

use crate::x25519::{Keypair, PublicKey};

#[derive(Builder)]
#[non_exhaustive]
pub struct Config {
    /// Network name.
    #[builder(with = |s: impl Into<String>| Arc::new(s.into()))]
    name: Arc<String>,

    /// The supported noise protocols.
    ///
    /// Nodes negotiate a common supported version which implies the exact
    /// noise protocol parameters they are going to use for the subsequent
    /// handshake.
    ///
    /// With this map, a node specifies the noise protocol names it supports
    /// per version number.
    #[builder(with = |xs: impl IntoIterator<Item = (Version, noise::Protocol)>| {
        let m = BTreeMap::from_iter(xs);
        assert! {
            !m.is_empty(),
            "at least one noise protocol is required"
        }
        assert! {
            m.keys().zip(m.keys().skip(1)).all(|(a, b)| u16::from(*a) + 1 == u16::from(*b)),
            "noise protocol versions must be consecutive"
        }
        m
    })]
    noise_protocols: BTreeMap<Version, noise::Protocol>,

    /// DH keypair
    keypair: Keypair,

    /// Address to bind to.
    bind: NetAddr,

    /// Network members with public key and network address.
    #[builder(with = <_>::from_iter)]
    parties: Vec<(PublicKey, NetAddr)>,

    #[builder(default = NonZeroUsize::new(100).expect("100 > 0"))]
    peer_budget: NonZeroUsize,

    /// Max. number of bytes per message to send or receive.
    #[builder(default = NonZeroUsize::new(10485760).expect("10485760 > 0"))]
    max_message_size: NonZeroUsize,

    /// Retry delays in seconds.
    #[builder(default = vec![1, 3, 5, 15, 30])]
    retry_delays: Vec<u8>,

    #[builder(default = Duration::from_secs(30))]
    max_retry_delay: Duration,

    /// Randomly delay the initial connect attempt between 0 and 1s.
    #[builder(default = true)]
    random_connect_delay: bool,

    #[builder(default = Duration::from_secs(30))]
    connect_timeout: Duration,

    #[builder(default = Duration::from_secs(10))]
    handshake_timeout: Duration,

    #[builder(default = Duration::from_secs(30))]
    receive_timeout: Duration,

    #[builder(default = Duration::from_secs(30))]
    backoff_duration: Duration,

    metrics: Option<Arc<dyn Metrics>>,
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

    pub fn with_metrics<M: Metrics + 'static>(mut self, m: M) -> Self {
        self.metrics = Some(Arc::new(m));
        self
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
