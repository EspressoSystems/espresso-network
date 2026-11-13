use anyhow::Context;
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    BackoffParams, BlockMerkleTree, Certificate2, FeeAccountProof, Leaf2, NodeState, SeqTypes,
    catchup::{
        CatchupRequest, FeeAccountsAt, FrontierAt, LeafAt, LeafResponse, RewardAccountsV1At,
        RewardTreeV2At, StateCertFor, Verified, VerifyError,
    },
    traits::{SequencerPersistence, StateCatchup},
    v0_3::{ChainConfig, RewardAccountProofV1},
    v0_4::PermittedRewardMerkleTreeV2,
};
use hotshot_types::simple_certificate::LightClientStateUpdateCertificateV2;
use tokio::time::timeout;

use crate::request_response::{
    RequestResponseProtocol,
    request::{Request, Response},
};

/// A leaf chain that the leaf chain request accepted.
enum LeafChain<'a> {
    /// A legacy chain decides the requested leaf on its own, so it comes with its verdict.
    Legacy(Box<Result<Verified<Leaf2>, VerifyError>>),
    NewProtocol {
        pending: PendingLeafChain<'a>,
        cert2_height: u64,
    },
}

/// A new-protocol leaf chain that still needs the cert2 that finalizes it.
struct PendingLeafChain<'a> {
    req: &'a LeafAt,
    leaves: Vec<Leaf2>,
}

impl CatchupRequest for PendingLeafChain<'_> {
    type Response = Certificate2<SeqTypes>;
    type Output = Leaf2;

    async fn verify(&self, cert2: Certificate2<SeqTypes>) -> Result<Verified<Leaf2>, VerifyError> {
        // Verification consumes the chain, and a rejected cert2 can be followed by another.
        self.req
            .verify(LeafResponse {
                leaves: self.leaves.clone(),
                cert2: Some(cert2),
            })
            .await
    }
}

impl<P: SequencerPersistence> RequestResponseProtocol<P> {
    /// Waits for a response to `request` that verifies against `req`. `extract` takes the payload
    /// out of a response of the kind `request` asks for.
    async fn request_verified<R: CatchupRequest>(
        &self,
        req: &R,
        request: Request,
        extract: fn(Response) -> Option<R::Response>,
    ) -> anyhow::Result<Verified<R::Output>> {
        let verdict = self
            .request_indefinitely(
                request,
                move |_request: &Request, response: Response| async move {
                    let response = extract(response)
                        .context("response is of another kind than the request")?;
                    match req.verify(response).await {
                        Err(VerifyError::Invalid(err)) => Err(err),
                        // No other responder can supply the stake table this node lacks, so
                        // `Unavailable` ends the wait instead of rejecting the response.
                        verdict => Ok(verdict),
                    }
                },
            )
            .await?;
        Ok(verdict?)
    }

