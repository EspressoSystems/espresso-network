use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    future::Future,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, ensure};
use async_lock::RwLock;
use derivative::Derivative;
use espresso_types::{
    NodeState, Transaction, ValidatedState,
    v0::traits::{EventConsumer as PersistenceEventConsumer, SequencerPersistence},
};
use futures::{
    future::join_all,
    stream::{BoxStream, Stream, StreamExt},
};
use hotshot_new_protocol::{
    coordinator::Coordinator,
    leader_trace::{CsvLeaderTracer, LeaderTracerHandle},
    network::{Cliquenet, NetworkError},
    store::{ConsensusStore, Restored},
};
use hotshot_orchestrator::client::OrchestratorClient;
use hotshot_types::{
    HotShotConfig, PeerConfig, ValidatorConfig,
    consensus::ConsensusMetricsValue,
    constants::EXTERNAL_EVENT_CHANNEL_SIZE,
    data::{Leaf2, ViewNumber},
    drb::drb_difficulty_selector,
    epoch_membership::EpochMembershipCoordinator,
    initializer::{HotShotInitializer, load_start_epoch_info},
    message::UpgradeLock,
    network::NetworkConfig,
    new_protocol::CoordinatorEvent,
    traits::{election::Membership, metrics::Metrics},
    upgrade_config::UpgradeConfig,
};
use parking_lot::Mutex;
use request_response::RequestResponseConfig;
use tokio::{
    select, spawn,
    sync::{
        broadcast,
        mpsc::{channel, unbounded_channel},
    },
    task::JoinHandle,
};
use tracing::{Instrument, Level, info};
use url::Url;
use versions::{NEW_PROTOCOL_VERSION, Version};

use crate::{
    SeqTypes, SequencerApiVersion,
    catchup::ParallelStateCatchup,
    consensus_handle::ConsensusHandle,
    external_event_handler::ExternalEventHandler,
    request_response::{
        RequestResponseProtocol,
        data_source::{DataSource, Storage as RequestResponseStorage},
        network::Sender as RequestResponseSender,
        recipient_source::RecipientSource,
    },
    startup_catchup::bootstrap_epoch_window,
    state_signature::{self, DecidedLeaf, StateSignatureMemStorage, StateSigner},
};

/// Inbound request-response messages are dropped (not queued) when this channel is full. A node
/// catching up has many batched requests in flight, each answered by up to `request_batch_size`
/// peers per batch interval, so this must comfortably exceed that fan-in.
const REQUEST_RESPONSE_CHANNEL_CAPACITY: usize = 256;

/// Responders block on the outbound channel while holding request-response admission permits.
const OUTBOUND_MESSAGE_CHANNEL_CAPACITY: usize = 128;

/// The sequencer context contains a consensus handle and other sequencer specific information.
#[derive(Derivative, Clone)]
#[derivative(Debug(bound = ""))]
pub struct SequencerContext<P: SequencerPersistence> {
    #[derivative(Debug = "ignore")]
    consensus_handle: Arc<ConsensusHandle<SeqTypes>>,

    #[derivative(Debug = "ignore")]
    persistence: Arc<P>,

    /// The request-response protocol
    #[derivative(Debug = "ignore")]
    #[allow(dead_code)]
    pub request_response_protocol: RequestResponseProtocol<P>,

    /// Light client state signatures produced by the state signer task.
    state_signatures: Arc<RwLock<StateSignatureMemStorage>>,

    /// An orchestrator to wait for before starting consensus, with the peer config this node
    /// registered there.
    #[derivative(Debug = "ignore")]
    wait_for_orchestrator: Option<(Arc<OrchestratorClient>, PeerConfig<SeqTypes>)>,

    /// Background tasks to shut down when the node is dropped.
    tasks: TaskList,

    /// Where the ingester API is served.
    consensus_api_addr: SocketAddr,

    detached: bool,

    node_state: NodeState,

    network_config: NetworkConfig<SeqTypes>,

    #[derivative(Debug = "ignore")]
    validator_config: ValidatorConfig<SeqTypes>,
}

