use std::{
    collections::{BTreeMap, HashSet},
    time::Duration,
};

use hotshot::types::BLSPubKey;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    traits::signature_key::SignatureKey,
};

use crate::tests::common::{
    runner::{NodeAction, NodeChange, TestRunner},
    utils::{build_timeout_cert, mock_membership_with_num_nodes},
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
/// Each node resumes from its persisted decided anchor (the newest decided
/// leaf at crash time) instead of re-growing the chain from genesis, and
/// the persisted action records keep every node out of the views it acted
/// in before the crash: views resume past the crash and no node records a
/// Vote or Propose action twice for any view.
/// The crash/recovery window is nondeterministic, so those views are
/// excluded from the usual per-view verification.
#[tokio::test(flavor = "multi_thread")]
async fn restart_all_nodes_with_storage() {
    let num_nodes = 5;
    let crash_view = 5;
    let mut runner = TestRunner::builder()
        .num_nodes(num_nodes)
        .target_decisions(35)
        .epoch_height(10)
        .persistent_storage(true)
        .tolerated_failed_views(views(1..=30))
        .node_changes(vec![(
            crash_view,
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

        let (anchor, _) = storage
            .anchor_leaf()
            .await
            .unwrap_or_else(|| panic!("node {idx} has no persisted anchor"));
        assert!(
            *anchor.view_number() > crash_view,
            "node {idx} anchor stuck at view {} — it did not keep deciding after the restart",
            anchor.view_number()
        );
    }
}

/// All nodes crash at the epoch boundary and restart with storage: the quorum
/// must re-cast its phase-2 votes (without re-recording any action) or the
/// epoch transition never completes.
#[tokio::test(flavor = "multi_thread")]
async fn restart_all_nodes_at_epoch_boundary() {
    let num_nodes = 5;
    let crash_view = 10;
    let mut runner = TestRunner::builder()
        .num_nodes(num_nodes)
        .target_decisions(35)
        .epoch_height(10)
        .persistent_storage(true)
        .tolerated_failed_views(views(1..=30))
        .node_changes(vec![(
            crash_view,
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

        let (anchor, _) = storage
            .anchor_leaf()
            .await
            .unwrap_or_else(|| panic!("node {idx} has no persisted anchor"));
        assert!(
            *anchor.view_number() > crash_view,
            "node {idx} anchor stuck at view {} — it did not keep deciding past the epoch \
             boundary after the restart",
            anchor.view_number()
        );
    }
}

// ---------------------------------------------------------------------------
// View divergence between cohorts
// ---------------------------------------------------------------------------

/// Reproduces a devnet stall after a mass restart. 4 of 6 nodes (2/3 of
/// stake — one short of the 2f+1 = 5 timeout-cert threshold) sit on view 1,
/// while nodes 4 and 5 (1/3 of stake — one short of the f+1 = 3 one-honest
/// threshold) advanced to view 3 via timeout certificates the majority never
/// received, e.g. formed while it was down.
///
/// Without catchup this deadlocks permanently: the majority's view-1 timeout
/// votes are stale to the minority (dropped on receipt), so it only ever
/// re-forms a one-honest cert for view 1, while the minority cannot reach
/// any threshold at view 3 — no certificate forms on any single view. The
/// stale-vote catchup reply and the TC attached to timeout votes let the
/// majority adopt view 3; the pooled timeout votes then form a full timeout
/// certificate and views progress normally.
#[tokio::test(flavor = "multi_thread")]
async fn view_divergence_between_cohorts_recovers() {
    let num_nodes = 6;
    let epoch_height = 100;
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    let (membership, _storage, _client) =
        mock_membership_with_num_nodes(num_nodes, epoch_height, public_key);
    let epoch_membership = membership
        .membership_for_epoch(Some(EpochNumber::genesis()))
        .unwrap();
    let ahead: Vec<_> = [1, 2]
        .map(|view| {
            build_timeout_cert(
                ViewNumber::new(view),
                EpochNumber::genesis(),
                &epoch_membership,
                &public_key,
                &private_key,
            )
        })
        .into_iter()
        .collect();

    TestRunner::builder()
        .num_nodes(num_nodes)
        .target_decisions(10)
        .epoch_height(epoch_height)
        .initial_timeout_certs(BTreeMap::from([(4, ahead.clone()), (5, ahead)]))
        .tolerated_failed_views(views(1..=3))
        .build()
        .run()
        .await
        .unwrap();
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
