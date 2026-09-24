//! Implementations of the v1 API traits and the v2 tonic service traits, both reading the one
//! data source this type wraps.

use std::{
    collections::HashMap,
    num::NonZeroUsize,
    ops::{Bound, Deref, Range},
    time::Duration,
};

use alloy::primitives::utils::format_ether;
use async_trait::async_trait;
use chrono::SecondsFormat;
use committable::Committable as _;
use disco_types::{error::Error as _, status::StatusCode};
use espresso_api::{
    error::{AvailabilityError, to_status},
    proto,
    v1::{self, HotShotAvailabilityApi},
};
use espresso_types::{
    NamespaceId, NamespaceProofQueryData, NsProof, SeqTypes,
    v0::sparse_mt::KeccakNode,
    v0_3::{RewardAccountV1, RewardAmount as InternalRewardAmount, RewardMerkleTreeV1},
    v0_4::{
        RewardAccountQueryDataV2 as InternalRewardAccountQueryData, RewardAccountV2,
        RewardMerkleTreeV2,
    },
    v0_6::RewardClaimError,
};
use futures::{StreamExt as _, TryStreamExt as _, join, stream::BoxStream};
use hotshot_contract_adapter::reward::RewardClaimInput as InternalRewardClaimInput;
use hotshot_events_service::events_source::EventsSource as _;
use hotshot_new_protocol::message::Certificate2;
use hotshot_query_service::{
    Header as HsHeader, QueryError,
    availability::{
        AvailabilityDataSource, BlockId as HsBlockId, BlockQueryData, BlockSummaryQueryData,
        LeafId as HsLeafId, LeafQueryData, Limits as HsLimits, PayloadQueryData,
        QueryablePayload as _, TransactionQueryData, TransactionWithProofQueryData,
        VidCommonQueryData,
    },
    data_source::{VersionedDataSource as _, storage::AvailabilityStorage as _},
    explorer::{
        BlockIdentifier, BlockRange, ExplorerDataSource as _, GetBlockSummariesRequest,
        GetTransactionSummariesRequest, TransactionIdentifier, TransactionRange,
        TransactionSummaryFilter,
    },
    merklized_state::{
        MerklizedStateDataSource, MerklizedStateHeightPersistence, Snapshot as HsSnapshot,
    },
    node::{NodeDataSource as _, WindowStart},
    status::HasMetrics as _,
    types::HeightIndexed,
};
use hotshot_types::{
    data::{EpochNumber, VidShare},
    utils::{epoch_from_block_number, root_block_in_epoch},
    vid::avidm::AvidMShare,
};
use jf_merkle_tree_compat::{
    MerkleTreeScheme,
    prelude::{MerkleProof as InternalMerkleProof, MerkleProof as JfMerkleProof},
};
use prometheus::Encoder as _;
use serde_json;
use tagged_base64::TaggedBase64;

use super::{
    RewardMerkleTreeDataSource, RewardMerkleTreeV2Data as InternalRewardTreeData,
    data_source::{
        CatchupDataSource, DatabaseMetadataSource, HotShotConfigDataSource, MigrationStatus,
        NodeKeysDataSource, NodePublicKeys, NodeStateDataSource, PruningDataSource,
        RequestResponseDataSource, StakeTableDataSource, StakeTableWithEpochNumber,
        StateCertDataSource, StateCertFetchingDataSource, StateSignatureDataSource,
        SubmitDataSource, TableSize, TokenDataSource,
    },
};

/// Timeout for failing requests due to missing data.
///
/// If data needed to respond to a request is missing, it can (in some cases) be fetched from an
/// external provider. This parameter controls how long the request handler will wait for
/// missing data to be fetched before giving up and failing the request.
///
/// Matches the `hotshot_query_service` availability API default.
const FETCH_TIMEOUT: Duration = Duration::from_millis(500);

/// Node API state implementation
///
/// This struct implements the v1 API traits (internal types) and the v2 tonic service traits
/// (proto types).
#[derive(Clone)]
pub struct NodeApiStateImpl<D> {
    data_source: D,
    env_vars: std::sync::Arc<Vec<String>>,
    public_node_config: Option<std::sync::Arc<crate::options::PublicNodeConfig>>,
    ranges_concurrency: NonZeroUsize,
}

impl<D> NodeApiStateImpl<D> {
    pub fn new(data_source: D) -> Self {
        Self {
            data_source,
            env_vars: std::sync::Arc::new(Vec::new()),
            public_node_config: None,
            ranges_concurrency: NonZeroUsize::new(4).unwrap(),
        }
    }

