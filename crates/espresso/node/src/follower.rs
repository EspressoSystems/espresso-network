//! A query node that follows the chain through the light client instead of taking part in
//! consensus. Verified blocks reach the query API through the same sink a validator hands its
//! decides to, so every API module serves them unchanged.
//!
//! The light client's root of trust is its genesis stake table. Unless
//! [`FollowerOptions::light_client_genesis`] pins one, it is derived from the network config, and
//! a follower starting on an empty database fetches that config from its config peers, which
//! default to the very upstreams it verifies. Pin the genesis wherever the upstreams are not
//! trusted on first start.

use std::{cmp::max, collections::HashMap, sync::Arc, time::Duration};

use ::light_client::{
    LightClient,
    client::{FallbackClient, QueryServiceClient},
    state::{Genesis as LightClientGenesis, LightClientOptions},
    storage::LightClientDbOptions,
};
use alloy::primitives::U256;
use anyhow::{Context, anyhow, bail, ensure};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use derivative::Derivative;
use espresso_api::error::SubmitError;
use espresso_types::{
    Certificate2, ChainConfig, Header, LcContractHeader, Leaf2, NodeState, PubKey, SeqTypes,
    Transaction, ValidatedState,
    traits::MembershipPersistence,
    v0::traits::{SequencerPersistence, StateCatchup, StateCertPersistence},
    v0_1::ChainId,
};
use futures::{FutureExt, future::BoxFuture};
use hotshot_query_service::{
    availability::{BlockInfo, BlockQueryData, LeafQueryData, VidCommonQueryData},
    types::HeightIndexed,
};
use hotshot_types::{
    ValidatorConfig,
    data::{EpochNumber, VidShare, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    initializer::load_start_epoch_info,
    message::UpgradeLock,
    network::NetworkConfig,
    traits::{
        ValidatedState as _,
        election::{Membership, MembershipSnapshot},
        metrics::Metrics,
        storage::Storage,
    },
    utils::{StateAndDelta, is_epoch_root, is_transition_block},
};
use http_client::{Client, StatusCode, error::ClientErr};
use parking_lot::Mutex;
use tokio::{sync::watch, time::sleep};
use url::Url;
use versions::NEW_PROTOCOL_VERSION;

use crate::{
    CatchupParams, Genesis, L1Params, LoadedNetworkConfig, NodeStateParts, SequencerApiVersion,
    api::{
        DecideSink,
        context::{ApiContext, ConsensusSource, Delta, NodeLightClient},
        light_client_genesis,
    },
    apply_genesis_overrides,
    context::TaskList,
    init_node_state, load_or_fetch_network_config,
    startup_catchup::{
        StartupCatchupError, StartupCatchupParams, ensure_stake_tables,
        stake_table_needed_by_block, state_peer_clients,
    },
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
    pub upstreams: Vec<Url>,
    /// The upstreams when unset.
    pub config_peers: Option<Vec<Url>>,
    pub catchup: CatchupParams,
    pub startup_catchup: StartupCatchupParams,
    pub options: FollowerOptions,
}

/// What the API holds. Clones share the follower's state and own nothing that stops it; a
/// [`FollowerContext`] does that.
#[derive(Derivative, Clone)]
#[derivative(Debug(bound = ""))]
pub struct FollowerHandle<P: SequencerPersistence> {
    #[derivative(Debug = "ignore")]
    consensus: Arc<FollowerConsensus>,
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
    persistence: P,
    l1_params: L1Params,
    sink: impl DecideSink + 'static,
) -> anyhow::Result<FollowerHandle<P>>
where
    P: SequencerPersistence + MembershipPersistence,
    Arc<P>: Storage<SeqTypes>,
{
    init_with_source(genesis, params, metrics, persistence, l1_params, sink, None).await
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
        Some(source),
    )
    .await
}

