// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(clippy::panic)]
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
};

use alloy::primitives::U256;
use async_broadcast::{broadcast, Receiver, Sender};
use async_lock::RwLock;
use futures::future::join_all;
use hotshot::{
    traits::TestableNodeImplementation,
    types::{Event, SystemContextHandle},
    HotShotInitializer, InitializerEpochInfo, MarketplaceConfig, SystemContext,
};
use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResultsProvider,
    block_types::TestBlockHeader,
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_fakeapi::fake_solver::FakeSolverState;
use hotshot_task_impls::events::HotShotEvent;
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    constants::EVENT_CHANNEL_SIZE,
    data::Leaf2,
    drb::INITIAL_DRB_RESULT,
    epoch_membership::EpochMembershipCoordinator,
    simple_certificate::QuorumCertificate2,
    traits::{
        election::Membership,
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType, Versions},
        storage::storage_add_drb_result,
    },
    HotShotConfig, ValidatorConfig,
};
use tide_disco::Url;
use tokio::{spawn, task::JoinHandle};
#[allow(deprecated)]
use tracing::info;

use super::{
    completion_task::CompletionTask, consistency_task::ConsistencyTask, txn_task::TxnTask,
};
use crate::{
    block_builder::{BuilderTask, TestBuilderImplementation},
    completion_task::CompletionTaskDescription,
    spinning_task::{ChangeNode, NodeAction, SpinningTask},
    test_builder::create_test_handle,
    test_launcher::{Network, TestLauncher},
    test_task::{spawn_timeout_task, TestResult, TestTask},
    txn_task::TxnTaskDescription,
    view_sync_task::ViewSyncTask,
};

pub trait TaskErr: std::error::Error + Sync + Send + 'static {}
impl<T: std::error::Error + Sync + Send + 'static> TaskErr for T {}

impl<
        TYPES: NodeType<
            InstanceState = TestInstanceState,
            ValidatedState = TestValidatedState,
            BlockHeader = TestBlockHeader,
        >,
        I: TestableNodeImplementation<TYPES>,
        V: Versions,
        N: ConnectedNetwork<TYPES::SignatureKey>,
    > TestRunner<TYPES, I, V, N>
