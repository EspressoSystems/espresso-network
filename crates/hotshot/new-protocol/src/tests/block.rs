use std::sync::Arc;

use committable::Committable;
use hotshot_example_types::{
    block_types::TestTransaction, node_types::TestTypes, state_types::TestInstanceState,
};
use hotshot_types::data::{EpochNumber, ViewNumber};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    helpers::test_upgrade_lock,
    message::{DedupManifest, TransactionMessage},
    tests::common::utils::mock_membership,
};

fn tx(n: u8) -> TestTransaction {
    TestTransaction::new(vec![n])
}

fn view(n: u64) -> ViewNumber {
    ViewNumber::new(n)
}

fn tx_msg(v: ViewNumber, transactions: Vec<TestTransaction>) -> TransactionMessage<TestTypes> {
    TransactionMessage {
        view: v,
        transactions,
    }
}

fn epoch() -> EpochNumber {
    EpochNumber::genesis()
}

fn small_config() -> BlockBuilderConfig {
    BlockBuilderConfig {
        max_retry_bytes: 1024,
        max_leader_bytes: 512,
        ttl: 5,
        dedup_window_size: 3,
    }
}

fn builder() -> BlockBuilder<TestTypes> {
    BlockBuilder::new(
        Arc::new(TestInstanceState::default()),
        mock_membership(),
        small_config(),
        test_upgrade_lock(),
    )
}

#[tokio::test]
async fn test_retry_buffer() {
    let mut b = builder();
    let t1 = tx(1);
    let t2 = tx(2);
    b.on_submit_transaction(t1.clone());
    b.on_submit_transaction(t2.clone());

    // t1 reconstructed and should be removed from retry
    b.on_block_reconstructed(vec![t1.commit()]);

    let forwarded = b.on_view_changed(view(1));
    assert_eq!(
        forwarded,
        vec![t2],
        "only unconfirmed tx should be forwarded"
    );

    // past ttl
    let forwarded = b.on_view_changed(view(6));
    assert!(forwarded.is_empty(), "tx past ttl should expire");
}

#[tokio::test]
async fn test_leader_buffer_drain() {
    let mut b = builder();
    b.on_transactions(tx_msg(view(1), vec![tx(1), tx(2)]));
    let (mut txns, manifest) = b.drain(view(1), epoch());
    txns.sort_by_key(|t| t.bytes().clone());
    assert_eq!(txns.len(), 2, "both transactions should be drained");
    assert_eq!(
        manifest.hashes.len(),
        2,
        "manifest should have one hash per tx"
    );

    // buffer is cleared after drain
    let (txns2, manifest2) = b.drain(view(2), epoch());
    assert!(txns2.is_empty(), "second drain should be empty");
    assert!(
        manifest2.hashes.is_empty(),
        "second drain manifest should have no hashes"
    );
}

/// A transaction of exactly `n` bytes, so a test can count the cap in
/// transactions rather than in bytes. `minimum_block_size` is the payload
/// length, and `small_config` caps the leader buffer at 512 bytes, so five
/// hundred-byte transactions fit and the sixth does not.
fn sized_tx(id: u8, n: usize) -> TestTransaction {
    let mut bytes = vec![0u8; n];
    bytes[0] = id;
    TestTransaction::new(bytes)
}

/// `max_leader_bytes` admits transactions up to the cap and refuses the one
/// that would cross it.
#[tokio::test]
async fn test_leader_buffer_respects_max_bytes() {
    let mut b = builder();
    let txs: Vec<_> = (0..6).map(|i| sized_tx(i, 100)).collect();
    b.on_transactions(tx_msg(view(1), txs.clone()));

    let (kept, _) = b.drain(view(1), epoch());
    assert_eq!(
        kept.len(),
        5,
        "500 bytes fit under the 512-byte cap, 600 do not"
    );
}

/// Dropping transactions from the leader buffer returns their bytes to the
/// budget, so the freed room is reusable. This is what the buffer's byte
/// counter is for: if the decrement were wrong in either direction, the second
/// batch would be admitted in the wrong number.
#[tokio::test]
async fn test_dedup_returns_leader_budget() {
    let mut b = builder();
    let first: Vec<_> = (0..5).map(|i| sized_tx(i, 100)).collect();
    b.on_transactions(tx_msg(view(1), first.clone()));

    // The leader for view 1 took three of them, so 300 bytes come back.
    let taken: Vec<_> = first.iter().take(3).map(Committable::commit).collect();
    b.on_dedup_manifest(DedupManifest {
        view: view(1),
        epoch: epoch(),
        hashes: taken,
    });

    // 200 bytes are still held, so three more hundred-byte transactions would
    // reach 500 and fit, while a fourth would cross the cap.
    let second: Vec<_> = (10..14).map(|i| sized_tx(i, 100)).collect();
    b.on_transactions(tx_msg(view(2), second));

    let (kept, _) = b.drain(view(2), epoch());
    assert_eq!(
        kept.len(),
        5,
        "two carried over plus three of the four new ones"
    );
}

