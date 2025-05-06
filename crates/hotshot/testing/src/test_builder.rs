// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::HashMap, num::NonZeroUsize, rc::Rc, sync::Arc, time::Duration};

use alloy::primitives::U256;
use async_lock::RwLock;
use hotshot::{
    tasks::EventTransformerState,
    traits::{NetworkReliability, NodeImplementation, TestableNodeImplementation},
    types::SystemContextHandle,
    HotShotInitializer, MarketplaceConfig, SystemContext, TwinsHandlerState,
};
use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResultsProvider, node_types::TestTypes,
    state_types::TestInstanceState, storage_types::TestStorage, testable_delay::DelayConfig,
};
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        node_implementation::{NodeType, Versions},
        storage::storage_add_drb_result,
    },
    HotShotConfig, PeerConfig, ValidatorConfig,
};
use hotshot_utils::anytrace::*;
use tide_disco::Url;
use vec1::Vec1;

use super::{
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    txn_task::TxnTaskDescription,
};
use crate::{
    helpers::{key_pair_for_id, TestNodeKeyMap},
    spinning_task::SpinningTaskDescription,
    test_launcher::{Network, ResourceGenerators, TestLauncher},
    test_task::TestTaskStateSeed,
    view_sync_task::ViewSyncTaskDescription,
};

pub type TransactionValidator = Arc<dyn Fn(&Vec<(u64, u64)>) -> Result<()> + Send + Sync>;

/// data describing how a round should be timed.
#[derive(Clone, Debug, Copy)]
pub struct TimingData {
    /// Base duration for next-view timeout, in milliseconds
    pub next_view_timeout: u64,
    /// The maximum amount of time a leader can wait to get a block from a builder
    pub builder_timeout: Duration,
    /// time to wait until we request data associated with a proposal
    pub data_request_delay: Duration,
    /// Delay before sending through the secondary network in CombinedNetworks
    pub secondary_network_delay: Duration,
    /// view sync timeout
    pub view_sync_timeout: Duration,
}

pub fn default_hotshot_config<TYPES: NodeType>(
    known_nodes_with_stake: Vec<PeerConfig<TYPES>>,
    known_da_nodes: Vec<PeerConfig<TYPES>>,
    num_bootstrap_nodes: usize,
    epoch_height: u64,
    epoch_start_block: u64,
) -> HotShotConfig<TYPES> {
    HotShotConfig {
        start_threshold: (1, 1),
        num_nodes_with_stake: NonZeroUsize::new(known_nodes_with_stake.len()).unwrap(),
        known_da_nodes: known_da_nodes.clone(),
        num_bootstrap: num_bootstrap_nodes,
        known_nodes_with_stake: known_nodes_with_stake.clone(),
        da_staked_committee_size: known_da_nodes.len(),
        fixed_leader_for_gpuvid: 1,
        next_view_timeout: 500,
        view_sync_timeout: Duration::from_millis(250),
        builder_timeout: Duration::from_millis(1000),
        data_request_delay: Duration::from_millis(200),
        // Placeholder until we spin up the builder
        builder_urls: vec1::vec1![Url::parse("http://localhost:9999").expect("Valid URL")],
        start_proposing_view: u64::MAX,
        stop_proposing_view: 0,
        start_voting_view: u64::MAX,
        stop_voting_view: 0,
        start_proposing_time: u64::MAX,
        stop_proposing_time: 0,
        start_voting_time: u64::MAX,
        stop_voting_time: 0,
        epoch_height,
        epoch_start_block,
    }
}

#[allow(clippy::type_complexity)]
pub fn gen_node_lists<TYPES: NodeType>(
    num_staked_nodes: u64,
    num_da_nodes: u64,
) -> (Vec<PeerConfig<TYPES>>, Vec<PeerConfig<TYPES>>) {
    let mut staked_nodes = Vec::new();
    let mut da_nodes = Vec::new();

    for n in 0..num_staked_nodes {
        let validator_config: ValidatorConfig<TYPES> = ValidatorConfig::generated_from_seed_indexed(
            [0u8; 32],
            n,
            U256::from(1),
            n < num_da_nodes,
        );

        let peer_config = validator_config.public_config();
        staked_nodes.push(peer_config.clone());

        if n < num_da_nodes {
            da_nodes.push(peer_config)
        }
    }

    (staked_nodes, da_nodes)
}

