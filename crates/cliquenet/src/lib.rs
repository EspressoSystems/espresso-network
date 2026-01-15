mod addr;
mod chan;
mod error;
mod frame;
mod id;
mod net;
mod time;

pub mod retry;

pub use addr::{Address, InvalidAddress};
pub use error::NetworkError;
pub use id::Id;
pub use net::{NetConf, NetConfBuilder, Network};
pub use retry::Retry;

/// Max. number of bytes for a message (potentially consisting of several frames).
pub const MAX_MESSAGE_SIZE: usize = 5 * 1024 * 1024;

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
