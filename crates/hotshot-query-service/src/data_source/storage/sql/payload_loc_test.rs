//! Payload write and read path through the blob store.

use std::{
    os::unix::fs::FileExt,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use committable::Committable;
use hotshot_example_types::node_types::TEST_VERSIONS;
use hotshot_types::traits::EncodeBytes;
use journal_lane::{format, lane::segment_path};

use super::{testing::TmpDb, *};
use crate::{
    availability::{
        AvailabilityDataSource, BlockInfo, BlockQueryData, LeafQueryData, PayloadQueryData,
        UpdateAvailabilityData,
    },
    data_source::{
        SqlDataSource,
        storage::{
            AvailabilityStorage, ExplorerStorage, NodeStorage, UpdateAvailabilityStorage,
            blob::BlobLoc,
        },
    },
    explorer::{BlockDetail, BlockIdentifier},
    fetching::{Provider, provider::AnyProvider, request::PayloadRequest},
    node::BlockId,
    testing::mocks::{MockPayload, MockTypes, mock_transaction},
};

type Leaf = LeafQueryData<MockTypes>;
type Block = BlockQueryData<MockTypes>;

/// A block at `height` with the given transactions. Blocks with the same `ns_table` and
/// transactions share one `payload` row.
pub(super) async fn block_at(height: u64, ns_table: u64, txs: &[&[u8]]) -> (Leaf, Block) {
    let mut leaf =
        Leaf::genesis(&Default::default(), &Default::default(), TEST_VERSIONS.test).await;
    let header = leaf.leaf.block_header_mut();
    header.block_number = height;
    header.metadata.num_transactions = ns_table;
    let payload = MockPayload {
        transactions: txs.iter().map(|tx| mock_transaction(tx.to_vec())).collect(),
    };
    let block = Block::new(leaf.header().clone(), payload);
    (leaf, block)
}

async fn connect(db: &TmpDb) -> SqlStorage {
    SqlStorage::connect(db.config(), StorageConnectionType::Query)
        .await
        .unwrap()
}

pub(super) async fn store(storage: &SqlStorage, leaf: &Leaf, block: &Block) {
    let mut tx = storage.write().await.unwrap();
    tx.insert_leaf(leaf).await.unwrap();
    tx.insert_block(block).await.unwrap();
    tx.commit().await.unwrap();
}

pub(super) async fn count(storage: &SqlStorage, sql: &str) -> i64 {
    let mut tx = storage.read().await.unwrap();
    let (count,) = query_as::<(i64,)>(sql)
        .fetch_one(tx.as_mut())
        .await
        .unwrap();
    count
}

pub(super) fn counter(storage: &SqlStorage, name: &str) -> usize {
    storage.metrics.get_counter(name).unwrap().get()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_payload_roundtrips_on_all_read_paths() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let mut blocks = vec![];
    for (height, txs) in [
        (0, vec![]),
        (1, vec![&[1u8, 2][..], &[3]]),
        (2, vec![&[4][..]]),
    ] {
        let (leaf, block) = block_at(height, height, &txs).await;
        store(&storage, &leaf, &block).await;
        blocks.push(block);
    }

    // Only the non-empty payloads have a locator, and no payload bytes are in SQL.
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 2);
    assert_eq!(
        count(
            &storage,
            "SELECT count(*) FROM payload WHERE length(data) > 0"
        )
        .await,
        0
    );

    let mut tx = storage.read().await.unwrap();
    for (i, block) in blocks.iter().enumerate() {
        let id = BlockId::<MockTypes>::Number(i);
        assert_eq!(&tx.get_block(id).await.unwrap(), block);
        assert_eq!(
            tx.get_payload(id).await.unwrap(),
            PayloadQueryData::from(block.clone())
        );
    }
    let range = tx.get_block_range(0..3).await.unwrap();
    assert_eq!(
        range.into_iter().collect::<QueryResult<Vec<_>>>().unwrap(),
        blocks
    );
    let range = tx.get_payload_range(0..3).await.unwrap();
    assert_eq!(
        range.into_iter().collect::<QueryResult<Vec<_>>>().unwrap(),
        blocks
            .iter()
            .cloned()
            .map(PayloadQueryData::from)
            .collect::<Vec<_>>()
    );
    assert_eq!(tx.get_block_ranges(&[0..1, 1..3]).await.unwrap(), blocks);

    let (_, txn) = blocks[1].enumerate().next().unwrap();
    assert_eq!(
        tx.get_block_with_transaction(txn.commit()).await.unwrap(),
        blocks[1]
    );

    let detail = tx
        .get_block_detail(BlockIdentifier::Height(1))
        .await
        .unwrap();
    assert_eq!(detail, BlockDetail::try_from(blocks[1].clone()).unwrap());
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_empty_payload_writes_no_record() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let blobs = storage.blob_store().unwrap().clone();
    let bytes = blobs.payload_bytes();

    let (leaf, block) = block_at(1, 1, &[]).await;
    store(&storage, &leaf, &block).await;

    assert_eq!(blobs.payload_bytes(), bytes);
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 0);
    let mut tx = storage.read().await.unwrap();
    assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), block);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_inline_payload_is_kept_when_blobs_are_enabled() {
    let db = TmpDb::init().await;
    let inline = connect(&db).await;
    let (leaf1, block1) = block_at(1, 5, &[&[1, 2, 3]]).await;
    store(&inline, &leaf1, &block1).await;
    assert_eq!(
        count(
            &inline,
            "SELECT count(*) FROM payload WHERE length(data) > 0"
        )
        .await,
        1
    );

    // A new height with the same payload must not blank the inline bytes of the shared row.
    let dir = tempfile::tempdir().unwrap();
    let cfg = db.config().blob(BlobCfg {
        dir: dir.path().to_path_buf(),
        share_retention: Duration::from_secs(60),
    });
    let blobbed = SqlStorage::connect(cfg, StorageConnectionType::Query)
        .await
        .unwrap();
    let (leaf2, block2) = block_at(2, 5, &[&[1, 2, 3]]).await;
    store(&blobbed, &leaf2, &block2).await;

    assert_eq!(count(&blobbed, "SELECT count(*) FROM payload").await, 1);
    assert_eq!(
        count(
            &blobbed,
            "SELECT count(*) FROM payload WHERE length(data) > 0"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &blobbed,
            "SELECT count(*) FROM payload_loc WHERE height = 2"
        )
        .await,
        1
    );
    assert_eq!(count(&blobbed, "SELECT count(*) FROM payload_loc").await, 1);

    let mut tx = blobbed.read().await.unwrap();
    assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), block1);
    assert_eq!(tx.get_block(BlockId::Number(2)).await.unwrap(), block2);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_shared_payload_has_one_row_and_two_locators() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf1, block1) = block_at(1, 7, &[&[9, 9]]).await;
    let (leaf2, block2) = block_at(2, 7, &[&[9, 9]]).await;

    let mut tx = storage.write().await.unwrap();
    tx.insert_leaf(&leaf1).await.unwrap();
    tx.insert_leaf(&leaf2).await.unwrap();
    tx.insert_block_range([&block1, &block2]).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(count(&storage, "SELECT count(*) FROM payload").await, 1);
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 2);
    let mut tx = storage.read().await.unwrap();
    assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), block1);
    assert_eq!(tx.get_block(BlockId::Number(2)).await.unwrap(), block2);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_staged_payload_is_not_appended_in_the_transaction() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let blobs = storage.blob_store().unwrap().clone();
    let (leaf1, block1) = block_at(1, 1, &[&[1]]).await;
    let (leaf2, block2) = block_at(2, 2, &[&[2]]).await;

    let loc = blobs
        .append_payload(1, block1.payload.encode().to_vec())
        .await
        .unwrap();
    let staged_bytes = blobs.payload_bytes();
    let mut tx = storage.write().await.unwrap();
    UpdateAvailabilityStorage::<MockTypes>::attach_blobs(&mut tx, vec![(1, loc)]);
    tx.insert_leaf(&leaf1).await.unwrap();
    tx.insert_block(&block1).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(counter(&storage, "blob_staged_in_tx"), 0);
    assert_eq!(blobs.payload_bytes(), staged_bytes);

    store(&storage, &leaf2, &block2).await;
    assert_eq!(counter(&storage, "blob_staged_in_tx"), 1);
    assert!(blobs.payload_bytes() > staged_bytes);

    let stored: String = {
        let mut tx = storage.read().await.unwrap();
        let (loc,) = query_as::<(String,)>("SELECT loc FROM payload_loc WHERE height = 1")
            .fetch_one(tx.as_mut())
            .await
            .unwrap();
        loc
    };
    assert_eq!(stored.parse::<BlobLoc>().unwrap(), loc);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_connect_refuses_locators_without_blob_dir() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf, block) = block_at(1, 1, &[&[1]]).await;
    store(&storage, &leaf, &block).await;
    drop(storage);

    let mut plain = db.config();
    plain.blob = None;
    let err = SqlStorage::connect(plain, StorageConnectionType::Query)
        .await
        .expect_err("connect must fail");
    assert_eq!(
        err.to_string(),
        "payload_loc rows present but ESPRESSO_NODE_BLOB_DIR unset"
    );

    connect(&db).await;
}