/// metadata describing a test
#[derive(Clone)]
pub struct TestDescription<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    /// `HotShotConfig` used for setting up the test infrastructure.
    ///
    /// Note: this is not the same as the `HotShotConfig` passed to test nodes for `SystemContext::init`;
    /// those configs are instead provided by the resource generators in the test launcher.
    pub test_config: HotShotConfig<TYPES>,
    /// Whether to skip initializing nodes that will start late, which will catch up later with
    /// `HotShotInitializer::from_reload` in the spinning task.
    pub skip_late: bool,
    /// overall safety property description
    pub overall_safety_properties: OverallSafetyPropertiesDescription,
    /// spinning properties
    pub spinning_properties: SpinningTaskDescription,
    /// txns timing
    pub txn_description: TxnTaskDescription,
    /// completion task
    pub completion_task_description: CompletionTaskDescription,
    /// timing data
    pub timing_data: TimingData,
    /// unrelabile networking metadata
    pub unreliable_network: Option<Box<dyn NetworkReliability>>,
    /// view sync check task
    pub view_sync_properties: ViewSyncTaskDescription,
    /// description of builders to run
    pub builders: Vec1<BuilderDescription>,
    /// description of fallback builder to run
    pub fallback_builder: BuilderDescription,
    /// description of the solver to run
    pub solver: FakeSolverApiDescription,
    /// nodes with byzantine behaviour
    pub behaviour: Rc<dyn Fn(u64) -> Behaviour<TYPES, I, V>>,
    /// Delay config if any to add delays to asynchronous calls
    pub async_delay_config: DelayConfig,
    /// view in which to propose an upgrade
    pub upgrade_view: Option<u64>,
    /// whether to initialize the solver on startup
    pub start_solver: bool,
    /// boxed closure used to validate the resulting transactions
    pub validate_transactions: TransactionValidator,
}

pub fn nonempty_block_threshold(threshold: (u64, u64)) -> TransactionValidator {
    Arc::new(move |transactions| {
        if matches!(threshold, (0, _)) {
            return Ok(());
        }

        let blocks: Vec<_> = transactions.iter().filter(|(view, _)| *view != 0).collect();

        let num_blocks = blocks.len() as u64;
        let mut num_nonempty_blocks = 0;

        ensure!(num_blocks > 0, "Failed to commit any non-genesis blocks");

        for (_, num_transactions) in blocks {
            if *num_transactions > 0 {
                num_nonempty_blocks += 1;
            }
        }

        ensure!(
          // i.e. num_nonempty_blocks / num_blocks >= threshold.0 / threshold.1
          num_nonempty_blocks * threshold.1 >= threshold.0 * num_blocks,
          "Failed to meet nonempty block threshold of {}/{}; got {num_nonempty_blocks} nonempty blocks out of a total of {num_blocks}", threshold.0, threshold.1
        );

        Ok(())
    })
}

pub fn nonempty_block_limit(limit: (u64, u64)) -> TransactionValidator {
    Arc::new(move |transactions| {
        if matches!(limit, (_, 0)) {
            return Ok(());
        }

        let blocks: Vec<_> = transactions.iter().filter(|(view, _)| *view != 0).collect();

        let num_blocks = blocks.len() as u64;
        let mut num_nonempty_blocks = 0;

        ensure!(num_blocks > 0, "Failed to commit any non-genesis blocks");

        for (_, num_transactions) in blocks {
            if *num_transactions > 0 {
                num_nonempty_blocks += 1;
            }
        }

        ensure!(
          // i.e. num_nonempty_blocks / num_blocks <= limit.0 / limit.1
          num_nonempty_blocks * limit.1 <= limit.0 * num_blocks,
          "Exceeded nonempty block limit of {}/{}; got {num_nonempty_blocks} nonempty blocks out of a total of {num_blocks}", limit.0, limit.1
        );

        Ok(())
    })
}

#[derive(Debug)]
pub enum Behaviour<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> {
    ByzantineTwins(Box<dyn TwinsHandlerState<TYPES, I, V>>),
    Byzantine(Box<dyn EventTransformerState<TYPES, I, V>>),
    Standard,
}

