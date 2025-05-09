// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
    CombinedImpl, EpochsTestVersions, TestTwoStakeTablesTypes, TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::{TestDescription, TimingData},
};

// A run where the CDN crashes part-way through, epochs enabled.
cross_tests!(
    TestName: test_combined_network_cdn_crash_with_epochs,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        };

        let overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            decide_timeout: Duration::from_secs(8),
            ..Default::default()
        };

        let completion_task_description = CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        );

        let mut metadata = TestDescription::default_multiple_rounds();
        metadata.timing_data = timing_data;
        metadata.overall_safety_properties = overall_safety_properties;
        metadata.completion_task_description = completion_task_description;

        let mut all_nodes = vec![];
        for node in 0..metadata.test_config.num_nodes_with_stake.into() {
            all_nodes.push(ChangeNode {
                idx: node,
                updown: NodeAction::NetworkDown,
            });
        }

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, all_nodes)],
        };

        metadata
    },
);

// A run where the CDN crashes partway through
// and then comes back up
cross_tests!(
    TestName: test_combined_network_reup_with_epochs,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        };

        let overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            decide_timeout: Duration::from_secs(10),
            ..Default::default()
        };

        let completion_task_description = CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        );

        let mut metadata = TestDescription::default_multiple_rounds();
        metadata.timing_data = timing_data;
        metadata.overall_safety_properties = overall_safety_properties;
        metadata.completion_task_description = completion_task_description;

        let mut all_down = vec![];
        let mut all_up = vec![];
        for node in 0..metadata.test_config.num_nodes_with_stake.into() {
            all_down.push(ChangeNode {
                idx: node,
                updown: NodeAction::NetworkDown,
            });
            all_up.push(ChangeNode {
                idx: node,
                updown: NodeAction::NetworkUp,
            });
        }

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(13, all_up), (5, all_down)],
        };

        metadata
    },
);

// A run where half of the nodes disconnect from the CDN
cross_tests!(
    TestName: test_combined_network_half_dc_with_epochs,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        };

        let overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            decide_timeout: Duration::from_secs(10),
            ..Default::default()
        };

        let completion_task_description = CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        );

        let mut metadata = TestDescription::default_multiple_rounds();
        metadata.timing_data = timing_data;
        metadata.overall_safety_properties = overall_safety_properties;
        metadata.completion_task_description = completion_task_description;

        let mut half = vec![];
        for node in 0..usize::from(metadata.test_config.num_nodes_with_stake) / 2 {
            half.push(ChangeNode {
                idx: node,
                updown: NodeAction::NetworkDown,
            });
        }

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, half)],
        };

        metadata
    },
);
