//! Pruning and garbage collection of blob segments.

use tempfile::TempDir;
use tokio::time::{sleep, timeout};

use super::{
    payload_loc_test::{block_at, count, counter, store},
    testing::TmpDb,
    *,
};
use crate::{data_source::storage::blob::BlobLoc, node::BlockId, testing::mocks::MockTypes};

/// Every blob record rolls its segment, so each lands in a segment of its own.
const ROLL_EACH: u64 = 1;

/// A store without blobs from `connect`, with a blob store of one-record segments attached.
async fn connect_small(db: &TmpDb, share_retention: Duration) -> (SqlStorage, TempDir) {
    let mut storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = BlobCfg {
        dir: dir.path().to_path_buf(),
        share_retention,
    };
    let blobs = BlobStore::open_with(cfg, Arc::new(StdFs), ROLL_EACH)
        .await
        .unwrap();
    blobs.install_metrics(&storage.metrics);
    storage.blobs = Some(blobs);
    (storage, dir)
}

async fn connect(db: &TmpDb) -> SqlStorage {
    SqlStorage::connect(db.config(), StorageConnectionType::Query)
        .await
        .unwrap()
}

/// Stores one block with a payload of its own at each height.
async fn store_blocks(storage: &SqlStorage, heights: &[u64]) -> Vec<(u64, BlobLoc)> {
    for &height in heights {
        let (leaf, block) = block_at(height, height, &[&[height as u8]]).await;
        store(storage, &leaf, &block).await;
    }
    let mut locs = vec![];
    for &height in heights {
        locs.push((height, loc_of(storage, height).await));
    }
    locs
}

/// Appends a record at a height beyond any test height, so the writer rolls past the segments
/// written so far.
async fn seal_segments(storage: &SqlStorage) {
    let blobs = storage.blob_store().unwrap();
    blobs.append_payload(1000, vec![0]).await.unwrap();
}

async fn loc_of(storage: &SqlStorage, height: u64) -> BlobLoc {
    let mut tx = storage.read().await.unwrap();
    let (loc,) = query_as::<(String,)>("SELECT loc FROM payload_loc WHERE height = $1")
        .bind(height as i64)
        .fetch_one(tx.as_mut())
        .await
        .unwrap();
    loc.parse().unwrap()
}

async fn readable(storage: &SqlStorage, locs: &[(u64, BlobLoc)]) -> Vec<bool> {
    let blobs = storage.blob_store().unwrap();
    let mut readable = vec![];
    for &(height, loc) in locs {
        readable.push(blobs.read_payload(height, loc).await.unwrap().is_some());
    }
    readable
}

fn bytes_gauge(storage: &SqlStorage, stream: &str) -> usize {
    storage
        .metrics
        .gauge_family("blob_bytes")
        .unwrap()
        .get(&[stream])
        .get()
}

