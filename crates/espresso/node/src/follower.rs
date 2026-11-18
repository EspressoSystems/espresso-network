//! A query node that follows the chain through the light client instead of taking part in
//! consensus. Verified blocks reach the query API through the same sink a validator hands its
//! decides to, so every API module serves them unchanged.
//!
//! The light client's root of trust is its genesis stake table. Unless
//! [`FollowerOptions::light_client_genesis`] pins one, it is derived from the network config: the
//! genesis stake table of the genesis file, or the config a node of a network started by an
//! orchestrator stored back then. Neither comes from the upstreams it verifies.

use std::{cmp::max, collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use ::light_client::{
    LightClient, LightClientMetrics,
    client::{ClientList, FallbackClient, QueryServiceClient},
    state::{Genesis as LightClientGenesis, LightClientOptions},
    storage::LightClientDbOptions,
};
use alloy::primitives::U256;
use anyhow::{Context, anyhow, ensure};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use consensus_api::{ConsensusNodeClient, proto::SubmitTransactionRequest};
use derivative::Derivative;
use espresso_api::error::SubmitError;
use espresso_types::{
    Certificate2, ChainConfig, Header, Leaf2, NodeState, PubKey, SeqTypes, Transaction,
    ValidatedState,
    traits::MembershipPersistence,
    v0::traits::{SequencerPersistence, StateCertPersistence},
    v0_1::ChainId,
};
use futures::{FutureExt, future::BoxFuture};
use hotshot_query_service::{
    availability::{BlockInfo, BlockQueryData, LeafQueryData, VidCommonQueryData},
    types::HeightIndexed,
};
use hotshot_types::{
    ValidatorConfig,
    consensus::ParticipationTracker,
    data::{EpochNumber, VidShare},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
    traits::{ValidatedState as _, metrics::Metrics, storage::Storage},
    utils::{epoch_from_block_number, root_block_in_epoch},
};
use http_client::{Client, StatusCode, error::ClientErr};
use parking_lot::Mutex;
use tokio::{
    sync::watch,
    time::{Instant, sleep, timeout_at},
};
use url::Url;
use versions::NEW_PROTOCOL_VERSION;

use crate::{
    CatchupParams, Genesis, L1Params, NodeStateParts, SequencerApiVersion,
    api::{
        DecideSink,
        context::{ApiContext, ConsensusSource, NodeLightClient},
        light_client_genesis,
    },
    apply_genesis_overrides,
    chain_view::{ChainView, Participation},
    context::TaskList,
    init_node_state,
    lc_contract_seed::LcContractSeed,
    load_network_config,
    peers::PeerSet,
    state_signature::StateSignatureMemStorage,
};

#[derive(Clone, Debug)]
pub struct FollowerOptions {
    pub poll_interval: Duration,
    /// Older heights are left to the query service's backfill.
    pub max_blocks_per_poll: u64,
    pub light_client: LightClientOptions,
    pub light_client_db: LightClientDbOptions,
    /// Derived from the network config when unset.
    pub light_client_genesis: Option<LightClientGenesis>,
}

impl FollowerOptions {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.max_blocks_per_poll > 0,
            "max_blocks_per_poll must be at least 1: a follower ingesting no blocks per poll \
             never follows"
        );
        ensure!(
            !self.poll_interval.is_zero(),
            "poll_interval must be positive: a follower polling without pause spins on its \
             upstreams"
        );
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct FollowerParams {
    /// The API services to start from. The follower finds the others through them.
    pub bootstrap: Vec<Url>,
    pub catchup: CatchupParams,
    pub bootstrap_epoch_catchup_timeout: Duration,
    pub options: FollowerOptions,
}

/// What the API holds. Clones share the follower's state and own nothing that stops it; a
/// [`FollowerContext`] does that.
#[derive(Derivative, Clone)]
#[derivative(Debug(bound = ""))]
pub struct FollowerHandle<P: SequencerPersistence> {
    #[derivative(Debug = "ignore")]
    consensus: Arc<ApiChain>,
    #[derivative(Debug = "ignore")]
    light_client: Option<Arc<NodeLightClient>>,
    node_state: NodeState,
    #[derivative(Debug = "ignore")]
    persistence: Arc<P>,
    network_config: NetworkConfig<SeqTypes>,
    /// The follow loop, and the API tasks [`Options::serve`](crate::api::Options::serve)
    /// attaches, until a [`FollowerContext`] takes them over. Shared, because dropping a
    /// [`TaskList`] aborts its tasks and dropping a clone of the handle must not.
    #[derivative(Debug = "ignore")]
    tasks: Arc<Mutex<TaskList>>,
}

/// Owns the follower's tasks: dropping it stops following unless it was
/// [detached](Self::detach). Not `Clone`, so nothing else can stop the follower; the API holds a
/// [`FollowerHandle`].
#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub struct FollowerContext<P: SequencerPersistence> {
    handle: FollowerHandle<P>,
    tasks: TaskList,
    detached: bool,
}