    pub fn with_ranges_concurrency(mut self, concurrency: NonZeroUsize) -> Self {
        self.ranges_concurrency = concurrency;
        self
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
}

#[async_trait]
impl<D> v1::RewardApi for NodeApiStateImpl<D>
where
    D: RewardMerkleTreeDataSource + Deref,
    D::Target: hotshot_query_service::merklized_state::MerklizedStateHeightPersistence
        + hotshot_query_service::merklized_state::MerklizedStateDataSource<
            SeqTypes,
            espresso_types::v0_3::RewardMerkleTreeV1,
            {
                <espresso_types::v0_3::RewardMerkleTreeV1 as jf_merkle_tree_compat::MerkleTreeScheme>::ARITY
            },
        > + hotshot_query_service::merklized_state::MerklizedStateDataSource<
            SeqTypes,
            espresso_types::v0_4::RewardMerkleTreeV2,
            {
                <espresso_types::v0_4::RewardMerkleTreeV2 as jf_merkle_tree_compat::MerkleTreeScheme>::ARITY
            },
        > + Send
        + Sync,
{
    type RewardClaimInput = InternalRewardClaimInput;
    type RewardBalance = InternalRewardAmount;
    type RewardAccountQueryData = InternalRewardAccountQueryData;
    type RewardAmounts = Vec<(alloy::primitives::Address, InternalRewardAmount)>;
    type RewardMerkleTreeData = Vec<u8>;
    type RewardAccountQueryDataV1 = espresso_types::v0_3::RewardAccountQueryDataV1;
    type RewardStatePathV1 = InternalMerkleProof<
        InternalRewardAmount,
        espresso_types::v0_3::RewardAccountV1,
        jf_merkle_tree_compat::prelude::Sha3Node,
        {
            <espresso_types::v0_3::RewardMerkleTreeV1 as jf_merkle_tree_compat::MerkleTreeScheme>::ARITY
        },
    >;
    type RewardStatePathV2 = InternalMerkleProof<
        InternalRewardAmount,
        RewardAccountV2,
        KeccakNode,
        {
            <espresso_types::v0_4::RewardMerkleTreeV2 as jf_merkle_tree_compat::MerkleTreeScheme>::ARITY
        },
    >;

    async fn get_reward_state_height(&self) -> anyhow::Result<u64> {
        let ds = &*self.data_source;
        ds.get_last_state_height()
            .await
            .map(|h| h as u64)
            .map_err(classify_query_error)
    }

    async fn get_reward_state_v2_height(&self) -> anyhow::Result<u64> {
        // `last_merklized_state_height` is the same row for every merklized-state module in
        // this file (reward V1/V2, block-state, fee-state), not just these two.
        self.get_reward_state_height().await
    }

    async fn get_reward_account_proof_v1(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV1> {
        let account: RewardAccountV1 = address
            .parse()
            .map_err(|_| bad_request(format!("invalid ethereum address: {}", address)))?;

        self.data_source
            .load_v1_reward_account_proof(height, account)
            .await
            .map_err(|err| {
                not_found(format!(
                    "failed to load v1 reward account {} at height {}: {}",
                    address, height, err
                ))
            })
    }

    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardClaimInput> {
        // Parse the Ethereum address
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| bad_request(format!("invalid ethereum address: {}", address)))?;

        // Load the reward account proof from the data source
        let proof = self
            .data_source
            .load_reward_account_proof_v2(block_height, addr.into())
            .await
            .map_err(|err| {
                not_found(format!(
                    "failed to load reward account {} at height {}: {}",
                    address, block_height, err
                ))
            })?;

        // Convert the proof to reward claim input (internal type)
        let claim_input = proof.to_reward_claim_input().map_err(|err| match err {
            RewardClaimError::ZeroRewardError => not_found(format!(
                "zero reward balance for {} at height {}",
                address, block_height
            )),
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
            .map_err(|_| bad_request(format!("invalid ethereum address: {}", address)))?;

        // Load the reward account proof from the data source
        let proof = self
            .data_source
            .load_reward_account_proof_v2(height, addr.into())
            .await
            .map_err(|err| {
                not_found(format!(
                    "failed to load reward account {} at height {}: {}",
                    address, height, err
                ))
            })?;

        Ok(InternalRewardAmount(proof.balance))
    }

    async fn get_latest_reward_balance(
        &self,
        address: String,
    ) -> anyhow::Result<Self::RewardBalance> {
        let addr: alloy::primitives::Address = address
            .parse()
            .map_err(|_| bad_request(format!("invalid ethereum address: {}", address)))?;

        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                not_found(format!(
                    "failed to load latest reward account {}: {}",
                    address, err
                ))
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
            .map_err(|_| bad_request(format!("invalid ethereum address: {}", address)))?;

        // Load and return the reward account proof directly (internal type)
        let proof = self
            .data_source
            .load_reward_account_proof_v2(height, addr.into())
            .await
            .map_err(|err| {
                not_found(format!(
                    "failed to load reward account {} at height {}: {}",
                    address, height, err
                ))
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
            .map_err(|_| bad_request(format!("invalid ethereum address: {}", address)))?;

        let proof = self
            .data_source
            .load_latest_reward_account_proof_v2(addr.into())
            .await
            .map_err(|err| {
                not_found(format!(
                    "failed to load latest reward account {}: {}",
                    address, err
                ))
            })?;

        Ok(proof)
    }

    async fn get_reward_amounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::RewardAmounts> {
        if limit > 10000 {
            return Err(bad_request(format!(
                "limit {} exceeds maximum allowed value of 10000",
                limit
            )));
        }

        let tree_bytes = self.data_source.load_tree(height).await.map_err(|err| {
            not_found(format!(
                "failed to load reward tree at height {}: {}",
                height, err
            ))
        })?;

        let tree_data: InternalRewardTreeData =
            bincode::deserialize(&tree_bytes).map_err(|err| {
                not_found(format!(
                    "failed to deserialize RewardMerkleTreeV2Data at height {}: {}",
                    height, err
                ))
            })?;

        let offset_usize = offset as usize;
        let limit_usize = limit as usize;

        if offset_usize > tree_data.balances.len() {
            return Err(not_found(format!("offset {} out of bounds", offset)));
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
            not_found(format!(
                "failed to load reward tree at height {}: {}",
                height, err
            ))
        })
    }

    async fn get_reward_state_path_v1(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Self::RewardStatePathV1> {
        let hs_snapshot = match snapshot {
            v1::Snapshot::Height(h) => HsSnapshot::Index(h),
            v1::Snapshot::Commit(c) => {
                let tb64: TaggedBase64 = c
                    .parse()
                    .map_err(|_| bad_request("failed to parse commit param"))?;
                let commit = (&tb64)
                    .try_into()
                    .map_err(|_| bad_request("failed to parse commit param"))?;
                HsSnapshot::Commit(commit)
            },
        };
        let key: RewardAccountV1 = key
            .parse()
            .map_err(|_| bad_request("failed to parse Key param"))?;
        let ds = &*self.data_source;
        MerklizedStateDataSource::<SeqTypes, RewardMerkleTreeV1, _>::get_path(ds, hs_snapshot, key)
            .await
            .map_err(classify_query_error)
    }

    async fn get_reward_state_path_v2(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Self::RewardStatePathV2> {
        let hs_snapshot = match snapshot {
            v1::Snapshot::Height(h) => HsSnapshot::Index(h),
            v1::Snapshot::Commit(c) => {
                let tb64: TaggedBase64 = c
                    .parse()
                    .map_err(|_| bad_request("failed to parse commit param"))?;
                let commit = (&tb64)
                    .try_into()
                    .map_err(|_| bad_request("failed to parse commit param"))?;
                HsSnapshot::Commit(commit)
            },
        };
        let key: RewardAccountV2 = key
            .parse()
            .map_err(|_| bad_request("failed to parse Key param"))?;
        let ds = &*self.data_source;
        MerklizedStateDataSource::<SeqTypes, RewardMerkleTreeV2, _>::get_path(ds, hs_snapshot, key)
            .await
            .map_err(classify_query_error)
    }
}

#[async_trait]
impl<D> v1::AvailabilityApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    // No `RewardMerkleTreeDataSource` bound here: unlike `v1::RewardApi`, none of these methods
    // touch the reward merkle tree, so filesystem storage (which doesn't implement it) can serve
    // this module too.
    D::Target: hotshot_query_service::availability::AvailabilityDataSource<SeqTypes>
        + hotshot_query_service::node::NodeDataSource<SeqTypes>
        + RequestResponseDataSource<SeqTypes>
        + StateCertDataSource
        + StateCertFetchingDataSource<SeqTypes>
        + Send
        + Sync,
{
    type NamespaceProofQueryData = espresso_types::NamespaceProofQueryData;
    type IncorrectEncodingProof = espresso_types::v0_3::AvidMIncorrectEncodingNsProof;
    type StateCertQueryDataV1 = espresso_types::StateCertQueryDataV1<SeqTypes>;
    type StateCertQueryDataV2 = espresso_types::StateCertQueryDataV2<SeqTypes>;

    async fn get_namespace_proof(
        &self,
        block_id: v1::availability::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::NamespaceProofQueryData> {
        let ns_id = NamespaceId::from(namespace);

        // Convert v1 BlockId to hotshot BlockId
        let hs_block_id = match block_id {
            v1::availability::BlockId::Height(h) => HsBlockId::Number(h as usize),
            v1::availability::BlockId::Hash(h) => {
                let hash = h
                    .parse()
                    .map_err(|_| bad_request(format!("invalid block hash: {}", h)))?;
                HsBlockId::Hash(hash)
            },
            v1::availability::BlockId::PayloadHash(h) => {
                let payload_hash = h
                    .parse()
                    .map_err(|_| bad_request(format!("invalid payload hash: {}", h)))?;
                HsBlockId::PayloadHash(payload_hash)
            },
        };

        // Fetch block and VID common data
        let ds = &*self.data_source;
        let timeout = FETCH_TIMEOUT;
        let (block_fetch, vid_fetch) =
            join!(ds.get_block(hs_block_id), ds.get_vid_common(hs_block_id));
        let (block, vid_common) = join!(
            block_fetch.with_timeout(timeout),
            vid_fetch.with_timeout(timeout)
        );

        let block =
            block.ok_or_else(|| not_found(format!("block {} not available", hs_block_id)))?;
        let vid_common = vid_common.ok_or_else(|| {
            not_found(format!(
                "VID common for block {} not available",
                hs_block_id
            ))
        })?;

        // Namespace absent from the block: an empty result, not an error.
        let ns_table = block.payload().ns_table();
        let Some(ns_index) = ns_table.find_ns_id(&ns_id) else {
            return Ok(espresso_types::NamespaceProofQueryData {
                transactions: vec![],
                proof: None,
            });
        };

        // Generate namespace proof
        let Some(proof) = NsProof::new(block.payload(), &ns_index, vid_common.common()) else {
            // Failed to generate proof - namespace exists but proof generation failed
            return Ok(espresso_types::NamespaceProofQueryData {
                transactions: vec![],
                proof: None,
            });
        };

        let transactions = proof.export_all_txs(&ns_id);

        Ok(espresso_types::NamespaceProofQueryData {
            transactions,
            proof: Some(proof),
        })
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
        if range_size > NAMESPACE_PROOF_RANGE_LIMIT {
            return Err(range_exceeded(format!(
                "range too large: {} blocks (max {})",
                range_size, NAMESPACE_PROOF_RANGE_LIMIT
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
        block_id: v1::availability::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Self::IncorrectEncodingProof> {
        let ns_id = NamespaceId::from(namespace);

        let hs_block_id = match block_id {
            v1::availability::BlockId::Height(h) => HsBlockId::Number(h as usize),
            v1::availability::BlockId::Hash(h) => {
                let hash = h
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid block hash: {}", h))?;
                HsBlockId::Hash(hash)
            },
            v1::availability::BlockId::PayloadHash(h) => {
                let payload_hash = h
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid payload hash: {}", h))?;
                HsBlockId::PayloadHash(payload_hash)
            },
        };

        let ds = &*self.data_source;
        let timeout = FETCH_TIMEOUT;
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

        // Block has incorrect encoding: fetch VID shares to construct the proof.
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

/// Check a ranges request against the same per-request object limit the range endpoints enforce,
/// and convert it for the data source.
///
/// Bounding the total heights also bounds how many ranges a request may carry, since every range
/// covers at least one height.
fn validate_ranges(ranges: Vec<Range<u64>>, limit: usize) -> anyhow::Result<Vec<Range<u64>>> {
    let mut total = 0usize;
    for range in &ranges {
        if range.is_empty() {
            return Err(bad_request(format!(
                "empty or inverted range {}..{}",
                range.start, range.end
            )));
        }
        // Heights are bound into the query as i64, so anything past that range cannot be queried
        // and would only be a way to overflow the accounting below.
        if range.end > i64::MAX as u64 {
            return Err(bad_request(format!("height {} out of range", range.end)));
        }

        total = total
            .checked_add((range.end - range.start) as usize)
            .ok_or_else(|| range_exceeded(format!("ranges cover more than {limit} heights")))?;
        if total > limit {
            return Err(range_exceeded(format!(
                "ranges cover more than {limit} heights"
            )));
        }
    }

    // The light client pairs leaves with proofs positionally, so a height must appear once, in
    // order.
    if !ranges.is_sorted_by(|a, b| a.end <= b.start) {
        return Err(bad_request("ranges must be ascending and disjoint"));
    }

    Ok(ranges)
}

// Range limits for list endpoints, read from `hotshot_query_service`'s `Options` (their only
// remaining declaration) so a dependency bump that changes the defaults changes enforcement too.
const NAMESPACE_PROOF_RANGE_LIMIT: u64 = 100;

fn small_object_range_limit() -> usize {
    hotshot_query_service::availability::Options::default().small_object_range_limit
}

fn large_object_range_limit() -> usize {
    hotshot_query_service::availability::Options::default().large_object_range_limit
}

#[async_trait]
impl<D> HotShotAvailabilityApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: AvailabilityDataSource<SeqTypes>
        + hotshot_query_service::data_source::VersionedDataSource
        + Send
        + Sync,
    for<'a> <D::Target as hotshot_query_service::data_source::VersionedDataSource>::ReadOnly<'a>:
        hotshot_query_service::data_source::storage::AvailabilityStorage<SeqTypes>,
{
    type Leaf = LeafQueryData<SeqTypes>;
    type Block = BlockQueryData<SeqTypes>;
    type Header = HsHeader<SeqTypes>;
    type Payload = PayloadQueryData<SeqTypes>;
    type VidCommon = VidCommonQueryData<SeqTypes>;
    type Transaction = TransactionQueryData<SeqTypes>;
    type TransactionWithProof = TransactionWithProofQueryData<SeqTypes>;
    type BlockSummary = BlockSummaryQueryData<SeqTypes>;
    type Limits = HsLimits;
    type Cert2 = Certificate2<SeqTypes>;

    async fn get_leaf(&self, id: v1::availability::LeafId) -> anyhow::Result<Self::Leaf> {
        let hs_id = match id {
            v1::availability::LeafId::Height(h) => HsLeafId::Number(h as usize),
            v1::availability::LeafId::Hash(h) => {
                HsLeafId::Hash(h.parse().map_err(|_| bad_request("invalid leaf hash"))?)
            },
        };
        let ds = &*self.data_source;
        ds.get_leaf(hs_id)
            .await
            .with_timeout(FETCH_TIMEOUT)
            .await
            .ok_or_else(|| not_found("leaf not found"))
    }

    async fn get_leaf_range(&self, from: usize, until: usize) -> anyhow::Result<Vec<Self::Leaf>> {
        enforce_range(from, until, small_object_range_limit())?;
        let timeout = FETCH_TIMEOUT;
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

    async fn get_header(&self, id: v1::availability::BlockId) -> anyhow::Result<Self::Header> {
        let hs_id = block_id_to_hs(id)?;
        let ds = &*self.data_source;
        ds.get_header(hs_id)
            .await
            .with_timeout(FETCH_TIMEOUT)
            .await
            .ok_or_else(|| not_found(format!("header not found for {}", hs_id)))
    }

    async fn get_header_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::Header>> {
        enforce_range(from, until, large_object_range_limit())?;
        let timeout = FETCH_TIMEOUT;
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

    async fn get_block(&self, id: v1::availability::BlockId) -> anyhow::Result<Self::Block> {
        let hs_id = block_id_to_hs(id)?;
        let ds = &*self.data_source;
        ds.get_block(hs_id)
            .await
            .with_timeout(FETCH_TIMEOUT)
            .await
            .ok_or_else(|| not_found(format!("block not found for {}", hs_id)))
    }

    async fn get_block_range(&self, from: usize, until: usize) -> anyhow::Result<Vec<Self::Block>> {
        enforce_range(from, until, large_object_range_limit())?;
        let timeout = FETCH_TIMEOUT;
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

    async fn get_payload(&self, id: v1::availability::PayloadId) -> anyhow::Result<Self::Payload> {
        let hs_id = payload_id_to_hs(id)?;
        let ds = &*self.data_source;
        ds.get_payload(hs_id)
            .await
            .with_timeout(FETCH_TIMEOUT)
            .await
            .ok_or_else(|| not_found(format!("payload not found for {}", hs_id)))
    }

    async fn get_payload_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::Payload>> {
        enforce_range(from, until, large_object_range_limit())?;
        let timeout = FETCH_TIMEOUT;
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
        id: v1::availability::BlockId,
    ) -> anyhow::Result<Self::VidCommon> {
        let hs_id = block_id_to_hs(id)?;
        let ds = &*self.data_source;
        ds.get_vid_common(hs_id)
            .await
            .with_timeout(FETCH_TIMEOUT)
            .await
            .ok_or_else(|| not_found(format!("VID common not found for {}", hs_id)))
    }

    async fn get_vid_common_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::VidCommon>> {
        enforce_range(from, until, small_object_range_limit())?;
        let timeout = FETCH_TIMEOUT;
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

    async fn get_leaf_ranges(&self, ranges: Vec<Range<u64>>) -> anyhow::Result<Vec<Self::Leaf>> {
        let ranges = validate_ranges(ranges, small_object_range_limit())?;

        // One read when every height is present. A miss falls through to the range endpoints,
        // for their window per height and 404 at the first one missing.
        let heights: u64 = ranges.iter().map(|range| range.end - range.start).sum();
        if let Ok(mut tx) = self.data_source.read().await
            && let Ok(leaves) = tx.get_leaf_ranges(&ranges).await
            && leaves.len() as u64 == heights
        {
            return Ok(leaves);
        }

        let ranges: Vec<_> = futures::stream::iter(ranges)
            .map(|range| self.get_leaf_range(range.start as usize, range.end as usize))
            .buffered(self.ranges_concurrency.get())
            .try_collect()
            .await?;
        Ok(ranges.into_iter().flatten().collect())
    }

    async fn get_block_ranges(&self, ranges: Vec<Range<u64>>) -> anyhow::Result<Vec<Self::Block>> {
        let ranges = validate_ranges(ranges, large_object_range_limit())?;

        let heights: u64 = ranges.iter().map(|range| range.end - range.start).sum();
        if let Ok(mut tx) = self.data_source.read().await
            && let Ok(blocks) = tx.get_block_ranges(&ranges).await
            && blocks.len() as u64 == heights
        {
            return Ok(blocks);
        }

        let ranges: Vec<_> = futures::stream::iter(ranges)
            .map(|range| self.get_block_range(range.start as usize, range.end as usize))
            .buffered(self.ranges_concurrency.get())
            .try_collect()
            .await?;
        Ok(ranges.into_iter().flatten().collect())
    }

    async fn get_vid_common_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> anyhow::Result<Vec<Self::VidCommon>> {
        let ranges = validate_ranges(ranges, small_object_range_limit())?;

        let heights: u64 = ranges.iter().map(|range| range.end - range.start).sum();
        if let Ok(mut tx) = self.data_source.read().await
            && let Ok(common) = tx.get_vid_common_ranges(&ranges).await
            && common.len() as u64 == heights
        {
            return Ok(common);
        }

        let ranges: Vec<_> = futures::stream::iter(ranges)
            .map(|range| self.get_vid_common_range(range.start as usize, range.end as usize))
            .buffered(self.ranges_concurrency.get())
            .try_collect()
            .await?;
        Ok(ranges.into_iter().flatten().collect())
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
            .with_timeout(FETCH_TIMEOUT)
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
        let tx_hash: hotshot_query_service::availability::TransactionHash<SeqTypes> = hash
            .parse()
            .map_err(|_| bad_request(format!("invalid transaction hash: {}", hash)))?;
        let bwt = ds
            .get_block_containing_transaction(tx_hash)
            .await
            .with_timeout(FETCH_TIMEOUT)
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
        let timeout = FETCH_TIMEOUT;

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
        let timeout = FETCH_TIMEOUT;

        let tx_hash: hotshot_query_service::availability::TransactionHash<SeqTypes> = hash
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
            .with_timeout(FETCH_TIMEOUT)
            .await
            .ok_or_else(|| not_found(format!("block {} not found", height)))?;
        Ok(BlockSummaryQueryData::from(block))
    }

    async fn get_block_summary_range(
        &self,
        from: usize,
        until: usize,
    ) -> anyhow::Result<Vec<Self::BlockSummary>> {
        enforce_range(from, until, large_object_range_limit())?;
        let timeout = FETCH_TIMEOUT;
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
            small_object_range_limit: small_object_range_limit(),
            large_object_range_limit: large_object_range_limit(),
        })
    }

    async fn get_cert2(&self, height: u64) -> anyhow::Result<Option<Self::Cert2>> {
        Ok(self
            .data_source
            .get_cert2(height)
            .await
            .with_timeout(FETCH_TIMEOUT)
            .await)
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

fn block_id_to_hs(id: v1::availability::BlockId) -> anyhow::Result<HsBlockId<SeqTypes>> {
    match id {
        v1::availability::BlockId::Height(h) => Ok(HsBlockId::Number(h as usize)),
        v1::availability::BlockId::Hash(h) => {
            let hash = h
                .parse()
                .map_err(|_| bad_request(format!("invalid block hash: {}", h)))?;
            Ok(HsBlockId::Hash(hash))
        },
        v1::availability::BlockId::PayloadHash(h) => {
            let payload_hash = h
                .parse()
                .map_err(|_| bad_request(format!("invalid payload hash: {}", h)))?;
            Ok(HsBlockId::PayloadHash(payload_hash))
        },
    }
}

fn payload_id_to_hs(id: v1::availability::PayloadId) -> anyhow::Result<HsBlockId<SeqTypes>> {
    match id {
        v1::availability::PayloadId::Height(h) => Ok(HsBlockId::Number(h as usize)),
        v1::availability::PayloadId::Hash(h) => {
            let payload_hash = h
                .parse()
                .map_err(|_| bad_request(format!("invalid payload hash: {}", h)))?;
            Ok(HsBlockId::PayloadHash(payload_hash))
        },
        v1::availability::PayloadId::BlockHash(h) => {
            let hash = h
                .parse()
                .map_err(|_| bad_request(format!("invalid block hash: {}", h)))?;
            Ok(HsBlockId::Hash(hash))
        },
    }
}

fn classify_query_error(err: hotshot_query_service::QueryError) -> anyhow::Error {
    match err {
        QueryError::NotFound | QueryError::Missing => not_found(err.to_string()),
        QueryError::Error { .. } => anyhow::anyhow!(err.to_string()),
    }
}

#[async_trait]
impl<D> v1::BlockStateApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::merklized_state::MerklizedStateDataSource<
            SeqTypes,
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
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Self::MerkleProof> {
        let hs_snapshot = match snapshot {
            v1::Snapshot::Height(h) => HsSnapshot::Index(h),
            v1::Snapshot::Commit(c) => {
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
        MerklizedStateDataSource::<SeqTypes, espresso_types::BlockMerkleTree, _>::get_path(
            ds,
            hs_snapshot,
            key,
        )
        .await
        .map_err(classify_query_error)
    }

    async fn get_block_state_height(&self) -> anyhow::Result<u64> {
        let ds = &*self.data_source;
        ds.get_last_state_height()
            .await
            .map(|h| h as u64)
            .map_err(classify_query_error)
    }
}

#[async_trait]
impl<D> v1::FeeStateApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::merklized_state::MerklizedStateDataSource<
            SeqTypes,
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
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Self::MerkleProof> {
        let hs_snapshot = match snapshot {
            v1::Snapshot::Height(h) => HsSnapshot::Index(h),
            v1::Snapshot::Commit(c) => {
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
        MerklizedStateDataSource::<SeqTypes, espresso_types::FeeMerkleTree, _>::get_path(
            ds,
            hs_snapshot,
            key,
        )
        .await
        .map_err(classify_query_error)
    }

    async fn get_fee_state_height(&self) -> anyhow::Result<u64> {
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
        > = MerklizedStateDataSource::<SeqTypes, espresso_types::FeeMerkleTree, _>::get_path(
            ds,
            HsSnapshot::Index(height as u64),
            key,
        )
        .await
        .map_err(classify_query_error)?;
        Ok(path.elem().copied())
    }
}

#[async_trait]
impl<D> v1::StatusApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::status::StatusDataSource + NodeKeysDataSource + Send + Sync,
{
    type Keys = NodePublicKeys;

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
        let ds = &*self.data_source;
        // Standard prometheus text exposition of the registry.
        let mut buffer = Vec::new();
        prometheus::TextEncoder::new().encode(&ds.metrics().registry().gather(), &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    async fn keys(&self) -> anyhow::Result<NodePublicKeys> {
        self.data_source
            .node_public_keys()
            .await
            .ok_or_else(|| not_found("this node has no validator keys"))
    }
}

#[tonic::async_trait]
impl<D> proto::status_service_server::StatusService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::status::StatusDataSource + NodeKeysDataSource + Send + Sync,
{
    async fn get_block_height(
        &self,
        _request: tonic::Request<proto::GetBlockHeightRequest>,
    ) -> Result<tonic::Response<proto::BlockHeightResponse>, tonic::Status> {
        let height = <Self as v1::StatusApi>::block_height(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockHeightResponse { height }))
    }

    async fn get_success_rate(
        &self,
        _request: tonic::Request<proto::GetSuccessRateRequest>,
    ) -> Result<tonic::Response<proto::SuccessRateResponse>, tonic::Status> {
        let rate = <Self as v1::StatusApi>::success_rate(self)
            .await
            .map_err(to_status)?;
        // A fresh node computes 0/0 and a restarted one height/0 until its first view tick
        // (the view gauge is in-memory, the height persisted). protoJSON cannot encode a
        // non-finite double and the generated deserializer rejects `null`, so clamp to zero.
        let rate = if rate.is_finite() { rate } else { 0. };
        Ok(tonic::Response::new(proto::SuccessRateResponse { rate }))
    }

    async fn get_time_since_last_decide(
        &self,
        _request: tonic::Request<proto::GetTimeSinceLastDecideRequest>,
    ) -> Result<tonic::Response<proto::TimeSinceLastDecideResponse>, tonic::Status> {
        let seconds = <Self as v1::StatusApi>::time_since_last_decide(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TimeSinceLastDecideResponse {
            seconds,
        }))
    }

    async fn get_node_keys(
        &self,
        _request: tonic::Request<proto::GetNodeKeysRequest>,
    ) -> Result<tonic::Response<proto::NodeKeysResponse>, tonic::Status> {
        let keys = <Self as v1::StatusApi>::keys(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::NodeKeysResponse {
            eth_account: keys.eth_account.map(|account| format!("{account:#x}")),
            consensus_key: Some(proto::BlsPublicKey {
                key: keys.consensus_key.to_string(),
            }),
            state_ver_key: Some(proto::SchnorrPublicKey {
                key: keys.state_ver_key.to_string(),
            }),
            x25519_key: keys.x25519_key.as_ref().map(ToString::to_string),
            p2p_addr: keys.p2p_addr.as_ref().map(ToString::to_string),
        }))
    }
}

#[async_trait]
impl<D> v1::ConfigApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: HotShotConfigDataSource + Send + Sync,
{
    type HotShotConfig = espresso_types::config::PublicNetworkConfig;
    type RuntimeConfig = crate::options::PublicNodeConfig;

    async fn hotshot_config(&self) -> anyhow::Result<Self::HotShotConfig> {
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

#[tonic::async_trait]
impl<D> proto::config_service_server::ConfigService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: HotShotConfigDataSource + Send + Sync,
{
    async fn get_hotshot_config(
        &self,
        _request: tonic::Request<proto::GetHotshotConfigRequest>,
    ) -> Result<tonic::Response<proto::HotshotConfigResponse>, tonic::Status> {
        let config = <Self as v1::ConfigApi>::hotshot_config(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(config.into()))
    }

    async fn get_env(
        &self,
        _request: tonic::Request<proto::GetEnvRequest>,
    ) -> Result<tonic::Response<proto::EnvResponse>, tonic::Status> {
        let variables = <Self as v1::ConfigApi>::env(self)
            .await
            .map_err(to_status)?
            .into_iter()
            .map(|entry| {
                let (name, value) = entry
                    .split_once('=')
                    .expect("ConfigApi::env yields KEY=value entries");
                proto::EnvVar {
                    name: name.to_string(),
                    value: value.to_string(),
                }
            })
            .collect();
        Ok(tonic::Response::new(proto::EnvResponse { variables }))
    }

    async fn get_runtime_config(
        &self,
        _request: tonic::Request<proto::GetRuntimeConfigRequest>,
    ) -> Result<tonic::Response<proto::RuntimeConfigResponse>, tonic::Status> {
        let config = <Self as v1::ConfigApi>::runtime_config(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(config.into()))
    }
}

#[async_trait]
impl<D> v1::NodeApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::node::NodeDataSource<SeqTypes>
        + StakeTableDataSource<SeqTypes>
        + PruningDataSource
        + Send
        + Sync,
{
    type VidShare = hotshot_types::data::VidShare;
    type SyncStatus = hotshot_query_service::node::SyncStatusQueryData;
    type HeaderWindow =
        hotshot_query_service::node::TimeWindowQueryData<hotshot_query_service::Header<SeqTypes>>;
    type Limits = hotshot_query_service::node::Limits;
    type StakeTable = Vec<hotshot_types::PeerConfig<SeqTypes>>;
    type StakeTableCurrent = StakeTableWithEpochNumber<SeqTypes>;
    type Validators = indexmap::IndexMap<
        alloy::primitives::Address,
        espresso_types::v0_3::AuthenticatedValidator<espresso_types::PubKey>,
    >;
    type AllValidators = Vec<espresso_types::v0_3::RegisteredValidator<espresso_types::PubKey>>;
    type Participation = std::collections::HashMap<espresso_types::PubKey, f64>;
    type BlockReward = Option<espresso_types::v0_3::RewardAmount>;
    type Block = hotshot_query_service::availability::BlockQueryData<SeqTypes>;
    type Leaf = hotshot_query_service::availability::LeafQueryData<SeqTypes>;

    async fn block_height(&self) -> anyhow::Result<u64> {
        let ds = &*self.data_source;
        let h = hotshot_query_service::node::NodeDataSource::block_height(ds)
            .await
            .map_err(classify_query_error)?;
        Ok(h as u64)
    }

    async fn count_transactions(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u64>,
    ) -> anyhow::Result<u64> {
        let ds = &*self.data_source;
        let from = match from {
            Some(f) => Bound::Included(f as usize),
            None => Bound::Unbounded,
        };
        let to = match to {
            Some(t) => Bound::Included(t as usize),
            None => Bound::Unbounded,
        };
        let ns = namespace.map(espresso_types::NamespaceId::from);
        let count = ds
            .count_transactions_in_range((from, to), ns)
            .await
            .map_err(classify_query_error)?;
        Ok(count as u64)
    }

    async fn payload_size(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u64>,
    ) -> anyhow::Result<u64> {
        let ds = &*self.data_source;
        let from = match from {
            Some(f) => Bound::Included(f as usize),
            None => Bound::Unbounded,
        };
        let to = match to {
            Some(t) => Bound::Included(t as usize),
            None => Bound::Unbounded,
        };
        let ns = namespace.map(espresso_types::NamespaceId::from);
        let size = ds
            .payload_size_in_range((from, to), ns)
            .await
            .map_err(classify_query_error)?;
        Ok(size as u64)
    }

    async fn get_vid_share(&self, id: v1::VidShareId) -> anyhow::Result<Self::VidShare> {
        let ds = &*self.data_source;
        let node_id: HsBlockId<SeqTypes> = match id {
            v1::VidShareId::Height(h) => HsBlockId::Number(h as usize),
            v1::VidShareId::Hash(h) => HsBlockId::Hash(
                h.parse()
                    .map_err(|_| bad_request(format!("invalid block hash: {h}")))?,
            ),
            v1::VidShareId::PayloadHash(h) => HsBlockId::PayloadHash(
                h.parse()
                    .map_err(|_| bad_request(format!("invalid payload hash: {h}")))?,
            ),
        };
        hotshot_query_service::node::NodeDataSource::vid_share(ds, node_id)
            .await
            .map_err(classify_query_error)
    }

    async fn sync_status(&self) -> anyhow::Result<Self::SyncStatus> {
        let ds = &*self.data_source;
        hotshot_query_service::node::NodeDataSource::sync_status(ds)
            .await
            .map_err(classify_query_error)
    }

    async fn get_header_window(
        &self,
        start: v1::HeaderWindowStart,
        end: u64,
    ) -> anyhow::Result<Self::HeaderWindow> {
        let ds = &*self.data_source;
        let start: WindowStart<SeqTypes> = match start {
            v1::HeaderWindowStart::Time(t) => WindowStart::Time(t),
            v1::HeaderWindowStart::Height(h) => WindowStart::Height(h),
            v1::HeaderWindowStart::Hash(h) => WindowStart::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?,
            ),
        };
        ds.get_header_window(start, end, node_window_limit())
            .await
            .map_err(classify_query_error)
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
            return Err(bad_request("Limit cannot be greater than 1000"));
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
        let ds = &*self.data_source;
        ds.get_oldest_block().await
    }

    async fn get_oldest_leaf(&self) -> anyhow::Result<Option<Self::Leaf>> {
        let ds = &*self.data_source;
        ds.get_oldest_leaf().await
    }
}

#[tonic::async_trait]
impl<D> proto::node_service_server::NodeService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::node::NodeDataSource<SeqTypes>
        + StakeTableDataSource<SeqTypes>
        + PruningDataSource
        + Send
        + Sync,
{
    async fn get_transaction_count(
        &self,
        request: tonic::Request<proto::GetTransactionCountRequest>,
    ) -> Result<tonic::Response<proto::TransactionCountResponse>, tonic::Status> {
        let proto::GetTransactionCountRequest {
            from,
            to,
            namespace,
        } = request.into_inner();
        let count = <Self as v1::NodeApi>::count_transactions(self, from, to, namespace)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TransactionCountResponse {
            count,
        }))
    }

    async fn get_payload_size(
        &self,
        request: tonic::Request<proto::GetPayloadSizeRequest>,
    ) -> Result<tonic::Response<proto::PayloadSizeResponse>, tonic::Status> {
        let proto::GetPayloadSizeRequest {
            from,
            to,
            namespace,
        } = request.into_inner();
        let size = <Self as v1::NodeApi>::payload_size(self, from, to, namespace)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::PayloadSizeResponse { size }))
    }

    async fn get_sync_status(
        &self,
        _request: tonic::Request<proto::GetSyncStatusRequest>,
    ) -> Result<tonic::Response<proto::SyncStatusResponse>, tonic::Status> {
        let status = <Self as v1::NodeApi>::sync_status(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::SyncStatusResponse::from(
            status,
        )))
    }

    async fn get_block_reward(
        &self,
        request: tonic::Request<proto::GetBlockRewardRequest>,
    ) -> Result<tonic::Response<proto::BlockRewardResponse>, tonic::Status> {
        let reward = <Self as v1::NodeApi>::get_block_reward(self, request.into_inner().epoch)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockRewardResponse {
            amount: reward.map(|amount| amount.to_string()),
        }))
    }

    async fn get_vid_share(
        &self,
        request: tonic::Request<proto::GetVidShareRequest>,
    ) -> Result<tonic::Response<proto::VidShareResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash, request.payload_hash) {
            (Some(height), None, None) => v1::VidShareId::Height(height),
            (None, Some(hash), None) => v1::VidShareId::Hash(hash),
            (None, None, Some(hash)) => v1::VidShareId::PayloadHash(hash),
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height, hash or payload_hash",
                ));
            },
        };
        let share = <Self as v1::NodeApi>::get_vid_share(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::VidShareResponse::try_from(
            &share,
        )?))
    }

    async fn get_header_window(
        &self,
        request: tonic::Request<proto::GetHeaderWindowRequest>,
    ) -> Result<tonic::Response<proto::HeaderWindowResponse>, tonic::Status> {
        let request = request.into_inner();
        let start = match (request.start_time, request.start_height, request.start_hash) {
            (Some(time), None, None) => v1::HeaderWindowStart::Time(time),
            (None, Some(height), None) => v1::HeaderWindowStart::Height(height),
            (None, None, Some(hash)) => v1::HeaderWindowStart::Hash(hash),
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of start_time, start_height or start_hash",
                ));
            },
        };
        let end = required(request.end, "end")?;
        let window = <Self as v1::NodeApi>::get_header_window(self, start, end)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::HeaderWindowResponse::from(
            &window,
        )))
    }

    async fn get_node_block_height(
        &self,
        _request: tonic::Request<proto::GetNodeBlockHeightRequest>,
    ) -> Result<tonic::Response<proto::NodeBlockHeightResponse>, tonic::Status> {
        let height = <Self as v1::NodeApi>::block_height(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::NodeBlockHeightResponse {
            height,
        }))
    }