impl<P> SequencerContext<P>
where
    P: SequencerPersistence,
{
    #[tracing::instrument(skip_all, fields(node_id = initializer.instance_state().node_id))]
    #[allow(clippy::too_many_arguments)]
    pub async fn init<F>(
        network_config: NetworkConfig<SeqTypes>,
        upgrade: versions::Upgrade,
        validator_config: ValidatorConfig<SeqTypes>,
        membership_coordinator: EpochMembershipCoordinator<SeqTypes>,
        initializer: HotShotInitializer<SeqTypes>,
        storage: Option<RequestResponseStorage>,
        state_catchup: ParallelStateCatchup,
        persistence: Arc<P>,
        coordinator_network: F,
        state_relay_server: Option<Url>,
        metrics: &dyn Metrics,
        stake_table_capacity: usize,
        event_consumer: impl PersistenceEventConsumer + 'static,
        bootstrap_epoch_catchup_timeout: Duration,
        empty_block_delay: Duration,
        block_sizes: BTreeMap<Version, u64>,
        leader_trace_dir: Option<PathBuf>,
        consensus_store: ConsensusStore<SeqTypes>,
        restored: Restored<SeqTypes>,
        decide_log_limit: u64,
        consensus_api_addr: Option<SocketAddr>,
    ) -> anyhow::Result<Self>
    where
        F: AsyncFnOnce(UpgradeLock<SeqTypes>) -> Result<Cliquenet<SeqTypes>, NetworkError>,
    {
        let config = &network_config.config;
        let pub_key = validator_config.public_key;
        tracing::info!(%pub_key, "initializing consensus");
        let upgrade_lock = restored_upgrade_lock(upgrade, &initializer);
        let version = running_version(&upgrade_lock);
        ensure!(
            version >= NEW_PROTOCOL_VERSION,
            "the chain runs version {version}, which predates the new protocol \
             ({NEW_PROTOCOL_VERSION}), the only consensus this node runs"
        );

        let instance_state = initializer.instance_state().clone();

        // Stick our node ID in `metrics` so it is easily accessible via the status API.
        metrics
            .create_gauge("node_index".into(), None)
            .set(instance_state.node_id as usize);

        // Start L1 client if it isn't already.
        instance_state.l1_client.spawn_tasks().await;

        info!(target: "announce", ?initializer, "starting up sequencer context with initializer");

        let stake_table = config.hotshot_stake_table();
        let stake_table_commit = stake_table.commitment(stake_table_capacity)?;
        let stake_table_epoch = None;
        let should_vote =
            state_signature::should_vote(&stake_table, &validator_config.state_public_key);

        let epoch_height = initializer.epoch_height();

        seed_membership(&membership_coordinator, config, &upgrade_lock, &initializer).await;

        let mut coordinator_network = coordinator_network(upgrade_lock.clone()).await?;

        let current_epoch = bootstrap_epoch_window(
            &membership_coordinator,
            epoch_height,
            bootstrap_epoch_catchup_timeout,
        )
        .await
        .context("startup stake-table catchup failed")?;
        tracing::info!(%current_epoch, "Startup catchup complete");

        // For cliquenet this dials the N-1/N/N+1 epoch window before consensus starts.
        if let Err(err) = coordinator_network.apply_epoch(current_epoch, &membership_coordinator) {
            tracing::warn!(%current_epoch, %err, "coordinator network apply_epoch failed at startup");
        }

        // `seed_anchor_cert2` keeps it only if it decided the initializer's anchor.
        let anchor_cert2 = restored.anchor.as_ref().and_then(|a| a.cert2.clone());

        let leader_tracer = leader_trace_dir
            .map(|dir| {
                let node_id = instance_state.node_id;
                let path = dir.join(format!("leader_trace_node{node_id}.csv"));
                CsvLeaderTracer::new(node_id, &path)
                    .map(|tracer| Arc::new(tracer) as LeaderTracerHandle)
                    .with_context(|| format!("creating leader trace {}", path.display()))
            })
            .transpose()?;

        let (external_tx, external_rx) = unbounded_channel();
        let coordinator = Coordinator::maker()
            .membership_coordinator(membership_coordinator.clone())
            .network(coordinator_network)
            .initializer(&initializer)
            .upgrade_lock(upgrade_lock.clone())
            .public_key(validator_config.public_key)
            .private_key(validator_config.private_key.clone())
            .state_private_key(validator_config.state_private_key.clone())
            .stake_table_capacity(stake_table_capacity)
            .timeout_duration(Duration::from_secs(10))
            .empty_block_delay(empty_block_delay)
            .block_sizes(block_sizes)
            .store(consensus_store.clone())
            .restored(restored)
            .decide_log_limit(decide_log_limit)
            .metrics(metrics)
            .consensus_metrics(ConsensusMetricsValue::new(metrics))
            .maybe_anchor_cert2(anchor_cert2)
            .maybe_leader_tracer(leader_tracer)
            .upgrade_config(UpgradeConfig {
                start_proposing_view: config.start_proposing_view,
                stop_proposing_view: config.stop_proposing_view,
                start_voting_view: config.start_voting_view,
                stop_voting_view: config.stop_voting_view,
                start_proposing_time: config.start_proposing_time,
                stop_proposing_time: config.stop_proposing_time,
                start_voting_time: config.start_voting_time,
                stop_voting_time: config.stop_voting_time,
            })
            .external_tx(external_tx)
            .make();

        let consensus_handle = Arc::new(ConsensusHandle::new(
            coordinator,
            external_rx,
            upgrade_lock,
            membership_coordinator.clone(),
            &initializer,
            EXTERNAL_EVENT_CHANNEL_SIZE,
            metrics,
        ));

        let mut state_signer = StateSigner::new(
            validator_config.state_private_key.clone(),
            validator_config.state_public_key.clone(),
            stake_table_commit,
            stake_table_epoch,
            stake_table_capacity,
            should_vote,
        );
        if let Some(url) = state_relay_server {
            state_signer = state_signer.with_relay_server(url);
        }

        // Create the channel for sending outbound messages from the external event handler
        let (outbound_message_sender, outbound_message_receiver) =
            channel(OUTBOUND_MESSAGE_CHANNEL_CAPACITY);
        let (request_response_sender, request_response_receiver) =
            channel(REQUEST_RESPONSE_CHANNEL_CAPACITY);

        // Configure the request-response protocol
        let request_response_config = RequestResponseConfig {
            incoming_request_ttl: Duration::from_secs(40),
            incoming_request_timeout: Duration::from_secs(5),
            request_batch_size: 5,
            request_batch_interval: Duration::from_secs(2),
            // Permits are held while a response is derived (possibly from SQL), and a peer
            // catching up legitimately issues several distinct requests concurrently
            max_incoming_requests: 32,
            max_incoming_requests_per_key: 4,
        };

        // Create the request-response protocol
        let request_response_protocol = RequestResponseProtocol::new(
            request_response_config,
            RequestResponseSender::new(outbound_message_sender),
            request_response_receiver,
            RecipientSource {
                memberships: membership_coordinator,
                consensus_handle: consensus_handle.clone(),
                public_key: validator_config.public_key,
            },
            DataSource {
                node_state: instance_state.clone(),
                storage,
                persistence: persistence.clone(),
                consensus_handle: consensus_handle.clone(),
            },
            validator_config.public_key,
            validator_config.private_key.clone(),
        );

        // Add the request-response protocol to the list of providers for state catchup. Since the interior is mutable,
        // the request-response protocol will now retroactively be used anywhere we passed in the original struct (e.g. in consensus
        // itself)
        state_catchup.add_provider(Arc::new(request_response_protocol.clone()));

        let mut tasks = TaskList::default();
        // A free port on localhost by default, for the API service in this process.
        let addr = consensus_api_addr.unwrap_or((std::net::Ipv4Addr::LOCALHOST, 0).into());
        let (consensus_api_addr, server) =
            hotshot_new_protocol::ingest::bind(consensus_store, epoch_height, addr)
                .with_context(|| format!("binding the ingester API to {addr}"))?;
        info!(addr = %consensus_api_addr, "serving the ingester API");
        tasks.spawn("ingester API", server);

        // Create the external event handler
        let external_event_handler = ExternalEventHandler::new(
            &mut tasks,
            request_response_sender,
            outbound_message_receiver,
            consensus_handle.clone(),
            pub_key,
            metrics
                .subgroup("request_response".into())
                .create_counter("inbound_dropped".into(), None)
                .into(),
        )
        .await
        .with_context(|| "Failed to create external event handler")?;

        Ok(Self::new(
            consensus_handle,
            persistence,
            state_signer,
            external_event_handler,
            request_response_protocol,
            instance_state,
            network_config,
            validator_config,
            event_consumer,
            consensus_api_addr,
        )
        .with_task_list(tasks))
    }

    /// Constructor
    #[allow(clippy::too_many_arguments)]
    fn new(
        consensus_handle: Arc<ConsensusHandle<SeqTypes>>,
        persistence: Arc<P>,
        state_signer: StateSigner<SequencerApiVersion>,
        external_event_handler: ExternalEventHandler,
        request_response_protocol: RequestResponseProtocol<P>,
        node_state: NodeState,
        network_config: NetworkConfig<SeqTypes>,
        validator_config: ValidatorConfig<SeqTypes>,
        event_consumer: impl PersistenceEventConsumer + 'static,
        consensus_api_addr: SocketAddr,
    ) -> Self {
        let events = consensus_handle.event_stream();

        let node_id = node_state.node_id;
        let mut ctx = Self {
            consensus_handle,
            persistence: persistence.clone(),
            state_signatures: Arc::clone(&state_signer.signatures),
            request_response_protocol,
            tasks: Default::default(),
            consensus_api_addr,
            detached: false,
            wait_for_orchestrator: None,
            node_state,
            network_config,
            validator_config,
        };

        let (signer_tx, signer_rx) = broadcast::channel(SIGNER_QUEUE_CAPACITY);
        ctx.spawn(
            "state signer",
            sign_decided_leaves(ctx.consensus_handle.clone(), state_signer, signer_rx),
        );

        ctx.spawn(
            "event handler",
            handle_events(
                node_id,
                events,
                signer_tx,
                external_event_handler,
                event_consumer,
            ),
        );

        ctx
    }

    /// Wait for a signal from the orchestrator before starting consensus.
    ///
    /// `peer_config` is what this node registered with the orchestrator. `/ready` posts it
    /// again, and the orchestrator equality-checks it against `known_nodes_with_stake`.
    pub fn wait_for_orchestrator(
        mut self,
        client: OrchestratorClient,
        peer_config: PeerConfig<SeqTypes>,
    ) -> Self {
        self.wait_for_orchestrator = Some((Arc::new(client), peer_config));
        self
    }

    /// Add a list of tasks to the given context.
    pub(crate) fn with_task_list(mut self, tasks: TaskList) -> Self {
        self.tasks.extend(tasks);
        self
    }

    /// Return the light client state signatures this node has produced.
    pub fn state_signatures(&self) -> Arc<RwLock<StateSignatureMemStorage>> {
        self.state_signatures.clone()
    }

    /// Stream consensus events.
    pub fn event_stream(&self) -> BoxStream<'static, CoordinatorEvent<SeqTypes>> {
        self.consensus_handle.event_stream()
    }

    pub async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()> {
        self.consensus_handle.submit_transaction(tx).await
    }

    pub fn consensus_handle(&self) -> Arc<ConsensusHandle<SeqTypes>> {
        self.consensus_handle.clone()
    }

    pub fn validator_config(&self) -> &ValidatorConfig<SeqTypes> {
        &self.validator_config
    }

    pub fn upgrade_lock(&self) -> UpgradeLock<SeqTypes> {
        self.consensus_handle.upgrade_lock()
    }

    pub async fn shutdown_consensus(&self) {
        self.consensus_handle.shut_down().await
    }

    pub async fn decided_leaf(&self) -> Leaf2<SeqTypes> {
        self.consensus_handle.decided_leaf().await
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<ValidatedState>> {
        self.consensus_handle.state(view).await
    }

    pub async fn decided_state(&self) -> Option<Arc<ValidatedState>> {
        self.consensus_handle.decided_state().await
    }

    pub fn node_id(&self) -> u64 {
        self.node_state.node_id
    }

    pub fn node_state(&self) -> NodeState {
        self.node_state.clone()
    }

    pub fn persistence(&self) -> Arc<P> {
        self.persistence.clone()
    }

    /// Start participating in consensus.
    pub async fn start_consensus(&self) {
        if let Some((orchestrator_client, peer_config)) = &self.wait_for_orchestrator {
            tracing::warn!("waiting for orchestrated start");
            orchestrator_client
                .wait_for_all_nodes_ready(PeerConfig::to_bytes(peer_config))
                .await;
        } else {
            // the network config was loaded from storage or fetched from
            // peers, so there is no need of orchestrator
            // This is the normal path for a node rejoining an existing network.
            tracing::info!("no orchestrator configured");
        }
        tracing::warn!("starting consensus");
        self.consensus_handle.start_consensus().await;
    }

    /// Spawn a background task attached to this context.
    ///
    /// When this context is dropped or [`shut_down`](Self::shut_down), background tasks will be
    /// cancelled in the reverse order that they were spawned.
    pub fn spawn(&mut self, name: impl Display, task: impl Future<Output: Debug> + Send + 'static) {
        self.tasks.spawn(name, task);
    }

    /// Where the ingester API is served, for an API service in this process.
    pub fn consensus_api_addr(&self) -> SocketAddr {
        self.consensus_api_addr
    }

    /// Spawn a short-lived background task attached to this context.
    ///
    /// When this context is dropped or [`shut_down`](Self::shut_down), background tasks will be
    /// cancelled in the reverse order that they were spawned.
    ///
    /// The only difference between a short-lived background task and a [long-lived](Self::spawn)
    /// one is how urgently logging related to the task is treated.
    pub fn spawn_short_lived(
        &mut self,
        name: impl Display,
        task: impl Future<Output: Debug> + Send + 'static,
    ) {
        self.tasks.spawn_short_lived(name, task);
    }

    /// Stop participating in consensus.
    pub async fn shut_down(&mut self) {
        tracing::info!("shutting down SequencerContext");
        self.consensus_handle.shut_down().await;
        self.tasks.shut_down();
        self.node_state.l1_client.shut_down_tasks().await;

        // Since we've already shut down, we can set `detached` so the drop
        // handler doesn't call `shut_down` again.
        self.detached = true;
    }

    /// Wait for consensus to complete.
    ///
    /// Under normal conditions, this function will block forever, which is a convenient way of
    /// keeping the main thread from exiting as long as there are still active background tasks.
    /// It returns once the coordinator has stopped, so a node whose coordinator died exits
    /// instead of serving its last state.
    pub async fn join(&mut self) {
        select! {
            () = self.tasks.join() => {},
            () = self.consensus_handle.stopped() => tracing::error!("coordinator stopped"),
        }
    }

    /// Allow this node to continue participating in consensus even after it is dropped.
    pub fn detach(&mut self) {
        // Set `detached` so the drop handler doesn't call `shut_down`.
        self.detached = true;
    }

    /// Get the network config
    pub fn network_config(&self) -> NetworkConfig<SeqTypes> {
        self.network_config.clone()
    }
}

