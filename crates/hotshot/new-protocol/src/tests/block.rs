use std::{collections::BTreeMap, marker::PhantomData, sync::Arc, time::Duration};

use committable::Committable;
use hotshot::types::BLSPubKey;
use hotshot_example_types::{
    block_types::TestTransaction, node_types::TestTypes, state_types::TestInstanceState,
};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    message::UpgradeLock,
    simple_certificate::UpgradeCertificate,
    simple_vote::UpgradeProposalData,
    traits::signature_key::SignatureKey,
};
use versions::{NEW_PROTOCOL_VERSION, TIMEOUT_EPOCH_VERSION, Upgrade, Version};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig, forward_budget},
    helpers::test_upgrade_lock,
    message::{BlockMessage, DedupManifest, Message, MessageType, TransactionMessage, Validated},
    network::{MIN_MESSAGE_LIMIT, message_limit},
    tests::common::utils::mock_membership,
};

fn tx(n: u8) -> TestTransaction {
    TestTransaction::new(vec![n])
}

fn view(n: u64) -> ViewNumber {
    ViewNumber::new(n)
}

fn submit(
    b: &mut BlockBuilder<TestTypes>,
    tx: TestTransaction,
) -> Vec<TransactionMessage<TestTypes>> {
    b.on_submit_transaction(tx).unwrap()
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

/// The same block size for every protocol version.
fn sizes(block_size: u64) -> BTreeMap<Version, u64> {
    BTreeMap::from([(versions::version(0, 0), block_size)])
}

fn small_config() -> BlockBuilderConfig {
    BlockBuilderConfig {
        max_retry_bytes: 1024,
        block_sizes: sizes(512),
        ttl: 5,
        dedup_window_size: 3,
        empty_block_delay: Duration::from_millis(500),
        fanout: 1,
        forward_transactions: true,
    }
}

fn builder() -> BlockBuilder<TestTypes> {
    builder_with(small_config())
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
async fn submit_sends_to_each_upcoming_leader() {
    let mut b = builder_with(BlockBuilderConfig {
        fanout: 2,
        ..small_config()
    });
    b.on_view_changed(view(4));

    assert_eq!(
        submit(&mut b, tx(1)),
        Vec::from([
            tx_msg(view(6), Vec::from([tx(1)])),
            tx_msg(view(7), Vec::from([tx(1)])),
        ])
    );
    assert!(
        submit(&mut b, tx(1)).is_empty(),
        "a pending transaction is not sent again when resubmitted"
    );
}

#[tokio::test]
async fn pending_transaction_is_resent_only_after_its_leaders_had_their_turn() {
    let mut b = builder_with(BlockBuilderConfig {
        fanout: 2,
        ttl: 10,
        ..small_config()
    });
    submit(&mut b, tx(1));

    for v in 1..=4 {
        assert!(
            b.on_view_changed(view(v)).is_empty(),
            "view {v} is too early to resend"
        );
    }
    assert_eq!(
        b.on_view_changed(view(5)),
        Vec::from([
            tx_msg(view(7), Vec::from([tx(1)])),
            tx_msg(view(8), Vec::from([tx(1)])),
        ])
    );
    assert!(
        b.on_view_changed(view(6)).is_empty(),
        "a resent transaction waits for its new leaders"
    );
}

#[tokio::test]
async fn test_retry_buffer() {
    let mut b = builder();
    let t1 = tx(1);
    let t2 = tx(2);
    b.on_submit_transaction(t1.clone()).unwrap();
    b.on_submit_transaction(t2.clone()).unwrap();

    b.on_block_reconstructed(view(1), vec![t1.commit()]);

    assert_eq!(
        b.on_view_changed(view(4)),
        Vec::from([tx_msg(view(6), Vec::from([t2]))]),
        "only unconfirmed tx should be resent"
    );

    let resent = b.on_view_changed(view(6));
    assert!(resent.is_empty(), "tx past ttl should expire");
}

#[tokio::test]
async fn test_forward_batch_stops_at_one_block() {
    let mut b = builder_with(BlockBuilderConfig {
        block_sizes: sizes(2),
        ..small_config()
    });
    b.on_submit_transaction(tx(1)).unwrap();
    b.on_view_changed(view(1));
    b.on_submit_transaction(tx(2)).unwrap();
    b.on_submit_transaction(tx(3)).unwrap();

    let resent = b.on_view_changed(view(5));
    assert_eq!(resent.len(), 1, "one message per upcoming leader");
    assert_eq!(
        resent[0].transactions.len(),
        2,
        "batch should stop at one block"
    );
    assert_eq!(
        resent[0].transactions[0],
        tx(1),
        "the oldest transaction goes first"
    );
}

/// An upgrade from `NEW_PROTOCOL_VERSION` to `TIMEOUT_EPOCH_VERSION` taking effect at `first_view`.
fn upgrading_at(first_view: u64) -> UpgradeLock<TestTypes> {
    let first_view = view(first_view);
    let data = UpgradeProposalData {
        old_version: NEW_PROTOCOL_VERSION,
        new_version: TIMEOUT_EPOCH_VERSION,
        decide_by: first_view,
        new_version_hash: Vec::new(),
        old_version_last_view: first_view - 1,
        new_version_first_view: first_view,
    };
    let commitment = data.commit();
    let cert = UpgradeCertificate::new(data, commitment, first_view, None, PhantomData);
    UpgradeLock::from_certificate(
        Upgrade::new(NEW_PROTOCOL_VERSION, TIMEOUT_EPOCH_VERSION),
        &Some(cert),
    )
}

fn builder_upgrading(old_size: u64, new_size: u64) -> BlockBuilder<TestTypes> {
    BlockBuilder::new(
        Arc::new(TestInstanceState::default()),
        mock_membership(),
        BlockBuilderConfig {
            block_sizes: BTreeMap::from([
                (NEW_PROTOCOL_VERSION, old_size),
                (TIMEOUT_EPOCH_VERSION, new_size),
            ]),
            ..small_config()
        },
        upgrading_at(5),
    )
}

#[tokio::test]
async fn test_larger_blocks_apply_once_the_upgrade_takes_effect() {
    let mut b = builder_upgrading(2, 4);
    for n in 1..=4 {
        b.on_submit_transaction(tx(n)).unwrap();
    }

    assert_eq!(
        b.on_view_changed(view(4))[0].transactions.len(),
        4,
        "a resend for view 6 uses the new block size"
    );

    b.on_transactions(tx_msg(view(4), (5..=7).map(tx).collect()));
    let (txns, _) = b.drain(view(4), epoch());
    assert_eq!(txns.len(), 2, "a block for view 4 uses the old size");

    b.on_transactions(tx_msg(view(5), (8..=11).map(tx).collect()));
    let (txns, _) = b.drain(view(5), epoch());
    assert_eq!(txns.len(), 4, "a block for view 5 uses the new size");
}

#[tokio::test]
async fn test_smaller_blocks_drop_what_no_longer_fits() {
    let mut b = builder_upgrading(4, 2);
    b.on_submit_transaction(TestTransaction::new(vec![0; 3]))
        .unwrap();
    b.on_submit_transaction(tx(1)).unwrap();

    assert_eq!(
        b.on_view_changed(view(4)),
        Vec::from([tx_msg(view(6), Vec::from([tx(1)]))]),
        "a transaction over the new size is dropped instead of blocking the batch"
    );
}

#[tokio::test]
async fn test_block_size_follows_the_running_version() {
    // The later version's size does not apply before its upgrade.
    let later = versions::version(u16::MAX, 0);
    let mut b = builder_with(BlockBuilderConfig {
        block_sizes: BTreeMap::from([(versions::version(0, 0), 2), (later, 100)]),
        ..small_config()
    });
    b.on_submit_transaction(TestTransaction::new(vec![0; 5]))
        .unwrap_err();
    assert!(b.on_view_changed(view(3)).is_empty());
}

#[tokio::test]
async fn test_forward_batch_fills_the_block_with_later_transactions() {
    let mut b = builder_with(BlockBuilderConfig {
        block_sizes: sizes(5),
        ..small_config()
    });
    b.on_submit_transaction(TestTransaction::new(vec![1; 3]))
        .unwrap();
    b.on_view_changed(view(1));
    b.on_submit_transaction(TestTransaction::new(vec![2; 3]))
        .unwrap();
    b.on_submit_transaction(TestTransaction::new(vec![3; 2]))
        .unwrap();

    let resent = b.on_view_changed(view(5));
    assert_eq!(
        resent,
        Vec::from([tx_msg(
            view(7),
            Vec::from([
                TestTransaction::new(vec![1; 3]),
                TestTransaction::new(vec![3; 2])
            ])
        )])
    );
}

#[tokio::test]
async fn test_transaction_larger_than_a_block_is_rejected() {
    let mut b = builder_with(BlockBuilderConfig {
        block_sizes: sizes(2),
        ..small_config()
    });
    b.on_submit_transaction(TestTransaction::new(vec![0; 5]))
        .unwrap_err();
    assert!(b.on_view_changed(view(3)).is_empty());
}

/// A batch filled to the forward budget encodes under the message limit.
#[tokio::test]
async fn test_full_forward_fits_in_a_message() {
    let block_size = MIN_MESSAGE_LIMIT.get() as u64;
    let limit = message_limit(block_size);
    let mut b = builder_with(BlockBuilderConfig {
        max_retry_bytes: u64::MAX,
        block_sizes: sizes(block_size),
        ..small_config()
    });
    let tx_len = 1000;
    for n in 0..limit.get() / tx_len + 1 {
        let mut payload = vec![0; tx_len];
        payload[..8].copy_from_slice(&n.to_le_bytes());
        b.on_submit_transaction(TestTransaction::new(payload))
            .unwrap();
    }

    let resent = b.on_view_changed(view(4)).remove(0);
    let forwarded = resent.transactions.len();
    let message = Message::<TestTypes, Validated> {
        sender: BLSPubKey::generated_from_seed_indexed([0u8; 32], 0).0,
        message_type: MessageType::Block(BlockMessage::Transactions(resent)),
    };
    let len = test_upgrade_lock::<TestTypes>()
        .serialize(&message)
        .unwrap()
        .len();
    assert!(
        len <= limit.get(),
        "{len} bytes exceed the {limit} byte limit"
    );
    assert_eq!(
        forwarded,
        forward_budget(limit) as usize / (tx_len + 8),
        "the message budget, not the block size, should stop the batch"
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

/// Every block for a view is built from the transactions taken for its first.
///
/// The leader disperses a block as soon as it is built, and peers keep one
/// share per leader and view, so the block it proposes after a timeout must
/// carry the payload already dispersed. A transaction arriving in between waits
/// for a later view.
#[tokio::test]
async fn test_request_block_same_view_reuses_transactions() {
    use crate::{block::BlockAndHeaderRequest, tests::common::utils::TestData};

    let mut b = builder();
    let test_data = TestData::new(2).await;
    let target_view = ViewNumber::new(5);
    let request = |parent_index: usize| BlockAndHeaderRequest {
        view: target_view,
        epoch: EpochNumber::genesis(),
        parent_proposal: test_data.views[parent_index].proposal.data.clone(),
    };

    b.on_transactions(tx_msg(view(4), vec![tx(1), tx(2)]));
    b.request_block(request(0));
    b.on_transactions(tx_msg(view(4), vec![tx(3)]));
    b.request_block(request(1));

    let mut outputs = Vec::new();
    for _ in 0..2 {
        let Some(Ok(output)) = b.next().await else {
            panic!("expected an Ok block builder output");
        };
        outputs.push(output);
    }
    assert_eq!(outputs[0].payload_commitment, outputs[1].payload_commitment);
    let mut hashes = outputs[1].manifest.hashes.clone();
    hashes.sort();
    let mut expected = vec![tx(1).commit(), tx(2).commit()];
    expected.sort();
    assert_eq!(hashes, expected);

    let (txns, _) = b.drain(view(6), epoch());
    assert_eq!(txns, vec![tx(3)]);
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
async fn leader_holds_transactions_for_later_views_and_builds_a_block_at_a_time() {
    let mut b = builder_with(BlockBuilderConfig {
        block_sizes: sizes(2),
        ..small_config()
    });
    b.on_transactions(tx_msg(view(1), (1..=5).map(tx).collect()));

    let (first, _) = b.drain(view(1), epoch());
    let (second, _) = b.drain(view(2), epoch());
    let (third, _) = b.drain(view(3), epoch());
    assert_eq!(first.len(), 2, "one block per build");
    assert_eq!(second.len(), 2, "the rest waits for the next build");
    assert!(
        third.is_empty(),
        "the pool holds fanout + 1 blocks, so the fifth transaction was refused"
    );
}

#[tokio::test]
async fn pooled_transactions_expire_after_ttl() {
    let mut b = builder();
    b.on_transactions(tx_msg(view(1), Vec::from([tx(1)])));
    b.on_view_changed(view(6));
    b.on_transactions(tx_msg(view(2), Vec::from([tx(2)])));
    b.on_view_changed(view(7));

    let (txns, _) = b.drain(view(7), epoch());
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
