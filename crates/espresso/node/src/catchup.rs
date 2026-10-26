use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, anyhow, bail, ensure};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use espresso_api::routes::v1 as paths;
use espresso_types::{
    BackoffParams, BlockMerkleCommitment, BlockMerkleTree, Certificate2, FeeAccount,
    FeeAccountProof, FeeMerkleCommitment, FeeMerkleTree, Leaf2, NodeState, SeqTypes,
    ValidatedState,
    config::PublicNetworkConfig,
    v0::traits::StateCatchup,
    v0_3::{
        ChainConfig, RewardAccountProofV1, RewardAccountV1, RewardMerkleCommitmentV1,
        RewardMerkleTreeV1,
    },
    v0_4::{
        PermittedRewardMerkleTreeV2, RewardAccountProofV2, RewardAccountV2,
        RewardMerkleCommitmentV2, RewardMerkleTreeV2, forgotten_accounts_include,
    },
};
use futures::{
    StreamExt,
    future::{Future, FutureExt, TryFuture, TryFutureExt},
    stream::FuturesUnordered,
};
use hotshot_new_protocol::utils::verify_new_protocol_leaf_chain;
use hotshot_types::{
    ValidatorConfig,
    data::ViewNumber,
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
    simple_certificate::LightClientStateUpdateCertificateV2,
    traits::{
        ValidatedState as ValidatedStateTrait,
        metrics::{Counter, CounterFamily, Metrics},
    },
    utils::verify_leaf_chain,
};
use http_client::{Request, error::ClientErr};
use itertools::Itertools;
use jf_merkle_tree_compat::{ForgetableMerkleTreeScheme, MerkleTreeScheme, prelude::MerkleNode};
use parking_lot::Mutex;
use priority_queue::PriorityQueue;
use serde::de::DeserializeOwned;
use tokio::time::{sleep, timeout};
use tokio_util::task::AbortOnDropHandle;
use url::Url;
use vbs::version::StaticVersionType;
use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION};

use crate::api::{
    BlocksFrontier, RewardMerkleTreeDataSource, RewardMerkleTreeV2Data, context::ConsensusSource,
};

// This newtype is probably not worth having. It's only used to be able to log
// URLs before doing requests.
#[derive(Debug, Clone)]
struct Client<ApiVer: StaticVersionType> {
    inner: http_client::Client<ClientErr, ApiVer>,
    url: Url,
    requests: Arc<Box<dyn Counter>>,
    failures: Arc<Box<dyn Counter>>,
}

impl<ApiVer: StaticVersionType> Client<ApiVer> {
    pub fn new(
        url: Url,
        requests: &(impl CounterFamily + ?Sized),
        failures: &(impl CounterFamily + ?Sized),
    ) -> Self {
        Self {
            inner: http_client::Client::new(url.clone()),
            requests: Arc::new(requests.create(vec![url.to_string()])),
            failures: Arc::new(failures.create(vec![url.to_string()])),
            url,
        }
    }

    pub fn get<T: DeserializeOwned>(&self, route: &str) -> Request<T, ClientErr, ApiVer> {
        self.inner.get(route)
    }
}

/// A score of a catchup peer, based on our interactions with that peer.
///
/// The score accounts for malicious peers -- i.e. peers that gave us an invalid response to a
/// verifiable request -- and faulty/unreliable peers -- those that fail to respond to requests at
/// all. The score has a comparison function where higher is better, or in other words `p1 > p2`
/// means we believe we are more likely to successfully catch up using `p1` than `p2`. This makes it
/// convenient and efficient to collect peers in a priority queue which we can easily convert to a
/// list sorted by reliability.
#[derive(Clone, Copy, Debug, Default)]
struct PeerScore {
    requests: usize,
    failures: usize,
}

impl Ord for PeerScore {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare failure rates: `self` is better than `other` if
        //      self.failures / self.requests < other.failures / other.requests
        // or equivalently
        //      other.failures * self.requests > self.failures * other.requests
        (other.failures * self.requests).cmp(&(self.failures * other.requests))
    }
}

impl PartialOrd for PeerScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PeerScore {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for PeerScore {}

#[derive(Debug, Clone, Default)]
pub struct StatePeers<ApiVer: StaticVersionType> {
    // Peer IDs, ordered by reliability score. Each ID is an index into `clients`.
    scores: Arc<RwLock<PriorityQueue<usize, PeerScore>>>,
    clients: Vec<Client<ApiVer>>,
    backoff: BackoffParams,
    /// Base timeout for per peer catchup request
    base_timeout: Duration,
}

impl<ApiVer: StaticVersionType> StatePeers<ApiVer> {
    async fn fetch<Fut>(
        &self,
        retry: usize,
        f: impl Fn(Client<ApiVer>) -> Fut,
    ) -> anyhow::Result<Fut::Ok>
    where
        Fut: TryFuture<Error: Display>,
    {
        // Since we have generally have multiple peers we can catch up from, we want a fairly
        // aggressive timeout for requests: if a peer is not responding quickly, we're better off
        // just trying the next one rather than waiting, and this prevents a malicious peer from
        // delaying catchup for a long time.
        //
        // However, if we set the timeout _too_ aggressively, we might fail to catch up even from an
        // honest peer, and thus never make progress. Thus, we start with a base timeout (default
        // 2s), which is reasonable for an HTTP request. If that fails with all of our peers, we
        // increase the timeout by the base amount for each successive retry, until we eventually
        // succeed. The base timeout is configurable via ESPRESSO_NODE_CATCHUP_BASE_TIMEOUT.
        let timeout_dur = self.base_timeout * (retry as u32 + 1);

        // Keep track of which peers we make requests to and which succeed (`true`) or fail (`false`),
        // so we can update reliability scores at the end.
        let mut requests = HashMap::new();
        let mut res = Err(anyhow!("failed fetching from every peer"));

        // Try each peer in order of reliability score, until we succeed. We clone out of
        // `self.scores` because it is small (contains only numeric IDs and scores), so this clone
        // is a lot cheaper than holding the read lock the entire time we are making requests (which
        // could be a while).
        let mut scores = { (*self.scores.read().await).clone() };
        let mut logs = vec!["Fetching failed.\n".to_string()];
        while let Some((id, score)) = scores.pop() {
            let client = &self.clients[id];
            tracing::info!("fetching from {}", client.url);
            match timeout(timeout_dur, TryFutureExt::into_future(f(client.clone()))).await {
                Ok(Ok(t)) => {
                    requests.insert(id, true);
                    res = Ok(t);
                    logs = Vec::new();
                    break;
                },
                Ok(Err(err)) => {
                    tracing::debug!(id, ?score, peer = %client.url, "error from peer: {err:#}");
                    logs.push(format!(
                        "Error from peer {} with id {id} and score {score:?}: {err:#}",
                        client.url
                    ));
                    requests.insert(id, false);
                },
                Err(_) => {
                    tracing::debug!(id, ?score, peer = %client.url, ?timeout_dur, "request timed out");
                    logs.push(format!(
                        "Error from peer {} with id {id} and score {score:?}: request timed out",
                        client.url
                    ));
                    requests.insert(id, false);
                },
            }
        }

        if !logs.is_empty() {
            tracing::warn!("{}", logs.join("\n"));
        }

        // Update client scores.
        let mut scores = self.scores.write().await;
        for (id, success) in requests {
            scores.change_priority_by(&id, |score| {
                score.requests += 1;
                self.clients[id].requests.add(1);
                if !success {
                    score.failures += 1;
                    self.clients[id].failures.add(1);
                }
            });
        }

        res
    }

