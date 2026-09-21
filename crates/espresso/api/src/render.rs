//! Consensus and query service types rendered as v2 proto messages.

use std::collections::HashMap;

use espresso_types::{
    BuilderSignature, FeeInfo, Header, L1BlockInfo, PubKey, SeqTypes,
    config::PublicNetworkConfig,
    v0_3::{RegisteredValidator, ResolvableChainConfig},
};
use hotshot_query_service_types::node::{ResourceSyncStatus, SyncStatus};
use hotshot_types::{
    HotShotConfig, PeerConfig, data::VidShare, network::BuilderType, traits::EncodeBytes as _,
};

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
                let json = serde_json::to_value(share).map_err(|err| {
                    tonic::Status::internal(format!("VID share does not serialize: {err}"))
                })?;
                let share = json.get("V0").ok_or_else(|| vid_missing("the V0 arm"))?;
                proto::vid_share_response::Share::V0(proto::AdvzVidShare {
                    index: vid_u32(share, "index")?,
                    aggregate_proofs: vid_string(share, "aggregate_proofs")?,
                    evals: vid_string(share, "evals")?,
                    evals_proof: Some(advz_merkle_proof(
                        share
                            .get("evals_proof")
                            .ok_or_else(|| vid_missing("evals_proof"))?,
                    )?),
                })
            },
            VidShare::V1(_) => {
                let json = serde_json::to_value(share).map_err(|err| {
                    tonic::Status::internal(format!("VID share does not serialize: {err}"))
                })?;
                let share = json.get("V1").ok_or_else(|| vid_missing("the V1 arm"))?;
                proto::vid_share_response::Share::V1(proto::AvidmVidShare {
                    index: vid_u32(share, "index")?,
                    ns_commits: vid_array(share, "ns_commits")?
                        .iter()
                        .map(|commit| {
                            commit
                                .as_str()
                                .map(str::to_owned)
                                .ok_or_else(|| vid_missing("ns_commits entry"))
                        })
                        .collect::<Result<_, _>>()?,
                    ns_lens: vid_array(share, "ns_lens")?
                        .iter()
                        .map(|len| len.as_u64().ok_or_else(|| vid_missing("ns_lens entry")))
                        .collect::<Result<_, _>>()?,
                    content: vid_array(share, "content")?
                        .iter()
                        .map(|content| {
                            Ok(proto::AvidmShareContent {
                                range: Some(shard_range(content)?),
                                payload: vid_string(content, "payload")?,
                                mt_proofs: vid_string(content, "mt_proofs")?,
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

fn advz_merkle_proof(value: &serde_json::Value) -> Result<proto::AdvzMerkleProof, tonic::Status> {
    Ok(proto::AdvzMerkleProof {
        pos: vid_string(value, "pos")?,
        proof: vid_array(value, "proof")?
            .iter()
            .map(advz_merkle_node)
            .collect::<Result<_, _>>()?,
    })
}

fn advz_merkle_node(value: &serde_json::Value) -> Result<proto::AdvzMerkleNode, tonic::Status> {
    let node = if let Some(leaf) = value.get("Leaf") {
        Node::Leaf(proto::AdvzMerkleNodeLeaf {
            elem: vid_string(leaf, "elem")?,
            pos: vid_string(leaf, "pos")?,
            value: vid_string(leaf, "value")?,
        })
    } else if let Some(branch) = value.get("Branch") {
        Node::Branch(proto::AdvzMerkleNodeBranch {
            children: vid_array(branch, "children")?
                .iter()
                .map(advz_merkle_node)
                .collect::<Result<_, _>>()?,
            value: vid_string(branch, "value")?,
        })
    } else if let Some(subtree) = value.get("ForgettenSubtree") {
        // Upstream's spelling, which the proto field name corrects.
        Node::ForgottenSubtree(proto::AdvzMerkleNodeForgottenSubtree {
            value: vid_string(subtree, "value")?,
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

fn shard_range(value: &serde_json::Value) -> Result<proto::ShardRange, tonic::Status> {
    let range = value.get("range").ok_or_else(|| vid_missing("range"))?;
    Ok(proto::ShardRange {
        start: vid_u64(range, "start")?,
        end: vid_u64(range, "end")?,
    })
}

fn vid_string(value: &serde_json::Value, field: &str) -> Result<String, tonic::Status> {
    value[field]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| vid_missing(field))
}

fn vid_u64(value: &serde_json::Value, field: &str) -> Result<u64, tonic::Status> {
    value[field].as_u64().ok_or_else(|| vid_missing(field))
}

fn vid_u32(value: &serde_json::Value, field: &str) -> Result<u32, tonic::Status> {
    u32::try_from(vid_u64(value, field)?)
        .map_err(|_| tonic::Status::internal(format!("VID share {field} does not fit in u32")))
}

fn vid_array<'a>(
    value: &'a serde_json::Value,
    field: &str,
) -> Result<&'a Vec<serde_json::Value>, tonic::Status> {
    value[field].as_array().ok_or_else(|| vid_missing(field))
}

/// An unexpected shape means the upstream type changed; better a 500 than empty fields.
fn vid_missing(field: &str) -> tonic::Status {
    tonic::Status::internal(format!("VID share JSON has no {field}"))
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
