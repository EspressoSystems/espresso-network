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
    traits::EncodeBytes as _,
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
            .map_err(to_status)?
            .hotshot_config()
            .into_hotshot_config();
        // Destructured without `..` so that a field added to HotShotConfig fails to compile here
        // instead of becoming a parameter v2 silently never serves.
        let hotshot_types::HotShotConfig {
            start_threshold: (start_threshold_numerator, start_threshold_denominator),
            num_nodes_with_stake,
            known_nodes_with_stake: _,
            known_da_nodes: _,
            da_committees: _,
            da_staked_committee_size,
            fixed_leader_for_gpuvid: _,
            next_view_timeout,
            view_sync_timeout,
            num_bootstrap: _,
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
        Ok(tonic::Response::new(proto::HotshotConfigResponse {
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
        }))
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
                let (name, value) = entry.split_once('=').unwrap_or((entry.as_str(), ""));
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
        let identity = config.identity;
        Ok(tonic::Response::new(proto::RuntimeConfigResponse {
            is_da: config.is_da,
            identity: Some(proto::NodeIdentity {
                node_name: identity.node_name,
                node_description: identity.node_description,
                company_name: identity.company_name,
                company_website: identity.company_website.map(|url| url.to_string()),
                country_code: identity.country_code,
                latitude: identity.latitude,
                longitude: identity.longitude,
                operating_system: identity.operating_system,
                node_type: identity.node_type,
                network_type: identity.network_type,
            }),
            storage_backend: match config.storage.backend {
                crate::options::StorageBackend::Sql => proto::StorageBackend::Sql,
                crate::options::StorageBackend::Fs => proto::StorageBackend::Fs,
                crate::options::StorageBackend::FsDefault => proto::StorageBackend::FsDefault,
            }
            .into(),
            genesis_file: config.genesis_file.to_string(),
            public_api_url: config.public_api_url.map(|url| url.to_string()),
            builder_urls: config
                .builder_urls
                .iter()
                .map(ToString::to_string)
                .collect(),
            state_relay_server_url: config.state_relay_server_url.to_string(),
            state_peers: config.state_peers.iter().map(ToString::to_string).collect(),
            config_peers: config
                .config_peers
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect(),
            orchestrator_url: config.orchestrator_url.to_string(),
            cdn_endpoint: config.cdn_endpoint,
            cliquenet_bind_address: config.cliquenet_bind_address.to_string(),
            cliquenet_advertise_address: config
                .cliquenet_advertise_address
                .map(|addr| addr.to_string()),
            libp2p_bind_address: config.libp2p_bind_address,
            libp2p_advertise_address: config.libp2p_advertise_address,
            libp2p_bootstrap_nodes: config
                .libp2p_bootstrap_nodes
                .unwrap_or_default()
                .iter()
                .map(ToString::to_string)
                .collect(),
            l1_provider_count: config.l1_provider_count as u64,
            l1_ws_provider_count: config.l1_ws_provider_count as u64,
        }))
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
        let bytes = <Self as v1::NodeApi>::payload_size(self, from, to, namespace)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::PayloadSizeResponse { bytes }))
    }

    async fn get_sync_status(
        &self,
        _request: tonic::Request<proto::GetSyncStatusRequest>,
    ) -> Result<tonic::Response<proto::SyncStatusResponse>, tonic::Status> {
        let status = <Self as v1::NodeApi>::sync_status(self)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(proto::SyncStatusResponse {
            blocks: Some(resource_sync_status(status.blocks)),
            leaves: Some(resource_sync_status(status.leaves)),
            vid_common: Some(resource_sync_status(status.vid_common)),
            pruned_height: status.pruned_height.map(|height| height as u64),
        }))
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
}

fn resource_sync_status(
    status: hotshot_query_service::node::ResourceSyncStatus,
) -> proto::ResourceSyncStatus {
    use hotshot_query_service::node::SyncStatus;

    proto::ResourceSyncStatus {
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
                started_at: migration.started_at.to_rfc3339(),
                completed_at: migration.completed_at.map(|time| time.to_rfc3339()),
                last_offset: migration.last_offset,
            })
            .collect();
        Ok(tonic::Response::new(proto::MigrationStatusResponse {
            migrations,
        }))
    }
}

/// v1 renders addresses through `ethers_core::H160`, which is `0x`-prefixed lowercase hex.
/// `FeeAccount`'s own `Display` drops the prefix, so it cannot be used here.
fn address_to_proto(address: &alloy::primitives::Address) -> String {
    format!("{address:#x}")
}

fn chain_config_to_proto(
    chain_config: espresso_types::v0_3::ResolvableChainConfig,
) -> proto::ResolvableChainConfig {
    use proto::resolvable_chain_config::ChainConfig;

    // A header carries either the config or only its commitment, and `resolve` is what tells
    // them apart: `commit` would hash a full config rather than report its absence.
    let resolved = match chain_config.resolve() {
        Some(config) => ChainConfig::Full(proto::ChainConfig {
            chain_id: config.chain_id.to_string(),
            max_block_size: *config.max_block_size,
            base_fee: config.base_fee.to_string(),
            fee_contract: config.fee_contract.as_ref().map(address_to_proto),
            fee_recipient: address_to_proto(&config.fee_recipient.0),
            stake_table_contract: config.stake_table_contract.as_ref().map(address_to_proto),
        }),
        None => ChainConfig::Commitment(chain_config.commit().to_string()),
    };
    proto::ResolvableChainConfig {
        chain_config: Some(resolved),
    }
}

