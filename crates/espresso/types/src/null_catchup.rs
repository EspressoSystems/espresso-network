//! Catchup that refuses every request.

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, bail};
use async_trait::async_trait;
use committable::{Commitment, Committable};
use hotshot_types::{
    data::ViewNumber, epoch_membership::EpochMembershipCoordinator,
    simple_certificate::LightClientStateUpdateCertificateV2,
};

use crate::{
    BackoffParams, BlockMerkleTree, ChainConfig, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    Leaf2, NodeState, SeqTypes,
    traits::StateCatchup,
    v0_3::{RewardAccountProofV1, RewardAccountV1, RewardMerkleCommitmentV1},
    v0_4::{PermittedRewardMerkleTreeV2, RewardMerkleCommitmentV2},
    v0_7::RewardAccountV2,
};

/// Disable catchup entirely.
#[derive(Clone, Debug)]
pub struct NullStateCatchup {
    backoff: BackoffParams,
    chain_configs: HashMap<Commitment<ChainConfig>, ChainConfig>,
}

impl Default for NullStateCatchup {
    fn default() -> Self {
        Self {
            backoff: BackoffParams::disabled(),
            chain_configs: Default::default(),
        }
    }
}

impl NullStateCatchup {
    /// Add a chain config preimage which can be fetched by hash during STF evaluation.
    ///
    /// [`NullStateCatchup`] is used to disable catchup entirely when evaluating the STF, which
    /// requires the [`ValidatedState`](espresso_types::ValidatedState) to be pre-seeded with all
    /// the dependencies of STF evaluation. However, the STF also depends on having the preimage of
    /// various [`ChainConfig`] commitments, which are not stored in the
    /// [`ValidatedState`](espresso_types::ValidatedState), but which instead must be supplied by a
    /// separate preimage oracle. Thus, [`NullStateCatchup`] may be populated with a set of
    /// [`ChainConfig`]s, which it can feed to the STF during evaluation.
    pub fn add_chain_config(&mut self, cf: ChainConfig) {
        self.chain_configs.insert(cf.commit(), cf);
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
        self.chain_configs
            .get(&commitment)
            .copied()
            .context(format!("chain config {commitment} not available"))
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
