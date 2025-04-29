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
    v0::traits::{EventConsumer as PersistenceEventConsumer, SequencerPersistence},
    NodeState, PubKey, Transaction, ValidatedState,
};
use futures::{
    future::{join_all, Future},
    stream::{Stream, StreamExt},
};
use hotshot::{
    types::{Event, EventType, SystemContextHandle},
    MarketplaceConfig, SystemContext,
};
use hotshot_events_service::events_source::{EventConsumer, EventsStreamer};
use hotshot_orchestrator::client::OrchestratorClient;
use hotshot_query_service::data_source::storage::SqlStorage;
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    data::{Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    light_client::compute_stake_table_commitment,
    network::NetworkConfig,
    traits::{metrics::Metrics, network::ConnectedNetwork, node_implementation::Versions},
    PeerConfig, ValidatorConfig,
};
use parking_lot::Mutex;
use request_response::RequestResponseConfig;
use tokio::{spawn, sync::mpsc::channel, task::JoinHandle};
use tracing::{Instrument, Level};
use url::Url;

use crate::{
    catchup::ParallelStateCatchup,
    external_event_handler::ExternalEventHandler,
    proposal_fetcher::ProposalFetcherConfig,
    request_response::{
        data_source::DataSource, network::Sender as RequestResponseSender,
        recipient_source::RecipientSource, RequestResponseProtocol,
    },
    state_signature::StateSigner,
    Node, SeqTypes, SequencerApiVersion,
};

/// The consensus handle
pub type Consensus<N, P, V> = SystemContextHandle<SeqTypes, Node<N, P>, V>;

/// The sequencer context contains a consensus handle and other sequencer specific information.
#[derive(Derivative, Clone)]
#[derivative(Debug(bound = ""))]
pub struct SequencerContext<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, V: Versions> {
    /// The consensus handle
    #[derivative(Debug = "ignore")]
    handle: Arc<RwLock<Consensus<N, P, V>>>,

