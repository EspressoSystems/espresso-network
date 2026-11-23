use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use app_tables::{AppWrite, TableName};
use committable::RawCommitmentBuilder;
use consensus_store::{
    Batch, Bytes, Error, Op, Store, Table,
    mem::{MemSnapshot, MemStore},
};
use hotshot_example_types::{block_types::TestBlockPayload, node_types::TestTypes};
use hotshot_types::{
    data::{EpochNumber, Leaf2, VidCommitment, VidCommitment2, ViewNumber},
    signature_key::BLSPubKey,
    simple_vote::LockView,
    traits::{BlockPayload, signature_key::SignatureKey},
};
use tokio::time::timeout;

use super::common::utils::{build_cert1, mock_membership};
use crate::{
    state::StateEntry,
    storage::{ActionKind, DecidedState, DecidedWrites, Storage, StorageOutput},
    store::{ConsensusStore, Import, StateChange},
};

/// A writer over a store that can be made to fail, counting the batches it commits.
fn test_storage() -> (
    Storage<TestTypes, FlakyStore>,
    ConsensusStore<TestTypes, FlakyStore>,
) {
    let (_, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let store = ConsensusStore::new(Arc::new(FlakyStore::default()));
    (
        Storage::new(store.clone(), &private_key, &store.load().unwrap()),
        store,
    )
}

/// A write surfaces exactly one completion, once it is in the store.
#[tokio::test]
async fn test_record_action_completion() {
    let (mut storage, store) = test_storage();
    let view = ViewNumber::new(7);

    storage.record_action(view, ActionKind::Vote);

    let stored = storage.next().await.expect("completion");
    assert_eq!(stored, StorageOutput::Action(view, ActionKind::Vote));
    let restored = store.load().unwrap();
    assert_eq!(restored.acted_view, Some(view));
    assert_eq!(restored.restart_view, Some(view + 1));
    assert_nothing_more(&mut storage).await;
}

/// A failing batch is retried until it succeeds, and only then completes.
#[tokio::test]
async fn test_record_action_retries_until_success() {
    let (mut storage, store) = test_storage();
    let view = ViewNumber::new(3);

    store.store().fail.store(true, Ordering::Relaxed);
    storage.record_action(view, ActionKind::Propose);
    assert!(
        timeout(Duration::from_millis(100), storage.next())
            .await
            .is_err(),
        "no completion while the batch keeps failing"
    );

    store.store().fail.store(false, Ordering::Relaxed);
    let stored = timeout(Duration::from_secs(5), storage.next())
        .await
        .expect("completes once the batch succeeds")
        .expect("completion");
    assert_eq!(stored, StorageOutput::Action(view, ActionKind::Propose));
    assert_eq!(store.load().unwrap().acted_view, Some(view));
    assert_nothing_more(&mut storage).await;
}

/// Writes queued before the writer runs share one commit, and complete in order.
#[tokio::test]
async fn test_writes_share_one_commit() {
    let (mut storage, store) = test_storage();
    let views = [5, 3, 9].map(ViewNumber::new);
    for view in views {
        storage.record_action(view, ActionKind::Vote);
    }

    for view in views {
        let stored = storage.next().await.expect("completion");
        assert_eq!(stored, StorageOutput::Action(view, ActionKind::Vote));
    }
    assert_eq!(store.store().commits.load(Ordering::Relaxed), 1);
    // The views only ever rise.
    assert_eq!(store.load().unwrap().acted_view, Some(ViewNumber::new(9)));
    assert_nothing_more(&mut storage).await;
}

/// A write from outside the writer joins the writer's batch and shares its commit, and is
/// reported once that commit is durable.
#[tokio::test]
async fn test_foreign_writes_share_the_writers_commit() {
    let (mut storage, store) = test_storage();
    let writes = [AppWrite::Put {
        table: TableName::new("a").unwrap(),
        key: Bytes::from_static(&[1]),
        value: Bytes::from_static(b"one"),
    }];
    let mut write = std::pin::pin!(store.app_write(&writes));
    assert!(
        futures::poll!(write.as_mut()).is_pending(),
        "the write waits for the writer"
    );
    let view = ViewNumber::new(3);
    storage.record_action(view, ActionKind::Vote);

    let (written, mut outputs) = driving(&mut storage, write).await;
    written.unwrap();
    if outputs.is_empty() {
        outputs.push(storage.next().await.unwrap());
    }
    assert_eq!(outputs, vec![StorageOutput::Action(view, ActionKind::Vote)]);
    assert_eq!(store.store().commits.load(Ordering::Relaxed), 1);
    assert_eq!(
        store.app_get(&TableName::new("a").unwrap(), &[1]).unwrap(),
        Some(Bytes::from_static(b"one"))
    );
}

/// A payload write reports the block it stored, and garbage collection removes it.
#[tokio::test]
async fn test_payload_completion_and_gc() {
    let (mut storage, store) = test_storage();
    let view = ViewNumber::new(4);
    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::empty();
    let commitment = VidCommitment2::default();

    storage.append_da(
        view,
        EpochNumber::genesis(),
        payload,
        metadata,
        VidCommitment::V2(commitment),
    );
    let stored = storage.next().await.expect("completion");
    assert_eq!(stored, StorageOutput::Payload(view, commitment));
    assert!(store.load().unwrap().payloads.contains(&(view, commitment)));

    storage.gc(view);
    storage.record_action(view, ActionKind::Vote);
    storage.next().await.expect("completion");
    assert!(store.load().unwrap().payloads.contains(&(view, commitment)));

    storage.gc(view + 1);
    storage.record_action(view + 1, ActionKind::Vote);
    storage.next().await.expect("completion");
    assert!(store.load().unwrap().payloads.is_empty());
}

/// With a payload sink, a payload is offered to the storage instances instead of stored, and is
/// reported durable only once one of them acknowledges it; an acknowledgement of a payload not
/// offered is ignored.
#[tokio::test]
async fn test_payload_sink_reports_acknowledged_payloads() {
    let (storage, store) = test_storage();
    let (queue, acks) = consensus_api::payloads::PayloadQueue::new();
    let mut storage = storage.with_payload_sink(queue.clone(), acks);
    let view = ViewNumber::new(4);
    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::empty();
    let commitment = VidCommitment2::default();

    storage.append_da(
        view,
        EpochNumber::genesis(),
        payload,
        metadata,
        VidCommitment::V2(commitment),
    );
    storage.record_action(view, ActionKind::Vote);
    assert_eq!(
        storage.next().await.expect("completion"),
        StorageOutput::Action(view, ActionKind::Vote),
        "not durable before an acknowledgement"
    );
    assert!(store.load().unwrap().payloads.is_empty(), "not stored");

    queue.ack((99, "another".into()));
    queue.ack((*view, VidCommitment::V2(commitment).to_string()));
    assert_eq!(
        storage.next().await.expect("completion"),
        StorageOutput::Payload(view, commitment)
    );
}

/// The caller drops `next` whenever something else wins its select; a payload sink survives
/// that, so later payloads still go to the storage instances instead of the store.
#[tokio::test]
async fn test_payload_sink_survives_a_cancelled_next() {
    let (storage, store) = test_storage();
    let (queue, acks) = consensus_api::payloads::PayloadQueue::new();
    let mut storage = storage.with_payload_sink(queue.clone(), acks);
    for _ in 0..3 {
        let waited = tokio::time::timeout(Duration::from_millis(10), storage.next()).await;
        assert!(waited.is_err(), "nothing to report, so next is cancelled");
    }
    let view = ViewNumber::new(4);
    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::empty();
    let commitment = VidCommitment2::default();
    storage.append_da(
        view,
        EpochNumber::genesis(),
        payload,
        metadata,
        VidCommitment::V2(commitment),
    );
    queue.ack((*view, VidCommitment::V2(commitment).to_string()));
    assert_eq!(
        storage.next().await.expect("completion"),
        StorageOutput::Payload(view, commitment)
    );
    assert!(store.load().unwrap().payloads.is_empty(), "not stored");
}

/// The lock only moves forward, and garbage collection never removes it or the boundary
/// certificate: nothing else carries the boundary certificate once its view has passed.
#[tokio::test]
async fn test_lock_is_monotonic_and_survives_gc() {
    let (mut storage, store) = test_storage();
    let epoch = EpochNumber::genesis();
    let membership = mock_membership().membership_for_epoch(epoch).unwrap();
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let cert_at = |view: u64| {
        let leaf = RawCommitmentBuilder::new("leaf").u64(view).finalize();
        build_cert1(
            leaf,
            epoch,
            view,
            &membership,
            ViewNumber::new(view),
            &public_key,
            &private_key,
        )
    };
    let later = cert_at(10);
    let earlier = cert_at(8);

    storage.append_high_qc2(later.clone());
    storage.append_boundary_qc2(later.clone());
    assert_eq!(
        storage.next().await.expect("completion"),
        StorageOutput::HighQc(LockView::of(&later))
    );
    storage.append_high_qc2(earlier.clone());
    storage.gc(ViewNumber::new(15));
    assert_eq!(
        storage.next().await.expect("completion"),
        StorageOutput::HighQc(LockView::of(&earlier))
    );

    let restored = store.load().unwrap();
    assert_eq!(restored.locked_qc, Some(later.clone()));
    assert_eq!(restored.boundary_qc, Some(later));
}

/// A restarted writer starts from the values its store holds, and does not lower them.
#[tokio::test]
async fn test_restarted_writer_keeps_the_stored_marks() {
    let (mut storage, store) = test_storage();
    let epoch = EpochNumber::genesis();
    let membership = mock_membership().membership_for_epoch(epoch).unwrap();
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let cert_at = |view: u64| {
        let leaf = RawCommitmentBuilder::new("leaf").u64(view).finalize();
        build_cert1(
            leaf,
            epoch,
            view,
            &membership,
            ViewNumber::new(view),
            &public_key,
            &private_key,
        )
    };
    let next = async |storage: &mut Storage<TestTypes, FlakyStore>| {
        timeout(Duration::from_secs(5), storage.next())
            .await
            .expect("an output")
            .unwrap()
    };
    let later = cert_at(10);
    storage.append_high_qc2(later.clone());
    storage.record_action(ViewNumber::new(12), ActionKind::Vote);
    next(&mut storage).await;
    next(&mut storage).await;

    let mut restarted = Storage::new(store.clone(), &private_key, &store.load().unwrap());
    restarted.append_high_qc2(cert_at(8));
    restarted.record_action(ViewNumber::new(11), ActionKind::Vote);
    // Neither raises a stored value, so both are reported without a write.
    let commits = store.store().commits.load(Ordering::Relaxed);
    assert_eq!(
        next(&mut restarted).await,
        StorageOutput::HighQc(LockView::of(&cert_at(8)))
    );
    assert_eq!(
        next(&mut restarted).await,
        StorageOutput::Action(ViewNumber::new(11), ActionKind::Vote)
    );
    assert_eq!(store.store().commits.load(Ordering::Relaxed), commits);

    let restored = store.load().unwrap();
    assert_eq!(restored.locked_qc, Some(later));
    assert_eq!(restored.acted_view, Some(ViewNumber::new(12)));
    assert_eq!(restored.restart_view, Some(ViewNumber::new(13)));
}

/// A store whose data is corrupt stops the writer.
#[tokio::test]
async fn test_corrupt_store_is_an_error() {
    let (mut storage, store) = test_storage();
    store.store().corrupt.store(true, Ordering::Relaxed);
    storage.record_action(ViewNumber::new(1), ActionKind::Vote);
    assert!(storage.next().await.is_err());
}

/// While the writer's batch keeps failing, foreign writes, such as the acknowledgements that
/// free space, are applied on their own. The writer's batch completes once it succeeds.
#[tokio::test]
async fn test_foreign_writes_apply_while_the_writers_batch_fails() {
    let (mut storage, store) = test_storage();
    store.store().fail_safety.store(true, Ordering::Relaxed);
    let view = ViewNumber::new(3);
    storage.record_action(view, ActionKind::Vote);
    // The writer's batch fails once before the foreign write arrives, which would otherwise
    // join it and fail with it.
    assert!(
        timeout(Duration::from_millis(100), storage.next())
            .await
            .is_err()
    );
    let table = TableName::new("a").unwrap();
    let writes = [AppWrite::Put {
        table: table.clone(),
        key: Bytes::from_static(&[1]),
        value: Bytes::from_static(b"one"),
    }];

    let (written, outputs) = timeout(
        Duration::from_secs(5),
        driving(&mut storage, store.app_write(&writes)),
    )
    .await
    .expect("the foreign write does not wait for the writer's batch");
    written.unwrap();
    assert!(outputs.is_empty(), "the writer's batch is still failing");
    assert_eq!(
        store.app_get(&table, &[1]).unwrap(),
        Some(Bytes::from_static(b"one"))
    );
    assert_eq!(store.load().unwrap().acted_view, None);

    store.store().fail_safety.store(false, Ordering::Relaxed);
    let stored = timeout(Duration::from_secs(5), storage.next())
        .await
        .expect("completes once the batch succeeds")
        .expect("completion");
    assert_eq!(stored, StorageOutput::Action(view, ActionKind::Vote));
    assert_eq!(store.load().unwrap().acted_view, Some(view));
}

/// The delay between retries doubles: attempts at 0, 0.3, 0.9 and 2.1 seconds, where a fixed
/// delay would make a fourth at 0.9 seconds.
#[tokio::test]
async fn test_retry_delay_doubles() {
    let (mut storage, store) = test_storage();
    store.store().fail.store(true, Ordering::Relaxed);
    storage.record_action(ViewNumber::new(1), ActionKind::Vote);
    assert!(
        timeout(Duration::from_millis(1500), storage.next())
            .await
            .is_err()
    );
    assert_eq!(store.store().attempts.load(Ordering::Relaxed), 3);
}

/// A store that can no longer tell whether a commit happened stops the writer.
#[tokio::test]
async fn test_poisoned_store_is_an_error() {
    let (mut storage, store) = test_storage();
    store.store().poisoned.store(true, Ordering::Relaxed);
    storage.record_action(ViewNumber::new(1), ActionKind::Vote);
    assert!(storage.next().await.is_err());
}

/// Runs `work` while driving `storage`, which writes from outside the writer wait on, and
/// returns what `work` gives with the outputs the writer reported meanwhile.
async fn driving<S: consensus_store::Store, R>(
    storage: &mut Storage<TestTypes, S>,
    work: impl Future<Output = R>,
) -> (R, Vec<StorageOutput<TestTypes>>) {
    let mut work = std::pin::pin!(work);
    let mut outputs = Vec::new();
    loop {
        tokio::select! {
            result = &mut work => return (result, outputs),
            output = storage.next() => outputs.push(output.expect("the writer works")),
        }
    }
}

async fn assert_nothing_more(storage: &mut Storage<TestTypes, FlakyStore>) {
    assert!(
        timeout(Duration::from_millis(50), storage.next())
            .await
            .is_err(),
        "no further completion"
    );
}

#[derive(Default)]
struct FlakyStore {
    inner: MemStore,
    fail: AtomicBool,

    /// Fails the batches that write to the table of locks and views, which only the writer does.
    fail_safety: AtomicBool,
    poisoned: AtomicBool,
    corrupt: AtomicBool,
    attempts: AtomicUsize,
    commits: AtomicUsize,
}

impl Store for FlakyStore {
    type Snapshot = MemSnapshot;

    fn apply(&self, batch: &Batch) -> Result<(), Error> {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        if self.poisoned.load(Ordering::Relaxed) {
            return Err(Error::Poisoned("test".into()));
        }
        if self.corrupt.load(Ordering::Relaxed) {
            return Err(Error::Corrupt {
                table: Table::new("test").unwrap(),
                key: Bytes::new(),
                reason: "test",
            });
        }
        let writes_safety = batch.ops().iter().any(|op| match op {
            Op::Put { table, .. } => table.name() == "safety",
            _ => false,
        });
        if self.fail.load(Ordering::Relaxed)
            || (writes_safety && self.fail_safety.load(Ordering::Relaxed))
        {
            return Err(Error::Io(std::io::Error::other("test")));
        }
        self.inner.apply(batch)?;
        self.commits.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn snapshot(&self) -> Result<MemSnapshot, Error> {
        self.inner.snapshot()
    }
}

/// Decides land in the decide log with their payloads, which outlive the payload table's
/// garbage collection; the anchor only moves forward; `ack` frees entries below a height.
#[tokio::test]
async fn test_decide_log() {
    let (mut storage, store) = test_storage();
    let data = super::common::utils::TestData::new(3).await;
    let [v0, v1, v2] = [&data.views[0], &data.views[1], &data.views[2]];
    let payload_of = |v: &super::common::utils::TestView| {
        let VidCommitment::V2(c) = v.leaf.payload_commitment() else {
            panic!("test blocks have V2 commitments");
        };
        (c, hotshot_types::traits::EncodeBytes::encode(&v.payload))
    };

    let mut batch = Batch::new();
    for v in [v0, v1, v2] {
        let (c, bytes) = payload_of(v);
        crate::store::put_payload(&mut batch, v.view_number, &c, bytes).unwrap();
    }
    store.store().apply(&batch).unwrap();

    // v2 decides v1 and v2, newest first.
    storage.append_decided(
        &[v2.leaf.clone(), v1.leaf.clone()],
        &v2.cert1,
        Some(&v2.cert2),
        &[None, None],
        &[],
    );
    storage.gc(v2.view_number + 1);
    flush(&mut storage).await;

    let entries = store.decided_from(0, usize::MAX).unwrap();
    assert_eq!(entries.len(), 2);
    let heights: Vec<_> = entries.iter().map(|(d, _)| d.leaf.height()).collect();
    assert_eq!(heights, vec![v1.leaf.height(), v2.leaf.height()]);
    assert_eq!(
        &entries[0].0.cert1,
        v2.leaf.justify_qc(),
        "an older leaf is certified by the next leaf's justify QC"
    );
    assert_eq!(entries[1].0.cert1, v2.cert1);
    assert_eq!(entries[1].0.cert2.as_ref(), Some(&v2.cert2));
    for ((decided, size), v) in entries.iter().zip([v1, v2]) {
        assert!(
            decided.leaf.block_payload().is_none(),
            "stored without payload"
        );
        let payload = payload_of(v).1;
        assert_eq!(*size, Some(payload.len() as u64));
        assert_eq!(
            store
                .decided_payload(decided.leaf.height())
                .unwrap()
                .as_deref(),
            Some(&payload[..])
        );
    }
    assert_eq!(store.earliest_decided().unwrap(), Some(v1.leaf.height()));
    assert!(
        store.load().unwrap().payloads.is_empty(),
        "garbage collection cleared the payload table"
    );
    assert_eq!(
        store.load().unwrap().anchor.map(|a| a.leaf.height()),
        Some(v2.leaf.height())
    );

    // A decide that fills a gap trails the anchor and does not move it.
    storage.append_decided(
        std::slice::from_ref(&v0.leaf),
        &v0.cert1,
        None,
        &[None],
        &[],
    );
    flush(&mut storage).await;
    assert_eq!(
        store.load().unwrap().anchor.map(|a| a.leaf.height()),
        Some(v2.leaf.height())
    );
    assert_eq!(store.decided_from(0, usize::MAX).unwrap().len(), 3);

    let ranges = [Range {
        start: 0,
        end: v2.leaf.height(),
    }];
    driving(&mut storage, store.ack(&ranges)).await.0.unwrap();
    let left: Vec<_> = store
        .decided_from(0, usize::MAX)
        .unwrap()
        .into_iter()
        .map(|(d, _)| d.leaf.height())
        .collect();
    assert_eq!(left, vec![v2.leaf.height()]);
}

/// Once the decide log reaches its limit, the writer drops its oldest entries, and reports
/// nothing about it.
#[tokio::test]
async fn test_decide_log_drops_the_oldest_entries() {
    let data = super::common::utils::TestData::new(2).await;
    let [v0, v1] = [&data.views[0], &data.views[1]];
    let (_, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let decide_both = |storage: &mut Storage<TestTypes, FlakyStore>| {
        for v in [v0, v1] {
            storage.append_decided(std::slice::from_ref(&v.leaf), &v.cert1, None, &[None], &[]);
        }
    };

    // The size of both entries, written without a limit.
    let (mut storage, store) = test_storage();
    decide_both(&mut storage);
    assert_nothing_more(&mut storage).await;
    let both = *store.decide_log_bytes().borrow();

    // With room for exactly both, the second reaches the limit, and the first goes.
    let store = ConsensusStore::new(Arc::new(FlakyStore::default()));
    let mut storage = Storage::new(store.clone(), &private_key, &store.load().unwrap())
        .with_decide_log_limit(both);
    decide_both(&mut storage);
    assert_nothing_more(&mut storage).await;
    assert_nothing_more(&mut storage).await;
    let left: Vec<_> = store
        .decided_from(0, usize::MAX)
        .unwrap()
        .into_iter()
        .map(|(d, _)| d.leaf.height())
        .collect();
    assert_eq!(left, vec![v1.leaf.height()]);
}

/// A decided epoch root carries its state certificate in the decide log, and the certificate is
/// finalized, also when it is written in the same batch as the decide.
#[tokio::test]
async fn test_decided_state_cert() {
    let (mut storage, store) = test_storage();
    let data = super::common::utils::TestData::new(2).await;
    let [v0, v1] = [&data.views[0], &data.views[1]];
    let cert = super::common::utils::build_state_cert_for_test(
        v1.leaf.block_header(),
        v1.view_number,
        EpochNumber::new(2),
        &Default::default(),
        1,
    );

    storage.append_state_cert(v1.view_number, cert.clone());
    storage.append_decided(
        &[v1.leaf.clone(), v0.leaf.clone()],
        &v1.cert1,
        Some(&v1.cert2),
        &[None, None],
        &[],
    );
    storage.decided(v1.view_number);
    flush(&mut storage).await;

    let entries = store.decided_from(0, usize::MAX).unwrap();
    let certs: Vec<_> = entries.iter().map(|(d, _)| d.state_cert.clone()).collect();
    assert_eq!(certs, vec![None, Some(cert.clone())]);
    assert_eq!(store.load().unwrap().state_cert, Some(cert));
}

/// The application's decided state is written in the batch of the decide, leaf by leaf from the
/// oldest, with the state of each leaf, and the decide log entry of each leaf carries what the
/// application made of its state.
#[tokio::test]
async fn test_decided_state_written_with_the_decide() {
    struct Order(u64);

    impl DecidedState<TestTypes> for Order {
        fn decided(
            &mut self,
            leaf: &Leaf2<TestTypes>,
            state: Option<&StateEntry<TestTypes>>,
        ) -> DecidedWrites {
            self.0 += 1;
            let writes = vec![AppWrite::Put {
                table: TableName::new("order").unwrap(),
                key: self.0.to_be_bytes().to_vec().into(),
                value: [
                    leaf.height().to_be_bytes().as_slice(),
                    &[state.is_some() as u8],
                ]
                .concat()
                .into(),
            }];
            let state = state.map(|_| StateChange {
                chain_config: leaf.height().to_be_bytes().to_vec(),
                ..Default::default()
            });
            DecidedWrites { writes, state }
        }
    }

    let (storage, store) = test_storage();
    let mut storage = storage.with_decided_state(Box::new(Order(0)));
    let data = super::common::utils::TestData::new(2).await;
    let [v0, v1] = [&data.views[0], &data.views[1]];
    let entry = StateEntry {
        state: Arc::new(Default::default()),
        delta: None,
        leaf: v1.leaf.clone(),
    };

    storage.append_decided(
        &[v1.leaf.clone(), v0.leaf.clone()],
        &v1.cert1,
        Some(&v1.cert2),
        &[None, None],
        &[Some(entry), None],
    );
    flush(&mut storage).await;

    let batches = store.store().commits.load(Ordering::SeqCst);
    assert_eq!(batches, 1, "the decided state joins the decide's batch");
    let order: Vec<_> = store
        .app_entries(&TableName::new("order").unwrap(), &[], None)
        .unwrap()
        .into_iter()
        .map(|(_, value)| value)
        .collect();
    let row = |v: &super::common::utils::TestView, with_state: bool| {
        [
            v.leaf.height().to_be_bytes().as_slice(),
            &[with_state as u8],
        ]
        .concat()
    };
    assert_eq!(order, vec![row(v0, false), row(v1, true)]);

    let states: Vec<_> = store
        .decided_from(v0.leaf.height(), 2)
        .unwrap()
        .into_iter()
        .map(|(decided, _)| decided.state)
        .collect();
    let with_state = StateChange {
        chain_config: v1.leaf.height().to_be_bytes().to_vec(),
        ..Default::default()
    };
    assert_eq!(states, vec![None, Some(with_state)]);
}

/// The application's tables keep their writes apart from each other and from consensus, and the
/// import writes them with the rest of the imported state.
#[tokio::test]
async fn test_app_tables() {
    let store = ConsensusStore::<TestTypes, MemStore>::new(Arc::new(MemStore::new()));
    let put = |table: &str, key: u8, value: &[u8]| AppWrite::Put {
        table: TableName::new(table).unwrap(),
        key: vec![key].into(),
        value: Bytes::copy_from_slice(value),
    };
    let bytes = |value: &[u8]| Bytes::copy_from_slice(value);
    store
        .import(Import {
            proposals: Vec::new(),
            locked_qc: None,
            boundary_qc: None,
            acted_view: None,
            restart_view: None,
            upgrade_cert: None,
            state_cert: None,
            payloads: Vec::new(),
            anchor: None,
            app: vec![
                put("a", 1, b"one"),
                put("a", 2, b"two"),
                put("b", 1, b"other"),
            ],
        })
        .unwrap();
    assert_eq!(
        store.app_get(&TableName::new("a").unwrap(), &[1]).unwrap(),
        Some(bytes(b"one"))
    );
    assert_eq!(
        store.app_get(&TableName::new("b").unwrap(), &[1]).unwrap(),
        Some(bytes(b"other"))
    );
    assert_eq!(
        store.app_get(&TableName::new("a").unwrap(), &[3]).unwrap(),
        None
    );

    store
        .app_write(&[
            put("a", 3, b"three"),
            AppWrite::DeleteBefore {
                table: TableName::new("a").unwrap(),
                key: vec![2].into(),
            },
        ])
        .await
        .unwrap();
    assert_eq!(
        store
            .app_entries(&TableName::new("a").unwrap(), &[], None)
            .unwrap(),
        vec![(bytes(&[2]), bytes(b"two")), (bytes(&[3]), bytes(b"three"))]
    );
    assert_eq!(
        store
            .app_entries(&TableName::new("a").unwrap(), &[3], None)
            .unwrap(),
        vec![(bytes(&[3]), bytes(b"three"))]
    );

    store
        .app_write(&[AppWrite::Clear {
            table: TableName::new("a").unwrap(),
        }])
        .await
        .unwrap();
    assert!(
        store
            .app_entries(&TableName::new("a").unwrap(), &[], None)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store.app_get(&TableName::new("b").unwrap(), &[1]).unwrap(),
        Some(bytes(b"other"))
    );

    assert!(store.imported().unwrap());
}

/// Applies everything queued, by queueing one more write and waiting for it.
async fn flush(storage: &mut Storage<TestTypes, FlakyStore>) {
    let view = ViewNumber::new(u64::MAX - 1);
    storage.record_action(view, ActionKind::Propose);
    loop {
        if storage.next().await.expect("completion")
            == StorageOutput::Action(view, ActionKind::Propose)
        {
            return;
        }
    }
}

/// Shutdown finishes when the queued writes report nothing durable, such as only garbage
/// collection, and when a batch is already being applied.
#[tokio::test]
async fn test_flush_completes_without_outputs() {
    let (mut storage, store) = test_storage();
    storage.gc(ViewNumber::new(3));
    timeout(Duration::from_secs(5), storage.flush())
        .await
        .expect("flush finishes");

    let (mut storage, _) = test_storage();
    storage.record_action(ViewNumber::new(1), ActionKind::Vote);
    // Start applying the batch, then shut down while it runs.
    let _ = timeout(Duration::ZERO, storage.next()).await;
    storage.gc(ViewNumber::new(3));
    timeout(Duration::from_secs(5), storage.flush())
        .await
        .expect("flush finishes");
    drop(store);
}