    pub fn from_urls(
        urls: Vec<Url>,
        backoff: BackoffParams,
        base_timeout: Duration,
        metrics: &(impl Metrics + ?Sized),
    ) -> Self {
        if urls.is_empty() {
            panic!("Cannot create StatePeers with no peers");
        }

        let metrics = metrics.subgroup("catchup".into());
        let requests = metrics.counter_family("requests".into(), vec!["peer".into()]);
        let failures = metrics.counter_family("request_failures".into(), vec!["peer".into()]);

        let scores = urls
            .iter()
            .enumerate()
            .map(|(i, _)| (i, PeerScore::default()))
            .collect();
        let clients = urls
            .into_iter()
            .map(|url| Client::new(url, &*requests, &*failures))
            .collect();

        Self {
            clients,
            scores: Arc::new(RwLock::new(scores)),
            backoff,
            base_timeout,
        }
    }

    #[tracing::instrument(skip(self, my_own_validator_config))]
    pub async fn fetch_config(
        &self,
        my_own_validator_config: ValidatorConfig<SeqTypes>,
    ) -> anyhow::Result<NetworkConfig<SeqTypes>> {
        self.backoff()
            .retry(self, move |provider, retry| {
                let my_own_validator_config = my_own_validator_config.clone();
                async move {
                    let cfg: PublicNetworkConfig = provider
                        .fetch(retry, |client| {
                            let url = client.url.join(&paths::config_hotshot()).unwrap();

                            reqwest::get(url.clone())
                        })
                        .await?
                        .json()
                        .await?;
                    cfg.into_network_config(my_own_validator_config)
                        .context("fetched config, but failed to convert to private config")
                }
                .boxed()
            })
            .await
    }
}

/// Verify a legacy (pre-V6) leaf chain
pub(crate) async fn verify_legacy_leaf_chain(
    leaf_chain: Vec<Leaf2>,
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    height: u64,
) -> anyhow::Result<Leaf2> {
    let upgrade_lock = UpgradeLock::<SeqTypes>::new(versions::Upgrade::trivial(EPOCH_VERSION));
    verify_leaf_chain(leaf_chain, coordinator, height, &upgrade_lock)
        .await
        .with_context(|| format!("failed to verify leaf chain at height {height}"))
}

#[async_trait]
impl<ApiVer: StaticVersionType> StateCatchup for StatePeers<ApiVer> {
    #[tracing::instrument(skip(self, _instance))]
    async fn try_fetch_accounts(
        &self,
        retry: usize,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        self.fetch(retry, |client| async move {
            let tree = client
                .inner
                .post::<FeeMerkleTree>(&paths::catchup_accounts(height, view.u64()))
                .body_binary(&accounts.to_vec())?
                .send()
                .await?;

            // Verify proofs.
            let mut proofs = Vec::new();
            for account in accounts {
                let (proof, _) = FeeAccountProof::prove(&tree, (*account).into())
                    .context(format!("response missing fee account {account}"))?;
                proof.verify(&fee_merkle_tree_root).context(format!(
                    "invalid proof for fee account {account}, root: {fee_merkle_tree_root}"
                ))?;
                proofs.push(proof);
            }

            anyhow::Ok(proofs)
        })
        .await
    }

    #[tracing::instrument(skip(self, _instance, mt))]
    async fn try_remember_blocks_merkle_tree(
        &self,
        retry: usize,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        *mt = self
            .fetch(retry, |client| {
                let mut mt = mt.clone();
                async move {
                    let frontier = client
                        .get::<BlocksFrontier>(&paths::catchup_blocks(height, view.u64()))
                        .send()
                        .await?;
                    let elem = frontier
                        .elem()
                        .context("provided frontier is missing leaf element")?;
                    mt.remember(mt.num_leaves() - 1, *elem, &frontier)
                        .context("verifying block proof")?;
                    anyhow::Ok(mt)
                }
            })
            .await?;
        Ok(())
    }

    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.fetch(retry, |client| async move {
            let cf = client
                .get::<ChainConfig>(&paths::catchup_chainconfig(commitment))
                .send()
                .await?;
            ensure!(
                cf.commit() == commitment,
                "received chain config with mismatched commitment: expected {commitment}, got {}",
                cf.commit()
            );
            Ok(cf)
        })
        .await
    }

    async fn try_fetch_leaf(
        &self,
        retry: usize,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        height: u64,
    ) -> anyhow::Result<Leaf2> {
        // Fetch the leaf chain. For new protocol heights this is a leaf range
        // `[height..=cert2_height]`
        // for legacy-protocol heights it's a 3-chain.
        let leaf_chain = self
            .fetch(retry, |client| async move {
                let chain = client
                    .get::<Vec<Leaf2>>(&paths::catchup_leafchain(height))
                    .send()
                    .await?;
                anyhow::Ok(chain)
            })
            .await
            .with_context(|| format!("failed to fetch leaf chain at height {height}"))?;

        let first = leaf_chain
            .first()
            .ok_or_else(|| anyhow!("empty leaf chain returned for height {height}"))?;

        if first.block_header().version() >= NEW_PROTOCOL_VERSION {
            let upgrade_lock =
                UpgradeLock::<SeqTypes>::new(versions::Upgrade::trivial(NEW_PROTOCOL_VERSION));

            // The chain terminates at the leaf the cert2 finalizes, so fetch that cert2 by its
            // exact height
            let cert2_height = leaf_chain
                .last()
                .ok_or_else(|| anyhow!("empty leaf chain returned for height {height}"))?
                .height();
            let cert2 = self
                .fetch(retry, |client| async move {
                    let cert2 = client
                        .get::<Certificate2<SeqTypes>>(&paths::catchup_cert2(cert2_height))
                        .send()
                        .await?;
                    anyhow::Ok(cert2)
                })
                .await
                .with_context(|| format!("failed to fetch cert2 for height {cert2_height}"))?;

            verify_new_protocol_leaf_chain(leaf_chain, &coordinator, height, &upgrade_lock, cert2)
                .await
                .with_context(|| {
                    format!("failed to verify leaf chain with cert2 at height {height}")
                })
        } else {
            verify_legacy_leaf_chain(leaf_chain, &coordinator, height).await
        }
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        retry: usize,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        let result = self
            .fetch(retry, |client| async move {
                // Try the catchup endpoint first which returns tree from consensuss decided state
                // if not present, then fall back to
                // the reward-state-v2 endpoint which returns from storage decided state
                let tree_bytes = match client
                    .inner
                    .get::<Vec<u8>>(&paths::catchup_reward_merkle_tree_v2(height, *view))
                    .send()
                    .await
                {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        tracing::info!(
                            "catchup endpoint failed, falling back to reward-state-v2: {err:#}"
                        );
                        client
                            .inner
                            .get::<Vec<u8>>(&paths::reward_merkle_tree_v2(height))
                            .send()
                            .await?
                    },
                };

                Ok::<Vec<u8>, anyhow::Error>(tree_bytes)
            })
            .await
            .context("Fetching from peer failed")?;

