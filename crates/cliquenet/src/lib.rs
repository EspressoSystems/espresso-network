mod chan;
mod error;
mod frame;
mod id;
mod net;
mod time;

#[cfg(feature = "metrics")]
mod metrics;

pub mod retry;

use std::{cmp::max, fmt, num::NonZeroUsize, sync::Arc};

use bon::Builder;
pub use error::{NetworkDown, NetworkError};
#[cfg(feature = "metrics")]
use hotshot_types::traits::metrics::Metrics;
use hotshot_types::{
    addr::NetAddr,
    x25519::{Keypair, PublicKey},
};
pub use id::Id;
pub use net::Network;
pub use retry::Retry;
use tokio::sync::Semaphore;

/// Max. number of bytes for a message (potentially consisting of several frames).
pub const MAX_MESSAGE_SIZE: usize = 8 * 1024 * 1024;

const NUM_DELAYS: usize = 5;
const LAST_DELAY: usize = NUM_DELAYS - 1;

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

#[derive(Debug, Builder)]
pub struct NetConf<K> {
    /// Network name.
    name: &'static str,

    /// Network public key.
    label: K,

    /// DH keypair
    keypair: Keypair,

    /// Address to bind to.
    bind: NetAddr,

    /// Committee members with key material and bind address.
    #[builder(with = <_>::from_iter)]
    parties: Vec<(K, PublicKey, NetAddr)>,

    /// Egress channel capacity per peer.
    #[builder(default = 64)]
    peer_capacity_egress: usize,

    /// Ingress channel capacity per peer.
    #[builder(default = 32)]
    peer_capacity_ingress: usize,

    /// Total egress channel capacity.
    #[builder(default = NonZeroUsize::new(max(peer_capacity_egress * parties.len(), 1)).unwrap())]
    total_capacity_egress: NonZeroUsize,

    /// Total ingress channel capacity.
    #[builder(default = NonZeroUsize::new(max(peer_capacity_ingress * parties.len(), 1)).unwrap())]
    total_capacity_ingress: NonZeroUsize,

    /// Max. number of bytes per message to send or receive.
    #[builder(default = MAX_MESSAGE_SIZE)]
    max_message_size: usize,

    /// Default retry delays in seconds.
    #[builder(default = [1, 3, 5, 15, 30])]
    retry_delays: [u8; NUM_DELAYS],

    #[cfg(feature = "metrics")]
    metrics: Box<dyn Metrics>,
}

impl<K> NetConf<K> {
    fn new_budget(&self) -> Arc<Semaphore> {
        Arc::new(Semaphore::new(self.peer_capacity_ingress))
    }
}