pub async fn create_test_handle<
    TYPES: NodeType<InstanceState = TestInstanceState>,
    I: NodeImplementation<TYPES>,
    V: Versions,
>(
    metadata: TestDescription<TYPES, I, V>,
    node_id: u64,
    network: Network<TYPES, I>,
    memberships: Arc<RwLock<TYPES::Membership>>,
    config: HotShotConfig<TYPES>,
    storage: I::Storage,
    marketplace_config: MarketplaceConfig<TYPES, I>,
) -> SystemContextHandle<TYPES, I, V> {
    let initializer = HotShotInitializer::<TYPES>::from_genesis::<V>(
        TestInstanceState::new(metadata.async_delay_config),
        metadata.test_config.epoch_height,
        metadata.test_config.epoch_start_block,
        vec![],
    )
    .await
    .unwrap();

    // See whether or not we should be DA
    let is_da = node_id < config.da_staked_committee_size as u64;

    let validator_config: ValidatorConfig<TYPES> =
        ValidatorConfig::generated_from_seed_indexed([0u8; 32], node_id, U256::from(1), is_da);

    // Get key pair for certificate aggregation
    let private_key = validator_config.private_key.clone();
    let public_key = validator_config.public_key.clone();
    let state_private_key = validator_config.state_private_key.clone();
    let membership_coordinator = EpochMembershipCoordinator::new(
        memberships,
        Some(storage_add_drb_result(storage.clone())),
        config.epoch_height,
    );

    let behaviour = (metadata.behaviour)(node_id);
    match behaviour {
        Behaviour::ByzantineTwins(state) => {
            let state = Box::leak(state);
            let (left_handle, _right_handle) = state
                .spawn_twin_handles(
                    public_key,
                    private_key,
                    state_private_key,
                    node_id,
                    config,
                    membership_coordinator,
                    network,
                    initializer,
                    ConsensusMetricsValue::default(),
                    storage,
                    marketplace_config,
                )
                .await;

            left_handle
        },
        Behaviour::Byzantine(state) => {
            let state = Box::leak(state);
            state
                .spawn_handle(
                    public_key,
                    private_key,
                    state_private_key,
                    node_id,
                    config,
                    membership_coordinator,
                    network,
                    initializer,
                    ConsensusMetricsValue::default(),
                    storage,
                    marketplace_config,
                )
                .await
        },
        Behaviour::Standard => {
            let hotshot = SystemContext::<TYPES, I, V>::new(
                public_key,
                private_key,
                state_private_key,
                node_id,
                config,
                membership_coordinator,
                network,
                initializer,
                ConsensusMetricsValue::default(),
                storage,
                marketplace_config,
            )
            .await;

            hotshot.run_tasks().await
        },
    }
}

/// Describes a possible change to builder status during test
#[derive(Clone, Debug)]
pub enum BuilderChange {
    // Builder should start up
    Up,
    // Builder should shut down completely
    Down,
    // Toggles whether builder should always respond
    // to claim calls with errors
    FailClaims(bool),
}

/// Metadata describing builder behaviour during a test
#[derive(Clone, Debug, Default)]
pub struct BuilderDescription {
    /// view number -> change to builder status
    pub changes: HashMap<u64, BuilderChange>,
}

#[derive(Clone, Debug)]
pub struct FakeSolverApiDescription {
    /// The rate at which errors occur in the mock solver API
    pub error_pct: f32,
}

