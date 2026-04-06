use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use async_lock::RwLock;
use derivative::Derivative;
use espresso_types::{
    NodeState, PubKey, Transaction, ValidatedState,
    v0::traits::{EventConsumer as PersistenceEventConsumer, SequencerPersistence},
};
use futures::{
    future::{Future, join_all},
    stream::{BoxStream, Stream, StreamExt},
};
use hotshot::SystemContext;
use hotshot_events_service::events_source::{EventConsumer, EventsStreamer};
use hotshot_new_protocol::coordinator::{Coordinator, error::Severity};
use hotshot_orchestrator::client::OrchestratorClient;
use hotshot_types::{
    PeerConfig, ValidatorConfig,
    consensus::ConsensusMetricsValue,
    constants::EXTERNAL_EVENT_CHANNEL_SIZE,
    data::{Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
    storage_metrics::StorageMetricsValue,
    traits::{metrics::Metrics, network::ConnectedNetwork},
};
use parking_lot::Mutex;
use request_response::RequestResponseConfig;
use tokio::{spawn, sync::mpsc::channel, task::JoinHandle};
use tracing::{Instrument, Level};
use url::Url;

use crate::{
    Node, SeqTypes, SequencerApiVersion,
    catchup::ParallelStateCatchup,
    consensus_handle::{ConsensusEvent, ConsensusHandle, event_from_output},
    external_event_handler::ExternalEventHandler,
    proposal_fetcher::ProposalFetcherConfig,
    request_response::{
        RequestResponseProtocol,
        data_source::{DataSource, Storage as RequestResponseStorage},
        network::Sender as RequestResponseSender,
        recipient_source::RecipientSource,
    },
    state_signature::{self, StateSigner},
};
pub type Consensus<N, P> = hotshot::types::SystemContextHandle<SeqTypes, Node<N, P>>;

/// The sequencer context contains a consensus handle and other sequencer specific information.
#[derive(Derivative, Clone)]
#[derivative(Debug(bound = ""))]
pub struct SequencerContext<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> {
    /// The consensus adapter that dispatches between old HotShot and new coordinator.
    #[derivative(Debug = "ignore")]
    consensus_handle: Arc<ConsensusHandle<SeqTypes, Node<N, P>>>,

    /// The request-response protocol
    #[derivative(Debug = "ignore")]
    #[allow(dead_code)]
    pub request_response_protocol: RequestResponseProtocol<Node<N, P>, N, P>,

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

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> SequencerContext<N, P> {
    #[tracing::instrument(skip_all, fields(node_id = instance_state.node_id))]
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        network_config: NetworkConfig<SeqTypes>,
        upgrade: versions::Upgrade,
        validator_config: ValidatorConfig<SeqTypes>,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        instance_state: NodeState,
        storage: Option<RequestResponseStorage>,
        state_catchup: ParallelStateCatchup,
        persistence: Arc<P>,
        network: Arc<N>,
        state_relay_server: Option<Url>,
        metrics: &dyn Metrics,
        stake_table_capacity: usize,
        event_consumer: impl PersistenceEventConsumer + 'static,
        proposal_fetcher_cfg: ProposalFetcherConfig,
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
            coordinator.clone(),
            network.clone(),
            initializer,
            ConsensusMetricsValue::new(metrics),
            Arc::clone(&persistence),
            StorageMetricsValue::new(metrics),
        )
        .await?
        .0;

        let genesis_validated_state = ValidatedState::default();
        let genesis_leaf =
            Leaf2::genesis(&genesis_validated_state, &instance_state, upgrade.base).await;

        let epoch_height = network_config.config.epoch_height;
        let (new_coordinator, query_tx) = Coordinator::<SeqTypes, Node<N, P>>::new(
            coordinator.clone(),
            (*network).clone(),
            Arc::new(instance_state.clone()),
            validator_config.public_key,
            validator_config.private_key.clone(),
            genesis_leaf,
            epoch_height,
            Duration::from_secs(10),
        );
        let hotshot_handle = Arc::new(RwLock::new(handle));
        let (consensus_handle, event_sender) = ConsensusHandle::new(
            hotshot_handle.clone(),
            query_tx,
            epoch_height,
            EXTERNAL_EVENT_CHANNEL_SIZE,
        );
        let consensus_handle = Arc::new(consensus_handle);

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
                memberships: coordinator,
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

        // Create the external event handler
        let mut tasks = TaskList::default();
        tasks.spawn(
            "coordinator",
            run_coordinator(new_coordinator, event_sender),
        );
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
            metrics,
        )
        .with_task_list(tasks))
    }

    /// Constructor
    #[allow(clippy::too_many_arguments)]
    fn new(
        consensus_handle: Arc<ConsensusHandle<SeqTypes, Node<N, P>>>,
        persistence: Arc<P>,
        state_signer: StateSigner<SequencerApiVersion>,
        external_event_handler: ExternalEventHandler,
        request_response_protocol: RequestResponseProtocol<Node<N, P>, N, P>,
        event_streamer: Arc<RwLock<EventsStreamer<SeqTypes>>>,
        node_state: NodeState,
        network_config: NetworkConfig<SeqTypes>,
        validator_config: ValidatorConfig<SeqTypes>,
        event_consumer: impl PersistenceEventConsumer + 'static,
        anchor_view: Option<ViewNumber>,
        proposal_fetcher_cfg: ProposalFetcherConfig,
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

        // Spawn event handling loop.
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
                anchor_view,
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
    pub fn event_stream(&self) -> BoxStream<'static, ConsensusEvent<SeqTypes>> {
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
    pub fn consensus_handle(&self) -> Arc<ConsensusHandle<SeqTypes, Node<N, P>>> {
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

    pub async fn decided_state(&self) -> Arc<ValidatedState> {
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
            tracing::error!("Cannot get info from orchestrator client");
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

#[tracing::instrument(skip_all, fields(node_id))]
#[allow(clippy::too_many_arguments)]
async fn handle_events<N, P>(
    consensus_handle: Arc<ConsensusHandle<SeqTypes, Node<N, P>>>,
    node_id: u64,
    mut events: impl Stream<Item = ConsensusEvent<SeqTypes>> + Unpin,
    persistence: Arc<P>,
    state_signer: Arc<RwLock<StateSigner<SequencerApiVersion>>>,
    external_event_handler: ExternalEventHandler,
    events_streamer: Option<Arc<RwLock<EventsStreamer<SeqTypes>>>>,
    event_consumer: impl PersistenceEventConsumer + 'static,
    anchor_view: Option<ViewNumber>,
) where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    if let Some(view) = anchor_view {
        // Process and clean up any leaves that we may have persisted last time we were running but
        // failed to handle due to a shutdown.
        if let Err(err) = persistence
            .append_decided_leaves(view, vec![], None, &event_consumer)
            .await
        {
            tracing::warn!(
                "failed to process decided leaves, chain may not be up to date: {err:#}"
            );
        }
    }

    while let Some(event) = events.next().await {
        tracing::debug!(node_id, ?event, "consensus event");

        match event {
            ConsensusEvent::LegacyEvent(ref hotshot_event) => {
                // Persistence and state signer consume the original HotShot event.
                persistence
                    .handle_event(hotshot_event, &event_consumer)
                    .await;
                state_signer
                    .write()
                    .await
                    .handle_event(hotshot_event, &consensus_handle)
                    .await;

                // Forward to the event streaming service.
                if let Some(events_streamer) = events_streamer.as_ref() {
                    events_streamer
                        .write()
                        .await
                        .handle_event(hotshot_event.clone())
                        .await;
                }
            },
            ConsensusEvent::NewDecide(_new_decide) => {
                // TODO: Handle new protocol decide events.
                // This will need to translate NewDecideEvent into the format
                // expected by persistence, state signer, and events streamer.
            },
            ConsensusEvent::ExternalMessageReceived { ref data, .. } => {
                if let Err(err) = external_event_handler.handle_event(data).await {
                    tracing::warn!("Failed to handle external message: {:?}", err);
                }
            },
            ConsensusEvent::QuorumProposal { .. } => {
                // Handled by the proposal fetcher via its own event stream.
            },
            ConsensusEvent::ViewChanged { .. } => {
                // View changes are tracked internally by the adapter.
            },
        }
    }
}

async fn run_coordinator<N, P>(
    mut coordinator: Coordinator<SeqTypes, Node<N, P>>,
    event_sender: async_broadcast::Sender<ConsensusEvent<SeqTypes>>,
) where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    loop {
        match coordinator.next_consensus_input().await {
            Ok(input) => coordinator.apply_consensus(input).await,
            Err(err) if err.severity == Severity::Critical => {
                tracing::error!(%err, "coordinator: critical error");
                return;
            },
            Err(err) => {
                tracing::warn!(%err, "coordinator: non-critical error");
            },
        }
        while let Some(output) = coordinator.outbox_mut().pop_front() {
            if let Some(event) = event_from_output(&output) {
                match event_sender.broadcast_direct(event).await {
                    Ok(None) => {},
                    Ok(Some(overflowed)) => {
                        tracing::debug!(
                            ?overflowed,
                            "coordinator event channel overflow, oldest event dropped"
                        );
                    },
                    Err(err) => {
                        tracing::warn!(%err, "failed to broadcast consensus event");
                    },
                }
            }
            if let Err(err) = coordinator.process_consensus_output(output).await {
                if err.severity == Severity::Critical {
                    tracing::error!(%err, "coordinator: critical error processing output");
                    return;
                } else {
                    tracing::warn!(%err, "coordinator: error processing output");
                }
            }
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
