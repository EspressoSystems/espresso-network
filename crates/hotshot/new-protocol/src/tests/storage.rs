use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use committable::RawCommitmentBuilder;
use consensus_store::{
    Batch, Error, Store,
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
    storage::{ActionKind, DecidedState, Storage, StorageOutput},
    store::{AppWrite, ConsensusStore, Import},
};

/// A writer over a store that can be made to fail, counting the batches it commits.
fn test_storage() -> (
    Storage<TestTypes, FlakyStore>,
    ConsensusStore<TestTypes, FlakyStore>,
) {
    let (_, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let store = ConsensusStore::new(Arc::new(FlakyStore::default()));
    (Storage::new(store.clone(), &private_key), store)
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

/// A store that can no longer tell whether a commit happened stops the writer.
#[tokio::test]
async fn test_poisoned_store_is_an_error() {
    let (mut storage, store) = test_storage();
    store.store().poisoned.store(true, Ordering::Relaxed);
    storage.record_action(ViewNumber::new(1), ActionKind::Vote);
    assert!(storage.next().await.is_err());
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
    poisoned: AtomicBool,
    commits: AtomicUsize,
}

impl Store for FlakyStore {
    type Snapshot = MemSnapshot;

    fn apply(&self, batch: &Batch) -> Result<(), Error> {
        if self.poisoned.load(Ordering::Relaxed) {
            return Err(Error::Poisoned("test".into()));
        }
        if self.fail.load(Ordering::Relaxed) {
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
        entries[0].0.cert1, None,
        "only the newest carries the certificates"
    );
    assert_eq!(entries[1].0.cert1.as_ref(), Some(&v2.cert1));
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

    store.ack(v2.leaf.height()).unwrap();
    let left: Vec<_> = store
        .decided_from(0, usize::MAX)
        .unwrap()
        .into_iter()
        .map(|(d, _)| d.leaf.height())
        .collect();
    assert_eq!(left, vec![v2.leaf.height()]);
}

/// The writer reports when the decide log reaches its limit, and again when the ingester frees
/// enough of it.
#[tokio::test]
async fn test_decide_log_limit() {
    let (_, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let store = ConsensusStore::new(Arc::new(FlakyStore::default()));
    let mut storage = Storage::new(store.clone(), &private_key).with_decide_log_limit(1);
    let data = super::common::utils::TestData::new(1).await;
    let v0 = &data.views[0];
    let view = ViewNumber::new(5);

    storage.append_decided(
        std::slice::from_ref(&v0.leaf),
        &v0.cert1,
        None,
        &[None],
        &[],
    );
    storage.record_action(view, ActionKind::Vote);
    // The log is measured while the batch completes, so either may be reported first.
    let mut outputs = vec![storage.next().await.unwrap(), storage.next().await.unwrap()];
    outputs.sort_by_key(|output| matches!(output, StorageOutput::DecideLogFull(_)));
    assert_eq!(
        outputs,
        vec![
            StorageOutput::Action(view, ActionKind::Vote),
            StorageOutput::DecideLogFull(true)
        ]
    );
    assert_nothing_more(&mut storage).await;

    store.ack(v0.leaf.height() + 1).unwrap();
    assert_eq!(
        timeout(Duration::from_secs(5), storage.next())
            .await
            .expect("the freed log is reported")
            .unwrap(),
        StorageOutput::DecideLogFull(false)
    );
    assert_nothing_more(&mut storage).await;
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
/// oldest, with the state of each leaf.
#[tokio::test]
async fn test_decided_state_written_with_the_decide() {
    struct Order(u64);

    impl DecidedState<TestTypes> for Order {
        fn decided(
            &mut self,
            leaf: &Leaf2<TestTypes>,
            state: Option<&StateEntry<TestTypes>>,
        ) -> Vec<AppWrite> {
            self.0 += 1;
            vec![AppWrite::Put {
                table: "order",
                key: self.0.to_be_bytes().to_vec(),
                value: [
                    leaf.height().to_be_bytes().as_slice(),
                    &[state.is_some() as u8],
                ]
                .concat(),
            }]
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
        .app_entries("order", &[])
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
}

/// The application's tables keep their writes apart from each other and from consensus, and the
/// import writes them with the rest of the imported state.
#[tokio::test]
async fn test_app_tables() {
    let store = ConsensusStore::<TestTypes, MemStore>::new(Arc::new(MemStore::new()));
    let put = |table, key: u8, value: &[u8]| AppWrite::Put {
        table,
        key: vec![key],
        value: value.to_vec(),
    };
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
    assert_eq!(store.app_get("a", &[1]).unwrap(), Some(b"one".to_vec()));
    assert_eq!(store.app_get("b", &[1]).unwrap(), Some(b"other".to_vec()));
    assert_eq!(store.app_get("a", &[3]).unwrap(), None);

    store
        .app_write(&[
            put("a", 3, b"three"),
            AppWrite::DeleteBefore {
                table: "a",
                key: vec![2],
            },
        ])
        .unwrap();
    assert_eq!(
        store.app_entries("a", &[]).unwrap(),
        vec![(vec![2], b"two".to_vec()), (vec![3], b"three".to_vec())]
    );
    assert_eq!(
        store.app_entries("a", &[3]).unwrap(),
        vec![(vec![3], b"three".to_vec())]
    );

    store.app_write(&[AppWrite::Clear { table: "a" }]).unwrap();
    assert!(store.app_entries("a", &[]).unwrap().is_empty());
    assert_eq!(store.app_get("b", &[1]).unwrap(), Some(b"other".to_vec()));

    assert!(
        store.app_get("Not a name", &[1]).is_err(),
        "invalid table names are refused"
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
