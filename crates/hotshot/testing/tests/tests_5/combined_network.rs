// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{CombinedImpl, TestTypes, TestVersions};
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::{TestDescription, TimingData},
};
use rand::Rng;
use tracing::instrument;

/// A run with both the CDN and libp2p functioning properly
#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn test_combined_network() {
    use hotshot_testing::block_builder::SimpleBuilderImplementation;

    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, CombinedImpl, TestVersions> = TestDescription {
        timing_data: TimingData {
            next_view_timeout: 10_000,

            ..Default::default()
        },
        overall_safety_properties: OverallSafetyPropertiesDescription {
            num_successful_views: 25,
            ..Default::default()
        },
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(120),
            },
        ),
        ..TestDescription::default_multiple_rounds()
    };

    metadata.test_config.epoch_height = 0;
    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

// A run where the CDN crashes part-way through

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn test_combined_network_cdn_crash() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, CombinedImpl, TestVersions> = TestDescription {
        timing_data: TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        },
        overall_safety_properties: OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            ..Default::default()
        },
        // allow more time to pass in CI
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        ),
        ..TestDescription::default_multiple_rounds()
    };

    let mut all_nodes = vec![];
    for node in 0..metadata.test_config.num_nodes_with_stake.into() {
        all_nodes.push(ChangeNode {
            idx: node,
            updown: NodeAction::NetworkDown,
        });
    }

    metadata.test_config.epoch_height = 0;
    metadata.spinning_properties = SpinningTaskDescription {
        node_changes: vec![(5, all_nodes)],
    };

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

// A run where the CDN crashes partway through
// and then comes back up

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn test_combined_network_reup() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, CombinedImpl, TestVersions> = TestDescription {
        timing_data: TimingData {
            next_view_timeout: 10_000,

            ..Default::default()
        },
        overall_safety_properties: OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            ..Default::default()
        },
        // allow more time to pass in CI
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        ),
        ..TestDescription::default_multiple_rounds()
    };

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

    metadata.test_config.epoch_height = 0;
    metadata.spinning_properties = SpinningTaskDescription {
        node_changes: vec![(13, all_up), (5, all_down)],
    };

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

// A run where half of the nodes disconnect from the CDN

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn test_combined_network_half_dc() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, CombinedImpl, TestVersions> = TestDescription {
        timing_data: TimingData {
            next_view_timeout: 10_000,

            ..Default::default()
        },
        overall_safety_properties: OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            ..Default::default()
        },
        // allow more time to pass in CI
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        ),
        ..TestDescription::default_multiple_rounds()
    };

    let mut half = vec![];
    for node in 0..usize::from(metadata.test_config.num_nodes_with_stake) / 2 {
        half.push(ChangeNode {
            idx: node,
            updown: NodeAction::NetworkDown,
        });
    }

    metadata.test_config.epoch_height = 0;
    metadata.spinning_properties = SpinningTaskDescription {
        node_changes: vec![(5, half)],
    };

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

fn generate_random_node_changes(
    total_nodes: usize,
    total_num_rounds: usize,
) -> Vec<(u64, Vec<ChangeNode>)> {
    let mut rng = rand::thread_rng();
    let mut node_changes = vec![];

    for _ in 0..total_nodes * 2 {
        let updown = if rng.gen::<bool>() {
            NodeAction::NetworkUp
        } else {
            NodeAction::NetworkDown
        };

        let node_change = ChangeNode {
            idx: rng.gen_range(0..total_nodes),
            updown,
        };

        let round = rng.gen_range(1..total_num_rounds) as u64;

        node_changes.push((round, vec![node_change]));
    }

    node_changes
}

// A fuzz test, where random network events take place on all nodes

#[tokio::test(flavor = "multi_thread")]
#[instrument]
#[ignore]
async fn test_stress_combined_network_fuzzy() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, CombinedImpl, TestVersions> = TestDescription {
        timing_data: TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        },

        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(120),
            },
        ),
        ..TestDescription::default_stress()
    }
    .set_num_nodes(20, 20);

    metadata.test_config.epoch_height = 0;
    metadata.spinning_properties = SpinningTaskDescription {
        node_changes: generate_random_node_changes(
            metadata.test_config.num_nodes_with_stake.into(),
            metadata.overall_safety_properties.num_successful_views * 2,
        ),
    };

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}