        let tree_data = bincode::deserialize::<RewardMerkleTreeV2Data>(&result)
            .context("Failed to deserialize merkle tree from catchup")?;

        let tree: PermittedRewardMerkleTreeV2 =
            PermittedRewardMerkleTreeV2::try_from_kv_set(tree_data.balances).await?;

        ensure!(
            tree.tree.commitment() == reward_merkle_tree_root,
            "RewardMerkleTreeV2 from peer failed commitment check."
        );
        ensure!(!forgotten_accounts_include(&tree, &accounts));

        Ok(tree)
    }

    #[tracing::instrument(skip(self, _instance))]
    async fn try_fetch_reward_accounts_v1(
        &self,
        retry: usize,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV1,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        self.fetch(retry, |client| async move {
            let tree = client
                .inner
                .post::<RewardMerkleTreeV1>(&paths::catchup_reward_accounts(height, view.u64()))
                .body_binary(&accounts.to_vec())?
                .send()
                .await?;

            // Verify proofs.
            let mut proofs = Vec::new();
            for account in accounts {
                let (proof, _) = RewardAccountProofV1::prove(&tree, (*account).into())
                    .context(format!("response missing reward account {account}"))?;
                proof.verify(&reward_merkle_tree_root).context(format!(
                    "invalid proof for v1 reward account {account}, root: \
                     {reward_merkle_tree_root} height {height} view {view}"
                ))?;
                proofs.push(proof);
            }

            anyhow::Ok(proofs)
        })
        .await
    }

    async fn try_fetch_state_cert(
        &self,
        retry: usize,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        self.fetch(retry, |client| async move {
            client
                .get::<LightClientStateUpdateCertificateV2<SeqTypes>>(&paths::catchup_state_cert(
                    epoch,
                ))
                .send()
                .await
        })
        .await
    }

    fn backoff(&self) -> &BackoffParams {
        &self.backoff
    }

    fn name(&self) -> String {
        format!(
            "StatePeers({})",
            self.clients
                .iter()
                .map(|client| client.url.to_string())
                .join(",")
        )
    }

    fn is_local(&self) -> bool {
        false
    }
}

pub(crate) trait CatchupStorage: Sync {
    /// Get the state of the requested `accounts`.
    ///
    /// The state is fetched from a snapshot at the given height and view, which _must_ correspond!
    /// `height` is provided to simplify lookups for backends where data is not indexed by view.
    /// This function is intended to be used for catchup, so `view` should be no older than the last
    /// decided view.
    ///
    /// If successful, this function also returns the leaf from `view`, if it is available. This can
    /// be used to add the recovered state to HotShot's state map, so that future requests can get
    /// the state from memory rather than storage.
    fn get_accounts(
        &self,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _accounts: &[FeeAccount],
    ) -> impl Send + Future<Output = anyhow::Result<(FeeMerkleTree, Leaf2)>> {
        // Merklized state catchup is only supported by persistence backends that provide merklized
        // state storage. This default implementation is overridden for those that do. Otherwise,
        // catchup can still be provided by fetching undecided merklized state from consensus
        // memory.
        async {
            bail!("merklized state catchup is not supported for this data source");
        }
    }

    /// Get the state of the requested `accounts` from the snapshot at `height` whose root is
    /// `root`.
    ///
    /// Unlike [`get_accounts`](Self::get_accounts), this does not read the leaf or header at
    /// `height`, so it works after the data pruner deleted them, as long as the snapshot is
    /// stored.
    fn get_accounts_at(
        &self,
        _height: u64,
        _root: FeeMerkleCommitment,
        _accounts: &[FeeAccount],
    ) -> impl Send + Future<Output = anyhow::Result<FeeMerkleTree>> {
        async {
            bail!("merklized state catchup is not supported for this data source");
        }
    }

    fn get_reward_accounts_v1(
        &self,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _accounts: &[RewardAccountV1],
    ) -> impl Send + Future<Output = anyhow::Result<(RewardMerkleTreeV1, Leaf2)>> {
        async {
            bail!("merklized state catchup is not supported for this data source");
        }
    }

    fn get_reward_accounts_v2(
        &self,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _accounts: &[RewardAccountV2],
    ) -> impl Send + Future<Output = anyhow::Result<(RewardMerkleTreeV2, Leaf2)>> {
        async {
            bail!("merklized state catchup is not supported for this data source");
        }
    }

    /// Get the blocks Merkle tree frontier.
    ///
    /// The state is fetched from a snapshot at the given height and view, which _must_ correspond!
    /// `height` is provided to simplify lookups for backends where data is not indexed by view.
    /// This function is intended to be used for catchup, so `view` should be no older than the last
    /// decided view.
    fn get_frontier(
        &self,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
    ) -> impl Send + Future<Output = anyhow::Result<BlocksFrontier>> {
        // Merklized state catchup is only supported by persistence backends that provide merklized
        // state storage. This default implementation is overridden for those that do. Otherwise,
        // catchup can still be provided by fetching undecided merklized state from consensus
        // memory.
        async {
            bail!("merklized state catchup is not supported for this data source");
        }
    }

    /// Get the blocks Merkle tree frontier from the snapshot at `height` whose root is `root`.
    ///
    /// Unlike [`get_frontier`](Self::get_frontier), this does not read the header at `height`, so
    /// it works after the data pruner deleted it, as long as the snapshot is stored.
    fn get_frontier_at(
        &self,
        _height: u64,
        _root: BlockMerkleCommitment,
    ) -> impl Send + Future<Output = anyhow::Result<BlocksFrontier>> {
        async {
            bail!("merklized state catchup is not supported for this data source");
        }
    }

    fn get_chain_config(
        &self,
        _commitment: Commitment<ChainConfig>,
    ) -> impl Send + Future<Output = anyhow::Result<ChainConfig>> {
        async {
            bail!("chain config catchup is not supported for this data source");
        }
    }

