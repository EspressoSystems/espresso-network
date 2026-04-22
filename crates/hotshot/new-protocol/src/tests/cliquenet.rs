use crate::tests::common::runner::TestRunner;

#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_same_chain() {
    TestRunner::default().run().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn three_nodes_decide() {
    TestRunner {
        num_nodes: 3,
        target_decisions: 50,
        ..Default::default()
    }
    .run()
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn epoch_changes() {
    TestRunner {
        epoch_height: 10,
        target_decisions: 50,
        ..Default::default()
    }
    .run()
    .await
    .unwrap();
}
