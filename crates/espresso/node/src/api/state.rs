//! Implementations of the v1 API traits and the v2 tonic service traits, both reading the one
//! data source this type wraps.

use std::{
    ops::{Bound, Deref},
    time::Duration,
};

use alloy::primitives::utils::format_ether;
use async_trait::async_trait;
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
use futures::{StreamExt as _, join, stream::BoxStream};
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
    types::HeightIndexed as _,
};
use hotshot_types::{
    data::VidShare,
    utils::{epoch_from_block_number, root_block_in_epoch},
    vid::avidm::AvidMShare,
};
use jf_merkle_tree_compat::prelude::{
    MerkleProof as InternalMerkleProof, MerkleProof as JfMerkleProof,
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

// Range limits for list endpoints, read from `hotshot_query_service`'s `Options` (their only
// remaining declaration) so a dependency bump that changes the defaults changes enforcement too.
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
    D::Target: AvailabilityDataSource<SeqTypes> + Send + Sync,
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
        Ok(self.data_source.node_public_keys().await)
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
        // A node that has not seen a view computes 0/0. protoJSON cannot encode a non-finite
        // double, `serde_json` would write `null`, and the generated deserializer rejects that.
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
        let keys = self.data_source.node_public_keys().await;
        Ok(tonic::Response::new(proto::NodeKeysResponse {
            eth_account: keys.eth_account.map(|account| format!("{account:#x}")),
            consensus_key: Some(proto::BlsPublicKey {
                key: keys.consensus_key.to_string(),
            }),
            state_ver_key: Some(proto::SchnorrPublicKey {
                key: keys.state_ver_key.to_string(),
            }),
            x25519_key: keys.x25519_key.as_ref().map(ToString::to_string),
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
        let ds = &*self.data_source;
        ds.get_oldest_block().await
    }

    async fn get_oldest_leaf(&self) -> anyhow::Result<Option<Self::Leaf>> {
        let ds = &*self.data_source;
        ds.get_oldest_leaf().await
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
impl<N, P, D> SubmitDataSourceErased
    for hotshot_query_service::data_source::ExtensibleDataSource<D, crate::api::ApiState<N, P>>
where
    N: hotshot_types::traits::network::ConnectedNetwork<espresso_types::PubKey>,
    P: espresso_types::v0::traits::SequencerPersistence,
    D: Send + Sync,
{
    async fn submit_erased(&self, tx: espresso_types::Transaction) -> anyhow::Result<()> {
        <Self as SubmitDataSource<N, P>>::submit(self, tx).await
    }
}

// Bare mode (no query/status API) has no `ExtensibleDataSource` wrapper: the app state is
// `ApiState<N, P>` directly, so it needs its own erased forwarding impl.
#[async_trait]
impl<N, P> SubmitDataSourceErased for crate::api::ApiState<N, P>
where
    N: hotshot_types::traits::network::ConnectedNetwork<espresso_types::PubKey>,
    P: espresso_types::v0::traits::SequencerPersistence,
{
    async fn submit_erased(&self, tx: espresso_types::Transaction) -> anyhow::Result<()> {
        <Self as SubmitDataSource<N, P>>::submit(self, tx).await
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

// Bare mode (no query/status API) has no `ExtensibleDataSource` wrapper: the app state is
// `ApiState<N, P>` directly, so it needs its own erased forwarding impl.
#[async_trait]
impl<N, P> StateSignatureDataSourceErased for crate::api::ApiState<N, P>
where
    N: hotshot_types::traits::network::ConnectedNetwork<espresso_types::PubKey>,
    P: espresso_types::v0::traits::SequencerPersistence,
{
    async fn get_state_signature_erased(
        &self,
        height: u64,
    ) -> Option<hotshot_types::light_client::LCV3StateSignatureRequestBody> {
        <Self as StateSignatureDataSource<N>>::get_state_signature(self, height).await
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
        + Send
        + Sync,
    for<'a> <D::Target as hotshot_query_service::data_source::VersionedDataSource>::ReadOnly<'a>:
        hotshot_query_service::data_source::storage::NodeStorage<SeqTypes>,
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
        .map_err(|err| anyhow::anyhow!("{err}"))
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
            .map_err(|err| anyhow::anyhow!("{err}"))
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
            .map_err(|err| not_found(format!("failed to get total supply. err={err:#}")))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(status: StatusCode) -> hotshot_query_service::Error {
        hotshot_query_service::Error::Custom {
            message: "boom".into(),
            status,
        }
    }

    // The only tests of the range limits since the query service's own API (and its
    // `test_range_limit`) was deleted: an in-limit range passes, one past the limit is a
    // RangeExceeded, which the HTTP layer serves as a 400.
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
}
