mod addr;
mod connection;
mod msg;
mod net;
mod queue;
mod time;

pub mod error;
pub mod x25519;

use std::{fmt, num::NonZeroUsize, sync::Arc, time::Duration};

pub use addr::NetAddr;
use bon::Builder;
pub use msg::Slot;
pub use net::{
    Network, NetworkController, NetworkReceiver, RetryPolicy, SendAction, SendCommand,
    SendCommandBuilder,
};

use crate::x25519::{Keypair, PublicKey};

#[derive(Debug, Builder)]
pub struct Config {
    /// Network name.
    #[builder(with = |s: impl Into<String>| Arc::new(s.into()))]
    name: Arc<String>,

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

    #[builder(default = Duration::from_secs(30))]
    connect_timeout: Duration,

    #[builder(default = Duration::from_secs(10))]
    handshake_timeout: Duration,

    #[builder(default = Duration::from_secs(30))]
    receive_timeout: Duration,
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
