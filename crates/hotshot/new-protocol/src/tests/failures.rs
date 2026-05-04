use std::{collections::BTreeSet, time::Duration};

use crate::tests::common::runner::TestRunner;

/// 10 nodes, 1 down.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_one_down() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .view_timeout(Duration::from_secs(5))
        .down_nodes(BTreeSet::from([9]))
        .build()
        .run()
        .await
        .unwrap();
}

/// 10 nodes, 2 down.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_two_down() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .view_timeout(Duration::from_secs(5))
        .down_nodes(BTreeSet::from([8, 9]))
        .build()
        .run()
        .await
        .unwrap();
}

/// 10 nodes, f=3 down.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_f_down() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .view_timeout(Duration::from_secs(5))
        .down_nodes(BTreeSet::from([7, 8, 9]))
        .build()
        .run()
        .await
        .unwrap();
}

/// 10 nodes, 1 down, with epochs.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_one_down_with_epochs() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .view_timeout(Duration::from_secs(5))
        .epoch_height(10)
        .down_nodes(BTreeSet::from([9]))
        .build()
        .run()
        .await
        .unwrap();
}

/// 10 nodes, f=3 down, with epochs.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_f_down_with_epochs() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .view_timeout(Duration::from_secs(5))
        .epoch_height(10)
        .down_nodes(BTreeSet::from([7, 8, 9]))
        .build()
        .run()
        .await
        .unwrap();
}
