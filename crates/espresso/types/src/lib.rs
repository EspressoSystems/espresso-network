pub mod v0;

// Re-export the latest major version compatibility types.
pub use v0::*;

pub mod eth_signature_key;
mod reference_tests;

/// Panics when compiled without the `node` feature. Node binaries call this at
/// startup so a build without consensus support fails immediately instead of
/// at the first gated consensus call (header validation or proposal, epoch
/// root update, L1 deposit fetch). The node binaries also require the feature
/// through their Cargo dependency on this crate, so this only fires if that
/// declaration is removed.
pub fn assert_node_feature() {
    #[cfg(not(feature = "node"))]
    panic!("espresso-types was compiled without the `node` feature");
}