impl Default for TimingData {
    fn default() -> Self {
        Self {
            next_view_timeout: 4000,
            builder_timeout: Duration::from_millis(500),
            data_request_delay: Duration::from_millis(200),
            secondary_network_delay: Duration::from_millis(1000),
            view_sync_timeout: Duration::from_millis(2000),
        }
    }
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> TestDescription<TYPES, I, V> {
    /// the default metadata for a stress test
    #[must_use]
    #[allow(clippy::redundant_field_names)]
    pub fn default_stress() -> Self {
        let num_nodes_with_stake = 100;

        Self {
            overall_safety_properties: OverallSafetyPropertiesDescription {
                num_successful_views: 50,
                ..OverallSafetyPropertiesDescription::default()
            },
            timing_data: TimingData {
                next_view_timeout: 2000,
                ..TimingData::default()
            },
            view_sync_properties: ViewSyncTaskDescription::Threshold(0, num_nodes_with_stake),
            ..Self::default()
        }
    }

    /// the default metadata for multiple rounds
    #[must_use]
    #[allow(clippy::redundant_field_names)]
    pub fn default_multiple_rounds() -> Self {
        let num_nodes_with_stake = 10;
        TestDescription::<TYPES, I, V> {
            overall_safety_properties: OverallSafetyPropertiesDescription {
                num_successful_views: 20,
                ..OverallSafetyPropertiesDescription::default()
            },
            timing_data: TimingData {
                ..TimingData::default()
            },
            view_sync_properties: ViewSyncTaskDescription::Threshold(0, num_nodes_with_stake),
            ..TestDescription::<TYPES, I, V>::default()
        }
    }

    /// Default setting with 20 nodes and 8 views of successful views.
    #[must_use]
    #[allow(clippy::redundant_field_names)]
    pub fn default_more_nodes() -> Self {
        let num_nodes_with_stake = 20;
        let num_da_nodes = 14;
        let epoch_height = 10;
        let epoch_start_block = 1;

        let (staked_nodes, da_nodes) = gen_node_lists::<TYPES>(num_nodes_with_stake, num_da_nodes);

        Self {
            test_config: default_hotshot_config::<TYPES>(
                staked_nodes,
                da_nodes,
                num_nodes_with_stake.try_into().unwrap(),
                epoch_height,
                epoch_start_block,
            ),
            // The first 14 (i.e., 20 - f) nodes are in the DA committee and we may shutdown the
            // remaining 6 (i.e., f) nodes. We could remove this restriction after fixing the
            // following issue.
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    // Increase the duration to get the expected number of successful views.
                    duration: Duration::new(340, 0),
                },
            ),
            overall_safety_properties: OverallSafetyPropertiesDescription {
                ..Default::default()
            },
            timing_data: TimingData {
                next_view_timeout: 5000,
                ..TimingData::default()
            },
            view_sync_properties: ViewSyncTaskDescription::Threshold(
                0,
                num_nodes_with_stake.try_into().unwrap(),
            ),
            ..Self::default()
        }
    }

    pub fn set_num_nodes(self, num_nodes: u64, num_da_nodes: u64) -> Self {
        assert!(num_da_nodes <= num_nodes, "Cannot build test with fewer DA than total nodes. You may have mixed up the arguments to the function");

        let (staked_nodes, da_nodes) = gen_node_lists::<TYPES>(num_nodes, num_da_nodes);

        Self {
            test_config: default_hotshot_config::<TYPES>(
                staked_nodes,
                da_nodes,
                self.test_config.num_bootstrap,
                self.test_config.epoch_height,
                self.test_config.epoch_start_block,
            ),
            ..self
        }
    }

    pub fn build_node_key_map(&self) -> Arc<TestNodeKeyMap> {
        let mut node_key_map = TestNodeKeyMap::new();
        for i in 0..self.test_config.num_nodes_with_stake.into() {
            let (private_key, public_key) = key_pair_for_id::<TestTypes>(i as u64);
            node_key_map.insert(public_key, private_key);
        }

        Arc::new(node_key_map)
    }
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> Default
    for TestDescription<TYPES, I, V>
{
    /// by default, just a single round
    #[allow(clippy::redundant_field_names)]
    fn default() -> Self {
        let num_nodes_with_stake = 7;
        let num_da_nodes = num_nodes_with_stake;
        let epoch_height = 10;
        let epoch_start_block = 1;

        let (staked_nodes, da_nodes) = gen_node_lists::<TYPES>(num_nodes_with_stake, num_da_nodes);

        Self {
            test_config: default_hotshot_config::<TYPES>(
                staked_nodes,
                da_nodes,
                num_nodes_with_stake.try_into().unwrap(),
                epoch_height,
                epoch_start_block,
            ),
            timing_data: TimingData::default(),
            skip_late: false,
            spinning_properties: SpinningTaskDescription {
                node_changes: vec![],
            },
            overall_safety_properties: OverallSafetyPropertiesDescription::default(),
            // arbitrary, haven't done the math on this
            txn_description: TxnTaskDescription::RoundRobinTimeBased(Duration::from_millis(100)),
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    // TODO ED Put a configurable time here - 10 seconds for now
                    duration: Duration::from_millis(10000),
                },
            ),
            unreliable_network: None,
            view_sync_properties: ViewSyncTaskDescription::Threshold(
                0,
                num_nodes_with_stake.try_into().unwrap(),
            ),
            builders: vec1::vec1![BuilderDescription::default(), BuilderDescription::default(),],
            fallback_builder: BuilderDescription::default(),
            solver: FakeSolverApiDescription {
                // Default to a 10% error rate.
                error_pct: 0.1,
            },
            behaviour: Rc::new(|_| Behaviour::Standard),
            async_delay_config: DelayConfig::default(),
            upgrade_view: None,
            start_solver: true,
            validate_transactions: Arc::new(|_| Ok(())),
        }
    }
}