fn l1_finalized_to_proto(info: Option<espresso_types::L1BlockInfo>) -> Option<proto::L1BlockInfo> {
    info.map(|info| proto::L1BlockInfo {
        number: info.number,
        // v1 hex-encodes this U256; a decimal string would not round-trip for its clients.
        timestamp: format!("{:#x}", info.timestamp),
        hash: format!("{:#x}", info.hash),
    })
}

fn builder_signature_to_proto(header: &HsHeader<SeqTypes>) -> Option<proto::BuilderSignature> {
    // The accessor smooths `Option` into a `Vec` across versions; empty means unsigned.
    header
        .builder_signature()
        .first()
        .map(|signature| proto::BuilderSignature {
            r: format!("{:#x}", signature.r()),
            s: format!("{:#x}", signature.s()),
            // alloy reports parity as a bool; v1 renders it as the recovery id, and its
            // deserializer accepts nothing but 27 or 28.
            v: if signature.v() { 28 } else { 27 },
        })
}

fn fee_info_to_proto(header: &HsHeader<SeqTypes>) -> Option<proto::FeeInfo> {
    header.fee_info().first().map(|fee| proto::FeeInfo {
        account: address_to_proto(&fee.account.0),
        amount: fee.amount.to_string(),
    })
}

/// The proto message per protocol version, mirroring the `Header` enum. Versions sharing a shape
/// share a message, so only the arm distinguishes 0.1 from 0.2 and 0.5 from 0.6.
fn header_to_proto(header: &HsHeader<SeqTypes>) -> proto::HeaderResponse {
    use espresso_types::Header;

    let chain_config = chain_config_to_proto(header.chain_config());
    let l1_finalized = l1_finalized_to_proto(header.l1_finalized());
    let builder_signature = builder_signature_to_proto(header);
    let fee_info = fee_info_to_proto(header);
    let ns_table = Some(proto::NsTable {
        bytes: header.ns_table().encode().to_vec(),
    });
    let payload_commitment = header.payload_commitment().to_string();
    let builder_commitment = header.builder_commitment().to_string();
    let block_merkle_tree_root = header.block_merkle_tree_root().to_string();
    let fee_merkle_tree_root = header.fee_merkle_tree_root().to_string();

    let shape_v1 = || proto::HeaderV1 {
        chain_config: Some(chain_config.clone()),
        height: header.height(),
        timestamp: header.timestamp_internal(),
        l1_head: header.l1_head(),
        l1_finalized: l1_finalized.clone(),
        payload_commitment: payload_commitment.clone(),
        builder_commitment: builder_commitment.clone(),
        ns_table: ns_table.clone(),
        block_merkle_tree_root: block_merkle_tree_root.clone(),
        fee_merkle_tree_root: fee_merkle_tree_root.clone(),
        fee_info: fee_info.clone(),
        builder_signature: builder_signature.clone(),
    };

    // Only 0.3 uses the first reward tree, so its root is read from the `Left` arm; every later
    // version reads the `Right` one. 0.1 and 0.2 have no reward root at all, and the accessor
    // would hand back the commitment of an empty tree rather than say so.
    let reward_merkle_tree_root = || match header.reward_merkle_tree_root() {
        either::Either::Left(root) => root.to_string(),
        either::Either::Right(root) => root.to_string(),
    };

    let shape_v3 = || proto::HeaderV3 {
        chain_config: Some(chain_config.clone()),
        height: header.height(),
        timestamp: header.timestamp_internal(),
        l1_head: header.l1_head(),
        l1_finalized: l1_finalized.clone(),
        payload_commitment: payload_commitment.clone(),
        builder_commitment: builder_commitment.clone(),
        ns_table: ns_table.clone(),
        block_merkle_tree_root: block_merkle_tree_root.clone(),
        fee_merkle_tree_root: fee_merkle_tree_root.clone(),
        fee_info: fee_info.clone(),
        builder_signature: builder_signature.clone(),
        reward_merkle_tree_root: reward_merkle_tree_root(),
    };

    let shape_v4 = || proto::HeaderV4 {
        chain_config: Some(chain_config.clone()),
        height: header.height(),
        timestamp: header.timestamp_internal(),
        timestamp_millis: header.timestamp_millis_internal(),
        l1_head: header.l1_head(),
        l1_finalized: l1_finalized.clone(),
        payload_commitment: payload_commitment.clone(),
        builder_commitment: builder_commitment.clone(),
        ns_table: ns_table.clone(),
        block_merkle_tree_root: block_merkle_tree_root.clone(),
        fee_merkle_tree_root: fee_merkle_tree_root.clone(),
        fee_info: fee_info.clone(),
        builder_signature: builder_signature.clone(),
        reward_merkle_tree_root: reward_merkle_tree_root(),
        total_reward_distributed: header
            .total_reward_distributed()
            .expect("0.4 and later headers carry total_reward_distributed")
            .to_string(),
        next_stake_table_hash: header.next_stake_table_hash().map(|hash| hash.to_string()),
    };

    let shape_v5 = || proto::HeaderV5 {
        chain_config: Some(chain_config.clone()),
        height: header.height(),
        timestamp: header.timestamp_internal(),
        timestamp_millis: header.timestamp_millis_internal(),
        l1_head: header.l1_head(),
        l1_finalized: l1_finalized.clone(),
        payload_commitment: payload_commitment.clone(),
        builder_commitment: builder_commitment.clone(),
        ns_table: ns_table.clone(),
        block_merkle_tree_root: block_merkle_tree_root.clone(),
        fee_merkle_tree_root: fee_merkle_tree_root.clone(),
        fee_info: fee_info.clone(),
        builder_signature: builder_signature.clone(),
        reward_merkle_tree_root: reward_merkle_tree_root(),
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
    };

    let header = match header {
        Header::V1(_) => proto::header_response::Header::V1(shape_v1()),
        Header::V2(_) => proto::header_response::Header::V2(shape_v1()),
        Header::V3(_) => proto::header_response::Header::V3(shape_v3()),
        Header::V4(_) => proto::header_response::Header::V4(shape_v4()),
        Header::V5(_) => proto::header_response::Header::V5(shape_v5()),
        Header::V6(_) => proto::header_response::Header::V6(shape_v5()),
    };
    proto::HeaderResponse {
        header: Some(header),
    }
}

