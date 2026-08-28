use std::collections::{BTreeMap, BTreeSet};

use hotshot_types::data::ViewNumber;

use crate::tests::common::runner::TestRunner;

#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_same_chain_over_cliquenet() {
    TestRunner::builder().build().run().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn three_nodes_decide_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(3)
        .target_decisions(50)
        .build()
        .run()
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn epoch_changes_over_cliquenet() {
    TestRunner::builder()
        .epoch_height(10)
        .target_decisions(50)
        .build()
        .run()
        .await
        .unwrap();
}

/// Nodes 1 and 4 cannot connect to each other; cliquenet does not relay.
/// When one of them leads the other never receives the proposal and cannot
/// vote, leaving 6 of 7 votes against the quorum threshold of 5
/// (`2n/3 + 1`) — one vote of slack, so every view is still required to
/// decide. The pair sits 3 apart in the round-robin leader order, so a
/// connected leader always follows a blocked one and re-propagates its
/// certificates.
#[tokio::test(flavor = "multi_thread")]
async fn blocked_pair_still_decides_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(7)
        .target_decisions(30)
        .epoch_height(10)
        .blocked_pairs(BTreeSet::from([(1, 4)]))
        .build()
        .run()
        .await
        .unwrap();
}

/// As in [`blocked_pair_still_decides_over_cliquenet`], but nodes 1, 4 and
/// 7 of 10 are mutually unreachable: a blocked leader still collects 8 of
/// 10 votes against the quorum threshold of 7 (`2n/3 + 1`) — one vote of
/// slack.
#[tokio::test(flavor = "multi_thread")]
async fn mutually_blocked_nodes_still_decide_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .epoch_height(10)
        .blocked_pairs(BTreeSet::from([(1, 4), (1, 7), (4, 7)]))
        .build()
        .run()
        .await
        .unwrap();
}

/// A node that misses one view's VID share broadcasts recovers by fetching
/// the payload, and the network certifies again.
///
/// Five of seven nodes are up, which is exactly the quorum threshold, so no
/// view can certify without every one of them. Node 1 receives view 2's
/// proposal, its own share fragments and every vote, and so takes full part
/// in that view — but never the other voters' share broadcasts, so it cannot
/// reconstruct the block. From then on it cannot vote on any proposal
/// parented at view 2, which is every later proposal, and the run only
/// completes if it recovers the payload from a peer.
///
/// The views around the deficit are tolerated rather than pinned: how many
/// time out before the fetch lands depends on scheduling. Completing at all
/// is the assertion — without the fetch this run never reaches its target.
#[tokio::test(flavor = "multi_thread")]
async fn share_starved_node_recovers_over_cliquenet() {
    TestRunner::builder()
        .num_nodes(7)
        .down_nodes(BTreeSet::from([5, 6]))
        .starved_of_shares(BTreeMap::from([(1, BTreeSet::from([ViewNumber::new(2)]))]))
        .tolerated_failed_views((2..=8).map(ViewNumber::new).collect())
        .target_decisions(12)
        .view_timeout(std::time::Duration::from_secs(2))
        .build()
        .run()
        .await
        .unwrap();
}