/// The follow loop is running when this returns; [`FollowerContext::new`] takes over stopping it.
pub async fn init_follower_node<P>(
    genesis: Genesis,
    params: FollowerParams,
    metrics: Box<dyn Metrics>,
    light_client_metrics: LightClientMetrics,
    persistence: P,
    l1_params: L1Params,
    sink: impl DecideSink + 'static,
) -> anyhow::Result<FollowerHandle<P>>
where
    P: SequencerPersistence + MembershipPersistence,
    Arc<P>: Storage<SeqTypes>,
{
    init_with_source(
        genesis,
        params,
        metrics,
        persistence,
        l1_params,
        sink,
        Upstream::LightClient(light_client_metrics),
    )
    .await
}

/// For tests that replay a recorded chain; the network config must already be persisted, as there
/// are no peers to fetch it from.
#[cfg(any(test, feature = "testing"))]
pub(crate) async fn init_replay_node<P>(
    genesis: Genesis,
    params: FollowerParams,
    metrics: Box<dyn Metrics>,
    persistence: P,
    l1_params: L1Params,
    sink: impl DecideSink + 'static,
    source: Arc<dyn ChainSource>,
) -> anyhow::Result<FollowerHandle<P>>
where
    P: SequencerPersistence + MembershipPersistence,
    Arc<P>: Storage<SeqTypes>,
{
    init_with_source(
        genesis,
        params,
        metrics,
        persistence,
        l1_params,
        sink,
        Upstream::Replay(source),
    )
    .await
}

enum Upstream {
    LightClient(LightClientMetrics),
    #[cfg(any(test, feature = "testing"))]
    Replay(Arc<dyn ChainSource>),
}

async fn init_with_source<P>(
    genesis: Genesis,
    params: FollowerParams,
    metrics: Box<dyn Metrics>,
    persistence: P,
    l1_params: L1Params,
    sink: impl DecideSink + 'static,
    upstream: Upstream,
) -> anyhow::Result<FollowerHandle<P>>
where
    P: SequencerPersistence + MembershipPersistence,
    Arc<P>: Storage<SeqTypes>,
{
    params.options.validate()?;
    let peers = PeerSet::new(params.bootstrap.clone());
    // A replay node has no upstreams (`legacy_chain::replay_params`) and walks.
    let replay = params.bootstrap.is_empty();
    let (mut handle, view) = init_api_service(
        &genesis,
        params.catchup.clone(),
        !replay,
        &*metrics,
        persistence,
        l1_params,
        SubmitTo::Peers(peers.clone()),
    )
    .await?;
    if !replay {
        handle.tasks.lock().spawn(
            "peer set",
            peers.clone().run(
                view.coordinator.clone(),
                view.decided.subscribe(),
                view.epoch_height,
            ),
        );
    }

    let source: Arc<dyn ChainSource> = match upstream {
        #[cfg(any(test, feature = "testing"))]
        Upstream::Replay(source) => source,
        Upstream::LightClient(light_client_metrics) => {
            let light_client = Arc::new(
                connect_light_client(
                    &params.options,
                    peers.clients(),
                    &handle.network_config,
                    genesis.chain_config.chain_id,
                    light_client_metrics,
                )
                .await?,
            );
            handle.light_client = Some(light_client.clone());
            light_client
        },
    };

    let next = sink
        .block_height()
        .await
        .context("reading the height the query database has reached")?;
    if let Some(leaf) = sink
        .last_leaf()
        .await
        .context("reading the newest stored block")?
    {
        view.resume(&leaf).await;
    }
    let follower = Follower {
        source,
        sink: Arc::new(sink),
        view,
        options: params.options,
        bootstrap_epoch_catchup_timeout: params.bootstrap_epoch_catchup_timeout,
        next,
    };
    follower.catch_up_at_startup().await;
    handle
        .tasks
        .lock()
        .spawn("light client follower", follower.run());
    Ok(handle)
}

