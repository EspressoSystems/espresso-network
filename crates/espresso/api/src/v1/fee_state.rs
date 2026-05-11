//! Fee State API trait for v1
//!
//! Returns internal espresso-types (legacy, no OpenAPI docs).
//! Uses associated types to avoid importing espresso-types in this crate.

use async_trait::async_trait;
use serde::Serialize;

/// Fee State API trait - returns internal types
///
/// Exposes the FeeMerkleTree (arity 256) merklized state queries.
#[async_trait]
pub trait FeeStateApi {
    /// Type for fee merkle proof (must be serializable to JSON)
    type FeeMerklePath: Serialize + Send + Sync;

    /// Type for fee balance (must be serializable to JSON)
    type FeeBalance: Serialize + Send + Sync;

    /// Get the current fee-state height
    async fn get_fee_state_height(&self) -> anyhow::Result<u64>;

    /// Get merkle path for a fee account at the given snapshot height
    async fn get_fee_state_path(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::FeeMerklePath>;

    /// Get merkle path for a fee account by commitment string
    async fn get_fee_state_path_by_commit(
        &self,
        commit: String,
        address: String,
    ) -> anyhow::Result<Self::FeeMerklePath>;

    /// Get the latest fee balance for an address
    async fn get_fee_balance(&self, address: String) -> anyhow::Result<Option<Self::FeeBalance>>;
}
