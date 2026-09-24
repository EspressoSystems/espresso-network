use std::{collections::HashSet, sync::Arc, time::Duration};

use committable::Committable;
use hotshot_example_types::{
    block_types::TestTransaction, node_types::TestTypes, state_types::TestInstanceState,
};
use hotshot_types::data::{EpochNumber, ViewNumber};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig, BlockError},
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
        max_mempool_bytes: 1024,
        max_leader_bytes: 512,
        forward_interval: 1,
        ttl: 5,
        dedup_window_size: 3,
        empty_block_delay: Duration::from_millis(500),
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
    b.on_submit_transaction(t1.clone()).unwrap();
    b.on_submit_transaction(t2.clone()).unwrap();

    // t1 reconstructed and should be removed from retry
    b.on_block_reconstructed(view(1), vec![t1.commit()]);

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

fn builder_with(config: BlockBuilderConfig) -> BlockBuilder<TestTypes> {
    BlockBuilder::new(
        Arc::new(TestInstanceState::default()),
        mock_membership(),
        config,
        test_upgrade_lock(),
    )
}

#[tokio::test]
async fn test_forward_batch_stops_at_leader_limit() {
    let mut b = builder_with(BlockBuilderConfig {
        max_leader_bytes: 2,
        ..small_config()
    });
    for n in 1..=3 {
        b.on_submit_transaction(tx(n)).unwrap();
    }

    let forwarded = b.on_view_changed(view(1));
    assert_eq!(forwarded.len(), 2, "batch should stop at max_leader_bytes");
}

#[tokio::test]
async fn test_forward_batch_sends_oversized_transaction_alone() {
    let mut b = builder_with(BlockBuilderConfig {
        max_leader_bytes: 2,
        ..small_config()
    });
    b.on_submit_transaction(TestTransaction::new(vec![0; 5]))
        .unwrap();

    let forwarded = b.on_view_changed(view(1));
    assert_eq!(
        forwarded.len(),
        1,
        "a transaction over the cap should still be forwarded"
    );
}

#[tokio::test]
async fn test_forward_interval_throttles_resends() {
    let mut b = builder_with(BlockBuilderConfig {
        forward_interval: 3,
        ..small_config()
    });
    let t = tx(1);
    b.on_submit_transaction(t.clone()).unwrap();

    assert_eq!(b.on_view_changed(view(1)).len(), 1);
    b.on_forwarded(view(1), &[t.commit()]);

    assert!(
        b.on_view_changed(view(2)).is_empty(),
        "tx should not be re-forwarded within the interval"
    );
    assert_eq!(
        b.on_view_changed(view(4)).len(),
        1,
        "tx should be re-forwarded after the interval"
    );
}

#[tokio::test]
async fn test_full_retry_buffer_rejects_submission() {
    let mut b = builder_with(BlockBuilderConfig {
        max_mempool_bytes: 2,
        ..small_config()
    });
    b.on_submit_transaction(tx(1)).unwrap();
    b.on_submit_transaction(tx(2)).unwrap();

    let err = b.on_submit_transaction(tx(3)).unwrap_err();
    assert!(matches!(err, BlockError::MempoolFull { .. }), "got {err}");
}

#[tokio::test]
async fn test_every_pending_transaction_is_forwarded_before_ttl() {
    let config = BlockBuilderConfig {
        max_leader_bytes: 2,
        forward_interval: 5,
        ttl: 50,
        ..small_config()
    };
    let mut b = builder_with(config);
    for n in 1..=6 {
        b.on_submit_transaction(tx(n)).unwrap();
    }

    let mut seen = HashSet::new();
    for v in 1..=50 {
        let forwarded = b.on_view_changed(view(v));
        let hashes: Vec<_> = forwarded.iter().map(Committable::commit).collect();
        seen.extend(hashes.iter().copied());
        b.on_forwarded(view(v), &hashes);
    }

    assert_eq!(seen.len(), 6, "every pending tx should reach a leader");
}

#[tokio::test]
async fn test_unforwarded_batch_is_retried_next_view() {
    let mut b = builder_with(BlockBuilderConfig {
        forward_interval: 5,
        ..small_config()
    });
    b.on_submit_transaction(tx(1)).unwrap();

    assert_eq!(b.on_view_changed(view(1)).len(), 1);
    assert_eq!(
        b.on_view_changed(view(2)).len(),
        1,
        "a batch the caller never sent should stay due"
    );
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

#[tokio::test]
async fn reconstructed_block_drops_its_transactions_from_leader_buffer() {
    let mut b = builder();
    b.on_transactions(tx_msg(view(1), Vec::from([tx(1), tx(2)])));
    b.on_block_reconstructed(view(1), Vec::from([tx(1).commit()]));
    let (txns, _) = b.drain(view(2), epoch());
    assert_eq!(txns, Vec::from([tx(2)]));
}

#[tokio::test]
async fn reconstructed_block_drops_later_copies_of_its_transactions() {
    let mut b = builder();
    b.on_block_reconstructed(view(1), Vec::from([tx(1).commit()]));
    b.on_transactions(tx_msg(view(2), Vec::from([tx(1)])));
    let (txns, _) = b.drain(view(2), epoch());
    assert!(txns.is_empty());
}
