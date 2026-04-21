use std::time::Duration;

use crate::tests::common::{network::MemoryTestNetwork, runner::TestRunner};

#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_same_chain_over_memory_network() {
    TestRunner::default()
        .run::<MemoryTestNetwork>()
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn three_nodes_decide_over_memory_network() {
    TestRunner {
        num_nodes: 3,
        target_decisions: 50,
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_decide_over_memory_network() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        max_runtime: Duration::from_secs(120),
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn epoch_changes_over_memory_network() {
    TestRunner {
        epoch_height: 10,
        target_decisions: 50,
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}
