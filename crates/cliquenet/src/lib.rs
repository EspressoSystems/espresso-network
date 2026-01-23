mod addr;
mod chan;
mod error;
mod frame;
mod id;
mod net;
mod time;
mod x25519;

pub mod retry;

use std::sync::Arc;

pub use addr::{Address, InvalidAddress};
use bon::Builder;
pub use error::NetworkError;
pub use id::Id;
pub use net::Network;
pub use retry::Retry;
use tokio::sync::Semaphore;
pub use x25519::{
    InvalidKeypair, InvalidPublicKey, InvalidSecretKey, Keypair, PublicKey, SecretKey,
};

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

#[derive(Debug, Builder)]
pub struct NetConf<K> {
    /// Network name.
    name: &'static str,

    /// Network public key.
    label: K,

    /// DH keypair
    keypair: Keypair,

    /// Address to bind to.
    bind: Address,

    /// Committee members with key material and bind address.
    #[builder(with = <_>::from_iter)]
    parties: Vec<(K, PublicKey, Address)>,

    /// Total egress channel capacity.
    #[builder(default = 64 * parties.len())]
    total_capacity_egress: usize,

    /// Total ingress channel capacity.
    #[builder(default = 32 * parties.len())]
    total_capacity_ingress: usize,

    /// Egress channel capacity per peer.
    #[builder(default = 64)]
    peer_capacity_egress: usize,

    /// Ingress channel capacity per peer.
    #[builder(default = 32)]
    peer_capacity_ingress: usize,

    /// Max. number of bytes per message to send or receive.
    #[builder(default = MAX_MESSAGE_SIZE)]
    max_message_size: usize,

    /// Default retry delays in seconds.
    #[builder(default = [1, 3, 5, 15, 30])]
    retry_delays: [u8; NUM_DELAYS],
}

impl<K> NetConf<K> {
    fn new_budget(&self) -> Arc<Semaphore> {
        Arc::new(Semaphore::new(self.peer_capacity_ingress))
    }
}
