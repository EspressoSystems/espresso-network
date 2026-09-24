//! Consensus and query service types rendered as v2 proto messages.

use std::{borrow::Borrow, collections::HashMap};

use ark_serialize::CanonicalSerialize;
use espresso_types::{
    BuilderSignature, FeeInfo, Header, L1BlockInfo, NamespaceProofQueryData, NsProof, Payload,
    PubKey, SeqTypes, Transaction, TxProof,
    config::PublicNetworkConfig,
    v0_3::{
        AvidMIncorrectEncodingNsProof, AvidMNsProof, RegisteredValidator, ResolvableChainConfig,
        StateCertQueryDataV1,
    },
    v0_4::StateCertQueryDataV2,
    v0_6::AvidmGf2NsProof,
};
use hotshot_query_service_types::{
    availability::{
        BlockQueryData, BlockSummaryQueryData, LeafQueryData, PayloadQueryData,
        TransactionQueryData, TransactionWithProofQueryData, VidCommonQueryData,
    },
    node::{
        Limits as NodeLimits, ResourceSyncStatus, SyncStatus, SyncStatusQueryData,
        TimeWindowQueryData,
    },
};
use hotshot_types::{
    HotShotConfig, PeerConfig,
    data::{Leaf2, VidCommon, VidShare, ViewChangeEvidence2},
    network::BuilderType,
    simple_certificate::{
        Certificate2, SimpleCertificate, SuccessThreshold, Threshold, TimeoutCertificate2,
        TimeoutCertificate3, UpgradeCertificate, ViewSyncFinalizeCertificate2,
    },
    simple_vote::{QuorumData2, Voteable},
    traits::EncodeBytes as _,
    vid::advz::{LargeRangeProofType, SmallRangeProofType},
};
use jf_merkle_tree_compat::{
    Element, Index, NodeValue,
    prelude::{MerkleNode, MerkleProof},
};
use tagged_base64::TaggedBase64;

use crate::proto::{
    self, advz_merkle_node::Node, header_response::Header as Shape,
    resolvable_chain_config::ChainConfig,
};

impl From<ResolvableChainConfig> for proto::ResolvableChainConfig {
    fn from(chain_config: ResolvableChainConfig) -> Self {
        // `commit` would hash a full config too; only `resolve` tells the two apart.
        let resolved = match chain_config.resolve() {
            Some(config) => ChainConfig::Full(proto::ChainConfig {
                chain_id: config.chain_id.to_string(),
                max_block_size: *config.max_block_size,
                base_fee: config.base_fee.to_string(),
                // v1 renders addresses as lowercase hex through `ethers_core::H160` and
                // `FixedBytes`; alloy's `Display` prints them EIP-55 checksummed instead.
                fee_contract: config.fee_contract.map(|address| format!("{address:#x}")),
                fee_recipient: format!("{:#x}", config.fee_recipient.0),
                stake_table_contract: config
                    .stake_table_contract
                    .map(|address| format!("{address:#x}")),
            }),
            None => ChainConfig::Commitment(chain_config.commit().to_string()),
        };
        Self {
            chain_config: Some(resolved),
        }
    }
}

impl From<L1BlockInfo> for proto::L1BlockInfo {
    fn from(info: L1BlockInfo) -> Self {
        Self {
            number: info.number,
            timestamp: format!("{:#x}", info.timestamp),
            hash: format!("{:#x}", info.hash),
        }
    }
}

impl From<&FeeInfo> for proto::FeeInfo {
    fn from(fee: &FeeInfo) -> Self {
        Self {
            account: format!("{:#x}", fee.account.0),
            amount: fee.amount.to_string(),
        }
    }
}

impl From<&BuilderSignature> for proto::BuilderSignature {
    fn from(signature: &BuilderSignature) -> Self {
        Self {
            r: format!("{:#x}", signature.r()),
            s: format!("{:#x}", signature.s()),
            // alloy's parity bool; v1 accepts only 27 or 28.
            v: if signature.v() { 28 } else { 27 },
        }
    }
}

/// The proto message per protocol version, mirroring the `Header` enum. Versions sharing a shape
/// share a message, so only the arm distinguishes 0.1 from 0.2 and 0.5 from 0.6 and 0.7.
impl From<&Header> for proto::HeaderResponse {
    fn from(header: &Header) -> Self {
        let shape = match header {
            Header::V1(_) => Shape::V1(header.into()),
            Header::V2(_) => Shape::V2(header.into()),
            Header::V3(_) => Shape::V3(header.into()),
            Header::V4(_) => Shape::V4(header_v4(header)),
            Header::V5(_) => Shape::V5(header_v5(header)),
            Header::V6(_) => Shape::V6(header_v5(header)),
            Header::V7(_) => Shape::V7(header_v5(header)),
        };
        Self {
            header: Some(shape),
        }
    }
}

impl From<&Header> for proto::HeaderV1 {
    fn from(header: &Header) -> Self {
        Self {
            chain_config: Some(header.chain_config().into()),
            height: header.height(),
            timestamp: header.timestamp_internal(),
            l1_head: header.l1_head(),
            l1_finalized: header.l1_finalized().map(Into::into),
            payload_commitment: header.payload_commitment().to_string(),
            builder_commitment: header.builder_commitment().to_string(),
            ns_table: Some(proto::NsTable {
                bytes: header.ns_table().encode().to_vec(),
            }),
            block_merkle_tree_root: header.block_merkle_tree_root().to_string(),
            fee_merkle_tree_root: header.fee_merkle_tree_root().to_string(),
            fee_info: header.fee_info().first().map(Into::into),
            // The accessor smooths `Option` into a `Vec` across versions; empty means unsigned.
            builder_signature: header.builder_signature().first().map(Into::into),
        }
    }
}

