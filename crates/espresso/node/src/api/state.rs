//! RewardApi trait implementations for espresso-node
//!
//! This module provides implementations for both v1::RewardApi (internal types)
//! and v2::RewardApi (proto types), backed by the same data source.

use std::time::Duration;

use alloy::primitives::U256;
use async_trait::async_trait;
use espresso_api::{error::AvailabilityError, v1::HotShotAvailabilityApi};
use espresso_types::{
    NamespaceId, NamespaceProofQueryData, NsProof, SeqTypes,
    v0::sparse_mt::KeccakNode,
    v0_3::RewardAmount as InternalRewardAmount,
    v0_4::{
        RewardAccountProofV2 as InternalRewardAccountProofV2,
        RewardAccountQueryDataV2 as InternalRewardAccountQueryData, RewardAccountV2,
        RewardMerkleProofV2 as InternalRewardMerkleProofV2,
    },
    v0_6::RewardClaimError,
};
use futures::{StreamExt as _, join, stream::BoxStream};
use hotshot_contract_adapter::reward::RewardClaimInput as InternalRewardClaimInput;
use hotshot_new_protocol::message::Certificate2;
use hotshot_query_service::{
    Header as HsHeader,
    availability::{
        AvailabilityDataSource, BlockId as HsBlockId, BlockQueryData, BlockSummaryQueryData,
        LeafId as HsLeafId, LeafQueryData, Limits as HsLimits, PayloadQueryData,
        QueryablePayload as _, TransactionQueryData, TransactionWithProofQueryData,
        VidCommonQueryData,
    },
    node::NodeDataSource as _,
    types::HeightIndexed as _,
};
use hotshot_types::{
    data::{EpochNumber, VidShare},
    vid::avidm::AvidMShare,
};
use jf_merkle_tree_compat::prelude::{
    MerkleNode as InternalMerkleNode, MerkleProof as InternalMerkleProof,
};
use serde_json;
use serialization_api::v2::{
    self, RewardAccountProofV2, RewardAccountQueryDataV2, RewardBalance, RewardBalances,
    RewardClaimInput, RewardMerkleProofV2, RewardMerkleTreeV2Data, merkle_node,
    reward_merkle_proof_v2::ProofType,
};
use tagged_base64::TaggedBase64;

use super::{
    RewardMerkleTreeDataSource, RewardMerkleTreeV2Data as InternalRewardTreeData,
    data_source::{
        RequestResponseDataSource as _, StakeTableDataSource, StateCertDataSource,
        StateCertFetchingDataSource, StateSignatureDataSource,
    },
};

/// Node API state implementation
///
/// This struct implements both v1::RewardApi (internal types) and v2::RewardApi (proto types).
#[derive(Clone)]
pub struct NodeApiStateImpl<D> {
    data_source: D,
    env_vars: std::sync::Arc<Vec<String>>,
    public_node_config: Option<std::sync::Arc<crate::options::PublicNodeConfig>>,
}

impl<D> NodeApiStateImpl<D> {
    pub fn new(data_source: D) -> Self {
        Self {
            data_source,
            env_vars: std::sync::Arc::new(Vec::new()),
            public_node_config: None,
        }
    }

    pub fn with_env_vars(mut self, env_vars: Vec<String>) -> Self {
        self.env_vars = std::sync::Arc::new(env_vars);
        self
    }

    pub fn with_public_node_config(
        mut self,
        config: Option<crate::options::PublicNodeConfig>,
    ) -> Self {
        self.public_node_config = config.map(std::sync::Arc::new);
        self
    }

    /// Convert RewardAccountProofV2 to proto
    fn convert_reward_account_proof_v2(
        &self,
        proof: &InternalRewardAccountProofV2,
    ) -> anyhow::Result<RewardAccountProofV2> {
        Ok(RewardAccountProofV2 {
            account: format!("{:#x}", proof.account),
            proof: Some(self.convert_reward_merkle_proof_v2(&proof.proof)?),
        })
    }

    /// Convert RewardMerkleProofV2 enum to proto
    fn convert_reward_merkle_proof_v2(
        &self,
        proof: &InternalRewardMerkleProofV2,
    ) -> anyhow::Result<RewardMerkleProofV2> {
        let proof_type = match proof {
            InternalRewardMerkleProofV2::Presence(p) => {
                ProofType::Presence(self.convert_merkle_proof(p)?)
            },
            InternalRewardMerkleProofV2::Absence(p) => {
                ProofType::Absence(self.convert_merkle_proof(p)?)
            },
        };

        Ok(RewardMerkleProofV2 {
            proof_type: Some(proof_type),
        })
    }

    /// Convert MerkleProof to proto
    fn convert_merkle_proof(
        &self,
        proof: &InternalMerkleProof<InternalRewardAmount, RewardAccountV2, KeccakNode, 2>,
    ) -> anyhow::Result<v2::MerkleProof> {
        let proof_nodes: Result<Vec<v2::MerkleNode>, _> = proof
            .proof
            .iter()
            .map(|node| self.convert_merkle_node(node))
            .collect();

        Ok(v2::MerkleProof {
            pos: TaggedBase64::new("FIELD", proof.pos.0.as_slice())
                .map_err(|e| anyhow::anyhow!("failed to encode proof pos: {}", e))?
                .to_string(),
            proof: proof_nodes?,
        })
    }

    /// Convert MerkleNode to proto (recursive)
    fn convert_merkle_node(
        &self,
        node: &InternalMerkleNode<InternalRewardAmount, RewardAccountV2, KeccakNode>,
    ) -> anyhow::Result<v2::MerkleNode> {
        let node_type = match node {
            InternalMerkleNode::Empty => merkle_node::NodeType::Empty(v2::Empty {
                dummy: Some(v2::EmptyData {}),
            }),
            InternalMerkleNode::Leaf { pos, elem, value } => {
                merkle_node::NodeType::Leaf(v2::Leaf {
                    pos: TaggedBase64::new("FIELD", pos.0.as_slice())
                        .map_err(|e| anyhow::anyhow!("failed to encode leaf pos: {}", e))?
                        .to_string(),
                    elem: TaggedBase64::new("FIELD", &elem.0.to_le_bytes::<32>())
                        .map_err(|e| anyhow::anyhow!("failed to encode leaf elem: {}", e))?
                        .to_string(),
                    value: TaggedBase64::new("FIELD", &value.0)
                        .map_err(|e| anyhow::anyhow!("failed to encode leaf value: {}", e))?
                        .to_string(),
                })
            },
            InternalMerkleNode::Branch { value, children } => {
                let proto_children: Result<Vec<v2::MerkleNode>, _> = children
                    .iter()
                    .map(|child| self.convert_merkle_node(child))
                    .collect();

                merkle_node::NodeType::Branch(v2::Branch {
                    value: TaggedBase64::new("FIELD", &value.0)
                        .map_err(|e| anyhow::anyhow!("failed to encode branch value: {}", e))?
                        .to_string(),
                    children: proto_children?,
                })
            },
            InternalMerkleNode::ForgettenSubtree { value } => {
                merkle_node::NodeType::ForgottenSubtree(v2::ForgottenSubtree {
                    value: TaggedBase64::new("FIELD", &value.0)
                        .map_err(|e| {
                            anyhow::anyhow!("failed to encode forgotten subtree value: {}", e)
                        })?
                        .to_string(),
                })
            },
        };

        Ok(v2::MerkleNode {
            node_type: Some(node_type),
        })
    }
}

// ============================================================================
// ApiSerializations implementation (conversion layer)
// ============================================================================

