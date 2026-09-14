//! A query node that follows the chain through the light client instead of taking part in
//! consensus. Verified blocks reach the query API through the same sink a validator hands its
//! decides to, so every API module serves them unchanged.
//!
//! The light client's root of trust is its genesis stake table. Unless
//! [`FollowerOptions::light_client_genesis`] pins one, it is derived from the network config, and
//! a follower starting on an empty database fetches that config from its config peers, which
//! default to the very upstreams it verifies. Pin the genesis wherever the upstreams are not
//! trusted on first start.

use std::{cmp::max, collections::HashMap, ops::Range, sync::Arc, time::Duration};

use ::light_client::{
    LightClient,
    client::{FallbackClient, QueryServiceClient},
    state::{Genesis as LightClientGenesis, LightClientOptions},
    storage::LightClientSqliteOptions,
};
use alloy::primitives::U256;
use anyhow::{Context, anyhow, bail, ensure};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use derivative::Derivative;
use espresso_types::{
    ChainConfig, Header, Leaf2, NodeState, PubKey, SeqTypes, Transaction, ValidatedState,
    traits::MembershipPersistence,
    v0::traits::{SequencerPersistence, StateCatchup},
    v0_1::ChainId,
};
use futures::{FutureExt, future::BoxFuture};
use hotshot_events_service::events_source::EventsStreamer;
use hotshot_query_service::{
    availability::{BlockInfo, BlockQueryData, LeafQueryData, VidCommonQueryData},
    types::HeightIndexed,
};
use hotshot_types::{
    ValidatorConfig,
    data::{EpochNumber, VidShare, ViewNumber},
    drb::INITIAL_DRB_RESULT,
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
    traits::{
        ValidatedState as _,
        election::{Membership, MembershipSnapshot},
        metrics::Metrics,
        storage::Storage,
    },
    utils::{
        StateAndDelta, epoch_from_block_number, is_epoch_root, is_transition_block,
        root_block_in_epoch,
    },
};
use http_client::{Client, error::ClientErr};
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
    startup_catchup::bootstrap_epoch_window,
    state_signature::StateSigner,
};

#[derive(Clone, Debug)]
pub struct FollowerOptions {
    pub poll_interval: Duration,
    /// Older heights are left to the query service's backfill.
    pub max_blocks_per_poll: u64,
    pub light_client: LightClientOptions,
    pub light_client_db: LightClientSqliteOptions,
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
    pub bootstrap_epoch_catchup_timeout: Duration,
    pub options: FollowerOptions,
}

#[derive(Derivative, Clone)]
#[derivative(Debug(bound = ""))]
pub struct FollowerContext<P: SequencerPersistence> {
    #[derivative(Debug = "ignore")]
    consensus: Arc<FollowerConsensus>,
    #[derivative(Debug = "ignore")]
    light_client: Arc<NodeLightClient>,
    node_state: NodeState,
    #[derivative(Debug = "ignore")]
    persistence: Arc<P>,
    network_config: NetworkConfig<SeqTypes>,
    #[derivative(Debug = "ignore")]
    decided: Arc<watch::Sender<Option<LeafQueryData<SeqTypes>>>>,
    #[derivative(Debug = "ignore")]
    sink: Arc<dyn DecideSink>,
    options: FollowerOptions,
    bootstrap_epoch_catchup_timeout: Duration,
    tasks: TaskList,
    detached: bool,
}

/// Following begins with [`FollowerContext::start`].
pub async fn init_follower_node<P>(
    genesis: Genesis,
    params: FollowerParams,
    metrics: Box<dyn Metrics>,
    persistence: P,
    l1_params: L1Params,
    sink: impl DecideSink + 'static,
) -> anyhow::Result<FollowerContext<P>>
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
    let epoch_height = network_config.config.epoch_height;
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

    seed_first_epoch(&coordinator, &network_config);
    if epoch_height > 0 {
        let current_epoch = bootstrap_epoch_window(
            &coordinator,
            epoch_height,
            params.bootstrap_epoch_catchup_timeout,
        )
        .await
        .context("startup stake-table catchup failed")?;
        tracing::info!(%current_epoch, "startup catchup complete");
    }

    let light_client = connect_light_client(
        &params.options,
        &params.upstreams,
        &network_config,
        genesis.chain_config.chain_id,
        fetched,
    )
    .await?;

    // Seeded like a validator's lock, so the API validates state certificates against the
    // version the network has upgraded to rather than the base version forever.
    let upgrade_lock =
        UpgradeLock::from_certificate(upgrade, &persistence.load_upgrade_certificate().await?);

    let (decided, decided_rx) = watch::channel(None);
    let chain_config = Arc::new(RwLock::new(genesis.chain_config));
    let consensus = FollowerConsensus {
        decided: decided_rx,
        chain_config,
        coordinator,
        upgrade_lock,
        upstreams: params.upstreams,
        epoch_height,
    };
    Ok(FollowerContext {
        consensus: Arc::new(consensus),
        light_client: Arc::new(light_client),
        node_state,
        persistence,
        network_config,
        decided: Arc::new(decided),
        sink: Arc::new(sink),
        options: params.options,
        bootstrap_epoch_catchup_timeout: params.bootstrap_epoch_catchup_timeout,
        tasks: TaskList::default(),
        detached: false,
    })
}