where
    I: TestableNodeImplementation<TYPES>,
    I: NodeImplementation<
        TYPES,
        Network = N,
        Storage = TestStorage<TYPES>,
        AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
    >,
{
    /// execute test
    ///
    /// # Panics
    /// if the test fails
    #[allow(clippy::too_many_lines)]
    pub async fn run_test<B: TestBuilderImplementation<TYPES>>(mut self) {
        let (test_sender, test_receiver) = broadcast(EVENT_CHANNEL_SIZE);
        let spinning_changes = self
            .launcher
            .metadata
            .spinning_properties
            .node_changes
            .clone();

        let mut late_start_nodes: HashSet<u64> = HashSet::new();
        let mut restart_nodes: HashSet<u64> = HashSet::new();
        for (_, changes) in &spinning_changes {
            for change in changes {
                if matches!(change.updown, NodeAction::Up) {
                    late_start_nodes.insert(change.idx.try_into().unwrap());
                }
                if matches!(change.updown, NodeAction::RestartDown(_)) {
                    restart_nodes.insert(change.idx.try_into().unwrap());
                }
            }
        }

        self.add_nodes::<B>(
            self.launcher
                .metadata
                .test_config
                .num_nodes_with_stake
                .into(),
            &late_start_nodes,
            &restart_nodes,
        )
        .await;
        let mut event_rxs = vec![];
        let mut internal_event_rxs = vec![];

        for node in &self.nodes {
            let r = node.handle.event_stream_known_impl();
            event_rxs.push(r);
        }
        for node in &self.nodes {
            let r = node.handle.internal_event_stream_receiver_known_impl();
            internal_event_rxs.push(r);
        }

        let TestRunner {
            launcher,
            nodes,
            solver_server,
            late_start,
            next_node_id: _,
            _pd: _,
        } = self;

        let mut task_futs = vec![];
        let meta = launcher.metadata.clone();

        let handles = Arc::new(RwLock::new(nodes));

        let txn_task =
            if let TxnTaskDescription::RoundRobinTimeBased(duration) = meta.txn_description {
                let txn_task = TxnTask {
                    handles: Arc::clone(&handles),
                    next_node_idx: Some(0),
                    duration,
                    shutdown_chan: test_receiver.clone(),
                };
                Some(txn_task)
            } else {
                None
            };

        // add completion task
        let CompletionTaskDescription::TimeBasedCompletionTaskBuilder(time_based) =
            meta.completion_task_description;
        let completion_task = CompletionTask {
            tx: test_sender.clone(),
            rx: test_receiver.clone(),
            duration: time_based.duration,
        };

        // add spinning task
        // map spinning to view
        let mut changes: BTreeMap<TYPES::View, Vec<ChangeNode>> = BTreeMap::new();
        for (view, mut change) in spinning_changes {
            changes
                .entry(TYPES::View::new(view))
                .or_insert_with(Vec::new)
                .append(&mut change);
        }

        let spinning_task_state = SpinningTask {
            epoch_height: launcher.metadata.test_config.epoch_height,
            epoch_start_block: launcher.metadata.test_config.epoch_start_block,
            start_epoch_info: Vec::new(), // #2652 REVIEW NOTE: Same as other instances of start_epoch_info
            handles: Arc::clone(&handles),
            late_start,
            latest_view: None,
            changes,
            last_decided_leaf: Leaf2::genesis::<V>(
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await,
            high_qc: QuorumCertificate2::genesis::<V>(
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await,
            next_epoch_high_qc: None,
            async_delay_config: launcher.metadata.async_delay_config,
            restart_contexts: HashMap::new(),
            channel_generator: launcher.resource_generators.channel_generator,
            state_cert: None,
        };
        let spinning_task = TestTask::<SpinningTask<TYPES, N, I, V>>::new(
            spinning_task_state,
            event_rxs.clone(),
            test_receiver.clone(),
        );

        let consistency_task_state = ConsistencyTask {
            consensus_leaves: BTreeMap::new(),
            safety_properties: launcher.metadata.overall_safety_properties.clone(),
            test_sender: test_sender.clone(),
            errors: vec![],
            ensure_upgrade: launcher.metadata.upgrade_view.is_some(),
            validate_transactions: launcher.metadata.validate_transactions,
            timeout_task: spawn_timeout_task(
                test_sender.clone(),
                launcher.metadata.overall_safety_properties.decide_timeout,
            ),
            _pd: PhantomData,
        };

        let consistency_task = TestTask::<ConsistencyTask<TYPES, V>>::new(
            consistency_task_state,
            event_rxs.clone(),
            test_receiver.clone(),
        );

        // add view sync task
        let view_sync_task_state = ViewSyncTask {
            hit_view_sync: HashSet::new(),
            description: launcher.metadata.view_sync_properties,
            _pd: PhantomData,
        };

        let view_sync_task = TestTask::<ViewSyncTask<TYPES, I>>::new(
            view_sync_task_state,
            internal_event_rxs,
            test_receiver.clone(),
        );

        let nodes = handles.read().await;

        // wait for networks to be ready
        for node in &*nodes {
            node.network.wait_for_ready().await;
        }

        // Start hotshot
        for node in &*nodes {
            if !late_start_nodes.contains(&node.node_id) {
                node.handle.hotshot.start_consensus().await;
            }
        }

        drop(nodes);

        for seed in launcher.additional_test_tasks {
            let task = TestTask::new(
                seed.into_state(Arc::clone(&handles)).await,
                event_rxs.clone(),
                test_receiver.clone(),
            );
            task_futs.push(task.run());
        }

        task_futs.push(consistency_task.run());
        task_futs.push(view_sync_task.run());
        task_futs.push(spinning_task.run());

        // `generator` tasks that do not process events.
        let txn_handle = txn_task.map(|txn| txn.run());
        let completion_handle = completion_task.run();

        let mut error_list = vec![];

        let results = join_all(task_futs).await;

        for result in results {
            match result {
                Ok(res) => match res {
                    TestResult::Pass => {
                        info!("Task shut down successfully");
                    },
                    TestResult::Fail(e) => error_list.push(e),
                },
                Err(e) => {
                    tracing::error!("Error Joining the test task {:?}", e);
                },
            }
        }

        if let Some(handle) = txn_handle {
            handle.abort();
        }
        // Shutdown all of the servers at the end
        // Aborting here doesn't cause any problems because we don't maintain any state
        if let Some(solver_server) = solver_server {
            solver_server.1.abort();
        }

        let mut nodes = handles.write().await;

        for node in &mut *nodes {
            node.handle.shut_down().await;
        }
        tracing::info!("Nodes shutdown");

        completion_handle.abort();

        assert!(
            error_list.is_empty(),
            "{}",
            error_list
                .iter()
                .fold("TEST FAILED! Results:".to_string(), |acc, error| {
                    format!("{acc}\n\n{error:?}")
                })
        );
    }

    pub async fn init_builders<B: TestBuilderImplementation<TYPES>>(
        &self,
    ) -> (Vec<Box<dyn BuilderTask<TYPES>>>, Vec<Url>, Url) {
        let config = self.launcher.metadata.test_config.clone();
        let mut builder_tasks = Vec::new();
        let mut builder_urls = Vec::new();
        for metadata in &self.launcher.metadata.builders {
            let builder_port = portpicker::pick_unused_port().expect("No free ports");
            let builder_url =
                Url::parse(&format!("http://localhost:{builder_port}")).expect("Invalid URL");
            let builder_task = B::start(
                0, // This field gets updated while the test is running, 0 is just to seed it
                builder_url.clone(),
                B::Config::default(),
                metadata.changes.clone(),
            )
            .await;
            builder_tasks.push(builder_task);
            builder_urls.push(builder_url);
        }

        let fallback_builder_port = portpicker::pick_unused_port().expect("No free ports");
        let fallback_builder_url =
            Url::parse(&format!("http://localhost:{fallback_builder_port}")).expect("Invalid URL");

        let fallback_builder_task = B::start(
            config.num_nodes_with_stake.into(),
            fallback_builder_url.clone(),
            B::Config::default(),
            self.launcher.metadata.fallback_builder.changes.clone(),
        )
        .await;

        builder_tasks.push(fallback_builder_task);

        (builder_tasks, builder_urls, fallback_builder_url)
    }

    /// Add auction solver.
    pub async fn add_solver(&mut self, builder_urls: Vec<Url>) {
        let solver_error_pct = self.launcher.metadata.solver.error_pct;
        let solver_port = portpicker::pick_unused_port().expect("No available ports");

        // This should basically never fail
        let solver_url: Url = format!("http://localhost:{solver_port}")
            .parse()
            .expect("Failed to parse solver URL");

        // Initialize the solver API state
        let solver_state = FakeSolverState::new(Some(solver_error_pct), builder_urls);

        // Then, fire it up as a background thread.
        self.solver_server = Some((
            solver_url.clone(),
            spawn(async move {
                solver_state
                    .run::<TYPES>(solver_url)
                    .await
                    .expect("Unable to run solver api");
            }),
        ));
    }

    /// Add nodes.
    ///
    /// # Panics
    /// Panics if unable to create a [`HotShotInitializer`]
    pub async fn add_nodes<B: TestBuilderImplementation<TYPES>>(
        &mut self,
        total: usize,
        late_start: &HashSet<u64>,
        restart: &HashSet<u64>,
    ) -> Vec<u64> {
        let mut results = vec![];
        let config = self.launcher.metadata.test_config.clone();

        // Num_nodes is updated on the fly now via claim_block_with_num_nodes. This stays around to seed num_nodes
        // in the builders for tests which don't update that field.
        let (mut builder_tasks, builder_urls, fallback_builder_url) =
            self.init_builders::<B>().await;

        if self.launcher.metadata.start_solver {
            self.add_solver(builder_urls.clone()).await;
        }

        // Collect uninitialized nodes because we need to wait for all networks to be ready before starting the tasks
        let mut uninitialized_nodes = Vec::new();
        let mut networks_ready = Vec::new();

        for i in 0..total {
            let mut config = config.clone();
            if let Some(upgrade_view) = self.launcher.metadata.upgrade_view {
                config.set_view_upgrade(upgrade_view);
            }
            let node_id = self.next_node_id;
            self.next_node_id += 1;
            tracing::debug!("launch node {}", i);

            config.builder_urls = builder_urls
                .clone()
                .try_into()
                .expect("Non-empty by construction");

            let network = (self.launcher.resource_generators.channel_generator)(node_id).await;
            let storage = (self.launcher.resource_generators.storage)(node_id);
            let mut marketplace_config =
                (self.launcher.resource_generators.marketplace_config)(node_id);
            if let Some(solver_server) = &self.solver_server {
                let mut new_auction_results_provider =
                    marketplace_config.auction_results_provider.as_ref().clone();

                new_auction_results_provider.broadcast_url = Some(solver_server.0.clone());

                marketplace_config.auction_results_provider = new_auction_results_provider.into();
            }

            marketplace_config.fallback_builder_url = fallback_builder_url.clone();

            let network_clone = network.clone();
            let networks_ready_future = async move {
                network_clone.wait_for_ready().await;
            };

            networks_ready.push(networks_ready_future);

            if late_start.contains(&node_id) {
                if self.launcher.metadata.skip_late {
                    self.late_start.insert(
                        node_id,
                        LateStartNode {
                            network: None,
                            context: LateNodeContext::UninitializedContext(
                                LateNodeContextParameters {
                                    storage,
                                    memberships: <TYPES as NodeType>::Membership::new(
                                        config.known_nodes_with_stake.clone(),
                                        config.known_da_nodes.clone(),
                                    ),
                                    config,
                                    marketplace_config,
                                },
                            ),
                        },
                    );
                } else {
                    let initializer = HotShotInitializer::<TYPES>::from_genesis::<V>(
                        TestInstanceState::new(self.launcher.metadata.async_delay_config.clone()),
                        config.epoch_height,
                        config.epoch_start_block,
                        vec![InitializerEpochInfo::<TYPES> {
                            epoch: TYPES::Epoch::new(1),
                            drb_result: INITIAL_DRB_RESULT,
                            block_header: None,
                        }],
                    )
                    .await
                    .unwrap();

                    // See whether or not we should be DA
                    let is_da = node_id < config.da_staked_committee_size as u64;

                    // We assign node's public key and stake value rather than read from config file since it's a test
                    let validator_config = ValidatorConfig::generated_from_seed_indexed(
                        [0u8; 32],
                        node_id,
                        U256::from(1),
                        is_da,
                    );

                    let hotshot = Self::add_node_with_config(
                        node_id,
                        network.clone(),
                        <TYPES as NodeType>::Membership::new(
                            config.known_nodes_with_stake.clone(),
                            config.known_da_nodes.clone(),
                        ),
                        initializer,
                        config,
                        validator_config,
                        storage,
                        marketplace_config,
                    )
                    .await;
                    self.late_start.insert(
                        node_id,
                        LateStartNode {
                            network: Some(network),
                            context: LateNodeContext::InitializedContext(hotshot),
                        },
                    );
                }
            } else {
                uninitialized_nodes.push((
                    node_id,
                    network,
                    <TYPES as NodeType>::Membership::new(
                        config.known_nodes_with_stake.clone(),
                        config.known_da_nodes.clone(),
                    ),
                    config,
                    storage,
                    marketplace_config,
                ));
            }

            results.push(node_id);
        }

        // Add the restart nodes after the rest.  This must be done after all the original networks are
        // created because this will reset the bootstrap info for the restarted nodes
        for node_id in &results {
            if restart.contains(node_id) {
                self.late_start.insert(
                    *node_id,
                    LateStartNode {
                        network: None,
                        context: LateNodeContext::Restart,
                    },
                );
            }
        }

        // Wait for all networks to be ready
        join_all(networks_ready).await;

        // Then start the necessary tasks
        for (node_id, network, memberships, config, storage, marketplace_config) in
            uninitialized_nodes
        {
            let handle = create_test_handle(
                self.launcher.metadata.clone(),
                node_id,
                network.clone(),
                Arc::new(RwLock::new(memberships)),
                config.clone(),
                storage,
                marketplace_config,
            )
            .await;

            match node_id.cmp(&(config.da_staked_committee_size as u64 - 1)) {
                std::cmp::Ordering::Less => {
                    if let Some(task) = builder_tasks.pop() {
                        task.start(Box::new(handle.event_stream()))
                    }
                },
                std::cmp::Ordering::Equal => {
                    // If we have more builder tasks than DA nodes, pin them all on the last node.
                    while let Some(task) = builder_tasks.pop() {
                        task.start(Box::new(handle.event_stream()))
                    }
                },
                std::cmp::Ordering::Greater => {},
            }

            self.nodes.push(Node {
                node_id,
                network,
                handle,
            });
        }

        results
    }

    /// add a specific node with a config
    /// # Panics
    /// if unable to initialize the node's `SystemContext` based on the config
    #[allow(clippy::too_many_arguments)]
    pub async fn add_node_with_config(
        node_id: u64,
        network: Network<TYPES, I>,
        memberships: TYPES::Membership,
        initializer: HotShotInitializer<TYPES>,
        config: HotShotConfig<TYPES>,
        validator_config: ValidatorConfig<TYPES>,
        storage: I::Storage,
        marketplace_config: MarketplaceConfig<TYPES, I>,
    ) -> Arc<SystemContext<TYPES, I, V>> {
        // Get key pair for certificate aggregation
        let private_key = validator_config.private_key.clone();
        let public_key = validator_config.public_key.clone();
        let state_private_key = validator_config.state_private_key.clone();
        let epoch_height = config.epoch_height;

        SystemContext::new(
            public_key,
            private_key,
            state_private_key,
            node_id,
            config,
            EpochMembershipCoordinator::new(
                Arc::new(RwLock::new(memberships)),
                Some(storage_add_drb_result(storage.clone())),
                epoch_height,
            ),
            network,
            initializer,
            ConsensusMetricsValue::default(),
            storage,
            marketplace_config,
        )
        .await
    }

    /// add a specific node with a config
    /// # Panics
    /// if unable to initialize the node's `SystemContext` based on the config
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub async fn add_node_with_config_and_channels(
        node_id: u64,
        network: Network<TYPES, I>,
        memberships: Arc<RwLock<TYPES::Membership>>,
        initializer: HotShotInitializer<TYPES>,
        config: HotShotConfig<TYPES>,
        validator_config: ValidatorConfig<TYPES>,
        storage: I::Storage,
        marketplace_config: MarketplaceConfig<TYPES, I>,
        internal_channel: (
            Sender<Arc<HotShotEvent<TYPES>>>,
            Receiver<Arc<HotShotEvent<TYPES>>>,
        ),
        external_channel: (Sender<Event<TYPES>>, Receiver<Event<TYPES>>),
    ) -> Arc<SystemContext<TYPES, I, V>> {
        // Get key pair for certificate aggregation
        let private_key = validator_config.private_key.clone();
        let public_key = validator_config.public_key.clone();
        let state_private_key = validator_config.state_private_key.clone();
        let epoch_height = config.epoch_height;

        SystemContext::new_from_channels(
            public_key,
            private_key,
            state_private_key,
            node_id,
            config,
            EpochMembershipCoordinator::new(
                memberships,
                Some(storage_add_drb_result(storage.clone())),
                epoch_height,
            ),
            network,
            initializer,
            ConsensusMetricsValue::default(),
            storage,
            marketplace_config,
            internal_channel,
            external_channel,
        )
        .await
    }
}

/// a node participating in a test
pub struct Node<TYPES: NodeType, I: TestableNodeImplementation<TYPES>, V: Versions> {
    /// The node's unique identifier
    pub node_id: u64,
    /// The underlying network belonging to the node
    pub network: Network<TYPES, I>,
    /// The handle to the node's internals
    pub handle: SystemContextHandle<TYPES, I, V>,
}

/// This type combines all of the parameters needed to build the context for a node that started
/// late during a unit test or integration test.
pub struct LateNodeContextParameters<TYPES: NodeType, I: TestableNodeImplementation<TYPES>> {
    /// The storage trait for Sequencer persistence.
    pub storage: I::Storage,

    /// The memberships of this particular node.
    pub memberships: TYPES::Membership,

    /// The config associated with this node.
    pub config: HotShotConfig<TYPES>,

    /// The marketplace config for this node.
    pub marketplace_config: MarketplaceConfig<TYPES, I>,
}

/// The late node context dictates how we're building a node that started late during the test.
#[allow(clippy::large_enum_variant)]
pub enum LateNodeContext<TYPES: NodeType, I: TestableNodeImplementation<TYPES>, V: Versions> {
    /// The system context that we're passing directly to the node, this means the node is already
    /// initialized successfully.
    InitializedContext(Arc<SystemContext<TYPES, I, V>>),

    /// The system context that we're passing to the node when it is not yet initialized, so we're
    /// initializing it based on the received leaf and init parameters.
    UninitializedContext(LateNodeContextParameters<TYPES, I>),
    /// The node is to be restarted so we will build the context from the node that was already running.
    Restart,
}

/// A yet-to-be-started node that participates in tests
pub struct LateStartNode<TYPES: NodeType, I: TestableNodeImplementation<TYPES>, V: Versions> {
    /// The underlying network belonging to the node
    pub network: Option<Network<TYPES, I>>,
    /// Either the context to which we will use to launch HotShot for initialized node when it's
    /// time, or the parameters that will be used to initialize the node and launch HotShot.
    pub context: LateNodeContext<TYPES, I, V>,
}

/// The runner of a test network
/// spin up and down nodes, execute rounds
pub struct TestRunner<
    TYPES: NodeType,
    I: TestableNodeImplementation<TYPES>,
    V: Versions,
    N: ConnectedNetwork<TYPES::SignatureKey>,
> {
    /// test launcher, contains a bunch of useful metadata and closures
    pub(crate) launcher: TestLauncher<TYPES, I, V>,
    /// nodes in the test
    pub(crate) nodes: Vec<Node<TYPES, I, V>>,
    /// the solver server running in the test
    pub(crate) solver_server: Option<(Url, JoinHandle<()>)>,
    /// nodes with a late start
    pub(crate) late_start: HashMap<u64, LateStartNode<TYPES, I, V>>,
    /// the next node unique identifier
    pub(crate) next_node_id: u64,
    /// Phantom for N
    pub(crate) _pd: PhantomData<N>,
}