/// A decided certificate at or below the configured base is from a completed upgrade; kept, it
/// would make `UpgradeLock::version` fail once the target moves on.
fn restored_upgrade_lock(
    upgrade: versions::Upgrade,
    initializer: &HotShotInitializer<SeqTypes>,
) -> UpgradeLock<SeqTypes> {
    let decided_upgrade_certificate = initializer
        .decided_upgrade_certificate()
        .filter(|cert| cert.data.new_version > upgrade.base)
        .cloned();
    tracing::info!(?upgrade, ?decided_upgrade_certificate, "resuming consensus");
    UpgradeLock::from_certificate(upgrade, &decided_upgrade_certificate)
}

/// The version the chain runs now: the decided upgrade's, or the base when none was decided.
fn running_version(upgrade_lock: &UpgradeLock<SeqTypes>) -> Version {
    upgrade_lock
        .decided_upgrade_cert()
        .map(|cert| cert.data.new_version)
        .unwrap_or(upgrade_lock.upgrade().base)
}

async fn seed_membership(
    membership_coordinator: &EpochMembershipCoordinator<SeqTypes>,
    config: &HotShotConfig<SeqTypes>,
    upgrade_lock: &UpgradeLock<SeqTypes>,
    initializer: &HotShotInitializer<SeqTypes>,
) {
    membership_coordinator.set_drb_difficulty_selector(drb_difficulty_selector(config));

    let running_version = running_version(upgrade_lock);
    let membership = membership_coordinator.membership();
    for da_committee in &config.da_committees {
        if running_version >= da_committee.start_version {
            membership.add_da_committee(
                da_committee.start_epoch.into(),
                da_committee.committee.clone(),
            );
        }
    }

    load_start_epoch_info(membership_coordinator, initializer.start_epoch_info()).await;
}

