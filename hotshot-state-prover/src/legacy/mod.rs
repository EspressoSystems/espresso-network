//! Legacy prover

/// State verifier circuit builder
pub mod circuit;
/// Utilities for test
pub mod mock_ledger;
/// Prover service related functionalities
pub mod service;
/// SNARK proof generation
pub mod snark;

pub use snark::*;
