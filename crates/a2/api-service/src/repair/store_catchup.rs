//! Catchup that answers leaves from the store and refuses every other request.

use std::sync::Arc;

use anyhow::Context;
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
use membership::null_catchup::NullStateCatchup;
use store_api::Store;

/// Catchup for an API instance that follows the chain in its store: a leaf comes from the store,
/// everything else is refused.
///
/// The follower needs earlier leaves when it applies a block whose state consensus did not send:
/// the epoch rewards at the last block of an epoch are computed from the leaf at the last block
/// of the epoch before, and a stake table that fell out of the retained epochs is derived again
/// from its epoch root. The store holds both, since the follower has passed them. Its leaves are
/// taken as the follower takes the headers it applies: consensus wrote them, or repair, which
/// checks their certificates.
#[derive(Clone, Debug)]
pub struct StoreCatchup {
    store: Arc<dyn Store>,
    refused: NullStateCatchup,
}

impl StoreCatchup {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self {
            store,
            refused: NullStateCatchup::default(),
        }
    }
}

#[async_trait]
impl StateCatchup for StoreCatchup {
    async fn try_fetch_leaf(
        &self,
        _retry: usize,
        _coordinator: EpochMembershipCoordinator<SeqTypes>,
        height: u64,
    ) -> anyhow::Result<Leaf2> {
        let item = self
            .store
            .item(height)
            .await?
            .with_context(|| format!("the store does not hold height {height}"))?;
        Ok(item.leaf)
    }

    async fn try_fetch_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        self.refused
            .try_fetch_accounts(
                retry,
                instance,
                height,
                view,
                fee_merkle_tree_root,
                accounts,
            )
            .await
    }

    async fn try_remember_blocks_merkle_tree(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        self.refused
            .try_remember_blocks_merkle_tree(retry, instance, height, view, mt)
            .await
    }

    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.refused.try_fetch_chain_config(retry, commitment).await
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        retry: usize,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        self.refused
            .try_fetch_reward_merkle_tree_v2(retry, height, view, reward_merkle_tree_root, accounts)
            .await
    }

    async fn try_fetch_reward_accounts_v1(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV1,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        self.refused
            .try_fetch_reward_accounts_v1(
                retry,
                instance,
                height,
                view,
                reward_merkle_tree_root,
                accounts,
            )
            .await
    }

    async fn try_fetch_state_cert(
        &self,
        retry: usize,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        self.refused.try_fetch_state_cert(retry, epoch).await
    }

    fn backoff(&self) -> &BackoffParams {
        self.refused.backoff()
    }

    fn name(&self) -> String {
        "StoreCatchup".into()
    }

    fn is_local(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use chain_store::{ChainStore, Item, Parts, mem::MemBackend};
    use espresso_types::ValidatedState;
    use hotshot_types::{
        simple_certificate::QuorumCertificate2, traits::states::ValidatedState as _,
    };
    use store_api::LocalStore;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;

    /// A leaf the store holds is answered as stored; a height it does not hold is refused, as is
    /// everything but leaves.
    #[tokio::test]
    async fn answers_leaves_from_the_store() {
        let node = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState::genesis(&node).0;
        let leaf = Leaf2::genesis(&state, &node, NEW_PROTOCOL_VERSION).await;
        let item = Item {
            leaf: leaf.clone(),
            cert1: QuorumCertificate2::genesis(
                &state,
                &node,
                Upgrade::trivial(NEW_PROTOCOL_VERSION),
            )
            .await,
        };
        let store = ChainStore::new(MemBackend::default());
        store.put(&item, &Parts::default()).await.unwrap();
        let catchup = StoreCatchup::new(LocalStore::spawn(store));

        let fetched = catchup
            .fetch_leaf(node.coordinator.clone(), 0)
            .await
            .unwrap();
        assert_eq!(fetched, leaf);
        assert!(
            catchup
                .fetch_leaf(node.coordinator.clone(), 1)
                .await
                .is_err()
        );
        assert!(
            catchup
                .fetch_chain_config(state.chain_config.commit())
                .await
                .is_err()
        );
    }
}