/// The three fields every certificate shares regardless of what was voted on.
fn certificate_common<V, T>(
    cert: &hotshot_types::simple_certificate::SimpleCertificate<SeqTypes, V, T>,
) -> (String, u64, Option<proto::QuorumSignatures>)
where
    V: hotshot_types::simple_vote::Voteable<SeqTypes>,
    T: hotshot_types::simple_certificate::Threshold<SeqTypes>,
{
    // v1 serializes the bitvec crate's memory layout; the API publishes who signed instead.
    let signatures = cert
        .signatures
        .as_ref()
        .map(|(signature, signers)| proto::QuorumSignatures {
            signature: signature.to_string(),
            signers: signers.iter().by_vals().collect(),
        });
    (
        cert.vote_commitment().to_string(),
        cert.view_number.u64(),
        signatures,
    )
}

fn quorum_data_to_proto(
    data: &hotshot_types::simple_vote::QuorumData2<SeqTypes>,
) -> proto::QuorumData2 {
    proto::QuorumData2 {
        leaf_commit: data.leaf_commit.to_string(),
        epoch: data.epoch.map(|epoch| epoch.u64()),
        block_number: data.block_number,
    }
}

fn quorum_certificate_to_proto(
    cert: &hotshot_types::simple_certificate::QuorumCertificate2<SeqTypes>,
) -> proto::QuorumCertificate2 {
    let (vote_commitment, view_number, signatures) = certificate_common(cert);
    proto::QuorumCertificate2 {
        data: Some(quorum_data_to_proto(&cert.data)),
        vote_commitment,
        view_number,
        signatures,
    }
}

/// The next-epoch QC votes on the same data as a QC, so it shares the message.
fn next_epoch_certificate_to_proto(
    cert: &hotshot_types::simple_certificate::NextEpochQuorumCertificate2<SeqTypes>,
) -> proto::QuorumCertificate2 {
    let (vote_commitment, view_number, signatures) = certificate_common(cert);
    proto::QuorumCertificate2 {
        data: Some(quorum_data_to_proto(&cert.data)),
        vote_commitment,
        view_number,
        signatures,
    }
}

fn certificate2_to_proto(cert: &Certificate2<SeqTypes>) -> proto::Certificate2 {
    let (vote_commitment, view_number, signatures) = certificate_common(cert);
    proto::Certificate2 {
        data: Some(proto::Vote2Data {
            leaf_commit: cert.data.leaf_commit.to_string(),
            epoch: cert.data.epoch.u64(),
            block_number: cert.data.block_number,
        }),
        vote_commitment,
        view_number,
        signatures,
    }
}

fn upgrade_certificate_to_proto(
    cert: &hotshot_types::simple_certificate::UpgradeCertificate<SeqTypes>,
) -> proto::UpgradeCertificate {
    let version = |version: vbs::version::Version| proto::Version {
        major: u32::from(version.major),
        minor: u32::from(version.minor),
    };
    let (vote_commitment, view_number, signatures) = certificate_common(cert);
    proto::UpgradeCertificate {
        data: Some(proto::UpgradeProposalData {
            old_version: Some(version(cert.data.old_version)),
            new_version: Some(version(cert.data.new_version)),
            decide_by: cert.data.decide_by.u64(),
            new_version_hash: cert.data.new_version_hash.clone(),
            old_version_last_view: cert.data.old_version_last_view.u64(),
            new_version_first_view: cert.data.new_version_first_view.u64(),
        }),
        vote_commitment,
        view_number,
        signatures,
    }
}

fn view_change_evidence_to_proto(
    evidence: &hotshot_types::data::ViewChangeEvidence2<SeqTypes>,
) -> proto::ViewChangeEvidence2 {
    use hotshot_types::data::ViewChangeEvidence2;
    use proto::view_change_evidence2::Evidence;

    let evidence = match evidence {
        ViewChangeEvidence2::Timeout(cert) => {
            let (vote_commitment, view_number, signatures) = certificate_common(cert);
            Evidence::Timeout(proto::TimeoutCertificate2 {
                data: Some(proto::TimeoutData2 {
                    view: cert.data.view.u64(),
                    epoch: cert.data.epoch.map(|epoch| epoch.u64()),
                }),
                vote_commitment,
                view_number,
                signatures,
            })
        },
        ViewChangeEvidence2::ViewSync(cert) => {
            let (vote_commitment, view_number, signatures) = certificate_common(cert);
            Evidence::ViewSync(proto::ViewSyncFinalizeCertificate2 {
                data: Some(proto::ViewSyncFinalizeData2 {
                    relay: cert.data.relay,
                    round: cert.data.round.u64(),
                    epoch: cert.data.epoch.map(|epoch| epoch.u64()),
                }),
                vote_commitment,
                view_number,
                signatures,
            })
        },
    };
    proto::ViewChangeEvidence2 {
        evidence: Some(evidence),
    }
}