    fn get_leaf_chain(
        &self,
        _height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<Leaf2>>> {
        async {
            bail!("leaf chain catchup is not supported for this data source");
        }
    }

    /// Load the cert2 stored at exactly `height`, if one exists.
    fn load_cert2(
        &self,
        _height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Option<Certificate2<SeqTypes>>>> {
        async { Ok(None) }
    }

    /// Load a decided leaf at the given height.
    fn get_leaf(&self, _height: u64) -> impl Send + Future<Output = anyhow::Result<Leaf2>> {
        async {
            bail!("leaf fetch is not supported for this data source");
        }
    }

    /// The serialized tree the header at `height` commits to.
    fn load_serialized_reward_merkle_tree_v2(
        &self,
        _height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async {
            bail!("reward merkle tree is not supported for this data source");
        }
    }
}

impl CatchupStorage for hotshot_query_service::data_source::MetricsDataSource {}

impl<T, S> CatchupStorage for hotshot_query_service::data_source::ExtensibleDataSource<T, S>
where
    T: CatchupStorage,
    S: Sync,
{
    async fn get_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<(FeeMerkleTree, Leaf2)> {
        self.inner()
            .get_accounts(instance, height, view, accounts)
            .await
    }

    async fn get_accounts_at(
        &self,
        height: u64,
        root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<FeeMerkleTree> {
        self.inner().get_accounts_at(height, root, accounts).await
    }

    async fn get_reward_accounts_v2(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV2],
    ) -> anyhow::Result<(RewardMerkleTreeV2, Leaf2)> {
        self.inner()
            .get_reward_accounts_v2(instance, height, view, accounts)
            .await
    }

    async fn get_reward_accounts_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<(RewardMerkleTreeV1, Leaf2)> {
        self.inner()
            .get_reward_accounts_v1(instance, height, view, accounts)
            .await
    }

    async fn get_frontier(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
    ) -> anyhow::Result<BlocksFrontier> {
        self.inner().get_frontier(instance, height, view).await
    }

    async fn get_frontier_at(
        &self,
        height: u64,
        root: BlockMerkleCommitment,
    ) -> anyhow::Result<BlocksFrontier> {
        self.inner().get_frontier_at(height, root).await
    }

    async fn get_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.inner().get_chain_config(commitment).await
    }
    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        self.inner().get_leaf_chain(height).await
    }

    async fn load_cert2(&self, height: u64) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        self.inner().load_cert2(height).await
    }

    async fn get_leaf(&self, height: u64) -> anyhow::Result<Leaf2> {
        self.inner().get_leaf(height).await
    }

    async fn load_serialized_reward_merkle_tree_v2(&self, height: u64) -> anyhow::Result<Vec<u8>> {
        self.inner()
            .load_serialized_reward_merkle_tree_v2(height)
            .await
    }
}

#[derive(Debug)]
pub(crate) struct SqlStateCatchup<T> {
    db: Arc<T>,
    backoff: BackoffParams,
}

impl<T> SqlStateCatchup<T> {
    pub(crate) fn new(db: Arc<T>, backoff: BackoffParams) -> Self {
        Self { db, backoff }
    }
}

#[async_trait]
impl<T> StateCatchup for SqlStateCatchup<T>
where
    T: CatchupStorage + RewardMerkleTreeDataSource + Send + Sync,
{
    async fn try_fetch_leaf(
        &self,
        _retry: usize,
        _coordinator: EpochMembershipCoordinator<SeqTypes>,
        height: u64,
    ) -> anyhow::Result<Leaf2> {
        // Leaves in our local DB were verified before they were stored, so we can return the leaf
        // at `height` directly without re-verifying.
        self.db
            .get_leaf(height)
            .await
            .with_context(|| format!("failed to load leaf at height {height} from DB"))
    }

    // TODO: add a test for the account proof validation
    // issue # 2102 (https://github.com/EspressoSystems/espresso-network/issues/2102)
    #[tracing::instrument(skip(self, _retry, instance))]
    async fn try_fetch_accounts(
        &self,
        _retry: usize,
        instance: &NodeState,
        block_height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        // Get the accounts. If the data pruner has deleted the leaf and header that `get_accounts`
        // reads, the snapshot can still be stored, so read it by the root we were given.
        let fee_merkle_tree_from_db = match self
            .db
            .get_accounts(instance, block_height, view, accounts)
            .await
        {
            Ok((tree, _)) => tree,
            Err(err) => {
                let tree = self
                    .db
                    .get_accounts_at(block_height, fee_merkle_tree_root, accounts)
                    .await
                    .with_context(|| format!("failed to get fee accounts from DB: {err:#}"))?;
                tracing::debug!("get_accounts failed, read the accounts by root instead: {err:#}");
                tree
            },
        };

        // Verify the accounts
        let mut proofs = Vec::new();
        for account in accounts {
            let (proof, _) = FeeAccountProof::prove(&fee_merkle_tree_from_db, (*account).into())
                .context(format!("response missing account {account}"))?;
            proof.verify(&fee_merkle_tree_root).context(format!(
                "invalid proof for fee account {account}, root: {fee_merkle_tree_root}"
            ))?;
            proofs.push(proof);
        }

        Ok(proofs)
    }

    #[tracing::instrument(skip(self, _retry, instance, mt))]
    async fn try_remember_blocks_merkle_tree(
        &self,
        _retry: usize,
        instance: &NodeState,
        bh: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        if bh == 0 {
            return Ok(());
        }

        // As for accounts, read the snapshot by root if the header `get_frontier` reads is gone.
        let proof = match self.db.get_frontier(instance, bh, view).await {
            Ok(proof) => proof,
            Err(err) => {
                let proof = self
                    .db
                    .get_frontier_at(bh, mt.commitment())
                    .await
                    .with_context(|| format!("failed to get frontier from DB: {err:#}"))?;
                tracing::debug!("get_frontier failed, read the frontier by root instead: {err:#}");
                proof
            },
        };
        match proof
            .proof
            .first()
            .context(format!("empty proof for frontier at height {bh}"))?
        {
            MerkleNode::Leaf { pos, elem, .. } => mt
                .remember(pos, elem, proof.clone())
                .context("failed to remember proof"),
            _ => bail!("invalid proof"),
        }
    }

    async fn try_fetch_chain_config(
        &self,
        _retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        let cf = self.db.get_chain_config(commitment).await?;

        if cf.commit() != commitment {
            panic!(
                "Critical error: Mismatched chain config detected. Expected chain config: {:?}, \
                 but got: {:?}.
                This may indicate a compromised database",
                commitment,
                cf.commit()
            )
        }

        Ok(cf)
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        _retry: usize,
        height: u64,
        _view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        let tree: PermittedRewardMerkleTreeV2 = self.db.load_reward_merkle_tree_v2(height).await?;

        ensure!(tree.tree.commitment() == reward_merkle_tree_root);
        ensure!(!forgotten_accounts_include(&tree, &accounts));

        Ok(tree)
    }

    #[tracing::instrument(skip(self, _retry, instance))]
    async fn try_fetch_reward_accounts_v1(
        &self,
        _retry: usize,
        instance: &NodeState,
        block_height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV1,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        // Get the accounts
        let (reward_merkle_tree_from_db, _) = self
            .db
            .get_reward_accounts_v1(instance, block_height, view, accounts)
            .await
            .with_context(|| "failed to get reward accounts from DB")?;
        // Verify the accounts
        let mut proofs = Vec::new();
        for account in accounts {
            let (proof, _) =
                RewardAccountProofV1::prove(&reward_merkle_tree_from_db, (*account).into())
                    .context(format!("response missing account {account}"))?;
            proof.verify(&reward_merkle_tree_root).context(format!(
                "invalid proof for v1 reward account {account}, root: {reward_merkle_tree_root}"
            ))?;
            proofs.push(proof);
        }

        Ok(proofs)
    }

    async fn try_fetch_state_cert(
        &self,
        _retry: usize,
        _epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        bail!("state cert catchup not supported for SqlStateCatchup");
    }

    fn backoff(&self) -> &BackoffParams {
        &self.backoff
    }

    fn name(&self) -> String {
        "SqlStateCatchup".into()
    }

    fn is_local(&self) -> bool {
        true
    }
}

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

