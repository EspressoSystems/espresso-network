//! The contract of a [`Backend`], checked against any backend, and a throwaway Postgres server.

use std::{
    env,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};

use bytes::Bytes;

use crate::{AppWrite, Backend, Column, KeyBounds, Part, PayloadIndex, Row, TableName, Totals};

/// A row at `height` whose payload commitment is `payload`.
pub fn row(height: u64, payload: &str) -> Row {
    Row {
        height,
        leaf: format!("leaf-{height}"),
        block: format!("block-{height}"),
        payload: payload.into(),
        item: Bytes::from(format!("item-{height}")),
    }
}

fn bytes(s: &str) -> Bytes {
    Bytes::from(s.to_string())
}

/// Checks the contract of [`Backend`] on `backend`, which must be empty.
pub async fn conformance(backend: &dyn Backend) {
    use Part::{Payload, VidCommon};

    assert_eq!(backend.newest().await.unwrap(), None);
    assert_eq!(backend.floor().await.unwrap(), 0);
    assert!(backend.missing_items(0, 10).await.unwrap().is_empty());
    assert_eq!(backend.newest_gap().await.unwrap(), None);

    // A lease goes to the first holder, which renews it, and to another once its time ran out.
    let long = std::time::Duration::from_secs(60);
    let short = std::time::Duration::from_millis(50);
    assert!(backend.lease("work", "a", long).await.unwrap());
    assert!(backend.lease("work", "a", short).await.unwrap(), "renewed");
    assert!(
        !backend.lease("work", "b", long).await.unwrap(),
        "held by a"
    );
    assert!(backend.lease("other", "b", long).await.unwrap());
    tokio::time::sleep(short * 3).await;
    assert!(
        backend.lease("work", "b", long).await.unwrap(),
        "a's time ran out"
    );
    assert!(
        !backend.lease("work", "a", long).await.unwrap(),
        "held by b"
    );

    let mut changes = backend.changes();
    changes.borrow_and_update();
    // Heights 1 and 5 share a payload commitment, as blocks with the same payload do. Height 4
    // arrives without its parts, height 5 without its VID common data.
    for (height, commitment, parts) in [
        (0, "p", vec![(Payload, "p0"), (VidCommon, "v0")]),
        (1, "shared", vec![(Payload, "p1"), (VidCommon, "v1")]),
        (4, "p", vec![]),
        (5, "shared", vec![(Payload, "p5")]),
    ] {
        let parts = parts.into_iter().map(|(p, d)| (p, bytes(d))).collect();
        backend.put(row(height, commitment), parts).await.unwrap();
    }
    assert!(changes.has_changed().unwrap(), "a write changes `changes`");

    // What was written is read back.
    assert_eq!(backend.item(1).await.unwrap(), Some(bytes("item-1")));
    assert_eq!(backend.part(1, Payload).await.unwrap(), Some(bytes("p1")));
    assert_eq!(backend.part(1, VidCommon).await.unwrap(), Some(bytes("v1")));
    assert_eq!(backend.item(2).await.unwrap(), None);
    assert_eq!(backend.part(4, Payload).await.unwrap(), None);
    assert_eq!(backend.newest().await.unwrap(), Some(5));
    assert_eq!(
        backend.record(1, true).await.unwrap(),
        Some((
            bytes("item-1"),
            vec![(Payload, bytes("p1")), (VidCommon, bytes("v1"))]
        ))
    );
    assert_eq!(
        backend.record(1, false).await.unwrap(),
        Some((bytes("item-1"), vec![(VidCommon, bytes("v1"))])),
        "without the payload"
    );
    assert_eq!(
        backend.record(4, true).await.unwrap(),
        Some((bytes("item-4"), vec![]))
    );
    assert_eq!(backend.record(2, true).await.unwrap(), None);

    // An empty part is kept as present.
    backend
        .put_parts(0, vec![(Part::CertsKnown, Bytes::new())])
        .await
        .unwrap();
    assert_eq!(
        backend.part(0, Part::CertsKnown).await.unwrap(),
        Some(Bytes::new())
    );
    assert_eq!(
        backend
            .missing_parts(Part::CertsKnown, 0, 10)
            .await
            .unwrap(),
        vec![1, 4, 5]
    );

    // Lookups find the height; a commitment several rows share finds the earliest.
    assert_eq!(
        backend.height_of(Column::Leaf, "leaf-4").await.unwrap(),
        Some(4)
    );
    assert_eq!(
        backend.height_of(Column::Block, "block-5").await.unwrap(),
        Some(5)
    );
    assert_eq!(
        backend.height_of(Column::Payload, "shared").await.unwrap(),
        Some(1)
    );
    assert_eq!(backend.height_of(Column::Leaf, "none").await.unwrap(), None);

    // Gaps: heights without an item up to the newest, and items without a part.
    assert_eq!(backend.missing_items(0, 10).await.unwrap(), vec![2..4]);
    assert_eq!(backend.missing_items(3, 10).await.unwrap(), vec![3..4]);
    assert_eq!(backend.missing_items(4, 10).await.unwrap(), vec![]);
    assert_eq!(backend.newest_gap().await.unwrap(), Some(2..4));
    assert_eq!(
        backend.missing_parts(Payload, 0, 10).await.unwrap(),
        vec![4]
    );
    assert_eq!(
        backend.missing_parts(VidCommon, 0, 10).await.unwrap(),
        vec![4, 5]
    );

    // Writing again keeps what is stored, and fills only what is missing.
    let mut other = row(1, "shared");
    other.item = bytes("other");
    backend
        .put(other, vec![(Payload, bytes("other"))])
        .await
        .unwrap();
    assert_eq!(backend.item(1).await.unwrap(), Some(bytes("item-1")));
    assert_eq!(backend.part(1, Payload).await.unwrap(), Some(bytes("p1")));
    backend
        .put_parts(4, vec![(Payload, bytes("p4"))])
        .await
        .unwrap();
    backend
        .put_parts(4, vec![(Payload, bytes("other"))])
        .await
        .unwrap();
    backend
        .put_parts(5, vec![(VidCommon, bytes("v5"))])
        .await
        .unwrap();
    assert_eq!(backend.part(4, Payload).await.unwrap(), Some(bytes("p4")));
    assert_eq!(backend.part(5, VidCommon).await.unwrap(), Some(bytes("v5")));
    assert!(
        backend
            .missing_parts(Payload, 0, 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        backend.missing_parts(VidCommon, 0, 10).await.unwrap(),
        vec![4]
    );

    // Application tables keep their keys apart, in bytewise order.
    let (a, b) = (TableName::of("a"), TableName::of("b"));
    let put = |table: &TableName, key: &[u8], value: &str| AppWrite::Put {
        table: table.clone(),
        key: Bytes::copy_from_slice(key),
        value: bytes(value),
    };
    backend
        .app_write(vec![
            put(&a, &[2], "a2"),
            put(&a, &[1], "a1"),
            put(&a, &[3, 0], "a30"),
            put(&b, &[1], "b1"),
            put(&a, &[1], "a1'"),
        ])
        .await
        .unwrap();
    assert_eq!(backend.app_get(&a, &[1]).await.unwrap(), Some(bytes("a1'")));
    assert_eq!(backend.app_get(&a, &[9]).await.unwrap(), None);
    let keys = |entries: Vec<(Bytes, Bytes)>| -> Vec<Vec<u8>> {
        entries.into_iter().map(|(key, _)| key.to_vec()).collect()
    };
    let from = |from: &'static [u8]| KeyBounds {
        from,
        until: None,
        limit: usize::MAX,
    };
    assert_eq!(
        keys(backend.app_entries(&a, from(&[])).await.unwrap()),
        vec![vec![1], vec![2], vec![3, 0]]
    );
    assert_eq!(
        keys(backend.app_entries(&a, from(&[2])).await.unwrap()),
        vec![vec![2], vec![3, 0]]
    );
    // Entries end before `until`, and `limit` bounds them.
    let bounded = KeyBounds {
        from: &[],
        until: Some(&[3]),
        limit: usize::MAX,
    };
    assert_eq!(
        keys(backend.app_entries(&a, bounded).await.unwrap()),
        vec![vec![1], vec![2]]
    );
    let limited = KeyBounds {
        limit: 2,
        ..from(&[])
    };
    assert_eq!(
        keys(backend.app_entries(&a, limited).await.unwrap()),
        vec![vec![1], vec![2]]
    );
    backend
        .app_write(vec![AppWrite::DeleteBefore {
            table: a.clone(),
            key: Bytes::from_static(&[2]),
        }])
        .await
        .unwrap();
    assert_eq!(
        keys(backend.app_entries(&a, from(&[])).await.unwrap()),
        vec![vec![2], vec![3, 0]]
    );
    backend
        .app_write(vec![AppWrite::Clear { table: a.clone() }])
        .await
        .unwrap();
    assert!(backend.app_entries(&a, from(&[])).await.unwrap().is_empty());
    assert_eq!(backend.app_get(&b, &[1]).await.unwrap(), Some(bytes("b1")));

    // A gap at the start counts, and `limit` bounds the answer.
    backend.put(row(8, "p"), Vec::new()).await.unwrap();
    backend.prune_below(0).await.unwrap();
    assert_eq!(backend.missing_items(0, 1).await.unwrap(), vec![2..4]);
    assert_eq!(
        backend.missing_items(0, 10).await.unwrap(),
        vec![2..4, 6..8]
    );
    assert_eq!(backend.newest_gap().await.unwrap(), Some(6..8));
    assert_eq!(backend.missing_parts(Payload, 0, 1).await.unwrap(), vec![8]);

    // Pruning deletes what is before the floor and refuses later writes there.
    backend.prune_below(2).await.unwrap();
    assert_eq!(backend.floor().await.unwrap(), 2);
    assert_eq!(backend.item(1).await.unwrap(), None);
    assert_eq!(backend.part(1, Payload).await.unwrap(), None);
    backend
        .put(row(1, "p"), vec![(Payload, bytes("p"))])
        .await
        .unwrap();
    backend
        .put_parts(0, vec![(Payload, bytes("p"))])
        .await
        .unwrap();
    assert_eq!(backend.item(1).await.unwrap(), None);
    assert_eq!(backend.part(0, Payload).await.unwrap(), None);
    assert_eq!(
        backend.missing_items(0, 10).await.unwrap(),
        vec![2..4, 6..8]
    );
    backend.prune_below(1).await.unwrap();
    assert_eq!(backend.floor().await.unwrap(), 2, "the floor only rises");

    // A gap the floor starts in counts from the floor.
    backend.prune_below(3).await.unwrap();
    assert_eq!(
        backend.missing_items(0, 10).await.unwrap(),
        vec![3..4, 6..8]
    );
    backend.prune_below(7).await.unwrap();
    assert_eq!(backend.newest_gap().await.unwrap(), Some(7..8));

    // Filling the gap leaves none.
    backend.put(row(7, "p"), Vec::new()).await.unwrap();
    assert_eq!(backend.newest_gap().await.unwrap(), None);

    index(backend).await;
    writes_with_item(backend).await;
}