async fn init_with_source<P>(
    genesis: Genesis,
    params: FollowerParams,
    metrics: Box<dyn Metrics>,
    persistence: P,
    l1_params: L1Params,
    sink: impl DecideSink + 'static,
    replay: Option<Arc<dyn ChainSource>>,
) -> anyhow::Result<FollowerHandle<P>>
where
    P: SequencerPersistence + MembershipPersistence,
    Arc<P>: Storage<SeqTypes>,
{
    params.options.validate()?;

    // A follower has no stake, so any key serves as its identity in the network config.
    let validator_config = ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
        rand::random(),
        0,
        U256::ONE,
        false,
    );
    let config_peers = params
        .config_peers
        .clone()
        .unwrap_or_else(|| params.upstreams.clone());
    let (mut network_config, fetched) = match load_or_fetch_network_config(
        &persistence,
        Some(config_peers),
        &validator_config,
        params.catchup.backoff,
        params.catchup.base_timeout,
    )
    .await?
    {
        Some(LoadedNetworkConfig::Persisted(config)) => (config, false),
        Some(LoadedNetworkConfig::Fetched(config)) => (config, true),
        None => bail!("a follower needs peers to fetch the network config from"),
    };
    apply_genesis_overrides(&mut network_config, &genesis);
    if fetched {
        persistence.save_config(&network_config).await?;
    }
    // From genesis rather than the network config, as `init_node_state` reads it: the light
    // client verifies epochs, so a follower cannot run without them.
    let epoch_height = genesis.epoch_height.unwrap_or_default();
    ensure!(
        epoch_height > 0,
        "a follower needs a positive epoch_height in genesis"
    );
    let upgrade = versions::Upgrade::new(genesis.base_version, genesis.upgrade_version);

    let NodeStateParts {
        node_state,
        state_catchup: _,
        persistence,
    } = init_node_state(
        &genesis,
        &network_config,
        l1_params,
        params.catchup.clone(),
        persistence,
        &*metrics,
    )
    .await?;
    let coordinator = node_state.coordinator.clone();

    load_start_epoch_info(
        &coordinator,
        &[],
        epoch_height,
        genesis.epoch_start_block.unwrap_or_default(),
    )
    .await;
    let state_peer_clients = state_peer_clients(&params.upstreams);
    let lc_node_state = replay.is_none().then(|| node_state.clone());

    let (source, light_client): (Arc<dyn ChainSource>, _) = match replay {
        Some(source) => (source, None),
        None => {
            let light_client = Arc::new(
                connect_light_client(
                    &params.options,
                    &params.upstreams,
                    &network_config,
                    genesis.chain_config.chain_id,
                    fetched,
                )
                .await?,
            );
            (light_client.clone(), Some(light_client))
        },
    };

    let next = sink
        .block_height()
        .await
        .context("reading the height the query database has reached")?;

    // Seeded like a validator's lock, so the API validates state certificates against the
    // version the network has upgraded to rather than the base version forever.
    let upgrade_lock =
        UpgradeLock::from_certificate(upgrade, &persistence.load_upgrade_certificate().await?);

    let (decided, decided_rx) = watch::channel(None);
    let chain_config = Arc::new(RwLock::new(genesis.chain_config));
    let consensus = FollowerConsensus {
        decided: decided_rx,
        chain_config: chain_config.clone(),
        coordinator: coordinator.clone(),
        upgrade_lock: upgrade_lock.clone(),
        upstreams: params.upstreams,
        epoch_height,
    };

    let follower = Follower {
        source,
        node_state: lc_node_state,
        sink: Arc::new(sink),
        persistence: persistence.clone(),
        coordinator,
        state_catchup: node_state.state_catchup.clone(),
        decided,
        chain_config,
        upgrade_lock,
        epoch_height,
        options: params.options,
        startup_catchup: params.startup_catchup,
        state_peer_clients,
        next,
    };
    // Boot must not depend on the upstreams: a restart with a persisted network config needs none
    // until the first poll. `follow` checks every block it stores, which covers skip-ahead.
    follower
        .ensure_next_stake_table(next)
        .await
        .context("startup stake-table catchup failed")?;
    let mut tasks = TaskList::default();
    tasks.spawn("light client follower", follower.run());

    Ok(FollowerHandle {
        consensus: Arc::new(consensus),
        light_client,
        node_state,
        persistence,
        network_config,
        tasks: Arc::new(Mutex::new(tasks)),
    })
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
struct FollowerConsensus {
    decided: watch::Receiver<Option<LeafQueryData<SeqTypes>>>,
    chain_config: Arc<RwLock<ChainConfig>>,
    coordinator: EpochMembershipCoordinator<SeqTypes>,
    upgrade_lock: UpgradeLock<SeqTypes>,
    upstreams: Vec<Url>,
    epoch_height: u64,
}