/// Catchup that sends each request once, for a caller that retries on its own.
///
/// The `fetch_*` methods of a [`ParallelStateCatchup`] race the `fetch_*` methods of its providers,
/// and remote ones retry until a peer answers. Peers may never answer, for example for a height
/// they have pruned, and then the caller waits for good. Here every `fetch_*` method makes one
/// `try_*` call on the inner provider and returns its error.
pub(crate) struct SingleAttemptCatchup<C> {
    inner: C,
    /// How many times the caller has tried before. It goes to the inner provider as the retry
    /// number, so that peers get longer to answer on each try, as on retries of a `fetch_*` call.
    retry: usize,
    backoff: BackoffParams,
}

impl<C> SingleAttemptCatchup<C> {
    pub(crate) fn new(inner: C, retry: usize) -> Self {
        Self {
            inner,
            retry,
            backoff: BackoffParams::disabled(),
        }
    }
}

#[async_trait]
impl<C: StateCatchup> StateCatchup for SingleAttemptCatchup<C> {
    async fn try_fetch_leaf(
        &self,
        retry: usize,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        height: u64,
    ) -> anyhow::Result<Leaf2> {
        self.inner
            .try_fetch_leaf(self.retry + retry, coordinator, height)
            .await
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
        self.inner
            .try_fetch_accounts(
                self.retry + retry,
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
        self.inner
            .try_remember_blocks_merkle_tree(self.retry + retry, instance, height, view, mt)
            .await
    }

    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.inner
            .try_fetch_chain_config(self.retry + retry, commitment)
            .await
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        retry: usize,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        self.inner
            .try_fetch_reward_merkle_tree_v2(
                self.retry + retry,
                height,
                view,
                reward_merkle_tree_root,
                accounts,
            )
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
        self.inner
            .try_fetch_reward_accounts_v1(
                self.retry + retry,
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
        self.inner
            .try_fetch_state_cert(self.retry + retry, epoch)
            .await
    }

    fn backoff(&self) -> &BackoffParams {
        &self.backoff
    }

    fn name(&self) -> String {
        self.inner.name()
    }

    fn is_local(&self) -> bool {
        self.inner.is_local()
    }
}

/// A provider and when it joins the race.
#[derive(Clone)]
struct Provider {
    catchup: Arc<dyn StateCatchup>,
    /// Starts after `fallback_delay` if a primary (non-fallback) remote provider is present.
    fallback: bool,
}

/// A catchup implementation that races all providers.
/// It returns the first successful result.
#[derive(Clone)]
pub struct ParallelStateCatchup {
    providers: Arc<Mutex<Vec<Provider>>>,
    backoff: BackoffParams,
    /// Timeout for local provider requests
    local_timeout: Duration,
    fallback_delay: Duration,
}

impl ParallelStateCatchup {
    /// Create a new [`ParallelStateCatchup`] with the given providers and local timeout.
    pub fn new(providers: &[Arc<dyn StateCatchup>], local_timeout: Duration) -> Self {
        Self {
            providers: Arc::new(Mutex::new(
                providers
                    .iter()
                    .map(|catchup| Provider {
                        catchup: catchup.clone(),
                        fallback: false,
                    })
                    .collect(),
            )),
            backoff: BackoffParams::disabled(),
            local_timeout,
            fallback_delay: Duration::ZERO,
        }
    }

    /// Delay before fallback providers start; see [`Provider::fallback`].
    pub fn with_fallback_delay(mut self, delay: Duration) -> Self {
        self.fallback_delay = delay;
        self
    }

    /// Add a provider to the list of providers
    pub fn add_provider(&self, provider: Arc<dyn StateCatchup>) {
        self.add(provider, false);
    }

    /// Add a remote provider that races after the fallback delay; see [`Provider::fallback`].
    pub fn add_fallback_provider(&self, provider: Arc<dyn StateCatchup>) {
        self.add(provider, true);
    }

    fn add(&self, catchup: Arc<dyn StateCatchup>, fallback: bool) {
        self.providers.lock().push(Provider { catchup, fallback });
    }

