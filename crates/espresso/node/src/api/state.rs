//! RewardApi trait implementations for espresso-node
//!
//! This module provides implementations for both v1::RewardApi (internal types)
//! and v2::RewardApi (proto types), backed by the same data source.

use alloy::primitives::U256;
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    BlockMerkleTree, Header, NsProof, SeqTypes,
    v0::sparse_mt::KeccakNode,
    v0_3::RewardAmount as InternalRewardAmount,
    v0_4::{
        RewardAccountProofV2 as InternalRewardAccountProofV2,
        RewardAccountQueryDataV2 as InternalRewardAccountQueryData, RewardAccountV2,
        RewardMerkleProofV2 as InternalRewardMerkleProofV2,
    },
    v0_6::RewardClaimError,
};
use hotshot_contract_adapter::reward::RewardClaimInput as InternalRewardClaimInput;
use hotshot_query_service::{
    availability::AvailabilityDataSource,
    merklized_state::{
        MerklizedState, MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot,
    },
};
use jf_merkle_tree_compat::{
    MerkleTreeScheme,
    prelude::{MerkleNode as InternalMerkleNode, MerkleProof as InternalMerkleProof, Sha3Node},
};
use serde_json;
use serialization_api::v2::{
    self, BlockMerklePathResponse, GetBlockMerklePathRequest,
    RewardAccountProofV2, RewardAccountQueryDataV2, RewardBalance, RewardBalances, RewardClaimInput,
    RewardMerkleProofV2, RewardMerkleTreeV2Data, merkle_node, reward_merkle_proof_v2::ProofType,
};
use tagged_base64::TaggedBase64;

