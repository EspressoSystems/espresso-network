//! What a deployment's ingester shares with its API instances and with other API services.
//!
//! The ingester fills gaps from other API services through the [chain service](proto), and
//! learns their URLs from discovery records. It writes what API instances serve besides blocks
//! into the [`Tables`] of the chain store, which they read and trust, as they trust the blocks.

pub mod discovery;
pub mod records;
mod tables;

pub use tables::{BlocksFrontier, KEPT_STATES, RewardSet, StateRow, Tables};

/// The services of the API service's own, generated from `proto/espresso/a2/v1`.
pub mod proto {
    tonic::include_proto!("espresso.a2.v1");
}
