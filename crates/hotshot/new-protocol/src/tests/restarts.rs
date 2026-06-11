use std::{collections::HashSet, time::Duration};

use crate::tests::common::{
    runner::{NodeAction, NodeChange, TestRunner},
    views,
};

// ---------------------------------------------------------------------------
// Restart from blank state (with epochs)
// ---------------------------------------------------------------------------

/// 10 nodes, 1 restarts from blank state at view 11, epoch_height=10.
///
/// Verifies that a single node can restart from genesis while the rest of
/// the network continues, and that it catches up and participates across
/// epoch boundaries.
#[tokio::test(flavor = "multi_thread")]
async fn restart_one_node_with_epochs() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .epoch_height(10)
        .node_changes(vec![(
            11,
            vec![NodeChange {
                idx: 5,
                action: NodeAction::Restart,
            }],
        )])
        .build()
        .run()
        .await
        .unwrap();
}

/// 10 nodes, f=3 restart from blank state simultaneously at view 15,
/// epoch_height=10.
///
/// Verifies the network recovers when the maximum tolerable number of
/// nodes restart at once.  Views where a restarting node is leader during
/// catchup are expected to fail.
#[tokio::test(flavor = "multi_thread")]
async fn restart_f_nodes_with_epochs() {
    // Nodes 7, 8, 9 restart at view 13.  Their first leader views after
    // restart are 17(7), 18(8), 19(9) — they should propose while catching
    // up.  Views 27(7), 28(8), 29(9) are in the epoch transition zone and
    // may also fail if the DRB hasn't arrived yet, but this tests that it does.
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .epoch_height(10)
        .node_changes(vec![(
            11,
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
        )])
        .build()
        .run()
        .await
        .unwrap();
}

/// 5 nodes all crash at view ~5 and restart with their persisted storage.
///
/// Decided state is not yet persisted (the chain re-grows from the genesis
/// anchor), but the persisted action records must keep every node out of
/// the views it acted in before the crash: views resume past the crash and
/// no node records a Vote or Propose action twice for any view.
/// The crash/recovery window is nondeterministic, so those views are
/// excluded from the usual per-view verification.
#[tokio::test(flavor = "multi_thread")]
async fn restart_all_nodes_with_storage() {
    let num_nodes = 5;
    let mut runner = TestRunner::builder()
        .num_nodes(num_nodes)
        .target_decisions(35)
        .epoch_height(10)
        .persistent_storage(true)
        .tolerated_failed_views(views(1..=30))
        .node_changes(vec![(
            5,
            (0..num_nodes)
                .map(|idx| NodeChange {
                    idx,
                    action: NodeAction::Restart,
                })
                .collect(),
        )])
        .build();
    runner.run().await.unwrap();

    for (idx, storage) in runner.node_storages().iter().enumerate() {
        let mut seen = HashSet::new();
        for (view, action) in storage.action_log().await {
            assert!(
                seen.insert((view, action)),
                "node {idx} recorded {action:?} twice for view {view} — it re-entered a view it \
                 had already acted in"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Late start (with epochs)
// ---------------------------------------------------------------------------

/// 10 nodes, 1 starts late at view 20, epoch_height=10.
///
/// Node 9 is initially offline.  At view 20 (after 2 epoch transitions)
/// it joins the network from genesis and must catch up.  Views where
/// node 9 was leader while offline are expected to fail.
#[tokio::test(flavor = "multi_thread")]
async fn late_start_one_node_with_epochs() {
    // Node 9 is leader for 9, 19, 29, 39.  It rejoins right after
    // it would have led for view 39 and should propose at view 49 in epoch 3
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(50)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(15)
        .expected_failed_views(views([9, 19, 29, 39]))
        .node_changes(vec![(
            39,
            vec![NodeChange {
                idx: 9,
                action: NodeAction::Start,
            }],
        )])
        .build()
        .run()
        .await
        .unwrap();
}

/// 10 nodes, f=3 start late at view 20, epoch_height=10.
///
/// Nodes 7-9 are initially offline (the network runs with 7 nodes, the
/// minimum quorum for n=10).  At view 23 they all join simultaneously.
#[tokio::test(flavor = "multi_thread")]
async fn late_start_f_nodes_with_epochs() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(50)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(15)
        .expected_failed_views(views([7, 8, 9, 17, 18, 19, 27, 28, 29, 37, 38, 39]))
        .node_changes(vec![
            (
                37,
                vec![
                    NodeChange {
                        idx: 7,
                        action: NodeAction::Start,
                    },
                    NodeChange {
                        idx: 8,
                        action: NodeAction::Start,
                    },
                ],
            ),
            (
                39,
                vec![NodeChange {
                    idx: 9,
                    action: NodeAction::Start,
                }],
            ),
        ])
        .build()
        .run()
        .await
        .unwrap();
}