    async fn get_node_limits(
        &self,
        _request: tonic::Request<proto::GetNodeLimitsRequest>,
    ) -> Result<tonic::Response<proto::NodeLimitsResponse>, tonic::Status> {
        let limits = <Self as v1::NodeApi>::limits(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::NodeLimitsResponse::from(
            limits,
        )))
    }

    async fn get_stake_table(
        &self,
        request: tonic::Request<proto::GetStakeTableRequest>,
    ) -> Result<tonic::Response<proto::StakeTableResponse>, tonic::Status> {
        let table = match request.into_inner().epoch {
            Some(epoch) => StakeTableWithEpochNumber {
                epoch: Some(EpochNumber::new(epoch)),
                stake_table: <Self as v1::NodeApi>::stake_table(self, epoch)
                    .await
                    .map_err(to_status)?,
            },
            None => <Self as v1::NodeApi>::stake_table_current(self)
                .await
                .map_err(to_status)?,
        };
        Ok(tonic::Response::new(table.into()))
    }

    async fn get_validators(
        &self,
        request: tonic::Request<proto::GetValidatorsRequest>,
    ) -> Result<tonic::Response<proto::ValidatorsResponse>, tonic::Status> {
        let epoch = required(request.into_inner().epoch, "epoch")?;
        let validators = <Self as v1::NodeApi>::get_validators(self, epoch)
            .await
            .map_err(to_status)?;
        let mut validators: Vec<proto::Validator> = validators
            .into_values()
            .map(|authenticated| authenticated.into_inner().into())
            .collect();
        // v1 serves a map, so the order is its own; the paged route reports account order.
        validators.sort_by(|a, b| a.account.cmp(&b.account));
        Ok(tonic::Response::new(proto::ValidatorsResponse {
            validators,
        }))
    }

    async fn get_all_validators(
        &self,
        request: tonic::Request<proto::GetAllValidatorsRequest>,
    ) -> Result<tonic::Response<proto::ValidatorsResponse>, tonic::Status> {
        let request = request.into_inner();
        let validators = <Self as v1::NodeApi>::get_all_validators(
            self,
            required(request.epoch, "epoch")?,
            required(request.offset, "offset")?,
            required(request.limit, "limit")?,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::ValidatorsResponse {
            validators: validators.into_iter().map(Into::into).collect(),
        }))
    }

    async fn get_proposal_participation(
        &self,
        request: tonic::Request<proto::GetProposalParticipationRequest>,
    ) -> Result<tonic::Response<proto::ParticipationResponse>, tonic::Status> {
        let fractions = match request.into_inner().epoch {
            Some(epoch) => <Self as v1::NodeApi>::proposal_participation(self, epoch).await,
            None => <Self as v1::NodeApi>::current_proposal_participation(self).await,
        }
        .map_err(to_status)?;
        Ok(tonic::Response::new(fractions.into()))
    }

    async fn get_vote_participation(
        &self,
        request: tonic::Request<proto::GetVoteParticipationRequest>,
    ) -> Result<tonic::Response<proto::ParticipationResponse>, tonic::Status> {
        let fractions = match request.into_inner().epoch {
            Some(epoch) => <Self as v1::NodeApi>::vote_participation(self, epoch).await,
            None => <Self as v1::NodeApi>::current_vote_participation(self).await,
        }
        .map_err(to_status)?;
        Ok(tonic::Response::new(fractions.into()))
    }
}

// These stay here rather than in the api crate's `render` for the same reason as the stake table
// below: the source types are this crate's, and the api crate cannot depend on this one.

impl From<crate::options::PublicNodeConfig> for proto::RuntimeConfigResponse {
    fn from(config: crate::options::PublicNodeConfig) -> Self {
        // Destructured without `..` so that a new field fails to compile here instead of becoming
        // a setting v2 silently never serves; the `_` bindings are the deliberate drops. The guard
        // stops at this level, as the storage and module structs below are read field by field.
        let crate::options::PublicNodeConfig {
            orchestrator_url,
            cdn_endpoint,
            cliquenet_bind_address,
            cliquenet_advertise_address,
            libp2p_bind_address,
            libp2p_advertise_address,
            libp2p_bootstrap_nodes,
            public_api_url,
            builder_urls,
            state_relay_server_url,
            state_peers,
            config_peers,
            is_da,
            genesis_file,
            genesis: _,
            identity,
            catchup_base_timeout: _,
            local_catchup_timeout: _,
            bootstrap_epoch_catchup_timeout: _,
            catchup_backoff: _,
            proposal_fetcher: _,
            libp2p: _,
            l1: _,
            l1_provider_count,
            l1_ws_provider_count,
            storage,
            modules,
        } = config;
        let crate::options::Identity {
            node_name,
            node_description,
            company_name,
            company_website,
            country_code,
            latitude,
            longitude,
            operating_system,
            node_type,
            network_type,
            icon_14x14_1x,
            icon_14x14_2x,
            icon_14x14_3x,
            icon_24x24_1x,
            icon_24x24_2x,
            icon_24x24_3x,
        } = identity;
        Self {
            is_da,
            identity: Some(proto::NodeIdentity {
                node_name,
                node_description,
                company_name,
                company_website: company_website.map(|url| url.to_string()),
                country_code,
                latitude,
                longitude,
                operating_system,
                node_type,
                network_type,
                icon_14x14_1x: icon_14x14_1x.map(|url| url.to_string()),
                icon_14x14_2x: icon_14x14_2x.map(|url| url.to_string()),
                icon_14x14_3x: icon_14x14_3x.map(|url| url.to_string()),
                icon_24x24_1x: icon_24x24_1x.map(|url| url.to_string()),
                icon_24x24_2x: icon_24x24_2x.map(|url| url.to_string()),
                icon_24x24_3x: icon_24x24_3x.map(|url| url.to_string()),
            }),
            storage: Some(storage.into()),
            genesis_file: genesis_file.to_string(),
            public_api_url: public_api_url.map(|url| url.to_string()),
            builder_urls: builder_urls.iter().map(ToString::to_string).collect(),
            state_relay_server_url: state_relay_server_url.to_string(),
            state_peers: state_peers.iter().map(ToString::to_string).collect(),
            config_peers: config_peers
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect(),
            orchestrator_url: orchestrator_url.to_string(),
            cdn_endpoint,
            // `unbracketed_string` rather than `to_string`: NetAddr's Display brackets an IPv6
            // literal and its serde impl does not, so v1 serves the unbracketed form.
            cliquenet_bind_address: cliquenet_bind_address.unbracketed_string(),
            cliquenet_advertise_address: cliquenet_advertise_address
                .map(|addr| addr.unbracketed_string()),
            libp2p_bind_address,
            libp2p_advertise_address,
            libp2p_bootstrap_nodes: libp2p_bootstrap_nodes
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect(),
            l1_provider_count: l1_provider_count as u64,
            l1_ws_provider_count: l1_ws_provider_count as u64,
            modules: Some(modules.into()),
        }
    }
}

impl From<crate::options::StorageConfig> for proto::NodeStorage {
    fn from(storage: crate::options::StorageConfig) -> Self {
        Self {
            backend: match storage.backend {
                crate::options::StorageBackend::Sql => proto::StorageBackend::Sql,
                crate::options::StorageBackend::Fs => proto::StorageBackend::Fs,
                crate::options::StorageBackend::FsDefault => proto::StorageBackend::FsDefault,
            }
            .into(),
            fs: storage.fs.map(|fs| proto::FsStorage {
                path: fs.path.display().to_string(),
                consensus_view_retention: fs.consensus_view_retention,
            }),
            sql: storage.sql.map(Into::into),
        }
    }
}

impl From<crate::options::SqlStorageConfig> for proto::SqlStorage {
    fn from(sql: crate::options::SqlStorageConfig) -> Self {
        let millis = |duration: Duration| duration.as_millis() as u64;
        Self {
            prune: sql.prune,
            archive: sql.archive,
            lightweight: sql.lightweight,
            disable_proactive_fetching: sql.disable_proactive_fetching,
            fetch_rate_limit: sql.fetch_rate_limit.map(|limit| limit as u64),
            active_fetch_delay_ms: sql.active_fetch_delay.map(millis),
            chunk_fetch_delay_ms: sql.chunk_fetch_delay.map(millis),
            sync_status_chunk_size: sql.sync_status_chunk_size.map(|size| size as u64),
            sync_status_ttl_ms: sql.sync_status_ttl.map(millis),
            proactive_scan_chunk_size: sql.proactive_scan_chunk_size.map(|size| size as u64),
            proactive_scan_interval_ms: sql.proactive_scan_interval.map(millis),
            idle_connection_timeout_ms: millis(sql.idle_connection_timeout),
            connection_timeout_ms: millis(sql.connection_timeout),
            slow_statement_threshold_ms: millis(sql.slow_statement_threshold),
            statement_timeout_ms: millis(sql.statement_timeout),
            min_connections: sql.min_connections,
            max_connections: sql.max_connections,
            query_min_connections: sql.query_min_connections,
            query_max_connections: sql.query_max_connections,
            pruning: Some(proto::PruningConfig {
                pruning_threshold: sql.pruning.pruning_threshold,
                minimum_retention_ms: sql.pruning.minimum_retention.map(millis),
                target_retention_ms: sql.pruning.target_retention.map(millis),
                batch_size: sql.pruning.batch_size,
                max_usage: sql.pruning.max_usage.map(u32::from),
                interval_ms: sql.pruning.interval.map(millis),
                pages: sql.pruning.pages,
            }),
            consensus_pruning: Some(proto::ConsensusPruningConfig {
                target_retention: sql.consensus_pruning.target_retention,
                minimum_retention: sql.consensus_pruning.minimum_retention,
                target_usage: sql.consensus_pruning.target_usage,
            }),
        }
    }
}

impl From<crate::options::ApiModulesConfig> for proto::ApiModules {
    fn from(modules: crate::options::ApiModulesConfig) -> Self {
        Self {
            http: modules.http.map(|http| proto::HttpModule {
                port: http.port.into(),
                max_connections: http.max_connections.map(|max| max as u64),
                tonic_port: http.tonic_port.map(u32::from),
            }),
            query: modules.query.map(|query| proto::QueryModule {
                peers: query.peers.iter().map(ToString::to_string).collect(),
                light_client: Some(proto::LightClientModuleOptions {
                    num_stake_tables_in_memory: query.light_client.num_stake_tables_in_memory
                        as u64,
                }),
                light_client_db: Some(proto::LightClientDbOptions {
                    num_connections: query.light_client_db.num_connections,
                    num_leaves: query.light_client_db.num_leaves,
                    num_stake_tables: query.light_client_db.num_stake_tables,
                    lc_path: query
                        .light_client_db
                        .lc_path
                        .map(|path| path.display().to_string()),
                }),
            }),
            submit: modules.submit,
            status: modules.status,
            catchup: modules.catchup,
            config: modules.config,
            hotshot_events: modules.hotshot_events,
            explorer: modules.explorer,
            light_client: modules.light_client,
        }
    }
}

// Stays here rather than in the api crate's `render`: the source type is this crate's, and the
// api crate cannot depend on this one.
impl From<StakeTableWithEpochNumber<SeqTypes>> for proto::StakeTableResponse {
    fn from(table: StakeTableWithEpochNumber<SeqTypes>) -> Self {
        Self {
            epoch: table.epoch.map(|epoch| *epoch),
            stake_table: table.stake_table.into_iter().map(Into::into).collect(),
        }
    }
}

fn node_window_limit() -> usize {
    hotshot_query_service::node::Options::default().window_limit
}

#[async_trait]
impl<D> v1::CatchupApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: CatchupDataSource + NodeStateDataSource + Send + Sync,
{
    type FeeAccount = espresso_types::FeeAccount;
    type RewardAccountV1 = espresso_types::v0_3::RewardAccountV1;
    type RewardAccountV2 = espresso_types::v0_4::RewardAccountV2;

    type AccountQueryData = espresso_types::AccountQueryData;
    type FeeMerkleTree = espresso_types::FeeMerkleTree;
    type BlocksFrontier = super::BlocksFrontier;
    type ChainConfig = espresso_types::v0_3::ChainConfig;
    type LeafChain = Vec<espresso_types::Leaf2>;
    type Cert2 = espresso_types::Certificate2<SeqTypes>;
    type RewardAccountQueryDataV1 = espresso_types::v0_3::RewardAccountQueryDataV1;
    type RewardMerkleTreeV1 = espresso_types::v0_3::RewardMerkleTreeV1;
    type RewardAccountQueryDataV2 = espresso_types::v0_4::RewardAccountQueryDataV2;
    type RewardMerkleTreeV2Data = serde_json::Value;
    type StateCert =
        hotshot_types::simple_certificate::LightClientStateUpdateCertificateV2<SeqTypes>;