fn payload_to_proto(payload: &espresso_types::Payload) -> proto::Payload {
    proto::Payload {
        raw_payload: payload.encode().to_vec(),
        ns_table: Some(proto::NsTable {
            bytes: payload.ns_table().encode().to_vec(),
        }),
    }
}

fn leaf_to_proto(leaf: &hotshot_types::data::Leaf2<SeqTypes>) -> proto::Leaf2 {
    proto::Leaf2 {
        view_number: leaf.view_number().u64(),
        justify_qc: Some(quorum_certificate_to_proto(&leaf.justify_qc())),
        next_epoch_justify_qc: leaf
            .next_epoch_justify_qc()
            .as_ref()
            .map(next_epoch_certificate_to_proto),
        parent_commitment: leaf.parent_commitment().to_string(),
        block_header: Some(header_to_proto(leaf.block_header())),
        upgrade_certificate: leaf
            .upgrade_certificate()
            .as_ref()
            .map(upgrade_certificate_to_proto),
        block_payload: leaf.block_payload().as_ref().map(payload_to_proto),
        view_change_evidence: leaf
            .view_change_evidence
            .as_ref()
            .map(view_change_evidence_to_proto),
        next_drb_result: leaf.next_drb_result.map(|result| result.to_vec()),
        with_epoch: leaf.with_epoch,
    }
}

fn leaf_query_data_to_proto(leaf: &LeafQueryData<SeqTypes>) -> proto::LeafResponse {
    proto::LeafResponse {
        leaf: Some(leaf_to_proto(&leaf.leaf)),
        qc: Some(quorum_certificate_to_proto(&leaf.qc)),
    }
}

fn block_to_proto(block: &BlockQueryData<SeqTypes>) -> proto::BlockResponse {
    proto::BlockResponse {
        header: Some(header_to_proto(block.header())),
        payload: Some(payload_to_proto(block.payload())),
        hash: block.hash().to_string(),
        size: block.size(),
        num_transactions: block.num_transactions(),
    }
}

fn payload_query_data_to_proto(payload: &PayloadQueryData<SeqTypes>) -> proto::PayloadResponse {
    proto::PayloadResponse {
        height: payload.height,
        block_hash: payload.block_hash().to_string(),
        hash: payload.hash().to_string(),
        size: payload.size(),
        data: Some(payload_to_proto(payload.data())),
    }
}

fn vid_common_to_proto(common: &VidCommonQueryData<SeqTypes>) -> proto::VidCommonResponse {
    use hotshot_types::data::VidCommon;
    use proto::vid_common_response::Common;

    let arm = match common.common() {
        VidCommon::V0(advz) => {
            // jellyfish keeps ADVZ's fields private; v1's encoding is the one public view of them.
            let value = serde_json::to_value(advz).expect("ADVZ common serializes");
            let text = |key: &str| value[key].as_str().unwrap_or_default().to_string();
            let small = |key: &str| value[key].as_u64().unwrap_or_default() as u32;
            Common::V0(proto::AdvzCommon {
                poly_commits: text("poly_commits"),
                all_evals_digest: text("all_evals_digest"),
                payload_byte_len: small("payload_byte_len"),
                num_storage_nodes: small("num_storage_nodes"),
                multiplicity: small("multiplicity"),
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
                .map(|commit| commit.to_string())
                .collect(),
            ns_lens: namespaced.ns_lens.iter().map(|len| *len as u64).collect(),
        }),
    };
    proto::VidCommonResponse {
        height: common.height,
        block_hash: common.block_hash().to_string(),
        payload_hash: common.payload_hash().to_string(),
        common: Some(arm),
    }
}

fn ns_proof_payload_to_proto(
    ns_index: usize,
    ns_payload: &[u8],
    ns_proof: &impl std::fmt::Display,
) -> proto::NsProofPayload {
    proto::NsProofPayload {
        ns_index: ns_index as u64,
        ns_payload: ns_payload.to_vec(),
        ns_proof: ns_proof.to_string(),
    }
}

/// v1 renders its byte-encoded fields as JSON integer arrays; this reads one back.
fn json_bytes(value: &serde_json::Value) -> Vec<u8> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| item.as_u64().unwrap_or_default() as u8)
                .collect()
        })
        .unwrap_or_default()
}

fn small_range_proof_from_json(value: &serde_json::Value) -> Option<proto::SmallRangeProof> {
    value.as_object().map(|_| proto::SmallRangeProof {
        proofs: value["proofs"].as_str().unwrap_or_default().to_string(),
        prefix_bytes: json_bytes(&value["prefix_bytes"]),
        suffix_bytes: json_bytes(&value["suffix_bytes"]),
    })
}

