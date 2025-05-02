use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display},
    sync::Arc,
    time::Duration,
};

use alloy::primitives::U256;
use anyhow::{anyhow, bail, ensure, Context};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use espresso_types::{
    config::PublicNetworkConfig,
    traits::SequencerPersistence,
    v0::traits::StateCatchup,
    v0_1::{RewardAccount, RewardAccountProof, RewardMerkleCommitment, RewardMerkleTree},
    v0_99::ChainConfig,
    BackoffParams, BlockMerkleTree, EpochVersion, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    FeeMerkleTree, Leaf2, NodeState, PubKey, SeqTypes, SequencerVersions, ValidatedState,
};
use futures::{
    future::{Future, FutureExt, TryFuture, TryFutureExt},
    stream::FuturesUnordered,
    StreamExt,
};
use hotshot_types::{
    consensus::Consensus,
    data::ViewNumber,
    message::UpgradeLock,
    network::NetworkConfig,
    traits::{
        metrics::{Counter, CounterFamily, Metrics},
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime as _, NodeType, Versions},
        ValidatedState as ValidatedStateTrait,
    },
    utils::{verify_leaf_chain, View, ViewInner},
    PeerConfig, ValidatorConfig,
};
use itertools::Itertools;
use jf_merkle_tree::{prelude::MerkleNode, ForgetableMerkleTreeScheme, MerkleTreeScheme};
use parking_lot::Mutex;
use priority_queue::PriorityQueue;
use serde::de::DeserializeOwned;
use surf_disco::Request;
use tide_disco::error::ServerError;
use tokio::time::timeout;
use tokio_util::task::AbortOnDropHandle;
use tracing::warn;
use url::Url;
use vbs::version::StaticVersionType;

use crate::api::BlocksFrontier;

// This newtype is probably not worth having. It's only used to be able to log
// URLs before doing requests.
#[derive(Debug, Clone)]
struct Client<ServerError, ApiVer: StaticVersionType> {
    inner: surf_disco::Client<ServerError, ApiVer>,
    url: Url,
    requests: Arc<Box<dyn Counter>>,
    failures: Arc<Box<dyn Counter>>,
}

impl<ApiVer: StaticVersionType> Client<ServerError, ApiVer> {
    pub fn new(
        url: Url,
        requests: &(impl CounterFamily + ?Sized),
        failures: &(impl CounterFamily + ?Sized),
    ) -> Self {
        Self {
            inner: surf_disco::Client::new(url.clone()),
            requests: Arc::new(requests.create(vec![url.to_string()])),
            failures: Arc::new(failures.create(vec![url.to_string()])),
            url,
        }
    }