#[cfg(feature = "embedded-db")]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_connect_refuses_locators_without_blob_dir_on_a_reused_pool() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf, block) = block_at(1, 1, &[&[1]]).await;
    store(&storage, &leaf, &block).await;

    let mut plain = db.config().pool(storage.pool());
    plain.blob = None;
    let err = SqlStorage::connect(plain, StorageConnectionType::Query)
        .await
        .expect_err("connect must fail");
    assert_eq!(
        err.to_string(),
        "payload_loc rows present but ESPRESSO_NODE_BLOB_DIR unset"
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_connect_refuses_the_blob_dir_of_another_database() {
    let first = TmpDb::init().await.with_blobs();
    connect(&first).await;

    let second = TmpDb::init().await;
    let cfg = second.config().blob(BlobCfg {
        dir: first.blob_dir().unwrap().to_path_buf(),
        share_retention: Duration::from_secs(60),
    });
    let err = SqlStorage::connect(cfg, StorageConnectionType::Query)
        .await
        .expect_err("connect must fail");
    assert!(
        format!("{err:#}").contains("belongs to a different database"),
        "{err:#}"
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_connect_restores_the_id_of_a_replaced_blob_dir() {
    let db = TmpDb::init().await.with_blobs();
    drop(connect(&db).await);
    let id_path = db.blob_dir().unwrap().join("ID");
    let id = std::fs::read_to_string(&id_path).unwrap();
    assert!(!id.is_empty());

    std::fs::remove_file(&id_path).unwrap();
    drop(connect(&db).await);

    assert_eq!(std::fs::read_to_string(&id_path).unwrap(), id);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_connect_with_reset_clears_the_blob_dir() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf, block) = block_at(1, 1, &[&[1]]).await;
    store(&storage, &leaf, &block).await;
    let loc = {
        let mut tx = storage.read().await.unwrap();
        let (loc,) = query_as::<(String,)>("SELECT loc FROM payload_loc WHERE height = 1")
            .fetch_one(tx.as_mut())
            .await
            .unwrap();
        loc.parse::<BlobLoc>().unwrap()
    };
    drop(storage);

    let storage = SqlStorage::connect(db.config().reset_schema(), StorageConnectionType::Query)
        .await
        .unwrap();

    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 0);
    let blobs = storage.blob_store().unwrap();
    assert_eq!(blobs.read_payload(1, loc).await.unwrap(), None);
    drop(storage);
    connect(&db).await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_shared_payload_without_its_own_locator_is_missing_in_sync_status() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf1, block1) = block_at(1, 7, &[&[9, 9]]).await;
    let (leaf2, _) = block_at(2, 7, &[&[9, 9]]).await;
    store(&storage, &leaf1, &block1).await;
    let mut tx = storage.write().await.unwrap();
    tx.insert_leaf(&leaf2).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx = storage.read().await.unwrap();
    let status = NodeStorage::<MockTypes>::sync_status_for_range(&mut tx, 1, 3)
        .await
        .unwrap();
    assert_eq!(status.blocks.missing, 1);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_unreadable_record_is_missing_until_restored() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf, block) = block_at(1, 1, &[&[1, 2, 3]]).await;
    store(&storage, &leaf, &block).await;
    drop(storage);

    for entry in std::fs::read_dir(db.blob_dir().unwrap().join("payload")).unwrap() {
        std::fs::remove_file(entry.unwrap().path()).unwrap();
    }
    let storage = connect(&db).await;

    let id = BlockId::<MockTypes>::Number(1);
    let mut tx = storage.read().await.unwrap();
    assert!(matches!(
        tx.get_block(id).await.unwrap_err(),
        QueryError::Missing
    ));
    assert!(matches!(
        tx.get_payload(id).await.unwrap_err(),
        QueryError::Missing
    ));
    drop(tx);
    assert_eq!(counter(&storage, "blob_missing"), 2);

    let mut tx = storage.write().await.unwrap();
    tx.insert_block(&block).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = storage.read().await.unwrap();
    assert_eq!(tx.get_block(id).await.unwrap(), block);
}

