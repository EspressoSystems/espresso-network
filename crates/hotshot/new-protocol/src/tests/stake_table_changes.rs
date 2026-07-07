//! Validator set changes at epoch boundaries, driven by a per-epoch stake
//! table schedule shared by all nodes.
//!
//! Epochs 1 and 2 are fixed at genesis (`set_first_epoch`), so epoch 3 is
//! the first epoch whose committee can differ. Its stake table is registered
//! network-wide when the epoch-1 root decides.
//!
//! These multi-node tests cover the boundary end to end (catchup, cliquenet
//! peer changes, thresholds, leader rotation). Single-node boundary-crossing
//! scenarios would need a per-epoch-aware `TestData` (cert signers, VID
//! recipients, leader keys) and are not covered here.

use std::{collections::BTreeMap, time::Duration};

use crate::tests::common::{
    runner::{NodeAction, NodeChange, TestRunner},
    utils::StakeTableSchedule,
};

/// 6 nodes, epoch_height=15; nodes 0-4 form epochs 1-2, node 5 joins the
/// committee at epoch 3 (blocks 31-45).
///
/// Node 5 is offline until view 17 (epoch 3's committee is registered when
/// block 10 decides), then starts from genesis and must catch up, be added
/// to the other nodes' cliquenet peers via its scheduled connect info, and
/// participate in epoch 3 — it leads views 35 and 41, which time out and
/// fail the test if it did not join.
#[tokio::test(flavor = "multi_thread")]
async fn validator_joins_at_epoch_boundary() {
    TestRunner::builder()
        .num_nodes(6)
        .target_decisions(45)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(15)
        .stake_table_schedule(StakeTableSchedule {
            initial: vec![0, 1, 2, 3, 4],
            changes: vec![(3, vec![0, 1, 2, 3, 4, 5])],
        })
        .node_changes(vec![(
            17,
            vec![NodeChange {
                idx: 5,
                action: NodeAction::Start,
            }],
        )])
        .build()
        .run()
        .await
        .unwrap();
}

/// 6 nodes, epoch_height=10; all form epochs 1-2, node 5 is removed from
/// the committee at epoch 3 (blocks 21-30).
///
/// Node 5 keeps running: it leads views 5, 11, and 17 while it is a member
/// (failures there fail verification). From epoch 3 on it must not vote or
/// propose, but it still follows the chain — the other nodes retain a
/// leaving validator as a network peer for one extra epoch, and broadcast
/// cert2s let it decide without VID shares. At the epoch-4 boundary the
/// peers drop it and it stops at block 30. The 5-node committee must keep
/// deciding through epoch 4.
#[tokio::test(flavor = "multi_thread")]
async fn validator_leaves_at_epoch_boundary() {
    let mut runner = TestRunner::builder()
        .num_nodes(6)
        .target_decisions(35)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(10)
        .stake_table_schedule(StakeTableSchedule {
            initial: vec![0, 1, 2, 3, 4, 5],
            changes: vec![(3, vec![0, 1, 2, 3, 4])],
        })
        .node_decision_targets(BTreeMap::from([(5, 12)]))
        .build();
    runner.run().await.unwrap();

    // All views 1..=35 decided, so view == block height; node 5's membership
    // ends with epoch 2 at view 20.
    for (view, action) in runner.node_storages()[5].action_log().await {
        assert!(
            *view <= 20,
            "node 5 recorded {action:?} for view {view} after its membership ended"
        );
    }

    let (anchor, _) = runner.node_storages()[5]
        .anchor_leaf()
        .await
        .expect("removed node should have decided while it was a member");
    assert!(
        anchor.height() >= 12,
        "node 5 stalled at block {} before its membership ended",
        anchor.height()
    );
    assert!(
        anchor.height() <= 30,
        "node 5 decided block {} after its one-epoch peer retention ended",
        anchor.height()
    );
}
