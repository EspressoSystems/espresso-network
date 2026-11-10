use anyhow::Context;
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    BackoffParams, BlockMerkleTree, FeeAccountProof, Leaf2, NodeState, SeqTypes,
    catchup::{
        FeeAccountsAt, LeafAt, RewardAccountsV1At, RewardTreeV2At, Verified, verify_chain_config,
    },
    traits::{SequencerPersistence, StateCatchup},
    v0_3::{ChainConfig, RewardAccountProofV1},
    v0_4::PermittedRewardMerkleTreeV2,
};
use hotshot_types::{data::ViewNumber, simple_certificate::LightClientStateUpdateCertificateV2};
use jf_merkle_tree_compat::{ForgetableMerkleTreeScheme, MerkleTreeScheme};
use tokio::time::timeout;

use crate::{
    api::RewardMerkleTreeV2Data,
    request_response::{
        RequestResponseProtocol,
        request::{Request, Response},
    },
};

#[async_trait]
impl<P: SequencerPersistence> StateCatchup for RequestResponseProtocol<P> {
    async fn try_fetch_leaf(&self, _retry: usize, req: &LeafAt) -> anyhow::Result<Verified<Leaf2>> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the leaf
        timeout(timeout_duration, self.fetch_leaf(req))
            .await
            .with_context(|| "timed out while fetching leaf")?
    }

    async fn try_fetch_accounts(
        &self,
        _retry: usize,
        instance: &NodeState,
        req: &FeeAccountsAt,
    ) -> anyhow::Result<Verified<Vec<FeeAccountProof>>> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the accounts
        timeout(timeout_duration, self.fetch_accounts(instance, req))
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
    ) -> anyhow::Result<Verified<ChainConfig>> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the chain config
        timeout(timeout_duration, self.fetch_chain_config(commitment))
            .await
            .with_context(|| "timed out while fetching chain config")?
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        _retry: usize,
        req: &RewardTreeV2At,
    ) -> anyhow::Result<Verified<PermittedRewardMerkleTreeV2>> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the reward accounts
        timeout(timeout_duration, self.fetch_reward_merkle_tree_v2(req))
            .await
            .with_context(|| "timed out while fetching reward merkle tree v2")?
    }

    async fn try_fetch_reward_accounts_v1(
        &self,
        _retry: usize,
        instance: &NodeState,
        req: &RewardAccountsV1At,
    ) -> anyhow::Result<Verified<Vec<RewardAccountProofV1>>> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        // Fetch the reward accounts
        timeout(
            timeout_duration,
            self.fetch_reward_accounts_v1(instance, req),
        )
        .await
        .with_context(|| "timed out while fetching reward accounts")?
    }

    async fn try_fetch_state_cert(
        &self,
        _retry: usize,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        let timeout_duration = self.config.request_batch_interval * 3;

        timeout(timeout_duration, self.fetch_state_cert(epoch))
            .await
            .with_context(|| "timed out while fetching state cert")?
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
        req: &FeeAccountsAt,
    ) -> anyhow::Result<Verified<Vec<FeeAccountProof>>> {
        let (height, view) = (req.height, req.view);
        tracing::info!("Fetching accounts for height: {height}, view: {view}");

        let owned_req = req.clone();
        let response_validation_fn = move |_request: &Request, response: Response| {
            let req = owned_req.clone();
            async move {
                let Response::Accounts(fee_merkle_tree) = response else {
                    return Err(anyhow::anyhow!("expected accounts response"));
                };
                req.verify(&fee_merkle_tree)
            }
        };

        // Wait for the protocol to send us the accounts
        let response = self
            .request_indefinitely(
                Request::Accounts(height, *view, req.accounts.clone()),
                response_validation_fn,
            )
            .await
            .with_context(|| "failed to request accounts")?;

        tracing::info!("Fetched accounts for height: {height}, view: {view}");

        Ok(response)
    }

    async fn fetch_leaf(&self, req: &LeafAt) -> anyhow::Result<Verified<Leaf2>> {
        let height = req.height;
        tracing::info!("Fetching leaf for height: {height}");

        // Fetch the leaf chain. For new-protocol heights the responder returns the leaf range
        // `[height..=cert2_height]`; for legacy-protocol heights, it returns a 3-chain.
        let leaf_chain = self
            .request_indefinitely(
                Request::Leaf(height),
                move |_request: &Request, response: Response| async move {
                    let Response::Leaf(leaves) = response else {
                        return Err(anyhow::anyhow!("expected leaf response"));
                    };
                    if leaves.is_empty() {
                        return Err(anyhow::anyhow!(
                            "received empty leaf chain for height {height}"
                        ));
                    }
                    Ok(leaves)
                },
            )
            .await
            .with_context(|| "failed to request leaf chain")?;

        // The chain terminates at the leaf the cert2 finalizes, so request that cert2 by its
        // exact height
        let cert2 = match req.cert2_height(&leaf_chain)? {
            Some(cert2_height) => Some(
                self.request_indefinitely(
                    Request::Cert2(cert2_height),
                    move |_request: &Request, response: Response| async move {
                        let Response::Cert2(cert2) = response else {
                            return Err(anyhow::anyhow!("expected cert2 response"));
                        };
                        if cert2.data.block_number != cert2_height {
                            return Err(anyhow::anyhow!(
                                "received cert2 at height {} but expected exactly {cert2_height}",
                                cert2.data.block_number
                            ));
                        }
                        Ok(cert2)
                    },
                )
                .await
                .with_context(|| format!("failed to request cert2 at height {cert2_height}"))?,
            ),
            None => None,
        };
        let result = req.verify(leaf_chain, cert2).await?;

        tracing::info!("Fetched leaf for height: {height}");
        Ok(result)
    }

    async fn fetch_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<Verified<ChainConfig>> {
        tracing::info!("Fetching chain config with commitment: {commitment}");

        let response_validation_fn = move |_request: &Request, response: Response| async move {
            let Response::ChainConfig(chain_config) = response else {
                return Err(anyhow::anyhow!("expected chain config response"));
            };
            verify_chain_config(commitment, chain_config)
        };

        // Wait for the protocol to send us the chain config
        let response = self
            .request_indefinitely(Request::ChainConfig(commitment), response_validation_fn)
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

    async fn fetch_reward_merkle_tree_v2(
        &self,
        req: &RewardTreeV2At,
    ) -> anyhow::Result<Verified<PermittedRewardMerkleTreeV2>> {
        let (height, view) = (req.height, req.view);
        tracing::info!("Fetching RewardMerkleTreeV2 for height: {height}");

        let owned_req = req.clone();
        let response_validation_fn = move |_request: &Request, response: Response| {
            let req = owned_req.clone();
            async move {
                let Response::RewardMerkleTreeV2(tree_bytes) = response else {
                    return Err(anyhow::anyhow!("expected reward accounts response"));
                };

                let tree_data = bincode::deserialize::<RewardMerkleTreeV2Data>(&tree_bytes)
                    .context(
                        "Failed to deserialize RewardMerkleTreeV2 for height {height} from remote",
                    )?;
                req.verify_balances(tree_data.balances).await
            }
        };

        // Wait for the protocol to send us the reward accounts
        let response = self
            .request_indefinitely(
                Request::RewardMerkleTreeV2(height, *view),
                response_validation_fn,
            )
            .await
            .with_context(|| "failed to request reward accounts")?;

        tracing::info!("Fetched RewardMerkleTreeV2 for height: {height}");

        Ok(response)
    }

    async fn fetch_reward_accounts_v1(
        &self,
        _instance: &NodeState,
        req: &RewardAccountsV1At,
    ) -> anyhow::Result<Verified<Vec<RewardAccountProofV1>>> {
        let (height, view) = (req.height, req.view);
        tracing::info!("Fetching v1 reward accounts for height: {height}, view: {view}");

        let owned_req = req.clone();
        let response_validation_fn = move |_request: &Request, response: Response| {
            let req = owned_req.clone();
            async move {
                let Response::RewardAccountsV1(reward_merkle_tree) = response else {
                    return Err(anyhow::anyhow!("expected v1 reward accounts response"));
                };
                req.verify(&reward_merkle_tree)
            }
        };

        // Wait for the protocol to send us the reward accounts
        let response = self
            .request_indefinitely(
                Request::RewardAccountsV1(height, *view, req.accounts.clone()),
                response_validation_fn,
            )
            .await
            .with_context(|| "failed to request v1 reward accounts")?;

        tracing::info!("Fetched v1 reward accounts for height: {height}, view: {view}");

        Ok(response)
    }

    async fn fetch_state_cert(
        &self,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        tracing::info!("Fetching state cert for epoch: {epoch}");

        // Create the response validation function
        let response_validation_fn = move |_request: &Request, response: Response| async move {
            // Make sure the response is a state cert response
            let Response::StateCert(state_cert) = response else {
                return Err(anyhow::anyhow!("expected state cert response"));
            };

            Ok(state_cert)
        };

        // Wait for the protocol to send us the state cert
        let response = self
            .request_indefinitely(Request::StateCert(epoch), response_validation_fn)
            .await
            .with_context(|| "failed to request state cert")?;

        tracing::info!("Fetched state cert for epoch: {epoch}");

        Ok(response)
    }

    fn is_local(&self) -> bool {
        false
    }
}