/// The API service's own view of the chain, which it keeps up with the blocks it stores: its
/// handle for the API, and the [`ChainView`] to feed each stored block to.
///
/// Its stake tables come from L1 and the blocks it stores, not from consensus, and it takes
/// transactions to `submit`.
pub(crate) async fn init_api_service<P>(
    genesis: &Genesis,
    catchup: CatchupParams,
    seed_from_lc_contract: bool,
    metrics: &dyn Metrics,
    persistence: P,
    l1_params: L1Params,
    submit: SubmitTo,
) -> anyhow::Result<(FollowerHandle<P>, Arc<ChainView<P>>)>
where
    P: SequencerPersistence + MembershipPersistence,
    Arc<P>: Storage<SeqTypes>,
{
    // The API service has no stake, so any key serves as its identity in the network config.
    let validator_config = ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
        rand::random(),
        0,
        U256::ONE,
        false,
    );
    let mut network_config =
        load_network_config(&persistence, genesis, &validator_config.public_key).await?;
    apply_genesis_overrides(&mut network_config, genesis);
    // From genesis rather than the network config, as `init_node_state` reads it.
    let epoch_height = genesis.epoch_height.unwrap_or_default();
    ensure!(
        epoch_height > 0,
        "an API service needs a positive epoch_height in genesis"
    );
    let upgrade = versions::Upgrade::new(genesis.base_version, genesis.upgrade_version);

    let NodeStateParts {
        node_state,
        state_catchup: _,
        persistence,
    } = init_node_state(
        genesis,
        &network_config,
        l1_params,
        catchup,
        persistence,
        None,
        metrics,
    )
    .await?;
    let coordinator = node_state.coordinator.clone();

    // Far behind the chain, the coordinator seeds its stake tables from the LC contract rather
    // than walking every epoch from the last one it holds.
    let backfill = seed_from_lc_contract
        .then(|| LcContractSeed::new(&node_state.chain_config, metrics))
        .flatten()
        .map(|seed| seed.install(&coordinator));
    let (handle, view) =
        api_service(node_state, persistence, network_config, upgrade, submit).await?;
    if let Some(backfill) = backfill {
        handle
            .tasks
            .lock()
            .spawn("stake table backfill", backfill.run(coordinator));
    }
    Ok((handle, view))
}

/// The API service over `node_state`, whose membership it keeps up with the blocks it stores.
pub(crate) async fn api_service<P: SequencerPersistence>(
    node_state: NodeState,
    persistence: Arc<P>,
    network_config: NetworkConfig<SeqTypes>,
    upgrade: versions::Upgrade,
    submit: SubmitTo,
) -> anyhow::Result<(FollowerHandle<P>, Arc<ChainView<P>>)> {
    let coordinator = node_state.coordinator.clone();
    let epoch_height = node_state.epoch_height.unwrap_or_default();
    // Seeded like a validator's lock, so the API validates state certificates against the
    // version the network has upgraded to rather than the base version forever.
    let upgrade_lock =
        UpgradeLock::from_certificate(upgrade, &persistence.load_upgrade_certificate().await?);

    let (decided, decided_rx) = watch::channel(None);
    let chain_config = Arc::new(RwLock::new(node_state.chain_config));
    let participation = Arc::new(Mutex::new(Participation::new(ParticipationTracker::new(
        &coordinator,
        EpochNumber::genesis(),
    ))));
    let consensus = ApiChain {
        decided: decided_rx,
        chain_config: chain_config.clone(),
        coordinator: coordinator.clone(),
        upgrade_lock: upgrade_lock.clone(),
        participation: participation.clone(),
        submit,
        epoch_height,
    };
    let view = Arc::new(ChainView {
        persistence: persistence.clone(),
        coordinator,
        state_catchup: node_state.state_catchup.clone(),
        decided,
        chain_config,
        upgrade_lock,
        epoch_height,
        participation,
    });
    let handle = FollowerHandle {
        consensus: Arc::new(consensus),
        light_client: None,
        node_state,
        persistence,
        network_config,
        tasks: Arc::new(Mutex::new(TaskList::default())),
    };
    Ok((handle, view))
}

