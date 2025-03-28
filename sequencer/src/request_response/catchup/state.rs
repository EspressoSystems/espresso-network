use std::num::NonZeroU64;

use crate::request_response::request::{Output, Request, ValidationContext};
use anyhow::Context;
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    traits::StateCatchup,
    v0_1::{RewardAccount, RewardAccountProof, RewardMerkleCommitment, RewardMerkleTree},
    v0_99::ChainConfig,
    BackoffParams, BlockMerkleTree, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    FeeMerkleTree, Leaf2, NodeState, PubKey, SeqTypes,
};
use hotshot::traits::NodeImplementation;
use hotshot_types::{data::ViewNumber, traits::node_implemtation::Versions, PeerConfig};

use crate::request_response::RequestResponseProtocol;

#[async_trait]
impl<I: NodeImplementation<SeqTypes>, V: Versions> StateCatchup for RequestResponseProtocol<I, V> {
    async fn try_fetch_leaves(&self, _retry: usize, _height: u64) -> anyhow::Result<Vec<Leaf2>> {
        unreachable!()
    }

    async fn try_fetch_accounts(
        &self,
        _retry: usize,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _fee_merkle_tree_root: FeeMerkleCommitment,
        _accounts: &[FeeAccount],
    ) -> anyhow::Result<FeeMerkleTree> {
        unreachable!()
    }

    async fn try_remember_blocks_merkle_tree(
        &self,
        _retry: usize,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        unreachable!()
    }

    async fn try_fetch_chain_config(
        &self,
        _retry: usize,
        _commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        unreachable!()
    }

    #[tracing::instrument(skip(self, _instance))]
    async fn try_fetch_reward_accounts(
        &self,
        _retry: usize,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _reward_merkle_tree_root: RewardMerkleCommitment,
        _accounts: &[RewardAccount],
    ) -> anyhow::Result<RewardMerkleTree> {
        unreachable!()
    }

    fn backoff(&self) -> &BackoffParams {
        unreachable!()
    }

    fn name(&self) -> String {
        "request-response".to_string()
    }

    async fn fetch_accounts(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: Vec<FeeAccount>,
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        tracing::info!("Fetching accounts for height: {height}, view: {view}");

        // Wait for the protocol to send us the accounts
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::Accounts(height, *view, accounts),
                &ValidationContext::Accounts(fee_merkle_tree_root),
            )
            .await
            .with_context(|| "failed to request accounts")?;

        // Validate the response. This should never fail
        let Output::Accounts(proofs) = response else {
            return Err(anyhow::anyhow!("expected accounts response"));
        };

        tracing::info!("Received accounts for height: {height}, view: {view}");

        Ok(proofs)
    }

    async fn fetch_leaf(
        &self,
        height: u64,
        stake_table: Vec<PeerConfig<PubKey>>,
        success_threshold: NonZeroU64,
        epoch_height: u64,
    ) -> anyhow::Result<Leaf2> {
        // Wait for the protocol to send us the accounts
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::Leaf(height),
                &ValidationContext::Leaf(epoch_height, stake_table, success_threshold),
            )
            .await
            .with_context(|| "failed to request leaf")?;

        // Validate the response. This should never fail
        let Output::Leaf(leaf) = response else {
            return Err(anyhow::anyhow!("expected leaf response"));
        };

        Ok(*leaf)
    }

    async fn fetch_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        // Wait for the protocol to send us the chain config
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::ChainConfig(commitment),
                &ValidationContext::None,
            )
            .await
            .with_context(|| "failed to request chain config")?;

        // Validate the response. This should never fail
        let Output::ChainConfig(chain_config) = response else {
            return Err(anyhow::anyhow!("expected chain config response"));
        };

        Ok(chain_config)
    }

    async fn remember_blocks_merkle_tree(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        // Clone the merkle tree
        let mt_clone = mt.clone();

        // Wait for the protocol to send us the blocks frontier
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::BlocksFrontier(height, *view),
                &ValidationContext::BlocksFrontier(mt_clone),
            )
            .await
            .with_context(|| "failed to request blocks frontier")?;

        // Validate the response. This should never fail
        let Output::BlocksFrontier(verified_mt) = response else {
            return Err(anyhow::anyhow!("expected blocks frontier response"));
        };

        // Replace the merkle tree
        *mt = verified_mt;

        Ok(())
    }

    async fn fetch_reward_accounts(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: Vec<RewardAccount>,
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        // Wait for the protocol to send us the reward accounts
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::RewardAccounts(height, *view, accounts),
                &ValidationContext::RewardAccounts(reward_merkle_tree_root),
            )
            .await
            .with_context(|| "failed to request reward accounts")?;

        // Validate the response. This should never fail
        let Output::RewardAccounts(proofs) = response else {
            return Err(anyhow::anyhow!("expected reward accounts response"));
        };

        Ok(proofs)
    }
}
