// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{Libp2pImpl, TestTypes, TestVersions};
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::{TestDescription, TimingData},
};
use tracing::instrument;

/// libp2p network test

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn libp2p_network() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, Libp2pImpl, TestVersions> = TestDescription {
        overall_safety_properties: OverallSafetyPropertiesDescription {
            check_leaf: true,
            ..Default::default()
        },
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::new(240, 0),
            },
        ),
        timing_data: TimingData {
            next_view_timeout: 4000,
            ..Default::default()
        },
        ..TestDescription::default_multiple_rounds()
    };
    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

/// libp2p network test with failures

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn libp2p_network_failures_2() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, Libp2pImpl, TestVersions> = TestDescription {
        overall_safety_properties: OverallSafetyPropertiesDescription {
            check_leaf: true,
            ..Default::default()
        },
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(240),
            },
        ),
        timing_data: TimingData {
            next_view_timeout: 4000,
            ..Default::default()
        },
        ..TestDescription::default_multiple_rounds()
    }
    .set_num_nodes(12, 12);
    metadata.test_config.epoch_height = 0;

    let dead_nodes = vec![ChangeNode {
        idx: 11,
        updown: NodeAction::Down,
    }];

    metadata.spinning_properties = SpinningTaskDescription {
        node_changes: vec![(3, dead_nodes)],
    };
    metadata.overall_safety_properties.expected_view_failures = vec![10, 11];
    metadata.overall_safety_properties.decide_timeout = Duration::from_secs(12);
    // Make sure we keep committing rounds after the bad leaders, but not the full 50 because of the numerous timeouts
    metadata.overall_safety_properties.num_successful_views = 15;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

/// stress test for libp2p

#[tokio::test(flavor = "multi_thread")]
#[instrument]
#[ignore]
async fn test_stress_libp2p_network() {
    hotshot::helpers::initialize_logging();

    let metadata: TestDescription<TestTypes, Libp2pImpl, TestVersions> =
        TestDescription::default_stress();
    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}