impl<P: SequencerPersistence> FollowerContext<P> {
    /// Takes the tasks out of `handle` and its clones, so from here only this context stops the
    /// follower.
    pub fn new(handle: FollowerHandle<P>) -> Self {
        let tasks = std::mem::take(&mut *handle.tasks.lock());
        Self {
            handle,
            tasks,
            detached: false,
        }
    }

    pub fn handle(&self) -> FollowerHandle<P> {
        self.handle.clone()
    }

    /// Waits for the first decided leaf.
    pub async fn decided_leaf(&self) -> Leaf2 {
        self.handle.consensus.decided_leaf().await
    }

    pub fn node_state(&self) -> NodeState {
        self.handle.node_state.clone()
    }

    /// `None` when replaying a recorded chain.
    pub fn light_client(&self) -> Option<Arc<NodeLightClient>> {
        self.handle.light_client.clone()
    }

    pub async fn shut_down(&mut self) {
        tracing::info!("shutting down FollowerContext");
        self.tasks.shut_down();
        self.handle.node_state.l1_client.shut_down_tasks().await;
        self.detached = true;
    }

    pub async fn join(&mut self) {
        self.tasks.join().await;
    }

    /// Keep following even after this context is dropped.
    pub fn detach(&mut self) {
        self.detached = true;
    }
}

impl<P: SequencerPersistence> Drop for FollowerContext<P> {
    fn drop(&mut self) {
        if !self.detached {
            let tasks = self.tasks.clone();
            let l1_client = self.handle.node_state.l1_client.clone();
            tokio::spawn(async move {
                tracing::info!("shutting down FollowerContext");
                tasks.shut_down();
                l1_client.shut_down_tasks().await;
            });
            self.detached = true;
        }
    }
}

impl<P: SequencerPersistence> ApiContext for FollowerHandle<P> {
    fn consensus(&self) -> Arc<dyn ConsensusSource> {
        self.consensus.clone()
    }

    fn membership(&self) -> Arc<dyn MembershipPersistence> {
        self.persistence.clone()
    }

    fn state_certs(&self) -> Arc<dyn StateCertPersistence> {
        self.persistence.clone()
    }

    fn node_state(&self) -> NodeState {
        self.node_state.clone()
    }

    fn network_config(&self) -> NetworkConfig<SeqTypes> {
        self.network_config.clone()
    }

    fn validator_config(&self) -> Option<&ValidatorConfig<SeqTypes>> {
        None
    }

    fn state_signatures(&self) -> Option<Arc<RwLock<StateSignatureMemStorage>>> {
        None
    }

    fn light_client(&self) -> Option<Arc<NodeLightClient>> {
        self.light_client.clone()
    }

    fn request_vid_shares(
        &self,
        _block_number: u64,
        _vid_common: VidCommonQueryData<SeqTypes>,
        _timeout: Duration,
    ) -> BoxFuture<'static, anyhow::Result<Vec<VidShare>>> {
        futures::future::ready(Err(anyhow!(
            "a follower has no request-response protocol to fetch VID shares with"
        )))
        .boxed()
    }

    fn with_task_list(self, tasks: TaskList) -> Self {
        self.tasks.lock().extend(tasks);
        self
    }
}

/// Behind an `Arc` in [`FollowerHandle`], as the API's `ConsensusSource`.
struct ApiChain {
    decided: watch::Receiver<Option<LeafQueryData<SeqTypes>>>,
    chain_config: Arc<RwLock<ChainConfig>>,
    coordinator: EpochMembershipCoordinator<SeqTypes>,
    upgrade_lock: UpgradeLock<SeqTypes>,
    participation: Arc<Mutex<Participation>>,
    submit: SubmitTo,
    epoch_height: u64,
}

/// Where an API service hands submitted transactions.
pub(crate) enum SubmitTo {
    /// The API services of other nodes, for a follower.
    Peers(Arc<PeerSet>),