impl<
        TYPES: NodeType<InstanceState = TestInstanceState>,
        I: TestableNodeImplementation<TYPES>,
        V: Versions,
    > TestDescription<TYPES, I, V>
where
    I: NodeImplementation<TYPES, AuctionResultsProvider = TestAuctionResultsProvider<TYPES>>,
{
    /// turn a description of a test (e.g. a [`TestDescription`]) into
    /// a [`TestLauncher`] that can be used to launch the test.
    /// # Panics
    /// if some of the configuration values are zero
    pub fn gen_launcher(self) -> TestLauncher<TYPES, I, V> {
        self.gen_launcher_with_tasks(vec![])
    }

    /// turn a description of a test (e.g. a [`TestDescription`]) into
    /// a [`TestLauncher`] that can be used to launch the test, with
    /// additional testing tasks to run in test harness
    /// # Panics
    /// if some of the configuration values are zero
    #[must_use]
    pub fn gen_launcher_with_tasks(
        self,
        additional_test_tasks: Vec<Box<dyn TestTaskStateSeed<TYPES, I, V>>>,
    ) -> TestLauncher<TYPES, I, V> {
        let TestDescription {
            timing_data,
            unreliable_network,
            test_config,
            ..
        } = self.clone();

        let num_nodes_with_stake = test_config.num_nodes_with_stake.into();
        let num_bootstrap_nodes = test_config.num_bootstrap;
        let da_staked_committee_size = test_config.da_staked_committee_size;

        let validator_config = Rc::new(move |node_id| {
            ValidatorConfig::<TYPES>::generated_from_seed_indexed(
                [0u8; 32],
                node_id,
                U256::from(1),
                // This is the config for node 0
                node_id < test_config.da_staked_committee_size as u64,
            )
        });

        let hotshot_config = Rc::new(move |_| test_config.clone());
        let TimingData {
            next_view_timeout,
            builder_timeout,
            data_request_delay,
            secondary_network_delay,
            view_sync_timeout,
        } = timing_data;
        // TODO this should really be using the timing config struct
        let mod_hotshot_config = move |hotshot_config: &mut HotShotConfig<TYPES>| {
            hotshot_config.next_view_timeout = next_view_timeout;
            hotshot_config.builder_timeout = builder_timeout;
            hotshot_config.data_request_delay = data_request_delay;
            hotshot_config.view_sync_timeout = view_sync_timeout;
        };

        let metadata = self.clone();
        TestLauncher {
            resource_generators: ResourceGenerators {
                channel_generator: <I as TestableNodeImplementation<TYPES>>::gen_networks(
                    num_nodes_with_stake,
                    num_bootstrap_nodes,
                    da_staked_committee_size,
                    unreliable_network,
                    secondary_network_delay,
                ),
                storage: Rc::new(move |_| {
                    let mut storage = TestStorage::<TYPES>::default();
                    // update storage impl to use settings delay option
                    storage.delay_config = metadata.async_delay_config.clone();
                    storage
                }),
                hotshot_config,
                validator_config,
                marketplace_config: Rc::new(|_| MarketplaceConfig::<TYPES, I> {
                    auction_results_provider: TestAuctionResultsProvider::<TYPES>::default().into(),
                    fallback_builder_url: Url::parse("http://localhost:9999").unwrap(),
                }),
            },
            metadata: self,
            additional_test_tasks,
        }
        .map_hotshot_config(mod_hotshot_config)
    }
}
