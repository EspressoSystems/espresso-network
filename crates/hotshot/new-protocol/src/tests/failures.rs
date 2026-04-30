use std::{collections::BTreeSet, time::Duration};

use crate::tests::common::runner::TestRunner;

/// 10 nodes, 1 down over Cliquenet.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_one_down_cliquenet() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        view_timeout: Duration::from_secs(5),
        down_nodes: BTreeSet::from([9]),
        ..Default::default()
    }
    .run()
    .await
    .unwrap();
}

/// 10 nodes, 2 down over Cliquenet.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_two_down_cliquenet() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        view_timeout: Duration::from_secs(5),
        down_nodes: BTreeSet::from([8, 9]),
        ..Default::default()
    }
    .run()
    .await
    .unwrap();
}

/// 10 nodes, f=3 down over Cliquenet.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_f_down_cliquenet() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        view_timeout: Duration::from_secs(5),
        down_nodes: BTreeSet::from([7, 8, 9]),
        ..Default::default()
    }
    .run()
    .await
    .unwrap();
}

/// 10 nodes, 1 down, with epochs over Cliquenet.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_one_down_with_epochs_cliquenet() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        view_timeout: Duration::from_secs(5),
        epoch_height: 10,
        down_nodes: BTreeSet::from([9]),
        ..Default::default()
    }
    .run()
    .await
    .unwrap();
}

/// 10 nodes, f=3 down, with epochs over Cliquenet.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_f_down_with_epochs_cliquenet() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        view_timeout: Duration::from_secs(5),
        epoch_height: 10,
        down_nodes: BTreeSet::from([7, 8, 9]),
        ..Default::default()
    }
    .run()
    .await
    .unwrap();
}
