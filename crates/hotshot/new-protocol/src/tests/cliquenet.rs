use std::time::Duration;

use crate::tests::common::{network::CliquenetTestNetwork, runner::TestRunner};

#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_same_chain_over_cliquenet() {
    TestRunner::default()
        .run::<CliquenetTestNetwork>()
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn three_nodes_decide_over_cliquenet() {
    TestRunner {
        num_nodes: 3,
        target_decisions: 50,
        max_runtime: Duration::from_secs(60),
        ..Default::default()
    }
    .run::<CliquenetTestNetwork>()
    .await
    .unwrap();
}
