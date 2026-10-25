use std::{sync::atomic::Ordering, time::Duration};

use committable::RawCommitmentBuilder;
use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    signature_key::BLSPubKey,
    traits::signature_key::SignatureKey,
};
use tokio::time::timeout;

use super::common::utils::{build_cert1, mock_membership};
use crate::storage::{ActionKind, Storage, StorageOutput};

fn test_storage() -> (
    Storage<TestTypes, TestStorage<TestTypes>>,
    TestStorage<TestTypes>,
) {
    let (_, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let inner = TestStorage::<TestTypes>::default();
    (Storage::new(inner.clone(), private_key), inner)
}

/// A successful write surfaces exactly one completion through `next()`.
#[tokio::test]
async fn test_record_action_completion() {
    let (mut storage, inner) = test_storage();
    let view = ViewNumber::new(7);

    storage.record_action(view, None, ActionKind::Vote);

    let stored = storage.next().await.expect("completion");
    assert_eq!(stored, StorageOutput::Action(view, ActionKind::Vote));
    assert_eq!(inner.last_actioned_view().await, view);
    assert_eq!(inner.restart_view().await, view + 1);
    assert!(storage.next().await.is_none());
}

/// A failing write retries until it succeeds and only then completes.
#[tokio::test]
async fn test_record_action_retries_until_success() {
    let (mut storage, inner) = test_storage();
    let view = ViewNumber::new(3);

    inner.should_return_err.store(true, Ordering::Relaxed);
    storage.record_action(view, None, ActionKind::Propose);
    assert!(
        timeout(Duration::from_millis(100), storage.next())
            .await
            .is_err(),
        "no completion while the write keeps failing"
    );

    inner.should_return_err.store(false, Ordering::Relaxed);
    let stored = timeout(Duration::from_secs(5), storage.next())
        .await
        .expect("completes once the write succeeds")
        .expect("completion");
    assert_eq!(stored, StorageOutput::Action(view, ActionKind::Propose));
    assert!(storage.next().await.is_none());
}

/// gc aborts in-flight writes below the given view; no completion fires.
#[tokio::test]
async fn test_gc_aborts_pending_writes() {
    let (mut storage, inner) = test_storage();

    inner.should_return_err.store(true, Ordering::Relaxed);
    storage.record_action(ViewNumber::new(2), None, ActionKind::Vote);
    storage.gc(ViewNumber::new(5));
    inner.should_return_err.store(false, Ordering::Relaxed);

    assert!(storage.next().await.is_none());
}

/// The boundary certificate is written however long storage fails, even after
/// gc passes its view: nothing else carries it once the view has passed.
#[tokio::test]
async fn test_gc_does_not_abort_the_boundary_certificate() {
    let (mut storage, inner) = test_storage();
    let epoch = EpochNumber::genesis();
    let membership = mock_membership().membership_for_epoch(Some(epoch)).unwrap();
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], 0);
    let leaf = RawCommitmentBuilder::new("leaf").u64(9).finalize();
    let view = ViewNumber::new(10);
    let cert = build_cert1(leaf, epoch, 9, &membership, view, &public_key, &private_key);

    inner.should_return_err.store(true, Ordering::Relaxed);
    storage.append_boundary_qc2(cert.clone());
    storage.gc(view + 5);
    inner.should_return_err.store(false, Ordering::Relaxed);

    timeout(Duration::from_secs(5), async {
        while inner.boundary_qc_cloned().await.is_none() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("the write completes once storage recovers");
    assert_eq!(inner.boundary_qc_cloned().await, Some(cert));
}