#[async_trait]
impl ConsensusSource for FollowerConsensus {
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

    async fn state(&self, _view: ViewNumber) -> Option<Arc<ValidatedState>> {
        None
    }

    async fn state_and_delta(&self, _view: ViewNumber) -> StateAndDelta<SeqTypes> {
        (None, None)
    }

    async fn undecided_leaves(&self) -> Vec<Leaf2> {
        Vec::new()
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
        let mut last_err = anyhow!("no upstream query nodes to forward the transaction to");
        for upstream in &self.upstreams {
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

    async fn update_leaf(
        &self,
        _leaf: Leaf2,
        _state: Arc<ValidatedState>,
        _delta: Option<Arc<Delta>>,
    ) -> anyhow::Result<()> {
        // The follower keeps no in-memory state per view; catchup serves from storage.
        Ok(())
    }

    async fn current_proposal_participation(&self) -> HashMap<PubKey, f64> {
        HashMap::new()
    }

    async fn proposal_participation(&self, _epoch: EpochNumber) -> HashMap<PubKey, f64> {
        HashMap::new()
    }

    async fn current_vote_participation(&self) -> HashMap<PubKey, f64> {
        HashMap::new()
    }

    async fn vote_participation(&self, _epoch: EpochNumber) -> HashMap<PubKey, f64> {
        HashMap::new()
    }
}

struct Follower<P> {
    source: Arc<dyn ChainSource>,
    node_state: Option<NodeState>,
    sink: Arc<dyn DecideSink>,
    persistence: Arc<P>,
    coordinator: EpochMembershipCoordinator<SeqTypes>,
    state_catchup: Arc<dyn StateCatchup>,
    decided: watch::Sender<Option<LeafQueryData<SeqTypes>>>,
    chain_config: Arc<RwLock<ChainConfig>>,
    upgrade_lock: UpgradeLock<SeqTypes>,
    epoch_height: u64,
    options: FollowerOptions,
    startup_catchup: StartupCatchupParams,
    state_peer_clients: Vec<(Url, QueryServiceClient)>,
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
        let from = first_height_to_ingest(self.next, tip, self.options.max_blocks_per_poll);
        if from >= tip {
            return Ok(());
        }
        if from > self.next {
            tracing::info!(
                next = self.next,
                from,
                tip,
                "skipping ahead; the query service backfills the gap"
            );
            self.next = from;
        }
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
        self.ensure_next_stake_table(leaf.height())
            .await
            .context("catching up the next stake table")?;
        self.sink.append(info).await.context("storing block")?;

        self.track_upgrade(leaf.leaf()).await;
        self.track_epoch(leaf.leaf()).await?;
        self.track_chain_config(leaf.header()).await;
        self.decided.send_replace(Some(leaf));
        Ok(())
    }

    /// As a validator does on decide, so the API validates state certificates against the version
    /// the network runs.
    async fn track_upgrade(&self, leaf: &Leaf2) {
        let Some(cert) = leaf.upgrade_certificate() else {
            return;
        };
        if self.upgrade_lock.decided_upgrade_cert().as_ref() == Some(cert) {
            return;
        }
        tracing::warn!(
            height = leaf.height(),
            version = ?cert.data.new_version,
            "the network decided an upgrade"
        );
        self.upgrade_lock.set_decided_upgrade_cert(cert.clone());
        if let Err(err) = self
            .persistence
            .store_upgrade_certificate(Some(cert.clone()))
            .await
        {
            tracing::warn!(
                height = leaf.height(),
                "cannot persist the upgrade certificate: {err:#}"
            );
        }
    }

    /// Expects the stake table of the epoch after the leaf's, which `ensure_next_stake_table`
    /// ensures. Awaited, so the transition block finds the stake table it supplies a DRB for, and a
    /// failed derivation fails the block and is retried with it. Nothing is redone when the
    /// membership already knows the result, as after a restart or a peer catchup.
    async fn track_epoch(&self, leaf: &Leaf2) -> anyhow::Result<()> {
        let height = leaf.height();
        let Some(epoch) = leaf.epoch(self.epoch_height) else {
            return Ok(());
        };
        if is_epoch_root(height, self.epoch_height) && !self.has_stake_table(epoch + 2) {
            self.coordinator
                .add_epoch_root(leaf.block_header().clone())
                .await
                .map_err(|err| anyhow!("deriving the stake table of epoch {}: {err}", epoch + 2))?;
        }
        if is_transition_block(height, self.epoch_height)
            && let Some(drb) = leaf.next_drb_result
            && !self.has_drb(epoch + 1)
        {
            self.coordinator.supply_drb(epoch + 1, drb);
        }
        Ok(())
    }

    /// Before a block at `height` is stored, the table of its epoch + 1 is held. Boot calls it for
    /// the next height the sink expects, `follow` per block.
    async fn ensure_next_stake_table(&self, height: u64) -> Result<(), StartupCatchupError> {
        let epoch = stake_table_needed_by_block(height, self.epoch_height);
        ensure_stake_tables(
            &self.coordinator,
            || read_lc_contract_header(self.node_state.as_ref()),
            &self.state_peer_clients,
            &self.startup_catchup,
            epoch..=epoch,
        )
        .await
    }

    fn has_stake_table(&self, epoch: EpochNumber) -> bool {
        self.coordinator.membership().snapshot(epoch).is_some()
    }

    fn has_drb(&self, epoch: EpochNumber) -> bool {
        self.coordinator
            .membership()
            .snapshot(epoch)
            .is_some_and(|snapshot| snapshot.has_drb())
    }

    /// Keep the resolved chain config current, so `submit` enforces the right block size.
    async fn track_chain_config(&self, header: &Header) {
        let resolvable = header.chain_config();
        if resolvable.commit() == self.chain_config.read().await.commit() {
            return;
        }
        let resolved = match resolvable.resolve() {
            Some(chain_config) => Ok(chain_config),
            None => {
                self.state_catchup
                    .fetch_chain_config(resolvable.commit())
                    .await
            },
        };
        match resolved {
            Ok(chain_config) => *self.chain_config.write().await = chain_config,
            Err(err) => tracing::warn!(
                height = header.height(),
                "cannot resolve the chain config: {err:#}"
            ),
        }
    }
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

/// The first height a poll ingests: `next`, or skipping ahead so at most `max_blocks_per_poll`
/// blocks below `tip` remain. Pure.
fn first_height_to_ingest(next: u64, tip: u64, max_blocks_per_poll: u64) -> u64 {
    max(next, tip.saturating_sub(max_blocks_per_poll))
}

/// The LC contract header, `None` when replaying a recorded chain without an LC contract.
async fn read_lc_contract_header(
    node_state: Option<&NodeState>,
) -> anyhow::Result<Option<LcContractHeader>> {
    match node_state {
        Some(node_state) => node_state.lc_contract_header().await,
        None => Ok(None),
    }
}

async fn connect_light_client(
    options: &FollowerOptions,
    upstreams: &[Url],
    network_config: &NetworkConfig<SeqTypes>,
    chain_id: ChainId,
    config_fetched: bool,
) -> anyhow::Result<NodeLightClient> {
    let db = options
        .light_client_db
        .clone()
        .connect()
        .await
        .context("opening the light client database")?;
    let client = FallbackClient::new(
        upstreams
            .iter()
            .cloned()
            .map(QueryServiceClient::new)
            .collect(),
    )?;
    let genesis = match options.light_client_genesis.clone() {
        Some(genesis) => genesis,
        None => {
            if config_fetched {
                tracing::warn!(
                    "deriving the light client's root of trust from a network config fetched from \
                     peers; pin a light client genesis unless those peers are trusted"
                );
            }
            light_client_genesis(network_config, chain_id)
        },
    };
    Ok(LightClient::from_genesis_with_options(
        db,
        client,
        genesis,
        options.light_client.clone(),
    ))
}