    pub fn get<T: DeserializeOwned>(&self, route: &str) -> Request<T, ServerError, ApiVer> {
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
    clients: Vec<Client<ServerError, ApiVer>>,
    backoff: BackoffParams,
}

impl<ApiVer: StaticVersionType> StatePeers<ApiVer> {
    async fn fetch<Fut>(
        &self,
        retry: usize,
        f: impl Fn(Client<ServerError, ApiVer>) -> Fut,
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
        // honest peer, and thus never make progress. Thus, we start with a timeout of 500ms, which
        // is aggressive but still very reasonable for an HTTP request. If that fails with all of
        // our peers, we increase the timeout by 1 second for each successive retry, until we
        // eventually succeed.
        let timeout_dur = Duration::from_millis(500) * (retry as u32 + 1);

        // Keep track of which peers we make requests to and which succeed (`true`) or fail (`false`),
        // so we can update reliability scores at the end.
        let mut requests = HashMap::new();
        let mut res = Err(anyhow!("failed fetching from every peer"));

        // Try each peer in order of reliability score, until we succeed. We clone out of
        // `self.scores` because it is small (contains only numeric IDs and scores), so this clone
        // is a lot cheaper than holding the read lock the entire time we are making requests (which
        // could be a while).
        let mut scores = { (*self.scores.read().await).clone() };
        while let Some((id, score)) = scores.pop() {
            let client = &self.clients[id];
            tracing::info!("fetching from {}", client.url);
            match timeout(timeout_dur, f(client.clone()).into_future()).await {
                Ok(Ok(t)) => {
                    requests.insert(id, true);
                    res = Ok(t);
                    break;
                },
                Ok(Err(err)) => {
                    tracing::warn!(id, ?score, peer = %client.url, "error from peer: {err:#}");
                    requests.insert(id, false);
                },
                Err(_) => {
                    tracing::warn!(id, ?score, peer = %client.url, ?timeout_dur, "request timed out");
                    requests.insert(id, false);
                },
            }
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
                    let cfg = provider
                        .fetch(retry, |client| {
                            client.get::<PublicNetworkConfig>("config/hotshot").send()
                        })
                        .await?;
                    cfg.into_network_config(my_own_validator_config)
                        .context("fetched config, but failed to convert to private config")
                }
                .boxed()
            })
            .await
    }
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
                .post::<FeeMerkleTree>(&format!("catchup/{height}/{}/accounts", view.u64()))
                .body_binary(&accounts.to_vec())?
                .send()
                .await?;

            // Verify proofs.
            let mut proofs = Vec::new();
            for account in accounts {
                let (proof, _) = FeeAccountProof::prove(&tree, (*account).into())
                    .context(format!("response missing account {account}"))?;
                proof
                    .verify(&fee_merkle_tree_root)
                    .context(format!("invalid proof for accoujnt {account}"))?;
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
                        .get::<BlocksFrontier>(&format!("catchup/{height}/{}/blocks", view.u64()))
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
                .get::<ChainConfig>(&format!("catchup/chain-config/{}", commitment))
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
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        // Get the leaf chain
        let leaf_chain = self
            .fetch(retry, |client| async move {
                let leaf = client
                    .get::<Vec<Leaf2>>(&format!("catchup/{}/leafchain", height))
                    .send()
                    .await?;
                anyhow::Ok(leaf)
            })
            .await
            .with_context(|| format!("failed to fetch leaf chain at height {height}"))?;

        // Verify it, returning the leaf at the given height
        verify_leaf_chain(
            leaf_chain,
            &stake_table,
            success_threshold,
            height,
            &UpgradeLock::<SeqTypes, SequencerVersions<EpochVersion, EpochVersion>>::new(),
        )
        .await
        .with_context(|| format!("failed to verify leaf chain at height {height}"))
    }

    #[tracing::instrument(skip(self, _instance))]
    async fn try_fetch_reward_accounts(
        &self,
        retry: usize,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        self.fetch(retry, |client| async move {
            let tree = client
                .inner
                .post::<RewardMerkleTree>(&format!(
                    "catchup/{height}/{}/reward-accounts",
                    view.u64()
                ))
                .body_binary(&accounts.to_vec())?
                .send()
                .await?;

            // Verify proofs.
            // Verify proofs.
            let mut proofs = Vec::new();
            for account in accounts {
                let (proof, _) = RewardAccountProof::prove(&tree, (*account).into())
                    .context(format!("response missing account {account}"))?;
                proof
                    .verify(&reward_merkle_tree_root)
                    .context(format!("invalid proof for reward account {account}"))?;
                proofs.push(proof);
            }

            anyhow::Ok(proofs)
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

    fn get_reward_accounts(
        &self,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _accounts: &[RewardAccount],
    ) -> impl Send + Future<Output = anyhow::Result<(RewardMerkleTree, Leaf2)>> {
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

    async fn get_reward_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<(RewardMerkleTree, Leaf2)> {
        self.inner()
            .get_reward_accounts(instance, height, view, accounts)
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

    async fn get_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.inner().get_chain_config(commitment).await
    }
    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        self.inner().get_leaf_chain(height).await
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
    T: CatchupStorage + Send + Sync,
{
    async fn try_fetch_leaf(
        &self,
        _retry: usize,
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        // Get the leaf chain
        let leaf_chain = self.db.get_leaf_chain(height).await?;

        // Verify the leaf chain
        let leaf = verify_leaf_chain(
            leaf_chain,
            &stake_table,
            success_threshold,
            height,
            &UpgradeLock::<SeqTypes, SequencerVersions<EpochVersion, EpochVersion>>::new(),
        )
        .await
        .with_context(|| "failed to verify leaf chain")?;

        Ok(leaf)
    }
    // TODO: add a test for the account proof validation
    // issue # 2102 (https://github.com/EspressoSystems/espresso-sequencer/issues/2102)
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
        // Get the accounts
        let (fee_merkle_tree_from_db, _) = self
            .db
            .get_accounts(instance, block_height, view, accounts)
            .await
            .with_context(|| "failed to get reward accounts from DB")?;

        // Verify the accounts
        let mut proofs = Vec::new();
        for account in accounts {
            let (proof, _) = FeeAccountProof::prove(&fee_merkle_tree_from_db, (*account).into())
                .context(format!("response missing account {account}"))?;
            proof
                .verify(&fee_merkle_tree_root)
                .context(format!("invalid proof for account {account}"))?;
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

        let proof = self.db.get_frontier(instance, bh, view).await?;
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
                "Critical error: Mismatched chain config detected. Expected chain config: {:?}, but got: {:?}.
                This may indicate a compromised database",
                commitment,
                cf.commit()
            )
        }

        Ok(cf)
    }

    #[tracing::instrument(skip(self, _retry, instance))]
    async fn try_fetch_reward_accounts(
        &self,
        _retry: usize,
        instance: &NodeState,
        block_height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        // Get the accounts
        let (reward_merkle_tree_from_db, _) = self
            .db
            .get_reward_accounts(instance, block_height, view, accounts)
            .await
            .with_context(|| "failed to get reward accounts from DB")?;

        // Verify the accounts
        let mut proofs = Vec::new();
        for account in accounts {
            let (proof, _) =
                RewardAccountProof::prove(&reward_merkle_tree_from_db, (*account).into())
                    .context(format!("response missing account {account}"))?;
            proof
                .verify(&reward_merkle_tree_root)
                .context(format!("invalid proof for account {account}"))?;
            proofs.push(proof);
        }

        Ok(proofs)
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
        _height: u64,
        _stake_table: Vec<PeerConfig<SeqTypes>>,
        _success_threshold: U256,
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

    async fn try_fetch_reward_accounts(
        &self,
        _retry: usize,
        _instance: &NodeState,
        _height: u64,
        _view: ViewNumber,
        _fee_merkle_tree_root: RewardMerkleCommitment,
        _account: &[RewardAccount],
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
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

/// A catchup implementation that parallelizes requests to many providers.
/// It returns the result of the first non-erroring provider to complete.
#[derive(Clone)]
pub struct ParallelStateCatchup {
    providers: Arc<Mutex<Vec<Arc<dyn StateCatchup>>>>,
}

impl ParallelStateCatchup {
    /// Create a new [`ParallelStateCatchup`] with two providers.
    pub fn new(providers: &[Arc<dyn StateCatchup>]) -> Self {
        Self {
            providers: Arc::new(Mutex::new(providers.to_vec())),
        }
    }

    /// Add a provider to the list of providers
    pub fn add_provider(&self, provider: Arc<dyn StateCatchup>) {
        self.providers.lock().push(provider);
    }

    /// Perform an async operation on all providers, returning the first result to succeed
    pub async fn on_all_providers<C, F, RT>(&self, closure: C) -> anyhow::Result<RT>
    where
        C: Fn(Arc<dyn StateCatchup>) -> F + Clone + Send + Sync + 'static,
        F: Future<Output = anyhow::Result<RT>> + Send + 'static,
        RT: Send + Sync + 'static,
    {
        // Make sure we have at least one provider
        let providers = self.providers.lock().clone();
        if providers.is_empty() {
            return Err(anyhow::anyhow!("no providers were initialized"));
        }

        // Spawn futures for each provider
        let mut futures = FuturesUnordered::new();
        for provider in providers {
            let closure = closure.clone();
            futures.push(AbortOnDropHandle::new(tokio::spawn(closure(provider))));
        }

        // Return the first successful result
        while let Some(result) = futures.next().await {
            // Unwrap the inner (join) result
            let result = match result {
                Ok(res) => res,
                Err(err) => {
                    warn!("Failed to join on provider: {err:#}. Trying next provider...");
                    continue;
                },
            };

            // If a provider fails, print why
            let result = match result {
                Ok(res) => res,
                Err(err) => {
                    warn!("Failed to fetch data: {err:#}. Trying next provider...");
                    continue;
                },
            };

            return Ok(result);
        }

        Err(anyhow::anyhow!("no providers returned a successful result"))
    }
}

/// A catchup implementation that parallelizes requests to a local and remote provider.
/// It returns the result of the first provider to complete.
#[async_trait]
impl StateCatchup for ParallelStateCatchup {
    async fn try_fetch_leaf(
        &self,
        retry: usize,
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        // Try fetching the leaf on all providers
        self.on_all_providers(move |provider| {
            // Clone things we need in the closure
            let stake_table_clone = stake_table.clone();

            async move {
                provider
                    .try_fetch_leaf(retry, height, stake_table_clone, success_threshold)
                    .await
            }
        })
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
        let instance_clone = instance.clone();
        let accounts_vec = accounts.to_vec();
        self.on_all_providers(move |provider| {
            // Clone things we need in the closure
            let instance_clone = instance_clone.clone();
            let accounts_clone = accounts_vec.clone();

            async move {
                provider
                    .try_fetch_accounts(
                        retry,
                        &instance_clone,
                        height,
                        view,
                        fee_merkle_tree_root,
                        &accounts_clone,
                    )
                    .await
            }
        })
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
        let mt_clone = mt.clone();
        let instance_clone = instance.clone();

        let merkle_tree = self
            .on_all_providers(move |provider| {
                let instance_clone = instance_clone.clone();
                let mut mt_clone = mt_clone.clone();

                async move {
                    provider
                        .try_remember_blocks_merkle_tree(
                            retry,
                            &instance_clone,
                            height,
                            view,
                            &mut mt_clone,
                        )
                        .await?;
                    Ok(mt_clone)
                }
            })
            .await
            .with_context(|| "failed to remember blocks merkle tree")?;

        // Update the original, local merkle tree
        *mt = merkle_tree;

        Ok(())
    }

    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.on_all_providers(move |provider| async move {
            provider.try_fetch_chain_config(retry, commitment).await
        })
        .await
    }

    async fn try_fetch_reward_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        let instance_clone = instance.clone();
        let accounts_vec = accounts.to_vec();
        self.on_all_providers(move |provider| {
            let instance_clone = instance_clone.clone();
            let accounts_clone = accounts_vec.clone();

            async move {
                provider
                    .try_fetch_reward_accounts(
                        retry,
                        &instance_clone,
                        height,
                        view,
                        reward_merkle_tree_root,
                        &accounts_clone,
                    )
                    .await
            }
        })
        .await
    }

    fn backoff(&self) -> &BackoffParams {
        unreachable!()
    }

    fn name(&self) -> String {
        format!(
            "[{}]",
            self.providers
                .lock()
                .iter()
                .map(|p| p.name())
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
        let instance_clone = instance.clone();
        self.on_all_providers(move |provider| {
            // Clone things we need in the closure
            let instance_clone = instance_clone.clone();
            let accounts_clone = accounts.clone();

            async move {
                provider
                    .fetch_accounts(
                        &instance_clone,
                        height,
                        view,
                        fee_merkle_tree_root,
                        accounts_clone,
                    )
                    .await
            }
        })
        .await
    }

    async fn fetch_leaf(
        &self,
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        self.on_all_providers(move |provider| {
            let stake_table_clone = stake_table.clone();

            async move {
                provider
                    .fetch_leaf(height, stake_table_clone, success_threshold)
                    .await
            }
        })
        .await
    }

    async fn fetch_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.on_all_providers(move |provider| async move {
            provider.fetch_chain_config(commitment).await
        })
        .await
    }

    async fn fetch_reward_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: Vec<RewardAccount>,
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        let instance_clone = instance.clone();
        self.on_all_providers(move |provider| {
            let instance_clone = instance_clone.clone();
            let accounts_clone = accounts.clone();

            async move {
                provider
                    .fetch_reward_accounts(
                        &instance_clone,
                        height,
                        view,
                        reward_merkle_tree_root,
                        accounts_clone,
                    )
                    .await
            }
        })
        .await
    }

    async fn remember_blocks_merkle_tree(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        let mt_clone = mt.clone();
        let instance_clone = instance.clone();

        let merkle_tree = self
            .on_all_providers(move |provider| {
                let instance_clone = instance_clone.clone();
                let mut mt_clone = mt_clone.clone();

                async move {
                    provider
                        .remember_blocks_merkle_tree(&instance_clone, height, view, &mut mt_clone)
                        .await?;
                    Ok(mt_clone)
                }
            })
            .await
            .with_context(|| "failed to remember blocks merkle tree")?;

        // Update the original, local merkle tree
        *mt = merkle_tree;

        Ok(())
    }

    fn is_local(&self) -> bool {
        self.providers.lock().iter().all(|p| p.is_local())
    }
}

/// Add accounts to the in-memory consensus state.
/// We use this during catchup after receiving verified accounts.
#[allow(clippy::type_complexity)]
pub async fn add_fee_accounts_to_state<
    N: ConnectedNetwork<PubKey>,
    V: Versions,
    P: SequencerPersistence,
>(
    consensus: &Arc<RwLock<Consensus<SeqTypes>>>,
    view: &<SeqTypes as NodeType>::View,
    accounts: &[FeeAccount],
    tree: &FeeMerkleTree,
    leaf: Leaf2,
) -> anyhow::Result<()> {
    // Get the consensus handle
    let mut consensus = consensus.write().await;

    let (state, delta) = match consensus.validated_state_map().get(view) {
        Some(View {
            view_inner: ViewInner::Leaf { state, delta, .. },
        }) => {
            let mut state = (**state).clone();

            // Add the fetched accounts to the state.
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

            (Arc::new(state), delta.clone())
        },
        _ => {
            // If we don't already have a leaf for this view, or if we don't have the view
            // at all, we can create a new view based on the recovered leaf and add it to
            // our state map. In this case, we must also add the leaf to the saved leaves
            // map to ensure consistency.
            let mut state = ValidatedState::from_header(leaf.block_header());
            state.fee_merkle_tree = tree.clone();
            (Arc::new(state), None)
        },
    };

    consensus
        .update_leaf(leaf, Arc::clone(&state), delta)
        .with_context(|| "failed to update leaf")?;

    Ok(())
}

/// Add accounts to the in-memory consensus state.
/// We use this during catchup after receiving verified accounts.
#[allow(clippy::type_complexity)]
pub async fn add_reward_accounts_to_state<
    N: ConnectedNetwork<PubKey>,
    V: Versions,
    P: SequencerPersistence,
>(
    consensus: &Arc<RwLock<Consensus<SeqTypes>>>,
    view: &<SeqTypes as NodeType>::View,
    accounts: &[RewardAccount],
    tree: &RewardMerkleTree,
    leaf: Leaf2,
) -> anyhow::Result<()> {
    // Get the consensus handle
    let mut consensus = consensus.write().await;

    let (state, delta) = match consensus.validated_state_map().get(view) {
        Some(View {
            view_inner: ViewInner::Leaf { state, delta, .. },
        }) => {
            let mut state = (**state).clone();

            // Add the fetched accounts to the state.
            for account in accounts {
                if let Some((proof, _)) = RewardAccountProof::prove(tree, (*account).into()) {
                    if let Err(err) = proof.remember(&mut state.reward_merkle_tree) {
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

            (Arc::new(state), delta.clone())
        },
        _ => {
            // If we don't already have a leaf for this view, or if we don't have the view
            // at all, we can create a new view based on the recovered leaf and add it to
            // our state map. In this case, we must also add the leaf to the saved leaves
            // map to ensure consistency.
            let mut state = ValidatedState::from_header(leaf.block_header());
            state.reward_merkle_tree = tree.clone();
            (Arc::new(state), None)
        },
    };

    consensus
        .update_leaf(leaf, Arc::clone(&state), delta)
        .with_context(|| "failed to update leaf")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

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
}
