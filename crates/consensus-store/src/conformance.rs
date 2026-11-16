//! Checks every [`Store`] must pass.
//!
//! A backend runs them with [`run`], passing a function that opens an empty store.

use std::{
    ops::Bound,
    panic::{AssertUnwindSafe, catch_unwind},
};

use bytes::Bytes;

use crate::{Batch, EXTERN_THRESHOLD, KeyRange, Snapshot, Store, Table, Usage, key};

/// Runs every check, each on a fresh store from `fresh`. Panics on the first failure.
pub fn run<S: Store>(mut fresh: impl FnMut() -> S) {
    let checks: &[fn(&S)] = &[
        put_get_delete,
        ops_apply_in_order,
        range_bounds,
        delete_range,
        update,
        copy,
        drop_table,
        usage,
        large_values,
        snapshot_isolation,
        failed_apply_changes_nothing,
    ];
    for check in checks {
        check(&fresh());
    }
}

/// Checks that the stored usage of each table equals the usage of its rows.
pub fn assert_usage_matches_rows(snap: &impl Snapshot, tables: &[&Table]) {
    for t in tables {
        let mut expected = Usage::default();
        for (k, v) in entries(snap, t, &KeyRange::all()) {
            expected.add(Usage::of_row(k.len(), v.len()));
        }
        assert_eq!(snap.usage(t).unwrap(), expected, "usage of table {t}");
    }
}

/// `len` bytes that depend on `seed`, so that values of equal length differ.
pub fn big(len: usize, seed: u64) -> Bytes {
    (0..len)
        .map(|i| {
            (i as u64)
                .wrapping_mul(31)
                .wrapping_add(seed.wrapping_mul(97)) as u8
        })
        .collect()
}

fn put_get_delete<S: Store>(store: &S) {
    let t = table("t");
    let other = table("other");
    apply(store, Batch::new().put(&t, "a", "1").put(&t, "b", "2"));
    apply(
        store,
        Batch::new()
            .put(&t, "a", "3")
            .delete(&t, "b")
            .delete(&t, "absent"),
    );

    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "a"), Some(bytes("3")));
    assert_eq!(get(&snap, &t, "b"), None);
    assert_eq!(get(&snap, &t, "absent"), None);
    assert_eq!(get(&snap, &other, "a"), None);
    assert!(entries(&snap, &other, &KeyRange::all()).is_empty());
    assert_eq!(snap.usage(&other).unwrap(), Usage::default());
    assert_usage_matches_rows(&snap, &[&t, &other]);
}

fn ops_apply_in_order<S: Store>(store: &S) {
    let t = table("t");
    apply(
        store,
        Batch::new()
            .put(&t, "k", "v1")
            .delete(&t, "k")
            .delete(&t, "j")
            .put(&t, "j", "v")
            .update(&t, "j", |v| Some([v.unwrap(), b"x"].concat().into())),
    );
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "k"), None);
    assert_eq!(get(&snap, &t, "j"), Some(bytes("vx")));
    assert_usage_matches_rows(&snap, &[&t]);
}

fn range_bounds<S: Store>(store: &S) {
    let t = table("t");
    let keys: Vec<Bytes> = ["", "a", "a\0", "a\0\0", "b", "ba", "c"]
        .into_iter()
        .map(bytes)
        .collect();
    let mut batch = Batch::new();
    for k in keys.iter().rev() {
        batch.put(&t, k.clone(), k.clone());
    }
    apply(store, &batch);

    let snap = store.snapshot().unwrap();
    for r in sample_ranges() {
        let expected: Vec<_> = keys
            .iter()
            .filter(|k| r.contains(k))
            .map(|k| (k.clone(), k.clone()))
            .collect();
        assert_eq!(entries(&snap, &t, &r), expected, "range {r:?}");
    }
}

fn delete_range<S: Store>(store: &S) {
    let t = table("t");
    for r in sample_ranges() {
        let mut batch = Batch::new();
        batch.drop_table(&t);
        for i in 0..8u64 {
            batch.put(
                &t,
                key::u64(i).to_vec(),
                big(i as usize % 2 * EXTERN_THRESHOLD + 1, i),
            );
        }
        apply(store, &batch);
        apply(store, Batch::new().delete_range(&t, r.clone()));

        let snap = store.snapshot().unwrap();
        let left: Vec<_> = entries(&snap, &t, &KeyRange::all())
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        let expected: Vec<_> = (0..8u64)
            .map(|i| Bytes::from(key::u64(i).to_vec()))
            .filter(|k| !r.contains(k))
            .collect();
        assert_eq!(left, expected, "range {r:?}");
        assert_usage_matches_rows(&snap, &[&t]);
    }
}