impl From<&Header> for proto::HeaderV3 {
    fn from(header: &Header) -> Self {
        Self {
            chain_config: Some(header.chain_config().into()),
            height: header.height(),
            timestamp: header.timestamp_internal(),
            l1_head: header.l1_head(),
            l1_finalized: header.l1_finalized().map(Into::into),
            payload_commitment: header.payload_commitment().to_string(),
            builder_commitment: header.builder_commitment().to_string(),
            ns_table: Some(proto::NsTable {
                bytes: header.ns_table().encode().to_vec(),
            }),
            block_merkle_tree_root: header.block_merkle_tree_root().to_string(),
            fee_merkle_tree_root: header.fee_merkle_tree_root().to_string(),
            fee_info: header.fee_info().first().map(Into::into),
            builder_signature: header.builder_signature().first().map(Into::into),
            reward_merkle_tree_root: reward_merkle_tree_root(header),
        }
    }
}

/// Private, unlike the 0.1 and 0.3 shapes above, because the two `expect`s are sound only under
/// the variant match in [`proto::HeaderResponse`]'s impl. A `From<&Header>` would let a 0.1 header
/// reach them.
fn header_v4(header: &Header) -> proto::HeaderV4 {
    proto::HeaderV4 {
        chain_config: Some(header.chain_config().into()),
        height: header.height(),
        timestamp: header.timestamp_internal(),
        timestamp_millis: header.timestamp_millis_internal(),
        l1_head: header.l1_head(),
        l1_finalized: header.l1_finalized().map(Into::into),
        payload_commitment: header.payload_commitment().to_string(),
        builder_commitment: header.builder_commitment().to_string(),
        ns_table: Some(proto::NsTable {
            bytes: header.ns_table().encode().to_vec(),
        }),
        block_merkle_tree_root: header.block_merkle_tree_root().to_string(),
        fee_merkle_tree_root: header.fee_merkle_tree_root().to_string(),
        fee_info: header.fee_info().first().map(Into::into),
        builder_signature: header.builder_signature().first().map(Into::into),
        reward_merkle_tree_root: reward_merkle_tree_root(header),
        total_reward_distributed: header
            .total_reward_distributed()
            .expect("0.4 and later headers carry total_reward_distributed")
            .to_string(),
        next_stake_table_hash: header.next_stake_table_hash().map(|hash| hash.to_string()),
    }
}

/// See [`header_v4`] for why this is not a `From` impl.
fn header_v5(header: &Header) -> proto::HeaderV5 {
    proto::HeaderV5 {
        chain_config: Some(header.chain_config().into()),
        height: header.height(),
        timestamp: header.timestamp_internal(),
        timestamp_millis: header.timestamp_millis_internal(),
        l1_head: header.l1_head(),
        l1_finalized: header.l1_finalized().map(Into::into),
        payload_commitment: header.payload_commitment().to_string(),
        builder_commitment: header.builder_commitment().to_string(),
        ns_table: Some(proto::NsTable {
            bytes: header.ns_table().encode().to_vec(),
        }),
        block_merkle_tree_root: header.block_merkle_tree_root().to_string(),
        fee_merkle_tree_root: header.fee_merkle_tree_root().to_string(),
        fee_info: header.fee_info().first().map(Into::into),
        builder_signature: header.builder_signature().first().map(Into::into),
        reward_merkle_tree_root: reward_merkle_tree_root(header),
        total_reward_distributed: header
            .total_reward_distributed()
            .expect("0.4 and later headers carry total_reward_distributed")
            .to_string(),
        next_stake_table_hash: header.next_stake_table_hash().map(|hash| hash.to_string()),
        leader_counts: header
            .leader_counts()
            .expect("0.5 and later headers carry leader_counts")
            .iter()
            .map(|count| *count as u32)
            .collect(),
    }
}

/// 0.3 uses the first reward tree (Left), later versions the second (Right). 0.1 and 0.2 have
/// none, and the accessor would fabricate an empty tree's commitment for them.
fn reward_merkle_tree_root(header: &Header) -> String {
    match header.reward_merkle_tree_root() {
        either::Either::Left(root) => root.to_string(),
        either::Either::Right(root) => root.to_string(),
    }
}

impl From<ResourceSyncStatus> for proto::ResourceSyncStatus {
    fn from(status: ResourceSyncStatus) -> Self {
        Self {
            missing: status.missing as u64,
            ranges: status
                .ranges
                .into_iter()
                .map(|range| proto::SyncStatusRange {
                    start: range.start as u64,
                    end: range.end as u64,
                    status: match range.status {
                        SyncStatus::Present => proto::SyncStatus::Present,
                        SyncStatus::Missing => proto::SyncStatus::Missing,
                        SyncStatus::Pruned => proto::SyncStatus::Pruned,
                    }
                    .into(),
                })
                .collect(),
        }
    }
}

impl From<SyncStatusQueryData> for proto::SyncStatusResponse {
    fn from(status: SyncStatusQueryData) -> Self {
        Self {
            blocks: Some(status.blocks.into()),
            leaves: Some(status.leaves.into()),
            vid_common: Some(status.vid_common.into()),
            pruned_height: status.pruned_height.map(|height| height as u64),
        }
    }
}

impl From<&TimeWindowQueryData<Header>> for proto::HeaderWindowResponse {
    fn from(window: &TimeWindowQueryData<Header>) -> Self {
        Self {
            window: window.window.iter().map(Into::into).collect(),
            prev: window.prev.as_ref().map(Into::into),
            next: window.next.as_ref().map(Into::into),
        }
    }
}

