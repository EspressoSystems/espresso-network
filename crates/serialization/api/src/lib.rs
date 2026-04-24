// Generated code - committed to git for visibility
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

    // Data API types

    /// Namespace proof type (transactions + proof)
    type NamespaceProof;

    /// Incorrect encoding proof type
    type IncorrectEncodingProof;

    // Consensus API types

    /// State certificate type
    type StateCertificate;

    /// Stake table type
    type StakeTable;

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

    // Data API serialization methods

    /// Serialize implementation's NamespaceProof to proto NamespaceProofResponse
    fn serialize_namespace_proof(
        &self,
        value: &Self::NamespaceProof,
    ) -> anyhow::Result<NamespaceProofResponse>;

    /// Serialize implementation's IncorrectEncodingProof to proto IncorrectEncodingProofResponse
    fn serialize_incorrect_encoding_proof(
        &self,
        value: &Self::IncorrectEncodingProof,
    ) -> anyhow::Result<IncorrectEncodingProofResponse>;

    // Consensus API serialization methods

    /// Serialize implementation's StateCertificate to proto StateCertificateResponse
    fn serialize_state_certificate(
        &self,
        value: &Self::StateCertificate,
    ) -> anyhow::Result<StateCertificateResponse>;

    /// Serialize implementation's StakeTable to proto StakeTableResponse
    fn serialize_stake_table(&self, value: &Self::StakeTable)
    -> anyhow::Result<StakeTableResponse>;

    // Helper conversion methods (for building structured proto messages)

    /// Convert a PeerConfig to proto PeerConfig
    fn convert_peer_config(&self, peer: &Self::PeerConfig) -> anyhow::Result<PeerConfig>
    where
        Self::PeerConfig: Sized;

    /// Convert a light client certificate to proto LightClientStateUpdateCertificateV2
    fn convert_light_client_cert(
        &self,
        cert: &Self::LightClientCert,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2>
    where
        Self::LightClientCert: Sized;

    /// Convert a namespace proof to proto NsProof
    fn convert_ns_proof(&self, proof: &Self::NsProof) -> anyhow::Result<NsProof>
    where
        Self::NsProof: Sized;

    // Associated types for helper conversions
    type PeerConfig;
    type LightClientCert;
    type NsProof;
}