fn update<S: Store>(store: &S) {
    let t = table("t");
    apply(store, Batch::new().update(&t, "k", |_| None));
    assert_eq!(get(&store.snapshot().unwrap(), &t, "k"), None);

    // Monotonic compare-and-set: keep the larger value.
    for (n, expected) in [(5, 5), (3, 5), (9, 9)] {
        apply(
            store,
            Batch::new().update(&t, "k", move |v| {
                let current = v.and_then(key::decode_u64);
                (current < Some(n)).then(|| key::u64(n).to_vec().into())
            }),
        );
        let snap = store.snapshot().unwrap();
        assert_eq!(
            get(&snap, &t, "k"),
            Some(Bytes::from(key::u64(expected).to_vec()))
        );
    }

    // The closure sees an extern value in full.
    let value = big(EXTERN_THRESHOLD + 7, 1);
    let seen = value.clone();
    apply(
        store,
        Batch::new()
            .put(&t, "big", value)
            .update(&t, "big", move |v| {
                assert_eq!(v, Some(seen.as_ref()));
                Some(bytes("small"))
            }),
    );
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "big"), Some(bytes("small")));
    assert_usage_matches_rows(&snap, &[&t]);
}

fn copy<S: Store>(store: &S) {
    let from = table("from");
    let to = table("to");
    let large = big(3 * EXTERN_THRESHOLD, 2);
    let other_large = big(2 * EXTERN_THRESHOLD, 3);
    apply(
        store,
        Batch::new()
            .put(&from, "small", "s")
            .put(&from, "large", large.clone())
            .put(&to, "kept", "x")
            .put(&to, "overwritten", other_large),
    );
    apply(
        store,
        Batch::new()
            .copy(&from, "small", &to, "small")
            .copy(&from, "large", &to, "large")
            .copy(&from, "large", &to, "overwritten")
            .copy(&from, "absent", &to, "kept")
            .copy(&from, "large", &from, "large"),
    );
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &to, "small"), Some(bytes("s")));
    assert_eq!(get(&snap, &to, "large"), Some(large.clone()));
    assert_eq!(get(&snap, &to, "overwritten"), Some(large.clone()));
    assert_eq!(get(&snap, &to, "kept"), Some(bytes("x")));
    assert_eq!(get(&snap, &from, "large"), Some(large.clone()));
    assert_usage_matches_rows(&snap, &[&from, &to]);

    // Copies outlive their source, and changing one changes no other.
    apply(
        store,
        Batch::new()
            .drop_table(&from)
            .put(&to, "small", "changed")
            .delete(&to, "overwritten"),
    );
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &to, "large"), Some(large));
    assert_eq!(get(&snap, &to, "small"), Some(bytes("changed")));
    assert_eq!(get(&snap, &from, "small"), None);
    assert_usage_matches_rows(&snap, &[&from, &to]);
}

fn drop_table<S: Store>(store: &S) {
    let dropped = table("dropped");
    let kept = table("kept");
    apply(
        store,
        Batch::new()
            .put(&dropped, "a", "1")
            .put(&dropped, "b", big(EXTERN_THRESHOLD + 1, 4))
            .put(&kept, "a", big(EXTERN_THRESHOLD + 1, 5)),
    );
    apply(
        store,
        Batch::new()
            .drop_table(&dropped)
            .drop_table(&table("never")),
    );

    let snap = store.snapshot().unwrap();
    assert!(entries(&snap, &dropped, &KeyRange::all()).is_empty());
    assert_eq!(snap.usage(&dropped).unwrap(), Usage::default());
    assert_eq!(get(&snap, &kept, "a"), Some(big(EXTERN_THRESHOLD + 1, 5)));

    apply(store, Batch::new().put(&dropped, "c", "3"));
    let snap = store.snapshot().unwrap();
    assert_eq!(
        entries(&snap, &dropped, &KeyRange::all()),
        vec![(bytes("c"), bytes("3"))]
    );
    assert_usage_matches_rows(&snap, &[&dropped, &kept]);
}

fn usage<S: Store>(store: &S) {
    let t = table("t");
    apply(
        store,
        Batch::new()
            .put(&t, "inline", big(EXTERN_THRESHOLD, 6))
            .put(&t, "extern", big(EXTERN_THRESHOLD + 1, 7)),
    );
    let snap = store.snapshot().unwrap();
    assert_eq!(
        snap.usage(&t).unwrap(),
        Usage {
            rows: 2,
            inline_bytes: ("inline".len() + EXTERN_THRESHOLD + "extern".len()) as u64,
            extern_bytes: EXTERN_THRESHOLD as u64 + 1,
        }
    );

    apply(
        store,
        Batch::new().put(&t, "extern", "now inline").put(
            &t,
            "inline",
            big(EXTERN_THRESHOLD + 2, 8),
        ),
    );
    let snap = store.snapshot().unwrap();
    assert_eq!(
        snap.usage(&t).unwrap(),
        Usage {
            rows: 2,
            inline_bytes: ("inline".len() + "extern".len() + "now inline".len()) as u64,
            extern_bytes: EXTERN_THRESHOLD as u64 + 2,
        }
    );

    apply(
        store,
        Batch::new().delete(&t, "inline").delete(&t, "extern"),
    );
    assert_eq!(
        store.snapshot().unwrap().usage(&t).unwrap(),
        Usage::default()
    );
}

