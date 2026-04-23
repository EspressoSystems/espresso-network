use std::sync::Arc;

use committable::Committable;
use hotshot_example_types::{
    block_types::TestTransaction, node_types::TestTypes, state_types::TestInstanceState,
};
use hotshot_types::data::{EpochNumber, ViewNumber};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    message::{DedupManifest, TransactionMessage},
    tests::common::utils::{mock_membership, upgrade_lock},
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

async fn builder() -> BlockBuilder<TestTypes> {
    BlockBuilder::new(
        Arc::new(TestInstanceState::default()),
        mock_membership().await,
        small_config(),
        upgrade_lock(),
    )
}

#[tokio::test]
async fn test_retry_buffer() {
    let mut b = builder().await;
    let t1 = tx(1);
    let t2 = tx(2);
    b.on_submit_transaction(t1.clone());
    b.on_submit_transaction(t2.clone());

    // t1 reconstructed and should be removed from retry
    b.on_block_reconstructed(vec![t1.commit()]);

    let forwarded = b.on_view_changed(view(1), epoch());
    assert_eq!(
        forwarded,
        vec![t2],
        "only unconfirmed tx should be forwarded"
    );

    // past ttl
    let forwarded = b.on_view_changed(view(6), epoch());
    assert!(forwarded.is_empty(), "tx past ttl should expire");
}

#[tokio::test]
async fn test_leader_buffer_drain() {
    let mut b = builder().await;
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

#[tokio::test]
async fn test_dedup_window() {
    let mut b = BlockBuilder::new(
        Arc::new(TestInstanceState::default()),
        mock_membership().await,
        BlockBuilderConfig {
            dedup_window_size: 2,
            ..small_config()
        },
        upgrade_lock(),
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
    b.on_view_changed(view(4), epoch());
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
