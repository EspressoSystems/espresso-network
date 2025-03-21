//! Marketplace builder
//! It mainly provides this API service to hotshot proposers:
//! 1. Serves a proposer(leader)'s request to provide a bundle of transactions
//!
//! It also provides one API service to external users:
//! 1. Serves a user's request to submit a private transaction

pub mod hooks;
pub mod service;

// tracking the testing
#[cfg(test)]
pub mod testing;