fn large_values<S: Store>(store: &S) {
    let t = table("t");
    let lens = [
        0,
        1,
        EXTERN_THRESHOLD - 1,
        EXTERN_THRESHOLD,
        EXTERN_THRESHOLD + 1,
        3 * EXTERN_THRESHOLD,
        4 << 20,
    ];
    let mut batch = Batch::new();
    for (i, len) in lens.iter().enumerate() {
        batch.put(&t, key::u64(i as u64).to_vec(), big(*len, i as u64));
    }
    apply(store, &batch);

    let snap = store.snapshot().unwrap();
    for (i, len) in lens.iter().enumerate() {
        assert_eq!(
            snap.get(&t, &key::u64(i as u64)).unwrap(),
            Some(big(*len, i as u64)),
            "length {len}"
        );
    }
    let all = entries(&snap, &t, &KeyRange::all());
    assert_eq!(all.len(), lens.len());
    for ((_, v), len) in all.iter().zip(lens) {
        assert_eq!(v.len(), len);
    }
    assert_usage_matches_rows(&snap, &[&t]);
}

fn snapshot_isolation<S: Store>(store: &S) {
    let t = table("t");
    let large = big(2 * EXTERN_THRESHOLD, 9);
    apply(
        store,
        Batch::new()
            .put(&t, "a", "1")
            .put(&t, "large", large.clone()),
    );
    let before = store.snapshot().unwrap();

    apply(
        store,
        Batch::new().put(&t, "a", "2").delete(&t, "large").put(
            &t,
            "c",
            big(2 * EXTERN_THRESHOLD, 10),
        ),
    );
    apply(store, Batch::new().drop_table(&t));
    let after = store.snapshot().unwrap();

    assert_eq!(get(&before, &t, "a"), Some(bytes("1")));
    assert_eq!(get(&before, &t, "large"), Some(large.clone()));
    assert_eq!(get(&before, &t, "c"), None);
    assert_eq!(
        entries(&before, &t, &KeyRange::all()),
        vec![(bytes("a"), bytes("1")), (bytes("large"), large)]
    );
    assert_eq!(before.usage(&t).unwrap().rows, 2);
    assert!(entries(&after, &t, &KeyRange::all()).is_empty());
}

fn failed_apply_changes_nothing<S: Store>(store: &S) {
    let t = table("t");
    apply(
        store,
        Batch::new().put(&t, "kept", big(EXTERN_THRESHOLD + 1, 11)),
    );
    let usage = store.snapshot().unwrap().usage(&t).unwrap();

    let mut batch = Batch::new();
    batch
        .put(&t, "new", big(EXTERN_THRESHOLD + 1, 12))
        .delete(&t, "kept")
        .update(&t, "new", |_| panic!("deliberate"));
    let result = catch_unwind(AssertUnwindSafe(|| store.apply(&batch)));
    assert!(matches!(result, Err(_) | Ok(Err(_))), "the batch must fail");

    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "kept"), Some(big(EXTERN_THRESHOLD + 1, 11)));
    assert_eq!(get(&snap, &t, "new"), None);
    assert_eq!(snap.usage(&t).unwrap(), usage);

    // The store still works, and the extern data written by the failed batch is not reused
    // in a way that corrupts later values.
    apply(
        store,
        Batch::new().put(&t, "after", big(EXTERN_THRESHOLD + 5, 13)),
    );
    let snap = store.snapshot().unwrap();
    assert_eq!(get(&snap, &t, "after"), Some(big(EXTERN_THRESHOLD + 5, 13)));
    assert_eq!(get(&snap, &t, "kept"), Some(big(EXTERN_THRESHOLD + 1, 11)));
    assert_usage_matches_rows(&snap, &[&t]);
}

/// Ranges over the keys used in [`range_bounds`] and [`delete_range`], including empty ones.
fn sample_ranges() -> Vec<KeyRange> {
    let k = |s: &str| Bytes::copy_from_slice(s.as_bytes());
    let n = |i: u64| Bytes::from(key::u64(i).to_vec());
    vec![
        KeyRange::all(),
        KeyRange::starting_at(k("a\0")),
        KeyRange::before(k("b")),
        KeyRange::between(k("a"), k("ba")),
        KeyRange::new(Bound::Excluded(k("a")), Bound::Included(k("b"))),
        KeyRange::new(Bound::Excluded(k("a")), Bound::Excluded(k("a\0"))),
        KeyRange::between(k("c"), k("a")),
        KeyRange::before(Bytes::new()),
        KeyRange::starting_at(n(3)),
        KeyRange::between(n(2), n(6)),
        KeyRange::new(Bound::Included(n(5)), Bound::Included(n(5))),
        KeyRange::between(n(6), n(2)),
    ]
}

fn table(name: &str) -> Table {
    Table::new(name).unwrap()
}

fn bytes(s: &str) -> Bytes {
    Bytes::copy_from_slice(s.as_bytes())
}

fn apply(store: &impl Store, batch: &Batch) {
    store.apply(batch).unwrap();
}

fn get(snap: &impl Snapshot, t: &Table, k: &str) -> Option<Bytes> {
    snap.get(t, k.as_bytes()).unwrap()
}

fn entries(snap: &impl Snapshot, t: &Table, keys: &KeyRange) -> Vec<(Bytes, Bytes)> {
    snap.range(t, keys)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
