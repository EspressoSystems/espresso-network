// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::{Duration, Instant};

use hotshot_example_types::node_types::{Libp2pImpl, TestTypes, TestVersions};
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    test_builder::{TestDescription, TimingData},
};
use hotshot_types::traits::network::{
    AsynchronousNetwork, ChaosNetwork, PartiallySynchronousNetwork, SynchronousNetwork,
};
use tracing::instrument;

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn libp2p_network_sync() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, Libp2pImpl, TestVersions> = TestDescription {
        overall_safety_properties: OverallSafetyPropertiesDescription {
            check_leaf: true,
            ..Default::default()
        },
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::new(360, 0),
            },
        ),
        unreliable_network: Some(Box::new(SynchronousNetwork {
            delay_high_ms: 30,
            delay_low_ms: 4,
        })),
        ..TestDescription::default_multiple_rounds()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_memory_network_sync() {
    use std::time::Duration;

    use hotshot_example_types::node_types::{MemoryImpl, TestTypes};
    use hotshot_testing::{
        completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
        test_builder::TestDescription,
    };

    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, MemoryImpl, TestVersions> = TestDescription {
        // allow more time to pass in CI
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(240),
            },
        ),
        unreliable_network: Some(Box::new(SynchronousNetwork {
            delay_high_ms: 30,
            delay_low_ms: 4,
        })),
        ..TestDescription::default()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
#[instrument]
async fn libp2p_network_async() {
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
            next_view_timeout: 25000,
            ..TestDescription::<TestTypes, Libp2pImpl, TestVersions>::default_multiple_rounds()
                .timing_data
        },
        unreliable_network: Some(Box::new(AsynchronousNetwork {
            keep_numerator: 9,
            keep_denominator: 10,
            delay_low_ms: 4,
            delay_high_ms: 30,
        })),
        ..TestDescription::default_multiple_rounds()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

#[cfg(test)]
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn test_memory_network_async() {
    use std::time::Duration;

    use hotshot_example_types::node_types::{MemoryImpl, TestTypes};
    use hotshot_testing::{
        completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
        test_builder::TestDescription,
    };

    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, MemoryImpl, TestVersions> = TestDescription {
        overall_safety_properties: OverallSafetyPropertiesDescription {
            check_leaf: true,
            ..Default::default()
        },
        // allow more time to pass in CI
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(240),
            },
        ),
        timing_data: TimingData {
            next_view_timeout: 1000,
            ..TestDescription::<TestTypes, MemoryImpl, TestVersions>::default_multiple_rounds()
                .timing_data
        },
        unreliable_network: Some(Box::new(AsynchronousNetwork {
            keep_numerator: 95,
            keep_denominator: 100,
            delay_low_ms: 4,
            delay_high_ms: 30,
        })),
        ..TestDescription::default()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_memory_network_partially_sync() {
    use std::time::Duration;

    use hotshot_example_types::node_types::{MemoryImpl, TestTypes};
    use hotshot_testing::{
        completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
        test_builder::TestDescription,
    };

    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, MemoryImpl, TestVersions> = TestDescription {
        overall_safety_properties: OverallSafetyPropertiesDescription {
            ..Default::default()
        },
        // allow more time to pass in CI
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(240),
            },
        ),
        timing_data: TimingData {
            next_view_timeout: 25000,
            ..Default::default()
        },
        unreliable_network: Some(Box::new(PartiallySynchronousNetwork {
            asynchronous: AsynchronousNetwork {
                keep_numerator: 8,
                keep_denominator: 10,
                delay_low_ms: 4,
                delay_high_ms: 30,
            },
            synchronous: SynchronousNetwork {
                delay_high_ms: 30,
                delay_low_ms: 4,
            },
            gst: std::time::Duration::from_millis(1000),
            start: Instant::now(),
        })),
        ..TestDescription::default()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn libp2p_network_partially_sync() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, Libp2pImpl, TestVersions> = TestDescription {
        overall_safety_properties: OverallSafetyPropertiesDescription {
            ..Default::default()
        },
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::new(240, 0),
            },
        ),
        unreliable_network: Some(Box::new(PartiallySynchronousNetwork {
            asynchronous: AsynchronousNetwork {
                keep_numerator: 8,
                keep_denominator: 10,
                delay_low_ms: 4,
                delay_high_ms: 30,
            },
            synchronous: SynchronousNetwork {
                delay_high_ms: 30,
                delay_low_ms: 4,
            },
            gst: std::time::Duration::from_millis(1000),
            start: Instant::now(),
        })),
        ..TestDescription::default_multiple_rounds()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

#[cfg(test)]
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn test_memory_network_chaos() {
    use std::time::Duration;

    use hotshot_example_types::node_types::{MemoryImpl, TestTypes};
    use hotshot_testing::{
        completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
        test_builder::TestDescription,
    };

    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, MemoryImpl, TestVersions> = TestDescription {
        // allow more time to pass in CI
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(240),
            },
        ),
        unreliable_network: Some(Box::new(ChaosNetwork {
            keep_numerator: 8,
            keep_denominator: 10,
            delay_low_ms: 4,
            delay_high_ms: 30,
            repeat_low: 1,
            repeat_high: 5,
        })),
        ..TestDescription::default()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
#[instrument]
async fn libp2p_network_chaos() {
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
        unreliable_network: Some(Box::new(ChaosNetwork {
            keep_numerator: 8,
            keep_denominator: 10,
            delay_low_ms: 4,
            delay_high_ms: 30,
            repeat_low: 1,
            repeat_high: 5,
        })),
        ..TestDescription::default_multiple_rounds()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}
