//! Consensus API trait for v2
//!
//! API structure:
//!
//! ```text
//! /v2/consensus/
//!   GET /state-certificate/{epoch}
//!   GET /stake-table/{epoch}
//! ```

use async_trait::async_trait;
use serialization_api::ApiSerializations;

/// Consensus API trait (v2)
#[async_trait]
pub trait ConsensusApi: ApiSerializations {
    /// Get state certificate for an epoch
    ///
    /// Returns the light client state update certificate for the specified epoch.
    /// Used to update light client state in L1 contracts with new stake table information.
    ///
    /// # Arguments
    /// * `epoch` - Epoch number
    async fn get_state_certificate(&self, epoch: u64) -> anyhow::Result<Self::StateCertificate>;

    /// Get stake table for an epoch
    ///
    /// Returns the stake table data for the specified epoch.
    ///
    /// # Arguments
    /// * `epoch` - Epoch number
    async fn get_stake_table(&self, epoch: u64) -> anyhow::Result<Self::StakeTable>;
}