impl<P: SequencerPersistence> Drop for SequencerContext<P> {
    fn drop(&mut self) {
        if !self.detached {
            // Spawn a task to shut down the context
            let consensus_handle = self.consensus_handle.clone();
            let tasks_clone = self.tasks.clone();
            let node_state_clone = self.node_state.clone();

            spawn(async move {
                tracing::info!("shutting down SequencerContext");
                consensus_handle.shut_down().await;
                tasks_clone.shut_down();
                node_state_clone.l1_client.shut_down_tasks().await;
            });

            // Set `detached` so the drop handler doesn't call `shut_down` again.
            self.detached = true;
        }
    }
}

#[tracing::instrument(skip_all, fields(node_id))]
async fn handle_events<C>(
    node_id: u64,
    mut events: impl Stream<Item = CoordinatorEvent<SeqTypes>> + Unpin,
    signer_tx: broadcast::Sender<DecidedLeaf>,
    external_event_handler: ExternalEventHandler,
    event_consumer: C,
) where
    C: PersistenceEventConsumer + 'static,
{
    while let Some(event) = events.next().await {
        tracing::debug!(node_id, ?event, "consensus event");

        match &event {
            CoordinatorEvent::ExternalMessageReceived { data, .. } => {
                if let Err(err) = external_event_handler.handle_event(data).await {
                    tracing::warn!("Failed to handle external message: {:?}", err);
                }
            },
            CoordinatorEvent::BlockPayloadReconstructed { .. } => {
                // Forward straight to the consumer: reconstructed payloads might not yet
                // have been stored by consensus storage,
                // Query service verifies the block against a decided leaf before storing it.
                if let Err(err) = event_consumer.handle_event(&event).await {
                    tracing::warn!("failed to handle reconstructed payload: {err:#}");
                }
            },
            _ => {},
        }

        if let Some(leaf) = DecidedLeaf::from_event(&event) {
            let _ = signer_tx.send(leaf);
        }
    }
}