    /// Perform an async operation on all providers, returning the first result to succeed
    async fn on_providers<C, F, RT>(&self, closure: C) -> anyhow::Result<RT>
    where
        C: Fn(Arc<dyn StateCatchup>) -> F + Clone + Send + Sync + 'static,
        F: Future<Output = anyhow::Result<RT>> + Send + 'static,
        RT: Send + Sync + 'static,
    {
        let providers = self.providers.lock().clone();
        if providers.is_empty() {
            return Err(anyhow::anyhow!("no providers were initialized"));
        }

        let has_primary_remote = providers
            .iter()
            .any(|p| !p.fallback && !p.catchup.is_local());
        let local_timeout = self.local_timeout;
        let fallback_delay = self.fallback_delay;

        let mut futures = FuturesUnordered::new();
        for Provider { catchup, fallback } in providers {
            let closure = closure.clone();
            let name = catchup.name();
            futures.push(AbortOnDropHandle::new(tokio::spawn(async move {
                let result = if catchup.is_local() {
                    timeout(local_timeout, closure(catchup))
                        .await
                        .unwrap_or_else(|_| {
                            let err = format!("{name} timed out after {local_timeout:?}");
                            tracing::warn!("{err}");
                            Err(anyhow::anyhow!(err))
                        })
                } else {
                    if fallback && has_primary_remote {
                        sleep(fallback_delay).await;
                    }
                    closure(catchup).await
                };
                (name, result)
            })));
        }

        let mut logs = vec!["No providers returned a successful result.\n".to_string()];
        // Return the first successful result
        while let Some(result) = futures.next().await {
            // Unwrap the inner (join) result
            let (name, result) = match result {
                Ok(res) => res,
                Err(err) => {
                    tracing::debug!("Failed to join on provider: {err:#}.");
                    logs.push(format!("Failed to join on provider: {err:#}."));
                    continue;
                },
            };

            // If a provider fails, print why
            let result = match result {
                Ok(res) => res,
                Err(err) => {
                    tracing::debug!("Failed to fetch data from {name}: {err:#}.");
                    logs.push(format!("Failed to fetch data from {name}: {err:#}."));
                    continue;
                },
            };

            return Ok(result);
        }

        Err(anyhow::anyhow!(logs.join("\n")))
    }
}

macro_rules! clone {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

#[async_trait]
impl StateCatchup for ParallelStateCatchup {
    async fn try_fetch_leaf(
        &self,
        retry: usize,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        height: u64,
    ) -> anyhow::Result<Leaf2> {
        self.on_providers(clone! {(coordinator) move |provider| {
            clone!{(coordinator) async move {
                provider
                    .try_fetch_leaf(retry, coordinator, height)
                    .await
            }}
        }})
        .await
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
        let accounts_vec = accounts.to_vec();
        self.on_providers(clone! {(instance, accounts_vec) move |provider| {
            clone! {(instance, accounts_vec) async move {
                provider
                    .try_fetch_accounts(
                        retry,
                        &instance,
                        height,
                        view,
                        fee_merkle_tree_root,
                        &accounts_vec,
                    )
                    .await
            }}
        }})
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
        *mt = self
            .on_providers(clone! {(mt, instance) move |provider| {
                let mut mt = mt.clone();
                clone! {(instance) async move {
                    provider
                        .try_remember_blocks_merkle_tree(
                            retry,
                            &instance,
                            height,
                            view,
                            &mut mt,
                        )
                        .await?;

                    Ok(mt)
                }}
            }})
            .await?;

        Ok(())
    }

    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.on_providers(move |provider| async move {
            provider.try_fetch_chain_config(retry, commitment).await
        })
        .await
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        retry: usize,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        self.on_providers(clone! {(accounts) move |provider| {
            clone! {(accounts) async move {
                provider
                    .try_fetch_reward_merkle_tree_v2(
                        retry,
                        height,
                        view,
                        reward_merkle_tree_root,
                        accounts,
                    )
                    .await
            }}
        }})
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
        let accounts_vec = accounts.to_vec();
        self.on_providers(clone! {(instance, accounts_vec) move |provider| {
            clone! {(instance, accounts_vec) async move {
                provider
                    .try_fetch_reward_accounts_v1(
                        retry,
                        &instance,
                        height,
                        view,
                        reward_merkle_tree_root,
                        &accounts_vec,
                    )
                    .await
            }}
        }})
        .await
    }

    async fn try_fetch_state_cert(
        &self,
        retry: usize,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        self.on_providers(move |provider| async move {
            provider.try_fetch_state_cert(retry, epoch).await
        })
        .await
    }

    fn backoff(&self) -> &BackoffParams {
        &self.backoff
    }

    fn name(&self) -> String {
        format!(
            "[{}]",
            self.providers
                .lock()
                .iter()
                .map(|p| p.catchup.name())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    async fn fetch_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: Vec<FeeAccount>,
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        self.on_providers(clone! {(instance, accounts) move |provider| {
            clone! {(instance, accounts) async move {
                provider
                    .fetch_accounts(&instance, height, view, fee_merkle_tree_root, accounts)
                    .await
            }}
        }})
        .await
    }

    async fn fetch_leaf(
        &self,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        height: u64,
    ) -> anyhow::Result<Leaf2> {
        self.on_providers(clone! {(coordinator) move |provider| {
            clone! {(coordinator) async move {
                provider.fetch_leaf(coordinator, height).await
            }}
        }})
        .await
    }

    async fn fetch_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.on_providers(
            move |provider| async move { provider.fetch_chain_config(commitment).await },
        )
        .await
    }

    async fn fetch_reward_accounts_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV1,
        accounts: Vec<RewardAccountV1>,
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        self.on_providers(clone! {(instance, accounts) move |provider| {
            clone! {(instance, accounts) async move {
                provider
                    .fetch_reward_accounts_v1(
                        &instance,
                        height,
                        view,
                        reward_merkle_tree_root,
                        accounts,
                    )
                    .await
            }}
        }})
        .await
    }

    async fn fetch_state_cert(
        &self,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        self.on_providers(move |provider| async move { provider.fetch_state_cert(epoch).await })
            .await
    }

    async fn remember_blocks_merkle_tree(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        *mt = self
            .on_providers(clone! {(mt, instance) move |provider| {
                let mut mt = mt.clone();
                clone! {(instance) async move {
                    provider
                        .remember_blocks_merkle_tree(&instance, height, view, &mut mt)
                        .await?;
                    Ok(mt)
                }}
            }})
            .await?;

        Ok(())
    }

    fn is_local(&self) -> bool {
        self.providers.lock().iter().all(|p| p.catchup.is_local())
    }

    fn with_local_timeout(&self, local_timeout: Duration) -> Option<Arc<dyn StateCatchup>> {
        // The copy shares the provider list, so it also sees providers added later.
        Some(Arc::new(Self {
            local_timeout,
            ..self.clone()
        }))
    }
}

/// Add accounts to the in-memory consensus state.
/// We use this during catchup after receiving verified accounts.
pub async fn add_fee_accounts_to_state(
    consensus_handle: &dyn ConsensusSource,
    view: &ViewNumber,
    accounts: &[FeeAccount],
    tree: &FeeMerkleTree,
    leaf: Leaf2,
) -> anyhow::Result<()> {
    let (existing_state, delta) = consensus_handle.state_and_delta(*view).await;
    let (state, delta) = match existing_state {
        Some(existing) => {
            let mut state = (*existing).clone();
            for account in accounts {
                if let Some((proof, _)) = FeeAccountProof::prove(tree, (*account).into()) {
                    if let Err(err) = proof.remember(&mut state.fee_merkle_tree) {
                        tracing::warn!(
                            ?view,
                            %account,
                            "cannot update fetched account state: {err:#}"
                        );
                    }
                } else {
                    tracing::warn!(?view, %account, "cannot update fetched account state because account is not in the merkle tree");
                };
            }
            (Arc::new(state), delta)
        },
        None => {
            // If we don't already have a leaf for this view, or if we don't have the view
            // at all, we can create a new view based on the recovered leaf and add it to
            // our state map. In this case, we must also add the leaf to the saved leaves
            // map to ensure consistency.
            let mut state = ValidatedState::from_header(leaf.block_header());
            state.fee_merkle_tree = tree.clone();
            (Arc::new(state), None)
        },
    };

    consensus_handle
        .update_leaf(leaf, state, delta)
        .await
        .with_context(|| "failed to update leaf")?;

    Ok(())
}

/// Add accounts to the in-memory consensus state.
/// We use this during catchup after receiving verified accounts.
pub async fn add_v2_reward_accounts_to_state(
    consensus_handle: &dyn ConsensusSource,
    view: &ViewNumber,
    accounts: &[RewardAccountV2],
    tree: &RewardMerkleTreeV2,
    leaf: Leaf2,
) -> anyhow::Result<()> {
    let (existing_state, delta) = consensus_handle.state_and_delta(*view).await;
    let (state, delta) = match existing_state {
        Some(existing) => {
            let mut state = (*existing).clone();
            for account in accounts {
                if let Some((proof, _)) = RewardAccountProofV2::prove(tree, (*account).into()) {
                    if let Err(err) = proof.remember(&mut state.reward_merkle_tree_v2) {
                        tracing::warn!(
                            ?view,
                            %account,
                            "cannot update fetched account state: {err:#}"
                        );
                    }
                } else {
                    tracing::warn!(?view, %account, "cannot update fetched account state because account is not in the merkle tree");
                };
            }
            (Arc::new(state), delta)
        },
        None => {
            // If we don't already have a leaf for this view, or if we don't have the view
            // at all, we can create a new view based on the recovered leaf and add it to
            // our state map. In this case, we must also add the leaf to the saved leaves
            // map to ensure consistency.
            let mut state = ValidatedState::from_header(leaf.block_header());
            state.reward_merkle_tree_v2 = tree.clone();
            (Arc::new(state), None)
        },
    };

    consensus_handle
        .update_leaf(leaf, state, delta)
        .await
        .with_context(|| "failed to update leaf")?;

    Ok(())
}

