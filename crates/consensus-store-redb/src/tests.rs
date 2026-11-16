use std::{fs, os::unix::fs::FileExt, path::Path};

use consensus_store::{
    Batch, EXTERN_THRESHOLD, Error, Snapshot, Store, Table,
    conformance::{self, assert_usage_matches_rows, big},
};
use tempfile::TempDir;

use crate::{Config, RedbStore};

/// Every extern value goes into a segment of its own.
fn tiny() -> Config {
    Config { segment_size: 1 }
}

#[test]
fn conformance_default_segments() {
    let mut dirs = Vec::new();
    conformance::run(|| {
        let dir = TempDir::new().unwrap();
        let store = RedbStore::open(dir.path(), Config::default()).unwrap();
        dirs.push(dir);
        store
    });
}

#[test]
fn conformance_tiny_segments() {
    let mut dirs = Vec::new();
    conformance::run(|| {
        let dir = TempDir::new().unwrap();
        let store = RedbStore::open(dir.path(), tiny()).unwrap();
        dirs.push(dir);
        store
    });
}

#[test]
fn reopen_keeps_data() {
    let dir = TempDir::new().unwrap();
    let t = table();
    {
        let store = RedbStore::open(dir.path(), tiny()).unwrap();
        store
            .apply(
                Batch::new()
                    .put(&t, "small", "s")
                    .put(&t, "a", big(EXTERN_THRESHOLD + 1, 1))
                    .put(&t, "b", big(EXTERN_THRESHOLD + 2, 2)),
            )
            .unwrap();
    }
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    store
        .apply(Batch::new().put(&t, "c", big(EXTERN_THRESHOLD + 3, 3)))
        .unwrap();
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "small").as_deref(), Some(&b"s"[..]));
    assert_eq!(get(&snap, &t, "a"), Some(big(EXTERN_THRESHOLD + 1, 1)));
    assert_eq!(get(&snap, &t, "b"), Some(big(EXTERN_THRESHOLD + 2, 2)));
    assert_eq!(get(&snap, &t, "c"), Some(big(EXTERN_THRESHOLD + 3, 3)));
    assert_usage_matches_rows(&snap, &[&t]);
}

#[test]
fn segments_are_deleted_once_unreferenced() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    let mut batch = Batch::new();
    for k in ["a", "b", "c"] {
        batch.put(&t, k, big(EXTERN_THRESHOLD + 1, 0));
    }
    store.apply(&batch).unwrap();
    assert_eq!(segment_files(dir.path()).len(), 3);

    // A copy keeps the segment alive after its first row goes.
    let other = Table::new("other").unwrap();
    store
        .apply(
            Batch::new()
                .copy(&t, "a", &other, "a")
                .delete(&t, "a")
                .delete(&t, "b"),
        )
        .unwrap();
    assert_eq!(segment_files(dir.path()).len(), 2);

    store.apply(Batch::new().drop_table(&other)).unwrap();
    assert_eq!(segment_files(dir.path()).len(), 1);

    // The active segment stays, even unreferenced.
    store.apply(Batch::new().delete(&t, "c")).unwrap();
    assert_eq!(segment_files(dir.path()).len(), 1);

    drop(store);
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    store
        .apply(Batch::new().put(&t, "d", big(EXTERN_THRESHOLD + 1, 4)))
        .unwrap();
    assert_eq!(segment_files(dir.path()).len(), 1);
    assert_eq!(
        get(&store.snapshot().unwrap(), &t, "d"),
        Some(big(EXTERN_THRESHOLD + 1, 4))
    );
}

#[test]
fn open_removes_crash_leftovers() {
    let dir = TempDir::new().unwrap();
    let t = table();
    {
        let store = RedbStore::open(dir.path(), Config::default()).unwrap();
        store
            .apply(Batch::new().put(&t, "a", big(EXTERN_THRESHOLD + 1, 1)))
            .unwrap();
    }
    let segments = segment_files(dir.path());
    assert_eq!(segments.len(), 1);
    let active = dir.path().join("extern").join(&segments[0]);
    let committed = fs::metadata(&active).unwrap().len();

    // An append that never committed, and segment files without an entry.
    let file = fs::OpenOptions::new().write(true).open(&active).unwrap();
    file.write_all_at(b"uncommitted", committed).unwrap();
    fs::write(dir.path().join("extern").join(format!("{:016x}", 7)), b"x").unwrap();
    fs::write(dir.path().join("extern").join("stray"), b"x").unwrap();

    let store = RedbStore::open(dir.path(), Config::default()).unwrap();
    assert_eq!(segment_files(dir.path()), segments);
    assert_eq!(fs::metadata(&active).unwrap().len(), committed);

    store
        .apply(Batch::new().put(&t, "b", big(EXTERN_THRESHOLD + 2, 2)))
        .unwrap();
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "a"), Some(big(EXTERN_THRESHOLD + 1, 1)));
    assert_eq!(get(&snap, &t, "b"), Some(big(EXTERN_THRESHOLD + 2, 2)));
}

