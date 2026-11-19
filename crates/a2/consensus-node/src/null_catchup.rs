//! Catchup that refuses every request.

use std::sync::Arc;

use anyhow::bail;
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    BackoffParams, BlockMerkleTree, ChainConfig, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    Leaf2, NodeState, SeqTypes,
    traits::StateCatchup,
    v0_3::{RewardAccountProofV1, RewardAccountV1, RewardMerkleCommitmentV1},
    v0_4::{PermittedRewardMerkleTreeV2, RewardMerkleCommitmentV2},
    v0_7::RewardAccountV2,
};
use hotshot_types::{
    data::ViewNumber, epoch_membership::EpochMembershipCoordinator,
    simple_certificate::LightClientStateUpdateCertificateV2,
};

/// Catchup that refuses every request: for the stake table fetcher of an API service, which
/// derives stake tables from L1 and its own store only, and for a consensus node until it catches
/// up from API services.
#[derive(Clone, Debug)]
pub struct NullStateCatchup {
    backoff: BackoffParams,
}

impl Default for NullStateCatchup {
    fn default() -> Self {
        Self {
            backoff: BackoffParams::disabled(),
        }
    }
}

#[async_trait]
impl StateCatchup for NullStateCatchup {
    async fn try_fetch_leaf(
        &self,
        _retry: usize,
        _coordinator: EpochMembershipCoordinator<SeqTypes>,
        _height: u64,
    ) -> anyhow::Result<Leaf2> {
        bail!("state catchup is disabled")
    }

    async fn try_fetch_accounts(
        &self,
        _retry: usize,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _fee_merkle_tree_root: FeeMerkleCommitment,
        _account: &[FeeAccount],
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        bail!("state catchup is disabled");
    }

    async fn try_remember_blocks_merkle_tree(
        &self,
        _retry: usize,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        bail!("state catchup is disabled");
    }

    async fn try_fetch_chain_config(
        &self,
        _retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        bail!("chain config {commitment} not available")
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        _retry: usize,
        _height: u64,
        _view: ViewNumber,
        _reward_merkle_tree_root: RewardMerkleCommitmentV2,
        _accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        bail!("state catchup is disabled");
    }

    async fn try_fetch_reward_accounts_v1(
        &self,
        _retry: usize,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _fee_merkle_tree_root: RewardMerkleCommitmentV1,
        _account: &[RewardAccountV1],
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        bail!("state catchup is disabled");
    }

    async fn try_fetch_state_cert(
        &self,
        _retry: usize,
        _epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        bail!("state catchup is disabled");
    }

    fn backoff(&self) -> &BackoffParams {
        &self.backoff
    }

    fn name(&self) -> String {
        "NullStateCatchup".into()
    }

    fn is_local(&self) -> bool {
        true
    }
}