/// Serves one payload and counts the requests.
#[derive(Debug)]
struct ServePayload {
    payload: MockPayload,
    requests: Arc<AtomicUsize>,
}

#[async_trait]
impl Provider<MockTypes, PayloadRequest> for ServePayload {
    async fn fetch(&self, _req: PayloadRequest) -> Option<MockPayload> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        Some(self.payload.clone())
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_unreadable_record_is_refetched_and_gets_a_new_locator() {
    let db = TmpDb::init().await.with_blobs();
    let (leaf, block) = block_at(1, 1, &[&[1, 2, 3]]).await;
    let requests = Arc::new(AtomicUsize::new(0));
    let provider = AnyProvider::<MockTypes>::default().with_block_provider(ServePayload {
        payload: block.payload().clone(),
        requests: requests.clone(),
    });
    let ds: SqlDataSource<MockTypes, _> = db
        .config()
        .builder(provider)
        .await
        .unwrap()
        .disable_proactive_fetching()
        .build()
        .await
        .unwrap();
    ds.append(BlockInfo::new(leaf, Some(block.clone()), None, None))
        .await
        .unwrap();
    let stored = loc_of(&ds).await;

    // Flips a payload byte, so the checksum fails.
    let segment = segment_path(&db.blob_dir().unwrap().join("payload"), stored.seq);
    let at = stored.offset + format::FRAME_HEADER_LEN as u64;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(segment)
        .unwrap();
    file.write_all_at(&[0xFF], at).unwrap();

    let fetched = tokio::time::timeout(Duration::from_secs(30), ds.get_block(1).await)
        .await
        .expect("block was not refetched");
    assert_eq!(fetched, block);
    assert!(requests.load(Ordering::SeqCst) >= 1);
    let restored = loc_of(&ds).await;
    assert_ne!(restored, stored);

    let mut tx = ds.read().await.unwrap();
    assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), block);
}