/// Application writes made with an item land with it, and not at all for a height before the
/// floor; heights after those used before.
async fn writes_with_item(backend: &dyn Backend) {
    let table = TableName::of("with-item");
    let put = |key: &str| AppWrite::Put {
        table: table.clone(),
        key: bytes(key),
        value: bytes("v"),
    };
    backend
        .put_with(row(100, "w"), Vec::new(), vec![put("a")])
        .await
        .unwrap();
    assert!(backend.item(100).await.unwrap().is_some());
    assert_eq!(
        backend.app_get(&table, b"a").await.unwrap(),
        Some(bytes("v"))
    );

    let floor = backend.floor().await.unwrap();
    assert!(floor > 0, "the checks before raised the floor");
    backend
        .put_with(row(floor - 1, "w"), Vec::new(), vec![put("b")])
        .await
        .unwrap();
    assert_eq!(backend.item(floor - 1).await.unwrap(), None);
    assert_eq!(backend.app_get(&table, b"b").await.unwrap(), None);
}

/// The transaction index and the counts, at heights after those [`conformance`] used before.
async fn index(backend: &dyn Backend) {
    let index = |transactions: &[&str], namespaces: &[(u32, u64, u64)]| {
        let index = PayloadIndex {
            transactions: transactions.iter().map(ToString::to_string).collect(),
            namespaces: namespaces
                .iter()
                .map(|&(id, transactions, bytes)| {
                    (
                        id,
                        Totals {
                            transactions,
                            bytes,
                        },
                    )
                })
                .collect(),
            total: Totals {
                transactions: transactions.len() as u64,
                bytes: namespaces.iter().map(|&(_, _, bytes)| bytes).sum(),
            },
        };
        vec![(Part::Index, crate::encode(&index).unwrap())]
    };
    // Height 11's index comes after its row; 12 holds a transaction 10 holds too.
    backend
        .put(
            row(10, "i10"),
            index(&["a", "b", "c"], &[(7, 2, 20), (9, 1, 5)]),
        )
        .await
        .unwrap();
    backend.put(row(11, "i11"), Vec::new()).await.unwrap();
    backend
        .put_parts(11, index(&["d"], &[(7, 1, 8)]))
        .await
        .unwrap();
    backend
        .put(row(12, "i12"), index(&["b"], &[(9, 1, 4)]))
        .await
        .unwrap();
    // A height's index, once written, is kept.
    backend
        .put_parts(11, index(&["x"], &[(7, 9, 99)]))
        .await
        .unwrap();

    let position = |hash| backend.transaction_position(hash);
    assert_eq!(position("b").await.unwrap(), Some((10, 1)), "the earliest");
    assert_eq!(position("d").await.unwrap(), Some((11, 0)));
    assert_eq!(position("x").await.unwrap(), None, "kept");
    let totals = |heights, namespace| backend.totals(heights, namespace);
    let of = |transactions, bytes| Totals {
        transactions,
        bytes,
    };
    assert_eq!(totals(10..=12, None).await.unwrap(), of(5, 37));
    assert_eq!(totals(11..=12, Some(7)).await.unwrap(), of(1, 8));
    assert_eq!(totals(10..=12, Some(9)).await.unwrap(), of(2, 9));
    assert_eq!(totals(13..=u64::MAX, None).await.unwrap(), of(0, 0));
    let (_, parts) = backend.record(10, true).await.unwrap().unwrap();
    assert!(
        parts.iter().all(|(part, _)| *part != Part::Index),
        "the index is not read back as a part"
    );

    backend.prune_below(11).await.unwrap();
    assert_eq!(position("a").await.unwrap(), None, "pruned");
    assert_eq!(totals(0..=u64::MAX, None).await.unwrap(), of(2, 12));
}

