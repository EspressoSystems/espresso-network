use std::{rc::Rc, time::Duration};

use hotshot_example_types::{
    node_types::{PushCdnImpl, TEST_VERSIONS},
    state_types::TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    byzantine::byzantine_behaviour::DishonestViewSyncWrongEpoch,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    test_builder::{Behaviour, TestDescription},
    view_sync_task::ViewSyncTaskDescription,
};

// Tests that dishonest nodes cannot form a precommit certificate for a new epoch without forming an eQC.
cross_tests!(
    TestName: view_sync_next_epoch,
    Impls: [PushCdnImpl],
    Types: [TestTypes],
    Versions: [TEST_VERSIONS.epoch],
    Ignore: false,
    Metadata: {
        let behaviour = Rc::new(move |node_id| {
            match node_id {
                0 | 1 | 9 => Behaviour::Byzantine(Box::new(DishonestViewSyncWrongEpoch {
                    first_dishonest_view_number: 9,
                    epoch_modifier: |e| e + 1,
                })),
                _ => Behaviour::Standard,
            }
        });

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_secs(240),
                },
            ),
            overall_safety_properties: OverallSafetyPropertiesDescription {
                num_successful_views: 30,
                ..OverallSafetyPropertiesDescription::default()
            },
            view_sync_properties: ViewSyncTaskDescription::Threshold(0, 13),
            behaviour,
            ..TestDescription::default()
        }.set_num_nodes(10, 10);

        metadata.overall_safety_properties.possible_view_failures = (0..100).collect();
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(60);
        metadata
    },
);

// Tests that dishonest nodes cannot form a precommit certificate for an old epoch.
cross_tests!(
    TestName: view_sync_old_epoch,
    Impls: [PushCdnImpl],
    Types: [TestTypes],
    Versions: [TEST_VERSIONS.epoch],
    Ignore: false,
    Metadata: {
        let behaviour = Rc::new(move |node_id| {
            match node_id {
                1..=3 => Behaviour::Byzantine(Box::new(DishonestViewSyncWrongEpoch {
                    first_dishonest_view_number: 11,
                    epoch_modifier: |e| e - 1,
                })),
                _ => Behaviour::Standard,
            }
        });

        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_secs(240),
                },
            ),
            overall_safety_properties: OverallSafetyPropertiesDescription {
                num_successful_views: 30,
                ..OverallSafetyPropertiesDescription::default()
            },
            view_sync_properties: ViewSyncTaskDescription::Threshold(0, 13),
            behaviour,
            ..TestDescription::default()
        }.set_num_nodes(10, 10);

        metadata.overall_safety_properties.possible_view_failures = (0..100).collect();
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(60);
        metadata
    },
);
