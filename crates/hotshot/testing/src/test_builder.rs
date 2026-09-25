// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::HashMap, num::NonZeroUsize, rc::Rc, time::Duration};

use hotshot::traits::TestableNodeImplementation;
use hotshot_example_types::storage_types::TestStorage;
use hotshot_types::{
    HotShotConfig, PeerConfig, ValidatorConfig,
    traits::{node_implementation::NodeType, signature_key::StakeTableEntryType},
};
use tide_disco::Url;
use versions::{MIN_SUPPORTED_VERSION, Upgrade};

use crate::{
    node_stake::TestNodeStakes,
    test_launcher::{ResourceGenerators, TestLauncher},
};

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

impl Default for TimingData {
    fn default() -> Self {
        Self {
            next_view_timeout: 6000,
            builder_timeout: Duration::from_millis(500),
            data_request_delay: Duration::from_millis(200),
            secondary_network_delay: Duration::from_millis(1000),
            view_sync_timeout: Duration::from_millis(2000),
        }
    }
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
        da_committees: Default::default(),
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
        stake_table_capacity: hotshot_types::light_client::DEFAULT_STAKE_TABLE_CAPACITY,
        drb_difficulty: 10,
        drb_upgrade_difficulty: 20,
    }
}

#[allow(clippy::type_complexity)]
pub fn gen_node_lists<TYPES: NodeType>(
    num_staked_nodes: u64,
    num_da_nodes: u64,
    node_stakes: &TestNodeStakes,
) -> (Vec<PeerConfig<TYPES>>, Vec<PeerConfig<TYPES>>) {
    let mut staked_nodes = Vec::new();
    let mut da_nodes = Vec::new();

    for n in 0..num_staked_nodes {
        let validator_config: ValidatorConfig<TYPES> = ValidatorConfig::generated_from_seed_indexed(
            [0u8; 32],
            n,
            node_stakes.get(n),
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

/// metadata describing a test network
#[derive(Clone)]
pub struct TestDescription<TYPES: NodeType> {
    /// `HotShotConfig` used for setting up the test infrastructure.
    ///
    /// Note: this is not the same as the `HotShotConfig` passed to test nodes for `SystemContext::init`;
    /// those configs are instead provided by the resource generators in the test launcher.
    pub test_config: HotShotConfig<TYPES>,
    /// timing data
    pub timing_data: TimingData,
    /// Configured version upgrade
    pub upgrade: versions::Upgrade,
    /// stake to apply to particular nodes. Nodes not included will have a stake of 1.
    pub node_stakes: TestNodeStakes,
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

impl<TYPES: NodeType> TestDescription<TYPES> {
    pub fn set_num_nodes(self, num_nodes: u64, num_da_nodes: u64) -> Self {
        assert!(
            num_da_nodes <= num_nodes,
            "Cannot build test with fewer DA than total nodes. You may have mixed up the \
             arguments to the function"
        );

        let (staked_nodes, da_nodes) =
            gen_node_lists::<TYPES>(num_nodes, num_da_nodes, &self.node_stakes);

        let upgrade = Upgrade::trivial(MIN_SUPPORTED_VERSION);
        Self {
            test_config: default_hotshot_config::<TYPES>(
                staked_nodes,
                da_nodes,
                self.test_config.num_bootstrap,
                self.test_config.epoch_height,
                self.test_config.epoch_start_block,
            ),
            upgrade,
            ..self
        }
    }

    /// turn a description of a test network into a [`TestLauncher`] whose resource
    /// generators produce each node's network, storage and config.
    /// # Panics
    /// if some of the configuration values are zero
    #[must_use]
    pub fn gen_launcher<I: TestableNodeImplementation<TYPES>>(mut self) -> TestLauncher<TYPES, I> {
        let mut connect_infos = HashMap::new();
        let networks = <I as TestableNodeImplementation<TYPES>>::gen_networks(
            self.test_config.num_nodes_with_stake.into(),
            self.test_config.num_bootstrap,
            self.test_config.da_staked_committee_size,
            None,
            self.timing_data.secondary_network_delay,
            &mut connect_infos,
        );

        // Update peer configs with address information created by `gen_networks`.
        for cfg in self.test_config.known_nodes_with_stake.iter_mut() {
            if let Some(info) = connect_infos.get(&cfg.stake_table_entry.public_key()) {
                cfg.connect_info = Some(info.clone())
            }
        }
        for cfg in self.test_config.known_da_nodes.iter_mut() {
            if let Some(info) = connect_infos.get(&cfg.stake_table_entry.public_key()) {
                cfg.connect_info = Some(info.clone())
            }
        }

        let TestDescription {
            timing_data,
            test_config,
            node_stakes,
            ..
        } = self.clone();

        let validator_config = Rc::new(move |node_id| {
            ValidatorConfig::<TYPES>::generated_from_seed_indexed(
                [0u8; 32],
                node_id,
                node_stakes.get(node_id),
                // This is the config for node 0
                node_id < test_config.da_staked_committee_size as u64,
            )
        });

        let hotshot_config = Rc::new(move |_| test_config.clone());

        let TimingData {
            next_view_timeout,
            builder_timeout,
            data_request_delay,
            view_sync_timeout,
            ..
        } = timing_data;

        // TODO this should really be using the timing config struct
        let mod_hotshot_config = move |hotshot_config: &mut HotShotConfig<TYPES>| {
            hotshot_config.next_view_timeout = next_view_timeout;
            hotshot_config.builder_timeout = builder_timeout;
            hotshot_config.data_request_delay = data_request_delay;
            hotshot_config.view_sync_timeout = view_sync_timeout;
        };

        TestLauncher {
            resource_generators: ResourceGenerators {
                channel_generator: networks,
                storage: Rc::new(|_| TestStorage::<TYPES>::default()),
                hotshot_config,
                validator_config,
            },
            metadata: self,
        }
        .map_hotshot_config(mod_hotshot_config)
    }
}

impl<TYPES: NodeType> Default for TestDescription<TYPES> {
    /// seven nodes, all on the DA committee, running the minimum supported version
    fn default() -> Self {
        let num_nodes_with_stake = 7;
        let num_da_nodes = num_nodes_with_stake;
        let epoch_height = 10;
        let epoch_start_block = 1;
        let node_stakes = TestNodeStakes::default();

        let (staked_nodes, da_nodes) =
            gen_node_lists::<TYPES>(num_nodes_with_stake, num_da_nodes, &node_stakes);

        Self {
            test_config: default_hotshot_config::<TYPES>(
                staked_nodes,
                da_nodes,
                num_nodes_with_stake.try_into().unwrap(),
                epoch_height,
                epoch_start_block,
            ),
            timing_data: TimingData::default(),
            upgrade: Upgrade::trivial(MIN_SUPPORTED_VERSION),
            node_stakes,
        }
    }
}