/// Decides the signer may fall behind by before it drops the oldest ones.
const SIGNER_QUEUE_CAPACITY: usize = 128;

/// Signs and relays the newest leaf of every decide, in order, off the event loop, because a
/// relay post can take as long as its HTTP timeout. The relay needs a threshold of signatures on
/// one height, so every node must sign the same heights. Signing every decide keeps that while
/// posts fall behind, and dropping only the oldest once the queue is full keeps it as well.
async fn sign_decided_leaves(
    consensus_handle: Arc<ConsensusHandle<SeqTypes>>,
    mut state_signer: StateSigner<SequencerApiVersion>,
    mut leaves: broadcast::Receiver<DecidedLeaf>,
) {
    loop {
        match leaves.recv().await {
            Ok(leaf) => {
                state_signer
                    .handle_decide(&leaf, consensus_handle.as_ref())
                    .await;
            },
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                tracing::warn!(skipped, "state signer fell behind, dropped oldest decides");
            },
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
}

#[derive(Debug, Default, Clone)]
#[allow(clippy::type_complexity)]
pub struct TaskList(Arc<Mutex<Vec<(String, JoinHandle<()>)>>>);

macro_rules! spawn_with_log_level {
    ($this:expr, $lvl:expr, $name:expr, $task: expr) => {
        let name = $name.to_string();
        let task = {
            let name = name.clone();
            let span = tracing::span!($lvl, "background task", name);
            spawn(
                async move {
                    tracing::event!($lvl, "spawning background task");
                    let res = $task.await;
                    tracing::event!($lvl, ?res, "background task exited");
                }
                .instrument(span),
            )
        };
        $this.0.lock().push((name, task));
    };
}

impl TaskList {
    /// Spawn a background task attached to this [`TaskList`].
    ///
    /// When this [`TaskList`] is dropped or [`shut_down`](Self::shut_down), background tasks will
    /// be cancelled in the reverse order that they were spawned.
    pub fn spawn(&mut self, name: impl Display, task: impl Future<Output: Debug> + Send + 'static) {
        spawn_with_log_level!(self, Level::INFO, name, task);
    }

    /// Spawn a short-lived background task attached to this [`TaskList`].
    ///
    /// When this [`TaskList`] is dropped or [`shut_down`](Self::shut_down), background tasks will
    /// be cancelled in the reverse order that they were spawned.
    ///
    /// The only difference between a short-lived background task and a [long-lived](Self::spawn)
    /// one is how urgently logging related to the task is treated.
    pub fn spawn_short_lived(
        &mut self,
        name: impl Display,
        task: impl Future<Output: Debug> + Send + 'static,
    ) {
        spawn_with_log_level!(self, Level::DEBUG, name, task);
    }

    /// Stop all background tasks.
    pub fn shut_down(&self) {
        let tasks: Vec<(String, JoinHandle<()>)> = self.0.lock().drain(..).collect();
        for (name, task) in tasks.into_iter().rev() {
            tracing::info!(name, "cancelling background task");
            task.abort();
        }
    }

    /// Wait for all background tasks to complete.
    pub async fn join(&mut self) {
        let tasks: Vec<(String, JoinHandle<()>)> = self.0.lock().drain(..).collect();
        join_all(tasks.into_iter().map(|(_, task)| task)).await;
    }

    pub fn extend(&mut self, tasks: TaskList) {
        self.0.lock().extend(
            tasks
                .0
                .lock()
                .drain(..)
                .collect::<Vec<(String, JoinHandle<()>)>>(),
        );
    }
}

impl Drop for TaskList {
    fn drop(&mut self) {
        self.shut_down()
    }
}
