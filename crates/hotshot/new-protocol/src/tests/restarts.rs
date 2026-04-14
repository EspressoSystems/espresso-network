use std::time::Duration;

use crate::tests::common::{
    network::MemoryTestNetwork,
    runner::{NodeAction, NodeChange, TestRunner},
};

// ---------------------------------------------------------------------------
// Restart from blank state (with epochs)
// ---------------------------------------------------------------------------

/// 10 nodes, 1 restarts from blank state at view 15, epoch_height=10.
///
/// Verifies that a single node can restart from genesis while the rest of
/// the network continues, and that it catches up and participates across
/// epoch boundaries.
#[tokio::test(flavor = "multi_thread")]
async fn restart_one_node_with_epochs() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        max_runtime: Duration::from_secs(120),
        epoch_height: 10,
        node_changes: vec![(
            15,
            vec![NodeChange {
                idx: 5,
                action: NodeAction::Restart,
            }],
        )],
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

/// 10 nodes, f=3 restart from blank state simultaneously at view 15,
/// epoch_height=10.
///
/// Verifies the network recovers when the maximum tolerable number of
/// nodes restart at once.
#[tokio::test(flavor = "multi_thread")]
async fn restart_f_nodes_with_epochs() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        max_runtime: Duration::from_secs(180),
        epoch_height: 10,
        node_changes: vec![(
            15,
            vec![
                NodeChange {
                    idx: 7,
                    action: NodeAction::Restart,
                },
                NodeChange {
                    idx: 8,
                    action: NodeAction::Restart,
                },
                NodeChange {
                    idx: 9,
                    action: NodeAction::Restart,
                },
            ],
        )],
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

/// 5 nodes, all restart from blank state simultaneously at view 15,
/// epoch_height=10.
///
/// Verifies the network can recover from a complete restart where every
/// node starts from genesis.
#[tokio::test(flavor = "multi_thread")]
async fn restart_all_nodes_with_epochs() {
    TestRunner {
        num_nodes: 5,
        target_decisions: 30,
        max_runtime: Duration::from_secs(300),
        epoch_height: 10,
        node_changes: vec![(
            15,
            (0..5)
                .map(|i| NodeChange {
                    idx: i,
                    action: NodeAction::Restart,
                })
                .collect(),
        )],
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

// ---------------------------------------------------------------------------
// Late start (with epochs)
// ---------------------------------------------------------------------------

/// 10 nodes, 1 starts late at view 20, epoch_height=10.
///
/// Node 9 is initially offline.  At view 20 (after 2 epoch transitions)
/// it joins the network from genesis and must catch up.
#[tokio::test(flavor = "multi_thread")]
async fn late_start_one_node_with_epochs() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        max_runtime: Duration::from_secs(120),
        epoch_height: 10,
        node_changes: vec![(
            20,
            vec![NodeChange {
                idx: 9,
                action: NodeAction::Start,
            }],
        )],
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}

/// 10 nodes, f=3 start late at view 20, epoch_height=10.
///
/// Nodes 7-9 are initially offline (the network runs with 7 nodes, the
/// minimum quorum for n=10).  At view 20 they all join simultaneously.
#[tokio::test(flavor = "multi_thread")]
async fn late_start_f_nodes_with_epochs() {
    TestRunner {
        num_nodes: 10,
        target_decisions: 30,
        max_runtime: Duration::from_secs(180),
        epoch_height: 10,
        node_changes: vec![(
            20,
            vec![
                NodeChange {
                    idx: 7,
                    action: NodeAction::Start,
                },
                NodeChange {
                    idx: 8,
                    action: NodeAction::Start,
                },
                NodeChange {
                    idx: 9,
                    action: NodeAction::Start,
                },
            ],
        )],
        ..Default::default()
    }
    .run::<MemoryTestNetwork>()
    .await
    .unwrap();
}
