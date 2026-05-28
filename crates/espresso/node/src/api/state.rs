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
        StateCertFetchingDataSource,
    },
};

/// Node API state implementation
///
/// This struct implements both v1::RewardApi (internal types) and v2::RewardApi (proto types).
#[derive(Clone)]
pub struct NodeApiStateImpl<D> {
    data_source: D,
}

impl<D> NodeApiStateImpl<D> {
    pub fn new(data_source: D) -> Self {
        Self { data_source }
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

// ============================================================================
// v1::BlockStateApi and v1::FeeStateApi implementations
// ============================================================================

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
                    .map_err(|_| anyhow::anyhow!("failed to parse commit param"))?;
                let commit = (&tb64)
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("failed to parse commit param"))?;
                HsSnapshot::Commit(commit)
            },
        };
        let key: u64 = key
            .parse()
            .map_err(|_| anyhow::anyhow!("failed to parse Key param"))?;
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
                    .map_err(|_| anyhow::anyhow!("failed to parse commit param"))?;
                let commit = (&tb64)
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("failed to parse commit param"))?;
                HsSnapshot::Commit(commit)
            },
        };
        let key: espresso_types::FeeAccount = key
            .parse()
            .map_err(|_| anyhow::anyhow!("failed to parse Key param"))?;
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
