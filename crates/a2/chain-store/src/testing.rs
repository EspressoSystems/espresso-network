//! The contract of a [`Backend`], checked against any backend, and a throwaway Postgres server.

use std::{
    env,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};

use bytes::Bytes;

use crate::{Backend, Column, Part, Row};

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
        backend.parts(1).await.unwrap(),
        vec![(Payload, bytes("p1")), (VidCommon, bytes("v1"))]
    );
    assert_eq!(backend.parts(4).await.unwrap(), vec![]);

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

    // Epochs whose certificates are checked are recorded.
    assert!(!backend.certs_checked(3).await.unwrap());
    backend.mark_certs_checked(3).await.unwrap();
    backend.mark_certs_checked(3).await.unwrap();
    assert!(backend.certs_checked(3).await.unwrap());
    assert!(!backend.certs_checked(4).await.unwrap());

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
