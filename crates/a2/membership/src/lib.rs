//! Membership as both processes of a deployment keep it: the stake tables, DRB results and epoch
//! roots that consensus and the ingester derive from the chain and L1, kept in the application
//! tables of their store, and the node state built on them.

pub mod node_state;
pub mod null_catchup;
pub mod store;

pub use store::{MembershipStore, RETAINED_EPOCHS};
