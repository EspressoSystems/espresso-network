// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::{
    membership::{
        randomized_committee::RandomizedStakeTable,
        randomized_committee_members::RandomizedCommitteeMembers,
        static_committee::StaticStakeTable, two_static_committees::TwoStakeTables,
    },
    node_types::{
        CombinedImpl, EpochsTestVersions, RandomOverlapQuorumFilterConfig,
        TestTypesEpochCatchupTypes,
    },
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::{TestDescription, TimingData},
};
use hotshot_types::signature_key::{BLSPubKey, SchnorrPubKey};

cross_tests!(
    TestName: test_catchup_epochs,
    Impls: [CombinedImpl],
    Types: [
        TestTypesEpochCatchupTypes<StaticStakeTable<BLSPubKey,SchnorrPubKey>>
    ],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 5000,
            ..Default::default()
        };

        let mut metadata = TestDescription::default().set_num_nodes(20, 7);

        let catchup_node = vec![ChangeNode {
            idx: 19,
            updown: NodeAction::Up,
        }];

        metadata.timing_data = timing_data;

        metadata.view_sync_properties =
            hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 20);

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(35, catchup_node)],
        };

        metadata.completion_task_description =
            CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_secs(120),
                },
            );
        metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 100,
            possible_view_failures: vec![18, 19, 38, 39],
            decide_timeout: Duration::from_secs(15),
            ..Default::default()
        };

        metadata.skip_late = true;

        metadata
    },
);

cross_tests!(
    TestName: test_two_stake_tables_catchup_epochs,
    Impls: [CombinedImpl],
    Types: [
        TestTypesEpochCatchupTypes<TwoStakeTables<BLSPubKey, SchnorrPubKey>>,
    ],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 5000,
            ..Default::default()
        };

        let mut metadata = TestDescription::default().set_num_nodes(20, 7);

        let catchup_node = vec![ChangeNode {
            idx: 18,
            updown: NodeAction::Up,
        }];

        metadata.timing_data = timing_data;

        metadata.view_sync_properties =
            hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 20);

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(35, catchup_node)],
        };

        metadata.completion_task_description =
            CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_secs(120),
                },
            );
        metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 100,
            possible_view_failures: vec![18, 19, 38, 39],
            decide_timeout: Duration::from_secs(15),
            ..Default::default()
        };

        metadata.skip_late = true;

        metadata
    },
);

cross_tests!(
    TestName: test_randomized_leader_catchup_epochs,
    Impls: [CombinedImpl],
    Types: [
        TestTypesEpochCatchupTypes<RandomizedStakeTable<BLSPubKey,SchnorrPubKey>>
    ],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 5000,
            ..Default::default()
        };

        let mut metadata = TestDescription::default().set_num_nodes(20, 7);

        let catchup_node = vec![ChangeNode {
            idx: 18,
            updown: NodeAction::Up,
        }];

        metadata.timing_data = timing_data;

        metadata.view_sync_properties =
            hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 20);

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(35, catchup_node)],
        };

        metadata.completion_task_description =
            CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_secs(120),
                },
            );
        metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 100,
            possible_view_failures: vec![33, 34, 39, 40],
            decide_timeout: Duration::from_secs(15),
            ..Default::default()
        };

        metadata.skip_late = true;

        metadata
    },
);

cross_tests!(
    TestName: test_randomized_committee_catchup_epochs,
    Impls: [CombinedImpl],
    Types: [
        TestTypesEpochCatchupTypes<RandomizedCommitteeMembers<BLSPubKey, SchnorrPubKey, RandomOverlapQuorumFilterConfig<123, 8, 10, 2, 5>, RandomOverlapQuorumFilterConfig<123, 3, 4, 1, 2>>>,
    ],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 5000,
            ..Default::default()
        };

        let mut metadata = TestDescription::default().set_num_nodes(20, 7);

        let catchup_node = vec![ChangeNode {
            idx: 10,
            updown: NodeAction::Up,
        }];

        metadata.timing_data = timing_data;

        metadata.view_sync_properties =
            hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 20);

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(35, catchup_node)],
        };

        metadata.completion_task_description =
            CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_secs(120),
                },
            );
        metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 100,
            possible_view_failures: vec![2, 3, 14, 15, 17, 18, 42, 43, 46, 47],
            decide_timeout: Duration::from_secs(20),
            ..Default::default()
        };

        metadata.skip_late = true;

        metadata
    },
);