impl<D> serialization_api::ApiSerializations for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Send + Sync + 'static,
    D::Target: RewardMerkleTreeDataSource + Send + Sync,
{
    // Request types
    type Address = alloy::primitives::Address;

    // Response types (internal types)
    type RewardClaimInput = InternalRewardClaimInput;
    type RewardBalance = U256;
    type RewardAccountQueryData = InternalRewardAccountQueryData;
    type RewardBalances = (Vec<(RewardAccountV2, InternalRewardAmount)>, u64); // (amounts, total)
    type RewardMerkleTreeData = InternalRewardTreeData;

    // Data API types
    type NamespaceProof = espresso_types::NamespaceProofQueryData;
    type IncorrectEncodingProof = espresso_types::v0_3::AvidMIncorrectEncodingNsProof;

    // Consensus API types
    type StateCertificate = espresso_types::StateCertQueryDataV2<espresso_types::SeqTypes>;
    type StakeTable = Vec<hotshot_types::PeerConfig<espresso_types::SeqTypes>>;

    // Helper conversion types
    type PeerConfig = hotshot_types::PeerConfig<espresso_types::SeqTypes>;
    type LightClientCert = hotshot_types::simple_certificate::LightClientStateUpdateCertificateV2<
        espresso_types::SeqTypes,
    >;
    type NsProof = espresso_types::NsProof;

    fn deserialize_address(&self, s: &str) -> anyhow::Result<Self::Address> {
        s.parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", s))
    }

    // Serialize internal types → proto types
    fn serialize_reward_claim_input(
        &self,
        address: &str,
        value: &Self::RewardClaimInput,
    ) -> anyhow::Result<RewardClaimInput> {
        // Serialize auth_data directly - it serializes to a hex string via serde
        let auth_data = serde_json::to_string(&value.auth_data)
            .map_err(|e| anyhow::anyhow!("failed to serialize auth_data: {}", e))?
            // Remove quotes added by JSON string serialization
            .trim_matches('"')
            .to_string();

        Ok(RewardClaimInput {
            address: address.to_string(),
            lifetime_rewards: format!("{:#x}", value.lifetime_rewards), // Hex for contract
            auth_data,
        })
    }

    fn serialize_reward_balance(
        &self,
        value: &Self::RewardBalance,
    ) -> anyhow::Result<RewardBalance> {
        Ok(RewardBalance {
            amount: value.to_string(), // Decimal string
        })
    }

    fn serialize_reward_account_query_data(
        &self,
        value: &Self::RewardAccountQueryData,
    ) -> anyhow::Result<RewardAccountQueryDataV2> {
        // Convert balance to decimal string
        let balance = value.balance.to_string();

        // Convert the proof
        let proof = Some(self.convert_reward_account_proof_v2(&value.proof)?);

        Ok(RewardAccountQueryDataV2 { balance, proof })
    }

    fn serialize_reward_balances(
        &self,
        value: &Self::RewardBalances,
    ) -> anyhow::Result<RewardBalances> {
        let (amounts_vec, total) = value;

        // Convert each account/amount pair to proto format
        let amounts = amounts_vec
            .iter()
            .map(|(account, amount)| serialization_api::v2::RewardAmount {
                address: format!("{:#x}", account.0),
                amount: amount.0.to_string(), // Decimal string
            })
            .collect();

        Ok(RewardBalances {
            amounts,
            total: *total,
        })
    }

    fn serialize_reward_merkle_tree_data(
        &self,
        value: &Self::RewardMerkleTreeData,
    ) -> anyhow::Result<RewardMerkleTreeV2Data> {
        let bytes = bincode::serialize(value)
            .map_err(|e| anyhow::anyhow!("failed to serialize RewardMerkleTreeV2Data: {}", e))?;
        Ok(RewardMerkleTreeV2Data { data: bytes })
    }

    // Data API serialization methods

    fn serialize_namespace_proof(
        &self,
        value: &Self::NamespaceProof,
    ) -> anyhow::Result<v2::NamespaceProofResponse> {
        // Serialize each transaction field explicitly using base64_bytes
        let transactions: Vec<v2::Transaction> = value
            .transactions
            .iter()
            .map(|tx| -> anyhow::Result<v2::Transaction> {
                let mut payload_bytes = Vec::new();
                base64_bytes::serialize(
                    &tx.payload,
                    &mut serde_json::Serializer::new(&mut payload_bytes),
                )
                .map_err(|e| anyhow::anyhow!("failed to serialize payload: {}", e))?;
                // Convert to string and remove quotes added by JSON serializer
                let payload_str = String::from_utf8(payload_bytes)?
                    .trim_matches('"')
                    .to_string();

                Ok(v2::Transaction {
                    namespace: tx.namespace.0,
                    payload: payload_str,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let proof = value
            .proof
            .as_ref()
            .map(|p| self.serialize_ns_proof(p))
            .transpose()?;

        Ok(serialization_api::v2::NamespaceProofResponse {
            transactions,
            proof,
        })
    }

    fn serialize_incorrect_encoding_proof(
        &self,
        value: &Self::IncorrectEncodingProof,
    ) -> anyhow::Result<v2::IncorrectEncodingProofResponse> {
        // Serialize the VID proof to JSON string
        let proof_data = serde_json::to_string(&value.0)?;
        Ok(serialization_api::v2::IncorrectEncodingProofResponse {
            proof: Some(v2::AvidMIncorrectEncodingNsProof { proof_data }),
        })
    }

    // Consensus API serialization methods

    fn serialize_state_certificate(
        &self,
        value: &Self::StateCertificate,
    ) -> anyhow::Result<v2::StateCertificateResponse> {
        let certificate = self.serialize_light_client_cert(&value.0)?;

        Ok(serialization_api::v2::StateCertificateResponse {
            certificate: Some(certificate),
        })
    }

    fn serialize_stake_table(
        &self,
        value: &Self::StakeTable,
    ) -> anyhow::Result<v2::StakeTableResponse> {
        let peers: Result<Vec<_>, _> = value
            .iter()
            .map(|peer| self.serialize_peer_config(peer))
            .collect();

        Ok(serialization_api::v2::StakeTableResponse { peers: peers? })
    }

    fn serialize_peer_config(&self, peer: &Self::PeerConfig) -> anyhow::Result<v2::PeerConfig> {
        let stake_table_entry = v2::StakeTableEntry {
            stake_key: Some(v2::BlsPublicKey {
                key: peer.stake_table_entry.stake_key.to_string(),
            }),
            stake_amount: peer.stake_table_entry.stake_amount.to_string(),
        };

        let state_ver_key = v2::SchnorrPublicKey {
            key: peer.state_ver_key.to_string(),
        };

        let connect_info = peer.connect_info.as_ref().map(|info| {
            let p2p_addr = match &info.p2p_addr {
                hotshot_types::addr::NetAddr::Inet(ip, port) => v2::NetAddr {
                    addr_type: Some(v2::net_addr::AddrType::Inet(v2::InetAddr {
                        host: match ip {
                            std::net::IpAddr::V4(_) => ip.to_string(),
                            std::net::IpAddr::V6(_) => format!("[{ip}]"),
                        },
                        port: *port as u32,
                    })),
                },
                hotshot_types::addr::NetAddr::Name(name, port) => v2::NetAddr {
                    addr_type: Some(v2::net_addr::AddrType::Name(v2::NameAddr {
                        name: name.to_string(),
                        port: *port as u32,
                    })),
                },
            };

            v2::PeerConnectInfo {
                x25519_key: info.x25519_key.to_string(),
                p2p_addr: Some(p2p_addr),
            }
        });

        Ok(v2::PeerConfig {
            stake_table_entry: Some(stake_table_entry),
            state_ver_key: Some(state_ver_key),
            connect_info,
        })
    }

    fn serialize_light_client_cert(
        &self,
        cert: &Self::LightClientCert,
    ) -> anyhow::Result<v2::LightClientStateUpdateCertificateV2> {
        let signatures: Result<Vec<_>, anyhow::Error> = cert
            .signatures
            .iter()
            .map(
                |(key, lcv3_sig, lcv2_sig)| -> anyhow::Result<v2::StateSignatureTuple> {
                    Ok(v2::StateSignatureTuple {
                        state_signature_key: Some(v2::SchnorrPublicKey {
                            key: key.to_string(),
                        }),
                        lcv3_signature: lcv3_sig.to_string(),
                        lcv2_signature: lcv2_sig.to_string(),
                    })
                },
            )
            .collect();

        Ok(v2::LightClientStateUpdateCertificateV2 {
            epoch: cert.epoch.u64(),
            light_client_state: cert.light_client_state.to_string(),
            next_stake_table_state: cert.next_stake_table_state.to_string(),
            signatures: signatures?,
            auth_root: cert.auth_root.to_string(),
        })
    }

    fn serialize_ns_proof(&self, proof: &Self::NsProof) -> anyhow::Result<v2::NsProof> {
        let proof_version = match proof {
            NsProof::V0(advz_proof) => {
                // Serialize the inner fields directly
                let json = serde_json::json!({
                    "ns_index": advz_proof.ns_index,
                    "ns_payload": advz_proof.ns_payload,
                    "ns_proof": advz_proof.ns_proof,
                });
                v2::ns_proof::ProofVersion::V0(serde_json::from_value(json)?)
            },
            NsProof::V1(avidm_proof) => {
                // Serialize ns_payload using base64_bytes
                let mut ns_payload_bytes = Vec::new();
                base64_bytes::serialize(
                    &avidm_proof.0.ns_payload,
                    &mut serde_json::Serializer::new(&mut ns_payload_bytes),
                )
                .map_err(|e| anyhow::anyhow!("failed to serialize ns_payload: {}", e))?;
                let ns_payload_str = String::from_utf8(ns_payload_bytes)?
                    .trim_matches('"')
                    .to_string();

                v2::ns_proof::ProofVersion::V1(v2::AvidMNsProof {
                    ns_index: avidm_proof.0.ns_index as u64,
                    ns_payload: ns_payload_str,
                    ns_proof: avidm_proof.0.ns_proof.to_string(),
                })
            },
            NsProof::V1IncorrectEncoding(incorrect_proof) => {
                // Serialize the whole proof to JSON string
                v2::ns_proof::ProofVersion::V1IncorrectEncoding(v2::AvidMIncorrectEncodingNsProof {
                    proof_data: serde_json::to_string(&incorrect_proof.0)?,
                })
            },
            NsProof::V2(gf2_proof) => {
                // Serialize ns_payload using base64_bytes
                let mut ns_payload_bytes = Vec::new();
                base64_bytes::serialize(
                    &gf2_proof.0.ns_payload,
                    &mut serde_json::Serializer::new(&mut ns_payload_bytes),
                )
                .map_err(|e| anyhow::anyhow!("failed to serialize ns_payload: {}", e))?;
                let ns_payload_str = String::from_utf8(ns_payload_bytes)?
                    .trim_matches('"')
                    .to_string();

                v2::ns_proof::ProofVersion::V2(v2::AvidmGf2NsProof {
                    ns_index: gf2_proof.0.ns_index as u64,
                    ns_payload: ns_payload_str,
                    ns_proof: gf2_proof.0.ns_proof.to_string(),
                })
            },
        };

        Ok(v2::NsProof {
            proof_version: Some(proof_version),
        })
    }
}

// ============================================================================
// RewardApiV2 implementation (business logic)
// ============================================================================

#[async_trait]
impl<D> espresso_api::v2::RewardApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: RewardMerkleTreeDataSource + Send + Sync,
{
    async fn get_reward_claim_input(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardClaimInput> {
        // Load the latest reward account proof from the data source
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account {:?}: {}",
                    address,
                    err
                )
            })?;

        // Convert the proof to reward claim input and return internal type
        proof.to_reward_claim_input().map_err(|err| match err {
            RewardClaimError::ZeroRewardError => {
                anyhow::anyhow!("zero reward balance for {:?}", address)
            },
            RewardClaimError::ProofConversionError(e) => {
                anyhow::anyhow!("failed to create solidity proof for {:?}: {}", address, e)
            },
        })
    }

    async fn get_reward_balance(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardBalance> {
        // Load the latest reward account proof from the data source
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account {:?}: {}",
                    address,
                    err
                )
            })?;

        // Return internal balance type
        Ok(proof.balance)
    }

    async fn get_reward_account_proof(
        &self,
        address: Self::Address,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        // Load the latest reward account proof from the data source and return internal type
        self.data_source
            .load_latest_reward_account_proof_v2(address.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load latest reward account proof for {:?}: {}",
                    address,
                    err
                )
            })
    }

    async fn get_reward_balances(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::RewardBalances> {
        // Validate limit (from reward.toml: limit <= 10000)
        if limit > 10000 {
            return Err(anyhow::anyhow!(
                "limit {} exceeds maximum allowed value of 10000",
                limit
            ));
        }

        // Load the merkle tree at the given height
        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })?;

        // Deserialize the tree into internal format
        let tree_data: InternalRewardTreeData =
            bincode::deserialize(&tree_bytes).map_err(|err| {
                anyhow::anyhow!(
                    "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                    height,
                    err
                )
            })?;

        let offset_usize = offset as usize;
        let limit_usize = limit as usize;
        let total = tree_data.balances.len() as u64;

        // Validate offset is within bounds
        if offset_usize > tree_data.balances.len() {
            return Err(anyhow::anyhow!("offset {} out of bounds", offset));
        }

        let end = std::cmp::min(offset_usize + limit_usize, tree_data.balances.len());
        let slice = &tree_data.balances[offset_usize..end];

        // Reverse order (matching Tide implementation) and return internal type with total
        let reversed: Vec<_> = slice.iter().rev().copied().collect();
        Ok((reversed, total))
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeData> {
        // Load the raw merkle tree bytes
        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })?;

        // Deserialize and return internal type
        bincode::deserialize(&tree_bytes).map_err(|err| {
            anyhow::anyhow!(
                "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                height,
                err
            )
        })
    }
}

// ============================================================================
// RewardApiV1 implementation (internal types, no proto conversion)
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::RewardApi for NodeApiStateImpl<D>
where
    D: RewardMerkleTreeDataSource,
{
    type RewardClaimInput = InternalRewardClaimInput;
    type RewardBalance = InternalRewardAmount;
    type RewardAccountQueryData = InternalRewardAccountQueryData;
    type RewardAmounts = Vec<(alloy::primitives::Address, InternalRewardAmount)>;
    type RewardMerkleTreeData = Vec<u8>;

    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardClaimInput> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load the reward account proof from the data source
        let proof = self
            .data_source
            .load_reward_account_proof_v2(block_height, addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account {} at height {}: {}",
                    address,
                    block_height,
                    err
                )
            })?;

        // Convert the proof to reward claim input (internal type)
        let claim_input = proof.to_reward_claim_input().map_err(|err| match err {
            RewardClaimError::ZeroRewardError => {
                anyhow::anyhow!(
                    "zero reward balance for {} at height {}",
                    address,
                    block_height
                )
            },
            RewardClaimError::ProofConversionError(e) => {
                anyhow::anyhow!(
                    "failed to create solidity proof for {} at height {}: {}",
                    address,
                    block_height,
                    e
                )
            },
        })?;

        Ok(claim_input)
    }

    async fn get_reward_balance(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardBalance> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load the reward account proof from the data source
        let proof = self
            .data_source
            .load_reward_account_proof_v2(height, addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account {} at height {}: {}",
                    address,
                    height,
                    err
                )
            })?;

        Ok(InternalRewardAmount(proof.balance))
    }

    async fn get_latest_reward_balance(
        &self,
        address: String,
    ) -> anyhow::Result<Self::RewardBalance> {
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!("failed to load latest reward account {}: {}", address, err)
            })?;

        Ok(InternalRewardAmount(proof.balance))
    }

    async fn get_reward_account_proof(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load and return the reward account proof directly (internal type)
        let proof = self
            .data_source
            .load_reward_account_proof_v2(height, addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to load reward account {} at height {}: {}",
                    address,
                    height,
                    err
                )
            })?;

        Ok(proof)
    }

    async fn get_latest_reward_account_proof(
        &self,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryData> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load and return the latest reward account proof directly (internal type)
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!("failed to load latest reward account {}: {}", address, err)
            })?;

        Ok(proof)
    }

    async fn get_reward_amounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::RewardAmounts> {
        // Validate limit (from reward.toml: limit <= 10000)
        if limit > 10000 {
            return Err(anyhow::anyhow!(
                "limit {} exceeds maximum allowed value of 10000",
                limit
            ));
        }

        // Load the merkle tree at the given height
        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })?;

        // Deserialize the tree into internal format
        let tree_data: InternalRewardTreeData =
            bincode::deserialize(&tree_bytes).map_err(|err| {
                anyhow::anyhow!(
                    "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                    height,
                    err
                )
            })?;

        let offset_usize = offset as usize;
        let limit_usize = limit as usize;

        // Validate offset is within bounds
        if offset_usize > tree_data.balances.len() {
            return Err(anyhow::anyhow!("offset {} out of bounds", offset));
        }

        let end = std::cmp::min(offset_usize + limit_usize, tree_data.balances.len());
        let slice = &tree_data.balances[offset_usize..end];

        let result: Vec<(alloy::primitives::Address, InternalRewardAmount)> = slice
            .iter()
            .rev()
            .map(|(account, amount)| (account.0, *amount))
            .collect();

        Ok(result)
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeData> {
        self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })
    }
}