use super::{
    RewardMerkleTreeDataSource, RewardMerkleTreeV2Data as InternalRewardTreeData,
    data_source::{StakeTableDataSource, StateCertDataSource, StateCertFetchingDataSource},
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
    type BlockMerklePath =
        InternalMerkleProof<Commitment<Header>, u64, Sha3Node, { BlockMerkleTree::ARITY }>;

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
    ) -> anyhow::Result<serialization_api::v2::NamespaceProofResponse> {
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
                    namespace: tx.namespace.0 as u32,
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
    ) -> anyhow::Result<serialization_api::v2::IncorrectEncodingProofResponse> {
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
    ) -> anyhow::Result<serialization_api::v2::StateCertificateResponse> {
        let certificate = self.serialize_light_client_cert(&value.0)?;

        Ok(serialization_api::v2::StateCertificateResponse {
            certificate: Some(certificate),
        })
    }

    fn serialize_stake_table(
        &self,
        value: &Self::StakeTable,
    ) -> anyhow::Result<serialization_api::v2::StakeTableResponse> {
        let peers: Result<Vec<_>, _> = value
            .iter()
            .map(|peer| self.serialize_peer_config(peer))
            .collect();

        Ok(serialization_api::v2::StakeTableResponse { peers: peers? })
    }

    fn serialize_peer_config(
        &self,
        peer: &Self::PeerConfig,
    ) -> anyhow::Result<serialization_api::v2::PeerConfig> {
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
                        ip: ip.to_string(),
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
    ) -> anyhow::Result<serialization_api::v2::LightClientStateUpdateCertificateV2> {
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

    fn serialize_ns_proof(
        &self,
        proof: &Self::NsProof,
    ) -> anyhow::Result<serialization_api::v2::NsProof> {
        use espresso_types::NsProof as InternalNsProof;

        let proof_version = match proof {
            InternalNsProof::V0(advz_proof) => {
                // Serialize the inner fields directly
                let json = serde_json::json!({
                    "ns_index": advz_proof.ns_index,
                    "ns_payload": advz_proof.ns_payload,
                    "ns_proof": advz_proof.ns_proof,
                });
                v2::ns_proof::ProofVersion::V0(serde_json::from_value(json)?)
            },
            InternalNsProof::V1(avidm_proof) => {
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
            InternalNsProof::V1IncorrectEncoding(incorrect_proof) => {
                // Serialize the whole proof to JSON string
                v2::ns_proof::ProofVersion::V1IncorrectEncoding(v2::AvidMIncorrectEncodingNsProof {
                    proof_data: serde_json::to_string(&incorrect_proof.0)?,
                })
            },
            InternalNsProof::V2(gf2_proof) => {
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

    fn serialize_block_merkle_path(
        &self,
        value: &Self::BlockMerklePath,
    ) -> anyhow::Result<BlockMerklePathResponse>
    where
        Self::BlockMerklePath: Sized,
    {
        let proof = serde_json::to_string(value)
            .map_err(|e| anyhow::anyhow!("failed to serialize block merkle path: {}", e))?;
        Ok(BlockMerklePathResponse { proof })
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
    type RewardBalance = U256;
    type RewardAccountQueryData = InternalRewardAccountQueryData;
    type RewardAmounts = Vec<(alloy::primitives::Address, U256)>;
    type RewardMerkleTreeData = InternalRewardTreeData;

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

        // Return the balance directly (U256)
        Ok(proof.balance)
    }

    async fn get_latest_reward_balance(
        &self,
        address: String,
    ) -> anyhow::Result<Self::RewardBalance> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid ethereum address: {}", address))?;

        // Load the latest reward account proof from the data source
        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                anyhow::anyhow!("failed to load latest reward account {}: {}", address, err)
            })?;

        // Return the balance directly (U256)
        Ok(proof.balance)
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

        // Reverse order (matching Tide implementation) and convert to (Address, U256)
        let result: Vec<(alloy::primitives::Address, U256)> = slice
            .iter()
            .rev()
            .map(|(account, amount)| (account.0, amount.0))
            .collect();

        Ok(result)
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> anyhow::Result<Self::RewardMerkleTreeData> {
        // Load the raw merkle tree bytes
        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            anyhow::anyhow!("failed to load reward tree at height {}: {}", height, err)
        })?;

        // Deserialize to internal RewardMerkleTreeV2Data
        let tree_data: InternalRewardTreeData =
            bincode::deserialize(&tree_bytes).map_err(|err| {
                anyhow::anyhow!(
                    "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                    height,
                    err
                )
            })?;

        Ok(tree_data)
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
        + MerklizedStateDataSource<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>
        + MerklizedStateHeightPersistence
        + Sync
        + Send,
{
    async fn get_namespace_proof(
        &self,
        namespace_id: u32,
        block_height: u64,
    ) -> anyhow::Result<Self::NamespaceProof> {
        use espresso_types::NamespaceId;
        use futures::join;
        use hotshot_query_service::availability::BlockId;

        let ns_id = NamespaceId::from(namespace_id);
        let block_id = BlockId::Number(block_height as usize);

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
        namespace_id: u32,
        from: u64,
        until: u64,
    ) -> anyhow::Result<Vec<Self::NamespaceProof>> {
        use espresso_types::NamespaceId;

        let ns_id = NamespaceId::from(namespace_id);

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
        use futures::{join, stream::StreamExt};

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
        use espresso_types::NsProof;
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
        namespace_id: u32,
        block_height: u64,
    ) -> anyhow::Result<Self::IncorrectEncodingProof> {
        use espresso_types::{NamespaceId, NsProof};
        use futures::join;
        use hotshot_query_service::availability::BlockId;

        let ns_id = NamespaceId::from(namespace_id);
        let block_id = BlockId::Number(block_height as usize);

        // Fetch block and VID common data
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

        // For incorrect encoding proof, we need special handling
        // Note: Full incorrect encoding proof support with VID share fetching
        // would require more complex implementation. For now, we return an error
        // if the basic proof generation fails.

        let ns_table = block.payload().ns_table();
        let ns_index = ns_table.find_ns_id(&ns_id).ok_or_else(|| {
            anyhow::anyhow!(
                "namespace {} not present in block {}",
                namespace_id,
                block_height
            )
        })?;

        // Try to generate a normal proof first
        if let Some(_proof) = NsProof::new(block.payload(), &ns_index, vid_common.common()) {
            return Err(anyhow::anyhow!(
                "block {} was correctly encoded",
                block_height
            ));
        }

        // If normal proof generation failed, it indicates incorrect encoding
        // but we can't generate the full incorrect encoding proof without VID share fetching
        Err(anyhow::anyhow!(
            "Incorrect encoding detected for namespace {} in block {}, but full proof generation \
             requires VID share fetching",
            namespace_id,
            block_height
        ))
    }

    async fn get_block_merkle_path(
        &self,
        request: GetBlockMerklePathRequest,
    ) -> anyhow::Result<Self::BlockMerklePath>
    where
        Self::BlockMerklePath: Sized,
    {
        let snapshot = match (request.snapshot_height, request.snapshot_commit) {
            (Some(height), _) => Snapshot::Index(height),
            (None, Some(commit_str)) => {
                let tb64: TaggedBase64 = commit_str
                    .parse()
                    .map_err(|e| anyhow::anyhow!("invalid TaggedBase64 snapshot_commit: {}", e))?;
                let commit_val = <BlockMerkleTree as MerklizedState<
                    SeqTypes,
                    { BlockMerkleTree::ARITY },
                >>::Commit::try_from(&tb64)
                .map_err(|e| anyhow::anyhow!("failed to parse block merkle commit: {}", e))?;
                Snapshot::Commit(commit_val)
            },
            (None, None) => {
                return Err(anyhow::anyhow!(
                    "Must specify either `snapshot_height` or `snapshot_commit`"
                ));
            },
        };
        (*self.data_source)
            .get_path(snapshot, request.block_height)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn get_block_merkle_height(&self) -> anyhow::Result<u64> {
        (*self.data_source)
            .get_last_state_height()
            .await
            .map(|h| h as u64)
            .map_err(|e| anyhow::anyhow!("{}", e))
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
        use std::time::Duration;

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
        use hotshot_types::data::EpochNumber;

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
        use espresso_types::NamespaceId;
        use futures::join;
        use hotshot_query_service::availability::BlockId as HsBlockId;

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
        use espresso_types::NsProof;
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
        use espresso_types::NamespaceId;

        let ns_id = NamespaceId::from(namespace);

        // Validate range
        if until <= from {
            return Err(anyhow::anyhow!(
                "invalid range: until ({}) must be greater than from ({})",
                until,
                from
            ));
        }

        let range_size = until - from;
        const MAX_RANGE: u64 = 100;
        if range_size > MAX_RANGE {
            return Err(anyhow::anyhow!(
                "range too large: {} blocks (max {})",
                range_size,
                MAX_RANGE
            ));
        }

        // Fetch blocks and VID common data for the range
        use futures::{join, stream::StreamExt};

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
        use espresso_types::NsProof;
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
        block_id: espresso_api::v1::availability::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::IncorrectEncodingProof> {
        use espresso_types::{NamespaceId, NsProof};
        use futures::join;
        use hotshot_query_service::availability::BlockId as HsBlockId;

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

        let block = block.ok_or_else(|| anyhow::anyhow!("block not found"))?;
        let vid_common = vid_common.ok_or_else(|| anyhow::anyhow!("VID common data not found"))?;

        // For incorrect encoding proof, we need special handling
        // Note: Full incorrect encoding proof support with VID share fetching
        // would require more complex implementation. For now, we return an error
        // if the basic proof generation fails.

        let ns_table = block.payload().ns_table();
        let ns_index = ns_table
            .find_ns_id(&ns_id)
            .ok_or_else(|| anyhow::anyhow!("namespace {} not present in block", namespace))?;

        // Try to generate a normal proof first
        if let Some(_proof) = NsProof::new(block.payload(), &ns_index, vid_common.common()) {
            return Err(anyhow::anyhow!("block was correctly encoded"));
        }

        // If normal proof generation failed, it indicates incorrect encoding
        // but we can't generate the full incorrect encoding proof without VID share fetching
        Err(anyhow::anyhow!(
            "Incorrect encoding detected for namespace {} but full proof generation requires VID \
             share fetching",
            namespace
        ))
    }

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCertQueryDataV1> {
        use std::time::Duration;

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
        use std::time::Duration;

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
// v1::BlockStateApi implementation
// ============================================================================

#[async_trait]
impl<D> espresso_api::v1::BlockStateApi for NodeApiStateImpl<D>
where
    D: std::ops::Deref + Clone + Send + Sync + 'static,
    D::Target: MerklizedStateDataSource<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>
        + MerklizedStateHeightPersistence
        + Send
        + Sync,
{
    type BlockMerklePath =
        InternalMerkleProof<Commitment<Header>, u64, Sha3Node, { BlockMerkleTree::ARITY }>;

    async fn get_block_merkle_path(
        &self,
        height: u64,
        key: u64,
    ) -> anyhow::Result<Self::BlockMerklePath> {
        let snapshot = Snapshot::Index(height);
        (*self.data_source)
            .get_path(snapshot, key)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn get_block_merkle_path_by_commit(
        &self,
        commit: String,
        key: u64,
    ) -> anyhow::Result<Self::BlockMerklePath> {
        let tb64: TaggedBase64 = commit
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid TaggedBase64 commit: {}", e))?;
        let commit_val =
            <BlockMerkleTree as MerklizedState<SeqTypes, { BlockMerkleTree::ARITY }>>::Commit::try_from(&tb64)
                .map_err(|e| anyhow::anyhow!("failed to parse block merkle commit: {}", e))?;
        let snapshot = Snapshot::Commit(commit_val);
        (*self.data_source)
            .get_path(snapshot, key)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn get_block_merkle_height(&self) -> anyhow::Result<usize> {
        (*self.data_source)
            .get_last_state_height()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

// ============================================================================
// v2::DataApi block-state additions
// ============================================================================
