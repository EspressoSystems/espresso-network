use alloy::primitives::U256;
use anyhow::Context;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use espresso_types::{
    traits::{SequencerPersistence, StateCatchup},
    v0_1::{RewardAccount, RewardAccountProof, RewardMerkleCommitment},
    v0_99::ChainConfig,
    BackoffParams, BlockMerkleTree, EpochVersion, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    Leaf2, NodeState, PubKey, SeqTypes, SequencerVersions,
};
use hotshot::traits::NodeImplementation;
use hotshot_types::{
    data::ViewNumber,
    message::UpgradeLock,
    traits::{network::ConnectedNetwork, node_implementation::Versions},
    utils::verify_leaf_chain,
    PeerConfig,
};
use jf_merkle_tree::{ForgetableMerkleTreeScheme, MerkleTreeScheme};
use tokio::time::timeout;

use crate::request_response::{
    request::{Request, Response},
    RequestResponseProtocol,
};

#[async_trait]
impl<
        I: NodeImplementation<SeqTypes>,
        V: Versions,
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
    > StateCatchup for RequestResponseProtocol<I, V, N, P>
{
    async fn try_fetch_leaf(
        &self,
        _retry: usize,
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the leaf
        timeout(
            timeout_duration,
            self.fetch_leaf(height, stake_table, success_threshold),
        )
        .await
        .with_context(|| "timed out while fetching leaf")?
    }

    async fn try_fetch_accounts(
        &self,
        _retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the accounts
        timeout(
            timeout_duration,
            self.fetch_accounts(
                instance,
                height,
                view,
                fee_merkle_tree_root,
                accounts.to_vec(),
            ),
        )
        .await
        .with_context(|| "timed out while fetching accounts")?
    }

    async fn try_remember_blocks_merkle_tree(
        &self,
        _retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Remember the blocks merkle tree
        timeout(
            timeout_duration,
            self.remember_blocks_merkle_tree(instance, height, view, mt),
        )
        .await
        .with_context(|| "timed out while remembering blocks merkle tree")?
    }

    async fn try_fetch_chain_config(
        &self,
        _retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the chain config
        timeout(timeout_duration, self.fetch_chain_config(commitment))
            .await
            .with_context(|| "timed out while fetching chain config")?
    }

    async fn try_fetch_reward_accounts(
        &self,
        _retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the reward accounts
        timeout(
            timeout_duration,
            self.fetch_reward_accounts(
                instance,
                height,
                view,
                reward_merkle_tree_root,
                accounts.to_vec(),
            ),
        )
        .await
        .with_context(|| "timed out while fetching reward accounts")?
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

        // Clone things we need in the first closure
        let accounts_clone = accounts.clone();
        let response_validation_fn = move |_request: &Request, response: Response| {
            // Clone again
            let accounts_clone = accounts_clone.clone();

            async move {
                // Make sure the response is an accounts response
                let Response::Accounts(fee_merkle_tree) = response else {
                    return Err(anyhow::anyhow!("expected accounts response"));
                };

                // Verify the merkle proofs
                let mut proofs = Vec::new();
                for account in accounts_clone {
                    let (proof, _) = FeeAccountProof::prove(&fee_merkle_tree, account.into())
                        .with_context(|| format!("response was missing account {account}"))?;
                    proof
                        .verify(&fee_merkle_tree_root)
                        .with_context(|| format!("invalid proof for account {account}"))?;
                    proofs.push(proof);
                }

                Ok(proofs)
            }
        };

        // Wait for the protocol to send us the accounts
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::Accounts(height, *view, accounts),
                response_validation_fn,
            )
            .await
            .with_context(|| "failed to request accounts")?;

        tracing::info!("Fetched accounts for height: {height}, view: {view}");

        Ok(response)
    }

    async fn fetch_leaf(
        &self,
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        tracing::info!("Fetching leaf for height: {height}");

        // Clone things we need in the first closure
        let stake_table_clone = stake_table.clone();
        let response_validation_fn = move |_request: &Request, response: Response| {
            // Clone again
            let stake_table_clone = stake_table_clone.clone();

            async move {
                // Make sure the response is a leaf response
                let Response::Leaf(leaf_chain) = response else {
                    return Err(anyhow::anyhow!("expected leaf response"));
                };

                // Verify the leaf chain
                let leaf = verify_leaf_chain(
                    leaf_chain,
                    &stake_table_clone,
                    success_threshold,
                    height,
                    &UpgradeLock::<SeqTypes, SequencerVersions<EpochVersion, EpochVersion>>::new(),
                )
                .await
                .with_context(|| "leaf chain verification failed")?;

                Ok(leaf)
            }
        };

        // Wait for the protocol to send us the accounts
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::Leaf(height),
                response_validation_fn,
            )
            .await
            .with_context(|| "failed to request leaf")?;

        tracing::info!("Fetched leaf for height: {height}");

        Ok(response)
    }

    async fn fetch_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        tracing::info!("Fetching chain config with commitment: {commitment}");

        // Create the response validation function
        let response_validation_fn = move |_request: &Request, response: Response| {
            async move {
                // Make sure the response is a chain config response
                let Response::ChainConfig(chain_config) = response else {
                    return Err(anyhow::anyhow!("expected chain config response"));
                };

                // Make sure the commitments match
                if commitment != chain_config.commit() {
                    return Err(anyhow::anyhow!("chain config commitment mismatch"));
                }

                Ok(chain_config)
            }
        };

        // Wait for the protocol to send us the chain config
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::ChainConfig(commitment),
                response_validation_fn,
            )
            .await
            .with_context(|| "failed to request chain config")?;

        tracing::info!("Fetched chain config with commitment: {commitment}");

        Ok(response)
    }

    async fn remember_blocks_merkle_tree(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        tracing::info!("Fetching blocks frontier for height: {height}, view: {view}");

        // Clone the merkle tree
        let mt_clone = mt.clone();

        // Create the response validation function
        let response_validation_fn = move |_request: &Request, response: Response| {
            // Clone the merkle tree
            let mut block_merkle_tree = mt_clone.clone();

            async move {
                // Make sure the response is a blocks frontier response
                let Response::BlocksFrontier(blocks_frontier) = response else {
                    return Err(anyhow::anyhow!("expected blocks frontier response"));
                };

                // Get the leaf element associated with the proof
                let leaf_elem = blocks_frontier
                    .elem()
                    .with_context(|| "provided frontier is missing leaf element")?;

                // Verify the block proof
                block_merkle_tree
                    .remember(
                        block_merkle_tree.num_leaves() - 1,
                        *leaf_elem,
                        blocks_frontier,
                    )
                    .with_context(|| "merkle tree verification failed")?;

                // Return the verified merkle tree
                Ok(block_merkle_tree)
            }
        };

        // Wait for the protocol to send us the blocks frontier
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::BlocksFrontier(height, *view),
                response_validation_fn,
            )
            .await
            .with_context(|| "failed to request blocks frontier")?;

        // Replace the merkle tree
        *mt = response;

        tracing::info!("Fetched blocks frontier for height: {height}, view: {view}");

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
        tracing::info!("Fetching reward accounts for height: {height}, view: {view}");

        // Clone things we need in the first closure
        let accounts_clone = accounts.clone();

        // Create the response validation function
        let response_validation_fn = move |_request: &Request, response: Response| {
            // Clone again
            let accounts_clone = accounts_clone.clone();

            async move {
                // Make sure the response is a reward accounts response
                let Response::RewardAccounts(reward_merkle_tree) = response else {
                    return Err(anyhow::anyhow!("expected reward accounts response"));
                };

                // Verify the merkle proofs
                let mut proofs = Vec::new();
                for account in accounts_clone {
                    let (proof, _) = RewardAccountProof::prove(&reward_merkle_tree, account.into())
                        .with_context(|| format!("response was missing account {account}"))?;
                    proof
                        .verify(&reward_merkle_tree_root)
                        .with_context(|| format!("invalid proof for account {account}"))?;
                    proofs.push(proof);
                }

                Ok(proofs)
            }
        };

        // Wait for the protocol to send us the reward accounts
        let response = self
            .request_indefinitely(
                &self.public_key,
                &self.private_key,
                self.config.incoming_request_ttl,
                Request::RewardAccounts(height, *view, accounts),
                response_validation_fn,
            )
            .await
            .with_context(|| "failed to request reward accounts")?;

        tracing::info!("Fetched reward accounts for height: {height}, view: {view}");

        Ok(response)
    }

    fn is_local(&self) -> bool {
        false
    }
}
