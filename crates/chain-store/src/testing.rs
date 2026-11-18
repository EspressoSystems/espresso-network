//! The contract of a [`Backend`], checked against any backend, and a throwaway Postgres server.

use std::{
    env,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};

use bytes::Bytes;

use crate::{Backend, Column, Row};

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
    assert_eq!(backend.newest().await.unwrap(), None);
    assert_eq!(backend.floor().await.unwrap(), 0);
    assert!(backend.missing_items(0, 10).await.unwrap().is_empty());

    let mut changes = backend.changes();
    changes.borrow_and_update();
    // Heights 1 and 5 share a payload commitment, as blocks with the same payload do.
    for (height, commitment, payload) in [
        (0, "p", Some("p0")),
        (1, "shared", Some("p1")),
        (4, "p", None),
        (5, "shared", Some("p5")),
    ] {
        backend
            .put(row(height, commitment), payload.map(bytes))
            .await
            .unwrap();
    }
    assert!(changes.has_changed().unwrap(), "a write changes `changes`");

    // What was written is read back.
    assert_eq!(backend.item(1).await.unwrap(), Some(bytes("item-1")));
    assert_eq!(backend.payload(1).await.unwrap(), Some(bytes("p1")));
    assert_eq!(backend.item(2).await.unwrap(), None);
    assert_eq!(backend.payload(4).await.unwrap(), None);
    assert_eq!(backend.newest().await.unwrap(), Some(5));

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

    // Gaps: heights without an item up to the newest, and items without a payload.
    assert_eq!(backend.missing_items(0, 10).await.unwrap(), vec![2..4]);
    assert_eq!(backend.missing_items(3, 10).await.unwrap(), vec![3..4]);
    assert_eq!(backend.missing_items(4, 10).await.unwrap(), vec![]);
    assert_eq!(backend.missing_payloads(0, 10).await.unwrap(), vec![4]);

    // Writing again keeps what is stored, and fills only what is missing.
    let mut other = row(1, "shared");
    other.item = bytes("other");
    backend.put(other, Some(bytes("other"))).await.unwrap();
    assert_eq!(backend.item(1).await.unwrap(), Some(bytes("item-1")));
    assert_eq!(backend.payload(1).await.unwrap(), Some(bytes("p1")));
    backend.put_payload(4, bytes("p4")).await.unwrap();
    backend.put_payload(4, bytes("other")).await.unwrap();
    assert_eq!(backend.payload(4).await.unwrap(), Some(bytes("p4")));
    assert!(backend.missing_payloads(0, 10).await.unwrap().is_empty());

    // A gap at the start counts, and `limit` bounds the answer.
    backend.put(row(8, "p"), None).await.unwrap();
    backend.prune_below(0).await.unwrap();
    assert_eq!(backend.missing_items(0, 1).await.unwrap(), vec![2..4]);
    assert_eq!(
        backend.missing_items(0, 10).await.unwrap(),
        vec![2..4, 6..8]
    );
    assert_eq!(backend.missing_payloads(0, 1).await.unwrap(), vec![8]);

    // Pruning deletes what is before the floor and refuses later writes there.
    backend.prune_below(2).await.unwrap();
    assert_eq!(backend.floor().await.unwrap(), 2);
    assert_eq!(backend.item(1).await.unwrap(), None);
    assert_eq!(backend.payload(1).await.unwrap(), None);
    backend.put(row(1, "p"), Some(bytes("p"))).await.unwrap();
    backend.put_payload(0, bytes("p")).await.unwrap();
    assert_eq!(backend.item(1).await.unwrap(), None);
    assert_eq!(backend.payload(0).await.unwrap(), None);
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