    /// The request-response protocol
    #[derivative(Debug = "ignore")]
    #[allow(dead_code)]
    request_response_protocol: RequestResponseProtocol<Node<N, P>, V, N, P>,

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

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, V: Versions> SequencerContext<N, P, V> {
    #[tracing::instrument(skip_all, fields(node_id = instance_state.node_id))]
    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        network_config: NetworkConfig<SeqTypes>,
        validator_config: ValidatorConfig<SeqTypes>,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        instance_state: NodeState,
        storage: Option<Arc<SqlStorage>>,
        state_catchup: ParallelStateCatchup,
        persistence: P,
        network: Arc<N>,
        state_relay_server: Option<Url>,
        metrics: &dyn Metrics,
        stake_table_capacity: usize,
        event_consumer: impl PersistenceEventConsumer + 'static,
        _: V,
        marketplace_config: MarketplaceConfig<SeqTypes, Node<N, P>>,
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
            .load_consensus_state::<V>(instance_state.clone())
            .await?;

        let stake_table_commit =
            compute_stake_table_commitment(&config.known_nodes_with_stake, stake_table_capacity)?;
        let stake_table_epoch = None;

        let event_streamer = Arc::new(RwLock::new(EventsStreamer::<SeqTypes>::new(
            config.known_nodes_with_stake.clone(),
            0,
        )));

        let persistence = Arc::new(persistence);

        let handle = SystemContext::init(
            validator_config.public_key,
            validator_config.private_key.clone(),
            validator_config.state_private_key.clone(),
            instance_state.node_id,
            config.clone(),
            coordinator.clone(),
            network.clone(),
            initializer,
            ConsensusMetricsValue::new(metrics),
            persistence.clone(),
            marketplace_config,
        )
        .await?
        .0;

        let mut state_signer = StateSigner::new(
            validator_config.state_private_key.clone(),
            validator_config.state_public_key.clone(),
            stake_table_commit,
            stake_table_epoch,
            stake_table_capacity,
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
            response_send_timeout: Duration::from_secs(10),
            request_batch_size: 15,
            request_batch_interval: Duration::from_secs(2),
            max_outgoing_responses: 20,
            response_validate_timeout: Duration::from_secs(1),
            max_incoming_responses: 20,
        };

        // Create the request-response protocol
        let request_response_protocol = RequestResponseProtocol::new(
            request_response_config,
            RequestResponseSender::new(outbound_message_sender),
            request_response_receiver,
            RecipientSource {
                memberships: coordinator,
                consensus: handle.hotshot.clone(),
                public_key: validator_config.public_key,
            },
            DataSource {
                node_state: instance_state.clone(),
                storage,
                consensus: handle.hotshot.clone(),
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
        let external_event_handler = ExternalEventHandler::<V>::new(
            &mut tasks,
            request_response_sender,
            outbound_message_receiver,
            network,
            pub_key,
            handle.hotshot.upgrade_lock.clone(),
        )
        .await
        .with_context(|| "Failed to create external event handler")?;

        Ok(Self::new(
            handle,
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
        handle: Consensus<N, P, V>,
        persistence: Arc<P>,
        state_signer: StateSigner<SequencerApiVersion>,
        external_event_handler: ExternalEventHandler<V>,
        request_response_protocol: RequestResponseProtocol<Node<N, P>, V, N, P>,
        event_streamer: Arc<RwLock<EventsStreamer<SeqTypes>>>,
        node_state: NodeState,
        network_config: NetworkConfig<SeqTypes>,
        validator_config: ValidatorConfig<SeqTypes>,
        event_consumer: impl PersistenceEventConsumer + 'static,
        anchor_view: Option<ViewNumber>,
        proposal_fetcher_cfg: ProposalFetcherConfig,
        metrics: &dyn Metrics,
    ) -> Self {
        let events = handle.event_stream();

        let node_id = node_state.node_id;
        let mut ctx = Self {
            handle: Arc::new(RwLock::new(handle)),
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
            ctx.handle.clone(),
            persistence.clone(),
            metrics,
        );

        // Spawn event handling loop.
        ctx.spawn(
            "event handler",
            handle_events(
                ctx.handle.clone(),
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
    pub async fn event_stream(&self) -> impl Stream<Item = Event<SeqTypes>> {
        self.handle.read().await.event_stream()
    }

    pub async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()> {
        self.handle.read().await.submit_transaction(tx).await?;
        Ok(())
    }

    /// get event streamer
    pub fn event_streamer(&self) -> Arc<RwLock<EventsStreamer<SeqTypes>>> {
        self.events_streamer.clone()
    }

    /// Return a reference to the underlying consensus handle.
    pub fn consensus(&self) -> Arc<RwLock<Consensus<N, P, V>>> {
        Arc::clone(&self.handle)
    }

    pub async fn shutdown_consensus(&self) {
        self.handle.write().await.shut_down().await
    }

    pub async fn decided_leaf(&self) -> Leaf2<SeqTypes> {
        self.handle.read().await.decided_leaf().await
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<ValidatedState>> {
        self.handle.read().await.state(view).await
    }

    pub async fn decided_state(&self) -> Arc<ValidatedState> {
        self.handle.read().await.decided_state().await
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
        self.handle.read().await.hotshot.start_consensus().await;
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
        self.handle.write().await.shut_down().await;
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

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, V: Versions> Drop
    for SequencerContext<N, P, V>
{
    fn drop(&mut self) {
        if !self.detached {
            // Spawn a task to shut down the context
            let handle_clone = self.handle.clone();
            let tasks_clone = self.tasks.clone();
            let node_state_clone = self.node_state.clone();

            spawn(async move {
                tracing::info!("shutting down SequencerContext");
                handle_clone.write().await.shut_down().await;
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
async fn handle_events<N, P, V>(
    consensus: Arc<RwLock<Consensus<N, P, V>>>,
    node_id: u64,
    mut events: impl Stream<Item = Event<SeqTypes>> + Unpin,
    persistence: Arc<P>,
    state_signer: Arc<RwLock<StateSigner<SequencerApiVersion>>>,
    external_event_handler: ExternalEventHandler<V>,
    events_streamer: Option<Arc<RwLock<EventsStreamer<SeqTypes>>>>,
    event_consumer: impl PersistenceEventConsumer + 'static,
    anchor_view: Option<ViewNumber>,
) where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
    V: Versions,
{
    if let Some(view) = anchor_view {
        // Process and clean up any leaves that we may have persisted last time we were running but
        // failed to handle due to a shutdown.
        if let Err(err) = persistence
            .append_decided_leaves(view, vec![], &event_consumer)
            .await
        {
            tracing::warn!(
                "failed to process decided leaves, chain may not be up to date: {err:#}"
            );
        }
    }

    while let Some(event) = events.next().await {
        tracing::debug!(node_id, ?event, "consensus event");

        // Store latest consensus state.
        persistence.handle_event(&event, &event_consumer).await;

        // Generate state signature.
        state_signer
            .write()
            .await
            .handle_event(&event, consensus.clone())
            .await;

        // Handle external messages
        if let EventType::ExternalMessageReceived { data, .. } = &event.event {
            if let Err(err) = external_event_handler.handle_event(data).await {
                tracing::warn!("Failed to handle external message: {:?}", err);
            };
        }

        // Send the event via the event streaming service
        if let Some(events_streamer) = events_streamer.as_ref() {
            events_streamer.write().await.handle_event(event).await;
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