impl From<NodeLimits> for proto::NodeLimitsResponse {
    fn from(limits: NodeLimits) -> Self {
        Self {
            window_limit: limits.window_limit as u64,
        }
    }
}

impl From<PeerConfig<SeqTypes>> for proto::PeerConfig {
    fn from(peer: PeerConfig<SeqTypes>) -> Self {
        Self {
            stake_table_entry: Some(proto::StakeTableEntry {
                stake_key: Some(proto::BlsPublicKey {
                    key: peer.stake_table_entry.stake_key.to_string(),
                }),
                stake_amount: format!("{:#x}", peer.stake_table_entry.stake_amount),
            }),
            state_ver_key: Some(proto::SchnorrPublicKey {
                key: peer.state_ver_key.to_string(),
            }),
            connect_info: peer.connect_info.map(|info| proto::PeerConnectInfo {
                p2p_addr: info.p2p_addr.unbracketed_string(),
                x25519_key: info.x25519_key.to_string(),
            }),
        }
    }
}

impl From<RegisteredValidator<PubKey>> for proto::Validator {
    fn from(registered: RegisteredValidator<PubKey>) -> Self {
        let mut delegators: Vec<_> = registered
            .delegators
            .into_iter()
            .map(|(account, amount)| proto::Delegator {
                account: format!("{account:#x}"),
                amount: format!("{amount:#x}"),
            })
            .collect();
        // v1 serves a map, so the order is undefined.
        delegators.sort_by(|a, b| a.account.cmp(&b.account));

        Self {
            account: format!("{:#x}", registered.account),
            stake_table_key: registered.stake_table_key.map(|key| proto::BlsPublicKey {
                key: key.to_string(),
            }),
            state_ver_key: registered.state_ver_key.map(|key| proto::SchnorrPublicKey {
                key: key.to_string(),
            }),
            stake: format!("{:#x}", registered.stake),
            commission: registered.commission.into(),
            delegators,
            authenticated: registered.authenticated,
            x25519_key: registered.x25519_key.as_ref().map(ToString::to_string),
            p2p_addr: registered
                .p2p_addr
                .as_ref()
                .map(|addr| addr.unbracketed_string()),
        }
    }
}

impl From<HashMap<PubKey, f64>> for proto::ParticipationResponse {
    fn from(fractions: HashMap<PubKey, f64>) -> Self {
        let mut entries: Vec<(String, f64)> = fractions
            .into_iter()
            .map(|(key, participation)| (key.to_string(), participation))
            .collect();
        // v1 serves a map, so the order is undefined.
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            participation: entries
                .into_iter()
                .map(|(key, participation)| proto::ParticipationEntry {
                    key: Some(proto::BlsPublicKey { key }),
                    participation,
                })
                .collect(),
        }
    }
}

/// v1's serialized form is the source for the ADVZ and AvidM arms: jellyfish keeps the ADVZ
/// share's fields private, and the AvidM payload is ark-serialized into single TaggedBase64
/// strings that only its serde impl produces. The gf2 arm maps from the share's own accessors,
/// because its payload is raw bytes that a JSON round trip would inflate into one number per byte.
impl TryFrom<&VidShare> for proto::VidShareResponse {
    type Error = tonic::Status;

    fn try_from(share: &VidShare) -> Result<Self, Self::Error> {
        // Matched on the enum rather than sniffed from the JSON, so a fourth scheme fails to
        // compile instead of surfacing as a 500.
        let arm = match share {
            VidShare::V0(_) => {
                let json = to_json(share)?;
                let share = json_entry(&json, "V0")?;
                let evals_proof = json_entry(share, "evals_proof")?;
                proto::vid_share_response::Share::V0(proto::AdvzVidShare {
                    index: json_field(share, "index")?,
                    aggregate_proofs: json_field(share, "aggregate_proofs")?,
                    evals: json_field(share, "evals")?,
                    evals_proof: Some(proto::AdvzMerkleProof {
                        pos: json_field(evals_proof, "pos")?,
                        proof: json_array(evals_proof, "proof")?
                            .iter()
                            .map(advz_merkle_node)
                            .collect::<Result<_, _>>()?,
                    }),
                })
            },
            VidShare::V1(_) => {
                let json = to_json(share)?;
                let share = json_entry(&json, "V1")?;
                proto::vid_share_response::Share::V1(proto::AvidmVidShare {
                    index: json_field(share, "index")?,
                    ns_commits: json_field(share, "ns_commits")?,
                    ns_lens: json_field(share, "ns_lens")?,
                    content: json_array(share, "content")?
                        .iter()
                        .map(|content| {
                            let range = json_entry(content, "range")?;
                            Ok(proto::AvidmShareContent {
                                range: Some(proto::ShardRange {
                                    start: json_field(range, "start")?,
                                    end: json_field(range, "end")?,
                                }),
                                payload: json_field(content, "payload")?,
                                mt_proofs: json_field(content, "mt_proofs")?,
                            })
                        })
                        .collect::<Result<_, tonic::Status>>()?,
                })
            },
            VidShare::V2(gf2) => proto::vid_share_response::Share::V2(proto::AvidmGf2VidShare {
                namespaces: gf2
                    .ns_shares()
                    .iter()
                    .map(|namespace| proto::AvidmGf2Namespace {
                        range: Some(proto::ShardRange {
                            start: namespace.range().start as u64,
                            end: namespace.range().end as u64,
                        }),
                        payload: namespace.payload().to_vec(),
                        mt_proofs: namespace
                            .mt_proofs()
                            .iter()
                            .map(ToString::to_string)
                            .collect(),
                    })
                    .collect(),
            }),
        };
        Ok(Self { share: Some(arm) })
    }
}

