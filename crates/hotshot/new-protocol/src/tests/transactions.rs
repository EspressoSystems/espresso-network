use std::collections::HashMap;

use hotshot_types::{data::ViewNumber, upgrade_config::UpgradeConfig};
use versions::{NEW_PROTOCOL_VERSION, TX_DIGEST_VERSION, Upgrade};

use crate::tests::common::runner::TestRunner;

/// Every submitted transaction is decided in at most one block.
#[tokio::test(flavor = "multi_thread")]
async fn submitted_transactions_are_decided_at_most_once() {
    let mut runner = TestRunner::builder()
        .num_nodes(5)
        .target_decisions(60)
        .transactions(300)
        .build();
    runner.run().await.unwrap();

    assert_decided_at_most_once(&runner);
}

/// Blocks before the upgrade identify transactions by Keccak and blocks after it by BLAKE3,
/// and a transaction pooled on one side of the switch is still deduped on the other.
#[tokio::test(flavor = "multi_thread")]
async fn submitted_transactions_are_decided_at_most_once_across_the_digest_upgrade() {
    let mut runner = TestRunner::builder()
        .num_nodes(5)
        .epoch_height(10)
        .target_decisions(80)
        .transactions(1000)
        .upgrade(Upgrade::new(NEW_PROTOCOL_VERSION, TX_DIGEST_VERSION))
        .upgrade_config(UpgradeConfig {
            start_proposing_view: 5,
            stop_proposing_view: 15,
            start_voting_view: 0,
            stop_voting_view: u64::MAX,
            start_proposing_time: 0,
            stop_proposing_time: u64::MAX,
            start_voting_time: 0,
            stop_voting_time: u64::MAX,
        })
        .build();
    runner.run().await.unwrap();

    let first_view = upgrade_view(&runner);
    let views = runner.decided_transactions().keys().collect::<Vec<_>>();
    assert!(
        views.iter().any(|view| **view < first_view)
            && views.iter().any(|view| **view >= first_view),
        "transactions must be decided on both sides of the upgrade at {first_view}, got {views:?}"
    );
    assert_decided_at_most_once(&runner);
}

fn upgrade_view(runner: &TestRunner) -> ViewNumber {
    runner.node_locks()[0]
        .as_ref()
        .expect("node ran")
        .upgrade_view()
        .expect("the upgrade was decided")
}

fn assert_decided_at_most_once(runner: &TestRunner) {
    let mut last_seen = HashMap::new();
    let mut repeats = Vec::new();
    for (view, commitments) in runner.decided_transactions() {
        for commitment in commitments {
            if let Some(earlier) = last_seen.insert(*commitment, *view) {
                repeats.push((earlier, *view));
            }
        }
    }

    assert!(
        !last_seen.is_empty(),
        "no submitted transaction was decided"
    );
    assert!(
        repeats.is_empty(),
        "{} of {} decided transactions were decided again, as (earlier view, later view): \
         {repeats:?}",
        repeats.len(),
        last_seen.len(),
    );
}
