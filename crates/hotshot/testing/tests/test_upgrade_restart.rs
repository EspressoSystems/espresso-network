// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
    EpochUpgradeTestVersions, MemoryImpl, TestTypes, TestTypesRandomizedLeader,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::TestDescription,
};

cross_tests!(
    TestName: test_epoch_upgrade_restart_1,
    Impls: [MemoryImpl],
    Types: [TestTypes, TestTypesRandomizedLeader],
    Versions: [EpochUpgradeTestVersions],
    Ignore: false,
    Metadata: {

        let mut catchup_nodes = vec![];
        for i in 0..7 {
            catchup_nodes.push(ChangeNode {
                idx: i,
                updown: NodeAction::RestartDown(0),
            })
        }

        let spinning_properties = SpinningTaskDescription {
            // Restart all the nodes in view 40, i.e. after the upgrade is decided but before the first epoch
            node_changes: vec![(40, catchup_nodes)],
        };

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(120),
                                             },
                                         ),
            upgrade_view: Some(5),
            spinning_properties,
            ..TestDescription::default()
        };

        // Keep going until the 2nd epoch transition
        metadata.overall_safety_properties.num_successful_views = 110;
        metadata.test_config.epoch_height = 50;

        metadata
    },
);

cross_tests!(
    TestName: test_epoch_upgrade_restart_2,
    Impls: [MemoryImpl],
    Types: [TestTypes, TestTypesRandomizedLeader],
    // TODO: we need some test infrastructure + Membership trait fixes to get this to work with:
    // Types: [TestTypes, TestTypesRandomizedLeader, TestTwoStakeTablesTypes],
    Versions: [EpochUpgradeTestVersions],
    Ignore: false,
    Metadata: {

        let mut catchup_nodes = vec![];
        for i in 0..7 {
            catchup_nodes.push(ChangeNode {
                idx: i,
                updown: NodeAction::RestartDown(0),
            })
        }

        let spinning_properties = SpinningTaskDescription {
            // Restart all the nodes in view 70, i.e. between the first and second epochs
            node_changes: vec![(70, catchup_nodes)],
        };

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(120),
                                             },
                                         ),
            upgrade_view: Some(5),
            spinning_properties,
            ..TestDescription::default()
        };

        // Keep going until the 2nd epoch transition
        metadata.overall_safety_properties.num_successful_views = 110;
        metadata.test_config.epoch_height = 50;

        metadata
    },
);
