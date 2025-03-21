// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{PushCdnImpl, TestTypes, TestVersions};
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    test_builder::{TestDescription, TimingData},
};
use tracing::instrument;

/// Push CDN network test

#[tokio::test(flavor = "multi_thread")]
#[instrument]
async fn push_cdn_network() {
    hotshot::helpers::initialize_logging();

    let mut metadata: TestDescription<TestTypes, PushCdnImpl, TestVersions> = TestDescription {
        timing_data: TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        },
        overall_safety_properties: OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            ..Default::default()
        },
        completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(60),
            },
        ),
        ..TestDescription::default()
    };

    metadata.test_config.epoch_height = 0;

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}