async fn loc_of(ds: &SqlDataSource<MockTypes, AnyProvider<MockTypes>>) -> BlobLoc {
    let mut tx = ds.read().await.unwrap();
    let (loc,) = query_as::<(String,)>("SELECT loc FROM payload_loc WHERE height = 1")
        .fetch_one(tx.as_mut())
        .await
        .unwrap();
    loc.parse().unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_restore_of_a_height_keeps_the_last_committed_locator() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf, block) = block_at(1, 1, &[&[1, 2, 3]]).await;
    let blobs = storage.blob_store().unwrap();
    let body = block.payload.encode().as_ref().to_vec();
    let first = blobs.append_payload(1, body.clone()).await.unwrap();
    let second = blobs.append_payload(1, body).await.unwrap();
    assert_ne!(first, second);

    for loc in [first, second] {
        let mut tx = storage.write().await.unwrap();
        UpdateAvailabilityStorage::<MockTypes>::attach_blobs(&mut tx, vec![(1, loc)]);
        tx.insert_leaf(&leaf).await.unwrap();
        tx.insert_block(&block).await.unwrap();
        tx.commit().await.unwrap();
    }

    let mut tx = storage.read().await.unwrap();
    let (stored,) = query_as::<(String,)>("SELECT loc FROM payload_loc WHERE height = 1")
        .fetch_one(tx.as_mut())
        .await
        .unwrap();
    assert_eq!(stored.parse::<BlobLoc>().unwrap(), second);
    assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), block);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_block_below_the_pruned_height_gets_no_locator() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let mut tx = storage.write().await.unwrap();
    tx.save_pruned_height(5).await.unwrap();
    tx.commit().await.unwrap();

    let (leaf, block) = block_at(3, 3, &[&[1, 2, 3]]).await;
    store(&storage, &leaf, &block).await;

    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 0);
    assert_eq!(count(&storage, "SELECT count(*) FROM payload").await, 0);
}
