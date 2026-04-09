//! Generated API schema types from protobuf schemas.
//!
//! **DO NOT MODIFY FILES IN THIS CRATE MANUALLY**
//!
//! Types are generated from .proto files in the `proto/` directory.
//! To change API schemas, edit the .proto files and run: cargo build -p serialization-api
//!
//! Generated Rust types are committed to git for visibility in code review.

// Generated code - committed to git for visibility in code review
pub mod v2 {
    include!("espresso.api.v2.rs");
}

pub use v2::*;

/// Trait for converting between implementation types and proto serialization types
///
/// Implementations define their own internal types for addresses and rewards data,
/// then provide conversions to/from the proto types for API serialization.
pub trait ApiSerializations {
    // Request types (implementation-defined)

    /// Address type used by the implementation
    type Address;

    // Response types (implementation-defined)

    /// Reward claim input type
    type RewardClaimInput;

    /// Reward balance type
    type RewardBalance;

    /// Reward account query data type (balance + proof)
    type RewardAccountQueryData;

    /// Paginated reward balances type
    type RewardBalances;

    /// Reward merkle tree snapshot data type
    type RewardMerkleTreeData;

    // Deserialize proto/string types → internal types

    /// Deserialize an address string from a proto request into the implementation's Address type
    fn deserialize_address(&self, s: &str) -> anyhow::Result<Self::Address>;

    // Serialize internal types → proto types

    /// Serialize implementation's RewardClaimInput to proto RewardClaimInput
    ///
    /// Takes the original address string since the internal type may not contain it
    fn serialize_reward_claim_input(
        &self,
        address: &str,
        value: &Self::RewardClaimInput,
    ) -> anyhow::Result<RewardClaimInput>;

    /// Serialize implementation's RewardBalance to proto RewardBalance
    fn serialize_reward_balance(
        &self,
        value: &Self::RewardBalance,
    ) -> anyhow::Result<RewardBalance>;

    /// Serialize implementation's RewardAccountQueryData to proto RewardAccountQueryDataV2
    fn serialize_reward_account_query_data(
        &self,
        value: &Self::RewardAccountQueryData,
    ) -> anyhow::Result<RewardAccountQueryDataV2>;

    /// Serialize implementation's RewardBalances to proto RewardBalances
    fn serialize_reward_balances(
        &self,
        value: &Self::RewardBalances,
    ) -> anyhow::Result<RewardBalances>;

    /// Serialize implementation's RewardMerkleTreeData to proto RewardMerkleTreeV2Data
    fn serialize_reward_merkle_tree_data(
        &self,
        value: &Self::RewardMerkleTreeData,
    ) -> anyhow::Result<RewardMerkleTreeV2Data>;
}
