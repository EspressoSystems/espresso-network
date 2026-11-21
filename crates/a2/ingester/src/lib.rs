//! The ingester: repairs what the store of a deployment lacks, once however many API instances
//! serve the store.
//!
//! The [`GapFiller`] fills final gaps and checks the certificates of filled heights from other
//! API services, and the [`ChainFollower`] writes the state of the recent heights consensus sent
//! none for. Membership, derived from L1, serves both. The chain itself, with the state, stake
//! tables and discovery records consensus derives, reaches the store through the storage
//! instances.

mod certs;
mod gaps;
mod membership;
mod peers;
mod state;
mod state_cert;

pub use gaps::GapFiller;
pub use membership::{Membership, node_state};
pub use peers::Peers;
pub use state::ChainFollower;
