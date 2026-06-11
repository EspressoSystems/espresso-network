use std::{sync::atomic::Ordering, time::Duration};

use hotshot::types::BLSPubKey;
use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
use hotshot_types::{data::ViewNumber, traits::signature_key::SignatureKey};
use tokio::time::timeout;

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