    async fn get_account(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Self::AccountQueryData> {
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
        accounts: Vec<Self::FeeAccount>,
    ) -> anyhow::Result<Self::FeeMerkleTree> {
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let instance = ds.node_state().await;
        ds.get_accounts(&instance, height, view, &accounts)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_blocks_frontier(
        &self,
        height: u64,
        view: u64,
    ) -> anyhow::Result<Self::BlocksFrontier> {
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let instance = ds.node_state().await;
        ds.get_frontier(&instance, height, view)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_chain_config(&self, commitment: String) -> anyhow::Result<Self::ChainConfig> {
        let ds = &*self.data_source;
        let parsed: committable::Commitment<espresso_types::v0_3::ChainConfig> = commitment
            .parse()
            .map_err(|err| bad_request(format!("malformed chain config commitment: {err}")))?;
        ds.get_chain_config(parsed)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Self::LeafChain> {
        let ds = &*self.data_source;
        ds.get_leaf_chain(height)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_cert2(&self, height: u64) -> anyhow::Result<Self::Cert2> {
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
        accounts: Vec<Self::RewardAccountV1>,
    ) -> anyhow::Result<Self::RewardMerkleTreeV1> {
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let instance = ds.node_state().await;
        ds.get_reward_accounts_v1(&instance, height, view, &accounts)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }

    async fn get_reward_account_v2(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Self::RewardAccountQueryDataV2> {
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
        let ds = &*self.data_source;
        let view = hotshot_types::data::ViewNumber::new(view);
        let bytes = ds
            .get_reward_merkle_tree_v2(height, view)
            .await
            .map_err(|err| not_found(format!("{err:#}")))?;
        // The wire format is the raw Vec<u8> from `get_reward_merkle_tree_v2` encoded as the
        // JSON body; keep it that way for existing clients.
        Ok(serde_json::to_value(bytes)?)
    }

    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Self::StateCert> {
        let ds = &*self.data_source;
        ds.get_state_cert(epoch)
            .await
            .map_err(|err| not_found(format!("{err:#}")))
    }
}

#[async_trait]
impl<D> v1::SubmitApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: SubmitDataSourceErased + Send + Sync,
{
    type Transaction = espresso_types::Transaction;
    type TxHash = committable::Commitment<espresso_types::Transaction>;

    async fn submit(&self, tx: Self::Transaction) -> anyhow::Result<Self::TxHash> {
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
impl<C, D> SubmitDataSourceErased
    for hotshot_query_service::data_source::ExtensibleDataSource<D, crate::api::ApiState<C>>
where
    C: crate::api::context::ApiContext,
    D: Send + Sync,
{
    async fn submit_erased(&self, tx: espresso_types::Transaction) -> anyhow::Result<()> {
        <Self as SubmitDataSource>::submit(self, tx).await
    }
}

// Bare mode (no query/status API) has no `ExtensibleDataSource` wrapper: the app state is
// `ApiState<C>` directly, so it needs its own erased forwarding impl.
#[async_trait]
impl<C> SubmitDataSourceErased for crate::api::ApiState<C>
where
    C: crate::api::context::ApiContext,
{
    async fn submit_erased(&self, tx: espresso_types::Transaction) -> anyhow::Result<()> {
        <Self as SubmitDataSource>::submit(self, tx).await
    }
}

#[async_trait]
impl<D> v1::StateSignatureApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
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
impl<C, D> StateSignatureDataSourceErased
    for hotshot_query_service::data_source::ExtensibleDataSource<D, crate::api::ApiState<C>>
where
    C: crate::api::context::ApiContext,
    D: Send + Sync,
{
    async fn get_state_signature_erased(
        &self,
        height: u64,
    ) -> Option<hotshot_types::light_client::LCV3StateSignatureRequestBody> {
        <Self as StateSignatureDataSource>::get_state_signature(self, height).await
    }
}

// Bare mode (no query/status API) has no `ExtensibleDataSource` wrapper: the app state is
// `ApiState<C>` directly, so it needs its own erased forwarding impl.
#[async_trait]
impl<C> StateSignatureDataSourceErased for crate::api::ApiState<C>
where
    C: crate::api::context::ApiContext,
{
    async fn get_state_signature_erased(
        &self,
        height: u64,
    ) -> Option<hotshot_types::light_client::LCV3StateSignatureRequestBody> {
        <Self as StateSignatureDataSource>::get_state_signature(self, height).await
    }
}

#[async_trait]
impl<D> v1::ExplorerApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::explorer::ExplorerDataSource<SeqTypes> + Send + Sync,
{
    type BlockDetail = hotshot_query_service::explorer::BlockDetailResponse<SeqTypes>;
    type BlockSummaries = hotshot_query_service::explorer::BlockSummaryResponse<SeqTypes>;
    type TransactionDetail = hotshot_query_service::explorer::TransactionDetailResponse<SeqTypes>;
    type TransactionSummaries =
        hotshot_query_service::explorer::TransactionSummariesResponse<SeqTypes>;
    type ExplorerSummary = hotshot_query_service::explorer::ExplorerSummaryResponse<SeqTypes>;
    type SearchResult = hotshot_query_service::explorer::SearchResultResponse<SeqTypes>;

    async fn get_block_detail(&self, ident: v1::BlockIdent) -> anyhow::Result<Self::BlockDetail> {
        let ds = &*self.data_source;
        let target = match ident {
            v1::BlockIdent::Height(h) => BlockIdentifier::Height(h as usize),
            v1::BlockIdent::Hash(h) => BlockIdentifier::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?,
            ),
            v1::BlockIdent::Latest => BlockIdentifier::Latest,
        };
        ds.get_block_detail(target)
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_block_summaries(
        &self,
        target: v1::BlockIdent,
        limit: u64,
    ) -> anyhow::Result<Self::BlockSummaries> {
        let ds = &*self.data_source;
        let num_blocks = std::num::NonZeroUsize::new(limit as usize)
            .ok_or_else(|| bad_request("limit must be greater than 0"))?;
        if num_blocks.get() > 100 {
            return Err(bad_request("limit must be <= 100"));
        }
        let target = match target {
            v1::BlockIdent::Height(h) => BlockIdentifier::Height(h as usize),
            v1::BlockIdent::Hash(h) => BlockIdentifier::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?,
            ),
            v1::BlockIdent::Latest => BlockIdentifier::Latest,
        };
        ds.get_block_summaries(GetBlockSummariesRequest(BlockRange { target, num_blocks }))
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_transaction_detail(
        &self,
        ident: v1::TxIdent,
    ) -> anyhow::Result<Self::TransactionDetail> {
        let ds = &*self.data_source;
        let target = match ident {
            v1::TxIdent::HeightAndOffset(h, o) => {
                TransactionIdentifier::HeightAndOffset(h as usize, o as usize)
            },
            v1::TxIdent::Hash(h) => TransactionIdentifier::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid tx hash {h}: {err}")))?,
            ),
            v1::TxIdent::Latest => TransactionIdentifier::Latest,
        };
        ds.get_transaction_detail(target)
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_transaction_summaries(
        &self,
        target: v1::TxIdent,
        limit: u64,
        filter: v1::TxSummaryFilter,
    ) -> anyhow::Result<Self::TransactionSummaries> {
        let ds = &*self.data_source;
        let num_transactions = std::num::NonZeroUsize::new(limit as usize)
            .ok_or_else(|| bad_request("limit must be greater than 0"))?;
        if num_transactions.get() > 100 {
            return Err(bad_request("limit must be <= 100"));
        }
        let target = match target {
            v1::TxIdent::HeightAndOffset(h, o) => {
                TransactionIdentifier::HeightAndOffset(h as usize, o as usize)
            },
            v1::TxIdent::Hash(h) => TransactionIdentifier::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid tx hash {h}: {err}")))?,
            ),
            v1::TxIdent::Latest => TransactionIdentifier::Latest,
        };
        let filter = match filter {
            v1::TxSummaryFilter::None => TransactionSummaryFilter::None,
            v1::TxSummaryFilter::Block(b) => TransactionSummaryFilter::Block(b as usize),
            v1::TxSummaryFilter::Namespace(n) => TransactionSummaryFilter::RollUp(n.into()),
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
        let ds = &*self.data_source;
        ds.get_explorer_summary()
            .await
            .map(Into::into)
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    async fn get_search_result(&self, query: String) -> anyhow::Result<Self::SearchResult> {
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

#[async_trait]
impl<D> v1::LightClientApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: AvailabilityDataSource<SeqTypes>
        + hotshot_query_service::merklized_state::MerklizedStateDataSource<
            SeqTypes,
            espresso_types::BlockMerkleTree,
            3,
        > + NodeStateDataSource
        + StakeTableDataSource<SeqTypes>
        + hotshot_query_service::data_source::VersionedDataSource
        + Sized
        + Clone
        + Send
        + Sync
        + 'static,
    for<'a> <D::Target as hotshot_query_service::data_source::VersionedDataSource>::ReadOnly<'a>:
        hotshot_query_service::data_source::storage::NodeStorage<SeqTypes>
            + hotshot_query_service::data_source::storage::AvailabilityStorage<SeqTypes>,
{
    type LeafProof = light_client::consensus::leaf::LeafProof;
    type HeaderProof = light_client::consensus::header::HeaderProof;
    type StakeTableEvents = Vec<espresso_types::v0_3::StakeTableEvent>;
    type PayloadProof = light_client::consensus::payload::PayloadProof;
    type NamespaceProof = light_client::consensus::namespace::NamespaceProof;

    async fn get_leaf_proof(
        &self,
        query: v1::LeafQuery,
        finalized: Option<u64>,
    ) -> anyhow::Result<Self::LeafProof> {
        let ds = &*self.data_source;
        let fetch_timeout = FETCH_TIMEOUT;

        let requested = match query {
            v1::LeafQuery::Height(h) => HsLeafId::Number(h as usize),
            v1::LeafQuery::Hash(h) => HsLeafId::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid leaf hash {h}: {err}")))?,
            ),
            v1::LeafQuery::BlockHash(h) => {
                let parsed = h
                    .parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?;
                let header = AvailabilityDataSource::get_header(ds, HsBlockId::Hash(parsed))
                    .await
                    .with_timeout(fetch_timeout)
                    .await
                    .ok_or_else(|| not_found(format!("unknown block hash {h}")))?;
                HsLeafId::Number(header.height() as usize)
            },
            v1::LeafQuery::PayloadHash(h) => {
                let parsed = h
                    .parse()
                    .map_err(|err| bad_request(format!("invalid payload hash {h}: {err}")))?;
                let header = AvailabilityDataSource::get_header(ds, HsBlockId::PayloadHash(parsed))
                    .await
                    .with_timeout(fetch_timeout)
                    .await
                    .ok_or_else(|| not_found(format!("unknown payload hash {h}")))?;
                HsLeafId::Number(header.height() as usize)
            },
        };

        let requested_leaf = AvailabilityDataSource::get_leaf(ds, requested)
            .await
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found(format!("unknown leaf {requested}")))?;

        crate::api::light_client::get_leaf_proof(
            ds,
            requested_leaf,
            finalized.map(|f| f as usize),
            fetch_timeout,
            lc_leaf_proof_chain_limit(),
        )
        .await
        .map_err(lc_error)
    }

    async fn get_header_proof(
        &self,
        root: u64,
        requested: v1::HeaderQuery,
    ) -> anyhow::Result<Self::HeaderProof> {
        let ds = &*self.data_source;
        let fetch_timeout = FETCH_TIMEOUT;
        let requested = match requested {
            v1::HeaderQuery::Height(h) => HsBlockId::Number(h as usize),
            v1::HeaderQuery::Hash(h) => HsBlockId::Hash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid block hash {h}: {err}")))?,
            ),
            v1::HeaderQuery::PayloadHash(h) => HsBlockId::PayloadHash(
                h.parse()
                    .map_err(|err| bad_request(format!("invalid payload hash {h}: {err}")))?,
            ),
        };
        crate::api::light_client::get_header_proof(ds, root, requested, fetch_timeout)
            .await
            .map_err(lc_error)
    }

    async fn get_light_client_stake_table(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Self::StakeTableEvents> {
        let ds = &*self.data_source;
        let fetch_timeout = FETCH_TIMEOUT;

        let node_state = NodeStateDataSource::node_state(ds).await;
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
        let epoch_root = AvailabilityDataSource::get_header::<HsBlockId<SeqTypes>>(
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
            let prev_epoch_root = AvailabilityDataSource::get_header::<HsBlockId<SeqTypes>>(
                ds,
                HsBlockId::Number(prev_epoch_root_height),
            )
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

        StakeTableDataSource::stake_table_events(ds, from_l1_block, to_l1_block).await
    }

    async fn get_payload_proof(&self, height: u64) -> anyhow::Result<Self::PayloadProof> {
        let ds = &*self.data_source;
        let fetch_timeout = FETCH_TIMEOUT;
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
        let ds = &*self.data_source;
        let fetch_timeout = FETCH_TIMEOUT;
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

    async fn get_payload_proof_ranges(
        &self,
        ranges: Vec<Range<u64>>,
    ) -> anyhow::Result<Vec<Self::PayloadProof>> {
        let ranges = validate_ranges(ranges, lc_large_object_range_limit())?;

        let heights: u64 = ranges.iter().map(|range| range.end - range.start).sum();
        let read = async {
            let mut tx = self.data_source.read().await.ok()?;
            let blocks = tx.get_block_ranges(&ranges).await.ok()?;
            let vid_common = tx.get_vid_common_ranges(&ranges).await.ok()?;
            (blocks.len() as u64 == heights && vid_common.len() as u64 == heights)
                .then_some((blocks, vid_common))
        }
        .await;
        if let Some((blocks, vid_common)) = read {
            // By height, not by position: a proof built from one height's payload and another
            // height's VID common cannot verify, and the two are read separately.
            let mut vid_common: HashMap<u64, _> = vid_common
                .into_iter()
                .map(|common| (common.height(), common))
                .collect();
            return blocks
                .into_iter()
                .map(|block| {
                    let common = vid_common.remove(&block.height()).ok_or_else(|| {
                        not_found(format!("VID common {} not found", block.height()))
                    })?;
                    Ok(light_client::consensus::payload::PayloadProof::new(
                        block.payload().clone(),
                        common.common().clone(),
                    ))
                })
                .collect();
        }

        let ranges: Vec<_> = futures::stream::iter(ranges)
            .map(|range| self.get_payload_proof_range(range.start, range.end))
            .buffered(self.ranges_concurrency.get())
            .try_collect()
            .await?;
        Ok(ranges.into_iter().flatten().collect())
    }

    async fn get_lc_namespace_proof(
        &self,
        height: u64,
        namespace: u64,
    ) -> anyhow::Result<Self::NamespaceProof> {
        let ds = &*self.data_source;
        let fetch_timeout = FETCH_TIMEOUT;
        let mut proofs = crate::api::light_client::get_namespace_proof_range(
            ds,
            height as usize,
            (height + 1) as usize,
            namespace,
            fetch_timeout,
            lc_large_object_range_limit(),
        )
        .await
        .map_err(lc_error)?;
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
        let fetch_timeout = FETCH_TIMEOUT;
        crate::api::light_client::get_namespace_proof_range(
            ds,
            start as usize,
            end as usize,
            namespace,
            fetch_timeout,
            lc_large_object_range_limit(),
        )
        .await
        .map_err(lc_error)
    }

    async fn get_lc_namespaces_proof_range(
        &self,
        start: u64,
        end: u64,
        namespaces: String,
    ) -> anyhow::Result<Vec<std::collections::HashMap<u64, Self::NamespaceProof>>> {
        let namespaces = crate::api::light_client::parse_namespaces_str(&namespaces)
            .map_err(|err| bad_request(err.to_string()))?;
        let ds = &*self.data_source;
        let fetch_timeout = FETCH_TIMEOUT;
        crate::api::light_client::get_namespaces_proof_range(
            ds,
            start as usize,
            end as usize,
            &namespaces,
            fetch_timeout,
            lc_large_object_range_limit(),
        )
        .await
        .map_err(lc_error)
    }
}

fn lc_large_object_range_limit() -> usize {
    hotshot_query_service::availability::Options::default().large_object_range_limit
}

/// Convert a query-service error to an [`AvailabilityError`]-carrying anyhow error so the HTTP
/// layer returns the status carried by the error (400/404) instead of 500.
pub(crate) fn lc_error(err: hotshot_query_service::Error) -> anyhow::Error {
    match err.status() {
        StatusCode::NOT_FOUND => not_found(err.to_string()),
        StatusCode::BAD_REQUEST => bad_request(err.to_string()),
        _ => anyhow::anyhow!("{err}"),
    }
}

/// Bounds the leaves in a single leaf proof, and so the memory to build and serialize it.
///
/// Tracks the `hotshot_query_service` small-object range limit, so a dependency bump that
/// changes that default changes this bound too.
fn lc_leaf_proof_chain_limit() -> usize {
    hotshot_query_service::availability::Options::default().small_object_range_limit
}

#[async_trait]
impl<D> v1::HotShotEventsApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_events_service::events_source::EventsSource<SeqTypes> + Send + Sync,
{
    type Event = std::sync::Arc<hotshot_types::event::Event<SeqTypes>>;
    type StartupInfo = hotshot_events_service::events_source::StartupInfo<SeqTypes>;

    async fn startup_info(&self) -> anyhow::Result<Self::StartupInfo> {
        let ds = &*self.data_source;
        Ok(ds.get_startup_info().await)
    }

    async fn events(&self) -> anyhow::Result<futures::stream::BoxStream<'static, Self::Event>> {
        let ds = &*self.data_source;
        let stream = ds.get_event_stream(None).await;
        Ok(Box::pin(stream))
    }
}

#[async_trait]
impl<D> v1::TokenApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: TokenDataSource<SeqTypes> + NodeStateDataSource + Send + Sync,
{
    async fn total_minted_supply(&self) -> anyhow::Result<String> {
        let ds = &*self.data_source;
        let value = ds
            .get_total_supply_l1()
            .await
            .map_err(|err| anyhow::anyhow!("failed to get total supply: {err:#}"))?;
        Ok(format_ether(value))
    }

    async fn circulating_supply(&self) -> anyhow::Result<String> {
        let calc = fetch_supply_inputs(&*self.data_source).await?;
        Ok(format_ether(calc.circulating_supply()))
    }

    async fn circulating_supply_ethereum(&self) -> anyhow::Result<String> {
        let calc = fetch_supply_inputs(&*self.data_source).await?;
        Ok(format_ether(calc.circulating_supply_ethereum()))
    }

    async fn total_issued_supply(&self) -> anyhow::Result<String> {
        let calc = fetch_supply_inputs(&*self.data_source).await?;
        Ok(format_ether(calc.total_issued_supply()))
    }

    async fn total_reward_distributed(&self) -> anyhow::Result<String> {
        let calc = fetch_supply_inputs(&*self.data_source).await?;
        Ok(format_ether(calc.total_reward_distributed()))
    }
}

async fn fetch_supply_inputs<S>(
    ds: &S,
) -> anyhow::Result<crate::api::unlock_schedule::SupplyCalculator>
where
    S: TokenDataSource<SeqTypes> + NodeStateDataSource + Sync + ?Sized,
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

#[tonic::async_trait]
impl<D> proto::token_service_server::TokenService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: TokenDataSource<SeqTypes> + NodeStateDataSource + Send + Sync,
{
    async fn get_total_minted_supply(
        &self,
        _request: tonic::Request<proto::GetTotalMintedSupplyRequest>,
    ) -> Result<tonic::Response<proto::TotalMintedSupplyResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::total_minted_supply(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TotalMintedSupplyResponse {
            amount,
        }))
    }

    async fn get_circulating_supply(
        &self,
        _request: tonic::Request<proto::GetCirculatingSupplyRequest>,
    ) -> Result<tonic::Response<proto::CirculatingSupplyResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::circulating_supply(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::CirculatingSupplyResponse {
            amount,
        }))
    }

    async fn get_circulating_supply_ethereum(
        &self,
        _request: tonic::Request<proto::GetCirculatingSupplyEthereumRequest>,
    ) -> Result<tonic::Response<proto::CirculatingSupplyEthereumResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::circulating_supply_ethereum(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::CirculatingSupplyEthereumResponse { amount },
        ))
    }

    async fn get_total_issued_supply(
        &self,
        _request: tonic::Request<proto::GetTotalIssuedSupplyRequest>,
    ) -> Result<tonic::Response<proto::TotalIssuedSupplyResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::total_issued_supply(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TotalIssuedSupplyResponse {
            amount,
        }))
    }

    async fn get_total_reward_distributed(
        &self,
        _request: tonic::Request<proto::GetTotalRewardDistributedRequest>,
    ) -> Result<tonic::Response<proto::TotalRewardDistributedResponse>, tonic::Status> {
        let amount = <Self as v1::TokenApi>::total_reward_distributed(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::TotalRewardDistributedResponse { amount },
        ))
    }
}

#[async_trait]
impl<D> v1::DatabaseApi for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: DatabaseMetadataSource + Send + Sync,
{
    type TableSizes = Vec<TableSize>;
    type MigrationStatus = Vec<MigrationStatus>;

    async fn get_table_sizes(&self) -> anyhow::Result<Self::TableSizes> {
        let ds = &*self.data_source;
        ds.get_table_sizes().await
    }

    async fn get_migration_status(&self) -> anyhow::Result<Self::MigrationStatus> {
        let ds = &*self.data_source;
        ds.get_migration_status().await
    }
}

#[tonic::async_trait]
impl<D> proto::database_service_server::DatabaseService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: DatabaseMetadataSource + Send + Sync,
{
    async fn get_table_sizes(
        &self,
        _request: tonic::Request<proto::GetTableSizesRequest>,
    ) -> Result<tonic::Response<proto::TableSizesResponse>, tonic::Status> {
        let tables = <Self as v1::DatabaseApi>::get_table_sizes(self)
            .await
            .map_err(to_status)?
            .into_iter()
            .map(|table| proto::TableSize {
                table_name: table.table_name,
                row_count: table.row_count,
                total_size_bytes: table.total_size_bytes,
            })
            .collect();
        Ok(tonic::Response::new(proto::TableSizesResponse { tables }))
    }

    async fn get_migration_status(
        &self,
        _request: tonic::Request<proto::GetMigrationStatusRequest>,
    ) -> Result<tonic::Response<proto::MigrationStatusResponse>, tonic::Status> {
        let migrations = <Self as v1::DatabaseApi>::get_migration_status(self)
            .await
            .map_err(to_status)?
            .into_iter()
            .map(|migration| proto::MigrationStatus {
                name: migration.name,
                // v1 serializes these through chrono's serde impl, which ends in `Z`, where plain
                // `to_rfc3339` would write `+00:00` and disagree with it and with protoJSON.
                started_at: migration
                    .started_at
                    .to_rfc3339_opts(SecondsFormat::AutoSi, true),
                completed_at: migration
                    .completed_at
                    .map(|time| time.to_rfc3339_opts(SecondsFormat::AutoSi, true)),
                last_offset: migration.last_offset,
            })
            .collect();
        Ok(tonic::Response::new(proto::MigrationStatusResponse {
            migrations,
        }))
    }
}

fn block_id_from_query(
    height: Option<u64>,
    hash: Option<String>,
    payload_hash: Option<String>,
) -> Result<v1::availability::BlockId, tonic::Status> {
    match (height, hash, payload_hash) {
        (Some(height), None, None) => Ok(v1::availability::BlockId::Height(height)),
        (None, Some(hash), None) => Ok(v1::availability::BlockId::Hash(hash)),
        (None, None, Some(payload_hash)) => {
            Ok(v1::availability::BlockId::PayloadHash(payload_hash))
        },
        _ => Err(tonic::Status::invalid_argument(
            "set exactly one of height, hash or payload_hash",
        )),
    }
}

/// Namespace ids are 32 bits on chain but travel as uint64 so they round-trip with the
/// responses' `namespace` fields. Anything wider is a client error, not a truncation.
fn namespace_from_query(namespace: Option<u64>) -> Result<Option<u32>, tonic::Status> {
    namespace
        .map(|namespace| {
            u32::try_from(namespace)
                .map_err(|_| tonic::Status::invalid_argument("namespace does not fit in 32 bits"))
        })
        .transpose()
}

fn required<T>(value: Option<T>, name: &str) -> Result<T, tonic::Status> {
    value.ok_or_else(|| tonic::Status::invalid_argument(format!("{name} is required")))
}

fn range_from_query(from: Option<u64>, until: Option<u64>) -> Result<Range<u64>, tonic::Status> {
    Ok(required(from, "from")?..required(until, "until")?)
}

/// A conversion error is sent as an `event: error` frame. Ending the stream there means a
/// subscriber that skips the frame cannot miss a height without noticing.
fn end_at_first_error<T, S>(items: S) -> BoxStream<'static, Result<T, tonic::Status>>
where
    T: Send + 'static,
    S: futures::Stream<Item = Result<T, tonic::Status>> + Send + 'static,
{
    items
        .scan(false, |failed, item| {
            if *failed {
                return futures::future::ready(None);
            }
            *failed = item.is_err();
            futures::future::ready(Some(item))
        })
        .boxed()
}

fn ranges_from_body(ranges: Vec<proto::HeightRange>) -> Result<Vec<Range<u64>>, tonic::Status> {
    ranges
        .into_iter()
        .map(|range| range_from_query(range.from, range.until))
        .collect()
}

#[tonic::async_trait]
impl<D> proto::availability_service_server::AvailabilityService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    // Delegates to both v1 availability traits, so it needs the bounds of both.
    D::Target: AvailabilityDataSource<SeqTypes>
        + hotshot_query_service::data_source::VersionedDataSource
        + hotshot_query_service::node::NodeDataSource<SeqTypes>
        + RequestResponseDataSource<SeqTypes>
        + StateCertDataSource
        + StateCertFetchingDataSource<SeqTypes>
        + Send
        + Sync,
    for<'a> <D::Target as hotshot_query_service::data_source::VersionedDataSource>::ReadOnly<'a>:
        hotshot_query_service::data_source::storage::AvailabilityStorage<SeqTypes>,
{
    async fn get_limits(
        &self,
        _request: tonic::Request<proto::GetLimitsRequest>,
    ) -> Result<tonic::Response<proto::LimitsResponse>, tonic::Status> {
        let limits = <Self as v1::HotShotAvailabilityApi>::get_limits(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LimitsResponse {
            small_object_range_limit: limits.small_object_range_limit as u64,
            large_object_range_limit: limits.large_object_range_limit as u64,
            namespace_proof_range_limit: NAMESPACE_PROOF_RANGE_LIMIT,
        }))
    }