fn advz_merkle_node(value: &serde_json::Value) -> Result<proto::AdvzMerkleNode, tonic::Status> {
    let node = if let Some(leaf) = value.get("Leaf") {
        Node::Leaf(proto::AdvzMerkleNodeLeaf {
            elem: json_field(leaf, "elem")?,
            pos: json_field(leaf, "pos")?,
            value: json_field(leaf, "value")?,
        })
    } else if let Some(branch) = value.get("Branch") {
        Node::Branch(proto::AdvzMerkleNodeBranch {
            children: json_array(branch, "children")?
                .iter()
                .map(advz_merkle_node)
                .collect::<Result<_, _>>()?,
            value: json_field(branch, "value")?,
        })
    } else if let Some(subtree) = value.get("ForgettenSubtree") {
        // Upstream's spelling, which the proto field name corrects.
        Node::ForgottenSubtree(proto::AdvzMerkleNodeForgottenSubtree {
            value: json_field(subtree, "value")?,
        })
    } else if value.as_str() == Some("Empty") {
        // A unit variant, so v1 writes it as a bare string.
        Node::Empty(proto::AdvzMerkleNodeEmpty {})
    } else {
        let arms: Vec<&str> = value
            .as_object()
            .map(|node| node.keys().map(String::as_str).collect())
            .unwrap_or_default();
        return Err(tonic::Status::internal(format!(
            "VID share JSON has an unknown Merkle node arm: {arms:?}"
        )));
    };
    Ok(proto::AdvzMerkleNode { node: Some(node) })
}

impl From<&[Header]> for proto::HeaderRangeResponse {
    fn from(headers: &[Header]) -> Self {
        Self {
            headers: headers.iter().map(Into::into).collect(),
        }
    }
}

impl From<&[LeafQueryData<SeqTypes>]> for proto::LeafRangeResponse {
    fn from(leaves: &[LeafQueryData<SeqTypes>]) -> Self {
        Self {
            leaves: leaves.iter().map(Into::into).collect(),
        }
    }
}

impl From<&[BlockQueryData<SeqTypes>]> for proto::BlockRangeResponse {
    fn from(blocks: &[BlockQueryData<SeqTypes>]) -> Self {
        Self {
            blocks: blocks.iter().map(Into::into).collect(),
        }
    }
}