fn tx_proof_to_proto(proof: &espresso_types::TxProof) -> proto::TxProof {
    use espresso_types::TxProof;
    use proto::tx_proof::Proof;

    let arm = match proof {
        TxProof::V0(advz) => {
            // Its fields are jellyfish range proofs or v1 byte encodings, none reachable from
            // here except through v1's own JSON.
            let value = serde_json::to_value(advz).expect("ADVZ tx proof serializes");
            Proof::V0(proto::AdvzTxProof {
                tx_index: json_bytes(&value["tx_index"]),
                payload_num_txs: json_bytes(&value["payload_num_txs"]),
                payload_proof_num_txs: small_range_proof_from_json(&value["payload_proof_num_txs"]),
                payload_tx_table_entries: json_bytes(&value["payload_tx_table_entries"]),
                payload_proof_tx_table_entries: small_range_proof_from_json(
                    &value["payload_proof_tx_table_entries"],
                ),
                payload_proof_tx: small_range_proof_from_json(&value["payload_proof_tx"]),
            })
        },
        TxProof::V1(avidm) => {
            let ns_proof = &avidm.ns_proof().0;
            Proof::V1(proto::AvidmTxProof {
                tx_index: avidm.tx_index().to_bytes().to_vec(),
                ns_proof: Some(ns_proof_payload_to_proto(
                    ns_proof.ns_index,
                    &ns_proof.ns_payload,
                    &ns_proof.ns_proof,
                )),
            })
        },
        TxProof::V2(gf2) => {
            let ns_proof = &gf2.ns_proof().0;
            Proof::V2(proto::AvidmGf2TxProof {
                tx_index: gf2.tx_index().to_bytes().to_vec(),
                ns_proof: Some(ns_proof_payload_to_proto(
                    ns_proof.ns_index,
                    &ns_proof.ns_payload,
                    &ns_proof.ns_proof,
                )),
            })
        },
    };
    proto::TxProof { proof: Some(arm) }
}

fn transaction_to_proto(tx: &TransactionQueryData<SeqTypes>) -> proto::TransactionResponse {
    proto::TransactionResponse {
        transaction: Some(proto::Transaction {
            namespace: tx.transaction().namespace().0,
            payload: tx.transaction().payload().to_vec(),
        }),
        hash: tx.hash().to_string(),
        index: tx.index(),
        block_hash: tx.block_hash().to_string(),
        block_height: tx.block_height(),
        namespace: tx.namespace().0,
        pos_in_namespace: tx.pos_in_namespace(),
    }
}

fn transaction_with_proof_to_proto(
    tx: &TransactionWithProofQueryData<SeqTypes>,
) -> proto::TransactionWithProofResponse {
    proto::TransactionWithProofResponse {
        transaction: Some(proto::Transaction {
            namespace: tx.transaction().namespace().0,
            payload: tx.transaction().payload().to_vec(),
        }),
        hash: tx.hash().to_string(),
        index: tx.index(),
        block_hash: tx.block_hash().to_string(),
        block_height: tx.block_height(),
        namespace: tx.namespace().0,
        pos_in_namespace: tx.pos_in_namespace(),
        proof: Some(tx_proof_to_proto(tx.proof())),
    }
}

fn block_summary_to_proto(
    summary: &BlockSummaryQueryData<SeqTypes>,
) -> proto::BlockSummaryResponse {
    proto::BlockSummaryResponse {
        header: Some(header_to_proto(&summary.header)),
        hash: summary.hash.to_string(),
        size: summary.size,
        num_transactions: summary.num_transactions,
    }
}

