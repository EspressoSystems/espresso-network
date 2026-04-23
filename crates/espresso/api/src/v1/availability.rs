//! Availability API trait for v1
//!
//! Returns internal espresso-types (legacy, no OpenAPI docs).
//! Uses associated types to avoid importing espresso-types in this crate.

use async_trait::async_trait;
use serde::Serialize;

/// Block identifier variants for namespace proof queries
#[derive(Debug, Clone)]
pub enum BlockId {
    /// Query by block height
    Height(u64),
    /// Query by block hash (TaggedBase64 encoded)
    Hash(String),
    /// Query by payload hash (TaggedBase64 encoded)
    PayloadHash(String),
}

/// Availability API trait - returns internal types
///
/// Uses associated types to avoid importing espresso-types in this crate.
/// Types must be Serialize for JSON responses, but don't need JsonSchema.
#[async_trait]
pub trait AvailabilityApi {
    /// Type for namespace proof query data (must be serializable to JSON)
    type NamespaceProofQueryData: Serialize + Send + Sync;

    /// Type for incorrect encoding proof (must be serializable to JSON)
    type IncorrectEncodingProof: Serialize + Send + Sync;

    /// Type for light client state certificate V1 (must be serializable to JSON)
    type StateCertQueryDataV1: Serialize + Send + Sync;

    /// Type for light client state certificate V2 (must be serializable to JSON)
    type StateCertQueryDataV2: Serialize + Send + Sync;

    /// Get namespace proof for a given block
    ///
    /// Returns the transactions in the specified namespace along with a proof.
    /// The block can be identified by height, hash, or payload hash.
    ///
    /// # Arguments
    /// * `block_id` - Block identifier (height, hash, or payload-hash)
    /// * `namespace` - Namespace ID to query
    ///
    /// # Returns
    /// Returns `Ok(None)` if the namespace is not present in the block.
    async fn get_namespace_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<Option<Self::NamespaceProofQueryData>>;

    /// Get namespace proofs for a range of blocks
    ///
    /// Returns a list of namespace proof data for each block in the range [from, until).
    ///
    /// # Arguments
    /// * `from` - Starting block height (inclusive)
    /// * `until` - Ending block height (exclusive)
    /// * `namespace` - Namespace ID to query
    ///
    /// # Returns
    /// Vector of namespace proof data for each block in the range.
    /// The allowable length of the range may be restricted by an implementation-defined limit.
    async fn get_namespace_proof_range(
        &self,
        from: u64,
        until: u64,
        namespace: u32,
    ) -> anyhow::Result<Vec<Self::NamespaceProofQueryData>>;

    /// Stream namespace proofs starting from a given height
    ///
    /// Opens a WebSocket connection and streams namespace data from each block.
    ///
    /// # Arguments
    /// * `start_height` - Block height to start streaming from
    /// * `namespace` - Namespace ID to query
    ///
    /// # Note
    /// This endpoint is currently not implemented and will return an error.
    /// WebSocket streaming support is deferred to future implementation.
    async fn stream_namespace_proofs(
        &self,
        start_height: u64,
        namespace: u32,
    ) -> anyhow::Result<()> {
        let _ = (start_height, namespace);
        anyhow::bail!("WebSocket streaming not yet implemented")
    }

    /// Generate a proof of incorrect encoding for the given block
    ///
    /// This endpoint attempts to generate a proof that demonstrates a block was
    /// incorrectly encoded. It will only succeed if the block was actually maliciously
    /// encoded; correctly encoded blocks will return an error.
    ///
    /// # Arguments
    /// * `block_id` - Block identifier (height, hash, or payload-hash)
    /// * `namespace` - Namespace ID to generate proof for
    ///
    /// # Returns
    /// Returns the incorrect encoding proof if the block was incorrectly encoded,
    /// or an error if the block was correctly encoded or the namespace is not present.
    async fn get_incorrect_encoding_proof(
        &self,
        block_id: BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::IncorrectEncodingProof>;

    /// Get light client state update certificate (V1) for the given epoch
    ///
    /// The light client state update certificate consists of the list of Schnorr signatures
    /// of the light client state at the end of the epoch. This is used to update light client
    /// state in the contract so that it has the new stake table information for the next epoch.
    ///
    /// # Arguments
    /// * `epoch` - Epoch number to query
    ///
    /// # Returns
    /// State certificate data (V1 format) for the epoch
    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1>;

    /// Get light client state update certificate (V2) for the given epoch
    ///
    /// The light client state update certificate consists of the list of Schnorr signatures
    /// of the light client state at the end of the epoch. This is used to update light client
    /// state in the contract so that it has the new stake table information for the next epoch.
    ///
    /// V2 includes the auth_root field, which is calculated based on the Keccak-256 hash of
    /// the reward Merkle tree roots.
    ///
    /// # Arguments
    /// * `epoch` - Epoch number to query
    ///
    /// # Returns
    /// State certificate data (V2 format) for the epoch
    async fn get_state_cert_v2(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV2>;
}