/// A throwaway Postgres server, with user `postgres` and trust auth, removed on drop. Needs
/// `initdb`, `postgres` and `pg_isready` on the `PATH`.
#[derive(Debug)]
pub struct TmpPostgres {
    port: u16,
    data_dir: PathBuf,
    server: Child,
}

impl TmpPostgres {
    pub async fn init() -> Self {
        let port = test_utils::reserve_tcp_port().expect("a free port");
        let data_dir = env::temp_dir().join(format!("chain-store-postgres-{port}"));
        let _ = std::fs::remove_dir_all(&data_dir);
        let initdb = Command::new("initdb")
            .arg("-D")
            .arg(&data_dir)
            .args(["-U", "postgres", "--auth=trust"])
            .output()
            .expect("initdb runs; is Postgres installed?");
        assert!(
            initdb.status.success(),
            "initdb: {}",
            String::from_utf8_lossy(&initdb.stderr)
        );
        let server = Command::new("postgres")
            .arg("-D")
            .arg(&data_dir)
            .args(["-p", &port.to_string(), "-h", "127.0.0.1", "-k"])
            .arg(&data_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("postgres starts");
        let db = Self {
            port,
            data_dir,
            server,
        };
        db.wait_until_ready().await;
        db
    }

    pub fn url(&self) -> String {
        format!("postgres://postgres@127.0.0.1:{}/postgres", self.port)
    }

    async fn wait_until_ready(&self) {
        for _ in 0..300 {
            let ready = Command::new("pg_isready")
                .args(["-h", "127.0.0.1", "-p", &self.port.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if ready {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("postgres on port {} did not become ready", self.port);
    }
}

impl Drop for TmpPostgres {
    fn drop(&mut self) {
        let _ = self.server.kill();
        let _ = self.server.wait();
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}
