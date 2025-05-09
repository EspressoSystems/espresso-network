// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
    CombinedImpl, EpochsTestVersions, Libp2pImpl, PushCdnImpl, TestTwoStakeTablesTypes, TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::TestDescription,
};
// Test to make sure we can decide in just 3 views
// This test fails with the old decide rule
cross_tests!(
    TestName: test_shorter_decide,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_millis(100000),
                },
            ),
            ..TestDescription::default()
        };
        // after the first 3 leaders the next leader is down. It's a hack to make sure we decide in
        // 3 views or else we get a timeout
        let dead_nodes = vec![
            ChangeNode {
                idx: 4,
                updown: NodeAction::Down,
            },

        ];
        metadata.test_config.epoch_height = 10;
        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(1, dead_nodes)]
        };
        metadata.overall_safety_properties.num_successful_views = 1;
        metadata
    },
);
