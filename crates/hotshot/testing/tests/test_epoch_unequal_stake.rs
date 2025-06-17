// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use alloy::primitives::U256;
use hotshot_example_types::node_types::{
    EpochsTestVersions, Libp2pImpl, MemoryImpl, PushCdnImpl, TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    node_stake::TestNodeStakes,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::TestDescription,
};

// This one only really works with StaticCommittee, because we know in advance which nodes will be the leader
// and can tailor our view failure set against that.
cross_tests!(
    TestName: test_unequal_stake_success_with_failing_majority_count,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(120),
                                             },
                                         ),
            ..TestDescription::default_with_stake(
                TestNodeStakes::default()
                    .with_stake(0, U256::from(10))
                    .with_stake(1, U256::from(10))
                    .with_stake(2, U256::from(10))
                ).set_num_nodes(9, 3)
        };
        metadata.test_config.epoch_height = 10;
        let dead_nodes = vec![
            ChangeNode {
                idx: 3,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 4,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 5,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 6,
                updown: NodeAction::Down,
            },
        ];

        // Even though several nodes are down, we should still succeed because nodes 0-2 have a disproportionately large stake
        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, dead_nodes)]
        };

        // We're going to have a lot of view failures of course, but with equal stake we should stop making progress at view 3
        metadata.overall_safety_properties.num_successful_views = 10;
        metadata.overall_safety_properties.expected_view_failures = vec![5];
        metadata.overall_safety_properties.possible_view_failures = vec![4, 6, 11, 12, 13, 14, 15];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(30);

        metadata
    },
);
