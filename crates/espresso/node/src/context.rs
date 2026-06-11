use std::{
    fmt::{Debug, Display},
    future::Future,
    marker::PhantomData,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use async_lock::RwLock;
use derivative::Derivative;
use espresso_types::{
    NodeState, Payload, PubKey, Transaction, ValidatedState,
    v0::traits::{
        DecidePayloadRecovery, EventConsumer as PersistenceEventConsumer, PendingDecide,
        RecoveredPayload, SequencerPersistence,
    },
};
use futures::{
    future::join_all,
    stream::{BoxStream, Stream, StreamExt},
};
use hotshot::SystemContext;
use hotshot_events_service::events_source::{EventConsumer, EventsStreamer};
use hotshot_new_protocol::{coordinator::Coordinator, network::Network};
use hotshot_orchestrator::client::OrchestratorClient;
use hotshot_types::{
    PeerConfig, ValidatorConfig,
    consensus::ConsensusMetricsValue,
    constants::EXTERNAL_EVENT_CHANNEL_SIZE,
    data::{Leaf2, VidCommitment, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
    new_protocol::CoordinatorEvent,
    storage_metrics::StorageMetricsValue,
    traits::{
        block_contents::BlockPayload,
        metrics::{Counter, Gauge, Histogram, Metrics},
        network::ConnectedNetwork,
    },
};
use parking_lot::Mutex;
use request_response::RequestResponseConfig;
use tokio::{
    spawn,
    sync::{mpsc::channel, watch},
    task::JoinHandle,
    time::sleep,
};
use tracing::{Instrument, Level};
use url::Url;

use crate::{
    Node, SeqTypes, SequencerApiVersion,
    catchup::ParallelStateCatchup,
    consensus_handle::ConsensusHandle,
    external_event_handler::ExternalEventHandler,
    persistence::PAYLOAD_RETENTION_VIEWS,
    proposal_fetcher::ProposalFetcherConfig,
    request_response::{
        RequestResponseProtocol,
        data_source::{DataSource, Storage as RequestResponseStorage},
        network::Sender as RequestResponseSender,
        payload_recovery::PayloadRecovery,
        recipient_source::RecipientSource,
    },
    startup_catchup::bootstrap_epoch_window,
    state_signature::{self, StateSigner},
};
pub(crate) type ConsensusNode<N, P> = Node<N, P>;
pub type Consensus<N, P> = hotshot::types::SystemContextHandle<SeqTypes, ConsensusNode<N, P>>;

/// The sequencer context contains a consensus handle and other sequencer specific information.
#[derive(Derivative, Clone)]
#[derivative(Debug(bound = ""))]
pub struct SequencerContext<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> {
    /// The consensus adapter that dispatches between old HotShot and new coordinator.
    #[derivative(Debug = "ignore")]
    consensus_handle: Arc<ConsensusHandle<SeqTypes, ConsensusNode<N, P>>>,

    /// The request-response protocol
    #[derivative(Debug = "ignore")]
    #[allow(dead_code)]
    pub request_response_protocol: RequestResponseProtocol<ConsensusNode<N, P>, N, P>,

    /// Context for generating state signatures.
    state_signer: Arc<RwLock<StateSigner<SequencerApiVersion>>>,

    /// An orchestrator to wait for before starting consensus.
    #[derivative(Debug = "ignore")]
    wait_for_orchestrator: Option<Arc<OrchestratorClient>>,

    /// Background tasks to shut down when the node is dropped.
    tasks: TaskList,

    /// events streamer to stream hotshot events to external clients
    events_streamer: Arc<RwLock<EventsStreamer<SeqTypes>>>,

    detached: bool,

    node_state: NodeState,

    network_config: NetworkConfig<SeqTypes>,

    #[derivative(Debug = "ignore")]
    validator_config: ValidatorConfig<SeqTypes>,
}

impl<N, P> SequencerContext<N, P>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    #[tracing::instrument(skip_all, fields(node_id = instance_state.node_id))]
    #[allow(clippy::too_many_arguments)]
    pub async fn init<T: Network<SeqTypes> + Send + 'static>(
        network_config: NetworkConfig<SeqTypes>,
        upgrade: versions::Upgrade,
        validator_config: ValidatorConfig<SeqTypes>,
        membership_coordinator: EpochMembershipCoordinator<SeqTypes>,
        instance_state: NodeState,
        storage: Option<RequestResponseStorage>,
        state_catchup: ParallelStateCatchup,
        persistence: Arc<P>,
        network: Arc<N>,
        coordinator_network: T,
        state_relay_server: Option<Url>,
        metrics: &dyn Metrics,
        stake_table_capacity: usize,
        event_consumer: impl PersistenceEventConsumer + 'static,
        proposal_fetcher_cfg: ProposalFetcherConfig,
        bootstrap_epoch_catchup_timeout: Duration,
    ) -> anyhow::Result<Self> {
        let config = &network_config.config;
        let pub_key = validator_config.public_key;
        tracing::info!(%pub_key, "initializing consensus");

        // Stick our node ID in `metrics` so it is easily accessible via the status API.
        metrics
            .create_gauge("node_index".into(), None)
            .set(instance_state.node_id as usize);

        // Start L1 client if it isn't already.
        instance_state.l1_client.spawn_tasks().await;

        // Load saved consensus state from storage.
        let (initializer, anchor_view) = persistence
            .load_consensus_state(instance_state.clone(), upgrade)
            .await?;

        tracing::warn!(
            "Starting up sequencer context with initializer:\n\n{:?}",
            initializer
        );

        let stake_table = config.hotshot_stake_table();
        let stake_table_commit = stake_table.commitment(stake_table_capacity)?;
        let stake_table_epoch = None;
        let should_vote =
            state_signature::should_vote(&stake_table, &validator_config.state_public_key);

        let epoch_height = initializer.epoch_height;

        let initializer_for_coordinator = initializer.clone();

        let event_streamer = Arc::new(RwLock::new(EventsStreamer::<SeqTypes>::new(
            stake_table.0,
            0,
        )));
        let handle = SystemContext::init(
            validator_config.public_key,
            validator_config.private_key.clone(),
            validator_config.state_private_key.clone(),
            instance_state.node_id,
            config.clone(),
            upgrade,
            membership_coordinator.clone(),
            network.clone(),
            initializer,
            ConsensusMetricsValue::new(metrics),
            Arc::clone(&persistence),
            StorageMetricsValue::new(metrics),
        )
        .await?
        .0;

        // `load_start_epoch_info` ran inside `SystemContext::init`, so
        // `first_epoch` is now seeded on the shared membership. Walk the
        // catchup chain forward to populate the stake-table window for the
        // current epoch.
        let current_epoch = bootstrap_epoch_window(
            &membership_coordinator,
            epoch_height,
            bootstrap_epoch_catchup_timeout,
        )
        .await
        .context("startup stake-table catchup failed")?;
        tracing::info!(%current_epoch, "Startup catchup complete");

        // Push the resolved peer window into the coordinator network. For
        // cliquenet this dials the N-1/N/N+1 sliding window for the current
        // epoch before consensus starts.
        let mut coordinator_network = coordinator_network;
        if let Err(err) = coordinator_network.apply_epoch(current_epoch, &membership_coordinator) {
            tracing::warn!(%current_epoch, %err, "coordinator network apply_epoch failed at startup");
        }

        let coordinator = Coordinator::maker()
            .membership_coordinator(membership_coordinator.clone())
            .network(coordinator_network)
            .initializer(&initializer_for_coordinator)
            .upgrade_lock(handle.hotshot.upgrade_lock.clone())
            .public_key(validator_config.public_key)
            .private_key(validator_config.private_key.clone())
            .state_private_key(validator_config.state_private_key.clone())
            .stake_table_capacity(stake_table_capacity)
            .timeout_duration(Duration::from_secs(10))
            .storage(Arc::clone(&persistence))
            .metrics(metrics)
            .make();

        let legacy_event_rx = handle.event_stream_known_impl().deactivate();
        let hotshot_handle = Arc::new(RwLock::new(handle));
        let consensus_handle = Arc::new(ConsensusHandle::new(
            hotshot_handle.clone(),
            coordinator,
            epoch_height,
            legacy_event_rx,
            EXTERNAL_EVENT_CHANNEL_SIZE,
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
        let (outbound_message_sender, outbound_message_receiver) = channel(20);
        let (request_response_sender, request_response_receiver) = channel(20);

        // Configure the request-response protocol
        let request_response_config = RequestResponseConfig {
            incoming_request_ttl: Duration::from_secs(40),
            incoming_request_timeout: Duration::from_secs(5),
            incoming_response_timeout: Duration::from_secs(5),
            request_batch_size: 5,
            request_batch_interval: Duration::from_secs(2),
            max_incoming_requests: 10,
            max_incoming_requests_per_key: 1,
            max_incoming_responses: 200,
        };

        // Create the request-response protocol
        let request_response_protocol = RequestResponseProtocol::new(
            request_response_config,
            RequestResponseSender::new(outbound_message_sender),
            request_response_receiver,
            RecipientSource {
                memberships: membership_coordinator.clone(),
                consensus_handle: consensus_handle.clone(),
                public_key: validator_config.public_key,
            },
            DataSource {
                node_state: instance_state.clone(),
                storage,
                persistence: persistence.clone(),
                consensus_handle: consensus_handle.clone(),
                phantom: PhantomData,
            },
            validator_config.public_key,
            validator_config.private_key.clone(),
        );

        // Add the request-response protocol to the list of providers for state catchup. Since the interior is mutable,
        // the request-response protocol will now retroactively be used anywhere we passed in the original struct (e.g. in consensus
        // itself)
        state_catchup.add_provider(Arc::new(request_response_protocol.clone()));

        // Recovers DA proposals from peers for views decided before their payload landed.
        let payload_recovery: Arc<dyn DecidePayloadRecovery> = Arc::new(PayloadRecovery::new(
            request_response_protocol.clone(),
            membership_coordinator.clone(),
            epoch_height,
        ));

        // Create the external event handler
        let mut tasks = TaskList::default();
        let external_event_handler = ExternalEventHandler::new(
            &mut tasks,
            request_response_sender,
            outbound_message_receiver,
            network,
            pub_key,
        )
        .await
        .with_context(|| "Failed to create external event handler")?;

        Ok(Self::new(
            consensus_handle,
            persistence,
            state_signer,
            external_event_handler,
            request_response_protocol,
            event_streamer,
            instance_state,
            network_config,
            validator_config,
            event_consumer,
            anchor_view,
            proposal_fetcher_cfg,
            payload_recovery,
            metrics,
        )
        .with_task_list(tasks))
    }

    /// Constructor
    #[allow(clippy::too_many_arguments)]
    fn new(
        consensus_handle: Arc<ConsensusHandle<SeqTypes, ConsensusNode<N, P>>>,
        persistence: Arc<P>,
        state_signer: StateSigner<SequencerApiVersion>,
        external_event_handler: ExternalEventHandler,
        request_response_protocol: RequestResponseProtocol<ConsensusNode<N, P>, N, P>,
        event_streamer: Arc<RwLock<EventsStreamer<SeqTypes>>>,
        node_state: NodeState,
        network_config: NetworkConfig<SeqTypes>,
        validator_config: ValidatorConfig<SeqTypes>,
        event_consumer: impl PersistenceEventConsumer + 'static,
        anchor_view: Option<ViewNumber>,
        proposal_fetcher_cfg: ProposalFetcherConfig,
        payload_recovery: Arc<dyn DecidePayloadRecovery>,
        metrics: &dyn Metrics,
    ) -> Self {
        let events = consensus_handle.event_stream();

        let node_id = node_state.node_id;
        let mut ctx = Self {
            consensus_handle,
            state_signer: Arc::new(RwLock::new(state_signer)),
            request_response_protocol,
            tasks: Default::default(),
            detached: false,
            wait_for_orchestrator: None,
            events_streamer: event_streamer.clone(),
            node_state,
            network_config,
            validator_config,
        };

        // Spawn proposal fetching tasks.
        proposal_fetcher_cfg.spawn(
            &mut ctx.tasks,
            ctx.consensus_handle.clone(),
            persistence.clone(),
            metrics,
        );

        // Shared between the event loop and the background decide processor.
        let event_consumer = Arc::new(event_consumer);

        // Wakes the background decide processor. `watch` coalesces: the processor is cursor-driven,
        // so it only needs the latest decided view.
        let (decide_tx, decide_rx) = watch::channel::<DecideSignal>(None);

        // Background decide processor: query-service ingestion + GC, decoupled from the event loop.
        ctx.spawn(
            "decide processor",
            process_decided_events_task(
                persistence.clone(),
                event_consumer.clone(),
                decide_rx,
                anchor_view,
                payload_recovery,
                DecideProcessorMetrics::new(metrics),
            ),
        );

        // Event loop. On a decide this only does the leaf write, then signals `decide_tx`.
        ctx.spawn(
            "event handler",
            handle_events(
                ctx.consensus_handle.clone(),
                node_id,
                events,
                persistence,
                ctx.state_signer.clone(),
                external_event_handler,
                Some(event_streamer.clone()),
                event_consumer,
                decide_tx,
            ),
        );

        ctx
    }

    /// Wait for a signal from the orchestrator before starting consensus.
    pub fn wait_for_orchestrator(mut self, client: OrchestratorClient) -> Self {
        self.wait_for_orchestrator = Some(Arc::new(client));
        self
    }

    /// Add a list of tasks to the given context.
    pub(crate) fn with_task_list(mut self, tasks: TaskList) -> Self {
        self.tasks.extend(tasks);
        self
    }

    /// Return a reference to the consensus state signer.
    pub fn state_signer(&self) -> Arc<RwLock<StateSigner<SequencerApiVersion>>> {
        self.state_signer.clone()
    }

    /// Stream consensus events.
    pub fn event_stream(&self) -> BoxStream<'static, CoordinatorEvent<SeqTypes>> {
        self.consensus_handle.event_stream()
    }

    pub async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()> {
        self.consensus_handle.submit_transaction(tx).await
    }

    /// get event streamer
    pub fn event_streamer(&self) -> Arc<RwLock<EventsStreamer<SeqTypes>>> {
        self.events_streamer.clone()
    }

    /// Return a reference to the consensus adapter.
    pub fn consensus_handle(&self) -> Arc<ConsensusHandle<SeqTypes, ConsensusNode<N, P>>> {
        self.consensus_handle.clone()
    }

    pub async fn upgrade_lock(&self) -> UpgradeLock<SeqTypes> {
        self.consensus_handle.upgrade_lock().await
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

    /// Start participating in consensus.
    pub async fn start_consensus(&self) {
        if let Some(orchestrator_client) = &self.wait_for_orchestrator {
            tracing::warn!("waiting for orchestrated start");
            let peer_config = PeerConfig::to_bytes(&self.validator_config.public_config()).clone();
            orchestrator_client
                .wait_for_all_nodes_ready(peer_config)
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
    pub async fn join(mut self) {
        self.tasks.join().await;
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

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> Drop for SequencerContext<N, P> {
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

/// Latest decide, sent from the event loop to the background processor with the in-memory event
/// data for live ingestion. `None` is the `watch` channel's initial value. Under lag the channel
/// coalesces; dropped views are regenerated from storage.
type DecideSignal = Option<PendingDecide>;

/// Metrics for the background decide processor. `backlog` (decided - processed) is the key signal:
/// sustained growth means staging tables accumulate (no data lost, but disk grows).
struct DecideProcessorMetrics {
    last_decided: Arc<dyn Gauge>,
    last_processed: Arc<dyn Gauge>,
    backlog: Arc<dyn Gauge>,
    duration: Arc<dyn Histogram>,
    failures: Arc<dyn Counter>,
    /// Block payloads recovered from peers for views decided without one.
    payloads_recovered: Arc<dyn Counter>,
    /// Failed attempts to recover a block payload from peers.
    payload_recovery_failures: Arc<dyn Counter>,
}

impl DecideProcessorMetrics {
    fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        let metrics = metrics.subgroup("decide_processor".into());
        Self {
            last_decided: metrics
                .create_gauge("last_decided".into(), Some("view".into()))
                .into(),
            last_processed: metrics
                .create_gauge("last_processed".into(), Some("view".into()))
                .into(),
            backlog: metrics
                .create_gauge("backlog".into(), Some("view".into()))
                .into(),
            duration: metrics
                .create_histogram("process_duration".into(), Some("seconds".into()))
                .into(),
            failures: metrics.create_counter("failures".into(), None).into(),
            payloads_recovered: metrics
                .create_counter("payloads_recovered".into(), None)
                .into(),
            payload_recovery_failures: metrics
                .create_counter("payload_recovery_failures".into(), None)
                .into(),
        }
    }
}

#[tracing::instrument(skip_all, fields(node_id))]
#[allow(clippy::too_many_arguments)]
async fn handle_events<N, P, C>(
    consensus_handle: Arc<ConsensusHandle<SeqTypes, ConsensusNode<N, P>>>,
    node_id: u64,
    mut events: impl Stream<Item = CoordinatorEvent<SeqTypes>> + Unpin,
    persistence: Arc<P>,
    state_signer: Arc<RwLock<StateSigner<SequencerApiVersion>>>,
    external_event_handler: ExternalEventHandler,
    events_streamer: Option<Arc<RwLock<EventsStreamer<SeqTypes>>>>,
    event_consumer: Arc<C>,
    decide_tx: watch::Sender<DecideSignal>,
) where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
    C: PersistenceEventConsumer + 'static,
{
    while let Some(event) = events.next().await {
        tracing::debug!(node_id, ?event, "consensus event");

        match &event {
            CoordinatorEvent::LegacyEvent(hotshot_event) => {
                if let hotshot_types::event::EventType::ExternalMessageReceived { ref data, .. } =
                    hotshot_event.event
                    && let Err(err) = external_event_handler.handle_event(data).await
                {
                    tracing::warn!("Failed to handle legacy external message: {:?}", err);
                }
                // Check if we're ready to start the new protocol
                consensus_handle.cutover_active().await;
            },
            CoordinatorEvent::ExternalMessageReceived { data, .. } => {
                if let Err(err) = external_event_handler.handle_event(data).await {
                    tracing::warn!("Failed to handle external message: {:?}", err);
                }
            },
            CoordinatorEvent::BlockPayloadReconstructed { view, .. }
            | CoordinatorEvent::VidShareValidated { view, .. } => {
                // The coordinator already persisted this to consensus storage (with retries);
                // forward it to the query service to back-fill the missing block payload or VID
                // data. Spawned so a slow write can't stall the event loop; idempotent.
                let consumer = event_consumer.clone();
                let event = event.clone();
                let view = *view;
                spawn(async move {
                    if let Err(err) = consumer.handle_event(&event).await {
                        tracing::warn!(
                            ?view,
                            "failed to store coordinator back-fill data in query service: {err:#}"
                        );
                    }
                });
            },
            _ => {},
        }

        // Critical path: only persist the decided leaves, then signal the background processor.
        // Signalling after the persist future means it never reads ahead of committed state.
        let persistence_fut = async {
            if let Some(signal) = persistence
                .persist_event(&event, event_consumer.as_ref())
                .await
            {
                // A closed receiver only happens during shutdown.
                let _ = decide_tx.send(Some(signal));
            }
        };

        let state_signer_fut = async {
            state_signer
                .write()
                .await
                .handle_event(&event, consensus_handle.as_ref())
                .await;
        };

        let events_streamer_fut = async {
            if let CoordinatorEvent::LegacyEvent(ref hotshot_event) = event
                && let Some(events_streamer) = events_streamer.as_ref()
            {
                events_streamer
                    .write()
                    .await
                    .handle_event(hotshot_event.clone())
                    .await;
            }
        };

        tokio::join!(persistence_fut, state_signer_fut, events_streamer_fut);
    }
}

const PROCESS_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// Turns persisted decided leaves into query-service decide events and GCs processed data.
/// Decoupled from [`handle_events`] so slow ingestion/GC can't stall (or drop) consensus events;
/// cursor-driven, so it can lag without losing data.
#[tracing::instrument(skip_all)]
async fn process_decided_events_task<P, C>(
    persistence: Arc<P>,
    consumer: Arc<C>,
    mut decide_rx: watch::Receiver<DecideSignal>,
    anchor_view: Option<ViewNumber>,
    payload_recovery: Arc<dyn DecidePayloadRecovery>,
    metrics: DecideProcessorMetrics,
) where
    P: SequencerPersistence,
    C: PersistenceEventConsumer + 'static,
{
    // Highest view confirmed processed, for the backlog gauge. Floored at the anchor view; the
    // cursor reported below raises it.
    let mut last_processed = anchor_view.map(|v| v.u64()).unwrap_or(0);

    // Process leaves persisted before a previous shutdown but not yet handled. Decide data
    // staged before the shutdown is read back from storage like everything else.
    if let Some(view) = anchor_view {
        match persistence
            .process_decided_events(view, None, consumer.as_ref())
            .await
        {
            Ok(outcome) => {
                if let Some(v) = outcome.processed {
                    last_processed = last_processed.max(v.u64());
                }
                spawn_payload_recovery(
                    &payload_recovery,
                    &persistence,
                    &consumer,
                    view.u64(),
                    outcome.missing_payload,
                    &metrics,
                );
            },
            Err(err) => tracing::warn!(
                "failed to process decided leaves on startup, chain may not be up to date: {err:#}"
            ),
        }
    }

    // Reused on a timeout to re-attempt the most recent decide when no new one has arrived.
    let mut latest: DecideSignal = None;

    loop {
        // Wait for the next decide, retrying the most recent one if none arrives within the timeout.
        match tokio::time::timeout(PROCESS_RETRY_INTERVAL, decide_rx.changed()).await {
            Ok(Ok(())) => latest = decide_rx.borrow_and_update().clone(),
            Ok(Err(_)) => {
                tracing::info!("decide signal channel closed, stopping decide processor");
                return;
            },
            Err(_) => {}, // Timed out; fall through to retry `latest`.
        }

        let Some(pending) = latest.clone() else {
            continue;
        };
        let decided = pending.view.u64();
        metrics.last_decided.set(decided as usize);
        metrics
            .backlog
            .set(decided.saturating_sub(last_processed) as usize);

        // Stage the decide event's in-memory data first, so just-decided views don't wait on
        // the coordinator's async storage writes and event generation reads storage only. On
        // failure, retry the whole signal rather than emitting events the staged data should
        // have filled.
        if !pending.data.is_empty()
            && let Err(err) = persistence.stage_decide_data(&pending.data).await
        {
            metrics.failures.add(1);
            tracing::warn!(
                view = ?pending.view,
                "failed to stage decide data, will retry: {err:#}"
            );
            continue;
        }

        let start = Instant::now();
        let result = persistence
            .process_decided_events(pending.view, pending.deciding_qc.clone(), consumer.as_ref())
            .await;
        metrics.duration.add_point(start.elapsed().as_secs_f64());

        match result {
            Ok(outcome) => {
                // Advance from the real cursor, not `decided`: if ingestion/GC lagged, `processed`
                // stays behind and the backlog gauge reflects it.
                if let Some(v) = outcome.processed {
                    last_processed = last_processed.max(v.u64());
                }
                // Recover payloads for leaves emitted without one, in the background, so the
                // cursor never waits on the network.
                spawn_payload_recovery(
                    &payload_recovery,
                    &persistence,
                    &consumer,
                    decided,
                    outcome.missing_payload,
                    &metrics,
                );
                // reset latest if we have processed all the decided leaves
                if let Some(pending) = &latest
                    && last_processed >= pending.view.u64()
                {
                    latest = None;
                }
                metrics.last_processed.set(last_processed as usize);
                metrics
                    .backlog
                    .set(decided.saturating_sub(last_processed) as usize);
            },
            Err(err) => {
                // Cursor not advanced, so this range is retried next iteration; no data is lost.
                metrics.failures.add(1);
                tracing::warn!(
                    view = ?pending.view,
                    "deferred decide processing failed: {err:#}"
                );
            },
        }
    }
}

/// Number of attempts to recover a view's payload from peers before giving up and leaving
/// the gap to the query service's own fetching.
const PAYLOAD_RECOVERY_ATTEMPTS: u32 = 3;

/// Pause between payload-recovery attempts, so a fast-failing error (e.g. stake table catchup
/// still in flight) doesn't burn every attempt within milliseconds.
const PAYLOAD_RECOVERY_RETRY_DELAY: Duration = Duration::from_secs(1);

/// Spawn background recovery of `missing` leaves' payloads from peers. Each leaf is reported by
/// exactly one successful pass (the cursor advances past it), so recovery runs once per leaf.
fn spawn_payload_recovery<P, C>(
    payload_recovery: &Arc<dyn DecidePayloadRecovery>,
    persistence: &Arc<P>,
    consumer: &Arc<C>,
    decided_view: u64,
    missing: Vec<Leaf2<SeqTypes>>,
    metrics: &DecideProcessorMetrics,
) where
    P: SequencerPersistence,
    C: PersistenceEventConsumer + 'static,
{
    let leaves = missing
        .into_iter()
        .filter(|leaf| {
            // Recovery is only supported for new-protocol (V2) payload commitments, and only
            // within the window peers retain DA proposals for: anything older has likely been
            // pruned everywhere and is left to the query service's peer fetching instead.
            matches!(
                leaf.block_header().payload_commitment(),
                VidCommitment::V2(_)
            ) && decided_view.saturating_sub(leaf.view_number().u64()) <= PAYLOAD_RETENTION_VIEWS
        })
        .collect::<Vec<_>>();
    if leaves.is_empty() {
        return;
    }
    spawn(recover_missing_payloads(
        payload_recovery.clone(),
        persistence.clone(),
        consumer.clone(),
        leaves,
        metrics.payloads_recovered.clone(),
        metrics.payload_recovery_failures.clone(),
    ));
}

/// Fetch missing block payloads from peers, persist each to consensus storage, then forward it
/// (and the regenerated VID common) to the query service.
pub(crate) async fn recover_missing_payloads<P, C>(
    recovery: Arc<dyn DecidePayloadRecovery>,
    persistence: Arc<P>,
    consumer: Arc<C>,
    leaves: Vec<Leaf2<SeqTypes>>,
    recovered: Arc<dyn Counter>,
    failures: Arc<dyn Counter>,
) where
    P: SequencerPersistence,
    C: PersistenceEventConsumer + 'static,
{
    for leaf in leaves {
        let view = leaf.view_number();
        let mut recovered_payload = None;
        for attempt in 1..=PAYLOAD_RECOVERY_ATTEMPTS {
            if attempt > 1 {
                sleep(PAYLOAD_RECOVERY_RETRY_DELAY).await;
            }
            match recovery.recover_payload(&leaf).await {
                Ok(Some(found)) => {
                    recovered_payload = Some(found);
                    break;
                },
                Ok(None) => {
                    tracing::warn!(?view, attempt, "could not recover block payload from peers");
                },
                Err(err) => {
                    tracing::warn!(?view, attempt, "payload recovery failed: {err:#}");
                },
            }
        }
        let Some(RecoveredPayload {
            proposal,
            vid_common,
        }) = recovered_payload
        else {
            failures.add(1);
            continue;
        };
        tracing::info!(?view, "recovered block payload from peers");
        recovered.add(1);

        // Consensus storage first, so the payload survives a restart and can be served to
        // peers; the write is idempotent.
        if let Err(err) = persistence
            .append_da2(&proposal, leaf.block_header().payload_commitment())
            .await
        {
            tracing::warn!(?view, "failed to store recovered payload: {err:#}");
        }

        // Then the query service: the payload, through the same event the coordinator emits
        // for late local reconstructions, ...
        let payload =
            Payload::from_bytes(&proposal.data.encoded_transactions, &proposal.data.metadata);
        let payload_event = CoordinatorEvent::BlockPayloadReconstructed {
            view,
            header: leaf.block_header().clone(),
            payload,
        };
        if let Err(err) = consumer.handle_event(&payload_event).await {
            tracing::warn!(
                ?view,
                "failed to store recovered payload in query service: {err:#}"
            );
        }

        // ... and the VID common regenerated from that payload, so VID-common queries are
        // served without waiting on the query service's own VID fetching.
        let vid_event = CoordinatorEvent::VidCommonRecovered {
            view,
            header: leaf.block_header().clone(),
            common: vid_common,
        };
        if let Err(err) = consumer.handle_event(&vid_event).await {
            tracing::warn!(
                ?view,
                "failed to store recovered VID common in query service: {err:#}"
            );
        }
    }
}

#[derive(Debug, Default, Clone)]
#[allow(clippy::type_complexity)]
pub(crate) struct TaskList(Arc<Mutex<Vec<(String, JoinHandle<()>)>>>);

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
