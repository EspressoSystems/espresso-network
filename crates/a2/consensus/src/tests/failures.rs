use std::{collections::BTreeSet, sync::Arc, time::Duration};

use hotshot_types::data::ViewNumber;
use versions::{TIMEOUT_EPOCH_VERSION, Upgrade};

use crate::{
    message::{ConsensusMessage, MessageType},
    tests::common::runner::TestRunner,
};

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

/// 10 nodes, f=3 down, with epochs, version 0.7.
///
/// The three nodes that are down lead one view in ten each, so views
/// `v % 10 ∈ {7, 8, 9}` time out. Those failures make the block height lag
/// the view, which puts the timeouts at views 27, 28 and 29 across the
/// epoch 2 to 3 boundary: nodes give up those views under the epoch each of
/// them is in, which is not the same epoch for all of them until the boundary
/// certificate has spread. Their votes are tallied per view and epoch, so the
/// two sides count separately, and with seven live nodes against a threshold
/// of seven there is no slack for a tally that stays split. What resolves it
/// is the re-signed vote on the next timer re-arm.
#[tokio::test(flavor = "multi_thread")]
async fn ten_nodes_f_down_with_epochs_bound() {
    TestRunner::builder()
        .num_nodes(10)
        .target_decisions(30)
        .max_runtime(Duration::from_secs(500))
        .view_timeout(Duration::from_secs(5))
        .epoch_height(10)
        .down_nodes(BTreeSet::from([7, 8, 9]))
        .upgrade(Upgrade::trivial(TIMEOUT_EPOCH_VERSION))
        .build()
        .run()
        .await
        .unwrap();
}

/// A view that fails after its proposal reached the next leader fails only itself.
///
/// The leader of view 5 is node 0, and its proposal reaches node 1 alone, the
/// leader of view 6. No certificate can form, so view 5 times out. Node 1 has
/// already built a block on that proposal and dispersed its VID shares. After
/// the timeout it proposes a block on its lock instead. Transactions keep
/// arriving in between. Peers keep one share per leader and view, so the block
/// on the lock is votable only if it carries the payload already dispersed.
#[tokio::test(flavor = "multi_thread")]
async fn proposal_withheld_from_all_but_next_leader() {
    const WITHHELD: u64 = 5;
    const NEXT_LEADER: usize = 1;

    TestRunner::builder()
        .num_nodes(5)
        .target_decisions(15)
        .view_timeout(Duration::from_secs(2))
        .transactions(2000)
        .expected_failed_views(BTreeSet::from([ViewNumber::new(WITHHELD)]))
        .drop_inbound(Arc::new(|node, message| {
            node != NEXT_LEADER
                && matches!(
                    &message.message_type,
                    MessageType::Consensus(ConsensusMessage::Proposal(p))
                        if *p.proposal.data.view_number == WITHHELD
                )
        }))
        .build()
        .run()
        .await
        .unwrap();
}
