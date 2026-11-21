//! What a deployment's store shares with its API instances and with other API services.
//!
//! Repairing API instances fill gaps from other API services through the [chain service](proto),
//! and learn their URLs from discovery records. What API instances serve besides blocks is in the
//! [`Tables`] of the chain store, which they read and trust, as they trust the blocks.

pub mod discovery;
pub mod records;
mod tables;

pub use tables::{BlocksFrontier, KEPT_STATES, RewardSet, StateRow, Tables};

/// The services of the API service's own, generated from `proto/espresso/a2/v1`.
pub mod proto {
    tonic::include_proto!("espresso.a2.v1");
}