    /// Waits for a leaf chain that has the leaf `req` asks for. A legacy chain must also verify.
    ///
    /// For new-protocol heights the responder returns the leaf range `[height..=cert2_height]`,
    /// for legacy-protocol heights a 3-chain.
    async fn request_leaf_chain<'a>(&self, req: &'a LeafAt) -> anyhow::Result<LeafChain<'a>> {
        let chain = self
            .request_indefinitely(
                Request::Leaf(req.height),
                move |_request: &Request, response: Response| async move {
                    let Response::Leaf(leaves) = response else {
                        anyhow::bail!("expected leaf response");
                    };
                    let Some(cert2_height) = req.cert2_height(&leaves)? else {
                        let response = LeafResponse {
                            leaves,
                            cert2: None,
                        };
                        return match req.verify(response).await {
                            Err(VerifyError::Invalid(err)) => Err(err),
                            // As in `request_verified`, `Unavailable` ends the wait.
                            verdict => Ok(LeafChain::Legacy(Box::new(verdict))),
                        };
                    };
                    Ok(LeafChain::NewProtocol {
                        pending: PendingLeafChain { req, leaves },
                        cert2_height,
                    })
                },
            )
            .await?;
        Ok(chain)
    }
}

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

    async fn try_fetch_blocks_frontier(
        &self,
        _retry: usize,
        instance: &NodeState,
        req: &FrontierAt,
    ) -> anyhow::Result<Verified<BlockMerkleTree>> {
        // Timeout after a few batches
        let timeout_duration = self.config.request_batch_interval * 3;

        timeout(timeout_duration, self.fetch_blocks_frontier(instance, req))
            .await
            .with_context(|| "timed out while fetching blocks frontier")?
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
        req: &StateCertFor,
    ) -> anyhow::Result<Verified<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let timeout_duration = self.config.request_batch_interval * 3;

        timeout(timeout_duration, self.fetch_state_cert(req))
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

        let accounts = self
            .request_verified(
                req,
                Request::Accounts(height, *view, req.accounts.clone()),
                |response| match response {
                    Response::Accounts(tree) => Some(tree),
                    _ => None,
                },
            )
            .await
            .context("failed to request accounts")?;

        tracing::info!("Fetched accounts for height: {height}, view: {view}");
        Ok(accounts)
    }

    async fn fetch_leaf(&self, req: &LeafAt) -> anyhow::Result<Verified<Leaf2>> {
        let height = req.height;
        tracing::info!("Fetching leaf for height: {height}");

        // Shorter than the timeout of `try_fetch_leaf`, so that one attempt has time to give up on
        // a chain and request another.
        let cert2_wait = self.config.request_batch_interval;
        let leaf = loop {
            let chain = self
                .request_leaf_chain(req)
                .await
                .context("failed to request leaf chain")?;
            let (pending, cert2_height) = match chain {
                LeafChain::Legacy(verdict) => break (*verdict)?,
                LeafChain::NewProtocol {
                    pending,
                    cert2_height,
                } => (pending, cert2_height),
            };
            let verified =
                self.request_verified(&pending, Request::Cert2(cert2_height), |response| {
                    match response {
                        Response::Cert2(cert2) => Some(cert2),
                        _ => None,
                    }
                });
            // No cert2 verifies a chain that is not the finalized one, so that wait would never
            // end. Give up on the chain and take another responder's.
            match timeout(cert2_wait, verified).await {
                Ok(leaf) => {
                    break leaf
                        .with_context(|| format!("failed to request cert2 at {cert2_height}"))?;
                },
                Err(_) => tracing::warn!(
                    height,
                    cert2_height,
                    "no cert2 finalized the leaf chain, requesting the chain again"
                ),
            }
        };

        tracing::info!("Fetched leaf for height: {height}");
        Ok(leaf)
    }

    async fn fetch_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<Verified<ChainConfig>> {
        tracing::info!("Fetching chain config with commitment: {commitment}");

        let chain_config = self
            .request_verified(&commitment, Request::ChainConfig(commitment), |response| {
                match response {
                    Response::ChainConfig(chain_config) => Some(chain_config),
                    _ => None,
                }
            })
            .await
            .context("failed to request chain config")?;

        tracing::info!("Fetched chain config with commitment: {commitment}");
        Ok(chain_config)
    }

    async fn fetch_blocks_frontier(
        &self,
        _instance: &NodeState,
        req: &FrontierAt,
    ) -> anyhow::Result<Verified<BlockMerkleTree>> {
        let (height, view) = (req.height, req.view);
        tracing::info!("Fetching blocks frontier for height: {height}, view: {view}");

        let tree =
            self.request_verified(req, Request::BlocksFrontier(height, *view), |response| {
                match response {
                    Response::BlocksFrontier(frontier) => Some(frontier),
                    _ => None,
                }
            })
            .await
            .context("failed to request blocks frontier")?;

        tracing::info!("Fetched blocks frontier for height: {height}, view: {view}");
        Ok(tree)
    }

    async fn fetch_reward_merkle_tree_v2(
        &self,
        req: &RewardTreeV2At,
    ) -> anyhow::Result<Verified<PermittedRewardMerkleTreeV2>> {
        let (height, view) = (req.height, req.view);
        tracing::info!("Fetching RewardMerkleTreeV2 for height: {height}");

        let tree = self
            .request_verified(
                req,
                Request::RewardMerkleTreeV2(height, *view),
                |response| match response {
                    Response::RewardMerkleTreeV2(tree_bytes) => Some(tree_bytes),
                    _ => None,
                },
            )
            .await
            .context("failed to request reward accounts")?;

        tracing::info!("Fetched RewardMerkleTreeV2 for height: {height}");
        Ok(tree)
    }

    async fn fetch_reward_accounts_v1(
        &self,
        _instance: &NodeState,
        req: &RewardAccountsV1At,
    ) -> anyhow::Result<Verified<Vec<RewardAccountProofV1>>> {
        let (height, view) = (req.height, req.view);
        tracing::info!("Fetching v1 reward accounts for height: {height}, view: {view}");

        let accounts = self
            .request_verified(
                req,
                Request::RewardAccountsV1(height, *view, req.accounts.clone()),
                |response| match response {
                    Response::RewardAccountsV1(tree) => Some(tree),
                    _ => None,
                },
            )
            .await
            .context("failed to request v1 reward accounts")?;

        tracing::info!("Fetched v1 reward accounts for height: {height}, view: {view}");
        Ok(accounts)
    }

    async fn fetch_state_cert(
        &self,
        req: &StateCertFor,
    ) -> anyhow::Result<Verified<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let epoch = req.epoch.u64();
        tracing::info!("Fetching state cert for epoch: {epoch}");

        let state_cert = self
            .request_verified(req, Request::StateCert(epoch), |response| match response {
                Response::StateCert(state_cert) => Some(state_cert),
                _ => None,
            })
            .await
            .context("failed to request state cert")?;

        tracing::info!("Fetched state cert for epoch: {epoch}");
        Ok(state_cert)
    }

    fn is_local(&self) -> bool {
        false
    }
}