#[test]
fn open_fails_on_missing_or_short_segment() {
    for damage in [Damage::Remove, Damage::Truncate] {
        let dir = TempDir::new().unwrap();
        {
            let store = RedbStore::open(dir.path(), Config::default()).unwrap();
            store
                .apply(Batch::new().put(&table(), "a", big(EXTERN_THRESHOLD + 1, 1)))
                .unwrap();
        }
        let path = dir
            .path()
            .join("extern")
            .join(&segment_files(dir.path())[0]);
        match damage {
            Damage::Remove => fs::remove_file(&path).unwrap(),
            Damage::Truncate => fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_len(10)
                .unwrap(),
        }
        let result = RedbStore::open(dir.path(), Config::default());
        assert!(
            matches!(&result, Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::InvalidData),
            "{damage:?}"
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum Damage {
    Remove,
    Truncate,
}

#[test]
fn checksum_mismatch_is_corrupt() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path(), Config::default()).unwrap();
    store
        .apply(Batch::new().put(&t, "a", big(EXTERN_THRESHOLD + 1, 1)))
        .unwrap();
    break_first_byte(
        &dir.path()
            .join("extern")
            .join(&segment_files(dir.path())[0]),
    );

    let snap = store.snapshot().unwrap();
    assert!(matches!(
        snap.get(&t, b"a"),
        Err(Error::Corrupt { reason, .. }) if reason.contains("checksum")
    ));
}

#[test]
fn panicked_apply_deletes_its_segments() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    store
        .apply(Batch::new().put(&t, "a", big(EXTERN_THRESHOLD + 1, 1)))
        .unwrap();
    let before = segment_files(dir.path());

    let mut batch = Batch::new();
    batch
        .put(&t, "b", big(EXTERN_THRESHOLD + 1, 2))
        .put(&t, "c", big(EXTERN_THRESHOLD + 1, 3))
        .update(&t, "a", |_| panic!("deliberate"));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| store.apply(&batch)));
    assert!(result.is_err());
    assert_eq!(segment_files(dir.path()), before);

    store
        .apply(Batch::new().put(&t, "d", big(EXTERN_THRESHOLD + 1, 4)))
        .unwrap();
    drop(store);
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    assert_eq!(segment_files(dir.path()).len(), before.len() + 1);
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "a"), Some(big(EXTERN_THRESHOLD + 1, 1)));
    assert_eq!(get(&snap, &t, "d"), Some(big(EXTERN_THRESHOLD + 1, 4)));
    assert_eq!(get(&snap, &t, "b"), None);
}

#[test]
fn rejected_apply_deletes_its_segments_and_can_be_retried() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    store
        .apply(Batch::new().put(&t, "a", big(EXTERN_THRESHOLD + 1, 1)))
        .unwrap();
    break_first_byte(
        &dir.path()
            .join("extern")
            .join(&segment_files(dir.path())[0]),
    );
    let before = segment_files(dir.path());

    // Reading "a" fails its checksum, so the batch is rejected after appending "b".
    let mut batch = Batch::new();
    batch
        .put(&t, "b", big(EXTERN_THRESHOLD + 1, 2))
        .update(&t, "a", |_| Some(bytes::Bytes::from_static(b"x")));
    assert!(matches!(store.apply(&batch), Err(Error::Corrupt { .. })));
    assert_eq!(segment_files(dir.path()), before);
    assert_eq!(get(&store.snapshot().unwrap(), &t, "b"), None);

    // Once "a" is gone, the same batch applies.
    store.apply(Batch::new().delete(&t, "a")).unwrap();
    store.apply(&batch).unwrap();
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "a").as_deref(), Some(&b"x"[..]));
    assert_eq!(get(&snap, &t, "b"), Some(big(EXTERN_THRESHOLD + 1, 2)));
    assert_usage_matches_rows(&snap, &[&t]);
}

#[test]
fn extern_reference_past_the_segment_is_corrupt() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path(), Config::default()).unwrap();
    store
        .apply(Batch::new().put(&t, "a", big(EXTERN_THRESHOLD + 1, 1)))
        .unwrap();
    for (offset, len) in [
        (0, u64::MAX),
        (u64::MAX, EXTERN_THRESHOLD as u64 + 1),
        (1, EXTERN_THRESHOLD as u64 + 1),
    ] {
        let raw = crate::row::encode_extern(crate::row::ExternRef {
            segment: 0,
            offset,
            len,
            crc: 0,
        });
        let txn = store.db.begin_write().unwrap();
        txn.open_table(crate::rows(&t))
            .unwrap()
            .insert(&b"a"[..], &raw[..])
            .unwrap();
        txn.commit().unwrap();
        assert!(
            matches!(
                store.snapshot().unwrap().get(&t, b"a"),
                Err(Error::Corrupt { reason, .. }) if reason.contains("beyond the end")
            ),
            "offset {offset}, len {len}"
        );
    }
}