/// `max_retry_bytes` refuses the submission that would cross it, and expiry
/// returns that budget.
#[tokio::test]
async fn test_retry_buffer_respects_max_bytes_and_frees_on_expiry() {
    let mut b = builder();
    for i in 0..11 {
        b.on_submit_transaction(sized_tx(i, 100));
    }
    assert_eq!(
        b.outstanding_transactions(),
        (10, 1000),
        "1000 bytes fit under the 1024-byte cap, 1100 do not"
    );

    // Every entry was submitted at view 0 with ttl 5, so all expire together.
    let forwarded = b.on_view_changed(view(6));
    assert!(
        forwarded.is_empty(),
        "expired transactions are not forwarded"
    );
    assert_eq!(
        b.outstanding_transactions(),
        (0, 0),
        "expiry returns the whole budget"
    );

    // The budget really is reusable, not merely reported as zero.
    for i in 20..31 {
        b.on_submit_transaction(sized_tx(i, 100));
    }
    assert_eq!(b.outstanding_transactions(), (10, 1000));
}

/// Two paths can emit `RequestBlockAndHeader` for the same view N+1 with
/// different parents:
///   1. `handle_proposal_with_vid_share(P_N)` — parent = P_N
///   2. `handle_timeout_certificate(cert.view = N)` — parent = proposals[locked_view]
///
/// Both must produce a block, because `maybe_propose` later picks the
/// header matching its current `parent_commitment`.  Keying the builder's
/// `calculations` map by view alone would silently drop one of them;
/// keying by `(view, parent_commitment)` lets both run.
#[tokio::test]
async fn test_request_block_same_view_different_parent_both_produce_output() {
    use std::collections::HashSet;

    use crate::{
        block::BlockAndHeaderRequest, helpers::proposal_commitment, tests::common::utils::TestData,
    };

    let mut b = builder();

    let test_data = TestData::new(3).await;
    let parent_a = test_data.views[0].proposal.data.clone();
    let parent_b = test_data.views[1].proposal.data.clone();
    let a_commit = proposal_commitment(&parent_a);
    let b_commit = proposal_commitment(&parent_b);
    assert_ne!(a_commit, b_commit);

    let target_view = ViewNumber::new(5);
    b.request_block(BlockAndHeaderRequest {
        view: target_view,
        epoch: EpochNumber::genesis(),
        parent_proposal: parent_a.clone(),
    });
    b.request_block(BlockAndHeaderRequest {
        view: target_view,
        epoch: EpochNumber::genesis(),
        parent_proposal: parent_b.clone(),
    });

    let mut got = HashSet::new();
    for _ in 0..2 {
        let Some(Ok(output)) = b.next().await else {
            panic!("expected an Ok block builder output");
        };
        assert_eq!(output.view, target_view);
        got.insert(proposal_commitment(&output.parent_proposal));
    }
    assert_eq!(got, HashSet::from([a_commit, b_commit]));
    assert!(b.next().await.is_none());
}

/// A duplicate request (same view AND same parent) is still deduped.
#[tokio::test]
async fn test_request_block_dedups_same_view_same_parent() {
    use crate::{block::BlockAndHeaderRequest, tests::common::utils::TestData};

    let mut b = builder();
    let test_data = TestData::new(2).await;
    let parent = test_data.views[0].proposal.data.clone();

    let target_view = ViewNumber::new(5);
    let req = || BlockAndHeaderRequest {
        view: target_view,
        epoch: EpochNumber::genesis(),
        parent_proposal: parent.clone(),
    };
    b.request_block(req());
    b.request_block(req());

    assert!(matches!(b.next().await, Some(Ok(_))));
    assert!(b.next().await.is_none());
}

#[tokio::test]
async fn test_dedup_window() {
    let mut b = BlockBuilder::new(
        Arc::new(TestInstanceState::default()),
        mock_membership(),
        BlockBuilderConfig {
            dedup_window_size: 2,
            ..small_config()
        },
        test_upgrade_lock(),
    );
    let t = tx(1);

    b.on_dedup_manifest(DedupManifest {
        view: view(1),
        epoch: epoch(),
        hashes: vec![t.commit()],
    });
    b.on_transactions(tx_msg(view(1), vec![t.clone()]));
    let (txns, _) = b.drain(view(1), epoch());
    assert!(
        txns.is_empty(),
        "tx should be blocked while in the dedup window"
    );

    // Advance past the threshold: current_view - view(1) > window_size(2)
    b.on_view_changed(view(4));
    b.on_dedup_manifest(DedupManifest {
        view: view(4),
        epoch: epoch(),
        hashes: vec![],
    });

    b.on_transactions(tx_msg(view(4), vec![t.clone()]));
    let (txns, _) = b.drain(view(4), epoch());
    assert_eq!(
        txns.len(),
        1,
        "tx should be accepted after dedup window eviction"
    );
}
