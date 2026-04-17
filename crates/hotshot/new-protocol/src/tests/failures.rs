use std::{collections::BTreeSet, time::Duration};

use crate::tests::common::{
    network::{CliquenetTestNetwork, MemoryTestNetwork},
    runner::TestRunner,
};

// ---------------------------------------------------------------------------
// MemoryNetwork
// ---------------------------------------------------------------------------

/// 10 nodes, 1 down over MemoryNetwork.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_one_down_memory() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        down_nodes: BTreeSet::from([9]),
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

/// 10 nodes, 2 down over MemoryNetwork.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_two_down_memory() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        down_nodes: BTreeSet::from([8, 9]),
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

/// 10 nodes, f=3 down over MemoryNetwork.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_f_down_memory() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        down_nodes: BTreeSet::from([7, 8, 9]),
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

/// 10 nodes, 1 down, with epochs over MemoryNetwork.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_one_down_with_epochs_memory() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        epoch_height: 10,
        down_nodes: BTreeSet::from([9]),
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

/// 10 nodes, f=3 down, with epochs over MemoryNetwork.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_f_down_with_epochs_memory() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        epoch_height: 10,
        down_nodes: BTreeSet::from([7, 8, 9]),
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

/// 20 nodes, f=6 down over MemoryNetwork.
#[tokio::test(flavor = "multi_thread")]
async fn twenty_nodes_f_down_memory() {
    TestRunner {
        num_nodes: 20,
        target_decisions: 30,
        view_timeout: Duration::from_secs(5),
        down_nodes: BTreeSet::from([14, 15, 16, 17, 18, 19]),
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

// TODO: The Shutdown action causes views after the shutdown to fail even
// when the leader is alive and epochs are disabled.  This appears to be a
// pre-existing Coordinator issue where aborting a node's task disrupts
// consensus for subsequent views.
//
// #[tokio::test(flavor = "multi_thread")]
// async fn ten_nodes_one_shutdown_memory() { ... }

// ---------------------------------------------------------------------------
// Cliquenet
// ---------------------------------------------------------------------------

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
    .run::<CliquenetTestNetwork>()
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
    .run::<CliquenetTestNetwork>()
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
    .run::<CliquenetTestNetwork>()
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
    .run::<CliquenetTestNetwork>()
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
    .run::<CliquenetTestNetwork>()
    .await
    .unwrap();
}
