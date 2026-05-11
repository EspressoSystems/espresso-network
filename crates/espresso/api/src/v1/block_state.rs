//! Block State API trait for v1
//!
//! Returns internal espresso-types (legacy, no OpenAPI docs).
//! Uses associated types to avoid importing espresso-types in this crate.

use async_trait::async_trait;
use serde::Serialize;

/// Block State API trait - returns internal types
///
/// Exposes the BlockMerkleTree (arity 3) merklized state queries.
#[async_trait]
pub trait BlockStateApi {
    /// Type for block merkle proof (must be serializable to JSON)
    type BlockMerklePath: Serialize + Send + Sync;

    /// Get the current block-state height
    async fn get_block_state_height(&self) -> anyhow::Result<u64>;

    /// Get merkle path for a block at the given snapshot height and key (block index)
    async fn get_block_state_path(
        &self,
        height: u64,
        key: u64,
    ) -> anyhow::Result<Self::BlockMerklePath>;

    /// Get merkle path for a block by commitment string and key (block index)
    async fn get_block_state_path_by_commit(
        &self,
        commit: String,
        key: u64,
    ) -> anyhow::Result<Self::BlockMerklePath>;
}
