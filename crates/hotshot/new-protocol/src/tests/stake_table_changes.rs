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

#[cfg(target_os = "linux")]
use std::net::Ipv4Addr;
use std::{collections::BTreeMap, time::Duration};

#[cfg(target_os = "linux")]
use hotshot_types::addr::NetAddr;

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
            ..Default::default()
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

/// 10 nodes, epoch_height=15; the committee narrows to nodes 0-4 at epoch 3
/// (blocks 31-45) and is replaced wholesale by the disjoint set 5-9 at
/// epoch 4 (blocks 46-60) — a 100% turnover of the active committee.
///
/// The incoming cohort followed epoch 3 via broadcast cert2s without VID
/// shares. The handoff works because the boundary leaf is final (Cert1 +
/// Cert2): the coordinator seeds a commitment-only state for it, so the
/// first epoch-4 leader and voters need neither the epoch-3 tail's payloads
/// nor its replayed state.
///
/// Reaching block 55 proves nodes 5-9 drove epoch 4: nodes 0-4 hold no
/// epoch-4 stake, so no quorum forms without the incoming cohort. The
/// 55-decision target also binds the outgoing nodes 0-4, which can only
/// meet it as retained followers — peers keep a leaving validator for one
/// extra epoch, so 0-4 follow epoch 4 by broadcast until the cliff at
/// block 60. Shortening that retention window breaks this test.
#[tokio::test(flavor = "multi_thread")]
async fn validator_set_replaced_at_epoch_boundary() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(55)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(15)
        .stake_table_schedule(StakeTableSchedule {
            initial: (0..10).collect(),
            changes: vec![(3, vec![0, 1, 2, 3, 4]), (4, vec![5, 6, 7, 8, 9])],
            ..Default::default()
        })
        .build()
        .run()
        .await
        .unwrap();
}

/// The replacement schedule of [`validator_set_replaced_at_epoch_boundary`],
/// but node 5 — in the incoming epoch-4 cohort — restarts from blank storage
/// mid-epoch-3, losing the followed chain and the seeded boundary states.
/// It must rebuild through catchup as a non-member and then lead and vote
/// in epoch 4. Since it is a non-member when it crashes, the restart must
/// be invisible to the chain: every view is required to decide.
#[tokio::test(flavor = "multi_thread")]
async fn incoming_validator_restarts_before_replacement_boundary() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(55)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(15)
        .stake_table_schedule(StakeTableSchedule {
            initial: (0..10).collect(),
            changes: vec![(3, vec![0, 1, 2, 3, 4]), (4, vec![5, 6, 7, 8, 9])],
            ..Default::default()
        })
        .node_changes(vec![(
            35,
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

/// 6 nodes, epoch_height=12; node 5 is a member of every epoch, but the
/// epoch-3 stake table (blocks 25-36) registers a new p2p address for it.
///
/// Node 5 lives on its own loopback IPs — 127.0.0.2 initially, rotating to
/// 127.0.0.3 — while its peers stay on 127.0.0.1 (hence Linux-only: macOS
/// does not alias 127.0.0.0/8). Cliquenet validates inbound connections by
/// source IP, and loopback dials always originate from 127.0.0.1, so node
/// 5's own dials are rejected by its peers and it is reachable only when
/// peers dial the address registered for it. If the rotated address did
/// not propagate, node 5 would stay partitioned after its restart and the
/// test fails.
///
/// Peers adopt the new address when they enter epoch 2: `apply_epoch`
/// eagerly merges the next epoch's connect info, so crossing into epoch 2
/// at block 13 re-points node 5's peers at the epoch-3 address. Node 5
/// restarts bound to the new address when view 11 decides (during view
/// ~12), i.e. it is already listening there when its peers re-point.
///
/// The epoch boundary must not be a view node 5 leads: peers re-point the
/// moment they validate the first epoch-2 proposal, and node 5 has not
/// rebound yet if it is that proposal's leader (its restart is gated on
/// the boundary view deciding). Cutting off the active leader mid-view
/// splits its certificates across the committee and, with the zero-slack
/// 5-of-6 quorum, can wedge the network for good. With epoch_height 12
/// the boundary view 13 is led by node 1 while node 5 (views 5, 11, 17,
/// ...) is idle through the handoff.
///
/// Node 5 leads views 17, 23, 29 and 35 after the rotation, every view is
/// required to decide, and its post-restart decisions (target 20) must
/// match the other nodes' chain. The 10s view timeout keeps a loaded CI
/// runner from timing out a view (5-of-6 leaves no vote slack, so a
/// single slow node fails the view).
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn validator_rotates_address_at_epoch_boundary() {
    let port = test_utils::reserve_tcp_port().expect("OS should have ephemeral ports available");
    let new_addr = NetAddr::Inet(Ipv4Addr::new(127, 0, 0, 3).into(), port);
    TestRunner::builder()
        .num_nodes(6)
        .target_decisions(35)
        .max_runtime(Duration::from_secs(500))
        .epoch_height(12)
        .view_timeout(Duration::from_secs(10))
        .node_ips(BTreeMap::from([(5, Ipv4Addr::new(127, 0, 0, 2).into())]))
        .stake_table_schedule(StakeTableSchedule {
            initial: vec![0, 1, 2, 3, 4, 5],
            changes: vec![(3, vec![0, 1, 2, 3, 4, 5])],
            addr_overrides: vec![(3, 5, new_addr.clone())],
        })
        .node_changes(vec![(
            11,
            vec![NodeChange {
                idx: 5,
                action: NodeAction::RestartAt(new_addr),
            }],
        )])
        .node_decision_targets(BTreeMap::from([(5, 20)]))
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
            ..Default::default()
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