    async fn get_header(
        &self,
        request: tonic::Request<proto::GetHeaderRequest>,
    ) -> Result<tonic::Response<proto::HeaderResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = block_id_from_query(request.height, request.hash, request.payload_hash)?;
        let header = <Self as v1::HotShotAvailabilityApi>::get_header(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::HeaderResponse::from(&header)))
    }

    async fn get_header_range(
        &self,
        request: tonic::Request<proto::GetHeaderRangeRequest>,
    ) -> Result<tonic::Response<proto::HeaderRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let headers = <Self as v1::HotShotAvailabilityApi>::get_header_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::HeaderRangeResponse::from(
            &*headers,
        )))
    }

    async fn get_leaf(
        &self,
        request: tonic::Request<proto::GetLeafRequest>,
    ) -> Result<tonic::Response<proto::LeafResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash) {
            (Some(height), None) => v1::availability::LeafId::Height(height),
            (None, Some(hash)) => v1::availability::LeafId::Hash(hash),
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height or hash",
                ));
            },
        };
        let leaf = <Self as v1::HotShotAvailabilityApi>::get_leaf(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LeafResponse::from(&leaf)))
    }

    async fn get_leaf_range(
        &self,
        request: tonic::Request<proto::GetLeafRangeRequest>,
    ) -> Result<tonic::Response<proto::LeafRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let leaves = <Self as v1::HotShotAvailabilityApi>::get_leaf_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LeafRangeResponse::from(
            &*leaves,
        )))
    }

    async fn get_leaf_ranges(
        &self,
        request: tonic::Request<proto::GetLeafRangesRequest>,
    ) -> Result<tonic::Response<proto::LeafRangeResponse>, tonic::Status> {
        let ranges = ranges_from_body(request.into_inner().ranges)?;
        let leaves = <Self as v1::HotShotAvailabilityApi>::get_leaf_ranges(self, ranges)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LeafRangeResponse::from(
            &*leaves,
        )))
    }

    async fn get_cert2(
        &self,
        request: tonic::Request<proto::GetCert2Request>,
    ) -> Result<tonic::Response<proto::Cert2Response>, tonic::Status> {
        let height = required(request.into_inner().height, "height")?;
        let cert2 = <Self as v1::HotShotAvailabilityApi>::get_cert2(self, height)
            .await
            .map_err(to_status)?
            .ok_or_else(|| {
                tonic::Status::not_found(format!("no cert2 available for height {height}"))
            })?;
        Ok(tonic::Response::new(proto::Cert2Response {
            certificate: Some((&cert2).into()),
        }))
    }

    async fn get_block(
        &self,
        request: tonic::Request<proto::GetBlockRequest>,
    ) -> Result<tonic::Response<proto::BlockResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = block_id_from_query(request.height, request.hash, request.payload_hash)?;
        let block = <Self as v1::HotShotAvailabilityApi>::get_block(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockResponse::from(&block)))
    }

    async fn get_block_range(
        &self,
        request: tonic::Request<proto::GetBlockRangeRequest>,
    ) -> Result<tonic::Response<proto::BlockRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let blocks = <Self as v1::HotShotAvailabilityApi>::get_block_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockRangeResponse::from(
            &*blocks,
        )))
    }

    async fn get_block_ranges(
        &self,
        request: tonic::Request<proto::GetBlockRangesRequest>,
    ) -> Result<tonic::Response<proto::BlockRangeResponse>, tonic::Status> {
        let ranges = ranges_from_body(request.into_inner().ranges)?;
        let blocks = <Self as v1::HotShotAvailabilityApi>::get_block_ranges(self, ranges)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockRangeResponse::from(
            &*blocks,
        )))
    }

    async fn get_payload(
        &self,
        request: tonic::Request<proto::GetPayloadRequest>,
    ) -> Result<tonic::Response<proto::PayloadResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash, request.block_hash) {
            (Some(height), None, None) => v1::availability::PayloadId::Height(height),
            (None, Some(hash), None) => v1::availability::PayloadId::Hash(hash),
            (None, None, Some(block_hash)) => v1::availability::PayloadId::BlockHash(block_hash),
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height, hash or block_hash",
                ));
            },
        };
        let payload = <Self as v1::HotShotAvailabilityApi>::get_payload(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::PayloadResponse::from(&payload)))
    }

    async fn get_payload_range(
        &self,
        request: tonic::Request<proto::GetPayloadRangeRequest>,
    ) -> Result<tonic::Response<proto::PayloadRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let payloads = <Self as v1::HotShotAvailabilityApi>::get_payload_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::PayloadRangeResponse::from(
            &*payloads,
        )))
    }

    async fn get_vid_common(
        &self,
        request: tonic::Request<proto::GetVidCommonRequest>,
    ) -> Result<tonic::Response<proto::VidCommonResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = block_id_from_query(request.height, request.hash, request.payload_hash)?;
        let common = <Self as v1::HotShotAvailabilityApi>::get_vid_common(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::VidCommonResponse::try_from(
            &common,
        )?))
    }

    async fn get_vid_common_range(
        &self,
        request: tonic::Request<proto::GetVidCommonRangeRequest>,
    ) -> Result<tonic::Response<proto::VidCommonRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let items = <Self as v1::HotShotAvailabilityApi>::get_vid_common_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::VidCommonRangeResponse::try_from(&*items)?,
        ))
    }

    async fn get_vid_common_ranges(
        &self,
        request: tonic::Request<proto::GetVidCommonRangesRequest>,
    ) -> Result<tonic::Response<proto::VidCommonRangeResponse>, tonic::Status> {
        let ranges = ranges_from_body(request.into_inner().ranges)?;
        let items = <Self as v1::HotShotAvailabilityApi>::get_vid_common_ranges(self, ranges)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::VidCommonRangeResponse::try_from(&*items)?,
        ))
    }

    async fn get_transaction(
        &self,
        request: tonic::Request<proto::GetTransactionRequest>,
    ) -> Result<tonic::Response<proto::TransactionResponse>, tonic::Status> {
        let request = request.into_inner();
        let tx = match (request.height, request.index, request.hash) {
            (Some(height), Some(index), None) => {
                <Self as v1::HotShotAvailabilityApi>::get_transaction_by_position(
                    self, height, index,
                )
                .await
            },
            (None, None, Some(hash)) => {
                <Self as v1::HotShotAvailabilityApi>::get_transaction_by_hash(self, hash).await
            },
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set height and index, or hash",
                ));
            },
        }
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::TransactionResponse::from(&tx)))
    }

    async fn get_transaction_proof(
        &self,
        request: tonic::Request<proto::GetTransactionProofRequest>,
    ) -> Result<tonic::Response<proto::TransactionWithProofResponse>, tonic::Status> {
        let request = request.into_inner();
        let tx = match (request.height, request.index, request.hash) {
            (Some(height), Some(index), None) => {
                <Self as v1::HotShotAvailabilityApi>::get_transaction_proof_by_position(
                    self, height, index,
                )
                .await
            },
            (None, None, Some(hash)) => {
                <Self as v1::HotShotAvailabilityApi>::get_transaction_proof_by_hash(self, hash)
                    .await
            },
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set height and index, or hash",
                ));
            },
        }
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::TransactionWithProofResponse::try_from(&tx)?,
        ))
    }

    async fn get_block_summary(
        &self,
        request: tonic::Request<proto::GetBlockSummaryRequest>,
    ) -> Result<tonic::Response<proto::BlockSummaryResponse>, tonic::Status> {
        let height = required(request.into_inner().height, "height")? as usize;
        let summary = <Self as v1::HotShotAvailabilityApi>::get_block_summary(self, height)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockSummaryResponse::from(
            &summary,
        )))
    }

    async fn get_block_summary_range(
        &self,
        request: tonic::Request<proto::GetBlockSummaryRangeRequest>,
    ) -> Result<tonic::Response<proto::BlockSummaryRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let summaries = <Self as v1::HotShotAvailabilityApi>::get_block_summary_range(
            self,
            range.start as usize,
            range.end as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::BlockSummaryRangeResponse::from(&*summaries),
        ))
    }

    async fn get_namespace_proof(
        &self,
        request: tonic::Request<proto::GetNamespaceProofRequest>,
    ) -> Result<tonic::Response<proto::NamespaceProofResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = block_id_from_query(request.height, request.hash, request.payload_hash)?;
        let namespace = required(namespace_from_query(request.namespace)?, "namespace")?;
        let proof = <Self as v1::AvailabilityApi>::get_namespace_proof(self, id, namespace)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::NamespaceProofResponse::try_from(&proof)?,
        ))
    }

    async fn get_namespace_proof_range(
        &self,
        request: tonic::Request<proto::GetNamespaceProofRangeRequest>,
    ) -> Result<tonic::Response<proto::NamespaceProofRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let range = range_from_query(request.from, request.until)?;
        let namespace = required(namespace_from_query(request.namespace)?, "namespace")?;
        let proofs = <Self as v1::AvailabilityApi>::get_namespace_proof_range(
            self,
            range.start,
            range.end,
            namespace,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::NamespaceProofRangeResponse::try_from(&*proofs)?,
        ))
    }

    async fn get_incorrect_encoding_proof(
        &self,
        request: tonic::Request<proto::GetIncorrectEncodingProofRequest>,
    ) -> Result<tonic::Response<proto::IncorrectEncodingProofResponse>, tonic::Status> {
        let request = request.into_inner();
        let height = required(request.height, "height")?;
        let namespace = required(namespace_from_query(request.namespace)?, "namespace")?;
        let proof = <Self as v1::AvailabilityApi>::get_incorrect_encoding_proof(
            self,
            v1::availability::BlockId::Height(height),
            namespace,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            proto::IncorrectEncodingProofResponse {
                proof: Some((&proof).try_into()?),
            },
        ))
    }

    async fn get_state_cert(
        &self,
        request: tonic::Request<proto::GetStateCertRequest>,
    ) -> Result<tonic::Response<proto::StateCertV1Response>, tonic::Status> {
        let cert = <Self as v1::AvailabilityApi>::get_state_cert(
            self,
            required(request.into_inner().epoch, "epoch")?,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::StateCertV1Response::from(
            &cert,
        )))
    }

    async fn get_state_cert_v2(
        &self,
        request: tonic::Request<proto::GetStateCertV2Request>,
    ) -> Result<tonic::Response<proto::StateCertV2Response>, tonic::Status> {
        let cert = <Self as v1::AvailabilityApi>::get_state_cert_v2(
            self,
            required(request.into_inner().epoch, "epoch")?,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::StateCertV2Response::from(
            &cert,
        )))
    }

    type StreamLeavesStream = BoxStream<'static, Result<proto::LeafResponse, tonic::Status>>;

    async fn stream_leaves(
        &self,
        request: tonic::Request<proto::StreamLeavesRequest>,
    ) -> Result<tonic::Response<Self::StreamLeavesStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let leaves = <Self as v1::HotShotAvailabilityApi>::stream_leaves(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            leaves
                .map(|leaf| Ok(proto::LeafResponse::from(&leaf)))
                .boxed(),
        ))
    }

    type StreamHeadersStream = BoxStream<'static, Result<proto::HeaderResponse, tonic::Status>>;

    async fn stream_headers(
        &self,
        request: tonic::Request<proto::StreamHeadersRequest>,
    ) -> Result<tonic::Response<Self::StreamHeadersStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let headers = <Self as v1::HotShotAvailabilityApi>::stream_headers(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            headers
                .map(|header| Ok(proto::HeaderResponse::from(&header)))
                .boxed(),
        ))
    }

    type StreamBlocksStream = BoxStream<'static, Result<proto::BlockResponse, tonic::Status>>;

    async fn stream_blocks(
        &self,
        request: tonic::Request<proto::StreamBlocksRequest>,
    ) -> Result<tonic::Response<Self::StreamBlocksStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let blocks = <Self as v1::HotShotAvailabilityApi>::stream_blocks(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            blocks
                .map(|block| Ok(proto::BlockResponse::from(&block)))
                .boxed(),
        ))
    }

    type StreamPayloadsStream = BoxStream<'static, Result<proto::PayloadResponse, tonic::Status>>;

    async fn stream_payloads(
        &self,
        request: tonic::Request<proto::StreamPayloadsRequest>,
    ) -> Result<tonic::Response<Self::StreamPayloadsStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let payloads = <Self as v1::HotShotAvailabilityApi>::stream_payloads(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(
            payloads
                .map(|payload| Ok(proto::PayloadResponse::from(&payload)))
                .boxed(),
        ))
    }

    type StreamVidCommonStream =
        BoxStream<'static, Result<proto::VidCommonResponse, tonic::Status>>;

    async fn stream_vid_common(
        &self,
        request: tonic::Request<proto::StreamVidCommonRequest>,
    ) -> Result<tonic::Response<Self::StreamVidCommonStream>, tonic::Status> {
        let from = required(request.into_inner().from, "from")? as usize;
        let items = <Self as v1::HotShotAvailabilityApi>::stream_vid_common(self, from)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(end_at_first_error(
            items.map(|item| proto::VidCommonResponse::try_from(&item)),
        )))
    }

    type StreamTransactionsStream =
        BoxStream<'static, Result<proto::TransactionResponse, tonic::Status>>;

    async fn stream_transactions(
        &self,
        request: tonic::Request<proto::StreamTransactionsRequest>,
    ) -> Result<tonic::Response<Self::StreamTransactionsStream>, tonic::Status> {
        let request = request.into_inner();
        let transactions = <Self as v1::HotShotAvailabilityApi>::stream_transactions(
            self,
            required(request.from, "from")? as usize,
            namespace_from_query(request.namespace)?,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(
            transactions
                .map(|tx| Ok(proto::TransactionResponse::from(&tx)))
                .boxed(),
        ))
    }

    type StreamNamespaceProofsStream =
        BoxStream<'static, Result<proto::NamespaceProofResponse, tonic::Status>>;

    async fn stream_namespace_proofs(
        &self,
        request: tonic::Request<proto::StreamNamespaceProofsRequest>,
    ) -> Result<tonic::Response<Self::StreamNamespaceProofsStream>, tonic::Status> {
        let request = request.into_inner();
        let namespace = required(namespace_from_query(request.namespace)?, "namespace")?;
        let proofs = <Self as v1::AvailabilityApi>::stream_namespace_proofs(
            self,
            required(request.from, "from")? as usize,
            namespace,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(end_at_first_error(proofs.map(
            |proof| proto::NamespaceProofResponse::try_from(&proof),
        ))))
    }
}

#[tonic::async_trait]
impl<D> proto::merklized_state_service_server::MerklizedStateService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: hotshot_query_service::merklized_state::MerklizedStateDataSource<
            SeqTypes,
            espresso_types::BlockMerkleTree,
            { <espresso_types::BlockMerkleTree as jf_merkle_tree_compat::MerkleTreeScheme>::ARITY },
        > + hotshot_query_service::merklized_state::MerklizedStateDataSource<
            SeqTypes,
            espresso_types::FeeMerkleTree,
            { <espresso_types::FeeMerkleTree as jf_merkle_tree_compat::MerkleTreeScheme>::ARITY },
        > + hotshot_query_service::merklized_state::MerklizedStateHeightPersistence
        + Send
        + Sync,
{
    async fn get_block_state_path(
        &self,
        request: tonic::Request<proto::GetBlockStatePathRequest>,
    ) -> Result<tonic::Response<proto::MerklePathResponse>, tonic::Status> {
        let request = request.into_inner();
        let key = required(request.key, "key")?;
        let snapshot = snapshot_from_query(request.height, request.commit)?;
        // v1 takes the key as a string because its route carried it as a path segment.
        let proof =
            <Self as v1::BlockStateApi>::get_block_state_path(self, snapshot, key.to_string())
                .await
                .map_err(to_status)?;
        Ok(tonic::Response::new(proto::MerklePathResponse::from(
            &proof,
        )))
    }

    async fn get_fee_state_path(
        &self,
        request: tonic::Request<proto::GetFeeStatePathRequest>,
    ) -> Result<tonic::Response<proto::MerklePathResponse>, tonic::Status> {
        let request = request.into_inner();
        let address = required(request.address, "address")?;
        let snapshot = snapshot_from_query(request.height, request.commit)?;
        let proof = <Self as v1::FeeStateApi>::get_fee_state_path(self, snapshot, address)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::MerklePathResponse::from(
            &proof,
        )))
    }

    async fn get_latest_fee_balance(
        &self,
        request: tonic::Request<proto::GetLatestFeeBalanceRequest>,
    ) -> Result<tonic::Response<proto::FeeBalanceResponse>, tonic::Status> {
        let address = required(request.into_inner().address, "address")?;
        let balance = <Self as v1::FeeStateApi>::get_fee_balance_latest(self, address)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::FeeBalanceResponse {
            balance: balance.unwrap_or_default().to_string(),
        }))
    }

    async fn get_state_height(
        &self,
        _request: tonic::Request<proto::GetStateHeightRequest>,
    ) -> Result<tonic::Response<proto::StateHeightResponse>, tonic::Status> {
        let height = <Self as v1::BlockStateApi>::get_block_state_height(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::StateHeightResponse { height }))
    }
}

#[tonic::async_trait]
impl<D> proto::reward_state_service_server::RewardStateService for NodeApiStateImpl<D>
where
    D: RewardMerkleTreeDataSource + Deref + Clone + Send + Sync + 'static,
    // Both trees, because every method here delegates to `v1::RewardApi`, which owns both.
    D::Target: MerklizedStateHeightPersistence
        + MerklizedStateDataSource<
            SeqTypes,
            RewardMerkleTreeV1,
            { <RewardMerkleTreeV1 as MerkleTreeScheme>::ARITY },
        > + MerklizedStateDataSource<
            SeqTypes,
            RewardMerkleTreeV2,
            { <RewardMerkleTreeV2 as MerkleTreeScheme>::ARITY },
        > + Send
        + Sync,
{
    async fn get_reward_balance(
        &self,
        request: tonic::Request<proto::GetRewardBalanceRequest>,
    ) -> Result<tonic::Response<proto::RewardBalanceResponse>, tonic::Status> {
        let request = request.into_inner();
        let address = required(request.address, "address")?;
        let height = required(request.height, "height")?;
        let balance = <Self as v1::RewardApi>::get_reward_balance(self, height, address)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::RewardBalanceResponse {
            balance: balance.to_string(),
        }))
    }

    async fn get_latest_reward_balance(
        &self,
        request: tonic::Request<proto::GetLatestRewardBalanceRequest>,
    ) -> Result<tonic::Response<proto::RewardBalanceResponse>, tonic::Status> {
        let address = required(request.into_inner().address, "address")?;
        let balance = <Self as v1::RewardApi>::get_latest_reward_balance(self, address)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::RewardBalanceResponse {
            balance: balance.to_string(),
        }))
    }

    async fn get_reward_account_proof(
        &self,
        request: tonic::Request<proto::GetRewardAccountProofRequest>,
    ) -> Result<tonic::Response<proto::RewardAccountProofResponse>, tonic::Status> {
        let request = request.into_inner();
        let address = required(request.address, "address")?;
        let height = required(request.height, "height")?;
        let query = <Self as v1::RewardApi>::get_reward_account_proof(self, height, address)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(reward_query_to_proto(query)))
    }

    async fn get_latest_reward_account_proof(
        &self,
        request: tonic::Request<proto::GetLatestRewardAccountProofRequest>,
    ) -> Result<tonic::Response<proto::RewardAccountProofResponse>, tonic::Status> {
        let address = required(request.into_inner().address, "address")?;
        let query = <Self as v1::RewardApi>::get_latest_reward_account_proof(self, address)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(reward_query_to_proto(query)))
    }

    async fn get_reward_claim_input(
        &self,
        request: tonic::Request<proto::GetRewardClaimInputRequest>,
    ) -> Result<tonic::Response<proto::RewardClaimInputResponse>, tonic::Status> {
        let request = request.into_inner();
        let address = required(request.address, "address")?;
        let height = required(request.height, "height")?;
        let input = <Self as v1::RewardApi>::get_reward_claim_input(self, height, address)
            .await
            .map_err(to_status)?;
        let auth_data: alloy::primitives::Bytes = input.auth_data.into();
        Ok(tonic::Response::new(proto::RewardClaimInputResponse {
            lifetime_rewards: input.lifetime_rewards.to_string(),
            auth_data: auth_data.to_string(),
        }))
    }

    async fn get_reward_amounts(
        &self,
        request: tonic::Request<proto::GetRewardAmountsRequest>,
    ) -> Result<tonic::Response<proto::RewardAmountsResponse>, tonic::Status> {
        let request = request.into_inner();
        let height = required(request.height, "height")?;
        let offset = required(request.offset, "offset")?;
        let limit = required(request.limit, "limit")?;
        let amounts = <Self as v1::RewardApi>::get_reward_amounts(self, height, offset, limit)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::RewardAmountsResponse {
            amounts: amounts
                .into_iter()
                .map(|(address, amount)| proto::RewardAmountPair {
                    address: address.to_string(),
                    amount: amount.to_string(),
                })
                .collect(),
        }))
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        request: tonic::Request<proto::GetRewardMerkleTreeV2Request>,
    ) -> Result<tonic::Response<proto::RewardMerkleTreeV2Response>, tonic::Status> {
        let height = required(request.into_inner().height, "height")?;
        let tree = <Self as v1::RewardApi>::get_reward_merkle_tree_v2(self, height)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::RewardMerkleTreeV2Response {
            tree,
        }))
    }

    async fn get_reward_state_path(
        &self,
        request: tonic::Request<proto::GetRewardStatePathRequest>,
    ) -> Result<tonic::Response<proto::MerklePathResponse>, tonic::Status> {
        let request = request.into_inner();
        let key = required(request.key, "key")?;
        let snapshot = snapshot_from_query(request.height, request.commit)?;
        let proof = <Self as v1::RewardApi>::get_reward_state_path_v2(self, snapshot, key)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::MerklePathResponse::from(
            &proof,
        )))
    }
}

/// Build the response for a reward account lookup, keeping the arm the tree answered with.
fn reward_query_to_proto(
    query: espresso_types::v0_4::RewardAccountQueryDataV2,
) -> proto::RewardAccountProofResponse {
    let proof = match query.proof.proof {
        espresso_types::v0_4::RewardMerkleProofV2::Presence(proof) => {
            proto::reward_merkle_proof::Proof::Presence(proto::MerklePathResponse::from(&proof))
        },
        espresso_types::v0_4::RewardMerkleProofV2::Absence(proof) => {
            proto::reward_merkle_proof::Proof::Absence(proto::MerklePathResponse::from(&proof))
        },
    };
    proto::RewardAccountProofResponse {
        balance: query.balance.to_string(),
        proof: Some(proto::RewardAccountProof {
            account: query.proof.account.to_string(),
            proof: Some(proto::RewardMerkleProof { proof: Some(proof) }),
        }),
    }
}