#[test]
fn uncertain_commit_poisons_until_reopened() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    store
        .apply(Batch::new().put(&t, "a", big(EXTERN_THRESHOLD + 1, 1)))
        .unwrap();

    // The commit succeeds, but reports failure.
    store
        .fail_next_commit
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let batch = {
        let mut b = Batch::new();
        b.put(&t, "b", big(EXTERN_THRESHOLD + 2, 2)).delete(&t, "a");
        b
    };
    assert!(matches!(store.apply(&batch), Err(Error::Poisoned(_))));
    assert!(matches!(
        store.apply(&Batch::new()),
        Err(Error::Poisoned(_))
    ));
    assert!(matches!(store.snapshot(), Err(Error::Poisoned(_))));

    // Reopening shows the batch as committed, with its segment intact and the freed one gone.
    drop(store);
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    assert_eq!(segment_files(dir.path()).len(), 1);
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "a"), None);
    assert_eq!(get(&snap, &t, "b"), Some(big(EXTERN_THRESHOLD + 2, 2)));
    assert_usage_matches_rows(&snap, &[&t]);
}

#[test]
fn corrupt_rows_can_be_removed() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    store
        .apply(Batch::new().put(&t, "kept", big(EXTERN_THRESHOLD + 1, 1)))
        .unwrap();
    let unknown_segment = crate::row::encode_extern(crate::row::ExternRef {
        segment: 99,
        offset: 0,
        len: EXTERN_THRESHOLD as u64 + 1,
        crc: 0,
    });
    for (key, raw) in [
        ("malformed", &[7u8, 7][..]),
        ("unknown", &unknown_segment[..]),
        ("in-range", &[9][..]),
    ] {
        let txn = store.db.begin_write().unwrap();
        txn.open_table(crate::rows(&t))
            .unwrap()
            .insert(key.as_bytes(), raw)
            .unwrap();
        txn.commit().unwrap();
    }

    store
        .apply(
            Batch::new()
                .delete(&t, "malformed")
                .put(&t, "unknown", "overwritten")
                .delete_range(&t, consensus_store::KeyRange::between("in", "io")),
        )
        .unwrap();
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "malformed"), None);
    assert_eq!(get(&snap, &t, "in-range"), None);
    assert_eq!(
        get(&snap, &t, "unknown").as_deref(),
        Some(&b"overwritten"[..])
    );
    assert_eq!(get(&snap, &t, "kept"), Some(big(EXTERN_THRESHOLD + 1, 1)));

    store.apply(Batch::new().drop_table(&t)).unwrap();
    assert!(get(&store.snapshot().unwrap(), &t, "kept").is_none());
}

#[test]
fn tables_left_empty_are_deleted() {
    let dir = TempDir::new().unwrap();
    let t = table();
    let gone = Table::new("gone").unwrap();
    let store = RedbStore::open(dir.path(), Config::default()).unwrap();
    store
        .apply(
            Batch::new()
                .put(&t, "a", "1")
                .put(&gone, "a", "1")
                .delete(&gone, "a")
                .delete(&Table::new("never").unwrap(), "a")
                .copy(&Table::new("absent").unwrap(), "a", &t, "b")
                .update(&Table::new("read").unwrap(), "a", |_| None),
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
fn open_keeps_foreign_directories_and_removes_non_canonical_names() {
    let dir = TempDir::new().unwrap();
    let t = table();
    {
        // Segments 0 to 10, so that segment 10 is named with a hex letter.
        let store = RedbStore::open(dir.path(), tiny()).unwrap();
        let mut batch = Batch::new();
        for i in 0..11u64 {
            batch.put(
                &t,
                consensus_store::key::u64(i).to_vec(),
                big(EXTERN_THRESHOLD + 1, i),
            );
        }
        store.apply(&batch).unwrap();
    }
    let ext = dir.path().join("extern");
    let segments = segment_files(dir.path());
    assert_eq!(segments.last().unwrap(), "000000000000000a");
    fs::create_dir(ext.join("lost+found")).unwrap();
    fs::write(ext.join("000000000000000A"), b"x").unwrap();
    fs::write(ext.join("+00000000000000a"), b"x").unwrap();

    let store = RedbStore::open(dir.path(), tiny()).unwrap();
    let mut expected = segments;
    expected.push("lost+found".to_owned());
    expected.sort();
    assert_eq!(segment_files(dir.path()), expected);
    assert_eq!(
        store
            .snapshot()
            .unwrap()
            .get(&t, &consensus_store::key::u64(10))
            .unwrap(),
        Some(big(EXTERN_THRESHOLD + 1, 10))
    );
}

#[test]
fn open_creates_nested_directories() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a").join("b").join("store");
    let store = RedbStore::open(&path, Config::default()).unwrap();
    store.apply(Batch::new().put(&table(), "k", "v")).unwrap();
    assert!(path.join("index.redb").exists());
}

fn break_first_byte(path: &Path) {
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut b = [0];
    file.read_exact_at(&mut b, 0).unwrap();
    file.write_all_at(&[!b[0]], 0).unwrap();
}

fn table() -> Table {
    Table::new("t").unwrap()
}

fn get(snap: &impl Snapshot, t: &Table, k: &str) -> Option<bytes::Bytes> {
    snap.get(t, k.as_bytes()).unwrap()
}

fn segment_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir.join("extern"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}