/// Add accounts to the in-memory consensus state.
/// We use this during catchup after receiving verified accounts.
pub async fn add_v1_reward_accounts_to_state(
    consensus_handle: &dyn ConsensusSource,
    view: &ViewNumber,
    accounts: &[RewardAccountV1],
    tree: &RewardMerkleTreeV1,
    leaf: Leaf2,
) -> anyhow::Result<()> {
    let (existing_state, delta) = consensus_handle.state_and_delta(*view).await;
    let (state, delta) = match existing_state {
        Some(existing) => {
            let mut state = (*existing).clone();
            for account in accounts {
                if let Some((proof, _)) = RewardAccountProofV1::prove(tree, (*account).into()) {
                    if let Err(err) = proof.remember(&mut state.reward_merkle_tree_v1) {
                        tracing::warn!(
                            ?view,
                            %account,
                            "cannot update fetched account state: {err:#}"
                        );
                    }
                } else {
                    tracing::warn!(?view, %account, "cannot update fetched account state because account is not in the merkle tree");
                };
            }
            (Arc::new(state), delta)
        },
        None => {
            // If we don't already have a leaf for this view, or if we don't have the view
            // at all, we can create a new view based on the recovered leaf and add it to
            // our state map. In this case, we must also add the leaf to the saved leaves
            // map to ensure consistency.
            let mut state = ValidatedState::from_header(leaf.block_header());
            state.reward_merkle_tree_v1 = tree.clone();
            (Arc::new(state), None)
        },
    };

    consensus_handle
        .update_leaf(leaf, state, delta)
        .await
        .with_context(|| "failed to update leaf")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::{
        sync::atomic::{AtomicUsize, Ordering as AtomicOrdering},
        time::Instant,
    };

    use espresso_types::v0::mock::MockStateCatchup;
    use hotshot_types::traits::metrics::NoMetrics;
    use test_utils::reserve_tcp_port;

    use super::*;
    use crate::SequencerApiVersion;

    /// A peer that never answers keeps the `fetch_*` methods of a [`ParallelStateCatchup`] retrying
    /// for good, so a caller that retries on its own never gets an error to retry from.
    /// [`SingleAttemptCatchup`] returns the error after one request.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn single_attempt_catchup_does_not_retry_peers() {
        // Nothing listens on this port, so every request to the peer fails.
        let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
        let peer = StatePeers::<SequencerApiVersion>::from_urls(
            vec![format!("http://127.0.0.1:{port}").parse().unwrap()],
            BackoffParams::default(),
            Duration::from_secs(1),
            &NoMetrics,
        );
        let peers = ParallelStateCatchup::new(&[Arc::new(peer)], Duration::from_secs(1));
        let chain_config = ChainConfig::default().commit();
        assert!(
            timeout(
                Duration::from_secs(3),
                peers.fetch_chain_config(chain_config)
            )
            .await
            .is_err(),
            "the parallel catchup should retry the peer until the timeout"
        );

        let once = SingleAttemptCatchup::new(peers, 0);
        let res = timeout(
            Duration::from_secs(3),
            once.fetch_chain_config(chain_config),
        )
        .await
        .expect("fetching the chain config should not retry");
        assert!(res.is_err());
        let instance = NodeState::mock();
        let fee_root = ValidatedState::default().fee_merkle_tree.commitment();
        let res = timeout(
            Duration::from_secs(3),
            once.fetch_accounts(
                &instance,
                1,
                ViewNumber::new(1),
                fee_root,
                vec![FeeAccount::default()],
            ),
        )
        .await
        .expect("fetching fee accounts should not retry");
        assert!(res.is_err());
        let mut frontier = ValidatedState::default().block_merkle_tree;
        let res = timeout(
            Duration::from_secs(3),
            once.remember_blocks_merkle_tree(&instance, 1, ViewNumber::new(1), &mut frontier),
        )
        .await
        .expect("fetching the block frontier should not retry");
        assert!(res.is_err());
    }

    /// The try number the caller passes in reaches the peers, so that they get longer to answer on
    /// later tries.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn single_attempt_catchup_passes_the_try_number_on() {
        const BASE_TIMEOUT: Duration = Duration::from_millis(200);
        // The peer accepts connections but never answers, so each request takes its whole timeout.
        let silent = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let peer = StatePeers::<SequencerApiVersion>::from_urls(
            vec![
                format!("http://{}", silent.local_addr().unwrap())
                    .parse()
                    .unwrap(),
            ],
            BackoffParams::default(),
            BASE_TIMEOUT,
            &NoMetrics,
        );
        let peers = ParallelStateCatchup::new(&[Arc::new(peer)], Duration::from_secs(1));

        let start = Instant::now();
        let res = SingleAttemptCatchup::new(peers, 4)
            .fetch_chain_config(ChainConfig::default().commit())
            .await;
        assert!(res.is_err());
        // On the fifth try, the peer gets five times the base timeout.
        let elapsed = start.elapsed();
        assert!(
            elapsed >= 5 * BASE_TIMEOUT,
            "the peer got only {elapsed:?} to answer"
        );
    }

    /// A copy with a longer local timeout waits that long on a slow local provider, where the
    /// original gives up on it.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn with_local_timeout_waits_longer_on_local_providers() {
        let slow = MockStateCatchup::default().with_delay(Duration::from_millis(500));
        let catchup = ParallelStateCatchup::new(&[Arc::new(slow)], Duration::from_millis(50));
        let chain_config = ChainConfig::default().commit();
        // There is no remote provider, so a local read cut short fails.
        assert!(
            catchup
                .try_fetch_chain_config(0, chain_config)
                .await
                .is_err()
        );

        let longer = catchup
            .with_local_timeout(Duration::from_secs(5))
            .expect("a parallel catchup has a local timeout");
        longer
            .try_fetch_chain_config(0, chain_config)
            .await
            .expect("the local provider answers within the longer timeout");

        // The copy shares the provider list, so it also sees providers added later.
        catchup.add_provider(Arc::new(StatePeers::<SequencerApiVersion>::from_urls(
            vec!["http://127.0.0.1:1".parse().unwrap()],
            BackoffParams::default(),
            Duration::from_secs(1),
            &NoMetrics,
        )));
        assert!(!longer.is_local());
    }

    #[test]
    fn test_peer_priority() {
        let good_peer = PeerScore {
            requests: 1000,
            failures: 2,
        };
        let bad_peer = PeerScore {
            requests: 10,
            failures: 1,
        };
        assert!(good_peer > bad_peer);

        let mut peers: PriorityQueue<_, _> = [(0, good_peer), (1, bad_peer)].into_iter().collect();
        assert_eq!(peers.pop(), Some((0, good_peer)));
        assert_eq!(peers.pop(), Some((1, bad_peer)));
    }

    /// Serves a chain config after `delay`; every other request fails.
    struct DelayedCatchup {
        local: bool,
        delay: Duration,
        backoff: BackoffParams,
        calls: Arc<AtomicUsize>,
    }

    impl DelayedCatchup {
        fn provider(local: bool, delay: Duration) -> Arc<dyn StateCatchup> {
            Self::counted(local, delay).0
        }

        fn counted(local: bool, delay: Duration) -> (Arc<dyn StateCatchup>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            let provider = Arc::new(Self {
                local,
                delay,
                backoff: BackoffParams::disabled(),
                calls: calls.clone(),
            });
            (provider, calls)
        }
    }

    #[async_trait]
    impl StateCatchup for DelayedCatchup {
        async fn try_fetch_chain_config(
            &self,
            _retry: usize,
            _commitment: Commitment<ChainConfig>,
        ) -> anyhow::Result<ChainConfig> {
            self.calls.fetch_add(1, AtomicOrdering::SeqCst);
            tokio::time::sleep(self.delay).await;
            Ok(ChainConfig::default())
        }

        async fn try_fetch_leaf(
            &self,
            _retry: usize,
            _coordinator: EpochMembershipCoordinator<SeqTypes>,
            _height: u64,
        ) -> anyhow::Result<Leaf2> {
            bail!("unsupported")
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
            bail!("unsupported")
        }

        async fn try_remember_blocks_merkle_tree(
            &self,
            _retry: usize,
            _instance: &NodeState,
            _height: u64,
            _view: ViewNumber,
            _mt: &mut BlockMerkleTree,
        ) -> anyhow::Result<()> {
            bail!("unsupported")
        }

        async fn try_fetch_reward_merkle_tree_v2(
            &self,
            _retry: usize,
            _height: u64,
            _view: ViewNumber,
            _reward_merkle_tree_root: RewardMerkleCommitmentV2,
            _accounts: Arc<Vec<RewardAccountV2>>,
        ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
            bail!("unsupported")
        }

        async fn try_fetch_reward_accounts_v1(
            &self,
            _retry: usize,
            _instance: &NodeState,
            _height: u64,
            _view: ViewNumber,
            _reward_merkle_tree_root: RewardMerkleCommitmentV1,
            _accounts: &[RewardAccountV1],
        ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
            bail!("unsupported")
        }

        async fn try_fetch_state_cert(
            &self,
            _retry: usize,
            _epoch: u64,
        ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
            bail!("unsupported")
        }

        fn backoff(&self) -> &BackoffParams {
            &self.backoff
        }

        fn name(&self) -> String {
            format!(
                "DelayedCatchup({})",
                if self.local { "local" } else { "remote" }
            )
        }

        fn is_local(&self) -> bool {
            self.local
        }
    }

    #[tokio::test(start_paused = true)]
    async fn test_remote_result_does_not_wait_for_local() {
        let local = DelayedCatchup::provider(true, Duration::from_secs(60));
        let remote = DelayedCatchup::provider(false, Duration::ZERO);
        let catchup = ParallelStateCatchup::new(&[local, remote], Duration::from_secs(30));

        let config = ChainConfig::default();
        timeout(
            Duration::from_secs(1),
            catchup.try_fetch_chain_config(0, config.commit()),
        )
        .await
        .expect("remote result should not wait for the local provider")
        .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn test_local_result_does_not_wait_for_remote() {
        let local = DelayedCatchup::provider(true, Duration::ZERO);
        let remote = DelayedCatchup::provider(false, Duration::from_secs(60));
        let catchup = ParallelStateCatchup::new(&[local, remote], Duration::from_secs(30));

        let config = ChainConfig::default();
        timeout(
            Duration::from_secs(1),
            catchup.fetch_chain_config(config.commit()),
        )
        .await
        .expect("local result should not wait for the remote provider")
        .unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn test_all_providers_failing_reports_every_error() {
        let local = DelayedCatchup::provider(true, Duration::ZERO);
        let remote = DelayedCatchup::provider(false, Duration::ZERO);
        let catchup = ParallelStateCatchup::new(&[local, remote], Duration::from_secs(30));

        let err = format!("{:#}", catchup.fetch_state_cert(1).await.unwrap_err());
        assert_eq!(err.matches("unsupported").count(), 2, "{err}");
        assert!(err.contains("DelayedCatchup(local)"), "{err}");
        assert!(err.contains("DelayedCatchup(remote)"), "{err}");
    }

    const FALLBACK_DELAY: Duration = Duration::from_secs(2);

    fn fallback_catchup(
        primary: Arc<dyn StateCatchup>,
        fallback: Arc<dyn StateCatchup>,
    ) -> ParallelStateCatchup {
        let catchup = ParallelStateCatchup::new(&[primary], Duration::from_secs(30))
            .with_fallback_delay(FALLBACK_DELAY);
        catchup.add_fallback_provider(fallback);
        catchup
    }

    #[tokio::test(start_paused = true)]
    async fn test_fallback_not_called_when_primary_answers_first() {
        let (primary, _) = DelayedCatchup::counted(false, Duration::from_secs(1));
        let (fallback, fallback_calls) = DelayedCatchup::counted(false, Duration::ZERO);
        let catchup = fallback_catchup(primary, fallback);

        catchup
            .fetch_chain_config(ChainConfig::default().commit())
            .await
            .unwrap();
        tokio::time::sleep(FALLBACK_DELAY * 2).await;
        assert_eq!(fallback_calls.load(AtomicOrdering::SeqCst), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn test_fallback_answers_after_delay_when_primary_hangs() {
        let (primary, _) = DelayedCatchup::counted(false, Duration::from_secs(3600));
        let (fallback, fallback_calls) = DelayedCatchup::counted(false, Duration::ZERO);
        let catchup = fallback_catchup(primary, fallback);

        let start = tokio::time::Instant::now();
        catchup
            .fetch_chain_config(ChainConfig::default().commit())
            .await
            .unwrap();
        assert_eq!(start.elapsed(), FALLBACK_DELAY);
        assert_eq!(fallback_calls.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn test_fallback_immediate_without_primary_remote() {
        let local = DelayedCatchup::provider(true, Duration::from_secs(3600));
        let (fallback, fallback_calls) = DelayedCatchup::counted(false, Duration::ZERO);
        let catchup = fallback_catchup(local, fallback);

        let start = tokio::time::Instant::now();
        catchup
            .fetch_chain_config(ChainConfig::default().commit())
            .await
            .unwrap();
        assert_eq!(start.elapsed(), Duration::ZERO);
        assert_eq!(fallback_calls.load(AtomicOrdering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn test_fallback_delay_is_decided_per_call() {
        let (fallback, fallback_calls) = DelayedCatchup::counted(false, Duration::ZERO);
        let catchup = ParallelStateCatchup::new(&[], Duration::from_secs(30))
            .with_fallback_delay(FALLBACK_DELAY);
        catchup.add_fallback_provider(fallback);
        let config = ChainConfig::default();

        let start = tokio::time::Instant::now();
        catchup.fetch_chain_config(config.commit()).await.unwrap();
        assert_eq!(start.elapsed(), Duration::ZERO);

        catchup.add_provider(DelayedCatchup::provider(false, Duration::from_secs(3600)));
        let start = tokio::time::Instant::now();
        catchup.fetch_chain_config(config.commit()).await.unwrap();
        assert_eq!(start.elapsed(), FALLBACK_DELAY);
        assert_eq!(fallback_calls.load(AtomicOrdering::SeqCst), 2);
    }
}
