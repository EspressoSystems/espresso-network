//! Data API trait for v2
//!
//! API structure:
//!
//! ```text
//! /v2/data/finalized/
//!   GET /namespace-proof?id={ns_id}&block={height}
//!   GET /namespace-proof?id={ns_id}&from={from}&until={until}
//!   GET /incorrect-encoding-proof?id={ns_id}&block={height}
//! ```

use async_trait::async_trait;
use serialization_api::ApiSerializations;

/// Data API trait (v2)
#[async_trait]
pub trait DataApi: ApiSerializations {
    /// Get namespace proof for a single block
    ///
    /// Returns transactions and proof for the specified namespace in the given block.
    ///
    /// # Arguments
    /// * `namespace_id` - Namespace to query
    /// * `block_height` - Block height
    async fn get_namespace_proof(
        &self,
        namespace_id: u32,
        block_height: u64,
    ) -> anyhow::Result<Self::NamespaceProof>;

    /// Get namespace proofs for a range of blocks
    ///
    /// Returns transactions and proofs for each block in the range [from, until).
    ///
    /// # Arguments
    /// * `namespace_id` - Namespace to query
    /// * `from` - Starting block height (inclusive)
    /// * `until` - Ending block height (exclusive)
    async fn get_namespace_proof_range(
        &self,
        namespace_id: u32,
        from: u64,
        until: u64,
    ) -> anyhow::Result<Vec<Self::NamespaceProof>>;

    /// Get incorrect encoding proof for a block
    ///
    /// Returns a proof that the specified block was incorrectly encoded.
    /// Returns an error if the block was correctly encoded.
    ///
    /// # Arguments
    /// * `namespace_id` - Namespace to check
    /// * `block_height` - Block height
    async fn get_incorrect_encoding_proof(
        &self,
        namespace_id: u32,
        block_height: u64,
    ) -> anyhow::Result<Self::IncorrectEncodingProof>;
}
