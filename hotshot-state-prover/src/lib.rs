//! SNARK-assisted `HotShot` light client state update verification

/// State verifier circuit builder
pub mod circuit;
/// Utilities for test
pub mod mock_ledger;
/// Prover service related functionalities
pub mod service;
/// SNARK proof generation
pub mod snark;

/// Legacy prover
pub mod legacy;

#[cfg(test)]
mod test_utils;

pub use snark::*;
