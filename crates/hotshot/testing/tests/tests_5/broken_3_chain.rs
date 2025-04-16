#![cfg(feature = "broken_3_chain_fixed")]
use std::time::Duration;

use hotshot_example_types::node_types::{PushCdnImpl, TestTypes};
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    spinning_task::{ChangeNode, SpinningTaskDescription, UpDown},
    test_builder::{TestDescription, TimingData},
};
use tracing::instrument;

/// Broken 3-chain test

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn broken_3_chain() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, PushCdnImpl> = TestDescription {
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
    };

    let dead_nodes = vec![
        ChangeNode {
            idx: 3,
            updown: UpDown::NetworkDown,
        },
        ChangeNode {
            idx: 6,
            updown: UpDown::NetworkDown,
        },
        ChangeNode {
            idx: 9,
            updown: UpDown::NetworkDown,
        },
    ];

    metadata.spinning_properties = SpinningTaskDescription {
        node_changes: vec![(3, dead_nodes)],
    };
    metadata.num_nodes_with_stake = 10;
    metadata.da_staked_committee_size = 10;
    metadata.start_nodes = 10;
    // Check whether we see at least 10 decides
    metadata.overall_safety_properties.num_successful_views = 10;
    metadata.overall_safety_properties.expected_view_failures = vec![2, 3, 5, 6, 8, 9];

    metadata
        .gen_launcher(0)
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}
