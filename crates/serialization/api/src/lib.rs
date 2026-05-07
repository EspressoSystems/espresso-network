// Generated code - committed to git for visibility
pub mod v2 {
    include!("espresso.api.v2.rs");
}

pub use v2::*;

pub trait ApiSerializations {
    type Address;
    type RewardClaimInput;
    type RewardBalance;
    type RewardAccountQueryData;
    type RewardBalances;
    type RewardMerkleTreeData;
    type NamespaceProof;
    type IncorrectEncodingProof;
    type StateCertificate;
    type StakeTable;
    type PeerConfig;
    type LightClientCert;
    type NsProof;

    fn deserialize_address(&self, s: &str) -> anyhow::Result<Self::Address>;

    fn serialize_reward_claim_input(
        &self,
        address: &str,
        value: &Self::RewardClaimInput,
    ) -> anyhow::Result<RewardClaimInput>;

    fn serialize_reward_balance(
        &self,
        value: &Self::RewardBalance,
    ) -> anyhow::Result<RewardBalance>;

    fn serialize_reward_account_query_data(
        &self,
        value: &Self::RewardAccountQueryData,
    ) -> anyhow::Result<RewardAccountQueryDataV2>;

    fn serialize_reward_balances(
        &self,
        value: &Self::RewardBalances,
    ) -> anyhow::Result<RewardBalances>;

    fn serialize_reward_merkle_tree_data(
        &self,
        value: &Self::RewardMerkleTreeData,
    ) -> anyhow::Result<RewardMerkleTreeV2Data>;

    fn serialize_namespace_proof(
        &self,
        value: &Self::NamespaceProof,
    ) -> anyhow::Result<NamespaceProofResponse>;

    fn serialize_incorrect_encoding_proof(
        &self,
        value: &Self::IncorrectEncodingProof,
    ) -> anyhow::Result<IncorrectEncodingProofResponse>;

    fn serialize_state_certificate(
        &self,
        value: &Self::StateCertificate,
    ) -> anyhow::Result<StateCertificateResponse>;

    fn serialize_stake_table(&self, value: &Self::StakeTable)
    -> anyhow::Result<StakeTableResponse>;

    fn serialize_peer_config(&self, peer: &Self::PeerConfig) -> anyhow::Result<PeerConfig>
    where
        Self::PeerConfig: Sized;

    fn serialize_light_client_cert(
        &self,
        cert: &Self::LightClientCert,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2>
    where
        Self::LightClientCert: Sized;

    fn serialize_ns_proof(&self, proof: &Self::NsProof) -> anyhow::Result<NsProof>
    where
        Self::NsProof: Sized;
}