impl<P: SequencerPersistence> FollowerContext<P> {
    pub async fn start(&mut self) -> anyhow::Result<()> {
        let next = self
            .sink
            .block_height()
            .await
            .context("reading the height the query database has reached")?;
        let follower = Follower {
            light_client: self.light_client.clone(),
            sink: self.sink.clone(),
            persistence: self.persistence.clone(),
            coordinator: self.consensus.coordinator.clone(),
            state_catchup: self.node_state.state_catchup.clone(),
            decided: self.decided.clone(),
            chain_config: self.consensus.chain_config.clone(),
            upgrade_lock: self.consensus.upgrade_lock.clone(),
            epoch_height: self.consensus.epoch_height,
            options: self.options.clone(),
            bootstrap_epoch_catchup_timeout: self.bootstrap_epoch_catchup_timeout,
            next,
        };
        self.tasks.spawn("light client follower", follower.run());
        Ok(())
    }

    /// Waits for the first decided leaf.
    pub async fn decided_leaf(&self) -> Leaf2 {
        self.consensus.decided_leaf().await
    }

    pub fn node_state(&self) -> NodeState {
        self.node_state.clone()
    }

    pub fn light_client(&self) -> Arc<NodeLightClient> {
        self.light_client.clone()
    }

    pub async fn shut_down(&mut self) {
        tracing::info!("shutting down FollowerContext");
        self.tasks.shut_down();
        self.node_state.l1_client.shut_down_tasks().await;
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

impl<P: SequencerPersistence> ApiContext for FollowerContext<P> {
    type Persistence = P;

    fn consensus(&self) -> Arc<dyn ConsensusSource> {
        self.consensus.clone()
    }

    fn persistence(&self) -> Arc<P> {
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

    fn state_signer(&self) -> Option<Arc<RwLock<StateSigner<SequencerApiVersion>>>> {
        None
    }

    fn event_streamer(&self) -> Option<Arc<RwLock<EventsStreamer<SeqTypes>>>> {
        None
    }

    fn light_client(&self) -> Option<Arc<NodeLightClient>> {
        Some(self.light_client.clone())
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

    fn with_task_list(mut self, tasks: TaskList) -> Self {
        self.tasks.extend(tasks);
        self
    }
}

impl<P: SequencerPersistence> Drop for FollowerContext<P> {
    fn drop(&mut self) {
        if !self.detached {
            let tasks = self.tasks.clone();
            let l1_client = self.node_state.l1_client.clone();
            tokio::spawn(async move {
                tracing::info!("shutting down FollowerContext");
                tasks.shut_down();
                l1_client.shut_down_tasks().await;
            });
            self.detached = true;
        }
    }
}

/// Separate from [`FollowerContext`] so the API can hold it without owning the node's tasks.
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

    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()> {
        let mut last_err = anyhow!("no upstream query nodes to forward the transaction to");
        for upstream in &self.upstreams {
            let client = Client::<ClientErr, SequencerApiVersion>::new(upstream.clone());
            let result = client
                .post::<Commitment<Transaction>>("submit/submit")
                .body_binary(&tx)?
                .send()
                .await;
            match result {
                Ok(_) => return Ok(()),
                Err(err) => {
                    tracing::warn!(%upstream, "forwarding transaction failed: {err}");
                    last_err =
                        anyhow::Error::from(err).context(format!("forwarding to {upstream}"));
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
    light_client: Arc<NodeLightClient>,
    sink: Arc<dyn DecideSink>,
    persistence: Arc<P>,
    coordinator: EpochMembershipCoordinator<SeqTypes>,
    state_catchup: Arc<dyn StateCatchup>,
    decided: Arc<watch::Sender<Option<LeafQueryData<SeqTypes>>>>,
    chain_config: Arc<RwLock<ChainConfig>>,
    upgrade_lock: UpgradeLock<SeqTypes>,
    epoch_height: u64,
    options: FollowerOptions,
    bootstrap_epoch_catchup_timeout: Duration,
    next: u64,
}

struct VerifiedBlock {
    leaf: LeafQueryData<SeqTypes>,
    block: BlockQueryData<SeqTypes>,
    vid_common: VidCommonQueryData<SeqTypes>,
}

impl<P: SequencerPersistence> Follower<P> {
    async fn run(mut self) {
        tracing::info!(from = self.next, "following the chain");
        loop {
            if let Err(err) = self.poll().await {
                tracing::warn!(next = self.next, "following the chain failed: {err:#}");
            }
            sleep(self.options.poll_interval).await;
        }
    }

    async fn poll(&mut self) -> anyhow::Result<()> {
        let tip = self
            .light_client
            .block_height()
            .await
            .context("fetching the chain height")?;
        let from = max(
            self.next,
            tip.saturating_sub(self.options.max_blocks_per_poll),
        );
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
            self.catch_up_skipped_epochs(self.next..from).await?;
            self.next = from;
        }
        for block in self.fetch_range(from, tip).await? {
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

    /// One finality proof and one payload proof request cover the whole range.
    async fn fetch_range(&self, from: u64, to: u64) -> anyhow::Result<Vec<VerifiedBlock>> {
        let (from, to) = (from as usize, to as usize);
        let leaves = self
            .light_client
            .fetch_leaves_in_range(from, to)
            .await
            .context("fetching leaves")?;
        let blocks = self
            .light_client
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

    async fn follow(&self, block: VerifiedBlock) -> anyhow::Result<()> {
        let VerifiedBlock {
            leaf,
            block,
            vid_common,
        } = block;
        let height = leaf.height();
        let mut info = BlockInfo::new(leaf.clone(), Some(block), Some(vid_common), None);
        // Upstream stores a cert2 only at the heights it finalized directly, so most answers
        // are `None`; asking at every height is what keeps the follower serving the same cert2s.
        if leaf.header().version() >= NEW_PROTOCOL_VERSION
            && let Some(cert2) = self
                .light_client
                .fetch_certificate2(height)
                .await
                .context("fetching cert2")?
        {
            info = info.with_cert2(cert2);
        }
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
        if self.upgrade_lock.decided_upgrade_cert().as_ref() == Some(&cert) {
            return;
        }
        tracing::warn!(
            height = leaf.height(),
            version = ?cert.data.new_version,
            "the network decided an upgrade"
        );
        self.upgrade_lock.set_decided_upgrade_cert(cert.clone());
        if let Err(err) = self.persistence.store_upgrade_certificate(Some(cert)).await {
            tracing::warn!(
                height = leaf.height(),
                "cannot persist the upgrade certificate: {err:#}"
            );
        }
    }

    /// Awaited, so the transition block finds the stake table it supplies a DRB for, and a
    /// failed derivation fails the block and is retried with it. Nothing is redone when the
    /// membership already knows the result, as after a restart or a catchup walk.
    async fn track_epoch(&self, leaf: &Leaf2) -> anyhow::Result<()> {
        if self.epoch_height == 0 {
            return Ok(());
        }
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

    /// Skipping an epoch root skips the stake table it derives, which later roots build on. DRBs
    /// the skipped transition blocks carried are left to the coordinator, which fetches them on
    /// demand.
    async fn catch_up_skipped_epochs(&self, skipped: Range<u64>) -> anyhow::Result<()> {
        if self.epoch_height == 0 || skipped.is_empty() {
            return Ok(());
        }
        let Some(first_epoch) = self.coordinator.membership().first_epoch() else {
            return Ok(());
        };
        let first = max(
            epoch_from_block_number(skipped.start, self.epoch_height),
            first_epoch.u64(),
        );
        let last = epoch_from_block_number(skipped.end - 1, self.epoch_height);
        let missing = (first..=last).any(|epoch| {
            skipped.contains(&root_block_in_epoch(epoch, self.epoch_height))
                && !self.has_stake_table(EpochNumber::new(epoch + 2))
        });
        if !missing {
            return Ok(());
        }
        tracing::warn!(
            ?skipped,
            "skipped epoch roots whose stake tables are missing; catching up from peers"
        );
        let current_epoch = bootstrap_epoch_window(
            &self.coordinator,
            self.epoch_height,
            self.bootstrap_epoch_catchup_timeout,
        )
        .await
        .context("stake-table catchup after skipping ahead")?;
        tracing::info!(%current_epoch, "stake-table catchup complete");
        Ok(())
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

/// As `SystemContext::init` does for a validator.
fn seed_first_epoch(
    coordinator: &EpochMembershipCoordinator<SeqTypes>,
    network_config: &NetworkConfig<SeqTypes>,
) {
    let config = &network_config.config;
    if config.epoch_height == 0 {
        return;
    }
    let first_epoch = EpochNumber::new(epoch_from_block_number(
        config.epoch_start_block,
        config.epoch_height,
    ));
    coordinator
        .membership()
        .set_first_epoch(first_epoch, INITIAL_DRB_RESULT);
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