#[tonic::async_trait]
impl<D> proto::availability_service_server::AvailabilityService for NodeApiStateImpl<D>
where
    D: Deref + Clone + Send + Sync + 'static,
    D::Target: AvailabilityDataSource<SeqTypes> + Send + Sync,
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
        }))
    }

    async fn get_header(
        &self,
        request: tonic::Request<proto::GetHeaderRequest>,
    ) -> Result<tonic::Response<proto::HeaderResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash, request.payload_hash) {
            (Some(height), None, None) => v1::availability::BlockId::Height(height),
            (None, Some(hash), None) => v1::availability::BlockId::Hash(hash),
            (None, None, Some(payload_hash)) => {
                v1::availability::BlockId::PayloadHash(payload_hash)
            },
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height, hash or payload_hash",
                ));
            },
        };
        let header = <Self as v1::HotShotAvailabilityApi>::get_header(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(header_to_proto(&header)))
    }

    async fn get_header_range(
        &self,
        request: tonic::Request<proto::GetHeaderRangeRequest>,
    ) -> Result<tonic::Response<proto::HeaderRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let headers = <Self as v1::HotShotAvailabilityApi>::get_header_range(
            self,
            request.from as usize,
            request.until as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::HeaderRangeResponse {
            headers: headers.iter().map(header_to_proto).collect(),
        }))
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
        Ok(tonic::Response::new(leaf_query_data_to_proto(&leaf)))
    }

    async fn get_leaf_range(
        &self,
        request: tonic::Request<proto::GetLeafRangeRequest>,
    ) -> Result<tonic::Response<proto::LeafRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let leaves = <Self as v1::HotShotAvailabilityApi>::get_leaf_range(
            self,
            request.from as usize,
            request.until as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::LeafRangeResponse {
            leaves: leaves.iter().map(leaf_query_data_to_proto).collect(),
        }))
    }

    async fn get_cert2(
        &self,
        request: tonic::Request<proto::GetCert2Request>,
    ) -> Result<tonic::Response<proto::Certificate2>, tonic::Status> {
        let height = request.into_inner().height;
        // v1 answers a missing certificate with 404 rather than an empty body; keep that.
        let cert2 = <Self as v1::HotShotAvailabilityApi>::get_cert2(self, height)
            .await
            .map_err(to_status)?
            .ok_or_else(|| {
                to_status(not_found(format!("no cert2 available for height {height}")))
            })?;
        Ok(tonic::Response::new(certificate2_to_proto(&cert2)))
    }

    async fn get_block(
        &self,
        request: tonic::Request<proto::GetBlockRequest>,
    ) -> Result<tonic::Response<proto::BlockResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash, request.payload_hash) {
            (Some(height), None, None) => v1::availability::BlockId::Height(height),
            (None, Some(hash), None) => v1::availability::BlockId::Hash(hash),
            (None, None, Some(payload_hash)) => {
                v1::availability::BlockId::PayloadHash(payload_hash)
            },
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height, hash or payload_hash",
                ));
            },
        };
        let block = <Self as v1::HotShotAvailabilityApi>::get_block(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(block_to_proto(&block)))
    }

    async fn get_block_range(
        &self,
        request: tonic::Request<proto::GetBlockRangeRequest>,
    ) -> Result<tonic::Response<proto::BlockRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let blocks = <Self as v1::HotShotAvailabilityApi>::get_block_range(
            self,
            request.from as usize,
            request.until as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockRangeResponse {
            blocks: blocks.iter().map(block_to_proto).collect(),
        }))
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
        Ok(tonic::Response::new(payload_query_data_to_proto(&payload)))
    }

    async fn get_payload_range(
        &self,
        request: tonic::Request<proto::GetPayloadRangeRequest>,
    ) -> Result<tonic::Response<proto::PayloadRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let payloads = <Self as v1::HotShotAvailabilityApi>::get_payload_range(
            self,
            request.from as usize,
            request.until as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::PayloadRangeResponse {
            payloads: payloads.iter().map(payload_query_data_to_proto).collect(),
        }))
    }

    async fn get_vid_common(
        &self,
        request: tonic::Request<proto::GetVidCommonRequest>,
    ) -> Result<tonic::Response<proto::VidCommonResponse>, tonic::Status> {
        let request = request.into_inner();
        let id = match (request.height, request.hash, request.payload_hash) {
            (Some(height), None, None) => v1::availability::BlockId::Height(height),
            (None, Some(hash), None) => v1::availability::BlockId::Hash(hash),
            (None, None, Some(payload_hash)) => {
                v1::availability::BlockId::PayloadHash(payload_hash)
            },
            _ => {
                return Err(tonic::Status::invalid_argument(
                    "set exactly one of height, hash or payload_hash",
                ));
            },
        };
        let common = <Self as v1::HotShotAvailabilityApi>::get_vid_common(self, id)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(vid_common_to_proto(&common)))
    }

    async fn get_vid_common_range(
        &self,
        request: tonic::Request<proto::GetVidCommonRangeRequest>,
    ) -> Result<tonic::Response<proto::VidCommonRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let items = <Self as v1::HotShotAvailabilityApi>::get_vid_common_range(
            self,
            request.from as usize,
            request.until as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::VidCommonRangeResponse {
            items: items.iter().map(vid_common_to_proto).collect(),
        }))
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
        Ok(tonic::Response::new(transaction_to_proto(&tx)))
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
        Ok(tonic::Response::new(transaction_with_proof_to_proto(&tx)))
    }

    async fn get_block_summary(
        &self,
        request: tonic::Request<proto::GetBlockSummaryRequest>,
    ) -> Result<tonic::Response<proto::BlockSummaryResponse>, tonic::Status> {
        let height = request.into_inner().height as usize;
        let summary = <Self as v1::HotShotAvailabilityApi>::get_block_summary(self, height)
            .await
            .map_err(to_status)?;
        Ok(tonic::Response::new(block_summary_to_proto(&summary)))
    }

    async fn get_block_summary_range(
        &self,
        request: tonic::Request<proto::GetBlockSummaryRangeRequest>,
    ) -> Result<tonic::Response<proto::BlockSummaryRangeResponse>, tonic::Status> {
        let request = request.into_inner();
        let summaries = <Self as v1::HotShotAvailabilityApi>::get_block_summary_range(
            self,
            request.from as usize,
            request.until as usize,
        )
        .await
        .map_err(to_status)?;
        Ok(tonic::Response::new(proto::BlockSummaryRangeResponse {
            summaries: summaries.iter().map(block_summary_to_proto).collect(),
        }))
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

    /// Fails when the proto message and the reference vector disagree about which fields exist,
    /// which value-by-value assertions cannot catch: they only check the fields already declared.
    fn assert_same_fields(declared: &[&str], reference: &serde_json::Value, what: &str) {
        let declared: std::collections::BTreeSet<&str> = declared.iter().copied().collect();
        let referenced: std::collections::BTreeSet<&str> = reference
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            declared, referenced,
            "{what} fields drifted from the reference vector"
        );
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

    #[test]
    fn v6_header_mirrors_the_reference_vector() {
        let (header, fields) = reference_header("v6");
        assert_same_fields(
            &[
                "chain_config",
                "height",
                "timestamp",
                "timestamp_millis",
                "l1_head",
                "l1_finalized",
                "payload_commitment",
                "builder_commitment",
                "ns_table",
                "block_merkle_tree_root",
                "fee_merkle_tree_root",
                "fee_info",
                "builder_signature",
                "reward_merkle_tree_root",
                "total_reward_distributed",
                "next_stake_table_hash",
                "leader_counts",
            ],
            &fields,
            "HeaderV5",
        );
        let proto::HeaderResponse { header: converted } = header_to_proto(&header);
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
        use base64::Engine as _;
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
        assert_same_fields(
            &[
                "chain_id",
                "max_block_size",
                "base_fee",
                "fee_contract",
                "fee_recipient",
                "stake_table_contract",
            ],
            expected,
            "ChainConfig",
        );

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

    /// Covers the four shapes and all six arms: every version's vector must select the arm named
    /// after it and carry exactly the fields v1 serializes, so a new protocol version cannot add
    /// a header field without failing here.
    #[test]
    fn every_header_version_maps_to_its_arm_and_fields() {
        const V1_FIELDS: &[&str] = &[
            "chain_config",
            "height",
            "timestamp",
            "l1_head",
            "l1_finalized",
            "payload_commitment",
            "builder_commitment",
            "ns_table",
            "block_merkle_tree_root",
            "fee_merkle_tree_root",
            "fee_info",
            "builder_signature",
        ];
        const V3_FIELDS: &[&str] = &[
            "chain_config",
            "height",
            "timestamp",
            "l1_head",
            "l1_finalized",
            "payload_commitment",
            "builder_commitment",
            "ns_table",
            "block_merkle_tree_root",
            "fee_merkle_tree_root",
            "fee_info",
            "builder_signature",
            "reward_merkle_tree_root",
        ];
        const V4_FIELDS: &[&str] = &[
            "chain_config",
            "height",
            "timestamp",
            "timestamp_millis",
            "l1_head",
            "l1_finalized",
            "payload_commitment",
            "builder_commitment",
            "ns_table",
            "block_merkle_tree_root",
            "fee_merkle_tree_root",
            "fee_info",
            "builder_signature",
            "reward_merkle_tree_root",
            "total_reward_distributed",
            "next_stake_table_hash",
        ];
        const V5_FIELDS: &[&str] = &[
            "chain_config",
            "height",
            "timestamp",
            "timestamp_millis",
            "l1_head",
            "l1_finalized",
            "payload_commitment",
            "builder_commitment",
            "ns_table",
            "block_merkle_tree_root",
            "fee_merkle_tree_root",
            "fee_info",
            "builder_signature",
            "reward_merkle_tree_root",
            "total_reward_distributed",
            "next_stake_table_hash",
            "leader_counts",
        ];

        for (version, shape, expected_fields) in [
            ("v1", "HeaderV1", V1_FIELDS),
            ("v2", "HeaderV1", V1_FIELDS),
            ("v3", "HeaderV3", V3_FIELDS),
            ("v4", "HeaderV4", V4_FIELDS),
            ("v5", "HeaderV5", V5_FIELDS),
            ("v6", "HeaderV5", V5_FIELDS),
        ] {
            let (header, fields) = reference_header(version);
            assert_same_fields(expected_fields, &fields, shape);

            use proto::header_response::Header;
            let converted = header_to_proto(&header).header.unwrap();
            let arm = match converted {
                Header::V1(_) => "v1",
                Header::V2(_) => "v2",
                Header::V3(_) => "v3",
                Header::V4(_) => "v4",
                Header::V5(_) => "v5",
                Header::V6(_) => "v6",
            };
            assert_eq!(arm, version, "{version} header selected the {arm} arm");
        }
    }

    /// No reference vector carries a commitment-only chain config, so the `Right` arm is checked
    /// here on its own. `resolve` must report absence rather than `commit` hashing an empty config.
    #[test]
    fn commitment_only_chain_config_keeps_the_commitment() {
        let config = espresso_types::v0_3::ChainConfig::default();
        let commitment = config.commit();
        let resolvable = espresso_types::v0_3::ResolvableChainConfig::from(commitment);

        let converted = chain_config_to_proto(resolvable).chain_config.unwrap();
        assert_eq!(
            converted,
            proto::resolvable_chain_config::ChainConfig::Commitment(commitment.to_string())
        );
    }

    /// No vector carries view-change evidence, an upgrade certificate, a phase-2 certificate or a
    /// signed QC, so those arms are built from the constructors. The signature assertion is the
    /// one that matters: the API prints the aggregate with `Display`, and this pins that to the
    /// TaggedBase64 form v1's serde emits, so a v2 client can hand the string back to v1.
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
            view_change_evidence_to_proto(&ViewChangeEvidence2::Timeout(timeout)).evidence
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
        assert_eq!(signatures.signers, vec![false, true, false, true]);
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
            view_change_evidence_to_proto(&ViewChangeEvidence2::ViewSync(view_sync)).evidence
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
                new_version_hash: vec![0xab, 0xcd],
                old_version_last_view: ViewNumber::new(19),
                new_version_first_view: ViewNumber::new(21),
            },
            Commitment::from_raw([3; 32]),
            ViewNumber::new(15),
            None,
            PhantomData,
        );
        let data = upgrade_certificate_to_proto(&upgrade).data.unwrap();
        assert_eq!(data.old_version.unwrap().minor, 3);
        assert_eq!(data.new_version.unwrap().minor, 4);
        assert_eq!(data.new_version_hash, vec![0xab, 0xcd]);
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
        let converted = certificate2_to_proto(&cert2);
        let data = converted.data.unwrap();
        assert_eq!(
            data.leaf_commit,
            Commitment::<hotshot_types::data::Leaf2<SeqTypes>>::from_raw([4; 32]).to_string()
        );
        assert_eq!((data.epoch, data.block_number), (5, 77));
        assert_eq!(converted.view_number, 30);
    }

    /// The vector is a list of transactions with AvidM proofs, so the V1 arm is pinned end to end;
    /// its `tx_index` is the 4-byte encoding that `TxIndex::to_bytes` must reproduce. There is no
    /// ADVZ (V0) transaction-proof vector, so that arm is exercised only by the conversion's own
    /// serde reads.
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
        assert_same_fields(
            &[
                "transaction",
                "hash",
                "index",
                "proof",
                "block_hash",
                "block_height",
                "namespace",
                "pos_in_namespace",
            ],
            first,
            "TransactionWithProofResponse",
        );

        let converted = transaction_with_proof_to_proto(&reference[0]);
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
        assert_same_fields(&["tx_index", "ns_proof"], expected, "AvidmTxProof");
        let expected_index: Vec<u8> = expected["tx_index"]
            .as_array()
            .unwrap()
            .iter()
            .map(|byte| byte.as_u64().unwrap() as u8)
            .collect();
        assert_eq!(proof.tx_index, expected_index);
        let ns_proof = proof.ns_proof.unwrap();
        let expected_ns = &expected["ns_proof"];
        assert_same_fields(
            &["ns_index", "ns_payload", "ns_proof"],
            expected_ns,
            "NsProofPayload",
        );
        assert_eq!(ns_proof.ns_index, expected_ns["ns_index"].as_u64().unwrap());
        assert_eq!(
            base64::engine::general_purpose::STANDARD.encode(&ns_proof.ns_payload),
            expected_ns["ns_payload"].as_str().unwrap()
        );
        assert_eq!(ns_proof.ns_proof, expected_ns["ns_proof"]);
    }

    /// One vector per VID scheme. The ADVZ arm is the one read back through serde, so its
    /// assertions are the ones proving that indirection preserves v1's values.
    #[test]
    fn vid_common_mirrors_the_reference_vectors() {
        use proto::vid_common_response::Common;

        let load = |path: &str| -> (VidCommonQueryData<SeqTypes>, serde_json::Value) {
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            (serde_json::from_value(json.clone()).unwrap(), json)
        };
        let outer = &["height", "block_hash", "payload_hash", "common"];

        let (reference, json) = load("../../../data/v1/vid_common_v0.json");
        assert_same_fields(outer, &json, "VidCommonResponse");
        assert_same_fields(
            &[
                "all_evals_digest",
                "multiplicity",
                "num_storage_nodes",
                "payload_byte_len",
                "poly_commits",
            ],
            &json["common"]["V0"],
            "AdvzCommon",
        );
        let converted = vid_common_to_proto(&reference);
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

        let (reference, json) = load("../../../data/v1/vid_common_v1.json");
        assert_same_fields(
            &["recovery_threshold", "total_weights"],
            &json["common"]["V1"],
            "AvidmCommon",
        );
        let Some(Common::V1(avidm)) = vid_common_to_proto(&reference).common else {
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

        let (reference, json) = load("../../../data/v2/vid_common_v2.json");
        assert_same_fields(
            &["ns_commits", "ns_lens", "param"],
            &json["common"]["V2"],
            "AvidmGf2Common",
        );
        let Some(Common::V2(gf2)) = vid_common_to_proto(&reference).common else {
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

    /// Both v1-era vectors carry a 0.1-shaped header; the payload bytes and namespace table are
    /// compared through base64, which is how v1 renders them and how protoJSON renders `bytes`.
    #[test]
    fn block_and_payload_mirror_the_reference_vectors() {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD;

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v1/block_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: BlockQueryData<SeqTypes> = serde_json::from_value(json.clone()).unwrap();
        let block = block_to_proto(&reference);
        assert_same_fields(
            &["header", "payload", "hash", "size", "num_transactions"],
            &json,
            "BlockResponse",
        );
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
        let payload = payload_query_data_to_proto(&reference);
        assert_same_fields(
            &["height", "block_hash", "hash", "size", "data"],
            &json,
            "PayloadResponse",
        );
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

    /// The v3 vector is the current leaf shape: a `Leaf2` certified by a `QuorumCertificate2`,
    /// carrying a 0.1-shaped header since header and leaf versions moved independently. `_pd` is
    /// the one v1 field dropped on purpose: it is `PhantomData` and always serializes as null.
    #[test]
    fn leaf_mirrors_the_reference_vector() {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("../../../data/v3/leaf_query_data.json").unwrap(),
        )
        .unwrap();
        let reference: LeafQueryData<SeqTypes> = serde_json::from_value(json.clone()).unwrap();
        let converted = leaf_query_data_to_proto(&reference);

        assert_same_fields(
            &[
                "view_number",
                "justify_qc",
                "next_epoch_justify_qc",
                "parent_commitment",
                "block_header",
                "upgrade_certificate",
                "block_payload",
                "view_change_evidence",
                "next_drb_result",
                "with_epoch",
            ],
            &json["leaf"],
            "Leaf2",
        );
        assert_same_fields(
            &[
                "_pd",
                "data",
                "vote_commitment",
                "view_number",
                "signatures",
            ],
            &json["qc"],
            "QuorumCertificate2",
        );

        let leaf = converted.leaf.unwrap();
        let expected = &json["leaf"];
        assert_eq!(leaf.view_number, expected["view_number"].as_u64().unwrap());
        assert_eq!(leaf.parent_commitment, expected["parent_commitment"]);
        assert_eq!(leaf.with_epoch, expected["with_epoch"].as_bool().unwrap());
        assert!(leaf.next_epoch_justify_qc.is_none());
        assert!(leaf.upgrade_certificate.is_none());
        assert!(leaf.view_change_evidence.is_none());
        assert!(leaf.next_drb_result.is_none());

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

        // The JSON carries a payload, but `LeafQueryData` deserializes through `new`, which
        // unfills it: a served leaf never holds its payload, that is the payload endpoint's job.
        // The mirror reports what the leaf holds, so this must be absent, not the JSON value.
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
        assert_same_fields(
            &[
                "chain_config",
                "height",
                "timestamp",
                "l1_head",
                "l1_finalized",
                "payload_commitment",
                "builder_commitment",
                "ns_table",
                "block_merkle_tree_root",
                "fee_merkle_tree_root",
                "fee_info",
                "builder_signature",
            ],
            &fields,
            "HeaderV1",
        );
        let proto::HeaderResponse { header: converted } = header_to_proto(&header);
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