    /// The node's own consensus, at its consensus API.
    Consensus(SocketAddr),
}

#[async_trait]
impl ConsensusSource for ApiChain {
    async fn decided_leaf(&self) -> Leaf2 {
        let mut decided = self.decided.clone();
        let leaf = decided
            .wait_for(Option::is_some)
            .await
            .expect("the follower context holds the sender");
        leaf.as_ref()
            .expect("waited for a decided leaf")
            .leaf()
            .clone()
    }

    async fn decided_state(&self) -> Option<Arc<ValidatedState>> {
        let leaf = self.decided.borrow().clone()?;
        let mut state = ValidatedState::from_header(leaf.header());
        state.chain_config = (*self.chain_config.read().await).into();
        Some(Arc::new(state))
    }

    async fn current_epoch(&self) -> Option<EpochNumber> {
        let decided = self.decided.borrow();
        decided
            .as_ref()
            .and_then(|leaf| leaf.leaf().epoch(self.epoch_height))
    }

    async fn membership_coordinator(&self) -> EpochMembershipCoordinator<SeqTypes> {
        self.coordinator.clone()
    }

    async fn upgrade_lock(&self) -> UpgradeLock<SeqTypes> {
        self.upgrade_lock.clone()
    }

    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<Commitment<Transaction>> {
        let upstreams = match &self.submit {
            SubmitTo::Peers(peers) => peers.urls(),
            SubmitTo::Consensus(addr) => {
                let commitment = tx.commit();
                let mut client = ConsensusNodeClient::connect(format!("http://{addr}"))
                    .await
                    .context("connecting to the consensus node API")?;
                let request = SubmitTransactionRequest {
                    transaction: bincode::serialize(&tx)?,
                };
                return match client.submit_transaction(request).await {
                    Ok(_) => Ok(commitment),
                    Err(status) => Err(match status.code() {
                        tonic::Code::InvalidArgument => {
                            SubmitError::Invalid(status.message().into()).into()
                        },
                        tonic::Code::ResourceExhausted => {
                            SubmitError::Overloaded(status.message().into()).into()
                        },
                        _ => anyhow!("submitting to consensus: {status}"),
                    }),
                };
            },
        };
        let mut last_err = anyhow!("no upstream query nodes to forward the transaction to");
        for upstream in upstreams.iter() {
            let client = Client::<ClientErr, SequencerApiVersion>::new(upstream.clone());
            let result = client
                .post::<Commitment<Transaction>>("submit/submit")
                .body_binary(&tx)?
                .send()
                .await;
            match result {
                Ok(commitment) => return Ok(commitment),
                Err(err) => {
                    tracing::warn!(%upstream, %err, "forwarding transaction failed");
                    match err.status {
                        // Every upstream would refuse it.
                        StatusCode::BAD_REQUEST => {
                            return Err(SubmitError::Invalid(err.message).into());
                        },
                        StatusCode::SERVICE_UNAVAILABLE => {
                            last_err = SubmitError::Overloaded(err.message).into();
                        },
                        _ => {
                            last_err = anyhow::Error::from(err)
                                .context(format!("forwarding to {upstream}"));
                        },
                    }
                },
            }
        }
        Err(last_err)
    }

    async fn current_proposal_participation(&self) -> HashMap<PubKey, f64> {
        self.participation
            .lock()
            .tracker
            .current_proposal_participation()
    }

    async fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        self.participation
            .lock()
            .tracker
            .proposal_participation(epoch)
    }

    async fn current_vote_participation(&self) -> HashMap<PubKey, f64> {
        self.participation
            .lock()
            .tracker
            .current_vote_participation()
    }

    async fn vote_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        self.participation.lock().tracker.vote_participation(epoch)
    }
}

struct Follower<P> {
    source: Arc<dyn ChainSource>,
    sink: Arc<dyn DecideSink>,
    view: Arc<ChainView<P>>,
    options: FollowerOptions,
    bootstrap_epoch_catchup_timeout: Duration,
    next: u64,
}

/// Where a follower reads the chain from.
#[async_trait]
pub(crate) trait ChainSource: Send + Sync {
    async fn block_height(&self) -> anyhow::Result<u64>;

    /// The blocks at heights `from..to`, in order.
    async fn fetch_range(&self, from: u64, to: u64) -> anyhow::Result<Vec<VerifiedBlock>>;

