//! The a2 consensus node: consensus with its store, for the Espresso chain.

pub mod catchup;
pub mod decided_state;
pub mod initializer;
pub mod membership_store;
pub mod node;
pub mod node_state;
pub mod null_catchup;
pub mod state_signer;

use espresso_types::SeqTypes;

/// The catchup protocol of consensus nodes and API services, generated from
/// `proto/espresso/a2/catchup/v1`.
pub mod proto {
    tonic::include_proto!("espresso.a2.catchup.v1");
}
