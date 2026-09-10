//! Observer nodes: never staked, fed by upstream validators over
//! config-pinned cliquenet peers.

use std::collections::BTreeMap;

use crate::tests::common::runner::{NodeAction, NodeChange, TestRunner};

async fn assert_observer_followed(runner: &TestRunner, idx: usize, min_height: u64) {
    let storage = &runner.node_storages()[idx];
    let actions = storage.action_log().await;
    assert!(
        actions.is_empty(),
        "observer {idx} took consensus actions: {actions:?}"
    );
    let (anchor, _) = storage
        .anchor_leaf()
        .await
        .expect("observer should have decided");
    assert!(
        anchor.height() >= min_height,
        "observer {idx} stalled at block {}",
        anchor.height()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn observer_follows_chain_across_epochs() {
    let mut runner = TestRunner::builder()
        .num_nodes(5)
        .observer_nodes(BTreeMap::from([(4, vec![0, 1])]))
        .epoch_height(10)
        .target_decisions(35)
        .build();
    runner.run().await.unwrap();
    assert_observer_followed(&runner, 4, 35).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn observer_follows_single_upstream() {
    let mut runner = TestRunner::builder()
        .num_nodes(4)
        .observer_nodes(BTreeMap::from([(3, vec![0])]))
        .target_decisions(20)
        .build();
    runner.run().await.unwrap();
    assert_observer_followed(&runner, 3, 20).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn observer_restart_resumes_following() {
    let mut runner = TestRunner::builder()
        .num_nodes(5)
        .observer_nodes(BTreeMap::from([(4, vec![0, 1, 2])]))
        .persistent_storage(true)
        .target_decisions(30)
        .node_changes(vec![(
            12,
            vec![NodeChange {
                idx: 4,
                action: NodeAction::Restart,
            }],
        )])
        .build();
    runner.run().await.unwrap();
    assert_observer_followed(&runner, 4, 30).await;
}