    /// The cert2 that finalized `header` directly, if any.
    async fn fetch_cert2(&self, header: &Header) -> anyhow::Result<Option<Certificate2<SeqTypes>>>;
}

pub(crate) struct VerifiedBlock {
    pub(crate) leaf: LeafQueryData<SeqTypes>,
    pub(crate) block: BlockQueryData<SeqTypes>,
    pub(crate) vid_common: VidCommonQueryData<SeqTypes>,
}

impl<P: SequencerPersistence> Follower<P> {
    async fn run(mut self) {
        tracing::info!(from = self.next, "following the chain");
        loop {
            if let Err(err) = self.poll().await {
                tracing::warn!(
                    next = self.next,
                    err = %format!("{err:#}"),
                    "following the chain failed"
                );
            }
            sleep(self.options.poll_interval).await;
        }
    }

    async fn poll(&mut self) -> anyhow::Result<()> {
        let tip = self
            .source
            .block_height()
            .await
            .context("fetching the chain height")?;
        let from = self.ingest_start(tip);
        if from >= tip {
            return Ok(());
        }
        if from > self.next {
            tracing::info!(
                next = self.next,
                from,
                tip,
                "skipping ahead; the query service backfills the gap above its pruned height"
            );
            self.next = from;
        }
        let deadline = Instant::now() + self.bootstrap_epoch_catchup_timeout;
        self.require_stake_table(from, deadline).await?;
        for block in self.source.fetch_range(from, tip).await? {
            let height = block.leaf.height();
            ensure!(
                height == self.next,
                "the light client returned block {height} where {} was expected",
                self.next
            );
            self.follow(block)
                .await
                .with_context(|| format!("following block {height}"))?;
            self.next = height + 1;
        }
        Ok(())
    }

    /// The height a poll against chain height `tip` ingests from.
    fn ingest_start(&self, tip: u64) -> u64 {
        max(
            self.next,
            tip.saturating_sub(self.options.max_blocks_per_poll),
        )
    }

    async fn follow(&self, block: VerifiedBlock) -> anyhow::Result<()> {
        let VerifiedBlock {
            leaf,
            block,
            vid_common,
        } = block;
        let mut info = BlockInfo::new(leaf.clone(), Some(block), Some(vid_common), None);
        // Upstream stores a cert2 only at the heights it finalized directly, so most answers
        // are `None`; asking at every height is what keeps the follower serving the same cert2s.
        if leaf.header().version() >= NEW_PROTOCOL_VERSION
            && let Some(cert2) = self
                .source
                .fetch_cert2(leaf.header())
                .await
                .context("fetching cert2")?
        {
            info = info.with_cert2(cert2);
        }
        self.sink
            .append(info, None)
            .await
            .context("storing block")?;

        self.view.track(&leaf).await
    }

    /// Requests the stake tables and the DRB the first poll needs, within one catchup timeout. A
    /// failure is logged and the first poll retries it. When the upstream height read fails, this
    /// targets the stored height, which can differ from where the first poll starts.
    async fn catch_up_at_startup(&self) {
        let deadline = Instant::now() + self.bootstrap_epoch_catchup_timeout;
        let Some(from) = self.start_height(deadline).await else {
            tracing::warn!("no chain height known; the first poll requests the stake tables");
            return;
        };
        if let Err(err) = self.require_stake_table(from, deadline).await {
            tracing::warn!(from, "startup stake table catchup failed: {err:#}");
        }
        // Background, so endpoints that need this epoch's DRB find it soon.
        let epoch = epoch_from_block_number(from, self.view.epoch_height);
        let _ = self
            .view
            .coordinator
            .membership_for_epoch(EpochNumber::new(epoch));
    }

    /// The height the first poll ingests from: the upstreams' when they answer by `deadline`, else
    /// the stored one, `None` for an empty database.
    async fn start_height(&self, deadline: Instant) -> Option<u64> {
        match timeout_at(deadline, self.source.block_height()).await {
            Ok(Ok(tip)) => return Some(self.ingest_start(tip)),
            Ok(Err(err)) => tracing::warn!("cannot read the chain height: {err:#}"),
            Err(_) => tracing::warn!("reading the chain height timed out"),
        }
        (self.next > 0).then_some(self.next)
    }

