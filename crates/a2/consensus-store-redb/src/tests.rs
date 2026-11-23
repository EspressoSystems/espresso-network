use consensus_store::{
    Batch, Error, Snapshot, Store, Table,
    conformance::{self, assert_usage_matches_rows, big},
};
use tempfile::TempDir;

use crate::RedbStore;

#[test]
fn conformance() {
    let mut dirs = Vec::new();
    conformance::run(|| {
        let dir = TempDir::new().unwrap();
        let store = RedbStore::open(dir.path()).unwrap();
        dirs.push(dir);
        store
    });
}

#[test]
fn reopen_keeps_data() {
    let dir = TempDir::new().unwrap();
    let t = table();
    {
        let store = RedbStore::open(dir.path()).unwrap();
        store
            .apply(
                Batch::new()
                    .put(&t, "small", "s")
                    .put(&t, "large", big(1 << 20, 1)),
            )
            .unwrap();
    }
    let store = RedbStore::open(dir.path()).unwrap();
    store.apply(Batch::new().put(&t, "more", "m")).unwrap();
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "small").as_deref(), Some(&b"s"[..]));
    assert_eq!(get(&snap, &t, "large"), Some(big(1 << 20, 1)));
    assert_eq!(get(&snap, &t, "more").as_deref(), Some(&b"m"[..]));
    assert_usage_matches_rows(&snap, &[&t]);
}

/// An apply that fails before its commit commits nothing, and the same batch applies when tried
/// again.
#[test]
fn rejected_apply_commits_nothing_and_can_be_retried() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path()).unwrap();
    store.apply(Batch::new().put(&t, "a", "1")).unwrap();

    let mut batch = Batch::new();
    batch.put(&t, "b", "2").put(&t, "c", "3").delete(&t, "a");
    store
        .reject_next_apply
        .store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(store.apply(&batch).is_err());
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "a").as_deref(), Some(&b"1"[..]));
    assert_eq!(get(&snap, &t, "b"), None);
    assert_usage_matches_rows(&snap, &[&t]);

    store.apply(&batch).unwrap();
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "a"), None);
    assert_eq!(get(&snap, &t, "b").as_deref(), Some(&b"2"[..]));
    assert_usage_matches_rows(&snap, &[&t]);
}

#[test]
fn uncertain_commit_poisons_until_reopened() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path()).unwrap();
    store.apply(Batch::new().put(&t, "a", "1")).unwrap();

    // The commit succeeds, but reports failure.
    store
        .fail_next_commit
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let batch = {
        let mut b = Batch::new();
        b.put(&t, "b", "2").delete(&t, "a");
        b
    };
    assert!(matches!(store.apply(&batch), Err(Error::Poisoned(_))));
    assert!(matches!(
        store.apply(&Batch::new()),
        Err(Error::Poisoned(_))
    ));
    assert!(matches!(store.snapshot(), Err(Error::Poisoned(_))));

    // Reopening shows the batch as committed.
    drop(store);
    let store = RedbStore::open(dir.path()).unwrap();
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "a"), None);
    assert_eq!(get(&snap, &t, "b").as_deref(), Some(&b"2"[..]));
    assert_usage_matches_rows(&snap, &[&t]);
}

#[test]
fn tables_left_empty_are_deleted() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let gone = Table::new("gone").unwrap();
    let store = RedbStore::open(dir.path()).unwrap();
    store
        .apply(
            Batch::new()
                .put(&t, "a", "1")
                .put(&gone, "a", "1")
                .delete(&gone, "a")
                .delete(&Table::new("never").unwrap(), "a"),
        )
        .unwrap();
    store.apply(Batch::new().drop_table(&t)).unwrap();
    store.apply(Batch::new().delete(&t, "a")).unwrap();

    let names: Vec<String> = redb::ReadableDatabase::begin_read(&store.db)
        .unwrap()
        .list_tables()
        .unwrap()
        .map(|h| redb::TableHandle::name(&h).to_owned())
        .collect();
    assert!(names.iter().all(|n| n.starts_with('_')), "{names:?}");
    assert_eq!(
        store.snapshot().unwrap().usage(&t).unwrap(),
        Default::default()
    );
}

#[test]
fn open_creates_nested_directories() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a").join("b").join("store");
    let store = RedbStore::open(&path).unwrap();
    store.apply(Batch::new().put(&table(), "k", "v")).unwrap();
    assert!(path.join("index.redb").exists());
}

fn table() -> Table {
    Table::new("t").unwrap()
}

fn get(snap: &impl Snapshot, t: &Table, k: &str) -> Option<bytes::Bytes> {
    snap.get(t, k.as_bytes()).unwrap()
}