fn snapshot_from_query(
    height: Option<u64>,
    commit: Option<String>,
) -> Result<v1::Snapshot, tonic::Status> {
    match (height, commit) {
        (Some(height), None) => Ok(v1::Snapshot::Height(height)),
        (None, Some(commit)) => Ok(v1::Snapshot::Commit(commit)),
        _ => Err(tonic::Status::invalid_argument(
            "set exactly one of height or commit",
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv6Addr};

    use alloy::primitives::{Address, U256};
    use base64::Engine as _;
    use espresso_types::{PubKey, v0_3::RegisteredValidator};
    use hotshot_query_service::node::{ResourceSyncStatus, SyncStatus, SyncStatusRange};
    use hotshot_types::{
        addr::NetAddr,
        vid::{
            advz::advz_scheme,
            avidm::{AvidMScheme, init_avidm_param},
            avidm_gf2::{AvidmGf2Scheme, init_avidm_gf2_param},
        },
        x25519,
    };
    use jf_advz::VidScheme as _;
    use proto::vid_share_response::Share;

    use super::*;

    /// v1 renders its byte-encoded fields as JSON integer arrays.
    fn json_bytes(value: &serde_json::Value) -> Vec<u8> {
        <Vec<u8> as serde::Deserialize>::deserialize(value).unwrap()
    }

    fn load_vector<T>(path: &str) -> (T, serde_json::Value)
    where
        T: serde::de::DeserializeOwned,
    {
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        (T::deserialize(&json).unwrap(), json)
    }

    /// The reference vectors are the canonical v1 encoding, so comparing the converted header
    /// against them is what makes "the v2 header mirrors v1" a checked claim rather than a
    /// reviewed one. Every representation the conversion picks by hand is pinned here: `0x`
    /// addresses, the hex L1 timestamp, decimal fee and reward amounts, TaggedBase64
    /// commitments, and the base64 namespace table.
    fn reference_header(version: &str) -> (espresso_types::Header, serde_json::Value) {
        let path = format!("../../../data/{version}/header.json");
        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let header: espresso_types::Header = serde_json::from_value(json.clone()).unwrap();
        // v1 headers are stored flat; later versions wrap their fields alongside the version.
        let fields = json.get("fields").cloned().unwrap_or(json);
        (header, fields)
    }

    /// Fails when the proto message and the reference vector disagree about which fields exist,
    /// which value-by-value assertions cannot catch: they only check the fields already declared.
    /// The declared side is read from the descriptor, so a field the proto lacks fails too.
    fn assert_same_fields(message: &str, reference: &serde_json::Value) {
        use prost::Message as _;
        let descriptors =
            prost_types::FileDescriptorSet::decode(espresso_api::FILE_DESCRIPTOR_SET).unwrap();
        let descriptor = descriptors
            .file
            .iter()
            .flat_map(|file| &file.message_type)
            .find(|candidate| candidate.name() == message)
            .unwrap_or_else(|| panic!("no proto message {message}"));
        let declared: std::collections::BTreeSet<&str> = descriptor
            .field
            .iter()
            .map(|field| match field.oneof_index {
                // v1 writes a tagged union as one key naming the union, not one per arm, so an
                // arm is compared under its oneof's name. A `proto3_optional` field is a
                // synthetic one-arm oneof and keeps its own name.
                Some(index) if !field.proto3_optional() => {
                    descriptor.oneof_decl[index as usize].name()
                },
                _ => field.name(),
            })
            .collect();
        let referenced: std::collections::BTreeSet<&str> = reference
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            // `_pd` is how v1 serializes a certificate's `PhantomData`, and carries nothing a
            // client can read, so no proto message mirrors it.
            .filter(|name| *name != "_pd")
            .collect();
        assert_eq!(
            declared, referenced,
            "{message} fields drifted from the reference vector"
        );
    }

    /// Covers the four shapes and all seven arms: every version's vector must select the arm named
    /// after it, and the proto message must carry exactly the fields v1 serializes, so neither a
    /// new protocol version nor a proto edit can add or drop a header field without failing here.
    #[test]
    fn every_header_version_maps_to_its_arm_and_fields() {
        for (version, shape) in [
            ("v1", "HeaderV1"),
            ("v2", "HeaderV1"),
            ("v3", "HeaderV3"),
            ("v4", "HeaderV4"),
            ("v5", "HeaderV5"),
            ("v6", "HeaderV5"),
            ("v7", "HeaderV5"),
        ] {
            let (header, fields) = reference_header(version);
            assert_same_fields(shape, &fields);

            use proto::header_response::Header;
            let converted = proto::HeaderResponse::from(&header).header.unwrap();
            // Every shape repeats these assignments in its own struct literal, so each one is
            // compared against the vector: the reference heights, timestamps and l1_head are
            // distinct, so a field wired to its neighbour fails here.
            macro_rules! assert_shared_fields {
                ($header:expr) => {{
                    let header = $header;
                    assert_eq!(header.height, fields["height"].as_u64().unwrap());
                    assert_eq!(header.timestamp, fields["timestamp"].as_u64().unwrap());
                    assert_eq!(header.l1_head, fields["l1_head"].as_u64().unwrap());
                    assert_eq!(
                        header.payload_commitment,
                        fields["payload_commitment"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.builder_commitment,
                        fields["builder_commitment"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.block_merkle_tree_root,
                        fields["block_merkle_tree_root"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.fee_merkle_tree_root,
                        fields["fee_merkle_tree_root"].as_str().unwrap()
                    );
                    let fee_info = header.fee_info.as_ref().unwrap();
                    assert_eq!(
                        fee_info.account,
                        fields["fee_info"]["account"].as_str().unwrap()
                    );
                    assert_eq!(
                        fee_info.amount,
                        fields["fee_info"]["amount"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.ns_table.as_ref().unwrap().bytes,
                        base64::engine::general_purpose::STANDARD
                            .decode(fields["ns_table"]["bytes"].as_str().unwrap())
                            .unwrap()
                    );
                    assert_eq!(
                        header.l1_finalized.is_some(),
                        !fields["l1_finalized"].is_null()
                    );
                    assert_eq!(
                        header.builder_signature.is_some(),
                        !fields["builder_signature"].is_null()
                    );
                    assert!(header.chain_config.is_some());
                }};
            }
            let arm = match converted {
                Header::V1(header) => {
                    assert_shared_fields!(header);
                    "v1"
                },
                Header::V2(header) => {
                    assert_shared_fields!(header);
                    "v2"
                },
                Header::V3(header) => {
                    assert_shared_fields!(&header);
                    assert_eq!(
                        header.reward_merkle_tree_root,
                        fields["reward_merkle_tree_root"].as_str().unwrap()
                    );
                    "v3"
                },
                Header::V4(header) => {
                    assert_shared_fields!(&header);
                    assert_eq!(
                        header.timestamp_millis,
                        fields["timestamp_millis"].as_u64().unwrap()
                    );
                    assert_eq!(
                        header.total_reward_distributed,
                        fields["total_reward_distributed"].as_str().unwrap()
                    );
                    assert_eq!(
                        header.next_stake_table_hash.as_deref(),
                        fields["next_stake_table_hash"].as_str()
                    );
                    "v4"
                },
                Header::V5(header) => {
                    assert_shared_fields!(&header);
                    "v5"
                },
                Header::V6(header) => {
                    assert_shared_fields!(&header);
                    "v6"
                },
                Header::V7(header) => {
                    assert_shared_fields!(&header);
                    "v7"
                },
            };
            assert_eq!(arm, version, "{version} header selected the {arm} arm");
        }
    }

    #[test]
    fn v6_header_mirrors_the_reference_vector() {
        let (header, fields) = reference_header("v6");
        assert_same_fields("HeaderV5", &fields);
        let proto::HeaderResponse { header: converted } = (&header).into();
        let Some(proto::header_response::Header::V6(converted)) = converted else {
            panic!("a 0.6 header must convert to the V6 arm, got {converted:?}");
        };

        assert_eq!(converted.height, fields["height"].as_u64().unwrap());
        assert_eq!(converted.timestamp, fields["timestamp"].as_u64().unwrap());
        assert_eq!(
            converted.timestamp_millis,
            fields["timestamp_millis"].as_u64().unwrap()
        );
        assert_eq!(converted.l1_head, fields["l1_head"].as_u64().unwrap());
        assert_eq!(
            converted.payload_commitment,
            fields["payload_commitment"].as_str().unwrap()
        );
        assert_eq!(
            converted.builder_commitment,
            fields["builder_commitment"].as_str().unwrap()
        );
        assert_eq!(
            converted.block_merkle_tree_root,
            fields["block_merkle_tree_root"].as_str().unwrap()
        );
        assert_eq!(
            converted.fee_merkle_tree_root,
            fields["fee_merkle_tree_root"].as_str().unwrap()
        );
        assert_eq!(
            converted.reward_merkle_tree_root,
            fields["reward_merkle_tree_root"].as_str().unwrap()
        );
        assert_eq!(
            converted.total_reward_distributed,
            fields["total_reward_distributed"].as_str().unwrap()
        );
        assert_eq!(
            converted.next_stake_table_hash.as_deref(),
            fields["next_stake_table_hash"].as_str()
        );

        let fee_info = converted.fee_info.unwrap();
        assert_eq!(fee_info.account, fields["fee_info"]["account"]);
        assert_eq!(fee_info.amount, fields["fee_info"]["amount"]);

        let l1_finalized = converted.l1_finalized.unwrap();
        assert_eq!(
            l1_finalized.number,
            fields["l1_finalized"]["number"].as_u64().unwrap()
        );
        assert_eq!(l1_finalized.timestamp, fields["l1_finalized"]["timestamp"]);
        assert_eq!(l1_finalized.hash, fields["l1_finalized"]["hash"]);

        let signature = converted.builder_signature.unwrap();
        assert_eq!(signature.r, fields["builder_signature"]["r"]);
        assert_eq!(signature.s, fields["builder_signature"]["s"]);
        assert_eq!(
            signature.v,
            fields["builder_signature"]["v"].as_u64().unwrap() as u32
        );

        // protoJSON base64s the bytes, which is how v1 renders the table too.
        let ns_table = converted.ns_table.unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(&ns_table.bytes),
            fields["ns_table"]["bytes"].as_str().unwrap()
        );

        let config = match converted.chain_config.unwrap().chain_config.unwrap() {
            proto::resolvable_chain_config::ChainConfig::Full(config) => config,
            other => panic!("the reference header carries a full config, got {other:?}"),
        };
        let expected = &fields["chain_config"]["chain_config"]["Left"];
        assert_same_fields("ChainConfig", expected);

        assert_eq!(config.chain_id, expected["chain_id"]);
        assert_eq!(
            config.max_block_size,
            expected["max_block_size"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
        );
        assert_eq!(config.base_fee, expected["base_fee"]);
        assert_eq!(config.fee_recipient, expected["fee_recipient"]);
        assert_eq!(
            config.fee_contract.as_deref(),
            expected["fee_contract"].as_str()
        );
        assert_eq!(
            config.stake_table_contract.as_deref(),
            expected["stake_table_contract"].as_str()
        );

        assert_eq!(converted.leader_counts.len(), 100);
        let expected_counts: Vec<u32> = fields["leader_counts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|count| count.as_u64().unwrap() as u32)
            .collect();
        assert_eq!(converted.leader_counts, expected_counts);
    }

    /// No reference vector carries a commitment-only chain config, so the `Right` arm is checked
    /// here on its own. `resolve` must report absence rather than `commit` hashing an empty config.
    #[test]
    fn commitment_only_chain_config_keeps_the_commitment() {
        let config = espresso_types::v0_3::ChainConfig::default();
        let commitment = config.commit();
        let resolvable = espresso_types::v0_3::ResolvableChainConfig::from(commitment);

        let converted = proto::ResolvableChainConfig::from(resolvable)
            .chain_config
            .unwrap();
        assert_eq!(
            converted,
            proto::resolvable_chain_config::ChainConfig::Commitment(commitment.to_string())
        );
    }

    // A test network only disperses with ADVZ, so the AvidM arms run only here, against the
    // namespaced wrappers the node stores rather than the inner per-namespace shares.
    #[test]
    fn every_vid_share_arm_maps_to_its_own_shape() {
        let payload = b"two namespaces worth of payload bytes, dispersed";
        let weights = [1u32, 1, 1];
        let ns_table = vec![0..24usize, 24..payload.len()];

        let mut advz = advz_scheme(3);
        let share = VidShare::V0(advz.disperse(payload).unwrap().shares.remove(0));
        let Share::V0(advz) = proto::VidShareResponse::try_from(&share)
            .unwrap()
            .share
            .unwrap()
        else {
            panic!("the V0 arm");
        };
        assert!(advz.aggregate_proofs.starts_with("FIELD~"));
        assert!(!advz.evals_proof.unwrap().proof.is_empty());

        let param = init_avidm_param(3).unwrap();
        let (_, mut shares) =
            AvidMScheme::ns_disperse(&param, &weights, payload, ns_table.clone()).unwrap();
        let share = VidShare::V1(shares.remove(0));
        let Share::V1(avidm) = proto::VidShareResponse::try_from(&share)
            .unwrap()
            .share
            .unwrap()
        else {
            panic!("the V1 arm");
        };
        assert_eq!(avidm.ns_lens, [24, 24]);
        assert_eq!(avidm.ns_commits.len(), 2);
        assert!(avidm.ns_commits[0].starts_with("AvidMCommit~"));
        assert_eq!(avidm.content.len(), 2);
        assert!(avidm.content[0].payload.starts_with("FIELD~"));

        let param = init_avidm_gf2_param(3).unwrap();
        let (_, _, mut shares) =
            AvidmGf2Scheme::ns_disperse(&param, &weights, payload, ns_table).unwrap();
        let share = VidShare::V2(shares.remove(0));
        let Share::V2(gf2) = proto::VidShareResponse::try_from(&share)
            .unwrap()
            .share
            .unwrap()
        else {
            panic!("the V2 arm");
        };
        assert_eq!(gf2.namespaces.len(), 2);
        assert!(!gf2.namespaces[0].payload.is_empty());
        assert!(gf2.namespaces[0].mt_proofs[0].starts_with("MERKLE_PROOF~"));
    }

    // No test network registers a validator, so this mapping is only exercised here.
    #[test]
    fn validator_maps_hex_quantities_and_sorts_delegators() {
        let delegator = |byte: u8| Address::from([byte; 20]);
        let key = x25519::Keypair::generated_from_seed_indexed([3; 32], 0)
            .unwrap()
            .public_key();
        let stake = U256::from(1_000_000_000_000_000_000u64);
        let p2p_addr = NetAddr::Inet(IpAddr::V6(Ipv6Addr::LOCALHOST), 9977);
        let registered = RegisteredValidator::<PubKey> {
            account: delegator(0xab),
            stake_table_key: None,
            state_ver_key: None,
            stake,
            commission: 1234,
            delegators: HashMap::from([
                (delegator(0xff), U256::from(10)),
                (delegator(0x01), U256::from(255)),
            ]),
            authenticated: true,
            x25519_key: Some(key),
            p2p_addr: Some(p2p_addr.clone()),
        };

        let proto = proto::Validator::from(registered);

        // serde renders this key in x25519's own base58, so the tagged form is worth pinning.
        let x25519_key = proto.x25519_key.as_deref().unwrap();
        assert_eq!(x25519_key.parse::<x25519::PublicKey>().unwrap(), key);
        assert!(x25519_key.starts_with("X25519_PK~"));
        assert_ne!(
            Some(x25519_key),
            serde_json::to_value(key).unwrap().as_str()
        );
        // v1 serializes the pre-bracketing form, so `to_string` would give `[::1]:9977`.
        assert_eq!(
            proto.p2p_addr.as_deref(),
            serde_json::to_value(&p2p_addr).unwrap().as_str()
        );
        assert_ne!(
            proto.p2p_addr.as_deref(),
            Some(p2p_addr.to_string().as_str())
        );

        assert_eq!(proto.account, "0xabababababababababababababababababababab");
        // v1 serializes a U256 as a hex quantity, where `to_string` would give it in decimal.
        assert_eq!(
            proto.stake,
            serde_json::to_value(stake).unwrap().as_str().unwrap()
        );
        assert_eq!(proto.commission, 1234);
        assert!(proto.authenticated);
        assert_eq!(proto.stake_table_key, None);
        assert_eq!(proto.state_ver_key, None);
        assert_eq!(
            proto
                .delegators
                .iter()
                .map(|delegator| (delegator.account.as_str(), delegator.amount.as_str()))
                .collect::<Vec<_>>(),
            [
                ("0x0101010101010101010101010101010101010101", "0xff"),
                ("0xffffffffffffffffffffffffffffffffffffffff", "0xa"),
            ]
        );
    }

    // `test_node_api_v2_agrees_with_v1` compares a fresh node's sync status, which the query
    // service caches at startup with no ranges, so this match is only exercised here.
    #[test]
    fn sync_status_ranges_keep_their_bounds_and_status() {
        let converted = proto::ResourceSyncStatus::from(ResourceSyncStatus {
            missing: 7,
            ranges: vec![
                SyncStatusRange {
                    start: 0,
                    end: 3,
                    status: SyncStatus::Pruned,
                },
                SyncStatusRange {
                    start: 3,
                    end: 5,
                    status: SyncStatus::Present,
                },
                SyncStatusRange {
                    start: 5,
                    end: 12,
                    status: SyncStatus::Missing,
                },
            ],
        });

        assert_eq!(converted.missing, 7);
        let ranges: Vec<_> = converted
            .ranges
            .iter()
            .map(|range| (range.start, range.end, range.status()))
            .collect();
        assert_eq!(
            ranges,
            [
                (0, 3, proto::SyncStatus::Pruned),
                (3, 5, proto::SyncStatus::Present),
                (5, 12, proto::SyncStatus::Missing),
            ]
        );
    }

    fn custom(status: StatusCode) -> hotshot_query_service::Error {
        hotshot_query_service::Error::Custom {
            message: "boom".into(),
            status,
        }
    }

    // The only tests of the range limits since the query service's own API (and its
    // `test_range_limit`) was deleted: an in-limit range passes, one past the limit is a
    // RangeExceeded, which the HTTP layer serves as a 400.
    #[tokio::test]
    async fn a_stream_ends_at_its_first_error() {
        let items = futures::stream::iter([
            Ok(1),
            Err(tonic::Status::internal("conversion failed")),
            Ok(2),
        ]);
        let delivered: Vec<_> = end_at_first_error(items).collect().await;
        assert_eq!(delivered.len(), 2);
        assert_eq!(delivered[0].as_ref().unwrap(), &1);
        assert!(delivered[1].is_err());
    }

    #[test]
    fn range_at_limit_is_allowed() {
        let limit = small_object_range_limit();
        enforce_range(0, limit, limit).unwrap();
        enforce_range(3, limit + 3, limit).unwrap();
    }

    #[test]
    fn range_past_limit_is_rejected() {
        for limit in [small_object_range_limit(), large_object_range_limit()] {
            let err = enforce_range(0, limit + 1, limit).unwrap_err();
            assert!(matches!(
                err.downcast_ref::<AvailabilityError>(),
                Some(AvailabilityError::RangeExceeded(_))
            ));
        }
    }

    /// No vector carries view-change evidence, an upgrade certificate, a phase-2 certificate or a
    /// signed QC, so those are built here. The aggregate signature must print as the TaggedBase64
    /// form v1's serde emits, so a v2 client can hand it back to v1.
    #[test]
    fn synthesized_certificates_convert_arm_by_arm() {
        use std::marker::PhantomData;

        use committable::Commitment;
        use espresso_types::PubKey;
        use hotshot_types::{
            data::{EpochNumber, ViewChangeEvidence2, ViewNumber},
            simple_certificate::{
                TimeoutCertificate2, UpgradeCertificate, ViewSyncFinalizeCertificate2,
            },
            simple_vote::{TimeoutData2, UpgradeProposalData, ViewSyncFinalizeData2, Vote2Data},
            traits::signature_key::SignatureKey as _,
        };
        use proto::view_change_evidence2::Evidence;

        let (_, private_key) = PubKey::generated_from_seed_indexed([7; 32], 0);
        let signature = PubKey::sign(&private_key, b"vote").unwrap();
        let mut signers = bitvec::vec::BitVec::<usize, bitvec::order::Lsb0>::repeat(false, 4);
        signers.set(1, true);
        signers.set(3, true);

        let timeout = TimeoutCertificate2::<SeqTypes>::new(
            TimeoutData2 {
                view: ViewNumber::new(9),
                epoch: Some(EpochNumber::new(2)),
            },
            Commitment::from_raw([1; 32]),
            ViewNumber::new(9),
            Some((signature.clone(), signers)),
            PhantomData,
        );
        let Some(Evidence::Timeout(converted)) =
            proto::ViewChangeEvidence2::from(&ViewChangeEvidence2::Timeout(timeout)).evidence
        else {
            panic!("a timeout certificate must select the timeout arm");
        };
        let data = converted.data.unwrap();
        assert_eq!(data.view, 9);
        assert_eq!(data.epoch, Some(2));
        assert_eq!(converted.view_number, 9);
        assert_eq!(
            converted.vote_commitment,
            Commitment::<TimeoutData2>::from_raw([1; 32]).to_string()
        );
        let signatures = converted.signatures.unwrap();
        assert_eq!(signatures.signers, Vec::from([false, true, false, true]));
        assert_eq!(signatures.signature, signature.to_string());
        assert!(signatures.signature.starts_with("BLS_SIG~"));
        assert_eq!(
            serde_json::to_value(&signature).unwrap(),
            serde_json::Value::String(signatures.signature)
        );

        let view_sync = ViewSyncFinalizeCertificate2::<SeqTypes>::new(
            ViewSyncFinalizeData2 {
                relay: 3,
                round: ViewNumber::new(10),
                epoch: None,
            },
            Commitment::from_raw([2; 32]),
            ViewNumber::new(10),
            None,
            PhantomData,
        );
        let Some(Evidence::ViewSync(converted)) =
            proto::ViewChangeEvidence2::from(&ViewChangeEvidence2::ViewSync(view_sync)).evidence
        else {
            panic!("a view sync certificate must select the view_sync arm");
        };
        let data = converted.data.unwrap();
        assert_eq!((data.relay, data.round, data.epoch), (3, 10, None));
        assert!(converted.signatures.is_none());

        let upgrade = UpgradeCertificate::<SeqTypes>::new(
            UpgradeProposalData {
                old_version: vbs::version::Version { major: 0, minor: 3 },
                new_version: vbs::version::Version { major: 0, minor: 4 },
                decide_by: ViewNumber::new(20),
                new_version_hash: Vec::from([0xab, 0xcd]),
                old_version_last_view: ViewNumber::new(19),
                new_version_first_view: ViewNumber::new(21),
            },
            Commitment::from_raw([3; 32]),
            ViewNumber::new(15),
            None,
            PhantomData,
        );
        let data = proto::UpgradeCertificate::from(&upgrade).data.unwrap();
        assert_eq!(data.old_version.unwrap().minor, 3);
        assert_eq!(data.new_version.unwrap().minor, 4);
        assert_eq!(data.new_version_hash, Vec::from([0xab, 0xcd]));
        assert_eq!(
            (
                data.decide_by,
                data.old_version_last_view,
                data.new_version_first_view
            ),
            (20, 19, 21)
        );

        let cert2 = Certificate2::<SeqTypes>::new(
            Vote2Data {
                leaf_commit: Commitment::from_raw([4; 32]),
                epoch: EpochNumber::new(5),
                block_number: 77,
            },
            Commitment::from_raw([5; 32]),
            ViewNumber::new(30),
            None,
            PhantomData,
        );
        let converted = proto::Certificate2::from(&cert2);
        let data = converted.data.unwrap();
        assert_eq!(
            data.leaf_commit,
            Commitment::<hotshot_types::data::Leaf2<SeqTypes>>::from_raw([4; 32]).to_string()
        );
        assert_eq!((data.epoch, data.block_number), (5, 77));
        assert_eq!(converted.view_number, 30);
    }

    #[test]
    fn namespace_proofs_mirror_the_reference_vectors() {
        use base64::Engine as _;
        use proto::ns_proof::Proof;
        let b64 = base64::engine::general_purpose::STANDARD;

        fn check_transactions(converted: &proto::NamespaceProofResponse, json: &serde_json::Value) {
            let expected = json["transactions"].as_array().unwrap();
            assert_eq!(converted.transactions.len(), expected.len());
            for (tx, expected) in converted.transactions.iter().zip(expected) {
                assert_eq!(tx.namespace, expected["namespace"].as_u64().unwrap());
                assert_eq!(
                    base64::engine::general_purpose::STANDARD.encode(&tx.payload),
                    expected["payload"].as_str().unwrap()
                );
            }
        }

        let (reference, json): (NamespaceProofQueryData, _) =
            load_vector("../../../data/v3/ns_proof_V0.json");
        assert_same_fields("NamespaceProofResponse", &json);
        let expected = &json["proof"]["V0"];
        assert_same_fields("AdvzNsProof", expected);
        let converted = proto::NamespaceProofResponse::try_from(&reference).unwrap();
        check_transactions(&converted, &json);
        let Some(Proof::V0(advz)) = converted.proof.unwrap().proof else {
            panic!("the ADVZ vector must select the v0 arm");
        };
        assert_eq!(advz.ns_index, json_bytes(&expected["ns_index"]));
        assert_eq!(
            b64.encode(&advz.ns_payload),
            expected["ns_payload"].as_str().unwrap()
        );
        let range_proof = advz.ns_proof.unwrap();
        let expected_proof = &expected["ns_proof"];
        assert_same_fields("LargeRangeProof", expected_proof);
        assert_eq!(range_proof.prefix_elems, expected_proof["prefix_elems"]);
        assert_eq!(range_proof.suffix_elems, expected_proof["suffix_elems"]);
        assert_eq!(
            range_proof.prefix_bytes,
            json_bytes(&expected_proof["prefix_bytes"])
        );
        assert_eq!(
            range_proof.suffix_bytes,
            json_bytes(&expected_proof["suffix_bytes"])
        );

        for (path, arm) in [
            ("../../../data/v4/ns_proof_V1.json", "V1"),
            ("../../../data/v6/ns_proof_V2.json", "V2"),
        ] {
            let (reference, json): (NamespaceProofQueryData, _) = load_vector(path);
            let expected = &json["proof"][arm];
            assert_same_fields("NsProofPayload", expected);
            let converted = proto::NamespaceProofResponse::try_from(&reference).unwrap();
            check_transactions(&converted, &json);
            let payload = match (arm, converted.proof.unwrap().proof) {
                ("V1", Some(Proof::V1(payload))) | ("V2", Some(Proof::V2(payload))) => payload,
                (arm, other) => panic!("the {arm} vector selected the wrong arm: {other:?}"),
            };
            assert_eq!(payload.ns_index, expected["ns_index"].as_u64().unwrap());
            assert_eq!(
                b64.encode(&payload.ns_payload),
                expected["ns_payload"].as_str().unwrap()
            );
            assert_eq!(payload.ns_proof, expected["ns_proof"]);
        }
    }

    /// No test can build this proof, since it needs a malicious dispersal and the vid crate keeps
    /// the items for one private. Deserializing the JSON v1 would serve pins the two field names
    /// the conversion reads, so an upstream rename fails here rather than as a 500.
    #[test]
    fn bad_encoding_namespace_proof_mirrors_its_v1_rendering() {
        // ark-serialize writes a `Vec` as a little-endian u64 length followed by its elements, so
        // eight zero bytes is the empty vector both fields hold here.
        let empty = tagged_base64::TaggedBase64::new("FIELD", &0u64.to_le_bytes())
            .unwrap()
            .to_string();
        let commitment = reference_header("v3").1["payload_commitment"]
            .as_str()
            .unwrap()
            .to_string();
        let merkle_proof: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v3/ns_proof_V1.json").unwrap(),
        )
        .unwrap();
        let merkle_proof = merkle_proof["proof"]["V1"]["ns_proof"].as_str().unwrap();

        let json = serde_json::json!({
            "ns_index": 1,
            "ns_commit": commitment,
            "ns_mt_proof": merkle_proof,
            "ns_proof": { "recovered_poly": empty, "raw_shares": empty },
        });
        let reference: espresso_types::v0_3::AvidMIncorrectEncodingNsProof =
            serde_json::from_value(json.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&reference).unwrap(),
            json,
            "v1 no longer renders the proof the way this test claims"
        );

        let converted = proto::AvidmBadEncodingNsProof::try_from(&reference).unwrap();
        assert_eq!(converted.ns_index, 1);
        assert_eq!(converted.ns_commit, commitment);
        assert_eq!(converted.ns_mt_proof, merkle_proof);
        let inner = converted.ns_proof.unwrap();
        assert_eq!(inner.recovered_poly, empty);
        assert_eq!(inner.raw_shares, empty);
    }

    /// Neither vector carries signatures, so the signature mapping is pinned only by its types.
    #[test]
    fn state_certs_mirror_the_reference_vectors() {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v3/state_cert.json").unwrap(),
        )
        .unwrap();
        let reference: espresso_types::v0_3::StateCertQueryDataV1<SeqTypes> =
            serde_json::from_value(json.clone()).unwrap();
        assert_same_fields("StateCertV1Response", &json);
        let converted = proto::StateCertV1Response::from(&reference);
        assert_eq!(converted.epoch, json["epoch"].as_u64().unwrap());
        assert_eq!(converted.light_client_state, json["light_client_state"]);
        assert_eq!(
            converted.next_stake_table_state,
            json["next_stake_table_state"]
        );
        assert_eq!(
            converted.signatures.len(),
            json["signatures"].as_array().unwrap().len()
        );

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v4/state_cert.json").unwrap(),
        )
        .unwrap();
        let reference: espresso_types::v0_4::StateCertQueryDataV2<SeqTypes> =
            serde_json::from_value(json.clone()).unwrap();
        assert_same_fields("StateCertV2Response", &json);
        let converted = proto::StateCertV2Response::from(&reference);
        assert_eq!(converted.epoch, json["epoch"].as_u64().unwrap());
        assert_eq!(converted.light_client_state, json["light_client_state"]);
        assert_eq!(converted.auth_root, json["auth_root"]);
        assert_eq!(
            converted.signatures.len(),
            json["signatures"].as_array().unwrap().len()
        );
    }

    /// Every proof in the vector is AvidM. The ADVZ arm is covered by
    /// `advz_transaction_proof_mirrors_its_v1_rendering`.
    #[test]
    fn transaction_with_proof_mirrors_the_reference_vector() {
        use base64::Engine as _;
        use proto::tx_proof::Proof;

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/transaction_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: Vec<TransactionWithProofQueryData<SeqTypes>> =
            serde_json::from_value(json.clone()).unwrap();
        let first = &json[0];
        assert_same_fields("TransactionWithProofResponse", first);

        let converted = proto::TransactionWithProofResponse::try_from(&reference[0]).unwrap();
        assert_eq!(converted.hash, first["hash"]);
        assert_eq!(converted.index, first["index"].as_u64().unwrap());
        assert_eq!(converted.block_hash, first["block_hash"]);
        assert_eq!(
            converted.block_height,
            first["block_height"].as_u64().unwrap()
        );
        assert_eq!(converted.namespace, first["namespace"].as_u64().unwrap());
        assert_eq!(
            u64::from(converted.pos_in_namespace),
            first["pos_in_namespace"].as_u64().unwrap()
        );
        let transaction = converted.transaction.unwrap();
        assert_eq!(
            transaction.namespace,
            first["transaction"]["namespace"].as_u64().unwrap()
        );
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(&transaction.payload),
            first["transaction"]["payload"].as_str().unwrap()
        );

        let Some(Proof::V1(proof)) = converted.proof.unwrap().proof else {
            panic!("the AvidM vector must select the v1 proof arm");
        };
        let expected = &first["proof"]["V1"];
        assert_same_fields("AvidmTxProof", expected);
        assert_eq!(proof.tx_index, json_bytes(&expected["tx_index"]));
        let ns_proof = proof.ns_proof.unwrap();
        let expected_ns = &expected["ns_proof"];
        assert_same_fields("NsProofPayload", expected_ns);
        assert_eq!(ns_proof.ns_index, expected_ns["ns_index"].as_u64().unwrap());
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(&ns_proof.ns_payload),
            expected_ns["ns_payload"].as_str().unwrap()
        );
        assert_eq!(ns_proof.ns_proof, expected_ns["ns_proof"]);
    }

    #[test]
    fn vid_common_mirrors_the_reference_vectors() {
        use proto::vid_common_response::Common;

        let (reference, json): (VidCommonQueryData<SeqTypes>, _) =
            load_vector("../../../data/v1/vid_common_v0.json");
        assert_same_fields("VidCommonResponse", &json);
        assert_same_fields("AdvzCommon", &json["common"]["V0"]);
        let converted = proto::VidCommonResponse::try_from(&reference).unwrap();
        assert_eq!(converted.height, json["height"].as_u64().unwrap());
        assert_eq!(converted.block_hash, json["block_hash"]);
        assert_eq!(converted.payload_hash, json["payload_hash"]);
        let Some(Common::V0(advz)) = converted.common else {
            panic!("the ADVZ vector must select the v0 arm");
        };
        let expected = &json["common"]["V0"];
        assert_eq!(advz.poly_commits, expected["poly_commits"]);
        assert_eq!(advz.all_evals_digest, expected["all_evals_digest"]);
        assert_eq!(
            u64::from(advz.payload_byte_len),
            expected["payload_byte_len"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(advz.num_storage_nodes),
            expected["num_storage_nodes"].as_u64().unwrap()
        );
        assert_eq!(
            u64::from(advz.multiplicity),
            expected["multiplicity"].as_u64().unwrap()
        );

        let (reference, json): (VidCommonQueryData<SeqTypes>, _) =
            load_vector("../../../data/v1/vid_common_v1.json");
        assert_same_fields("AvidmCommon", &json["common"]["V1"]);
        let Some(Common::V1(avidm)) = proto::VidCommonResponse::try_from(&reference)
            .unwrap()
            .common
        else {
            panic!("the AvidM vector must select the v1 arm");
        };
        let expected = &json["common"]["V1"];
        assert_eq!(
            avidm.total_weights,
            expected["total_weights"].as_u64().unwrap()
        );
        assert_eq!(
            avidm.recovery_threshold,
            expected["recovery_threshold"].as_u64().unwrap()
        );

        let (reference, json): (VidCommonQueryData<SeqTypes>, _) =
            load_vector("../../../data/v2/vid_common_v2.json");
        assert_same_fields("AvidmGf2Common", &json["common"]["V2"]);
        let Some(Common::V2(gf2)) = proto::VidCommonResponse::try_from(&reference)
            .unwrap()
            .common
        else {
            panic!("the AvidmGf2 vector must select the v2 arm");
        };
        let expected = &json["common"]["V2"];
        let param = gf2.param.unwrap();
        assert_eq!(
            param.total_weights,
            expected["param"]["total_weights"].as_u64().unwrap()
        );
        assert_eq!(
            param.recovery_threshold,
            expected["param"]["recovery_threshold"].as_u64().unwrap()
        );
        let expected_commits: Vec<&str> = expected["ns_commits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|commit| commit.as_str().unwrap())
            .collect();
        assert_eq!(gf2.ns_commits, expected_commits);
        let expected_lens: Vec<u64> = expected["ns_lens"]
            .as_array()
            .unwrap()
            .iter()
            .map(|len| len.as_u64().unwrap())
            .collect();
        assert_eq!(gf2.ns_lens, expected_lens);
    }

    /// The payload bytes and namespace table are compared through base64, which is how v1 renders
    /// them and how protoJSON renders `bytes`.
    #[test]
    fn block_and_payload_mirror_the_reference_vectors() {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD;

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/block_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: BlockQueryData<SeqTypes> = serde_json::from_value(json.clone()).unwrap();
        let block = proto::BlockResponse::from(&reference);
        assert_same_fields("BlockResponse", &json);
        assert_eq!(block.hash, json["hash"]);
        assert_eq!(block.size, json["size"].as_u64().unwrap());
        assert_eq!(
            block.num_transactions,
            json["num_transactions"].as_u64().unwrap()
        );
        let Some(proto::header_response::Header::V1(header)) = block.header.unwrap().header else {
            panic!("an unwrapped 0.1-shaped header must convert to the V1 arm");
        };
        assert_eq!(header.height, json["header"]["height"].as_u64().unwrap());
        let payload = block.payload.unwrap();
        assert_eq!(
            b64.encode(&payload.raw_payload),
            json["payload"]["raw_payload"].as_str().unwrap()
        );
        assert_eq!(
            b64.encode(&payload.ns_table.unwrap().bytes),
            json["payload"]["ns_table"]["bytes"].as_str().unwrap()
        );

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/payload_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: PayloadQueryData<SeqTypes> = serde_json::from_value(json.clone()).unwrap();
        let payload = proto::PayloadResponse::from(&reference);
        assert_same_fields("PayloadResponse", &json);
        assert_eq!(payload.height, json["height"].as_u64().unwrap());
        assert_eq!(payload.block_hash, json["block_hash"]);
        assert_eq!(payload.hash, json["hash"]);
        assert_eq!(payload.size, json["size"].as_u64().unwrap());
        let data = payload.data.unwrap();
        assert_eq!(
            b64.encode(&data.raw_payload),
            json["data"]["raw_payload"].as_str().unwrap()
        );
        assert_eq!(
            b64.encode(&data.ns_table.unwrap().bytes),
            json["data"]["ns_table"]["bytes"].as_str().unwrap()
        );
    }

    /// The per-namespace map is the one summary field that comes from the payload, so no header
    /// vector covers it.
    #[test]
    fn block_summary_mirrors_its_v1_rendering() {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/block_query_data.json").unwrap(),
        )
        .unwrap();
        let block: BlockQueryData<SeqTypes> = serde_json::from_value(json).unwrap();
        let summary = BlockSummaryQueryData::from(block);
        let expected = serde_json::to_value(&summary).unwrap();
        let converted = proto::BlockSummaryResponse::from(&summary);

        assert_same_fields("BlockSummaryResponse", &expected);
        assert_eq!(converted.hash, expected["hash"]);
        assert_eq!(converted.size, expected["size"].as_u64().unwrap());
        assert_eq!(
            converted.num_transactions,
            expected["num_transactions"].as_u64().unwrap()
        );

        // protoJSON writes a map key as a string whatever its proto type, which is also what
        // serde_json does with v1's `NamespaceId` keys.
        let expected_namespaces = expected["namespaces"].as_object().unwrap();
        assert!(
            expected_namespaces.len() > 1,
            "the vector should span several namespaces"
        );
        assert_eq!(converted.namespaces.len(), expected_namespaces.len());
        let mut counted = 0;
        for (namespace, expected_info) in expected_namespaces {
            let info = &converted.namespaces[&namespace.parse::<u64>().unwrap()];
            assert_eq!(
                info.num_transactions,
                expected_info["num_transactions"].as_u64().unwrap()
            );
            assert_eq!(info.size, expected_info["size"].as_u64().unwrap());
            counted += info.num_transactions;
        }
        assert_eq!(counted, converted.num_transactions);
    }

    #[test]
    fn leaf_mirrors_the_reference_vector() {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v3/leaf_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: LeafQueryData<SeqTypes> = serde_json::from_value(json.clone()).unwrap();
        let converted = proto::LeafResponse::from(&reference);

        assert_same_fields("Leaf2", &json["leaf"]);
        assert_same_fields("QuorumCertificate2", &json["qc"]);

        let leaf = converted.leaf.unwrap();
        let expected = &json["leaf"];
        assert_eq!(leaf.view_number, expected["view_number"].as_u64().unwrap());
        assert_eq!(leaf.parent_commitment, expected["parent_commitment"]);
        assert_eq!(leaf.with_epoch, expected["with_epoch"].as_bool().unwrap());
        assert!(leaf.next_epoch_justify_qc.is_none());
        assert!(leaf.upgrade_certificate.is_none());
        assert!(leaf.view_change_evidence.is_none());
        assert!(leaf.next_drb_result.is_empty());

        // The v3-era vector embeds a 0.1-shaped header: twelve flat fields, no version wrapper,
        // no reward root. Header versions and leaf versions moved independently.
        let header_json = &expected["block_header"];
        assert!(
            header_json.get("fields").is_none(),
            "vector header grew a version wrapper; update the expected arm"
        );
        let Some(proto::header_response::Header::V1(header)) = leaf.block_header.unwrap().header
        else {
            panic!("an unwrapped 0.1-shaped header must convert to the V1 arm");
        };
        assert_eq!(header.height, header_json["height"].as_u64().unwrap());

        // `LeafQueryData` deserializes through `new`, which drops the payload the JSON carries,
        // so the conversion must report none.
        assert!(leaf.block_payload.is_none());

        let justify = leaf.justify_qc.unwrap();
        let expected_justify = &expected["justify_qc"];
        assert_eq!(
            justify.view_number,
            expected_justify["view_number"].as_u64().unwrap()
        );
        assert_eq!(justify.vote_commitment, expected_justify["vote_commitment"]);
        assert_eq!(
            justify.data.as_ref().unwrap().leaf_commit,
            expected_justify["data"]["leaf_commit"]
        );

        let qc = converted.qc.unwrap();
        let expected_qc = &json["qc"];
        assert_eq!(qc.view_number, expected_qc["view_number"].as_u64().unwrap());
        assert_eq!(qc.vote_commitment, expected_qc["vote_commitment"]);
        let data = qc.data.unwrap();
        assert_eq!(data.leaf_commit, expected_qc["data"]["leaf_commit"]);
        assert_eq!(data.epoch, expected_qc["data"]["epoch"].as_u64());
        assert_eq!(
            data.block_number,
            expected_qc["data"]["block_number"].as_u64()
        );
        // The vector's certificates are unsigned, so absence must map to absence.
        assert!(qc.signatures.is_none());
    }

    /// A 0.1 header has no reward tree, no millisecond timestamp and no leader counts, so the V1
    /// message must not carry them. This is the case where the `reward_merkle_tree_root`
    /// accessor would have reported the commitment of an empty tree instead of nothing.
    #[test]
    fn v1_header_mirrors_the_reference_vector() {
        let (header, fields) = reference_header("v1");
        assert_same_fields("HeaderV1", &fields);
        let proto::HeaderResponse { header: converted } = (&header).into();
        let Some(proto::header_response::Header::V1(converted)) = converted else {
            panic!("a 0.1 header must convert to the V1 arm, got {converted:?}");
        };

        assert_eq!(converted.height, fields["height"].as_u64().unwrap());
        assert_eq!(converted.timestamp, fields["timestamp"].as_u64().unwrap());
        assert_eq!(converted.l1_head, fields["l1_head"].as_u64().unwrap());
        assert_eq!(
            converted.payload_commitment,
            fields["payload_commitment"].as_str().unwrap()
        );
        let fee_info = converted.fee_info.unwrap();
        assert_eq!(fee_info.account, fields["fee_info"]["account"]);
        assert_eq!(fee_info.amount, fields["fee_info"]["amount"]);
    }

    #[test]
    fn ranges_within_limits_are_allowed() {
        let ranges = validate_ranges(vec![0..5, 10..12], 100).unwrap();
        assert_eq!(ranges, [0..5, 10..12]);
        validate_ranges(vec![], 100).unwrap();
    }

    #[test]
    fn oversized_or_empty_ranges_are_rejected() {
        // More heights than the object limit.
        let err = validate_ranges(vec![0..60, 100..160], 100).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<AvailabilityError>(),
            Some(AvailabilityError::RangeExceeded(_))
        ));

        // Many single-height ranges are bounded by the object limit like anything else.
        let many = (0..101u64).map(|i| i * 2..i * 2 + 1).collect();
        let err = validate_ranges(many, 100).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<AvailabilityError>(),
            Some(AvailabilityError::RangeExceeded(_))
        ));

        // An empty range would otherwise reach the query builder as a contradictory bound.
        #[allow(clippy::single_range_in_vec_init)]
        let err = validate_ranges(vec![5..5], 100).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<AvailabilityError>(),
            Some(AvailabilityError::BadRequest(_))
        ));

        // A range wide enough to overflow the running total must not wrap past the limit.
        let err = validate_ranges(vec![0..100, 0..u64::MAX], 100).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<AvailabilityError>(),
            Some(AvailabilityError::BadRequest(_) | AvailabilityError::RangeExceeded(_))
        ));
    }

    #[test]
    fn unordered_ranges_are_rejected() {
        // Touching is fine: a run split at a chunk boundary arrives this way.
        validate_ranges(vec![0..5, 5..7], 100).unwrap();
        for ranges in [vec![5..7, 0..5], vec![0..5, 3..7]] {
            let err = validate_ranges(ranges, 100).unwrap_err();
            assert!(matches!(
                err.downcast_ref::<AvailabilityError>(),
                Some(AvailabilityError::BadRequest(_))
            ));
        }
    }

    // Tripwire: the enforced and advertised limits come from `hotshot_query_service`'s
    // `Options` defaults. If a dependency change moves them, this fails so the new bound is
    // adopted deliberately rather than silently.
    #[test]
    fn range_limits_track_known_defaults() {
        assert_eq!(small_object_range_limit(), 500);
        assert_eq!(large_object_range_limit(), 100);
        assert_eq!(node_window_limit(), 500);
    }

    // Regression: the light-client trait methods used to map query-service errors through
    // `anyhow::anyhow!("{err}")`, erasing the status; every 400/404 became a 500.
    #[test]
    fn lc_error_preserves_bad_request() {
        let err = lc_error(custom(StatusCode::BAD_REQUEST));
        assert!(matches!(
            err.downcast_ref::<AvailabilityError>(),
            Some(AvailabilityError::BadRequest(_))
        ));
    }

    #[test]
    fn lc_error_preserves_not_found() {
        let err = lc_error(custom(StatusCode::NOT_FOUND));
        assert!(matches!(
            err.downcast_ref::<AvailabilityError>(),
            Some(AvailabilityError::NotFound(_))
        ));
    }

    #[test]
    fn lc_error_other_statuses_stay_internal() {
        let err = lc_error(custom(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(err.downcast_ref::<AvailabilityError>().is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn advz_transaction_proof_mirrors_its_v1_rendering() {
        use hotshot_query_service::availability::QueryablePayload;
        use hotshot_types::{
            data::VidCommon,
            traits::{EncodeBytes, block_contents::BlockPayload},
            vid::advz::advz_scheme,
        };
        use jf_advz::VidScheme;
        use proto::tx_proof::Proof;

        // No reference vector carries an ADVZ transaction proof, so build one the way a V0 node
        // did and compare against v1's JSON. The accessor-backed fields are the real check. The
        // range proofs are read from that same JSON, so their loop only pins that the keys exist
        // and that an absent proof stays absent.
        let namespace = espresso_types::NamespaceId::from(7_u32);
        let transactions: Vec<_> = (1_u8..=3)
            .map(|byte| espresso_types::Transaction::new(namespace, vec![byte; 8]))
            .collect();
        let (payload, _) = espresso_types::Payload::from_transactions(
            transactions,
            &Default::default(),
            &Default::default(),
        )
        .await
        .unwrap();
        let common = VidCommon::V0(advz_scheme(10).disperse(payload.encode()).unwrap().common);
        let index = payload.iter(payload.ns_table()).next().unwrap();
        let (_, proof) = espresso_types::TxProof::new(&index, &payload, &common).unwrap();
        let rendered = serde_json::to_value(&proof).unwrap();
        let rendered = &rendered["V0"];

        let Some(Proof::V0(converted)) = proto::TxProof::try_from(&proof).unwrap().proof else {
            panic!("an ADVZ common yields the V0 arm");
        };
        assert_eq!(converted.tx_index, json_bytes(&rendered["tx_index"]));
        assert_eq!(
            converted.payload_num_txs,
            json_bytes(&rendered["payload_num_txs"])
        );
        assert_eq!(
            converted.payload_tx_table_entries,
            json_bytes(&rendered["payload_tx_table_entries"])
        );
        for (proof, key) in [
            (converted.payload_proof_num_txs, "payload_proof_num_txs"),
            (
                converted.payload_proof_tx_table_entries,
                "payload_proof_tx_table_entries",
            ),
            (converted.payload_proof_tx, "payload_proof_tx"),
        ] {
            let Some(proof) = proof else {
                assert!(
                    rendered[key].is_null(),
                    "{key} is present in v1's rendering"
                );
                continue;
            };
            assert_eq!(
                proof.proofs,
                rendered[key]["proofs"].as_str().unwrap(),
                "{key}"
            );
            assert_eq!(
                proof.prefix_bytes,
                json_bytes(&rendered[key]["prefix_bytes"]),
                "{key}"
            );
            assert_eq!(
                proof.suffix_bytes,
                json_bytes(&rendered[key]["suffix_bytes"]),
                "{key}"
            );
        }
    }

    /// A node under test registers no runtime config, so `test_v2_api_agrees_with_v1` only ever
    /// reaches the 404. This covers the mapping itself: every field the proto promises comes from
    /// the matching `PublicNodeConfig` field, with identity values distinct enough that a mapping
    /// crossing two of them fails.
    #[tokio::test]
    async fn runtime_config_mirrors_public_node_config() {
        use proto::config_service_server::ConfigService as _;

        use crate::options::{
            Identity, PublicNodeConfig,
            tests::{parse_options_with, test_genesis},
        };

        struct UnusedDataSource;

        impl HotShotConfigDataSource for UnusedDataSource {
            async fn get_config(&self) -> espresso_types::config::PublicNetworkConfig {
                unreachable!("the runtime config is served from the state, not the data source")
            }
        }

        let opt = parse_options_with(&[
            "--config-peers",
            "https://peer1.test,https://peer2.test",
            "--cliquenet-bind-address",
            "[2001:db8::1]:9999",
            "--",
            "http",
            "--port",
            "24000",
            "--",
            "query",
            "--peers",
            "https://query1.test,https://query2.test",
            "--light-client-db-num-connections",
            "7",
            "--light-client-db-num-leaves",
            "11",
            "--light-client-db-num-stake-tables",
            "13",
            "--",
            "config",
        ]);
        let mut cfg = PublicNodeConfig::new(&opt, &opt.modules(), &test_genesis());
        cfg.identity = Identity {
            node_name: Some("node-name".into()),
            node_description: Some("node-description".into()),
            company_name: Some("company-name".into()),
            company_website: Some("https://company.test/".parse().unwrap()),
            country_code: Some("DE".into()),
            latitude: Some(1.5),
            longitude: Some(-2.5),
            operating_system: Some("operating-system".into()),
            node_type: Some("node-type".into()),
            network_type: Some("network-type".into()),
            icon_14x14_1x: Some("https://icons.test/14/1".parse().unwrap()),
            icon_14x14_2x: Some("https://icons.test/14/2".parse().unwrap()),
            icon_14x14_3x: Some("https://icons.test/14/3".parse().unwrap()),
            icon_24x24_1x: Some("https://icons.test/24/1".parse().unwrap()),
            icon_24x24_2x: Some("https://icons.test/24/2".parse().unwrap()),
            icon_24x24_3x: Some("https://icons.test/24/3".parse().unwrap()),
        };

        let state = NodeApiStateImpl::new(std::sync::Arc::new(UnusedDataSource))
            .with_public_node_config(Some(cfg.clone()));
        let runtime = state
            .get_runtime_config(tonic::Request::new(proto::GetRuntimeConfigRequest {}))
            .await
            .unwrap()
            .into_inner();

        fn strings<T: ToString>(values: &[T]) -> Vec<String> {
            values.iter().map(ToString::to_string).collect()
        }

        assert_eq!(
            runtime,
            proto::RuntimeConfigResponse {
                is_da: cfg.is_da,
                identity: Some(proto::NodeIdentity {
                    node_name: Some("node-name".into()),
                    node_description: Some("node-description".into()),
                    company_name: Some("company-name".into()),
                    company_website: Some("https://company.test/".into()),
                    country_code: Some("DE".into()),
                    latitude: Some(1.5),
                    longitude: Some(-2.5),
                    operating_system: Some("operating-system".into()),
                    node_type: Some("node-type".into()),
                    network_type: Some("network-type".into()),
                    icon_14x14_1x: Some("https://icons.test/14/1".into()),
                    icon_14x14_2x: Some("https://icons.test/14/2".into()),
                    icon_14x14_3x: Some("https://icons.test/14/3".into()),
                    icon_24x24_1x: Some("https://icons.test/24/1".into()),
                    icon_24x24_2x: Some("https://icons.test/24/2".into()),
                    icon_24x24_3x: Some("https://icons.test/24/3".into()),
                }),
                storage: Some(proto::NodeStorage {
                    backend: proto::StorageBackend::FsDefault as i32,
                    // Not pinned to `None`: the default backend parses an empty argv, which
                    // still reads ESPRESSO_NODE_STORAGE_PATH.
                    fs: cfg.storage.fs.as_ref().map(|fs| proto::FsStorage {
                        path: fs.path.display().to_string(),
                        consensus_view_retention: fs.consensus_view_retention,
                    }),
                    sql: None,
                }),
                genesis_file: cfg.genesis_file.to_string(),
                public_api_url: cfg.public_api_url.as_ref().map(ToString::to_string),
                builder_urls: strings(&cfg.builder_urls),
                state_relay_server_url: cfg.state_relay_server_url.to_string(),
                state_peers: strings(&cfg.state_peers),
                config_peers: strings(cfg.config_peers.as_deref().unwrap()),
                orchestrator_url: cfg.orchestrator_url.to_string(),
                cdn_endpoint: cfg.cdn_endpoint.clone(),
                cliquenet_bind_address: cfg.cliquenet_bind_address.unbracketed_string(),
                cliquenet_advertise_address: cfg
                    .cliquenet_advertise_address
                    .as_ref()
                    .map(|addr| addr.unbracketed_string()),
                libp2p_bind_address: cfg.libp2p_bind_address.clone(),
                libp2p_advertise_address: cfg.libp2p_advertise_address.clone(),
                libp2p_bootstrap_nodes: cfg
                    .libp2p_bootstrap_nodes
                    .as_deref()
                    .map(strings)
                    .unwrap_or_default(),
                l1_provider_count: cfg.l1_provider_count as u64,
                l1_ws_provider_count: cfg.l1_ws_provider_count as u64,
                modules: Some(proto::ApiModules {
                    http: Some(proto::HttpModule {
                        port: 24000,
                        max_connections: None,
                        tonic_port: None,
                    }),
                    query: Some(proto::QueryModule {
                        peers: vec![
                            "https://query1.test/".to_string(),
                            "https://query2.test/".to_string(),
                        ],
                        light_client: Some(proto::LightClientModuleOptions {
                            num_stake_tables_in_memory: cfg
                                .modules
                                .query
                                .as_ref()
                                .unwrap()
                                .light_client
                                .num_stake_tables_in_memory
                                as u64,
                        }),
                        // Three same-typed fields whose defaults are 5/100/100, so the flags above
                        // give each a distinct value: a crossed pair would pass otherwise.
                        light_client_db: Some(proto::LightClientDbOptions {
                            num_connections: 7,
                            num_leaves: 11,
                            num_stake_tables: 13,
                            lc_path: None,
                        }),
                    }),
                    submit: false,
                    status: false,
                    catchup: false,
                    config: true,
                    hotshot_events: false,
                    explorer: false,
                    light_client: false,
                }),
            }
        );
        // IPv6 because that is the only case where NetAddr's Display and its serde impl
        // disagree, and v1 goes through serde.
        assert_eq!(runtime.config_peers.len(), 2);
        assert_eq!(runtime.cliquenet_bind_address, "2001:db8::1:9999");
    }

    /// A TestNetwork leaves all of these at their defaults, so the live parity test compares them
    /// `None` to `None` and empty to empty. Exercised here with values instead.
    #[test]
    fn hotshot_config_renders_the_fields_a_test_network_leaves_empty() {
        use espresso_types::config::PublicNetworkConfig;
        use hotshot_types::{
            PeerConfig, VersionedDaCommittee,
            network::{BuilderType, CombinedNetworkConfig, Libp2pConfig, NetworkConfig},
        };

        let peer_id = libp2p::PeerId::random();
        let multiaddr: libp2p::Multiaddr = "/ip4/10.0.0.1/tcp/1769".parse().unwrap();
        let committee_member = PeerConfig::<SeqTypes>::test_default();

        let mut network_config = NetworkConfig::<SeqTypes> {
            commit_sha: "deadbeef".to_string(),
            cdn_marshal_address: Some("marshal.test:8083".to_string()),
            builder: BuilderType::Random,
            libp2p_config: Some(Libp2pConfig {
                bootstrap_nodes: vec![(peer_id, multiaddr.clone())],
            }),
            combined_network_config: Some(CombinedNetworkConfig {
                delay_duration: Duration::from_millis(1500),
            }),
            ..Default::default()
        };
        network_config.config.da_committees = vec![VersionedDaCommittee {
            start_version: vbs::version::Version { major: 0, minor: 6 },
            start_epoch: 10,
            committee: vec![committee_member.clone()],
        }];

        let served: proto::HotshotConfigResponse = PublicNetworkConfig::from(network_config).into();

        assert_eq!(served.commit_sha, "deadbeef");
        assert_eq!(
            served.cdn_marshal_address.as_deref(),
            Some("marshal.test:8083")
        );
        assert_eq!(served.builder, proto::BuilderType::Random as i32);
        assert_eq!(
            served.libp2p_config,
            Some(proto::Libp2pNetworkConfig {
                bootstrap_nodes: vec![proto::Libp2pBootstrapNode {
                    peer_id: peer_id.to_string(),
                    multiaddr: multiaddr.to_string(),
                }],
            })
        );
        assert_eq!(
            served.combined_network_config,
            Some(proto::CombinedNetworkConfig {
                delay_duration_ms: 1500,
            })
        );
        // The version renders as v1's `version_ser` writes it, not as `Debug`.
        let da_committee = &served.da_committees[0];
        assert_eq!(served.da_committees.len(), 1);
        assert_eq!(da_committee.start_version, "0.6");
        assert_eq!(da_committee.start_epoch, 10);
        assert_eq!(
            da_committee.committee,
            vec![proto::PeerConfig::from(committee_member)]
        );
    }

    // Postgres only, as in `options.rs`: storage-sql under embedded-db needs a `--path` that is
    // irrelevant here.
    #[cfg(not(feature = "embedded-db"))]
    #[tokio::test]
    async fn sql_storage_settings_are_served_in_full() {
        use proto::config_service_server::ConfigService as _;

        use crate::options::{
            PublicNodeConfig,
            tests::{parse_options_with, test_genesis},
        };

        struct UnusedDataSource;

        impl HotShotConfigDataSource for UnusedDataSource {
            async fn get_config(&self) -> espresso_types::config::PublicNetworkConfig {
                unreachable!("the runtime config is served from the state, not the data source")
            }
        }

        let opt = parse_options_with(&[
            "--cliquenet-bind-address",
            "127.0.0.1:9999",
            "--",
            "storage-sql",
            "--prune",
            "--pruning-threshold",
            "1000000000000",
        ]);
        let cfg = PublicNodeConfig::new(&opt, &opt.modules(), &test_genesis());
        let sql = cfg.storage.sql.clone().expect("storage-sql was configured");

        let state = NodeApiStateImpl::new(std::sync::Arc::new(UnusedDataSource))
            .with_public_node_config(Some(cfg));
        let storage = state
            .get_runtime_config(tonic::Request::new(proto::GetRuntimeConfigRequest {}))
            .await
            .unwrap()
            .into_inner()
            .storage
            .expect("the runtime config always reports a backend");

        assert_eq!(storage.backend, proto::StorageBackend::Sql as i32);
        assert_eq!(storage.fs, None);
        // Compared against the source, not against `sql.clone().into()`, which would assert the
        // mapping against itself. v1 serves the durations as `{secs, nanos}`.
        assert_eq!(
            storage.sql,
            Some(proto::SqlStorage {
                prune: true,
                archive: sql.archive,
                lightweight: sql.lightweight,
                disable_proactive_fetching: sql.disable_proactive_fetching,
                fetch_rate_limit: sql.fetch_rate_limit.map(|limit| limit as u64),
                active_fetch_delay_ms: sql.active_fetch_delay.map(|delay| delay.as_millis() as u64),
                chunk_fetch_delay_ms: sql.chunk_fetch_delay.map(|delay| delay.as_millis() as u64),
                sync_status_chunk_size: sql.sync_status_chunk_size.map(|size| size as u64),
                sync_status_ttl_ms: sql.sync_status_ttl.map(|ttl| ttl.as_millis() as u64),
                proactive_scan_chunk_size: sql.proactive_scan_chunk_size.map(|size| size as u64),
                proactive_scan_interval_ms: sql
                    .proactive_scan_interval
                    .map(|interval| interval.as_millis() as u64),
                idle_connection_timeout_ms: sql.idle_connection_timeout.as_millis() as u64,
                connection_timeout_ms: sql.connection_timeout.as_millis() as u64,
                slow_statement_threshold_ms: sql.slow_statement_threshold.as_millis() as u64,
                statement_timeout_ms: sql.statement_timeout.as_millis() as u64,
                min_connections: sql.min_connections,
                max_connections: sql.max_connections,
                query_min_connections: sql.query_min_connections,
                query_max_connections: sql.query_max_connections,
                pruning: Some(proto::PruningConfig {
                    pruning_threshold: Some(1000000000000),
                    minimum_retention_ms: sql
                        .pruning
                        .minimum_retention
                        .map(|retention| retention.as_millis() as u64),
                    target_retention_ms: sql
                        .pruning
                        .target_retention
                        .map(|retention| retention.as_millis() as u64),
                    batch_size: sql.pruning.batch_size,
                    max_usage: sql.pruning.max_usage.map(u32::from),
                    interval_ms: sql
                        .pruning
                        .interval
                        .map(|interval| interval.as_millis() as u64),
                    pages: sql.pruning.pages,
                }),
                consensus_pruning: Some(proto::ConsensusPruningConfig {
                    target_retention: sql.consensus_pruning.target_retention,
                    minimum_retention: sql.consensus_pruning.minimum_retention,
                    target_usage: sql.consensus_pruning.target_usage,
                }),
            })
        );
        // The defaults these come from are non-zero, so the comparisons above are not vacuous.
        let served = storage.sql.unwrap();
        assert!(served.statement_timeout_ms > 0);
        assert!(served.consensus_pruning.unwrap().target_retention > 0);
    }

    fn assert_matches_v1_rendering<E, I, T, const ARITY: usize>(
        proof: &jf_merkle_tree_compat::prelude::MerkleProof<E, I, T, ARITY>,
    ) where
        E: jf_merkle_tree_compat::Element
            + ark_serialize::CanonicalSerialize
            + ark_serialize::CanonicalDeserialize,
        I: jf_merkle_tree_compat::Index
            + ark_serialize::CanonicalSerialize
            + ark_serialize::CanonicalDeserialize,
        T: jf_merkle_tree_compat::NodeValue,
    {
        let expected = serde_json::to_value(proof).unwrap();
        let converted = proto::MerklePathResponse::from(proof);

        assert_eq!(converted.pos, expected["pos"].as_str().unwrap());
        let expected_path = expected["proof"].as_array().unwrap();
        assert_eq!(converted.proof.len(), expected_path.len());
        assert!(
            !expected_path.is_empty(),
            "a path of no nodes would assert nothing"
        );
        for (node, expected) in converted.proof.iter().zip(expected_path) {
            assert_merkle_node(node, expected);
        }
    }

    #[test]
    fn block_state_path_mirrors_its_v1_rendering() {
        use committable::Committable as _;
        use jf_merkle_tree_compat::MerkleTreeScheme as _;

        let commitment = reference_header("v3").0.commit();
        let tree = espresso_types::BlockMerkleTree::from_elems(Some(32), [commitment, commitment])
            .unwrap();
        // The block tree is light-weight: every leaf but the frontier is forgotten, so only the
        // last index can be looked up here.
        let (_, proof) = tree.lookup(1).expect_ok().unwrap();
        assert_matches_v1_rendering(&proof);
    }

    /// The fee tree indexes by account and branches 256 ways where the block tree indexes by
    /// height and branches 3, so it exercises the conversion over a different `Index` and a
    /// different arity.
    #[test]
    fn fee_state_path_mirrors_its_v1_rendering() {
        use jf_merkle_tree_compat::MerkleTreeScheme as _;

        let account = espresso_types::FeeAccount::default();
        let tree = espresso_types::FeeMerkleTree::from_kv_set(
            20,
            [(account, espresso_types::FeeAmount::from(123u64))],
        )
        .unwrap();
        let (_, proof) = tree.lookup(account).expect_ok().unwrap();
        assert_matches_v1_rendering(&proof);
    }

    fn assert_merkle_node(node: &proto::AdvzMerkleNode, expected: &serde_json::Value) {
        use proto::advz_merkle_node::Node;

        match node.node.as_ref().unwrap() {
            Node::Empty(_) => assert_eq!(expected, "Empty"),
            Node::Branch(branch) => {
                let expected = &expected["Branch"];
                assert_eq!(branch.value, expected["value"].as_str().unwrap());
                let children = expected["children"].as_array().unwrap();
                assert_eq!(branch.children.len(), children.len());
                for (child, expected) in branch.children.iter().zip(children) {
                    assert_merkle_node(child, expected);
                }
            },
            Node::Leaf(leaf) => {
                let expected = &expected["Leaf"];
                assert_eq!(leaf.value, expected["value"].as_str().unwrap());
                assert_eq!(leaf.pos, expected["pos"].as_str().unwrap());
                assert_eq!(leaf.elem, expected["elem"].as_str().unwrap());
            },
            Node::ForgottenSubtree(forgotten) => {
                let expected = &expected["ForgettenSubtree"];
                assert_eq!(forgotten.value, expected["value"].as_str().unwrap());
            },
        }
    }

    #[test]
    fn snapshot_query_takes_exactly_one_selector() {
        assert!(matches!(
            snapshot_from_query(Some(7), None).unwrap(),
            v1::Snapshot::Height(7)
        ));
        assert!(matches!(
            snapshot_from_query(None, Some("MERKLE_COMM~x".to_owned())).unwrap(),
            v1::Snapshot::Commit(_)
        ));
        for (height, commit) in [(None, None), (Some(7), Some("MERKLE_COMM~x".to_owned()))] {
            assert_eq!(
                snapshot_from_query(height, commit).unwrap_err().code(),
                tonic::Code::InvalidArgument
            );
        }
    }
}