/// Appends a share per height and each time waits until the writer has indexed it.
async fn append_shares(blobs: &BlobStore, heights: &[u64]) {
    for &height in heights {
        blobs.append_share(height, vec![height as u8; 10]);
        timeout(Duration::from_secs(5), async {
            while blobs.read_share(height).await.unwrap().is_none() {
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("share was never written");
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_unlinks_segments_up_to_the_pruned_height() {
    let db = TmpDb::init().await;
    let (storage, _dir) = connect_small(&db, Duration::from_secs(3600)).await;
    let locs = store_blocks(&storage, &[1, 2, 3, 4]).await;
    seal_segments(&storage).await;
    let blobs = storage.blob_store().unwrap().clone();
    let before = blobs.payload_bytes();
    assert_eq!(readable(&storage, &locs).await, [true; 4]);

    storage.prune_data_batch(2).await.unwrap();

    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 2);
    assert_eq!(readable(&storage, &locs).await, [false, false, true, true]);
    assert!(blobs.payload_bytes() < before);
    assert_eq!(
        bytes_gauge(&storage, "payload"),
        blobs.payload_bytes() as usize
    );
    assert_eq!(counter(&storage, "blob_gc_unlinked"), 2);

    storage.prune_data_batch(3).await.unwrap();
    assert_eq!(readable(&storage, &locs).await, [false, false, false, true]);
    assert_eq!(counter(&storage, "blob_gc_unlinked"), 3);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_keeps_a_segment_holding_a_backfilled_height() {
    let db = TmpDb::init().await;
    let (storage, _dir) = connect_small(&db, Duration::from_secs(3600)).await;
    // Height 1 lands after height 10, so its segment inherits the bound 10.
    let locs = store_blocks(&storage, &[10, 1, 11]).await;
    seal_segments(&storage).await;

    storage.prune_data_batch(5).await.unwrap();
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 2);
    assert_eq!(readable(&storage, &locs).await, [true; 3]);
    assert_eq!(counter(&storage, "blob_gc_unlinked"), 0);

    storage.prune_data_batch(10).await.unwrap();
    assert_eq!(readable(&storage, &locs).await, [false, false, true]);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_shared_payload_keeps_the_upper_height_readable() {
    let db = TmpDb::init().await;
    let (storage, _dir) = connect_small(&db, Duration::from_secs(3600)).await;
    let (leaf1, block1) = block_at(1, 7, &[&[9, 9]]).await;
    let (leaf2, block2) = block_at(2, 7, &[&[9, 9]]).await;
    store(&storage, &leaf1, &block1).await;
    store(&storage, &leaf2, &block2).await;
    seal_segments(&storage).await;

    storage.prune_data_batch(1).await.unwrap();

    assert_eq!(count(&storage, "SELECT count(*) FROM payload").await, 1);
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 1);
    let mut tx = storage.read().await.unwrap();
    assert_eq!(
        tx.get_block(BlockId::<MockTypes>::Number(2)).await.unwrap(),
        block2
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_reader_with_an_unlinked_locator_gets_missing() {
    let db = TmpDb::init().await;
    let (storage, _dir) = connect_small(&db, Duration::from_secs(3600)).await;
    store_blocks(&storage, &[1, 2]).await;
    seal_segments(&storage).await;

    storage
        .blob_store()
        .unwrap()
        .gc_payload_below(1)
        .await
        .unwrap();

    let mut tx = storage.read().await.unwrap();
    let id = BlockId::<MockTypes>::Number(1);
    assert!(matches!(
        tx.get_block(id).await.unwrap_err(),
        QueryError::Missing
    ));
    assert!(tx.get_block(BlockId::<MockTypes>::Number(2)).await.is_ok());
    drop(tx);
    assert_eq!(counter(&storage, "blob_missing"), 1);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_gc_at_connect_reclaims_segments_after_a_crash() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let locs = store_blocks(&storage, &[1, 2]).await;
    drop(storage);

    let storage = connect(&db).await;
    assert_eq!(readable(&storage, &locs).await, [true, true]);

    // The prune commits, then the process stops before `gc_payload_below`.
    let mut tx = storage.write().await.unwrap();
    tx.save_pruned_height(2).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = storage.prune_write().await.unwrap();
    tx.delete_batch(2).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 0);
    assert_eq!(readable(&storage, &locs).await, [true, true]);
    drop(storage);

    let storage = connect(&db).await;
    assert_eq!(readable(&storage, &locs).await, [false, false]);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_disk_usage_counts_sealed_payload_bytes() {
    let db = TmpDb::init().await;
    let (storage, _dir) = connect_small(&db, Duration::from_secs(3600)).await;
    let before = storage.get_disk_usage().await.unwrap();
    let blobs = storage.blob_store().unwrap();

    blobs.append_payload(1, vec![0; 100_000]).await.unwrap();
    seal_segments(&storage).await;

    assert!(storage.get_disk_usage().await.unwrap() >= before + 100_000);
}

/// The active segment cannot be unlinked, so counting it would keep threshold pruning running
/// past what pruning can free.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_disk_usage_ignores_the_active_payload_segment() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let before = storage.get_disk_usage().await.unwrap();

    let blobs = storage.blob_store().unwrap();
    blobs.append_payload(1, vec![0; 100_000]).await.unwrap();

    assert!(storage.get_disk_usage().await.unwrap() < before + 100_000);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_run_unlinks_expired_share_segments_once() {
    let db = TmpDb::init().await;
    let (mut storage, _dir) = connect_small(&db, Duration::ZERO).await;
    storage.set_pruning_config(PrunerCfg::new());
    let blobs = storage.blob_store().unwrap().clone();
    append_shares(&blobs, &[1, 2, 1000]).await;

    let mut pruner = None;
    storage.prune(&mut pruner).await.unwrap();
    assert_eq!(blobs.read_share(1).await.unwrap(), None);
    assert_eq!(blobs.read_share(2).await.unwrap(), None);

    // Later calls of the same run leave shares alone; the next run unlinks them.
    append_shares(&blobs, &[3, 1001]).await;
    storage.prune(&mut pruner).await.unwrap();
    assert!(blobs.read_share(3).await.unwrap().is_some());
    storage.prune(&mut None).await.unwrap();
    assert_eq!(blobs.read_share(3).await.unwrap(), None);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_gc_at_connect_unlinks_expired_share_segments() {
    let db = TmpDb::init().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = db.config().blob(BlobCfg {
        dir: dir.path().to_path_buf(),
        share_retention: Duration::ZERO,
    });
    let storage = SqlStorage::connect(cfg.clone(), StorageConnectionType::Query)
        .await
        .unwrap();
    append_shares(storage.blob_store().unwrap(), &[1]).await;
    drop(storage);

    let storage = SqlStorage::connect(cfg, StorageConnectionType::Query)
        .await
        .unwrap();
    assert_eq!(
        storage.blob_store().unwrap().read_share(1).await.unwrap(),
        None
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pruning_gc_at_connect_finishes_the_delete_before_unlinking() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let locs = store_blocks(&storage, &[1, 2]).await;
    drop(storage);

    // The process stops after the pruned height commits and before the delete batch.
    let storage = connect(&db).await;
    let mut tx = storage.write().await.unwrap();
    tx.save_pruned_height(2).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 2);
    drop(storage);

    let storage = connect(&db).await;
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 0);
    assert_eq!(count(&storage, "SELECT count(*) FROM header").await, 0);
    assert_eq!(readable(&storage, &locs).await, [false, false]);
}