impl From<&[PayloadQueryData<SeqTypes>]> for proto::PayloadRangeResponse {
    fn from(payloads: &[PayloadQueryData<SeqTypes>]) -> Self {
        Self {
            payloads: payloads.iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<&[VidCommonQueryData<SeqTypes>]> for proto::VidCommonRangeResponse {
    type Error = tonic::Status;

    fn try_from(items: &[VidCommonQueryData<SeqTypes>]) -> Result<Self, Self::Error> {
        Ok(Self {
            vid_common: items
                .iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<&[BlockSummaryQueryData<SeqTypes>]> for proto::BlockSummaryRangeResponse {
    fn from(summaries: &[BlockSummaryQueryData<SeqTypes>]) -> Self {
        Self {
            summaries: summaries.iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<&[NamespaceProofQueryData]> for proto::NamespaceProofRangeResponse {
    type Error = tonic::Status;

    fn try_from(proofs: &[NamespaceProofQueryData]) -> Result<Self, Self::Error> {
        Ok(Self {
            proofs: proofs
                .iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl<E, I, T, const ARITY: usize> From<&MerkleProof<E, I, T, ARITY>> for proto::MerklePathResponse
where
    E: Element + CanonicalSerialize,
    I: Index + CanonicalSerialize,
    T: NodeValue,
{
    fn from(proof: &MerkleProof<E, I, T, ARITY>) -> Self {
        Self {
            pos: field_tb64(&proof.pos),
            proof: proof.proof.iter().map(Into::into).collect(),
        }
    }
}

impl<E, I, T> From<&MerkleNode<E, I, T>> for proto::AdvzMerkleNode
where
    E: Element + CanonicalSerialize,
    I: Index + CanonicalSerialize,
    T: NodeValue,
{
    fn from(node: &MerkleNode<E, I, T>) -> Self {
        let node = match node {
            MerkleNode::Empty => Node::Empty(proto::AdvzMerkleNodeEmpty {}),
            MerkleNode::Branch { value, children } => Node::Branch(proto::AdvzMerkleNodeBranch {
                value: field_tb64(value),
                children: children.iter().map(|child| Self::from(&**child)).collect(),
            }),
            MerkleNode::Leaf { value, pos, elem } => Node::Leaf(proto::AdvzMerkleNodeLeaf {
                value: field_tb64(value),
                pos: field_tb64(pos),
                elem: field_tb64(elem),
            }),
            MerkleNode::ForgettenSubtree { value } => {
                Node::ForgottenSubtree(proto::AdvzMerkleNodeForgottenSubtree {
                    value: field_tb64(value),
                })
            },
        };
        Self { node: Some(node) }
    }
}

/// The encoding jellyfish's `canonical` serde helper gives every hash, index and element of a
/// proof: ark-compressed bytes under the `FIELD` tag, whatever the underlying type is.
/// `block_state_path_mirrors_its_v1_rendering` pins the two to the same bytes.
fn field_tb64<T>(value: &T) -> String
where
    T: CanonicalSerialize,
{
    let mut bytes = Vec::new();
    value
        .serialize_compressed(&mut bytes)
        .expect("serializing to a Vec cannot fail");
    TaggedBase64::new("FIELD", &bytes)
        .expect("FIELD is a valid tag")
        .to_string()
}

/// jellyfish keeps the fields of the VID shares and common, the range proofs and the bad-encoding
/// proof private, so v1's serde encoding is their one public view.
fn to_json(value: &impl serde::Serialize) -> Result<serde_json::Value, tonic::Status> {
    serde_json::to_value(value)
        .map_err(|err| tonic::Status::internal(format!("v1 encoding failed: {err}")))
}

/// A missing or mistyped field means the upstream type changed, which is a 500 rather than an
/// empty value. v1 writes byte fields as integer arrays, which read back as `Vec<u8>`.
fn json_field<T>(value: &serde_json::Value, field: &str) -> Result<T, tonic::Status>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(json_entry(value, field)?)
        .map_err(|err| tonic::Status::internal(format!("v1 JSON {field}: {err}")))
}

fn json_entry<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a serde_json::Value, tonic::Status> {
    value
        .get(field)
        .ok_or_else(|| tonic::Status::internal(format!("v1 JSON has no {field}")))
}

fn json_array<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a [serde_json::Value], tonic::Status> {
    json_entry(value, field)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| tonic::Status::internal(format!("v1 JSON {field}: not an array")))
}

// Not beside the runtime config in the node crate: both types are foreign there, so the orphan
// rule refuses the impl. The destructure below reaches only the inner config, because the
// wrapper's fields are private; a test in espresso-types guards those.
impl From<PublicNetworkConfig> for proto::HotshotConfigResponse {
    fn from(public_config: PublicNetworkConfig) -> Self {
        let config = public_config.hotshot_config().into_hotshot_config();
        // Destructured without `..` so that a field added to HotShotConfig fails to compile here
        // instead of becoming a parameter v2 silently never serves.
        let HotShotConfig {
            start_threshold: (start_threshold_numerator, start_threshold_denominator),
            num_nodes_with_stake,
            known_nodes_with_stake,
            known_da_nodes,
            da_committees,
            da_staked_committee_size,
            fixed_leader_for_gpuvid,
            next_view_timeout,
            view_sync_timeout,
            num_bootstrap,
            builder_timeout,
            data_request_delay,
            builder_urls,
            start_proposing_view,
            stop_proposing_view,
            start_voting_view,
            stop_voting_view,
            start_proposing_time,
            stop_proposing_time,
            start_voting_time,
            stop_voting_time,
            epoch_height,
            epoch_start_block,
            stake_table_capacity,
            drb_difficulty,
            drb_upgrade_difficulty,
        } = config;
        Self {
            start_threshold_numerator,
            start_threshold_denominator,
            num_nodes_with_stake: num_nodes_with_stake.get() as u64,
            da_staked_committee_size: da_staked_committee_size as u64,
            next_view_timeout_ms: next_view_timeout,
            view_sync_timeout_ms: view_sync_timeout.as_millis() as u64,
            builder_timeout_ms: builder_timeout.as_millis() as u64,
            data_request_delay_ms: data_request_delay.as_millis() as u64,
            builder_urls: builder_urls.iter().map(ToString::to_string).collect(),
            start_proposing_view,
            stop_proposing_view,
            start_voting_view,
            stop_voting_view,
            start_proposing_time,
            stop_proposing_time,
            start_voting_time,
            stop_voting_time,
            epoch_height,
            epoch_start_block,
            stake_table_capacity: stake_table_capacity as u64,
            drb_difficulty,
            drb_upgrade_difficulty,
            known_nodes_with_stake: known_nodes_with_stake.into_iter().map(Into::into).collect(),
            known_da_nodes: known_da_nodes.into_iter().map(Into::into).collect(),
            da_committees: da_committees
                .into_iter()
                .map(|da_committee| proto::VersionedDaCommittee {
                    start_version: da_committee.start_version.to_string(),
                    start_epoch: da_committee.start_epoch,
                    committee: da_committee.committee.into_iter().map(Into::into).collect(),
                })
                .collect(),
            fixed_leader_for_gpuvid: fixed_leader_for_gpuvid as u64,
            num_bootstrap: num_bootstrap as u64,
            commit_sha: public_config.commit_sha().to_string(),
            indexed_da: public_config.indexed_da(),
            cdn_marshal_address: public_config.cdn_marshal_address().map(ToString::to_string),
            libp2p_config: public_config
                .libp2p_config()
                .map(|libp2p| proto::Libp2pNetworkConfig {
                    bootstrap_nodes: libp2p
                        .bootstrap_nodes
                        .iter()
                        .map(|(peer_id, multiaddr)| proto::Libp2pBootstrapNode {
                            peer_id: peer_id.to_string(),
                            multiaddr: multiaddr.to_string(),
                        })
                        .collect(),
                }),
            combined_network_config: public_config.combined_network_config().map(|combined| {
                proto::CombinedNetworkConfig {
                    delay_duration_ms: combined.delay_duration.as_millis() as u64,
                }
            }),
            builder: match public_config.builder() {
                BuilderType::External => proto::BuilderType::External,
                BuilderType::Simple => proto::BuilderType::Simple,
                BuilderType::Random => proto::BuilderType::Random,
            }
            .into(),
        }
    }
}

fn quorum_signatures<V, T>(
    cert: &SimpleCertificate<SeqTypes, V, T>,
) -> Option<proto::QuorumSignatures>
where
    V: Voteable<SeqTypes>,
    T: Threshold<SeqTypes>,
{
    // v1 serializes the bitvec crate's memory layout, the API publishes who signed instead.
    cert.signatures
        .as_ref()
        .map(|(signature, signers)| proto::QuorumSignatures {
            signature: signature.to_string(),
            signers: signers.iter().by_vals().collect(),
        })
}

impl From<&QuorumData2<SeqTypes>> for proto::QuorumData2 {
    fn from(data: &QuorumData2<SeqTypes>) -> Self {
        Self {
            leaf_commit: data.leaf_commit.to_string(),
            epoch: data.epoch.map(|epoch| epoch.u64()),
            block_number: data.block_number,
        }
    }
}

/// The QC and the next-epoch QC, which vote on the same data.
impl<V> From<&SimpleCertificate<SeqTypes, V, SuccessThreshold>> for proto::QuorumCertificate2
where
    V: Voteable<SeqTypes> + Borrow<QuorumData2<SeqTypes>>,
{
    fn from(cert: &SimpleCertificate<SeqTypes, V, SuccessThreshold>) -> Self {
        Self {
            data: Some(cert.data.borrow().into()),
            vote_commitment: cert.vote_commitment().to_string(),
            view_number: cert.view_number.u64(),
            signatures: quorum_signatures(cert),
        }
    }
}

impl From<&Certificate2<SeqTypes>> for proto::Certificate2 {
    fn from(cert: &Certificate2<SeqTypes>) -> Self {
        Self {
            data: Some(proto::Vote2Data {
                leaf_commit: cert.data.leaf_commit.to_string(),
                epoch: cert.data.epoch.u64(),
                block_number: cert.data.block_number,
            }),
            vote_commitment: cert.vote_commitment().to_string(),
            view_number: cert.view_number.u64(),
            signatures: quorum_signatures(cert),
        }
    }
}

impl From<vbs::version::Version> for proto::ProtocolVersion {
    fn from(version: vbs::version::Version) -> Self {
        Self {
            major: u32::from(version.major),
            minor: u32::from(version.minor),
        }
    }
}

impl From<&UpgradeCertificate<SeqTypes>> for proto::UpgradeCertificate {
    fn from(cert: &UpgradeCertificate<SeqTypes>) -> Self {
        Self {
            data: Some(proto::UpgradeProposalData {
                old_version: Some(cert.data.old_version.into()),
                new_version: Some(cert.data.new_version.into()),
                decide_by: cert.data.decide_by.u64(),
                new_version_hash: cert.data.new_version_hash.clone(),
                old_version_last_view: cert.data.old_version_last_view.u64(),
                new_version_first_view: cert.data.new_version_first_view.u64(),
            }),
            vote_commitment: cert.vote_commitment().to_string(),
            view_number: cert.view_number.u64(),
            signatures: quorum_signatures(cert),
        }
    }
}

impl From<&ViewChangeEvidence2<SeqTypes>> for proto::ViewChangeEvidence2 {
    fn from(evidence: &ViewChangeEvidence2<SeqTypes>) -> Self {
        use proto::view_change_evidence2::Evidence;

        let evidence = match evidence {
            ViewChangeEvidence2::Timeout(cert) => Evidence::Timeout(cert.into()),
            ViewChangeEvidence2::Timeout3(cert) => Evidence::Timeout3(cert.into()),
            ViewChangeEvidence2::ViewSync(cert) => Evidence::ViewSync(cert.into()),
        };
        Self {
            evidence: Some(evidence),
        }
    }
}

impl From<&TimeoutCertificate2<SeqTypes>> for proto::TimeoutCertificate2 {
    fn from(cert: &TimeoutCertificate2<SeqTypes>) -> Self {
        Self {
            data: Some(proto::TimeoutData2 {
                view: cert.data.view.u64(),
                epoch: cert.data.epoch.map(|epoch| epoch.u64()),
            }),
            vote_commitment: cert.vote_commitment().to_string(),
            view_number: cert.view_number.u64(),
            signatures: quorum_signatures(cert),
        }
    }
}

impl From<&TimeoutCertificate3<SeqTypes>> for proto::TimeoutCertificate3 {
    fn from(cert: &TimeoutCertificate3<SeqTypes>) -> Self {
        Self {
            data: Some(proto::TimeoutData3 {
                view: cert.data.view.u64(),
                epoch: cert.data.epoch.u64(),
            }),
            vote_commitment: cert.vote_commitment().to_string(),
            view_number: cert.view_number.u64(),
            signatures: quorum_signatures(cert),
        }
    }
}

impl From<&ViewSyncFinalizeCertificate2<SeqTypes>> for proto::ViewSyncFinalizeCertificate2 {
    fn from(cert: &ViewSyncFinalizeCertificate2<SeqTypes>) -> Self {
        Self {
            data: Some(proto::ViewSyncFinalizeData2 {
                relay: cert.data.relay,
                round: cert.data.round.u64(),
                epoch: cert.data.epoch.map(|epoch| epoch.u64()),
            }),
            vote_commitment: cert.vote_commitment().to_string(),
            view_number: cert.view_number.u64(),
            signatures: quorum_signatures(cert),
        }
    }
}

impl From<&Payload> for proto::Payload {
    fn from(payload: &Payload) -> Self {
        Self {
            raw_payload: payload.raw_payload().to_vec(),
            ns_table: Some(proto::NsTable {
                bytes: payload.ns_table().encode().to_vec(),
            }),
        }
    }
}

impl From<&Leaf2<SeqTypes>> for proto::Leaf2 {
    fn from(leaf: &Leaf2<SeqTypes>) -> Self {
        Self {
            view_number: leaf.view_number().u64(),
            justify_qc: Some(leaf.justify_qc().into()),
            next_epoch_justify_qc: leaf.next_epoch_justify_qc().map(Into::into),
            parent_commitment: leaf.parent_commitment().to_string(),
            block_header: Some(leaf.block_header().into()),
            upgrade_certificate: leaf.upgrade_certificate().map(Into::into),
            block_payload: leaf.block_payload_ref().map(Into::into),
            view_change_evidence: leaf.view_change_evidence.as_ref().map(Into::into),
            next_drb_result: leaf
                .next_drb_result
                .map(|result| result.to_vec())
                .unwrap_or_default(),
            with_epoch: leaf.with_epoch,
        }
    }
}

impl From<&LeafQueryData<SeqTypes>> for proto::LeafResponse {
    fn from(leaf: &LeafQueryData<SeqTypes>) -> Self {
        Self {
            leaf: Some(leaf.leaf().into()),
            qc: Some(leaf.qc().into()),
        }
    }
}

impl From<&BlockQueryData<SeqTypes>> for proto::BlockResponse {
    fn from(block: &BlockQueryData<SeqTypes>) -> Self {
        Self {
            header: Some(block.header().into()),
            payload: Some(block.payload().into()),
            hash: block.hash().to_string(),
            size: block.size(),
            num_transactions: block.num_transactions(),
        }
    }
}

impl From<&PayloadQueryData<SeqTypes>> for proto::PayloadResponse {
    fn from(payload: &PayloadQueryData<SeqTypes>) -> Self {
        Self {
            height: payload.height,
            block_hash: payload.block_hash().to_string(),
            hash: payload.hash().to_string(),
            size: payload.size(),
            data: Some(payload.data().into()),
        }
    }
}

impl TryFrom<&VidCommonQueryData<SeqTypes>> for proto::VidCommonResponse {
    type Error = tonic::Status;

    fn try_from(common: &VidCommonQueryData<SeqTypes>) -> Result<Self, Self::Error> {
        use proto::vid_common_response::Common;

        let arm = match common.common() {
            VidCommon::V0(advz) => {
                let value = to_json(advz)?;
                Common::V0(proto::AdvzCommon {
                    poly_commits: json_field(&value, "poly_commits")?,
                    all_evals_digest: json_field(&value, "all_evals_digest")?,
                    payload_byte_len: json_field(&value, "payload_byte_len")?,
                    num_storage_nodes: json_field(&value, "num_storage_nodes")?,
                    multiplicity: json_field(&value, "multiplicity")?,
                })
            },
            VidCommon::V1(param) => Common::V1(proto::AvidmCommon {
                total_weights: param.total_weights as u64,
                recovery_threshold: param.recovery_threshold as u64,
            }),
            VidCommon::V2(namespaced) => Common::V2(proto::AvidmGf2Common {
                param: Some(proto::AvidmGf2Param {
                    total_weights: namespaced.param.total_weights as u64,
                    recovery_threshold: namespaced.param.recovery_threshold as u64,
                }),
                ns_commits: namespaced
                    .ns_commits
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                ns_lens: namespaced.ns_lens.iter().map(|len| *len as u64).collect(),
            }),
        };
        Ok(Self {
            height: common.height,
            block_hash: common.block_hash().to_string(),
            payload_hash: common.payload_hash().to_string(),
            common: Some(arm),
        })
    }
}

impl From<&AvidMNsProof> for proto::NsProofPayload {
    fn from(proof: &AvidMNsProof) -> Self {
        Self {
            ns_index: proof.0.ns_index as u64,
            ns_payload: proof.0.ns_payload.to_vec(),
            ns_proof: proof.0.ns_proof.to_string(),
        }
    }
}

impl From<&AvidmGf2NsProof> for proto::NsProofPayload {
    fn from(proof: &AvidmGf2NsProof) -> Self {
        Self {
            ns_index: proof.0.ns_index as u64,
            ns_payload: proof.0.ns_payload.to_vec(),
            ns_proof: proof.0.ns_proof.to_string(),
        }
    }
}

impl TryFrom<&TxProof> for proto::TxProof {
    type Error = tonic::Status;

    fn try_from(proof: &TxProof) -> Result<Self, Self::Error> {
        use proto::tx_proof::Proof;

        let arm = match proof {
            TxProof::V0(advz) => Proof::V0(proto::AdvzTxProof {
                tx_index: advz.tx_index().to_bytes().to_vec(),
                payload_num_txs: advz.payload_num_txs().to_payload_bytes().to_vec(),
                payload_proof_num_txs: Some(advz.payload_proof_num_txs().try_into()?),
                payload_tx_table_entries: advz.payload_tx_table_entries().to_payload_bytes(),
                payload_proof_tx_table_entries: Some(
                    advz.payload_proof_tx_table_entries().try_into()?,
                ),
                payload_proof_tx: advz.payload_proof_tx().map(TryInto::try_into).transpose()?,
            }),
            TxProof::V1(avidm) => Proof::V1(proto::AvidmTxProof {
                tx_index: avidm.tx_index().to_bytes().to_vec(),
                ns_proof: Some(avidm.ns_proof().into()),
            }),
            TxProof::V2(gf2) => Proof::V2(proto::AvidmGf2TxProof {
                tx_index: gf2.tx_index().to_bytes().to_vec(),
                ns_proof: Some(gf2.ns_proof().into()),
            }),
        };
        Ok(Self { proof: Some(arm) })
    }
}

impl TryFrom<&SmallRangeProofType> for proto::SmallRangeProof {
    type Error = tonic::Status;

    fn try_from(proof: &SmallRangeProofType) -> Result<Self, Self::Error> {
        let value = to_json(proof)?;
        Ok(Self {
            proofs: json_field(&value, "proofs")?,
            prefix_bytes: json_field(&value, "prefix_bytes")?,
            suffix_bytes: json_field(&value, "suffix_bytes")?,
        })
    }
}

impl TryFrom<&LargeRangeProofType> for proto::LargeRangeProof {
    type Error = tonic::Status;

    fn try_from(proof: &LargeRangeProofType) -> Result<Self, Self::Error> {
        let value = to_json(proof)?;
        Ok(Self {
            prefix_elems: json_field(&value, "prefix_elems")?,
            suffix_elems: json_field(&value, "suffix_elems")?,
            prefix_bytes: json_field(&value, "prefix_bytes")?,
            suffix_bytes: json_field(&value, "suffix_bytes")?,
        })
    }
}

impl From<&Transaction> for proto::Transaction {
    fn from(tx: &Transaction) -> Self {
        Self {
            namespace: tx.namespace().0,
            payload: tx.payload().to_vec(),
        }
    }
}

impl From<&TransactionQueryData<SeqTypes>> for proto::TransactionResponse {
    fn from(tx: &TransactionQueryData<SeqTypes>) -> Self {
        Self {
            transaction: Some(tx.transaction().into()),
            hash: tx.hash().to_string(),
            index: tx.index(),
            block_hash: tx.block_hash().to_string(),
            block_height: tx.block_height(),
            namespace: tx.namespace().0,
            pos_in_namespace: tx.pos_in_namespace(),
        }
    }
}

impl TryFrom<&TransactionWithProofQueryData<SeqTypes>> for proto::TransactionWithProofResponse {
    type Error = tonic::Status;

    fn try_from(tx: &TransactionWithProofQueryData<SeqTypes>) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction: Some(tx.transaction().into()),
            hash: tx.hash().to_string(),
            index: tx.index(),
            block_hash: tx.block_hash().to_string(),
            block_height: tx.block_height(),
            namespace: tx.namespace().0,
            pos_in_namespace: tx.pos_in_namespace(),
            proof: Some(tx.proof().try_into()?),
        })
    }
}

impl From<&BlockSummaryQueryData<SeqTypes>> for proto::BlockSummaryResponse {
    fn from(summary: &BlockSummaryQueryData<SeqTypes>) -> Self {
        Self {
            header: Some((&summary.header).into()),
            hash: summary.hash.to_string(),
            size: summary.size,
            num_transactions: summary.num_transactions,
            namespaces: summary
                .namespaces
                .iter()
                .map(|(namespace, info)| {
                    (
                        namespace.0,
                        proto::NamespaceInfo {
                            num_transactions: info.num_transactions,
                            size: info.size,
                        },
                    )
                })
                .collect(),
        }
    }
}

impl TryFrom<&AvidMIncorrectEncodingNsProof> for proto::AvidmBadEncodingNsProof {
    type Error = tonic::Status;

    fn try_from(proof: &AvidMIncorrectEncodingNsProof) -> Result<Self, Self::Error> {
        let inner = &proof.0;
        let value = to_json(&inner.ns_proof)?;
        Ok(Self {
            ns_index: inner.ns_index as u64,
            ns_commit: inner.ns_commit.to_string(),
            ns_mt_proof: inner.ns_mt_proof.to_string(),
            ns_proof: Some(proto::AvidmBadEncodingProof {
                recovered_poly: json_field(&value, "recovered_poly")?,
                raw_shares: json_field(&value, "raw_shares")?,
            }),
        })
    }
}

impl TryFrom<&NsProof> for proto::NsProof {
    type Error = tonic::Status;

    fn try_from(proof: &NsProof) -> Result<Self, Self::Error> {
        use proto::ns_proof::Proof;

        let arm = match proof {
            NsProof::V0(advz) => Proof::V0(proto::AdvzNsProof {
                ns_index: advz.ns_index.to_bytes().to_vec(),
                ns_payload: advz.ns_payload.as_bytes_slice().to_vec(),
                ns_proof: advz.ns_proof.as_ref().map(TryInto::try_into).transpose()?,
            }),
            NsProof::V1(avidm) => Proof::V1(avidm.into()),
            NsProof::V1IncorrectEncoding(bad) => Proof::V1IncorrectEncoding(bad.try_into()?),
            NsProof::V2(gf2) => Proof::V2(gf2.into()),
        };
        Ok(Self { proof: Some(arm) })
    }
}

impl TryFrom<&NamespaceProofQueryData> for proto::NamespaceProofResponse {
    type Error = tonic::Status;

    fn try_from(data: &NamespaceProofQueryData) -> Result<Self, Self::Error> {
        Ok(Self {
            proof: data.proof.as_ref().map(TryInto::try_into).transpose()?,
            transactions: data.transactions.iter().map(Into::into).collect(),
        })
    }
}

impl From<&StateCertQueryDataV1<SeqTypes>> for proto::StateCertV1Response {
    fn from(cert: &StateCertQueryDataV1<SeqTypes>) -> Self {
        let cert = &cert.0;
        Self {
            epoch: cert.epoch.u64(),
            light_client_state: cert.light_client_state.to_string(),
            next_stake_table_state: cert.next_stake_table_state.to_string(),
            signatures: cert
                .signatures
                .iter()
                .map(|(key, signature)| proto::StateSignatureV1 {
                    key: key.to_string(),
                    signature: signature.to_string(),
                })
                .collect(),
        }
    }
}

impl From<&StateCertQueryDataV2<SeqTypes>> for proto::StateCertV2Response {
    fn from(cert: &StateCertQueryDataV2<SeqTypes>) -> Self {
        let cert = &cert.0;
        Self {
            epoch: cert.epoch.u64(),
            light_client_state: cert.light_client_state.to_string(),
            next_stake_table_state: cert.next_stake_table_state.to_string(),
            signatures: cert
                .signatures
                .iter()
                .map(|(key, lcv3, lcv2)| proto::StateSignatureV2 {
                    key: key.to_string(),
                    lcv3_signature: lcv3.to_string(),
                    lcv2_signature: lcv2.to_string(),
                })
                .collect(),
            auth_root: format!("{:#x}", cert.auth_root),
        }
    }
}
