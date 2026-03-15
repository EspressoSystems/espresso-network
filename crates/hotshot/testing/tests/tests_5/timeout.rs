// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#[cfg(test)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
#[ignore]
async fn test_timeout_libp2p() {
    use hotshot_example_types::node_types::{Libp2pImpl, TestTypes};
    use hotshot_testing::{
        block_builder::SimpleBuilderImplementation,
        overall_safety_task::OverallSafetyPropertiesDescription,
        spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
        test_builder::{TestDescription, TimingData},
    };

    let timing_data = TimingData {
        next_view_timeout: 2000,
        ..Default::default()
    };

    let mut metadata: TestDescription<TestTypes, Libp2pImpl> = TestDescription {
        ..Default::default()
    }
    .set_num_nodes(10, 10);

    let dead_nodes = vec![ChangeNode {
        idx: 9,
        updown: NodeAction::Down,
    }];

    metadata.timing_data = timing_data;

    metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
        num_successful_views: 25,
        ..Default::default()
    };

    metadata.spinning_properties = SpinningTaskDescription {
        node_changes: vec![(5, dead_nodes)],
    };

    metadata
        .gen_launcher()
        .launch()
        .run_test::<SimpleBuilderImplementation>()
        .await;
}
