use std::collections::HashMap;

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