    /// Holds the stake tables that following from `height` needs, waiting until `deadline`. Far
    /// behind, the coordinator seeds. A failure or timeout is not an error while the table is
    /// held: the DRB step after the install can fail or run long, and a follower supplies DRB
    /// results itself from the next transition block on.
    async fn require_stake_table(&self, height: u64, deadline: Instant) -> anyhow::Result<()> {
        let needed = needed_stake_table_epoch(height, self.view.epoch_height);
        if needed <= self.view.coordinator.membership().first_epoch() + 1 {
            return Ok(());
        }
        let outcome =
            match timeout_at(deadline, self.view.coordinator.wait_for_stake_table(needed)).await {
                Ok(Ok(_)) => return Ok(()),
                Ok(Err(err)) => format!("failed: {err}"),
                Err(_) => "timed out".to_string(),
            };
        ensure!(
            self.view.has_stake_table(needed),
            "the catchup to the stake table of epoch {needed} {outcome}"
        );
        Ok(())
    }
}

/// The newest stake table following from `height` needs: the next epoch's and, once the epoch's
/// root block lies below `height`, the one after.
fn needed_stake_table_epoch(height: u64, epoch_height: u64) -> EpochNumber {
    let epoch = epoch_from_block_number(height, epoch_height);
    let root_skipped = root_block_in_epoch(epoch, epoch_height) < height;
    EpochNumber::new(epoch + 1 + u64::from(root_skipped))
}

#[async_trait]
impl ChainSource for NodeLightClient {
    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(NodeLightClient::block_height(self).await?)
    }

    /// One finality proof and one payload proof request cover the whole range.
    async fn fetch_range(&self, from: u64, to: u64) -> anyhow::Result<Vec<VerifiedBlock>> {
        let (from, to) = (from as usize, to as usize);
        let leaves = self
            .fetch_leaves_in_range(from, to)
            .await
            .context("fetching leaves")?;
        let blocks = self
            .fetch_blocks_and_vid_common_in_range(from, to)
            .await
            .context("fetching payloads")?;
        ensure!(
            leaves.len() == to - from && blocks.len() == to - from,
            "the light client returned {} leaves and {} payloads for {} blocks",
            leaves.len(),
            blocks.len(),
            to - from
        );
        Ok(leaves
            .into_iter()
            .zip(blocks)
            .map(|(leaf, (block, vid_common))| VerifiedBlock {
                leaf,
                block,
                vid_common,
            })
            .collect())
    }

    async fn fetch_cert2(&self, header: &Header) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        Ok(self.fetch_certificate2_for_header(header).await?)
    }
}

async fn connect_light_client(
    options: &FollowerOptions,
    clients: ClientList<QueryServiceClient>,
    network_config: &NetworkConfig<SeqTypes>,
    chain_id: ChainId,
    metrics: LightClientMetrics,
) -> anyhow::Result<NodeLightClient> {
    let db = options
        .light_client_db
        .clone()
        .connect()
        .await
        .context("opening the light client database")?;
    let client = FallbackClient::shared(clients)?;
    let genesis = match options.light_client_genesis.clone() {
        Some(genesis) => genesis,
        None => light_client_genesis(network_config, chain_id),
    };
    Ok(
        LightClient::from_genesis_with_options(db, client, genesis, options.light_client.clone())
            .with_metrics(metrics),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const EPOCH_HEIGHT: u64 = 100;

    #[rstest]
    // Epoch 1 spans blocks 1..=100 and its root block is 95.
    #[case::genesis(0, 2)]
    #[case::first_epoch_root(95, 2)]
    #[case::first_epoch_after_root(96, 3)]
    // Epoch 2 spans blocks 101..=200 and its root block is 195.
    #[case::before_root(194, 3)]
    #[case::root(195, 3)]
    #[case::after_root(196, 4)]
    #[case::last_block_of_epoch(200, 4)]
    #[case::first_block_of_next_epoch(201, 4)]
    fn needed_epoch_at_height(#[case] height: u64, #[case] expected: u64) {
        assert_eq!(
            needed_stake_table_epoch(height, EPOCH_HEIGHT),
            EpochNumber::new(expected)
        );
    }
}