// ============================================================================
// v2::DataApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v2::DataApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: RewardMerkleTreeDataSource
        + hotshot_query_service::availability::AvailabilityDataSource<espresso_types::SeqTypes>
        + hotshot_query_service::node::NodeDataSource<espresso_types::SeqTypes>
        + super::data_source::RequestResponseDataSource<espresso_types::SeqTypes>
        + Sync
        + Send,
{
    async fn get_namespace_proof(
        &self,
        namespace_id: u64,
        block_height: u64,
    ) -> anyhow::Result<Self::NamespaceProof> {
        let ns_id = NamespaceId(namespace_id);
        let block_id = HsBlockId::Number(block_height as usize);

        // Fetch block and VID common data concurrently
        let ds = &*self.data_source;
        let timeout = std::time::Duration::from_millis(500);
        let (block_fetch, vid_fetch) = join!(ds.get_block(block_id), ds.get_vid_common(block_id));
        let (block_opt, vid_opt) = join!(
            block_fetch.with_timeout(timeout),
            vid_fetch.with_timeout(timeout)
        );

        let block = block_opt.ok_or_else(|| anyhow::anyhow!("block {} not found", block_height))?;
        let vid_common = vid_opt.ok_or_else(|| {
            anyhow::anyhow!("VID common data for block {} not found", block_height)
        })?;

        // Generate namespace proof
        let ns_table = block.payload().ns_table();
        let ns_index = ns_table.find_ns_id(&ns_id).ok_or_else(|| {
            anyhow::anyhow!(
                "namespace {} not present in block {}",
                namespace_id,
                block_height
            )
        })?;

        let proof =
            NsProof::new(block.payload(), &ns_index, vid_common.common()).ok_or_else(|| {
                anyhow::anyhow!(
                    "failed to generate namespace proof for block {}",
                    block_height
                )
            })?;

        let transactions = proof.export_all_txs(&ns_id);

        Ok(espresso_types::NamespaceProofQueryData {
            transactions,
            proof: Some(proof),
        })
    }

    async fn get_namespace_proof_range(
        &self,
        namespace_id: u64,
        from: u64,
        until: u64,
    ) -> anyhow::Result<Vec<Self::NamespaceProof>> {
        let ns_id = NamespaceId(namespace_id);

        // Validate range
        if until <= from {
            return Err(anyhow::anyhow!(
                "invalid range: until ({}) must be greater than from ({})",
                until,
                from
            ));
        }

        let range_size = until - from;
        const MAX_RANGE: u64 = 100; // Match limit from availability.toml
        if range_size > MAX_RANGE {
            return Err(anyhow::anyhow!(
                "range too large: {} blocks (max {})",
                range_size,
                MAX_RANGE
            ));
        }

        // Fetch blocks and VID common data for the range
        let (blocks_stream, vids_stream) = join!(
            self.data_source
                .get_block_range(from as usize..until as usize),
            self.data_source
                .get_vid_common_range(from as usize..until as usize)
        );

        let blocks: Vec<_> = blocks_stream
            .then(|block| async move { block.resolve().await })
            .collect()
            .await;
        let vids: Vec<_> = vids_stream
            .then(|vid| async move { vid.resolve().await })
            .collect()
            .await;

        if blocks.len() != vids.len() {
            return Err(anyhow::anyhow!(
                "mismatch between blocks and VID common data"
            ));
        }

        // Generate proofs for each block
        let mut proofs = Vec::new();

        for (block, vid) in blocks.into_iter().zip(vids) {
            let ns_table = block.payload().ns_table();

            // Check if namespace exists in this block
            if let Some(ns_index) = ns_table.find_ns_id(&ns_id) {
                if let Some(proof) = NsProof::new(block.payload(), &ns_index, vid.common()) {
                    let transactions = proof.export_all_txs(&ns_id);
                    proofs.push(espresso_types::NamespaceProofQueryData {
                        transactions,
                        proof: Some(proof),
                    });
                } else {
                    // Failed to generate proof - return empty result for this block
                    proofs.push(espresso_types::NamespaceProofQueryData {
                        transactions: vec![],
                        proof: None,
                    });
                }
            } else {
                // Namespace not present in this block
                proofs.push(espresso_types::NamespaceProofQueryData {
                    transactions: vec![],
                    proof: None,
                });
            }
        }

        Ok(proofs)
    }

    async fn get_incorrect_encoding_proof(
        &self,
        namespace_id: u64,
        block_height: u64,
    ) -> anyhow::Result<Self::IncorrectEncodingProof> {
        let ns_id = NamespaceId(namespace_id);
        let block_id = HsBlockId::Number(block_height as usize);

        let ds = &*self.data_source;
        let timeout = std::time::Duration::from_millis(500);
        let (block_fetch, vid_fetch) = join!(ds.get_block(block_id), ds.get_vid_common(block_id));
        let (block, vid_common) = join!(
            block_fetch.with_timeout(timeout),
            vid_fetch.with_timeout(timeout)
        );

        let block = block.ok_or_else(|| anyhow::anyhow!("block {} not found", block_height))?;
        let vid_common = vid_common.ok_or_else(|| {
            anyhow::anyhow!("VID common data for block {} not found", block_height)
        })?;

        let ns_table = block.payload().ns_table();
        let ns_index = ns_table.find_ns_id(&ns_id).ok_or_else(|| {
            anyhow::anyhow!(
                "namespace {} not present in block {}",
                namespace_id,
                block_height
            )
        })?;

        if NsProof::new(block.payload(), &ns_index, vid_common.common()).is_some() {
            return Err(anyhow::anyhow!(
                "block {} was correctly encoded",
                block_height
            ));
        }

        // Block has incorrect encoding — fetch VID shares to construct the proof.
        let vid_shares_future = ds
            .request_vid_shares(block_height, vid_common.clone(), Duration::from_secs(40))
            .await;
        let mut vid_shares = vid_shares_future
            .await
            .map_err(|e| anyhow::anyhow!("failed to fetch VID shares: {e:#}"))?;

        if let Ok(local_share) = ds.vid_share(block_height as usize).await {
            vid_shares.push(local_share);
        }

        let avidm_shares: Vec<AvidMShare> = vid_shares
            .into_iter()
            .filter_map(|s| {
                if let VidShare::V1(s) = s {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();

        match NsProof::v1_1_new_with_incorrect_encoding(
            &avidm_shares,
            ns_table,
            &ns_index,
            &vid_common.payload_hash(),
            vid_common.common(),
        ) {
            Some(NsProof::V1IncorrectEncoding(proof)) => Ok(proof),
            _ => Err(anyhow::anyhow!(
                "failed to generate incorrect encoding proof"
            )),
        }
    }
}

// ============================================================================
// v2::ConsensusApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v2::ConsensusApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: RewardMerkleTreeDataSource
        + super::data_source::StateCertDataSource
        + super::data_source::StateCertFetchingDataSource<espresso_types::SeqTypes>
        + super::data_source::StakeTableDataSource<espresso_types::SeqTypes>
        + Send
        + Sync,
{
    async fn get_state_certificate(&self, epoch: u64) -> anyhow::Result<Self::StateCertificate> {
        let ds = &*self.data_source;

        // Try to get from local storage first
        let state_cert = ds.get_state_cert_by_epoch(epoch).await?;

        let cert = match state_cert {
            Some(cert) => cert,
            None => {
                // Not found locally, try to fetch from peers
                const TIMEOUT: Duration = Duration::from_secs(40);
                let cert = ds.request_state_cert(epoch, TIMEOUT).await.map_err(|e| {
                    anyhow::anyhow!("failed to fetch state cert for epoch {}: {}", epoch, e)
                })?;

                // Store the fetched certificate
                ds.insert_state_cert(epoch, cert.clone()).await?;

                cert
            },
        };

        Ok(espresso_types::StateCertQueryDataV2(cert))
    }

    async fn get_stake_table(&self, epoch: u64) -> anyhow::Result<Self::StakeTable> {
        let ds = &*self.data_source;
        ds.get_stake_table(Some(EpochNumber::new(epoch))).await
    }
}

// ============================================================================
// v1::AvailabilityApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::AvailabilityApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: RewardMerkleTreeDataSource
        + hotshot_query_service::availability::AvailabilityDataSource<espresso_types::SeqTypes>
        + hotshot_query_service::node::NodeDataSource<espresso_types::SeqTypes>
        + super::data_source::RequestResponseDataSource<espresso_types::SeqTypes>
        + super::data_source::StateCertDataSource
        + super::data_source::StateCertFetchingDataSource<espresso_types::SeqTypes>
        + Send
        + Sync,
{
    type NamespaceProofQueryData = espresso_types::NamespaceProofQueryData;
    type IncorrectEncodingProof = espresso_types::v0_3::AvidMIncorrectEncodingNsProof;
    type StateCertQueryDataV1 = espresso_types::StateCertQueryDataV1<espresso_types::SeqTypes>;
    type StateCertQueryDataV2 = espresso_types::StateCertQueryDataV2<espresso_types::SeqTypes>;

    async fn get_namespace_proof(
        &self,
        block_id: espresso_api::v1::availability::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Option<Self::NamespaceProofQueryData>> {
        let ns_id = NamespaceId::from(namespace);

        // Convert v1 BlockId to hotshot BlockId
        let hs_block_id = match block_id {
            espresso_api::v1::availability::BlockId::Height(h) => HsBlockId::Number(h as usize),
            espresso_api::v1::availability::BlockId::Hash(h) => {
                let hash = h
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid block hash: {}", h))?;
                HsBlockId::Hash(hash)
            },
            espresso_api::v1::availability::BlockId::PayloadHash(h) => {
                let payload_hash = h
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid payload hash: {}", h))?;
                HsBlockId::PayloadHash(payload_hash)
            },
        };

        // Fetch block and VID common data
        let ds = &*self.data_source;
        let timeout = std::time::Duration::from_millis(500);
        let (block_fetch, vid_fetch) =
            join!(ds.get_block(hs_block_id), ds.get_vid_common(hs_block_id));
        let (block, vid_common) = join!(
            block_fetch.with_timeout(timeout),
            vid_fetch.with_timeout(timeout)
        );

        let Some(block) = block else {
            return Ok(None);
        };
        let Some(vid_common) = vid_common else {
            return Ok(None);
        };

        // Check if namespace is present
        let ns_table = block.payload().ns_table();
        let Some(ns_index) = ns_table.find_ns_id(&ns_id) else {
            return Ok(None);
        };

        // Generate namespace proof
        let Some(proof) = NsProof::new(block.payload(), &ns_index, vid_common.common()) else {
            // Failed to generate proof - namespace exists but proof generation failed
            return Ok(Some(espresso_types::NamespaceProofQueryData {
                transactions: vec![],
                proof: None,
            }));
        };

        let transactions = proof.export_all_txs(&ns_id);

        Ok(Some(espresso_types::NamespaceProofQueryData {
            transactions,
            proof: Some(proof),
        }))
    }

    async fn get_namespace_proof_range(
        &self,
        from: u64,
        until: u64,
        namespace: u32,
    ) -> anyhow::Result<Vec<Self::NamespaceProofQueryData>> {
        let ns_id = NamespaceId::from(namespace);

        // Validate range
        if until <= from {
            return Err(bad_request(format!(
                "invalid range: until ({}) must be greater than from ({})",
                until, from
            )));
        }

        let range_size = until - from;
        const MAX_RANGE: u64 = 100;
        if range_size > MAX_RANGE {
            return Err(range_exceeded(format!(
                "range too large: {} blocks (max {})",
                range_size, MAX_RANGE
            )));
        }

        // Fetch blocks and VID common data for the range
        let (blocks_stream, vids_stream) = join!(
            self.data_source
                .get_block_range(from as usize..until as usize),
            self.data_source
                .get_vid_common_range(from as usize..until as usize)
        );

        let blocks: Vec<_> = blocks_stream
            .then(|block| async move { block.resolve().await })
            .collect()
            .await;
        let vids: Vec<_> = vids_stream
            .then(|vid| async move { vid.resolve().await })
            .collect()
            .await;

        if blocks.len() != vids.len() {
            return Err(anyhow::anyhow!(
                "mismatch between blocks and VID common data"
            ));
        }

        // Generate proofs for each block
        let mut proofs = Vec::new();

        for (block, vid) in blocks.into_iter().zip(vids) {
            let ns_table = block.payload().ns_table();

            // Check if namespace exists in this block
            if let Some(ns_index) = ns_table.find_ns_id(&ns_id) {
                if let Some(proof) = NsProof::new(block.payload(), &ns_index, vid.common()) {
                    let transactions = proof.export_all_txs(&ns_id);
                    proofs.push(espresso_types::NamespaceProofQueryData {
                        transactions,
                        proof: Some(proof),
                    });
                } else {
                    // Failed to generate proof - return empty result for this block
                    proofs.push(espresso_types::NamespaceProofQueryData {
                        transactions: vec![],
                        proof: None,
                    });
                }
            } else {
                // Namespace not present in this block
                proofs.push(espresso_types::NamespaceProofQueryData {
                    transactions: vec![],
                    proof: None,
                });
            }
        }

        Ok(proofs)
    }

    async fn stream_namespace_proofs(
        &self,
        from: usize,
        namespace: u32,
    ) -> anyhow::Result<BoxStream<'static, Self::NamespaceProofQueryData>> {
        let ns_id = NamespaceId::from(namespace);
        let ds = self.data_source.clone();
        let blocks = (*ds).subscribe_blocks(from).await;
        let vids = (*ds).subscribe_vid_common(from).await;

        let stream = blocks
            .zip(vids)
            .map(move |(block, vid)| {
                let ns_table = block.payload().ns_table();
                if let Some(ns_index) = ns_table.find_ns_id(&ns_id) {
                    if let Some(proof) = NsProof::new(block.payload(), &ns_index, vid.common()) {
                        let transactions = proof.export_all_txs(&ns_id);
                        NamespaceProofQueryData {
                            transactions,
                            proof: Some(proof),
                        }
                    } else {
                        NamespaceProofQueryData {
                            transactions: vec![],
                            proof: None,
                        }
                    }
                } else {
                    NamespaceProofQueryData {
                        transactions: vec![],
                        proof: None,
                    }
                }
            })
            .boxed();

        Ok(stream)
    }

    async fn get_incorrect_encoding_proof(
        &self,
        block_id: espresso_api::v1::availability::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::IncorrectEncodingProof> {
        let ns_id = NamespaceId::from(namespace);

        let hs_block_id = match block_id {
            espresso_api::v1::availability::BlockId::Height(h) => HsBlockId::Number(h as usize),
            espresso_api::v1::availability::BlockId::Hash(h) => {
                let hash = h
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid block hash: {}", h))?;
                HsBlockId::Hash(hash)
            },
            espresso_api::v1::availability::BlockId::PayloadHash(h) => {
                let payload_hash = h
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid payload hash: {}", h))?;
                HsBlockId::PayloadHash(payload_hash)
            },
        };

        let ds = &*self.data_source;
        let timeout = std::time::Duration::from_millis(500);
        let (block_fetch, vid_fetch) =
            join!(ds.get_block(hs_block_id), ds.get_vid_common(hs_block_id));
        let (block, vid_common) = join!(
            block_fetch.with_timeout(timeout),
            vid_fetch.with_timeout(timeout)
        );

        let block = block.ok_or_else(|| anyhow::anyhow!("block not found"))?;
        let vid_common = vid_common.ok_or_else(|| anyhow::anyhow!("VID common data not found"))?;

        let ns_table = block.payload().ns_table();
        let ns_index = ns_table
            .find_ns_id(&ns_id)
            .ok_or_else(|| anyhow::anyhow!("namespace {} not present in block", namespace))?;

        if NsProof::new(block.payload(), &ns_index, vid_common.common()).is_some() {
            return Err(anyhow::anyhow!("block was correctly encoded"));
        }

        // Block has incorrect encoding — fetch VID shares to construct the proof.
        let vid_shares_future = ds
            .request_vid_shares(block.height(), vid_common.clone(), Duration::from_secs(40))
            .await;
        let mut vid_shares = vid_shares_future
            .await
            .map_err(|e| anyhow::anyhow!("failed to fetch VID shares: {e:#}"))?;

        if let Ok(local_share) = ds.vid_share(block.height() as usize).await {
            vid_shares.push(local_share);
        }

        let avidm_shares: Vec<AvidMShare> = vid_shares
            .into_iter()
            .filter_map(|s| {
                if let VidShare::V1(s) = s {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();

        match NsProof::v1_1_new_with_incorrect_encoding(
            &avidm_shares,
            ns_table,
            &ns_index,
            &vid_common.payload_hash(),
            vid_common.common(),
        ) {
            Some(NsProof::V1IncorrectEncoding(proof)) => Ok(proof),
            _ => Err(anyhow::anyhow!(
                "failed to generate incorrect encoding proof"
            )),
        }
    }

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1> {
        // Try to get from local storage first
        let state_cert = self.data_source.get_state_cert_by_epoch(epoch).await?;

        let cert = match state_cert {
            Some(cert) => cert,
            None => {
                // Not found locally, try to fetch from peers
                const TIMEOUT: Duration = Duration::from_secs(40);
                let cert = self
                    .data_source
                    .request_state_cert(epoch, TIMEOUT)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("failed to fetch state cert for epoch {}: {}", epoch, e)
                    })?;

                // Store the fetched certificate
                self.data_source
                    .insert_state_cert(epoch, cert.clone())
                    .await?;

                cert
            },
        };

        Ok(espresso_types::StateCertQueryDataV1::from(
            espresso_types::StateCertQueryDataV2(cert),
        ))
    }

    async fn get_state_cert_v2(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV2> {
        // Try to get from local storage first
        let state_cert = self.data_source.get_state_cert_by_epoch(epoch).await?;

        let cert = match state_cert {
            Some(cert) => cert,
            None => {
                // Not found locally, try to fetch from peers
                const TIMEOUT: Duration = Duration::from_secs(40);
                let cert = self
                    .data_source
                    .request_state_cert(epoch, TIMEOUT)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("failed to fetch state cert for epoch {}: {}", epoch, e)
                    })?;

                // Store the fetched certificate
                self.data_source
                    .insert_state_cert(epoch, cert.clone())
                    .await?;

                cert
            },
        };

        Ok(espresso_types::StateCertQueryDataV2(cert))
    }
}

// ============================================================================
// v1::HotShotAvailabilityApi implementation
// ============================================================================

fn not_found(msg: impl Into<String>) -> anyhow::Error {
    AvailabilityError::NotFound(msg.into()).into()
}

fn bad_request(msg: impl Into<String>) -> anyhow::Error {
    AvailabilityError::BadRequest(msg.into()).into()
}

fn range_exceeded(msg: impl Into<String>) -> anyhow::Error {
    AvailabilityError::RangeExceeded(msg.into()).into()
}

fn enforce_range(from: usize, until: usize, limit: usize) -> anyhow::Result<()> {
    if until.saturating_sub(from) > limit {
        return Err(range_exceeded(format!(
            "range {from}..{until} exceeds limit {limit}"
        )));
    }
    Ok(())
}

#[async_trait]
impl<D> HotShotAvailabilityApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: AvailabilityDataSource<espresso_types::SeqTypes> + Send + Sync,
{
    type Leaf = LeafQueryData<espresso_types::SeqTypes>;
    type Block = BlockQueryData<espresso_types::SeqTypes>;
    type Header = HsHeader<espresso_types::SeqTypes>;
    type Payload = PayloadQueryData<espresso_types::SeqTypes>;
    type VidCommon = VidCommonQueryData<espresso_types::SeqTypes>;
    type Transaction = TransactionQueryData<espresso_types::SeqTypes>;
    type TransactionWithProof = TransactionWithProofQueryData<espresso_types::SeqTypes>;
    type BlockSummary = BlockSummaryQueryData<espresso_types::SeqTypes>;
    type Limits = HsLimits;
    type Cert2 = Certificate2<espresso_types::SeqTypes>;

    async fn get_leaf(
        &self,
        id: espresso_api::v1::availability::LeafId,
    ) -> anyhow::Result<Self::Leaf> {
        let hs_id = match id {
            espresso_api::v1::availability::LeafId::Height(h) => HsLeafId::Number(h as usize),
            espresso_api::v1::availability::LeafId::Hash(h) => {
                HsLeafId::Hash(h.parse().map_err(|_| bad_request("invalid leaf hash"))?)
            },
        };
        let ds = &*self.data_source;
        ds.get_leaf(hs_id)
            .await
            .with_timeout(Duration::from_millis(500))
            .await
            .ok_or_else(|| not_found("leaf not found"))
    }

    async fn get_leaf_range(&self, from: usize, until: usize) -> anyhow::Result<Vec<Self::Leaf>> {
        enforce_range(from, until, 500)?;
        let timeout = Duration::from_millis(500);
        let ds = &*self.data_source;
        let stream = ds.get_leaf_range(from..until).await;
        let mut results = Vec::new();
        futures::pin_mut!(stream);
        let mut i = from;
        while let Some(fetch) = stream.next().await {
            let item = fetch
                .with_timeout(timeout)
                .await
                .ok_or_else(|| not_found(format!("leaf {} not found", i)))?;
            results.push(item);
            i += 1;
        }
        Ok(results)
    }

    async fn get_header(
        &self,
        id: espresso_api::v1::availability::BlockId,
    ) -> anyhow::Result<Self::Header> {
        let hs_id = block_id_to_hs(id)?;
        let ds = &*self.data_source;
        ds.get_header(hs_id)
            .await
            .with_timeout(Duration::from_millis(500))
            .await
            .ok_or_else(|| not_found(format!("header not found for {}", hs_id)))
    }

    async fn get_header_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::Header>> {
        enforce_range(from, until, 100)?;
        let timeout = Duration::from_millis(500);
        let ds = &*self.data_source;
        let stream = ds.get_header_range(from..until).await;
        let mut results = Vec::new();
        futures::pin_mut!(stream);
        let mut i = from;
        while let Some(fetch) = stream.next().await {
            let item = fetch
                .with_timeout(timeout)
                .await
                .ok_or_else(|| not_found(format!("header {} not found", i)))?;
            results.push(item);
            i += 1;
        }
        Ok(results)
    }

    async fn get_block(
        &self,
        id: espresso_api::v1::availability::BlockId,
    ) -> anyhow::Result<Self::Block> {
        let hs_id = block_id_to_hs(id)?;
        let ds = &*self.data_source;
        ds.get_block(hs_id)
            .await
            .with_timeout(Duration::from_millis(500))
            .await
            .ok_or_else(|| not_found(format!("block not found for {}", hs_id)))
    }

    async fn get_block_range(&self, from: usize, until: usize) -> anyhow::Result<Vec<Self::Block>> {
        enforce_range(from, until, 100)?;
        let timeout = Duration::from_millis(500);
        let ds = &*self.data_source;
        let stream = ds.get_block_range(from..until).await;
        let mut results = Vec::new();
        futures::pin_mut!(stream);
        let mut i = from;
        while let Some(fetch) = stream.next().await {
            let item = fetch
                .with_timeout(timeout)
                .await
                .ok_or_else(|| not_found(format!("block {} not found", i)))?;
            results.push(item);
            i += 1;
        }
        Ok(results)
    }

    async fn get_payload(
        &self,
        id: espresso_api::v1::availability::PayloadId,
    ) -> anyhow::Result<Self::Payload> {
        let hs_id = payload_id_to_hs(id)?;
        let ds = &*self.data_source;
        ds.get_payload(hs_id)
            .await
            .with_timeout(Duration::from_millis(500))
            .await
            .ok_or_else(|| not_found(format!("payload not found for {}", hs_id)))
    }

    async fn get_payload_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::Payload>> {
        enforce_range(from, until, 100)?;
        let timeout = Duration::from_millis(500);
        let ds = &*self.data_source;
        let stream = ds.get_payload_range(from..until).await;
        let mut results = Vec::new();
        futures::pin_mut!(stream);
        let mut i = from;
        while let Some(fetch) = stream.next().await {
            let item = fetch
                .with_timeout(timeout)
                .await
                .ok_or_else(|| not_found(format!("payload {} not found", i)))?;
            results.push(item);
            i += 1;
        }
        Ok(results)
    }

    async fn get_vid_common(
        &self,
        id: espresso_api::v1::availability::BlockId,
    ) -> anyhow::Result<Self::VidCommon> {
        let hs_id = block_id_to_hs(id)?;
        let ds = &*self.data_source;
        ds.get_vid_common(hs_id)
            .await
            .with_timeout(Duration::from_millis(500))
            .await
            .ok_or_else(|| not_found(format!("VID common not found for {}", hs_id)))
    }

    async fn get_vid_common_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::VidCommon>> {
        enforce_range(from, until, 500)?;
        let timeout = Duration::from_millis(500);
        let ds = &*self.data_source;
        let stream = ds.get_vid_common_range(from..until).await;
        let mut results = Vec::new();
        futures::pin_mut!(stream);
        let mut i = from;
        while let Some(fetch) = stream.next().await {
            let item = fetch
                .with_timeout(timeout)
                .await
                .ok_or_else(|| not_found(format!("VID common {} not found", i)))?;
            results.push(item);
            i += 1;
        }
        Ok(results)
    }

    async fn get_transaction_by_position(
        &self,
        height: u64,
        index: u64,
    ) -> anyhow::Result<Self::Transaction> {
        let ds = &*self.data_source;
        let block = ds
            .get_block(HsBlockId::Number(height as usize))
            .await
            .with_timeout(Duration::from_millis(500))
            .await
            .ok_or_else(|| not_found(format!("block {} not found", height)))?;

        let idx = block
            .payload()
            .nth(block.metadata(), index as usize)
            .ok_or_else(|| {
                not_found(format!(
                    "transaction index {} out of bounds in block {}",
                    index, height
                ))
            })?;
        let tx = block
            .transaction(&idx)
            .ok_or_else(|| not_found(format!("transaction not found at index {}", index)))?;
        TransactionQueryData::new(tx, &block, &idx, index)
            .ok_or_else(|| anyhow::anyhow!("failed to build transaction query data"))
    }

    async fn get_transaction_by_hash(&self, hash: String) -> anyhow::Result<Self::Transaction> {
        let ds = &*self.data_source;
        let tx_hash: hotshot_query_service::availability::TransactionHash<
            espresso_types::SeqTypes,
        > = hash
            .parse()
            .map_err(|_| bad_request(format!("invalid transaction hash: {}", hash)))?;
        let bwt = ds
            .get_block_containing_transaction(tx_hash)
            .await
            .with_timeout(Duration::from_millis(500))
            .await
            .ok_or_else(|| not_found("transaction not found"))?;
        Ok(bwt.transaction)
    }

    async fn get_transaction_proof_by_position(
        &self,
        height: u64,
        index: u64,
    ) -> anyhow::Result<Self::TransactionWithProof> {
        let ds = &*self.data_source;
        let timeout = Duration::from_millis(500);

        let (block_fetch, vid_fetch) = futures::join!(
            ds.get_block(HsBlockId::Number(height as usize)),
            ds.get_vid_common(HsBlockId::Number(height as usize))
        );
        let (block, vid) = futures::join!(
            block_fetch.with_timeout(timeout),
            vid_fetch.with_timeout(timeout)
        );

        let block = block.ok_or_else(|| not_found(format!("block {} not found", height)))?;
        let vid =
            vid.ok_or_else(|| not_found(format!("VID common not found for block {}", height)))?;

        let idx = block
            .payload()
            .nth(block.metadata(), index as usize)
            .ok_or_else(|| {
                not_found(format!(
                    "transaction index {} out of bounds in block {}",
                    index, height
                ))
            })?;
        let tx = block
            .transaction(&idx)
            .ok_or_else(|| not_found(format!("transaction not found at index {}", index)))?;
        let tx_data = TransactionQueryData::new(tx, &block, &idx, index)
            .ok_or_else(|| anyhow::anyhow!("failed to build transaction query data"))?;
        let proof = block
            .transaction_proof(&vid, &idx)
            .ok_or_else(|| anyhow::anyhow!("failed to build transaction proof"))?;
        Ok(TransactionWithProofQueryData::new(tx_data, proof))
    }

    async fn get_transaction_proof_by_hash(
        &self,
        hash: String,
    ) -> anyhow::Result<Self::TransactionWithProof> {
        let ds = &*self.data_source;
        let timeout = Duration::from_millis(500);

        let tx_hash: hotshot_query_service::availability::TransactionHash<
            espresso_types::SeqTypes,
        > = hash
            .parse()
            .map_err(|_| bad_request(format!("invalid transaction hash: {}", hash)))?;
        let bwt = ds
            .get_block_containing_transaction(tx_hash)
            .await
            .with_timeout(timeout)
            .await
            .ok_or_else(|| not_found("transaction not found"))?;

        let vid = ds
            .get_vid_common(HsBlockId::Number(bwt.block.height() as usize))
            .await
            .with_timeout(timeout)
            .await
            .ok_or_else(|| {
                not_found(format!(
                    "VID common not found for block {}",
                    bwt.block.height()
                ))
            })?;

        let proof = bwt
            .block
            .transaction_proof(&vid, &bwt.index)
            .ok_or_else(|| anyhow::anyhow!("failed to build transaction proof"))?;
        Ok(TransactionWithProofQueryData::new(bwt.transaction, proof))
    }

    async fn get_block_summary(&self, height: usize) -> anyhow::Result<Self::BlockSummary> {
        let ds = &*self.data_source;
        let block = ds
            .get_block(HsBlockId::Number(height))
            .await
            .with_timeout(Duration::from_millis(500))
            .await
            .ok_or_else(|| not_found(format!("block {} not found", height)))?;
        Ok(BlockSummaryQueryData::from(block))
    }

    async fn get_block_summary_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::BlockSummary>> {
        enforce_range(from, until, 100)?;
        let timeout = Duration::from_millis(500);
        let ds = &*self.data_source;
        let stream = ds.get_block_range(from..until).await;
        let mut results = Vec::new();
        futures::pin_mut!(stream);
        let mut i = from;
        while let Some(fetch) = stream.next().await {
            let block = fetch
                .with_timeout(timeout)
                .await
                .ok_or_else(|| not_found(format!("block {} not found", i)))?;
            results.push(BlockSummaryQueryData::from(block));
            i += 1;
        }
        Ok(results)
    }

    async fn get_limits(&self) -> anyhow::Result<Self::Limits> {
        Ok(HsLimits {
            small_object_range_limit: 500,
            large_object_range_limit: 100,
        })
    }

    async fn get_cert2(&self, height: u64) -> anyhow::Result<Option<Self::Cert2>> {
        self.data_source
            .get_cert2(height)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn stream_leaves(&self, from: usize) -> anyhow::Result<BoxStream<'static, Self::Leaf>> {
        let ds = self.data_source.clone();
        Ok((*ds).subscribe_leaves(from).await.boxed())
    }

    async fn stream_headers(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, Self::Header>> {
        let ds = self.data_source.clone();
        Ok((*ds).subscribe_headers(from).await.boxed())
    }

    async fn stream_blocks(&self, from: usize) -> anyhow::Result<BoxStream<'static, Self::Block>> {
        let ds = self.data_source.clone();
        Ok((*ds).subscribe_blocks(from).await.boxed())
    }

    async fn stream_payloads(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, Self::Payload>> {
        let ds = self.data_source.clone();
        Ok((*ds).subscribe_payloads(from).await.boxed())
    }

    async fn stream_vid_common(
        &self,
        from: usize,
    ) -> anyhow::Result<BoxStream<'static, Self::VidCommon>> {
        let ds = self.data_source.clone();
        Ok((*ds).subscribe_vid_common(from).await.boxed())
    }

    async fn stream_transactions(
        &self,
        from: usize,
        namespace: Option<u32>,
    ) -> anyhow::Result<BoxStream<'static, Self::Transaction>> {
        let ds = self.data_source.clone();
        let stream = (*ds)
            .subscribe_blocks(from)
            .await
            .flat_map(move |block| {
                let ns_filter = namespace.map(NamespaceId::from);
                let txs: Vec<Self::Transaction> = block
                    .enumerate()
                    .enumerate()
                    .filter_map(|(position_in_block, (tx_index, _tx))| {
                        let tx = block.transaction(&tx_index)?;
                        if let Some(ns) = ns_filter
                            && tx.namespace() != ns
                        {
                            return None;
                        }
                        TransactionQueryData::new(tx, &block, &tx_index, position_in_block as u64)
                    })
                    .collect();
                futures::stream::iter(txs)
            })
            .boxed();
        Ok(stream)
    }
}

fn block_id_to_hs(
    id: espresso_api::v1::availability::BlockId,
) -> anyhow::Result<HsBlockId<SeqTypes>> {
    match id {
        espresso_api::v1::availability::BlockId::Height(h) => Ok(HsBlockId::Number(h as usize)),
        espresso_api::v1::availability::BlockId::Hash(h) => {
            let hash = h
                .parse()
                .map_err(|_| bad_request(format!("invalid block hash: {}", h)))?;
            Ok(HsBlockId::Hash(hash))
        },
        espresso_api::v1::availability::BlockId::PayloadHash(h) => {
            let payload_hash = h
                .parse()
                .map_err(|_| bad_request(format!("invalid payload hash: {}", h)))?;
            Ok(HsBlockId::PayloadHash(payload_hash))
        },
    }
}

fn payload_id_to_hs(
    id: espresso_api::v1::availability::PayloadId,
) -> anyhow::Result<HsBlockId<SeqTypes>> {
    match id {
        espresso_api::v1::availability::PayloadId::Height(h) => Ok(HsBlockId::Number(h as usize)),
        espresso_api::v1::availability::PayloadId::Hash(h) => {
            let payload_hash = h
                .parse()
                .map_err(|_| bad_request(format!("invalid payload hash: {}", h)))?;
            Ok(HsBlockId::PayloadHash(payload_hash))
        },
        espresso_api::v1::availability::PayloadId::BlockHash(h) => {
            let hash = h
                .parse()
                .map_err(|_| bad_request(format!("invalid block hash: {}", h)))?;
            Ok(HsBlockId::Hash(hash))
        },
    }
}

fn classify_query_error(err: hotshot_query_service::QueryError) -> anyhow::Error {
    use hotshot_query_service::QueryError;
    match err {
        QueryError::NotFound | QueryError::Missing => not_found(err.to_string()),
        QueryError::Error { .. } => anyhow::anyhow!(err.to_string()),
    }
}

#[async_trait]
impl<D> espresso_api::v1::BlockStateApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::merklized_state::MerklizedStateDataSource<
            espresso_types::SeqTypes,
            espresso_types::BlockMerkleTree,
            { <espresso_types::BlockMerkleTree as jf_merkle_tree_compat::MerkleTreeScheme>::ARITY },
        > + hotshot_query_service::merklized_state::MerklizedStateHeightPersistence
        + Send
        + Sync,
{
    type MerkleProof = InternalMerkleProof<
        committable::Commitment<espresso_types::Header>,
        u64,
        jf_merkle_tree_compat::prelude::Sha3Node,
        3,
    >;

    async fn get_block_state_path(
        &self,
        snapshot: espresso_api::v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Self::MerkleProof> {
        use hotshot_query_service::merklized_state::{
            MerklizedStateDataSource, Snapshot as HsSnapshot,
        };

        let hs_snapshot = match snapshot {
            espresso_api::v1::Snapshot::Height(h) => HsSnapshot::Index(h),
            espresso_api::v1::Snapshot::Commit(c) => {
                let tb64: TaggedBase64 = c
                    .parse()
                    .map_err(|_| bad_request("failed to parse commit param"))?;
                let commit = (&tb64)
                    .try_into()
                    .map_err(|_| bad_request("failed to parse commit param"))?;
                HsSnapshot::Commit(commit)
            },
        };
        let key: u64 = key
            .parse()
            .map_err(|_| bad_request("failed to parse Key param"))?;
        let ds = &*self.data_source;
        MerklizedStateDataSource::<
            espresso_types::SeqTypes,
            espresso_types::BlockMerkleTree,
            _,
        >::get_path(ds, hs_snapshot, key)
        .await
        .map_err(classify_query_error)
    }

    async fn get_block_state_height(&self) -> anyhow::Result<u64> {
        use hotshot_query_service::merklized_state::MerklizedStateHeightPersistence;

        let ds = &*self.data_source;
        ds.get_last_state_height()
            .await
            .map(|h| h as u64)
            .map_err(classify_query_error)
    }
}

#[async_trait]
impl<D> espresso_api::v1::FeeStateApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::merklized_state::MerklizedStateDataSource<
            espresso_types::SeqTypes,
            espresso_types::FeeMerkleTree,
            { <espresso_types::FeeMerkleTree as jf_merkle_tree_compat::MerkleTreeScheme>::ARITY },
        > + hotshot_query_service::merklized_state::MerklizedStateHeightPersistence
        + Send
        + Sync,
{
    type MerkleProof = InternalMerkleProof<
        espresso_types::FeeAmount,
        espresso_types::FeeAccount,
        jf_merkle_tree_compat::prelude::Sha3Node,
        256,
    >;
    type FeeAmount = espresso_types::FeeAmount;

    async fn get_fee_state_path(
        &self,
        snapshot: espresso_api::v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Self::MerkleProof> {
        use hotshot_query_service::merklized_state::{
            MerklizedStateDataSource, Snapshot as HsSnapshot,
        };

        let hs_snapshot = match snapshot {
            espresso_api::v1::Snapshot::Height(h) => HsSnapshot::Index(h),
            espresso_api::v1::Snapshot::Commit(c) => {
                let tb64: TaggedBase64 = c
                    .parse()
                    .map_err(|_| bad_request("failed to parse commit param"))?;
                let commit = (&tb64)
                    .try_into()
                    .map_err(|_| bad_request("failed to parse commit param"))?;
                HsSnapshot::Commit(commit)
            },
        };
        let key: espresso_types::FeeAccount = key
            .parse()
            .map_err(|_| bad_request("failed to parse Key param"))?;
        let ds = &*self.data_source;
        MerklizedStateDataSource::<
            espresso_types::SeqTypes,
            espresso_types::FeeMerkleTree,
            _,
        >::get_path(ds, hs_snapshot, key)
        .await
        .map_err(classify_query_error)
    }

    async fn get_fee_state_height(&self) -> anyhow::Result<u64> {
        use hotshot_query_service::merklized_state::MerklizedStateHeightPersistence;

        let ds = &*self.data_source;
        ds.get_last_state_height()
            .await
            .map(|h| h as u64)
            .map_err(classify_query_error)
    }

    async fn get_fee_balance_latest(
        &self,
        address: String,
    ) -> anyhow::Result<Option<Self::FeeAmount>> {
        use hotshot_query_service::merklized_state::{
            MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot as HsSnapshot,
        };
        use jf_merkle_tree_compat::prelude::MerkleProof as JfMerkleProof;

        let key: espresso_types::FeeAccount = address
            .parse()
            .map_err(|_| bad_request("failed to parse address"))?;
        let ds = &*self.data_source;
        let height = ds
            .get_last_state_height()
            .await
            .map_err(classify_query_error)?;
        let path: JfMerkleProof<
            espresso_types::FeeAmount,
            espresso_types::FeeAccount,
            jf_merkle_tree_compat::prelude::Sha3Node,
            256,
        > = MerklizedStateDataSource::<
            espresso_types::SeqTypes,
            espresso_types::FeeMerkleTree,
            _,
        >::get_path(ds, HsSnapshot::Index(height as u64), key)
        .await
        .map_err(classify_query_error)?;
        Ok(path.elem().copied())
    }
}

// ============================================================================
// v1::StatusApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::StatusApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::status::StatusDataSource + Send + Sync,
{
    async fn block_height(&self) -> anyhow::Result<u64> {
        let ds = &*self.data_source;
        let h = hotshot_query_service::status::StatusDataSource::block_height(ds)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(h as u64)
    }

    async fn success_rate(&self) -> anyhow::Result<f64> {
        let ds = &*self.data_source;
        hotshot_query_service::status::StatusDataSource::success_rate(ds)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn time_since_last_decide(&self) -> anyhow::Result<u64> {
        let ds = &*self.data_source;
        hotshot_query_service::status::StatusDataSource::elapsed_time_since_last_decide(ds)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn metrics(&self) -> anyhow::Result<String> {
        use hotshot_query_service::status::HasMetrics;
        use tide_disco::metrics::Metrics as _;
        let ds = &*self.data_source;
        ds.metrics().export().map_err(|e| anyhow::anyhow!("{e}"))
    }
}

// ============================================================================
// v1::ConfigApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::ConfigApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: super::data_source::HotShotConfigDataSource + Send + Sync,
{
    type HotShotConfig = espresso_types::config::PublicNetworkConfig;
    type RuntimeConfig = crate::options::PublicNodeConfig;

    async fn hotshot_config(&self) -> anyhow::Result<Self::HotShotConfig> {
        use super::data_source::HotShotConfigDataSource as _;
        let ds = &*self.data_source;
        Ok(ds.get_config().await)
    }

    async fn env(&self) -> anyhow::Result<Vec<String>> {
        Ok((*self.env_vars).clone())
    }

    async fn runtime_config(&self) -> anyhow::Result<Self::RuntimeConfig> {
        self.public_node_config.as_deref().cloned().ok_or_else(|| {
            espresso_api::error::AvailabilityError::NotFound(
                "runtime config not available".to_string(),
            )
            .into()
        })
    }
}

// ============================================================================
// v1::NodeApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::NodeApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::node::NodeDataSource<espresso_types::SeqTypes>
        + super::data_source::StakeTableDataSource<espresso_types::SeqTypes>
        + super::data_source::PruningDataSource
        + Send
        + Sync,
{
    type VidShare = hotshot_types::data::VidShare;
    type SyncStatus = hotshot_query_service::node::SyncStatusQueryData;
    type HeaderWindow = hotshot_query_service::node::TimeWindowQueryData<
        hotshot_query_service::Header<espresso_types::SeqTypes>,
    >;
    type Limits = hotshot_query_service::node::Limits;
    type StakeTable = Vec<hotshot_types::PeerConfig<espresso_types::SeqTypes>>;
    type StakeTableCurrent =
        super::data_source::StakeTableWithEpochNumber<espresso_types::SeqTypes>;
    type Validators = indexmap::IndexMap<
        alloy::primitives::Address,
        espresso_types::v0_3::AuthenticatedValidator<espresso_types::PubKey>,
    >;
    type AllValidators = Vec<espresso_types::v0_3::RegisteredValidator<espresso_types::PubKey>>;
    type Participation = std::collections::HashMap<espresso_types::PubKey, f64>;
    type BlockReward = Option<espresso_types::v0_3::RewardAmount>;
    type Block = hotshot_query_service::availability::BlockQueryData<espresso_types::SeqTypes>;
    type Leaf = hotshot_query_service::availability::LeafQueryData<espresso_types::SeqTypes>;

    async fn block_height(&self) -> anyhow::Result<u64> {
        let ds = &*self.data_source;
        let h = hotshot_query_service::node::NodeDataSource::block_height(ds)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(h as u64)
    }

    async fn count_transactions(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u32>,
    ) -> anyhow::Result<u64> {
        use std::ops::Bound;
        let ds = &*self.data_source;
        let from = match from {
            Some(f) => Bound::Included(f as usize),
            None => Bound::Unbounded,
        };
        let to = match to {
            Some(t) => Bound::Included(t as usize),
            None => Bound::Unbounded,
        };
        let ns = namespace.map(|n| espresso_types::NamespaceId::from(n as u64));
        let count = ds
            .count_transactions_in_range((from, to), ns)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(count as u64)
    }

    async fn payload_size(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u32>,
    ) -> anyhow::Result<u64> {
        use std::ops::Bound;
        let ds = &*self.data_source;
        let from = match from {
            Some(f) => Bound::Included(f as usize),
            None => Bound::Unbounded,
        };
        let to = match to {
            Some(t) => Bound::Included(t as usize),
            None => Bound::Unbounded,
        };
        let ns = namespace.map(|n| espresso_types::NamespaceId::from(n as u64));
        let size = ds
            .payload_size_in_range((from, to), ns)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(size as u64)
    }

    async fn get_vid_share(
        &self,
        id: espresso_api::v1::VidShareId,
    ) -> anyhow::Result<Self::VidShare> {
        let ds = &*self.data_source;
        let node_id: HsBlockId<espresso_types::SeqTypes> = match id {
            espresso_api::v1::VidShareId::Height(h) => HsBlockId::Number(h as usize),
            espresso_api::v1::VidShareId::Hash(h) => HsBlockId::Hash(
                h.parse()
                    .map_err(|_| bad_request(format!("invalid block hash: {h}")))?,
            ),
            espresso_api::v1::VidShareId::PayloadHash(h) => HsBlockId::PayloadHash(
                h.parse()
                    .map_err(|_| bad_request(format!("invalid payload hash: {h}")))?,
            ),
        };
        hotshot_query_service::node::NodeDataSource::vid_share(ds, node_id)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn sync_status(&self) -> anyhow::Result<Self::SyncStatus> {
        let ds = &*self.data_source;
        hotshot_query_service::node::NodeDataSource::sync_status(ds)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn get_header_window(
        &self,
        start: espresso_api::v1::HeaderWindowStart,
        end: u64,
    ) -> anyhow::Result<Self::HeaderWindow> {
        use hotshot_query_service::node::WindowStart;
        let ds = &*self.data_source;
        let start: WindowStart<espresso_types::SeqTypes> = match start {
            espresso_api::v1::HeaderWindowStart::Time(t) => WindowStart::Time(t),
            espresso_api::v1::HeaderWindowStart::Height(h) => WindowStart::Height(h),
            espresso_api::v1::HeaderWindowStart::Hash(h) => WindowStart::Hash(
                h.parse()
                    .map_err(|_| anyhow::anyhow!("invalid block hash: {h}"))?,
            ),
        };
        ds.get_header_window(start, end, node_window_limit())
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    async fn limits(&self) -> anyhow::Result<Self::Limits> {
        Ok(hotshot_query_service::node::Limits {
            window_limit: node_window_limit(),
        })
    }

    async fn stake_table(&self, epoch: u64) -> anyhow::Result<Self::StakeTable> {
        let ds = &*self.data_source;
        ds.get_stake_table(Some(hotshot_types::data::EpochNumber::new(epoch)))
            .await
    }

    async fn stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent> {
        let ds = &*self.data_source;
        ds.get_stake_table_current().await
    }

    async fn da_stake_table(&self, epoch: u64) -> anyhow::Result<Self::StakeTable> {
        let ds = &*self.data_source;
        ds.get_da_stake_table(Some(hotshot_types::data::EpochNumber::new(epoch)))
            .await
    }

    async fn da_stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent> {
        let ds = &*self.data_source;
        ds.get_da_stake_table_current().await
    }

    async fn get_validators(&self, epoch: u64) -> anyhow::Result<Self::Validators> {
        let ds = &*self.data_source;
        ds.get_validators(hotshot_types::data::EpochNumber::new(epoch))
            .await
    }

    async fn get_all_validators(
        &self,
        epoch: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::AllValidators> {
        if limit > 1000 {
            return Err(anyhow::anyhow!("Limit cannot be greater than 1000"));
        }
        let ds = &*self.data_source;
        ds.get_all_validators(hotshot_types::data::EpochNumber::new(epoch), offset, limit)
            .await
    }

    async fn current_proposal_participation(&self) -> anyhow::Result<Self::Participation> {
        let ds = &*self.data_source;
        Ok(ds.current_proposal_participation().await)
    }

    async fn proposal_participation(&self, epoch: u64) -> anyhow::Result<Self::Participation> {
        let ds = &*self.data_source;
        Ok(ds
            .proposal_participation(hotshot_types::data::EpochNumber::new(epoch))
            .await)
    }

    async fn current_vote_participation(&self) -> anyhow::Result<Self::Participation> {
        let ds = &*self.data_source;
        Ok(ds.current_vote_participation().await)
    }

    async fn vote_participation(&self, epoch: u64) -> anyhow::Result<Self::Participation> {
        let ds = &*self.data_source;
        Ok(ds
            .vote_participation(hotshot_types::data::EpochNumber::new(epoch))
            .await)
    }

    async fn get_block_reward(&self, epoch: Option<u64>) -> anyhow::Result<Self::BlockReward> {
        let ds = &*self.data_source;
        ds.get_block_reward(epoch.map(hotshot_types::data::EpochNumber::new))
            .await
    }

    async fn get_oldest_block(&self) -> anyhow::Result<Option<Self::Block>> {
        use super::data_source::PruningDataSource as _;
        let ds = &*self.data_source;
        ds.get_oldest_block().await
    }

    async fn get_oldest_leaf(&self) -> anyhow::Result<Option<Self::Leaf>> {
        use super::data_source::PruningDataSource as _;
        let ds = &*self.data_source;
        ds.get_oldest_leaf().await
    }
}

fn node_window_limit() -> usize {
    hotshot_query_service::node::Options::default().window_limit
}

// ============================================================================
// v1::CatchupApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::CatchupApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: super::data_source::CatchupDataSource
        + super::data_source::NodeStateDataSource
        + Send
        + Sync,
{
    type AccountQueryData = espresso_types::AccountQueryData;
    type FeeMerkleTree = espresso_types::FeeMerkleTree;
    type BlocksFrontier = super::BlocksFrontier;
    type ChainConfig = espresso_types::v0_3::ChainConfig;
    type LeafChain = Vec<espresso_types::Leaf2>;
    type Cert2 = espresso_types::Certificate2<espresso_types::SeqTypes>;
    type RewardAccountQueryDataV1 = espresso_types::v0_3::RewardAccountQueryDataV1;
    type RewardMerkleTreeV1 = espresso_types::v0_3::RewardMerkleTreeV1;
    type RewardAccountQueryDataV2 = espresso_types::v0_4::RewardAccountQueryDataV2;
    type RewardMerkleTreeV2Data = serde_json::Value;
    type StateCert = hotshot_types::simple_certificate::LightClientStateUpdateCertificateV2<
        espresso_types::SeqTypes,
    >;

    async fn get_account(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Self::AccountQueryData> {
        use super::data_source::{CatchupDataSource as _, NodeStateDataSource as _};
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let account: espresso_types::FeeAccount = address
            .parse()
            .map_err(|err| bad_request(format!("malformed fee account {address}: {err}")))?;
        let instance = ds.node_state().await;
        ds.get_account(&instance, height, view, account)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_accounts(
        &self,
        height: u64,
        view: u64,
        accounts: Vec<String>,
    ) -> anyhow::Result<Self::FeeMerkleTree> {
        use super::data_source::{CatchupDataSource as _, NodeStateDataSource as _};
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let parsed: Vec<espresso_types::FeeAccount> = accounts
            .iter()
            .map(|a| {
                a.parse()
                    .map_err(|err| bad_request(format!("malformed fee account {a}: {err}")))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let instance = ds.node_state().await;
        ds.get_accounts(&instance, height, view, &parsed)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_blocks_frontier(
        &self,
        height: u64,
        view: u64,
    ) -> anyhow::Result<Self::BlocksFrontier> {
        use super::data_source::{CatchupDataSource as _, NodeStateDataSource as _};
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let instance = ds.node_state().await;
        ds.get_frontier(&instance, height, view)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_chain_config(&self, commitment: String) -> anyhow::Result<Self::ChainConfig> {
        use super::data_source::CatchupDataSource as _;
        let ds = &*self.data_source;
        let parsed: committable::Commitment<espresso_types::v0_3::ChainConfig> = commitment
            .parse()
            .map_err(|err| bad_request(format!("malformed chain config commitment: {err}")))?;
        ds.get_chain_config(parsed)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Self::LeafChain> {
        use super::data_source::CatchupDataSource as _;
        let ds = &*self.data_source;
        ds.get_leaf_chain(height)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_cert2(&self, height: u64) -> anyhow::Result<Self::Cert2> {
        use super::data_source::CatchupDataSource as _;
        let ds = &*self.data_source;
        let response = ds
            .get_cert2(height)
            .await
            .map_err(|err| not_found(format!("{err:#}")))?;
        response.ok_or_else(|| not_found(format!("no cert2 available for height {height}")))
    }

    async fn get_reward_account_v1(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV1> {
        use super::data_source::{CatchupDataSource as _, NodeStateDataSource as _};
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let account: espresso_types::v0_4::RewardAccountV2 = address
            .parse()
            .map_err(|err| bad_request(format!("malformed reward account {address}: {err}")))?;
        let instance = ds.node_state().await;
        ds.get_reward_account_v1(&instance, height, view, account.into())
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_reward_accounts_v1(
        &self,
        height: u64,
        view: u64,
        accounts: Vec<String>,
    ) -> anyhow::Result<Self::RewardMerkleTreeV1> {
        use super::data_source::{CatchupDataSource as _, NodeStateDataSource as _};
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let parsed: Vec<espresso_types::v0_3::RewardAccountV1> = accounts
            .iter()
            .map(|a| {
                a.parse()
                    .map_err(|err| bad_request(format!("malformed reward account {a}: {err}")))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let instance = ds.node_state().await;
        ds.get_reward_accounts_v1(&instance, height, view, &parsed)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_reward_account_v2(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV2> {
        use super::data_source::{CatchupDataSource as _, NodeStateDataSource as _};
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let account: espresso_types::v0_4::RewardAccountV2 = address
            .parse()
            .map_err(|err| bad_request(format!("malformed reward account {address}: {err}")))?;
        let instance = ds.node_state().await;
        ds.get_reward_account_v2(&instance, height, view, account)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
        view: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeV2Data> {
        use super::data_source::CatchupDataSource as _;
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let bytes = ds
            .get_reward_merkle_tree_v2(height, view)
            .await
            .map_err(|err| not_found(format!("{err:#}")))?;
        // tide-disco returns the raw Vec<u8> from `get_reward_merkle_tree_v2`. To preserve
        // identical wire output, re-encode the bytes themselves as the JSON body.
        Ok(serde_json::to_value(bytes)?)
    }

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCert> {
        use super::data_source::CatchupDataSource as _;
        let ds = &*self.data_source;
        ds.get_state_cert(epoch)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }
}

// ============================================================================
// v1::SubmitApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::SubmitApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: SubmitDataSourceErased + Send + Sync,
{
    type Transaction = espresso_types::Transaction;
    type TxHash = committable::Commitment<espresso_types::Transaction>;

    async fn submit(&self, tx: Self::Transaction) -> anyhow::Result<Self::TxHash> {
        use committable::Committable as _;
        let hash = tx.commit();
        let ds = &*self.data_source;
        ds.submit_erased(tx)
            .await
            .map_err(|err| anyhow::anyhow!("{err:#}"))?;
        Ok(hash)
    }
}

/// Network-agnostic submit hook used by the axum wrapper. The original
/// `SubmitDataSource<N, P>` trait is parameterized by the network type; this
/// erased trait lets `NodeApiStateImpl` avoid carrying those parameters.
#[async_trait]
pub(crate) trait SubmitDataSourceErased {
    async fn submit_erased(&self, tx: espresso_types::Transaction) -> anyhow::Result<()>;
}

#[async_trait]
impl<N, P, D> SubmitDataSourceErased
    for hotshot_query_service::data_source::ExtensibleDataSource<D, crate::api::ApiState<N, P>>
where
    N: hotshot_types::traits::network::ConnectedNetwork<espresso_types::PubKey>,
    P: espresso_types::v0::traits::SequencerPersistence,
    D: Send + Sync,
{
    async fn submit_erased(&self, tx: espresso_types::Transaction) -> anyhow::Result<()> {
        <Self as super::data_source::SubmitDataSource<N, P>>::submit(self, tx).await
    }
}

// ============================================================================
// v1::StateSignatureApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::StateSignatureApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: StateSignatureDataSourceErased + Send + Sync,
{
    type Signature = hotshot_types::light_client::LCV3StateSignatureRequestBody;

    async fn get_state_signature(&self, height: u64) -> anyhow::Result<Self::Signature> {
        let ds = &*self.data_source;
        ds.get_state_signature_erased(height)
            .await
            .ok_or_else(|| not_found("Signature not found."))
    }
}

#[async_trait]
pub(crate) trait StateSignatureDataSourceErased {
    async fn get_state_signature_erased(
        &self,
        height: u64,
    ) -> Option<hotshot_types::light_client::LCV3StateSignatureRequestBody>;
}

#[async_trait]
impl<N, P, D> StateSignatureDataSourceErased
    for hotshot_query_service::data_source::ExtensibleDataSource<D, crate::api::ApiState<N, P>>
where
    N: hotshot_types::traits::network::ConnectedNetwork<espresso_types::PubKey>,
    P: espresso_types::v0::traits::SequencerPersistence,
    D: Send + Sync,
{
    async fn get_state_signature_erased(
        &self,
        height: u64,
    ) -> Option<hotshot_types::light_client::LCV3StateSignatureRequestBody> {
        <Self as StateSignatureDataSource<N>>::get_state_signature(self, height).await
    }
}

// ============================================================================
// v1::ExplorerApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::ExplorerApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target:
        hotshot_query_service::explorer::ExplorerDataSource<espresso_types::SeqTypes> + Send + Sync,
{
    type BlockDetail =
        hotshot_query_service::explorer::BlockDetailResponse<espresso_types::SeqTypes>;
    type BlockSummaries =
        hotshot_query_service::explorer::BlockSummaryResponse<espresso_types::SeqTypes>;
    type TransactionDetail =
        hotshot_query_service::explorer::TransactionDetailResponse<espresso_types::SeqTypes>;
    type TransactionSummaries =
        hotshot_query_service::explorer::TransactionSummariesResponse<espresso_types::SeqTypes>;
    type ExplorerSummary =
        hotshot_query_service::explorer::ExplorerSummaryResponse<espresso_types::SeqTypes>;
    type SearchResult =
        hotshot_query_service::explorer::SearchResultResponse<espresso_types::SeqTypes>;

    async fn get_block_detail(
        &self,
        ident: espresso_api::v1::BlockIdent,
    ) -> anyhow::Result<Self::BlockDetail> {
        use hotshot_query_service::explorer::{BlockIdentifier, ExplorerDataSource as _};
        let ds = &*self.data_source;
        let target = match ident {
            espresso_api::v1::BlockIdent::Height(h) => BlockIdentifier::Height(h as usize),
            espresso_api::v1::BlockIdent::Hash(h) => BlockIdentifier::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?,
            ),
            espresso_api::v1::BlockIdent::Latest => BlockIdentifier::Latest,
        };
        ds.get_block_detail(target)
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_block_summaries(
        &self,
        target: espresso_api::v1::BlockIdent,
        limit: u64,
    ) -> anyhow::Result<Self::BlockSummaries> {
        use hotshot_query_service::explorer::{
            BlockIdentifier, BlockRange, ExplorerDataSource as _, GetBlockSummariesRequest,
        };
        let ds = &*self.data_source;
        let num_blocks = std::num::NonZeroUsize::new(limit as usize)
            .ok_or_else(|| bad_request("limit must be greater than 0"))?;
        if num_blocks.get() > 100 {
            return Err(bad_request("limit must be <= 100"));
        }
        let target = match target {
            espresso_api::v1::BlockIdent::Height(h) => BlockIdentifier::Height(h as usize),
            espresso_api::v1::BlockIdent::Hash(h) => BlockIdentifier::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?,
            ),
            espresso_api::v1::BlockIdent::Latest => BlockIdentifier::Latest,
        };
        ds.get_block_summaries(GetBlockSummariesRequest(BlockRange { target, num_blocks }))
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_transaction_detail(
        &self,
        ident: espresso_api::v1::TxIdent,
    ) -> anyhow::Result<Self::TransactionDetail> {
        use hotshot_query_service::explorer::{ExplorerDataSource as _, TransactionIdentifier};
        let ds = &*self.data_source;
        let target = match ident {
            espresso_api::v1::TxIdent::HeightAndOffset(h, o) => {
                TransactionIdentifier::HeightAndOffset(h as usize, o as usize)
            },
            espresso_api::v1::TxIdent::Hash(h) => TransactionIdentifier::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid tx hash {h}: {err}")))?,
            ),
            espresso_api::v1::TxIdent::Latest => TransactionIdentifier::Latest,
        };
        ds.get_transaction_detail(target)
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_transaction_summaries(
        &self,
        target: espresso_api::v1::TxIdent,
        limit: u64,
        filter: espresso_api::v1::TxSummaryFilter,
    ) -> anyhow::Result<Self::TransactionSummaries> {
        use hotshot_query_service::explorer::{
            ExplorerDataSource as _, GetTransactionSummariesRequest, TransactionIdentifier,
            TransactionRange, TransactionSummaryFilter,
        };
        let ds = &*self.data_source;
        let num_transactions = std::num::NonZeroUsize::new(limit as usize)
            .ok_or_else(|| bad_request("limit must be greater than 0"))?;
        if num_transactions.get() > 100 {
            return Err(bad_request("limit must be <= 100"));
        }
        let target = match target {
            espresso_api::v1::TxIdent::HeightAndOffset(h, o) => {
                TransactionIdentifier::HeightAndOffset(h as usize, o as usize)
            },
            espresso_api::v1::TxIdent::Hash(h) => TransactionIdentifier::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid tx hash {h}: {err}")))?,
            ),
            espresso_api::v1::TxIdent::Latest => TransactionIdentifier::Latest,
        };
        let filter = match filter {
            espresso_api::v1::TxSummaryFilter::None => TransactionSummaryFilter::None,
            espresso_api::v1::TxSummaryFilter::Block(b) => {
                TransactionSummaryFilter::Block(b as usize)
            },
            espresso_api::v1::TxSummaryFilter::Namespace(n) => {
                TransactionSummaryFilter::RollUp(n.into())
            },
        };
        ds.get_transaction_summaries(GetTransactionSummariesRequest {
            range: TransactionRange {
                target,
                num_transactions,
            },
            filter,
        })
        .await
        .map(Into::into)
        .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_explorer_summary(&self) -> anyhow::Result<Self::ExplorerSummary> {
        use hotshot_query_service::explorer::ExplorerDataSource as _;
        let ds = &*self.data_source;
        ds.get_explorer_summary()
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_search_result(&self, query: String) -> anyhow::Result<Self::SearchResult> {
        use hotshot_query_service::explorer::ExplorerDataSource as _;
        let ds = &*self.data_source;
        let parsed: tagged_base64::TaggedBase64 = query
            .parse()
            .map_err(|err| bad_request(format!("invalid search query {query}: {err}")))?;
        ds.get_search_results(parsed)
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }
}

// ============================================================================
// v1::LightClientApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::LightClientApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: AvailabilityDataSource<espresso_types::SeqTypes>
        + hotshot_query_service::merklized_state::MerklizedStateDataSource<
            espresso_types::SeqTypes,
            espresso_types::BlockMerkleTree,
            3,
        > + super::data_source::NodeStateDataSource
        + super::data_source::StakeTableDataSource<espresso_types::SeqTypes>
        + hotshot_query_service::data_source::VersionedDataSource
        + Sized
        + Send
        + Sync,
    for<'a> <D::Target as hotshot_query_service::data_source::VersionedDataSource>::ReadOnly<'a>:
        hotshot_query_service::data_source::storage::NodeStorage<espresso_types::SeqTypes>,
{
    type LeafProof = light_client::consensus::leaf::LeafProof;
    type HeaderProof = light_client::consensus::header::HeaderProof;
    type StakeTableEvents = Vec<espresso_types::v0_3::StakeTableEvent>;
    type PayloadProof = light_client::consensus::payload::PayloadProof;
    type NamespaceProof = light_client::consensus::namespace::NamespaceProof;

    async fn get_leaf_proof(
        &self,
        query: espresso_api::v1::LeafQuery,
        finalized: Option<u64>,
    ) -> anyhow::Result<Self::LeafProof> {
        use hotshot_query_service::availability::LeafId;
        let ds = &*self.data_source;
        let fetch_timeout = lc_fetch_timeout();

        let requested = match query {
            espresso_api::v1::LeafQuery::Height(h) => LeafId::Number(h as usize),
            espresso_api::v1::LeafQuery::Hash(h) => LeafId::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid leaf hash {h}: {err}")))?,
            ),
            espresso_api::v1::LeafQuery::BlockHash(h) => {
                let parsed = h
                    .parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?;
                let header = AvailabilityDataSource::get_header(ds, HsBlockId::Hash(parsed))
                    .await
                    .with_timeout(fetch_timeout)
                    .await
                    .ok_or_else(|| not_found(format!("unknown block hash {h}")))?;
                LeafId::Number(header.height() as usize)
            },
            espresso_api::v1::LeafQuery::PayloadHash(h) => {
                let parsed = h
                    .parse()
                    .map_err(|err| bad_request(format!("invalid payload hash {h}: {err}")))?;
                let header = AvailabilityDataSource::get_header(ds, HsBlockId::PayloadHash(parsed))
                    .await
                    .with_timeout(fetch_timeout)
                    .await
                    .ok_or_else(|| not_found(format!("unknown payload hash {h}")))?;
                LeafId::Number(header.height() as usize)
            },
        };

        let requested_leaf = AvailabilityDataSource::get_leaf(ds, requested)
            .await
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found(format!("unknown leaf {requested}")))?;

        let proof_result = if let Some(finalized) = finalized {
            crate::api::light_client::get_leaf_proof_with_finalized_assumption(
                ds,
                requested_leaf,
                finalized as usize,
                fetch_timeout,
            )
            .await
        } else if requested_leaf.header().version() >= versions::NEW_PROTOCOL_VERSION {
            crate::api::light_client::get_leaf_proof_with_cert2(ds, requested_leaf, fetch_timeout)
                .await
        } else {
            crate::api::light_client::get_leaf_proof_with_qc_chain(
                ds,
                requested_leaf,
                fetch_timeout,
            )
            .await
        };
        proof_result.map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_header_proof(
        &self,
        root: u64,
        requested: espresso_api::v1::HeaderQuery,
    ) -> anyhow::Result<Self::HeaderProof> {
        let ds = &*self.data_source;
        let fetch_timeout = lc_fetch_timeout();
        let requested = match requested {
            espresso_api::v1::HeaderQuery::Height(h) => HsBlockId::Number(h as usize),
            espresso_api::v1::HeaderQuery::Hash(h) => HsBlockId::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?,
            ),
            espresso_api::v1::HeaderQuery::PayloadHash(h) => HsBlockId::PayloadHash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid payload hash {h}: {err}")))?,
            ),
        };
        crate::api::light_client::get_header_proof(ds, root, requested, fetch_timeout)
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_light_client_stake_table(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Self::StakeTableEvents> {
        use hotshot_types::utils::{epoch_from_block_number, root_block_in_epoch};
        let ds = &*self.data_source;
        let fetch_timeout = lc_fetch_timeout();

        let node_state = super::data_source::NodeStateDataSource::node_state(ds).await;
        let epoch_height = node_state
            .epoch_height
            .ok_or_else(|| anyhow::anyhow!("epoch state not set"))?;
        let first_epoch = epoch_from_block_number(node_state.epoch_start_block, epoch_height);
        if epoch < first_epoch + 2 {
            return Err(bad_request(format!(
                "epoch must be at least {}",
                first_epoch + 2
            )));
        }

        let epoch_root_height = root_block_in_epoch(epoch - 2, epoch_height) as usize;
        let epoch_root = AvailabilityDataSource::get_header::<HsBlockId<espresso_types::SeqTypes>>(
            ds,
            HsBlockId::Number(epoch_root_height),
        )
        .await
        .with_timeout(fetch_timeout)
        .await
        .ok_or_else(|| not_found(format!("missing epoch root header {epoch_root_height}")))?;
        let to_l1_block = epoch_root
            .l1_finalized()
            .ok_or_else(|| anyhow::anyhow!("epoch root header is missing L1 finalized block"))?
            .number();

        let from_l1_block = if epoch >= first_epoch + 3 {
            let prev_epoch_root_height = root_block_in_epoch(epoch - 3, epoch_height) as usize;
            let prev_epoch_root = AvailabilityDataSource::get_header::<
                HsBlockId<espresso_types::SeqTypes>,
            >(ds, HsBlockId::Number(prev_epoch_root_height))
            .await
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| {
                not_found(format!(
                    "missing previous epoch root header {prev_epoch_root_height}"
                ))
            })?;
            prev_epoch_root
                .l1_finalized()
                .ok_or_else(|| {
                    anyhow::anyhow!("previous epoch root header is missing L1 finalized block")
                })?
                .number()
                + 1
        } else {
            0
        };

        super::data_source::StakeTableDataSource::stake_table_events(ds, from_l1_block, to_l1_block)
            .await
    }

    async fn get_payload_proof(&self, height: u64) -> anyhow::Result<Self::PayloadProof> {
        let ds = &*self.data_source;
        let fetch_timeout = lc_fetch_timeout();
        let height = height as usize;
        let payload = AvailabilityDataSource::get_payload(ds, height)
            .await
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found(format!("missing payload {height}")))?;
        let vid_common = AvailabilityDataSource::get_vid_common(ds, height)
            .await
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found(format!("missing VID common {height}")))?;
        Ok(light_client::consensus::payload::PayloadProof::new(
            payload.data().clone(),
            vid_common.common().clone(),
        ))
    }

    async fn get_payload_proof_range(
        &self,
        start: u64,
        end: u64,
    ) -> anyhow::Result<Vec<Self::PayloadProof>> {
        use futures::StreamExt as _;
        let ds = &*self.data_source;
        let fetch_timeout = lc_fetch_timeout();
        let start = start as usize;
        let end = end as usize;

        let payloads_stream = AvailabilityDataSource::get_payload_range(ds, start..end).await;
        let vid_stream = AvailabilityDataSource::get_vid_common_range(ds, start..end).await;
        let mut out = Vec::new();
        let mut payloads = payloads_stream.enumerate();
        let mut vid_commons = vid_stream.enumerate();
        loop {
            let (next_payload, next_vid) =
                futures::future::join(payloads.next(), vid_commons.next()).await;
            let (Some((i, payload_fut)), Some((_, vid_fut))) = (next_payload, next_vid) else {
                break;
            };
            let payload = payload_fut
                .with_timeout(fetch_timeout)
                .await
                .ok_or_else(|| not_found(format!("missing payload {}", start + i)))?;
            let vid_common = vid_fut
                .with_timeout(fetch_timeout)
                .await
                .ok_or_else(|| not_found(format!("missing VID common {}", start + i)))?;
            out.push(light_client::consensus::payload::PayloadProof::new(
                payload.data().clone(),
                vid_common.common().clone(),
            ));
        }
        Ok(out)
    }

    async fn get_lc_namespace_proof(
        &self,
        height: u64,
        namespace: u64,
    ) -> anyhow::Result<Self::NamespaceProof> {
        let ds = &*self.data_source;
        let fetch_timeout = lc_fetch_timeout();
        let mut proofs = crate::api::light_client::get_namespace_proof_range(
            ds,
            height as usize,
            (height + 1) as usize,
            namespace,
            fetch_timeout,
            lc_large_object_range_limit(),
        )
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))?;
        if proofs.len() != 1 {
            return Err(anyhow::anyhow!("internal consistency error"));
        }
        Ok(proofs.remove(0))
    }

    async fn get_lc_namespace_proof_range(
        &self,
        start: u64,
        end: u64,
        namespace: u64,
    ) -> anyhow::Result<Vec<Self::NamespaceProof>> {
        let ds = &*self.data_source;
        let fetch_timeout = lc_fetch_timeout();
        crate::api::light_client::get_namespace_proof_range(
            ds,
            start as usize,
            end as usize,
            namespace,
            fetch_timeout,
            lc_large_object_range_limit(),
        )
        .await
        .map_err(|err| anyhow::anyhow!("{err}"))
    }
}

fn lc_fetch_timeout() -> std::time::Duration {
    std::time::Duration::from_millis(500)
}

fn lc_large_object_range_limit() -> usize {
    hotshot_query_service::availability::Options::default().large_object_range_limit
}

// ============================================================================
// v1::HotShotEventsApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::HotShotEventsApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target:
        hotshot_events_service::events_source::EventsSource<espresso_types::SeqTypes> + Send + Sync,
{
    type Event = std::sync::Arc<hotshot_types::event::Event<espresso_types::SeqTypes>>;
    type StartupInfo = hotshot_events_service::events_source::StartupInfo<espresso_types::SeqTypes>;

    async fn startup_info(&self) -> anyhow::Result<Self::StartupInfo> {
        use hotshot_events_service::events_source::EventsSource as _;
        let ds = &*self.data_source;
        Ok(ds.get_startup_info().await)
    }

    async fn events(&self) -> anyhow::Result<futures::stream::BoxStream<'static, Self::Event>> {
        use hotshot_events_service::events_source::EventsSource as _;
        let ds = &*self.data_source;
        let stream = ds.get_event_stream(None).await;
        Ok(Box::pin(stream))
    }
}

// ============================================================================
// v1::TokenApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::TokenApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: super::data_source::TokenDataSource<espresso_types::SeqTypes>
        + super::data_source::NodeStateDataSource
        + Send
        + Sync,
{
    async fn total_minted_supply(&self) -> anyhow::Result<String> {
        use super::data_source::TokenDataSource as _;
        let ds = &*self.data_source;
        let value = ds
            .get_total_supply_l1()
            .await
            .map_err(|err| not_found(format!("failed to get total supply. err={err:#}")))?;
        Ok(alloy::primitives::utils::format_ether(value))
    }

    async fn circulating_supply(&self) -> anyhow::Result<String> {
        let calc = fetch_supply_inputs(&*self.data_source).await?;
        Ok(alloy::primitives::utils::format_ether(
            calc.circulating_supply(),
        ))
    }

    async fn circulating_supply_ethereum(&self) -> anyhow::Result<String> {
        let calc = fetch_supply_inputs(&*self.data_source).await?;
        Ok(alloy::primitives::utils::format_ether(
            calc.circulating_supply_ethereum(),
        ))
    }

    async fn total_issued_supply(&self) -> anyhow::Result<String> {
        let calc = fetch_supply_inputs(&*self.data_source).await?;
        Ok(alloy::primitives::utils::format_ether(
            calc.total_issued_supply(),
        ))
    }

    async fn total_reward_distributed(&self) -> anyhow::Result<String> {
        let calc = fetch_supply_inputs(&*self.data_source).await?;
        Ok(alloy::primitives::utils::format_ether(
            calc.total_reward_distributed(),
        ))
    }
}

async fn fetch_supply_inputs<S>(
    ds: &S,
) -> anyhow::Result<crate::api::unlock_schedule::SupplyCalculator>
where
    S: super::data_source::TokenDataSource<espresso_types::SeqTypes>
        + super::data_source::NodeStateDataSource
        + Sync
        + ?Sized,
{
    let node_state = ds.node_state().await;
    let chain_id = node_state.chain_config.chain_id;

    let header = ds.get_decided_header().await;
    let now_secs = header.timestamp_internal();
    let total_reward_distributed = header.total_reward_distributed();

    let initial_supply = ds
        .get_initial_supply_l1()
        .await
        .map_err(|err| anyhow::anyhow!("failed to get initial supply: {err:#}"))?;

    let total_supply_l1 = ds
        .get_total_supply_l1()
        .await
        .map_err(|err| anyhow::anyhow!("failed to get total supply: {err:#}"))?;

    Ok(crate::api::unlock_schedule::SupplyCalculator::new(
        chain_id,
        now_secs,
        initial_supply,
        total_supply_l1,
        total_reward_distributed,
    ))
}

// ============================================================================
// v1::DatabaseApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::DatabaseApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: super::data_source::DatabaseMetadataSource + Send + Sync,
{
    type TableSizes = Vec<super::data_source::TableSize>;

    async fn get_table_sizes(&self) -> anyhow::Result<Self::TableSizes> {
        use super::data_source::DatabaseMetadataSource as _;
        let ds = &*self.data_source;
        ds.get_table_sizes().await
    }
}
